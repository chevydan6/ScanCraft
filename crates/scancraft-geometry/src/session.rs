//! Atomic tracking/fusion publication and bounded in-memory session lifecycle.
use crate::{
    Cloud,
    fusion::{FusionUpdate, VoxelFusion},
    tracking::{RegistrationMethod, Tracker, TrackingConfig, TrackingState, TrackingUpdate},
    work::WorkControl,
};
use std::collections::HashSet;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Ready,
    Running,
    Paused,
    Stopped,
}
#[derive(Debug, Clone)]
pub struct SessionFrame {
    pub tracking: TrackingUpdate,
    /// None for a rejected alignment. Failed integration never publishes a record.
    pub fusion: Option<FusionUpdate>,
}
pub struct ScanSession {
    id: String,
    state: SessionState,
    tracker: Tracker,
    fusion: VoxelFusion,
    history: Vec<SessionFrame>,
    maximum_frames: usize,
}
impl ScanSession {
    pub fn new(
        id: String,
        tracking: TrackingConfig,
        voxel_mm: f64,
        maximum_voxels: usize,
        maximum_frames: usize,
    ) -> Result<Self, String> {
        Self::new_with_method(
            id,
            tracking,
            voxel_mm,
            maximum_voxels,
            maximum_frames,
            RegistrationMethod::PointToPoint,
        )
    }
    pub fn new_with_method(
        id: String,
        tracking: TrackingConfig,
        voxel_mm: f64,
        maximum_voxels: usize,
        maximum_frames: usize,
        method: RegistrationMethod,
    ) -> Result<Self, String> {
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_ .".contains(&b))
            || !(1..=100_000).contains(&maximum_frames)
        {
            return Err("invalid session ID or frame history capacity".into());
        }
        Ok(Self {
            id,
            state: SessionState::Ready,
            tracker: Tracker::new_with_method(tracking, method)?,
            fusion: VoxelFusion::new(voxel_mm, maximum_voxels)?,
            history: Vec::new(),
            maximum_frames,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn state(&self) -> SessionState {
        self.state
    }
    pub fn tracking_state(&self) -> TrackingState {
        self.tracker.state()
    }
    pub fn history(&self) -> &[SessionFrame] {
        &self.history
    }
    pub fn last_accepted(&self) -> Option<u64> {
        self.tracker.last_accepted()
    }
    pub fn last_integrated(&self) -> Option<u64> {
        self.fusion.last_frame()
    }
    pub fn cloud(&self, work: &WorkControl) -> Result<Cloud, String> {
        self.fusion.cloud(work)
    }
    pub fn start(&mut self) -> Result<(), String> {
        if self.state != SessionState::Ready {
            return Err("session can only start from Ready".into());
        }
        self.state = SessionState::Running;
        Ok(())
    }
    pub fn pause(&mut self) -> Result<(), String> {
        if self.state != SessionState::Running {
            return Err("only a running session can pause".into());
        }
        self.state = SessionState::Paused;
        Ok(())
    }
    pub fn resume(&mut self) -> Result<(), String> {
        if self.state != SessionState::Paused {
            return Err("only a paused session can resume".into());
        }
        self.state = SessionState::Running;
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), String> {
        if self.state == SessionState::Stopped {
            return Err("session is already stopped".into());
        }
        self.state = SessionState::Stopped;
        Ok(())
    }
    /// Tracking uses explicit unique source indices, while fusion receives the
    /// complete cloud. No automatic dropping, re-anchoring or history eviction.
    /// Tracker and history publish only after fusion has committed successfully.
    pub fn process(
        &mut self,
        frame_id: u64,
        cloud: &Cloud,
        tracking_indices: &[usize],
        work: &WorkControl,
    ) -> Result<&SessionFrame, String> {
        work.check()?;
        if self.state != SessionState::Running {
            return Err("session must be running to process frames".into());
        }
        if self.history.len() == self.maximum_frames {
            return Err("session frame history capacity exceeded".into());
        }
        if cloud.points.is_empty()
            || cloud.points.len() > 100_000
            || !(3..=512).contains(&tracking_indices.len())
        {
            return Err(
                "session requires 1..=100000 cloud points and 3..=512 tracking samples".into(),
            );
        }
        cloud.validate_with(work)?;
        let mut seen = HashSet::new();
        let mut samples = Vec::with_capacity(tracking_indices.len());
        for index in tracking_indices {
            if !seen.insert(*index) {
                return Err("duplicate tracking sample index".into());
            }
            samples.push(
                cloud
                    .points
                    .get(*index)
                    .ok_or("tracking sample index out of bounds")?
                    .position,
            );
        }
        // Small bounded tracker copy; never clone the potentially large fusion map.
        let mut candidate = self.tracker.clone();
        let tracking = if candidate.method().requires_normals() {
            let normals: Vec<_> = tracking_indices
                .iter()
                .map(|i| {
                    cloud.points[*i]
                        .normal
                        .ok_or("point-to-plane tracking sample is missing a normal")
                })
                .collect::<Result<_, _>>()?;
            candidate.track_with_normals(frame_id, &samples, &normals, work)?
        } else {
            candidate.track(frame_id, &samples, work)?
        };
        let fusion = if tracking.accepted {
            Some(self.fusion.integrate_tracked(cloud, &tracking, work)?)
        } else {
            work.check()?;
            None
        };
        // No cancellation/error boundary after a successful fusion commit.
        self.tracker = candidate;
        self.history.push(SessionFrame { tracking, fusion });
        Ok(self.history.last().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;
    fn config() -> TrackingConfig {
        TrackingConfig {
            max_correspondence_mm: 2.,
            max_rms_mm: 0.01,
            minimum_overlap: 0.8,
            max_step_translation_mm: 1.,
            max_step_rotation_rad: 0.1,
            iterations: 20,
            tolerance_mm: 1e-8,
        }
    }
    fn cloud(shift: f64) -> Cloud {
        Cloud {
            points: [[0.25, 0.25, 0.25], [10.25, 0.25, 0.25], [0.25, 20.25, 0.25]]
                .into_iter()
                .map(|p| Point {
                    position: [p[0] + shift, p[1], p[2]],
                    color: None,
                    normal: None,
                })
                .collect(),
        }
    }
    #[test]
    fn fusion_failure_rolls_back_tracking_reference_history_and_frame_consumption() {
        let work = WorkControl::default();
        let mut s = ScanSession::new("trial".into(), config(), 1., 3, 10).unwrap();
        s.start().unwrap();
        s.process(0, &cloud(0.), &[0, 1, 2], &work).unwrap();
        let before = s.cloud(&work).unwrap();
        let mut large = cloud(0.2);
        large.points.push(Point {
            position: [100.; 3],
            color: None,
            normal: None,
        });
        assert!(
            s.process(1, &large, &[0, 1, 2], &work)
                .unwrap_err()
                .contains("capacity")
        );
        assert_eq!(s.last_accepted(), Some(0));
        assert_eq!(s.last_integrated(), Some(0));
        assert_eq!(s.history().len(), 1);
        assert_eq!(s.cloud(&work).unwrap(), before);
        let result = s.process(1, &cloud(0.2), &[0, 1, 2], &work).unwrap();
        assert!(result.tracking.accepted);
        assert!(result.fusion.is_some());
        assert!(
            (result
                .tracking
                .world_from_frame
                .as_ref()
                .unwrap()
                .translation[0]
                + 0.2)
                .abs()
                < 1e-10
        );
        assert_eq!(s.last_accepted(), s.last_integrated());
    }
    #[test]
    fn lifecycle_loss_recovery_and_history_limit_are_explicit() {
        let work = WorkControl::default();
        let mut s = ScanSession::new("session-1".into(), config(), 1., 10, 3).unwrap();
        assert!(s.process(0, &cloud(0.), &[0, 1, 2], &work).is_err());
        assert!(s.resume().is_err());
        s.start().unwrap();
        assert!(s.start().is_err());
        s.process(0, &cloud(0.), &[0, 1, 2], &work).unwrap();
        s.pause().unwrap();
        assert!(s.process(1, &cloud(0.1), &[0, 1, 2], &work).is_err());
        assert!(s.pause().is_err());
        s.resume().unwrap();
        let lost = s.process(1, &cloud(1000.), &[0, 1, 2], &work).unwrap();
        assert!(!lost.tracking.accepted);
        assert!(lost.fusion.is_none());
        assert_eq!(s.tracking_state(), TrackingState::Lost);
        assert_eq!(s.last_integrated(), Some(0));
        assert!(
            s.process(2, &cloud(0.1), &[0, 1, 2], &work)
                .unwrap()
                .tracking
                .accepted
        );
        assert_eq!(s.last_integrated(), Some(2));
        assert!(
            s.process(3, &cloud(0.2), &[0, 1, 2], &work)
                .unwrap_err()
                .contains("history capacity")
        );
        assert_eq!(s.history().len(), 3);
        s.stop().unwrap();
        assert!(s.resume().is_err());
        assert!(s.stop().is_err());
        assert_eq!(s.cloud(&work).unwrap().points.len(), 3);
    }
    #[test]
    fn invalid_sampling_and_cancelled_processing_never_publish_state() {
        let work = WorkControl::default();
        let mut s = ScanSession::new("trial".into(), config(), 1., 10, 10).unwrap();
        s.start().unwrap();
        for indices in [&[0, 0, 2][..], &[0, 1, 3][..], &[0, 1][..]] {
            assert!(s.process(0, &cloud(0.), indices, &work).is_err());
        }
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(s.process(0, &cloud(0.), &[0, 1, 2], &cancelled).is_err());
        assert!(s.history().is_empty());
        assert_eq!(s.tracking_state(), TrackingState::Empty);
        assert!(s.cloud(&work).unwrap().points.is_empty());
        assert!(
            s.process(0, &cloud(0.), &[0, 1, 2], &work)
                .unwrap()
                .tracking
                .accepted
        );
    }
}
