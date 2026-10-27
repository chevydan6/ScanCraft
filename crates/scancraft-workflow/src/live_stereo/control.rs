//! Single-session cooperative commands and a one-slot display-only mailbox.
//! Device/CPU calls are not preempted, and previews never attest calibration.
use super::Pair;
use scancraft_core::StreamProfile;
use scancraft_geometry::Cloud;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub mod display;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Phase {
    Initializing,
    Running,
    Paused,
    Stopping,
    Finishing,
    Complete,
    Failed,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thumbnail {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub pair_id: usize,
    pub cameras: [Thumbnail; 2],
    pub cloud: Cloud,
    pub fused_points: usize,
    pub tracking: Value,
}
pub struct Control {
    remote_display: bool,
    remote_replacements: AtomicUsize,
    paused: AtomicBool,
    stop: AtomicBool,
    started: AtomicBool,
    phase: AtomicU8,
    pub(super) recorded: [AtomicUsize; 2],
    pub(super) queue_drops: [AtomicUsize; 2],
    preview_enabled: bool,
    preview: Mutex<Option<Snapshot>>,
    skipped: AtomicUsize,
}
impl Default for Control {
    fn default() -> Self {
        Self::new(false)
    }
}
impl Control {
    pub fn new(preview_enabled: bool) -> Self {
        Self {
            remote_display: false,
            remote_replacements: AtomicUsize::new(0),
            paused: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            started: AtomicBool::new(false),
            phase: AtomicU8::new(Phase::Initializing as u8),
            recorded: Default::default(),
            queue_drops: Default::default(),
            preview_enabled,
            preview: Mutex::new(None),
            skipped: AtomicUsize::new(0),
        }
    }
    pub fn set_paused(&self, paused: bool) -> Result<(), String> {
        if self.stop_requested()
            || matches!(
                self.phase(),
                Phase::Finishing | Phase::Complete | Phase::Failed
            )
        {
            return Err("scan is finishing or stopped".into());
        }
        self.paused.store(paused, Ordering::Relaxed);
        Ok(())
    }
    pub fn pause_requested(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
    /// Graceful stop closes capture, drains queued work, verifies and publishes.
    /// WorkControl cancellation instead aborts publication and retains partial data.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
    pub fn stop_requested(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
    pub fn phase(&self) -> Phase {
        match self.phase.load(Ordering::Relaxed) {
            0 => Phase::Initializing,
            1 => Phase::Running,
            2 => Phase::Paused,
            3 => Phase::Stopping,
            4 => Phase::Finishing,
            5 => Phase::Complete,
            _ => Phase::Failed,
        }
    }
    pub fn recorded_frames(&self) -> [usize; 2] {
        self.recorded.each_ref().map(|v| v.load(Ordering::Relaxed))
    }
    pub fn processing_drops(&self) -> [usize; 2] {
        self.queue_drops
            .each_ref()
            .map(|v| v.load(Ordering::Relaxed))
    }
    pub fn skipped_previews(&self) -> usize {
        self.skipped.load(Ordering::Relaxed)
    }
    /// The private lock protects only Option take/replace; no rendering or callbacks run under it.
    pub fn take_preview(&self) -> Option<Snapshot> {
        self.preview
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
    pub(super) fn begin(&self) -> Result<(), String> {
        if self.remote_display {
            return Err("remote display control cannot run a local scan".into());
        }
        self.started
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .map_err(|_| "live control is single-use".to_string())?;
        Ok(())
    }
    pub(super) fn set_phase(&self, phase: Phase) {
        self.phase.store(phase as u8, Ordering::Relaxed);
    }
    pub(super) fn preview_enabled(&self) -> bool {
        self.preview_enabled
    }
    pub(super) fn wants_preview(&self) -> bool {
        if !self.preview_enabled {
            return false;
        }
        let empty = self
            .preview
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_none();
        if !empty {
            self.skipped.fetch_add(1, Ordering::Relaxed);
        }
        empty
    }
    pub(super) fn offer(&self, snapshot: Snapshot) {
        *self.preview.lock().unwrap_or_else(|e| e.into_inner()) = Some(snapshot);
    }
}
fn thumbnail(profile: &StreamProfile, pixels: &[u8]) -> Thumbnail {
    let width = profile.width as usize;
    let height = profile.height as usize;
    let stride = width.div_ceil(320).max(height.div_ceil(240)).max(1);
    let out_width = width.div_ceil(stride);
    let out_height = height.div_ceil(stride);
    let mut output = Vec::with_capacity(out_width * out_height);
    for y in 0..out_height {
        for x in 0..out_width {
            output.push(pixels[y * stride * width + x * stride]);
        }
    }
    Thumbnail {
        width: out_width,
        height: out_height,
        pixels: output,
    }
}
pub(super) fn snapshot(
    pair: &Pair,
    profiles: &[StreamProfile; 2],
    pair_id: usize,
    cloud: Cloud,
    fused_points: usize,
    tracking: Value,
) -> Snapshot {
    Snapshot {
        pair_id,
        cameras: [
            thumbnail(&profiles[0], &pair.left.frame.data),
            thumbnail(&profiles[1], &pair.right.frame.data),
        ],
        cloud,
        fused_points,
        tracking,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(id: usize) -> Snapshot {
        Snapshot {
            pair_id: id,
            cameras: [
                Thumbnail {
                    width: 1,
                    height: 1,
                    pixels: vec![3],
                },
                Thumbnail {
                    width: 1,
                    height: 1,
                    pixels: vec![4],
                },
            ],
            cloud: Cloud { points: vec![] },
            fused_points: 0,
            tracking: Value::Null,
        }
    }
    #[test]
    fn commands_are_single_session_and_mailbox_is_bounded() {
        let c = Control::new(true);
        c.begin().unwrap();
        assert!(c.begin().is_err());
        c.set_paused(true).unwrap();
        assert!(c.pause_requested());
        c.set_paused(false).unwrap();
        assert!(c.wants_preview());
        c.offer(sample(0));
        assert!(!c.wants_preview());
        assert_eq!(c.skipped_previews(), 1);
        c.offer(sample(1));
        assert_eq!(c.take_preview().unwrap().pair_id, 1);
        assert!(c.take_preview().is_none());
        c.request_stop();
        assert!(c.set_paused(false).is_err());
        c.set_phase(Phase::Complete);
        assert_eq!(c.phase(), Phase::Complete);
    }
    #[test]
    fn thumbnail_caps_preserve_selected_raw_samples_and_source() {
        let profile = StreamProfile {
            interface: 0,
            width: 1920,
            height: 1200,
            fourcc: *b"GREY",
            fps: 30,
        };
        let source: Vec<_> = (0..1920 * 1200).map(|i| (i % 251) as u8).collect();
        let original = source.clone();
        let image = thumbnail(&profile, &source);
        assert_eq!([image.width, image.height], [320, 200]);
        assert_eq!(image.pixels.len(), 64000);
        for y in 0..200 {
            for x in 0..320 {
                assert_eq!(image.pixels[y * 320 + x], source[y * 6 * 1920 + x * 6]);
            }
        }
        assert_eq!(source, original);
    }
}
