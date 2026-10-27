//! Fixed-intrinsic stereo extrinsics and target poses from supplied correspondences.
//!
//! Independent reprojection least squares with shared native Brown derivatives
//! and Schur elimination. Comparable numerical scope to OpenCV stereoCalibrate
//! with CALIB_FIX_INTRINSIC, not copied OpenCV source or hardware qualification.
use crate::{
    CameraModel,
    normal_blocks::{
        NormalSystem, PoseVector, ViewNormal, information_min_eigenvalue, scaled_gradient_inf,
        solve_step,
    },
    planar::{
        InitialIntrinsics, PlanarObservation, PlanarView, homography, recover_pose, validate_view,
    },
    projection_differential::{point_pose_jacobian, project, step_pose},
    stereo::{RigidPose, StereoCalibration},
};
use nalgebra::{Matrix3, SMatrix, SVector, Vector3, linalg::SVD};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
type PairResidual = SVector<f64, 4>;
type PoseJacobian = SMatrix<f64, 4, 6>;

/// One physical target location declared visible in both supplied images.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StereoObservation {
    pub target_xy_mm: [f64; 2],
    pub left_pixel: [f64; 2],
    pub right_pixel: [f64; 2],
}
/// One declared common target pose; neither exposure simultaneity nor identity is attested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StereoView {
    pub observations: Vec<StereoObservation>,
}
/// Numerical termination/acceptance policies, never physical accuracy thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StereoFitOptions {
    pub max_iterations: usize,
    /// Final maximum individual 2D Euclidean residual, in either camera.
    pub max_reprojection_error_px: f64,
}
impl Default for StereoFitOptions {
    fn default() -> Self {
        Self {
            max_iterations: 80,
            max_reprojection_error_px: 1.,
        }
    }
}
/// One jointly optimized target pose in left-camera axes and residual diagnostics.
#[derive(Debug, Clone, Serialize)]
pub struct StereoViewFit {
    pub left_from_target: RigidPose,
    pub left_rms_px: f64,
    pub right_rms_px: f64,
    pub left_max_error_px: f64,
    pub right_max_error_px: f64,
}
/// Diagnostic fit; no capture-record creation, device binding or lens qualification.
#[derive(Debug, Clone, Serialize)]
pub struct StereoExtrinsicFit {
    pub fixed_left: CameraModel,
    pub fixed_right: CameraModel,
    /// p_right=R*p_left+t_mm, consistent with existing triangulation.
    pub right_from_left: RigidPose,
    pub views: Vec<StereoViewFit>,
    pub initial_rms_px: f64,
    /// RMS per 2D camera observation: sqrt(sum(left²+right²)/(2*target_points)).
    pub rms_px: f64,
    pub max_error_px: f64,
    pub accepted_iterations: usize,
    pub scaled_gradient_inf: f64,
    /// Scaled pose-eliminated information eigenvalue, not uncertainty/covariance.
    pub scaled_information_min_eigenvalue: f64,
}
#[derive(Clone)]
struct State {
    rig: RigidPose,
    poses: Vec<RigidPose>,
}
impl State {
    fn stepped(&self, rig: PoseVector, poses: &[PoseVector]) -> Result<Self, String> {
        Ok(Self {
            rig: step_pose(&self.rig, rig)?,
            poses: self
                .poses
                .iter()
                .zip(poses)
                .map(|(pose, step)| step_pose(pose, *step))
                .collect::<Result<_, _>>()?,
        })
    }
}
struct Cameras<'a> {
    models: [&'a CameraModel; 2],
    intrinsics: [InitialIntrinsics; 2],
}
impl<'a> Cameras<'a> {
    fn new(left: &'a CameraModel, right: &'a CameraModel) -> Self {
        Self {
            models: [left, right],
            intrinsics: [left, right].map(|c| InitialIntrinsics {
                fx: c.fx,
                fy: c.fy,
                cx: c.cx,
                cy: c.cy,
                skew: c.skew,
            }),
        }
    }
}
struct Score {
    squared: f64,
    maximum: f64,
    points: usize,
    per_view: Vec<([f64; 2], [f64; 2])>,
}
fn check(cancel: &dyn Fn() -> bool) -> Result<(), String> {
    if cancel() {
        Err("stereo calibration cancelled".into())
    } else {
        Ok(())
    }
}

/// Jointly fit one right-from-left pose and each target pose with fixed lenses.
///
/// Requires 3..=64 declared common target poses with 4..=4096 matched points each.
/// Target coordinates must share metric meaning between cameras; incorrect labels,
/// size or synchronization cannot be certified by this numerical fit. Seeds are
/// derived from undistorted homographies in each known camera and an SO(3) mean.
/// No robust loss, automatic outlier removal, intrinsic fitting or installation.
pub fn fit_stereo_extrinsics(
    left: &CameraModel,
    right: &CameraModel,
    views: &[StereoView],
    options: &StereoFitOptions,
    cancel: &dyn Fn() -> bool,
) -> Result<StereoExtrinsicFit, String> {
    let requested = Cell::new(false);
    let latch = || {
        let stop = requested.get() || cancel();
        requested.set(stop);
        stop
    };
    let cancel: &dyn Fn() -> bool = &latch;
    check(cancel)?;
    left.validate()?;
    right.validate()?;
    if [left.width, left.height, right.width, right.height]
        .iter()
        .any(|&v| v > 1_000_000)
        || !(3..=64).contains(&views.len())
        || !(1..=100).contains(&options.max_iterations)
        || !options.max_reprojection_error_px.is_finite()
        || options.max_reprojection_error_px <= 0.
    {
        return Err("invalid fixed-intrinsic stereo fitting bounds/options".into());
    }
    let cameras = Cameras::new(left, right);
    let mut state = initialize(left, right, views, cancel)?;
    let mut score = evaluate(&cameras, &state, views, cancel)?;
    let initial_rms = (score.squared / (2 * score.points) as f64).sqrt();
    let mut damping = 1e-3;
    let mut converged = false;
    let mut accepted = 0;
    for _ in 0..options.max_iterations {
        check(cancel)?;
        let normal = build_normal(&cameras, &state, views, cancel)?;
        if scaled_gradient_inf(&normal) < 1e-8 {
            converged = true;
            break;
        }
        let mut improved = None;
        for _ in 0..16 {
            check(cancel)?;
            if let Ok((rig, poses)) = solve_step(&normal, damping)
                && let Ok(candidate) = state.stepped(rig, &poses)
            {
                check(cancel)?;
                match evaluate(&cameras, &candidate, views, cancel) {
                    Ok(trial) if trial.squared < score.squared => {
                        let mut energy = rig.dot(&normal.global.diagonal().component_mul(&rig));
                        for (view, step) in normal.views.iter().zip(&poses) {
                            energy += step.dot(&view.pose.diagonal().component_mul(step));
                        }
                        improved = Some((
                            candidate,
                            trial,
                            (energy / (2 * score.points) as f64).sqrt(),
                        ));
                        break;
                    }
                    _ => check(cancel)?,
                }
            }
            damping *= 10.;
            if !damping.is_finite() || damping > 1e16 {
                break;
            }
        }
        let Some((trial, trial_score, step)) = improved else {
            return Err("stereo fit could not find a decreasing finite step".into());
        };
        let improvement = score.squared - trial_score.squared;
        let old = score.squared;
        state = trial;
        score = trial_score;
        accepted += 1;
        damping = (damping / 3.).max(1e-12);
        if damping <= 1. && (step < 1e-8 || (improvement <= 1e-12 * old.max(1.) && step < 1e-6)) {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err("stereo fit iteration limit exceeded".into());
    }
    if score.maximum > options.max_reprojection_error_px {
        return Err("stereo fit exceeds final reprojection limit".into());
    }
    let normal = build_normal(&cameras, &state, views, cancel)?;
    let gradient = scaled_gradient_inf(&normal);
    if !gradient.is_finite() || gradient > 1e-6 {
        return Err("stereo fit stalled before numerical stationarity".into());
    }
    let information = information_min_eigenvalue(&normal)?;
    StereoCalibration {
        left: left.clone(),
        right: right.clone(),
        right_from_left: state.rig.clone(),
    }
    .validate()?;
    check(cancel)?;
    Ok(StereoExtrinsicFit {
        fixed_left: left.clone(),
        fixed_right: right.clone(),
        right_from_left: state.rig,
        views: state
            .poses
            .into_iter()
            .zip(score.per_view)
            .zip(views)
            .map(|((pose, (s, m)), view)| StereoViewFit {
                left_from_target: pose,
                left_rms_px: (s[0] / view.observations.len() as f64).sqrt(),
                right_rms_px: (s[1] / view.observations.len() as f64).sqrt(),
                left_max_error_px: m[0],
                right_max_error_px: m[1],
            })
            .collect(),
        initial_rms_px: initial_rms,
        rms_px: (score.squared / (2 * score.points) as f64).sqrt(),
        max_error_px: score.maximum,
        accepted_iterations: accepted,
        scaled_gradient_inf: gradient,
        scaled_information_min_eigenvalue: information,
    })
}

/// Estimate target pose from already validated known-lens observations.
fn pose_seed(
    camera: &CameraModel,
    view: &PlanarView,
    cancel: &dyn Fn() -> bool,
) -> Result<RigidPose, String> {
    let target: Vec<_> = view.observations.iter().map(|p| p.target_xy_mm).collect();
    let mut normalized = Vec::with_capacity(target.len());
    for (i, p) in view.observations.iter().enumerate() {
        if i % 256 == 0 {
            check(cancel)?;
        }
        let ray = camera.ray_validated(p.pixel)?;
        normalized.push([ray[0] / ray[2], ray[1] / ray[2]]);
    }
    recover_pose(homography(&target, &normalized)?, target[0])
}
/// Seed each known-camera pose, then project the average relative rotation to SO(3).
fn initialize(
    left: &CameraModel,
    right: &CameraModel,
    views: &[StereoView],
    cancel: &dyn Fn() -> bool,
) -> Result<State, String> {
    let mut poses = Vec::with_capacity(views.len());
    let mut rotation = Matrix3::zeros();
    let mut translation = Vector3::zeros();
    for view in views {
        check(cancel)?;
        // Check count before any observation-sized allocation.
        if !(4..=4096).contains(&view.observations.len()) {
            return Err("expected 4..=4096 stereo target observations".into());
        }
        let l = PlanarView {
            observations: view
                .observations
                .iter()
                .map(|p| PlanarObservation {
                    target_xy_mm: p.target_xy_mm,
                    pixel: p.left_pixel,
                })
                .collect(),
        };
        let r = PlanarView {
            observations: view
                .observations
                .iter()
                .map(|p| PlanarObservation {
                    target_xy_mm: p.target_xy_mm,
                    pixel: p.right_pixel,
                })
                .collect(),
        };
        validate_view(&l, [left.width, left.height])?;
        validate_view(&r, [right.width, right.height])?;
        let lp = pose_seed(left, &l, cancel)?;
        let rp = pose_seed(right, &r, cancel)?;
        let relative = rp.matrix() * lp.matrix().transpose();
        rotation += relative / views.len() as f64;
        translation += (Vector3::from(rp.translation_mm)
            - relative * Vector3::from(lp.translation_mm))
            / views.len() as f64;
        poses.push(lp);
    }
    let svd = SVD::try_new(rotation, true, true, 1e-14, 1000)
        .ok_or("stereo rotation mean did not converge")?;
    let u = svd.u.ok_or("missing mean rotation U")?;
    let vt = svd.v_t.ok_or("missing mean rotation Vt")?;
    let mut sign = Matrix3::identity();
    sign[(2, 2)] = (u * vt).determinant().signum();
    let r = u * sign * vt;
    let rig = RigidPose {
        rotation: std::array::from_fn(|i| std::array::from_fn(|j| r[(i, j)])),
        translation_mm: translation.into(),
    };
    rig.validate()?;
    Ok(State { rig, poses })
}
/// Left/right residual derivatives for one shared rig and one independent target pose.
fn rows(
    cameras: &Cameras<'_>,
    state: &State,
    pose: &RigidPose,
    p: &StereoObservation,
) -> Result<(PairResidual, PoseJacobian, PoseJacobian), String> {
    let rotated = pose.matrix() * Vector3::new(p.target_xy_mm[0], p.target_xy_mm[1], 0.);
    let left = rotated + Vector3::from(pose.translation_mm);
    let rig_rotated = state.rig.matrix() * left;
    let right = rig_rotated + Vector3::from(state.rig.translation_mm);
    let l = project(&cameras.intrinsics[0], cameras.models[0].distortion, left)?;
    let r = project(&cameras.intrinsics[1], cameras.models[1].distortion, right)?;
    for (projection, camera) in [&l, &r].into_iter().zip(cameras.models) {
        if projection.normalized[0].hypot(projection.normalized[1]) > camera.valid_radius {
            return Err("stereo target outside supplied lens domain".into());
        }
    }
    let mut rig = SMatrix::<f64, 4, 6>::zeros();
    rig.fixed_view_mut::<2, 6>(2, 0)
        .copy_from(&(r.point_jacobian * point_pose_jacobian(rig_rotated)));
    let mut target = SMatrix::<f64, 4, 6>::zeros();
    let point_target = point_pose_jacobian(rotated);
    target
        .fixed_view_mut::<2, 6>(0, 0)
        .copy_from(&(l.point_jacobian * point_target));
    target
        .fixed_view_mut::<2, 6>(2, 0)
        .copy_from(&(r.point_jacobian * state.rig.matrix() * point_target));
    let residual = SVector::<f64, 4>::new(
        l.pixel[0] - p.left_pixel[0],
        l.pixel[1] - p.left_pixel[1],
        r.pixel[0] - p.right_pixel[0],
        r.pixel[1] - p.right_pixel[1],
    );
    if !residual
        .iter()
        .chain(rig.iter())
        .chain(target.iter())
        .all(|v| v.is_finite())
    {
        return Err("stereo residual/Jacobian overflow".into());
    }
    Ok((residual, rig, target))
}
/// Compute per-camera Euclidean residuals; fixed model/domain failures reject trials.
fn evaluate(
    cameras: &Cameras<'_>,
    state: &State,
    views: &[StereoView],
    cancel: &dyn Fn() -> bool,
) -> Result<Score, String> {
    let mut score = Score {
        squared: 0.,
        maximum: 0.,
        points: 0,
        per_view: Vec::with_capacity(views.len()),
    };
    for (pose, view) in state.poses.iter().zip(views) {
        check(cancel)?;
        let mut s = [0.; 2];
        let mut m = [0_f64; 2];
        for (i, p) in view.observations.iter().enumerate() {
            if i % 256 == 0 {
                check(cancel)?;
            }
            let (residual, _, _) = rows(cameras, state, pose, p)?;
            for camera in 0..2 {
                let error = residual[2 * camera].hypot(residual[2 * camera + 1]);
                s[camera] += error * error;
                m[camera] = m[camera].max(error);
            }
        }
        score.squared += s[0] + s[1];
        score.maximum = score.maximum.max(m[0]).max(m[1]);
        score.points += view.observations.len();
        score.per_view.push((s, m));
    }
    if !score.squared.is_finite() {
        return Err("stereo residual sum overflow".into());
    }
    Ok(score)
}
/// Accumulate small shared/independent blocks, without a full dense Jacobian.
fn build_normal(
    cameras: &Cameras<'_>,
    state: &State,
    views: &[StereoView],
    cancel: &dyn Fn() -> bool,
) -> Result<NormalSystem<6>, String> {
    let mut system = NormalSystem {
        global: SMatrix::zeros(),
        gradient: PoseVector::zeros(),
        views: Vec::with_capacity(views.len()),
    };
    for (pose, view) in state.poses.iter().zip(views) {
        check(cancel)?;
        let mut block = ViewNormal {
            pose: SMatrix::zeros(),
            cross: SMatrix::zeros(),
            gradient: PoseVector::zeros(),
        };
        for (i, p) in view.observations.iter().enumerate() {
            if i % 256 == 0 {
                check(cancel)?;
            }
            let (residual, rig, target) = rows(cameras, state, pose, p)?;
            system.global += rig.transpose() * rig;
            system.gradient += rig.transpose() * residual;
            block.pose += target.transpose() * target;
            block.cross += rig.transpose() * target;
            block.gradient += target.transpose() * residual;
        }
        if !block
            .pose
            .iter()
            .chain(block.cross.iter())
            .chain(block.gradient.iter())
            .all(|v| v.is_finite())
        {
            return Err("stereo target normal-equation overflow".into());
        }
        system.views.push(block);
    }
    if !system
        .global
        .iter()
        .chain(system.gradient.iter())
        .all(|v| v.is_finite())
    {
        return Err("stereo rig normal-equation overflow".into());
    }
    Ok(system)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Brown5;
    use nalgebra::Rotation3;
    fn fixture(
        skew: bool,
        noise: f64,
    ) -> (
        CameraModel,
        CameraModel,
        RigidPose,
        Vec<StereoView>,
        Vec<RigidPose>,
    ) {
        let left = CameraModel {
            width: 1280,
            height: 800,
            fx: 900.,
            fy: 850.,
            cx: 629.,
            cy: 393.,
            skew: if skew { 6. } else { 0. },
            distortion: Brown5 {
                k1: -0.09,
                k2: 0.025,
                k3: -0.005,
                p1: 0.0008,
                p2: -0.0011,
            },
            valid_radius: 1.,
        };
        let right = CameraModel {
            width: 1440,
            height: 900,
            fx: 1000.,
            fy: 960.,
            cx: 711.,
            cy: 446.,
            skew: if skew { -3. } else { 0. },
            distortion: Brown5 {
                k1: -0.07,
                k2: 0.018,
                k3: -0.003,
                p1: -0.0005,
                p2: 0.0007,
            },
            valid_radius: 1.,
        };
        let r = Rotation3::from_euler_angles(-0.01, 0.025, 0.005);
        let rig = RigidPose {
            rotation: std::array::from_fn(|i| std::array::from_fn(|j| r[(i, j)])),
            translation_mm: [-80., 3., 1.],
        };
        let mut views = Vec::new();
        let mut poses = Vec::new();
        for i in 0..8 {
            let f = i as f64;
            let r = Rotation3::from_euler_angles(
                0.4 * (f * 0.8).sin(),
                0.3 * (f * 0.6).cos(),
                0.1 * (f * 0.3).sin(),
            );
            let pose = RigidPose {
                rotation: std::array::from_fn(|a| std::array::from_fn(|b| r[(a, b)])),
                translation_mm: [-15. + f * 4., 10. * f.cos(), 350. + f * 20.],
            };
            let mut observations = Vec::new();
            for y in 0..7 {
                for x in 0..9 {
                    let xy = [(x as f64 - 4.) * 22., (y as f64 - 3.) * 22.];
                    let point = pose.apply([xy[0], xy[1], 0.]);
                    let mut lp = left.project(point).unwrap();
                    let mut rp = right.project(rig.apply(point)).unwrap();
                    let phase = (i * 63 + y * 9 + x) as f64;
                    lp[0] += noise * (phase * 1.7).sin();
                    lp[1] += noise * (phase * 0.9).cos();
                    rp[0] += noise * (phase * 1.3 + 0.4).cos();
                    rp[1] += noise * (phase * 0.7 - 0.2).sin();
                    observations.push(StereoObservation {
                        target_xy_mm: xy,
                        left_pixel: lp,
                        right_pixel: rp,
                    });
                }
            }
            views.push(StereoView { observations });
            poses.push(pose);
        }
        (left, right, rig, views, poses)
    }
    #[test]
    fn recovers_asymmetric_distorted_skewed_rig_and_target_poses() {
        for skew in [false, true] {
            let (left, right, truth, views, poses) = fixture(skew, 0.);
            let fit = fit_stereo_extrinsics(
                &left,
                &right,
                &views,
                &StereoFitOptions {
                    max_reprojection_error_px: 1e-5,
                    ..Default::default()
                },
                &|| false,
            )
            .unwrap();
            assert_eq!(fit.fixed_left, left);
            assert_eq!(fit.fixed_right, right);
            assert!((fit.right_from_left.matrix() - truth.matrix()).norm() < 1e-8);
            assert!(
                (Vector3::from(fit.right_from_left.translation_mm)
                    - Vector3::from(truth.translation_mm))
                .norm()
                    < 1e-5
            );
            assert!(fit.rms_px < 1e-7);
            for (actual, truth) in fit.views.iter().zip(poses) {
                assert!((actual.left_from_target.matrix() - truth.matrix()).norm() < 1e-8);
                assert!(
                    (Vector3::from(actual.left_from_target.translation_mm)
                        - Vector3::from(truth.translation_mm))
                    .norm()
                        < 1e-5
                );
            }
        }
    }
    #[test]
    fn analytic_stereo_rig_and_target_jacobians_match_finite_differences() {
        let (left, right, rig, views, poses) = fixture(true, 0.);
        let cameras = Cameras::new(&left, &right);
        let state = State {
            rig,
            poses: vec![poses[2].clone()],
        };
        let observation = &views[2].observations[15];
        let (_, global, pose) = rows(&cameras, &state, &state.poses[0], observation).unwrap();
        for column in 0..12 {
            let epsilon = 1e-6;
            let mut g = PoseVector::zeros();
            let mut p = PoseVector::zeros();
            if column < 6 {
                g[column] = epsilon;
            } else {
                p[column - 6] = epsilon;
            }
            let a = state.stepped(g, &[p]).unwrap();
            let b = state.stepped(-g, &[-p]).unwrap();
            let derivative = (rows(&cameras, &a, &a.poses[0], observation).unwrap().0
                - rows(&cameras, &b, &b.poses[0], observation).unwrap().0)
                / (2. * epsilon);
            let expected = if column < 6 {
                global.column(column).into_owned()
            } else {
                pose.column(column - 6).into_owned()
            };
            assert!(
                (derivative - expected).norm() < 2e-5,
                "column{column}: {derivative:?} vs {expected:?}"
            );
        }
        assert_eq!(global.fixed_rows::<2>(0).norm(), 0.);
    }
    #[test]
    fn noisy_joint_fit_improves_and_rejects_bad_final_residual() {
        let (left, right, truth, views, _) = fixture(false, 0.02);
        let fit =
            fit_stereo_extrinsics(&left, &right, &views, &StereoFitOptions::default(), &|| {
                false
            })
            .unwrap();
        assert!(
            fit.accepted_iterations > 0 && fit.rms_px < fit.initial_rms_px && fit.rms_px < 0.03
        );
        assert!(
            (Vector3::from(fit.right_from_left.translation_mm)
                - Vector3::from(truth.translation_mm))
            .norm()
                < 0.1
        );
        assert!(
            fit_stereo_extrinsics(
                &left,
                &right,
                &views,
                &StereoFitOptions {
                    max_reprojection_error_px: 1e-6,
                    ..Default::default()
                },
                &|| false
            )
            .is_err()
        );
    }
    #[test]
    fn metric_scaling_and_camera_swap_preserve_pose_conventions() {
        let (left, right, truth, views, _) = fixture(false, 0.);
        for scale in [0.001, 1000.] {
            let mut v = views.clone();
            for view in &mut v {
                for p in &mut view.observations {
                    for x in &mut p.target_xy_mm {
                        *x *= scale;
                    }
                }
            }
            let fit =
                fit_stereo_extrinsics(&left, &right, &v, &StereoFitOptions::default(), &|| false)
                    .unwrap();
            assert!(
                (Vector3::from(fit.right_from_left.translation_mm) / scale
                    - Vector3::from(truth.translation_mm))
                .norm()
                    < 1e-5
            );
            assert!((fit.right_from_left.matrix() - truth.matrix()).norm() < 1e-8);
        }
        let mut swapped = views.clone();
        for view in &mut swapped {
            for p in &mut view.observations {
                std::mem::swap(&mut p.left_pixel, &mut p.right_pixel);
            }
        }
        let fit = fit_stereo_extrinsics(
            &right,
            &left,
            &swapped,
            &StereoFitOptions::default(),
            &|| false,
        )
        .unwrap();
        let r = truth.matrix().transpose();
        let t = -r * Vector3::from(truth.translation_mm);
        assert!((fit.right_from_left.matrix() - r).norm() < 1e-8);
        assert!((Vector3::from(fit.right_from_left.translation_mm) - t).norm() < 1e-5);
    }
    #[test]
    fn cancellation_nonconvergence_and_invalid_input_do_not_mutate_cameras() {
        let (left, right, _, views, _) = fixture(false, 0.02);
        let original = serde_json::to_string(&views).unwrap();
        assert!(
            fit_stereo_extrinsics(&left, &right, &views, &StereoFitOptions::default(), &|| {
                true
            })
            .unwrap_err()
            .contains("cancelled")
        );
        for at in [10, 62] {
            let calls = Cell::new(0);
            let pulse = || {
                calls.set(calls.get() + 1);
                calls.get() == at
            };
            assert!(
                fit_stereo_extrinsics(&left, &right, &views, &StereoFitOptions::default(), &pulse)
                    .unwrap_err()
                    .contains("cancelled")
            );
            assert_eq!(calls.get(), at);
        }
        assert!(
            fit_stereo_extrinsics(
                &left,
                &right,
                &views,
                &StereoFitOptions {
                    max_iterations: 1,
                    ..Default::default()
                },
                &|| false
            )
            .unwrap_err()
            .contains("iteration limit")
        );
        for n in [0, 101] {
            assert!(
                fit_stereo_extrinsics(
                    &left,
                    &right,
                    &views,
                    &StereoFitOptions {
                        max_iterations: n,
                        ..Default::default()
                    },
                    &|| false
                )
                .is_err()
            );
        }
        for x in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(
                fit_stereo_extrinsics(
                    &left,
                    &right,
                    &views,
                    &StereoFitOptions {
                        max_reprojection_error_px: x,
                        ..Default::default()
                    },
                    &|| false
                )
                .is_err()
            );
        }
        assert!(
            fit_stereo_extrinsics(
                &left,
                &right,
                &views[..2],
                &StereoFitOptions::default(),
                &|| false
            )
            .is_err()
        );
        assert_eq!(serde_json::to_string(&views).unwrap(), original);
    }
    #[test]
    fn rejects_collinear_duplicate_bad_pairing_domain_and_zero_baseline() {
        let (left, right, _, views, _) = fixture(false, 0.);
        let mut v = views.clone();
        for view in &mut v {
            view.observations.truncate(9);
        }
        assert!(
            fit_stereo_extrinsics(&left, &right, &v, &StereoFitOptions::default(), &|| false)
                .is_err()
        );
        v = views.clone();
        v[0].observations[1].target_xy_mm = v[0].observations[0].target_xy_mm;
        assert!(
            fit_stereo_extrinsics(&left, &right, &v, &StereoFitOptions::default(), &|| false)
                .is_err()
        );
        v = views.clone();
        v[0].observations[0].right_pixel[0] = f64::NAN;
        assert!(
            fit_stereo_extrinsics(&left, &right, &v, &StereoFitOptions::default(), &|| false)
                .is_err()
        );
        v = views.clone();
        for (i, view) in v.iter_mut().enumerate() {
            for (j, p) in view.observations.iter_mut().enumerate() {
                p.right_pixel = views[(i + 1) % views.len()].observations[j].right_pixel;
            }
        }
        assert!(
            fit_stereo_extrinsics(
                &left,
                &right,
                &v,
                &StereoFitOptions {
                    max_reprojection_error_px: 0.05,
                    ..Default::default()
                },
                &|| false
            )
            .is_err()
        );
        let mut small = left.clone();
        small.valid_radius = 0.01;
        assert!(
            fit_stereo_extrinsics(
                &small,
                &right,
                &views,
                &StereoFitOptions::default(),
                &|| false
            )
            .is_err()
        );
        v = views.clone();
        for view in &mut v {
            for p in &mut view.observations {
                p.right_pixel = p.left_pixel;
            }
        }
        assert!(
            fit_stereo_extrinsics(&left, &left, &v, &StereoFitOptions::default(), &|| false)
                .unwrap_err()
                .contains("baseline")
        );
    }
}
