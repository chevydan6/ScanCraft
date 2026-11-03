//! Fixed-camera planar pose fitting for held-out image validation.
//!
//! Independent homography initialization and reprojection LM, reusing native
//! Brown derivatives and small normal equations. No OpenCV source/runtime.
//! Target coordinates and translation share caller-supplied units, not assumed mm.
use crate::{
    CameraModel,
    normal_blocks::{NormalSystem, information_min_eigenvalue, scaled_gradient_inf, solve_step},
    planar::{InitialIntrinsics, PlanarObservation, PlanarView, validate_view},
    projection_differential::{point_pose_jacobian, project, step_pose},
    stereo::RigidPose,
    stereo_fit::pose_seed,
};
use nalgebra::{SMatrix, SVector, Vector2, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub target_xy: [f64; 2],
    pub pixel: [f64; 2],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub max_iterations: usize,
    pub max_reprojection_error_px: f64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            max_iterations: 80,
            max_reprojection_error_px: 1.,
        }
    }
}
/// A diagnostic fit of six pose parameters with all camera parameters fixed.
/// Low residuals do not prove physical scale, camera accuracy or pose uniqueness.
#[derive(Debug, Clone, Serialize)]
pub struct Fit {
    pub fixed_camera: CameraModel,
    /// p_camera = rotation * [target_x, target_y, 0] + translation.
    pub rotation: [[f64; 3]; 3],
    pub translation_in_supplied_target_units: [f64; 3],
    pub initial_rms_px: f64,
    /// sqrt(sum of squared 2D Euclidean residuals / number of observations).
    pub rms_px: f64,
    pub max_error_px: f64,
    /// Predicted minus observed pixel, in input order; no hidden point rejection.
    pub residuals_px: Vec<[f64; 2]>,
    pub accepted_iterations: usize,
    pub scaled_gradient_inf: f64,
    /// Scaled undamped pose information eigenvalue, not covariance/confidence.
    pub scaled_information_min_eigenvalue: f64,
}
fn check(cancel: &dyn Fn() -> bool) -> Result<(), String> {
    if cancel() {
        Err("planar pose fit cancelled".into())
    } else {
        Ok(())
    }
}
struct Score {
    normal: NormalSystem<6>,
    squared: f64,
    maximum: f64,
    residuals: Vec<[f64; 2]>,
}
fn evaluate(
    camera: &CameraModel,
    pose: &RigidPose,
    points: &[Observation],
    cancel: &dyn Fn() -> bool,
) -> Result<Score, String> {
    let k = InitialIntrinsics {
        fx: camera.fx,
        fy: camera.fy,
        cx: camera.cx,
        cy: camera.cy,
        skew: camera.skew,
    };
    let mut score = Score {
        normal: NormalSystem {
            global: SMatrix::zeros(),
            gradient: SVector::zeros(),
            views: Vec::new(),
        },
        squared: 0.,
        maximum: 0.,
        residuals: Vec::with_capacity(points.len()),
    };
    let rotation = pose.matrix();
    for (i, point) in points.iter().enumerate() {
        if i % 256 == 0 {
            check(cancel)?;
        }
        let rotated = rotation * Vector3::new(point.target_xy[0], point.target_xy[1], 0.);
        let projection = project(
            &k,
            camera.distortion,
            rotated + Vector3::from(pose.translation_mm),
        )?;
        if projection.normalized[0].hypot(projection.normalized[1]) > camera.valid_radius {
            return Err("planar pose outside supplied lens domain".into());
        }
        let residual = projection.pixel - Vector2::from(point.pixel);
        let jacobian = projection.point_jacobian * point_pose_jacobian(rotated);
        score.normal.global += jacobian.transpose() * jacobian;
        score.normal.gradient += jacobian.transpose() * residual;
        score.squared += residual.norm_squared();
        score.maximum = score.maximum.max(residual.norm());
        score.residuals.push(residual.into());
    }
    if !score.squared.is_finite()
        || !score
            .normal
            .global
            .iter()
            .chain(score.normal.gradient.iter())
            .all(|v| v.is_finite())
    {
        return Err("planar pose normal system overflow".into());
    }
    Ok(score)
}

/// Fit one planar target pose using 4..=4096 distinct correspondences.
///
/// All observed inverse rays and candidate forward projections must lie within
/// the supplied fixed camera domain. No lens fitting, robust loss, point removal,
/// alternate-solution search, unit inference, calibration activation or hardware
/// access. Caller must establish that validation images were excluded from lens
/// training. This routine cannot attest that provenance from coordinates alone.
pub fn fit(
    camera: &CameraModel,
    points: &[Observation],
    options: &Options,
    cancel: &dyn Fn() -> bool,
) -> Result<Fit, String> {
    check(cancel)?;
    camera.validate()?;
    if !(1..=100).contains(&options.max_iterations)
        || !options.max_reprojection_error_px.is_finite()
        || options.max_reprojection_error_px <= 0.
    {
        return Err("invalid planar pose fit options".into());
    }
    if !(4..=4096).contains(&points.len()) {
        return Err("expected 4..=4096 observations".into());
    }
    // These private numerical helpers are homogeneous in target length. Their
    // historical *_mm fields are used internally only; public input/output here
    // deliberately preserve supplied units without asserting metric provenance.
    let view = PlanarView {
        observations: points
            .iter()
            .map(|p| PlanarObservation {
                target_xy_mm: p.target_xy,
                pixel: p.pixel,
            })
            .collect(),
    };
    validate_view(&view, [camera.width, camera.height])?;
    let mut pose = pose_seed(camera, &view, cancel)?;
    let mut score = evaluate(camera, &pose, points, cancel)?;
    let initial_rms_px = (score.squared / points.len() as f64).sqrt();
    let mut damping = 1e-3;
    let mut accepted = 0;
    let mut converged = false;
    for _ in 0..options.max_iterations {
        check(cancel)?;
        let gradient = scaled_gradient_inf(&score.normal);
        if gradient < 1e-8 {
            converged = true;
            break;
        }
        let mut improved = None;
        for _ in 0..16 {
            check(cancel)?;
            if let Ok((step, _)) = solve_step(&score.normal, damping)
                && let Ok(candidate) = step_pose(&pose, step)
                && let Ok(trial) = evaluate(camera, &candidate, points, cancel)
                && trial.squared < score.squared
            {
                improved = Some((candidate, trial));
                break;
            }
            damping *= 10.;
            if !damping.is_finite() || damping > 1e16 {
                break;
            }
        }
        check(cancel)?;
        let Some((candidate, trial)) = improved else {
            if gradient <= 1e-6 {
                converged = true;
                break;
            }
            return Err("planar pose could not find a decreasing finite step".into());
        };
        pose = candidate;
        score = trial;
        accepted += 1;
        damping = (damping / 3.).max(1e-12);
    }
    let gradient = scaled_gradient_inf(&score.normal);
    // A final accepted step can reach stationarity on the iteration boundary.
    if (!converged && gradient > 1e-6) || !gradient.is_finite() {
        return Err("planar pose iteration limit or stationarity failure".into());
    }
    if score.maximum > options.max_reprojection_error_px {
        return Err("planar pose exceeds final reprojection limit".into());
    }
    let information = information_min_eigenvalue(&score.normal)?;
    check(cancel)?;
    Ok(Fit {
        fixed_camera: camera.clone(),
        rotation: pose.rotation,
        translation_in_supplied_target_units: pose.translation_mm,
        initial_rms_px,
        rms_px: (score.squared / points.len() as f64).sqrt(),
        max_error_px: score.maximum,
        residuals_px: score.residuals,
        accepted_iterations: accepted,
        scaled_gradient_inf: gradient,
        scaled_information_min_eigenvalue: information,
    })
}

#[cfg(test)]
mod tests;
