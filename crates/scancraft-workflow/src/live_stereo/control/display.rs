//! Validated display-only state for a supervised scan process. Commands, raw
//! recordings, calibration binding and source qualification are never imported.
use super::{Control, Phase, Snapshot};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

/// The transport must bound serialized message size before deserializing this
/// value. Validation additionally bounds geometry and thumbnail dimensions.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayUpdate {
    pub phase: Phase,
    pub recorded_frames: [usize; 2],
    pub processing_drops: [usize; 2],
    pub skipped_previews: usize,
    pub preview: Option<Snapshot>,
}
impl DisplayUpdate {
    /// Validate before changing parent state; these checks attest display safety,
    /// not scanner identity, simultaneous exposures or geometric accuracy.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(preview) = &self.preview {
            if preview.cloud.points.len() > 50_000
                || preview.fused_points < preview.cloud.points.len()
            {
                return Err("remote preview exceeds its point budget or fused count".into());
            }
            preview.cloud.validate()?;
            for camera in &preview.cameras {
                if camera.width == 0
                    || camera.height == 0
                    || camera.width > 320
                    || camera.height > 240
                    || camera.width.checked_mul(camera.height) != Some(camera.pixels.len())
                {
                    return Err(
                        "remote preview thumbnail has invalid dimensions or pixel count".into(),
                    );
                }
            }
        }
        Ok(())
    }
}
impl Control {
    /// Parent-side display/command controller. A local runner cannot begin with
    /// it, and imported display updates never replace its command atomics.
    pub fn new_remote_display(preview_enabled: bool) -> Self {
        let mut control = Self::new(preview_enabled);
        control.remote_display = true;
        control
    }
    /// Export local counters and move the one-slot preview out of the mailbox.
    /// This does not consume or clear pause/stop requests.
    pub fn take_display_update(&self) -> DisplayUpdate {
        DisplayUpdate {
            phase: self.phase(),
            recorded_frames: self.recorded_frames(),
            processing_drops: self.processing_drops(),
            skipped_previews: self.skipped_previews(),
            preview: self.take_preview(),
        }
    }
    /// Import only validated display state into a remote parent controller.
    /// Local-runner controls reject this call before any state is changed.
    pub fn apply_display_update(&self, update: DisplayUpdate) -> Result<(), String> {
        if !self.remote_display {
            return Err("display import requires a remote display control".into());
        }
        update.validate()?;
        self.set_phase(update.phase);
        for (counter, value) in self.recorded.iter().zip(update.recorded_frames) {
            counter.store(value, Ordering::Relaxed);
        }
        for (counter, value) in self.queue_drops.iter().zip(update.processing_drops) {
            counter.store(value, Ordering::Relaxed);
        }
        if let Some(preview) = update.preview {
            let mut slot = self.preview.lock().unwrap_or_else(|e| e.into_inner());
            if slot.replace(preview).is_some() {
                self.remote_replacements.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.skipped.store(
            update
                .skipped_previews
                .saturating_add(self.remote_replacements.load(Ordering::Relaxed)),
            Ordering::Relaxed,
        );
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::live_stereo::control::Thumbnail;
    use scancraft_geometry::{Cloud, Point};
    use serde_json::Value;
    fn update() -> DisplayUpdate {
        DisplayUpdate {
            phase: Phase::Running,
            recorded_frames: [12, 13],
            processing_drops: [2, 3],
            skipped_previews: 4,
            preview: Some(Snapshot {
                pair_id: 7,
                cameras: [
                    Thumbnail {
                        width: 1,
                        height: 1,
                        pixels: vec![5],
                    },
                    Thumbnail {
                        width: 1,
                        height: 1,
                        pixels: vec![6],
                    },
                ],
                cloud: Cloud { points: vec![] },
                fused_points: 10,
                tracking: Value::Null,
            }),
        }
    }
    #[test]
    fn roundtrip_import_preserves_parent_commands_and_replaces_only_display_state() {
        let remote = Control::new_remote_display(true);
        remote.set_paused(true).unwrap();
        remote.request_stop();
        let serialized = serde_json::to_vec(&update()).unwrap();
        remote
            .apply_display_update(serde_json::from_slice(&serialized).unwrap())
            .unwrap();
        assert!(remote.pause_requested());
        assert!(remote.stop_requested());
        assert_eq!(remote.recorded_frames(), [12, 13]);
        assert_eq!(remote.processing_drops(), [2, 3]);
        assert_eq!(remote.phase(), Phase::Running);
        assert_eq!(remote.take_preview().unwrap().pair_id, 7);
        assert!(remote.begin().is_err());
    }
    #[test]
    fn invalid_update_cannot_partially_change_counters_or_mailbox() {
        let remote = Control::new_remote_display(true);
        for kind in 0..5 {
            let mut bad = update();
            let preview = bad.preview.as_mut().unwrap();
            match kind {
                0 => preview.cameras[0].width = usize::MAX,
                1 => preview.cameras[1].pixels.clear(),
                2 => preview.cameras[0].height = 0,
                3 => preview.cloud.points.push(Point {
                    position: [f64::NAN, 0., 0.],
                    color: None,
                    normal: None,
                }),
                _ => {
                    preview.fused_points = 0;
                    preview.cloud.points.push(Point {
                        position: [0.; 3],
                        color: None,
                        normal: None,
                    });
                }
            }
            assert!(remote.apply_display_update(bad).is_err());
            assert_eq!(remote.recorded_frames(), [0, 0]);
            assert!(remote.take_preview().is_none());
            assert_eq!(remote.phase(), Phase::Initializing);
        }
        let mut large = update();
        let preview = large.preview.as_mut().unwrap();
        preview.cloud.points = vec![
            Point {
                position: [0.; 3],
                color: None,
                normal: None
            };
            50_001
        ];
        preview.fused_points = 50_001;
        assert!(remote.apply_display_update(large).is_err());
    }
    #[test]
    fn local_runner_controls_reject_import_and_transport_replacements_are_counted() {
        let local = Control::new(true);
        assert!(local.apply_display_update(update()).is_err());
        assert_eq!(local.recorded_frames(), [0, 0]);
        local.begin().unwrap();
        let remote = Control::new_remote_display(true);
        remote.apply_display_update(update()).unwrap();
        remote.apply_display_update(update()).unwrap();
        assert_eq!(remote.skipped_previews(), 5);
        assert!(remote.take_preview().is_some());
        assert!(remote.take_preview().is_none());
        let mut value = serde_json::to_value(update()).unwrap();
        value["paused"] = Value::Bool(false);
        assert!(serde_json::from_value::<DisplayUpdate>(value).is_err());
    }
}
