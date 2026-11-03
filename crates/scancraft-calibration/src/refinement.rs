//! Joint Brown5 camera/target-pose refinement of supplied planar correspondences.
//!
//! Independent least-squares implementation: Zhang MSR-TR-98-71 section 3.2,
//! Brown5 projection as documented by OpenCV calib3d, and standard block Schur
//! elimination. No upstream implementation code is copied. A fit is not physical
//! calibration qualification and is never automatically installed for capture.
use crate::{
    Brown5, CameraModel,
    normal_blocks::{information_min_eigenvalue, scaled_gradient_inf, solve_step},
    planar::{InitialIntrinsics, InitializedView, PlanarInitialization, PlanarView, validate_view},
    stereo::RigidPose,
};
use nalgebra::{SMatrix, SVector, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use std::cell::Cell;

type GlobalMatrix = SMatrix<f64, 10, 10>;
type GlobalVector = SVector<f64, 10>;
type PoseMatrix = SMatrix<f64, 6, 6>;
type PoseVector = SVector<f64, 6>;
type CrossMatrix = SMatrix<f64, 10, 6>;
type GlobalJacobian = SMatrix<f64, 2, 10>;
type PoseJacobian = SMatrix<f64, 2, 6>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementOptions {
    /// 1..=100 accepted-iteration opportunities, each with at most 16 LM trials.
    pub max_iterations: usize,
    /// Maximum individual Euclidean image residual accepted in the final fit.
    pub max_reprojection_error_px: f64,
    /// False fixes skew to zero, including when the linear initializer had skew.
    pub estimate_skew: bool,
}
impl Default for RefinementOptions {
    fn default() -> Self {
        Self {
            max_iterations: 80,
            max_reprojection_error_px: 1.,
            estimate_skew: false,
        }
    }
}

/// Numerically fitted parameters, deliberately not a CameraModel/CalibrationRecord.
#[derive(Debug, Clone, Serialize)]
pub struct RefinedPlanarCalibration {
    pub image_size: [u32; 2],
    pub intrinsics: InitialIntrinsics,
    pub distortion: Brown5,
    pub views: Vec<InitializedView>,
    pub initial_rms_px: f64,
    pub rms_px: f64,
    pub max_error_px: f64,
    pub accepted_iterations: usize,
    /// Maximum radius among observations, NOT a physically qualified lens domain.
    pub observed_max_normalized_radius: f64,
    /// Rank diagnostic of the final scaled, pose-eliminated information matrix.
    /// This is not a covariance estimate or physical confidence interval.
    pub scaled_information_min_eigenvalue: f64,
    pub estimate_skew: bool,
    /// Numerical stationarity diagnostic, not physical uncertainty.
    pub scaled_gradient_inf: f64,
}

#[derive(Clone)]
struct State {
    // log fx, log fy, cx, cy, skew, k1, k2, k3, p1, p2.
    camera: GlobalVector,
    poses: Vec<RigidPose>,
}
impl State {
    fn distortion(&self) -> Brown5 {
        Brown5 {
            k1: self.camera[5],
            k2: self.camera[6],
            k3: self.camera[7],
            p1: self.camera[8],
            p2: self.camera[9],
        }
    }
    fn intrinsics(&self) -> InitialIntrinsics {
        InitialIntrinsics {
            fx: self.camera[0].exp(),
            fy: self.camera[1].exp(),
            cx: self.camera[2],
            cy: self.camera[3],
            skew: self.camera[4],
        }
    }
    fn stepped(&self, global: GlobalVector, poses: &[PoseVector]) -> Result<Self, String> {
        let mut state = self.clone();
        state.camera += global;
        if !state.camera.iter().all(|x| x.is_finite()) {
            return Err("camera step overflow".into());
        }
        for (pose, step) in state.poses.iter_mut().zip(poses) {
            *pose = crate::projection_differential::step_pose(pose, *step)?;
        }
        Ok(state)
    }
}
type ViewNormal = crate::normal_blocks::ViewNormal<10>;
type NormalSystem = crate::normal_blocks::NormalSystem<10>;
struct Score {
    squared: f64,
    maximum: f64,
    radius: f64,
    per_view: Vec<(f64, f64)>,
    points: usize,
}
fn check_cancel(cancel: &dyn Fn() -> bool) -> Result<(), String> {
    if cancel() {
        Err("calibration refinement cancelled".into())
    } else {
        Ok(())
    }
}

/// Refine intrinsics, all five Brown coefficients, and each camera-from-target pose.
///
/// Assumes valid metric correspondences and a sufficiently close initial solution.
/// Uses squared pixel residuals without robust loss or automatic outlier removal.
/// Returns an error for cancellation, nonconvergence, weak final information,
/// excessive final residuals, or a folded model over the observed-radius disk.
/// The disk check is numerical only; no capture binding/domain is inferred.
pub fn refine_planar(
    views: &[PlanarView],
    initial: &PlanarInitialization,
    options: &RefinementOptions,
    cancel: &dyn Fn() -> bool,
) -> Result<RefinedPlanarCalibration, String> {
    // Retain a cancellation observation even if the caller's callback is a pulse.
    // Otherwise an invalid-trial retry could accidentally swallow cancellation.
    let requested = Cell::new(false);
    let latched_cancel = || {
        let stop = requested.get() || cancel();
        requested.set(stop);
        stop
    };
    let cancel: &dyn Fn() -> bool = &latched_cancel;
    check_cancel(cancel)?;
    if !(3..=64).contains(&views.len())
        || initial.views.len() != views.len()
        || initial.image_size.iter().any(|&n| n == 0 || n > 1_000_000)
        || !(1..=100).contains(&options.max_iterations)
        || !options.max_reprojection_error_px.is_finite()
        || options.max_reprojection_error_px <= 0.
    {
        return Err("invalid planar refinement inputs/options".into());
    }
    let mut count = 0;
    for view in views {
        check_cancel(cancel)?;
        validate_view(view, initial.image_size)?;
        count += view.observations.len();
    }
    let globals = if options.estimate_skew { 10 } else { 9 };
    if 2 * count <= globals + 6 * views.len() {
        return Err("insufficient residual degrees of freedom for Brown5 calibration".into());
    }
    let k = &initial.intrinsics;
    if ![k.fx, k.fy, k.cx, k.cy, k.skew]
        .iter()
        .all(|v| v.is_finite())
        || k.fx <= 0.
        || k.fy <= 0.
    {
        return Err("invalid initial intrinsics".into());
    }
    let mut state = State {
        camera: GlobalVector::from_row_slice(&[
            k.fx.ln(),
            k.fy.ln(),
            k.cx,
            k.cy,
            if options.estimate_skew { k.skew } else { 0. },
            0.,
            0.,
            0.,
            0.,
            0.,
        ]),
        poses: initial
            .views
            .iter()
            .map(|v| v.camera_from_target.clone())
            .collect(),
    };
    for pose in &state.poses {
        pose.validate()?;
    }
    let mut score = evaluate(&state, views, cancel)?;
    let initial_rms = (score.squared / count as f64).sqrt();
    let mut damping = 1e-3;
    let mut converged = false;
    let mut accepted = 0;
    for _ in 0..options.max_iterations {
        check_cancel(cancel)?;
        let normal = build_normal(&state, views, options.estimate_skew, cancel)?;
        let gradient = scaled_gradient_inf(&normal);
        if gradient < 1e-8 {
            converged = true;
            break;
        }
        let mut improved = None;
        for _ in 0..16 {
            check_cancel(cancel)?;
            if let Ok((global, poses)) = solve_step(&normal, damping)
                && let Ok(candidate) = state.stepped(global, &poses)
            {
                // Cancellation must propagate even if the numerical trial is invalid.
                check_cancel(cancel)?;
                match evaluate(&candidate, views, cancel) {
                    Ok(candidate_score) if candidate_score.squared < score.squared => {
                        let mut step_energy =
                            global.dot(&(normal.global.diagonal().component_mul(&global)));
                        for (block, step) in normal.views.iter().zip(&poses) {
                            step_energy += step.dot(&(block.pose.diagonal().component_mul(step)));
                        }
                        improved = Some((
                            candidate,
                            candidate_score,
                            (step_energy / count as f64).sqrt(),
                        ));
                        break;
                    }
                    _ => check_cancel(cancel)?,
                }
            }
            damping *= 10.;
            if !damping.is_finite() || damping > 1e16 {
                break;
            }
        }
        let Some((candidate, candidate_score, step_size)) = improved else {
            return Err("Brown5 refinement could not find a decreasing finite step".into());
        };
        let improvement = score.squared - candidate_score.squared;
        let old_score = score.squared;
        state = candidate;
        score = candidate_score;
        accepted += 1;
        damping = (damping / 3.).max(1e-12);
        if damping <= 1.
            && (step_size < 1e-8 || (improvement <= 1e-12 * old_score.max(1.) && step_size < 1e-6))
        {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err("Brown5 refinement iteration limit exceeded".into());
    }
    if score.maximum > options.max_reprojection_error_px {
        return Err("refined calibration exceeds the reprojection limit".into());
    }
    let final_normal = build_normal(&state, views, options.estimate_skew, cancel)?;
    let final_gradient = scaled_gradient_inf(&final_normal);
    if !final_gradient.is_finite() || final_gradient > 1e-6 {
        return Err("Brown5 refinement stalled before numerical stationarity".into());
    }
    let information = information_min_eigenvalue(&final_normal)?;
    let intrinsics = state.intrinsics();
    // This temporary model checks numerical monotonicity only. Observed maximum
    // radius does NOT establish coverage, extrapolation validity or physical units.
    CameraModel {
        width: initial.image_size[0],
        height: initial.image_size[1],
        fx: intrinsics.fx,
        fy: intrinsics.fy,
        cx: intrinsics.cx,
        cy: intrinsics.cy,
        skew: intrinsics.skew,
        distortion: state.distortion(),
        valid_radius: score.radius,
    }
    .validate()?;
    check_cancel(cancel)?;
    Ok(RefinedPlanarCalibration {
        image_size: initial.image_size,
        intrinsics,
        distortion: state.distortion(),
        views: state
            .poses
            .into_iter()
            .zip(score.per_view)
            .zip(views)
            .map(|((pose, (s, m)), view)| InitializedView {
                camera_from_target: pose,
                rms_px: (s / view.observations.len() as f64).sqrt(),
                max_error_px: m,
            })
            .collect(),
        initial_rms_px: initial_rms,
        rms_px: (score.squared / score.points as f64).sqrt(),
        max_error_px: score.maximum,
        accepted_iterations: accepted,
        observed_max_normalized_radius: score.radius,
        scaled_information_min_eigenvalue: information,
        estimate_skew: options.estimate_skew,
        scaled_gradient_inf: final_gradient,
    })
}

fn project_jacobians(
    state: &State,
    pose: &RigidPose,
    target: [f64; 2],
) -> Result<(Vector2<f64>, GlobalJacobian, PoseJacobian, f64), String> {
    let rotated = pose.matrix() * Vector3::new(target[0], target[1], 0.);
    let point = rotated + Vector3::from(pose.translation_mm);
    let k = state.intrinsics();
    let projection = crate::projection_differential::project(&k, state.distortion(), point)?;
    let [x, y] = projection.normalized;
    let d = projection.distorted;
    let pixel = projection.pixel;
    let mut global = GlobalJacobian::zeros();
    global[(0, 0)] = k.fx * d[0];
    global[(1, 1)] = k.fy * d[1];
    global[(0, 2)] = 1.;
    global[(1, 3)] = 1.;
    global[(0, 4)] = d[1];
    let r2 = x * x + y * y;
    let distortion_derivatives = [
        (x * r2, y * r2),
        (x * r2 * r2, y * r2 * r2),
        (x * r2 * r2 * r2, y * r2 * r2 * r2),
        (2. * x * y, r2 + 2. * y * y),
        (r2 + 2. * x * x, 2. * x * y),
    ];
    for (i, (dx, dy)) in distortion_derivatives.iter().enumerate() {
        global[(0, i + 5)] = k.fx * dx + k.skew * dy;
        global[(1, i + 5)] = k.fy * dy;
    }
    let pose_jacobian =
        projection.point_jacobian * crate::projection_differential::point_pose_jacobian(rotated);
    if !pixel
        .iter()
        .chain(global.iter())
        .chain(pose_jacobian.iter())
        .all(|v| v.is_finite())
    {
        return Err("calibration projection/Jacobian overflow".into());
    }
    Ok((pixel, global, pose_jacobian, x.hypot(y)))
}

fn evaluate(
    state: &State,
    views: &[PlanarView],
    cancel: &dyn Fn() -> bool,
) -> Result<Score, String> {
    let mut result = Score {
        squared: 0.,
        maximum: 0.,
        radius: 0.,
        per_view: Vec::with_capacity(views.len()),
        points: 0,
    };
    for (pose, view) in state.poses.iter().zip(views) {
        check_cancel(cancel)?;
        let mut squared = 0.;
        let mut maximum: f64 = 0.;
        for (i, p) in view.observations.iter().enumerate() {
            if i % 256 == 0 {
                check_cancel(cancel)?;
            }
            let (pixel, _, _, radius) = project_jacobians(state, pose, p.target_xy_mm)?;
            let residual = (pixel - Vector2::from(p.pixel)).norm();
            squared += residual * residual;
            maximum = maximum.max(residual);
            result.radius = result.radius.max(radius);
        }
        result.squared += squared;
        result.maximum = result.maximum.max(maximum);
        result.points += view.observations.len();
        result.per_view.push((squared, maximum));
    }
    if !result.squared.is_finite() || !result.radius.is_finite() || result.radius <= 0. {
        return Err("calibration residual/radius overflow".into());
    }
    Ok(result)
}

fn build_normal(
    state: &State,
    views: &[PlanarView],
    estimate_skew: bool,
    cancel: &dyn Fn() -> bool,
) -> Result<NormalSystem, String> {
    let mut system = NormalSystem {
        global: GlobalMatrix::zeros(),
        gradient: GlobalVector::zeros(),
        views: Vec::with_capacity(views.len()),
    };
    for (pose, view) in state.poses.iter().zip(views) {
        check_cancel(cancel)?;
        let mut block = ViewNormal {
            pose: PoseMatrix::zeros(),
            cross: CrossMatrix::zeros(),
            gradient: PoseVector::zeros(),
        };
        for (i, p) in view.observations.iter().enumerate() {
            if i % 256 == 0 {
                check_cancel(cancel)?;
            }
            let (pixel, mut global, pose, _) = project_jacobians(state, pose, p.target_xy_mm)?;
            if !estimate_skew {
                global.column_mut(4).fill(0.);
            }
            let residual = pixel - Vector2::from(p.pixel);
            system.global += global.transpose() * global;
            system.gradient += global.transpose() * residual;
            block.pose += pose.transpose() * pose;
            block.gradient += pose.transpose() * residual;
            block.cross += global.transpose() * pose;
        }
        if !block
            .pose
            .iter()
            .chain(block.cross.iter())
            .chain(block.gradient.iter())
            .all(|v| v.is_finite())
        {
            return Err("pose normal-equation overflow".into());
        }
        system.views.push(block);
    }
    if !estimate_skew {
        system.global[(4, 4)] = 1.;
    }
    if !system
        .global
        .iter()
        .chain(system.gradient.iter())
        .all(|v| v.is_finite())
    {
        return Err("camera normal-equation overflow".into());
    }
    Ok(system)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planar::{PlanarObservation, initialize_planar};
    use nalgebra::Rotation3;
    use std::cell::Cell;

    fn fixture(skew: f64, noise: f64) -> (CameraModel, Vec<PlanarView>, Vec<RigidPose>) {
        let camera = CameraModel {
            width: 1280,
            height: 800,
            fx: 900.,
            fy: 850.,
            cx: 629.,
            cy: 393.,
            skew,
            distortion: Brown5 {
                k1: -0.12,
                k2: 0.035,
                k3: -0.008,
                p1: 0.0008,
                p2: -0.0011,
            },
            valid_radius: 1.,
        };
        let mut views = Vec::new();
        let mut poses = Vec::new();
        for i in 0..12 {
            let f = i as f64;
            let r = Rotation3::from_euler_angles(
                0.45 * (f * 0.8).sin(),
                0.4 * (f * 0.6).cos(),
                0.2 * (f * 0.3).sin(),
            );
            let pose = RigidPose {
                rotation: std::array::from_fn(|a| std::array::from_fn(|b| r[(a, b)])),
                translation_mm: [-25. + f * 4., 15. * f.sin(), 340. + f * 15.],
            };
            let mut observations = Vec::new();
            for y in 0..9 {
                for x in 0..11 {
                    let target = [(x as f64 - 5.) * 22., (y as f64 - 4.) * 22.];
                    let mut pixel = camera
                        .project(pose.apply([target[0], target[1], 0.]))
                        .unwrap();
                    let phase = (i * 99 + y * 11 + x) as f64;
                    pixel[0] += noise * (phase * 1.7).sin();
                    pixel[1] += noise * (phase * 0.9).cos();
                    observations.push(PlanarObservation {
                        target_xy_mm: target,
                        pixel,
                    });
                }
            }
            views.push(PlanarView { observations });
            poses.push(pose);
        }
        (camera, views, poses)
    }

    #[test]
    fn recovers_brown5_intrinsics_and_poses_from_pinhole_seed() {
        for skew in [0., 6.] {
            let (truth, views, poses) = fixture(skew, 0.);
            let seed = initialize_planar([1280, 800], &views, 50.).unwrap();
            let options = RefinementOptions {
                estimate_skew: skew != 0.,
                max_reprojection_error_px: 1e-5,
                ..Default::default()
            };
            let fit = refine_planar(&views, &seed, &options, &|| false).unwrap();
            assert!(
                fit.rms_px < 1e-6 && fit.rms_px < fit.initial_rms_px * 1e-4,
                "{} -> {}",
                fit.initial_rms_px,
                fit.rms_px
            );
            let actual = &fit.intrinsics;
            for (a, b) in [
                (actual.fx, truth.fx),
                (actual.fy, truth.fy),
                (actual.cx, truth.cx),
                (actual.cy, truth.cy),
                (actual.skew, truth.skew),
            ] {
                assert!((a - b).abs() < 1e-4, "{a} != {b}");
            }
            let d = fit.distortion;
            let t = truth.distortion;
            for (a, b) in [
                (d.k1, t.k1),
                (d.k2, t.k2),
                (d.k3, t.k3),
                (d.p1, t.p1),
                (d.p2, t.p2),
            ] {
                assert!((a - b).abs() < 1e-4, "distortion {a} != {b}");
            }
            for (a, b) in fit.views.iter().zip(poses) {
                assert!((a.camera_from_target.matrix() - b.matrix()).norm() < 1e-6);
                assert!(
                    (Vector3::from(a.camera_from_target.translation_mm)
                        - Vector3::from(b.translation_mm))
                    .norm()
                        < 1e-4
                );
            }
        }
    }

    #[test]
    fn metric_scaling_preserves_refined_intrinsics_and_distortion() {
        let (truth, views, poses) = fixture(0., 0.);
        for scale in [0.001, 1000.] {
            let mut scaled = views.clone();
            for view in &mut scaled {
                for observation in &mut view.observations {
                    for coordinate in &mut observation.target_xy_mm {
                        *coordinate *= scale;
                    }
                }
            }
            let seed = initialize_planar([1280, 800], &scaled, 50.).unwrap();
            let fit =
                refine_planar(&scaled, &seed, &RefinementOptions::default(), &|| false).unwrap();
            assert!((fit.intrinsics.fx - truth.fx).abs() < 1e-4);
            assert!((fit.distortion.k3 - truth.distortion.k3).abs() < 1e-4);
            for (actual, expected) in fit.views.iter().zip(&poses) {
                assert!(
                    (Vector3::from(actual.camera_from_target.translation_mm) / scale
                        - Vector3::from(expected.translation_mm))
                    .norm()
                        < 1e-4
                );
            }
        }
    }

    #[test]
    fn analytic_camera_and_pose_jacobians_match_central_differences() {
        let (truth, _, poses) = fixture(6., 0.);
        let d = truth.distortion;
        let state = State {
            camera: GlobalVector::from_row_slice(&[
                truth.fx.ln(),
                truth.fy.ln(),
                truth.cx,
                truth.cy,
                truth.skew,
                d.k1,
                d.k2,
                d.k3,
                d.p1,
                d.p2,
            ]),
            poses: vec![poses[2].clone()],
        };
        let target = [103., -67.];
        let (_, global, pose, _) = project_jacobians(&state, &state.poses[0], target).unwrap();
        for column in 0..16 {
            let epsilon = 1e-6;
            let mut camera_step = GlobalVector::zeros();
            let mut pose_step = PoseVector::zeros();
            if column < 10 {
                camera_step[column] = epsilon;
            } else {
                pose_step[column - 10] = epsilon;
            }
            let plus = state.stepped(camera_step, &[pose_step]).unwrap();
            let minus = state.stepped(-camera_step, &[-pose_step]).unwrap();
            let difference = (project_jacobians(&plus, &plus.poses[0], target).unwrap().0
                - project_jacobians(&minus, &minus.poses[0], target)
                    .unwrap()
                    .0)
                / (2. * epsilon);
            let expected = if column < 10 {
                global.column(column).into_owned()
            } else {
                pose.column(column - 10).into_owned()
            };
            assert!(
                (difference - expected).norm() < 1e-5,
                "column {column}: {difference:?} != {expected:?}"
            );
        }
    }

    #[test]
    fn noisy_fit_improves_and_final_individual_error_limit_is_enforced() {
        let (_, views, _) = fixture(0., 0.02);
        let seed = initialize_planar([1280, 800], &views, 50.).unwrap();
        let fit = refine_planar(&views, &seed, &RefinementOptions::default(), &|| false).unwrap();
        assert!(fit.rms_px < 0.03 && fit.rms_px < fit.initial_rms_px / 10.);
        assert_eq!(fit.intrinsics.skew, 0.);
        assert!(
            refine_planar(
                &views,
                &seed,
                &RefinementOptions {
                    max_reprojection_error_px: 1e-6,
                    ..Default::default()
                },
                &|| false
            )
            .is_err()
        );
    }

    #[test]
    fn cancellation_and_nonconvergence_preserve_input() {
        let (_, views, _) = fixture(0., 0.);
        let seed = initialize_planar([1280, 800], &views, 50.).unwrap();
        let original = serde_json::to_string(&seed).unwrap();
        assert!(
            refine_planar(&views, &seed, &RefinementOptions::default(), &|| true)
                .unwrap_err()
                .contains("cancelled")
        );
        let calls = Cell::new(0);
        let cancel = || {
            calls.set(calls.get() + 1);
            calls.get() > 30
        };
        assert!(
            refine_planar(&views, &seed, &RefinementOptions::default(), &cancel)
                .unwrap_err()
                .contains("cancelled")
        );
        // 30 reaches initial scoring; 66 reaches scoring of the first LM trial
        // for this 12-view fixture. A transient request must not become a merely
        // rejected numerical trial and disappear when the callback returns false.
        for at in [30, 66] {
            let calls = Cell::new(0);
            let pulse = || {
                calls.set(calls.get() + 1);
                calls.get() == at
            };
            assert!(
                refine_planar(&views, &seed, &RefinementOptions::default(), &pulse)
                    .unwrap_err()
                    .contains("cancelled")
            );
            assert_eq!(calls.get(), at);
        }
        assert!(
            refine_planar(
                &views,
                &seed,
                &RefinementOptions {
                    max_iterations: 1,
                    ..Default::default()
                },
                &|| false
            )
            .unwrap_err()
            .contains("iteration limit")
        );
        assert_eq!(serde_json::to_string(&seed).unwrap(), original);
    }

    #[test]
    fn rejects_invalid_options_poses_and_insufficient_degrees_of_freedom() {
        let (_, views, _) = fixture(0., 0.);
        let seed = initialize_planar([1280, 800], &views, 50.).unwrap();
        for iterations in [0, 101] {
            assert!(
                refine_planar(
                    &views,
                    &seed,
                    &RefinementOptions {
                        max_iterations: iterations,
                        ..Default::default()
                    },
                    &|| false
                )
                .is_err()
            );
        }
        for limit in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(
                refine_planar(
                    &views,
                    &seed,
                    &RefinementOptions {
                        max_reprojection_error_px: limit,
                        ..Default::default()
                    },
                    &|| false
                )
                .is_err()
            );
        }
        let mut invalid = seed.clone();
        invalid.intrinsics.fx = 0.;
        assert!(refine_planar(&views, &invalid, &RefinementOptions::default(), &|| false).is_err());
        invalid = seed.clone();
        invalid.views[0].camera_from_target.rotation[0][0] = 5.;
        assert!(refine_planar(&views, &invalid, &RefinementOptions::default(), &|| false).is_err());
        invalid = seed.clone();
        invalid.views.pop();
        assert!(refine_planar(&views, &invalid, &RefinementOptions::default(), &|| false).is_err());
        let mut few = views[..3].to_vec();
        for view in &mut few {
            view.observations = [0, 10, 88, 98]
                .map(|i| view.observations[i].clone())
                .to_vec();
        }
        let seed = initialize_planar([1280, 800], &few, 50.).unwrap();
        assert!(
            refine_planar(&few, &seed, &RefinementOptions::default(), &|| false)
                .unwrap_err()
                .contains("degrees of freedom")
        );
    }

    #[test]
    fn final_information_rejects_unobservable_repeated_pinhole_views() {
        let (mut camera, _, poses) = fixture(0., 0.);
        camera.distortion = Brown5::ZERO;
        let pose = &poses[0];
        let mut observations = Vec::new();
        for y in 0..9 {
            for x in 0..11 {
                let xy = [(x as f64 - 5.) * 22., (y as f64 - 4.) * 22.];
                observations.push(PlanarObservation {
                    target_xy_mm: xy,
                    pixel: camera.project(pose.apply([xy[0], xy[1], 0.])).unwrap(),
                });
            }
        }
        let views = vec![PlanarView { observations }; 4];
        let seed = PlanarInitialization {
            image_size: [1280, 800],
            intrinsics: InitialIntrinsics {
                fx: camera.fx,
                fy: camera.fy,
                cx: camera.cx,
                cy: camera.cy,
                skew: 0.,
            },
            views: vec![
                InitializedView {
                    camera_from_target: pose.clone(),
                    rms_px: 0.,
                    max_error_px: 0.
                };
                4
            ],
            rms_px: 0.,
            max_error_px: 0.,
            constraint_singular_values: [0.; 6],
        };
        let error =
            refine_planar(&views, &seed, &RefinementOptions::default(), &|| false).unwrap_err();
        assert!(
            error.contains("unobservable") || error.contains("degenerate"),
            "{error}"
        );
    }
}
