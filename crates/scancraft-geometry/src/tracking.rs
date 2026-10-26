//! Bounded CPU frame-to-frame tracking prototype. All coordinates are millimeters.
//! Local ICP quality is not evidence of globally correct or physical registration.
use crate::{
    registration::{Rigid, icp_with, rigid_fit},
    work::WorkControl,
};

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackingConfig {
    pub max_correspondence_mm: f64,
    pub max_rms_mm: f64,
    /// Both source inlier coverage and distinct target coverage must reach this fraction.
    pub minimum_overlap: f64,
    pub max_step_translation_mm: f64,
    pub max_step_rotation_rad: f64,
    pub iterations: usize,
    pub tolerance_mm: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RegistrationMethod {
    #[default]
    PointToPoint,
    PointToPlane,
    PointToPlaneHuber {
        delta_mm: f64,
    },
}
impl RegistrationMethod {
    pub fn requires_normals(self) -> bool {
        !matches!(self, Self::PointToPoint)
    }
    fn loss(self) -> crate::point_plane::Loss {
        match self {
            Self::PointToPlaneHuber { delta_mm } => crate::point_plane::Loss::Huber { delta_mm },
            _ => crate::point_plane::Loss::Quadratic,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackingState {
    Empty,
    Tracking,
    Lost,
}
#[derive(Clone, Debug)]
pub struct TrackingUpdate {
    pub frame_id: u64,
    pub state: TrackingState,
    pub accepted: bool,
    pub reason: String,
    /// None on a rejected frame: the previous pose is never presented as a new observation.
    pub world_from_frame: Option<Rigid>,
    pub rms_mm: Option<f64>,
    pub source_overlap: f64,
    pub target_overlap: f64,
}
#[derive(Clone)]
pub struct Tracker {
    config: TrackingConfig,
    method: RegistrationMethod,
    reference_normals: Vec<[f64; 3]>,
    state: TrackingState,
    reference: Vec<[f64; 3]>,
    world_from_reference: Rigid,
    last_attempt: Option<u64>,
    last_accepted: Option<u64>,
}
impl Tracker {
    pub fn new(config: TrackingConfig) -> Result<Self, String> {
        Self::new_with_method(config, RegistrationMethod::PointToPoint)
    }
    pub fn method(&self) -> RegistrationMethod {
        self.method
    }
    pub fn new_with_method(
        config: TrackingConfig,
        method: RegistrationMethod,
    ) -> Result<Self, String> {
        method.loss().validate()?;
        if ![
            config.max_correspondence_mm,
            config.max_rms_mm,
            config.minimum_overlap,
            config.max_step_translation_mm,
            config.max_step_rotation_rad,
            config.tolerance_mm,
        ]
        .iter()
        .all(|v| v.is_finite() && *v > 0.)
            || config.minimum_overlap > 1.
            || config.max_rms_mm > config.max_correspondence_mm
            || config.max_step_rotation_rad > std::f64::consts::PI
            || !(2..=90).contains(&config.iterations)
        {
            return Err("invalid tracking quality gates".into());
        }
        Ok(Self {
            config,
            method,
            reference_normals: Vec::new(),
            state: TrackingState::Empty,
            reference: Vec::new(),
            world_from_reference: Rigid::identity(),
            last_attempt: None,
            last_accepted: None,
        })
    }
    pub fn state(&self) -> TrackingState {
        self.state
    }
    pub fn last_accepted(&self) -> Option<u64> {
        self.last_accepted
    }
    /// Re-anchor deliberately; callers must treat the next frame as a new world/session.
    pub fn reset(&mut self) {
        self.reference.clear();
        self.reference_normals.clear();
        self.world_from_reference = Rigid::identity();
        self.last_attempt = None;
        self.last_accepted = None;
        self.state = TrackingState::Empty;
    }
    /// Input is a caller-selected bounded tracking cloud (3..=512 finite points).
    /// Rejected attempts retain the last accepted reference and world pose. Later
    /// frames can recover against that reference; there is no silent re-anchoring.
    /// Errors/cancellation leave all tracker state unchanged.
    pub fn track(
        &mut self,
        frame_id: u64,
        points: &[[f64; 3]],
        work: &WorkControl,
    ) -> Result<TrackingUpdate, String> {
        self.track_inner(frame_id, points, None, work)
    }
    pub fn track_with_normals(
        &mut self,
        frame_id: u64,
        points: &[[f64; 3]],
        normals: &[[f64; 3]],
        work: &WorkControl,
    ) -> Result<TrackingUpdate, String> {
        self.track_inner(frame_id, points, Some(normals), work)
    }
    fn track_inner(
        &mut self,
        frame_id: u64,
        points: &[[f64; 3]],
        normals: Option<&[[f64; 3]]>,
        work: &WorkControl,
    ) -> Result<TrackingUpdate, String> {
        work.check()?;
        if self.method.requires_normals() && (points.len() < 6 || normals.is_none()) {
            return Err(
                "point-to-plane tracking requires at least six points and supplied normals".into(),
            );
        }
        if let Some(normals) = normals
            && (points.len() > 512
                || normals.len() != points.len()
                || normals.iter().any(|n| {
                    let length = n[0].hypot(n[1]).hypot(n[2]);
                    !length.is_finite() || length <= 0.
                }))
        {
            return Err(
                "tracking normals must match the bounded points and be finite and nonzero".into(),
            );
        }
        if !(3..=512).contains(&points.len()) || self.last_attempt.is_some_and(|id| frame_id <= id)
        {
            return Err(
                "tracking requires 3..=512 points and strictly increasing frame IDs".into(),
            );
        }
        // Reject malformed/collinear clouds as caller errors before touching state.
        rigid_fit(points, points)?;
        work.check()?;
        if self.state == TrackingState::Empty {
            self.reference = points.to_vec();
            self.reference_normals = normals.unwrap_or_default().to_vec();
            self.last_attempt = Some(frame_id);
            self.last_accepted = Some(frame_id);
            self.state = TrackingState::Tracking;
            return Ok(TrackingUpdate {
                frame_id,
                state: self.state,
                accepted: true,
                reason: "new session anchor".into(),
                world_from_frame: Some(Rigid::identity()),
                rms_mm: Some(0.),
                source_overlap: 1.,
                target_overlap: 1.,
            });
        }
        let c = self.config;
        let fit = match self.method {
            RegistrationMethod::PointToPoint => icp_with(
                points,
                &self.reference,
                Rigid::identity(),
                c.max_correspondence_mm,
                c.iterations,
                c.tolerance_mm,
                work,
            )
            .map(|r| (r.transform, r.converged)),
            RegistrationMethod::PointToPlane | RegistrationMethod::PointToPlaneHuber { .. } => {
                crate::point_plane::icp_with_loss(
                    points,
                    &self.reference,
                    &self.reference_normals,
                    Rigid::identity(),
                    crate::point_plane::Config {
                        maximum_distance_mm: c.max_correspondence_mm,
                        maximum_iterations: c.iterations,
                        tolerance_mm: c.tolerance_mm,
                        maximum_rotation_step_rad: c.max_step_rotation_rad.min(0.5),
                    },
                    self.method.loss(),
                    work,
                )
                .map(|r| (r.transform, r.converged))
            }
        };
        work.check()?;
        let mut update = TrackingUpdate {
            frame_id,
            state: TrackingState::Lost,
            accepted: false,
            reason: String::new(),
            world_from_frame: None,
            rms_mm: None,
            source_overlap: 0.,
            target_overlap: 0.,
        };
        match fit {
            Err(reason) => update.reason = reason,
            Ok((fit_transform, fit_converged)) => {
                let index = crate::spatial::SpatialIndex::build(&self.reference, work)?;
                let mut used = vec![false; self.reference.len()];
                let mut inliers = 0usize;
                let mut sum = 0.;
                for point in points {
                    work.check()?;
                    let p = fit_transform.apply(*point);
                    if let Some(neighbor) = index.nearest(p, work)?
                        && neighbor.distance_mm <= c.max_correspondence_mm
                    {
                        used[neighbor.index] = true;
                        inliers += 1;
                        sum += neighbor.distance_mm * neighbor.distance_mm;
                    }
                }
                update.source_overlap = inliers as f64 / points.len() as f64;
                update.target_overlap =
                    used.iter().filter(|v| **v).count() as f64 / self.reference.len() as f64;
                let rms = if inliers > 0 {
                    (sum / inliers as f64).sqrt()
                } else {
                    f64::INFINITY
                };
                if rms.is_finite() {
                    update.rms_mm = Some(rms);
                }
                let angle = ((fit_transform.rotation.trace() - 1.) / 2.)
                    .clamp(-1., 1.)
                    .acos();
                update.reason = if !fit_converged {
                    "ICP did not converge"
                } else if !rms.is_finite() || rms > c.max_rms_mm {
                    "alignment residual exceeds gate"
                } else if update.source_overlap < c.minimum_overlap
                    || update.target_overlap < c.minimum_overlap
                {
                    "insufficient bidirectional coverage"
                } else if fit_transform.translation.norm() > c.max_step_translation_mm
                    || angle > c.max_step_rotation_rad
                {
                    "motion exceeds step gate"
                } else {
                    "alignment accepted"
                }
                .into();
                if update.reason == "alignment accepted" {
                    let pose = self.world_from_reference.compose(&fit_transform);
                    if !pose
                        .rotation
                        .iter()
                        .chain(pose.translation.iter())
                        .all(|v| v.is_finite())
                    {
                        return Err("world pose overflow".into());
                    }
                    update.accepted = true;
                    update.state = TrackingState::Tracking;
                    update.world_from_frame = Some(pose);
                }
            }
        }
        work.check()?;
        if let Some(pose) = &update.world_from_frame {
            self.world_from_reference = pose.clone();
            self.reference = points.to_vec();
            self.reference_normals = normals.unwrap_or_default().to_vec();
            self.last_accepted = Some(frame_id);
        }
        self.last_attempt = Some(frame_id);
        self.state = update.state;
        Ok(update)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{Matrix3, Vector3};
    fn config() -> TrackingConfig {
        TrackingConfig {
            max_correspondence_mm: 2.,
            max_rms_mm: 0.05,
            minimum_overlap: 0.8,
            max_step_translation_mm: 1.,
            max_step_rotation_rad: 0.05,
            iterations: 20,
            tolerance_mm: 1e-8,
        }
    }
    fn scene() -> Vec<[f64; 3]> {
        vec![
            [0., 0., 0.],
            [10., 0., 0.],
            [0., 20., 0.],
            [0., 0., 30.],
            [4., 5., 8.],
            [-7., 2., 4.],
        ]
    }
    fn pose(frame: u64) -> Rigid {
        let a = frame as f64 * 0.004;
        Rigid {
            rotation: Matrix3::new(a.cos(), -a.sin(), 0., a.sin(), a.cos(), 0., 0., 0., 1.),
            translation: Vector3::new(frame as f64 * 0.2, 0., 0.),
        }
    }
    fn observations(world: &[[f64; 3]], pose: &Rigid) -> Vec<[f64; 3]> {
        world
            .iter()
            .map(|p| (pose.rotation.transpose() * (Vector3::from(*p) - pose.translation)).into())
            .collect()
    }
    #[test]
    fn trajectory_composes_camera_to_world_and_recovers_without_reanchoring() {
        let mut tracker = Tracker::new(config()).unwrap();
        let world = scene();
        let work = WorkControl::default();
        for frame in 0..20 {
            let expected = pose(frame);
            let output = tracker
                .track(frame, &observations(&world, &expected), &work)
                .unwrap();
            assert!(output.accepted, "{}", output.reason);
            let actual = output.world_from_frame.unwrap();
            assert!((actual.translation - expected.translation).norm() < 1e-10);
            assert!((actual.rotation - expected.rotation).norm() < 1e-10);
            assert!(output.rms_mm.unwrap() < 1e-10);
        }
        let bad: Vec<_> = world.iter().map(|p| [p[0] + 1000., p[1], p[2]]).collect();
        let output = tracker.track(20, &bad, &work).unwrap();
        assert_eq!(tracker.state(), TrackingState::Lost);
        assert!(!output.accepted);
        assert!(output.world_from_frame.is_none());
        assert_eq!(tracker.last_accepted(), Some(19));
        let expected = pose(20);
        let output = tracker
            .track(21, &observations(&world, &expected), &work)
            .unwrap();
        assert!(output.accepted);
        assert_eq!(tracker.state(), TrackingState::Tracking);
        assert!(
            (output.world_from_frame.unwrap().translation - expected.translation).norm() < 1e-10
        );
    }
    #[test]
    fn quality_rejections_preserve_anchor_and_never_expose_a_new_pose() {
        let world = scene();
        let work = WorkControl::default();
        let mut tracker = Tracker::new(config()).unwrap();
        tracker.track(0, &world, &work).unwrap();
        let output = tracker.track(1, &world[..3], &work).unwrap();
        assert!(!output.accepted);
        assert!(output.reason.contains("coverage"));
        assert_eq!(output.target_overlap, 0.5);
        assert_eq!(tracker.last_accepted(), Some(0));
        let shifted: Vec<_> = world.iter().map(|p| [p[0] + 1.5, p[1], p[2]]).collect();
        let output = tracker.track(2, &shifted, &work).unwrap();
        assert!(!output.accepted);
        assert!(output.reason.contains("motion"));
        assert!(output.world_from_frame.is_none());
        let mut noisy = world.clone();
        noisy[0][0] += 0.5;
        let output = tracker.track(3, &noisy, &work).unwrap();
        assert!(!output.accepted);
        assert!(output.reason.contains("residual"));
        let output = tracker.track(4, &world, &work).unwrap();
        assert!(output.accepted);
        assert!(output.world_from_frame.unwrap().translation.norm() < 1e-10);
    }
    #[test]
    fn cancellation_bad_inputs_and_frame_order_leave_state_unchanged() {
        let mut tracker = Tracker::new(config()).unwrap();
        let world = scene();
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(tracker.track(0, &world, &cancelled).is_err());
        assert_eq!(tracker.state(), TrackingState::Empty);
        let work = WorkControl::default();
        tracker.track(0, &world, &work).unwrap();
        assert!(tracker.track(0, &world, &work).is_err());
        assert!(tracker.track(1, &[[0.; 3]; 3], &work).is_err());
        let mut invalid = world.clone();
        invalid[0][0] = f64::NAN;
        assert!(tracker.track(1, &invalid, &work).is_err());
        assert!(tracker.track(1, &world, &cancelled).is_err());
        assert_eq!(tracker.last_accepted(), Some(0));
        assert_eq!(tracker.state(), TrackingState::Tracking);
        assert!(tracker.track(1, &world, &work).unwrap().accepted);
        tracker.reset();
        assert_eq!(tracker.state(), TrackingState::Empty);
        assert_eq!(tracker.last_accepted(), None);
        assert!(tracker.track(0, &world, &work).unwrap().accepted);
        let mut c = config();
        c.minimum_overlap = f64::NAN;
        assert!(Tracker::new(c).is_err());
        let mut c = config();
        c.iterations = 91;
        assert!(Tracker::new(c).is_err());
    }
}
