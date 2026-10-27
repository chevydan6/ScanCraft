//! Linear planar pinhole initialization, not a qualified camera calibration.
//!
//! Independent Rust implementation of Zhang, MSR-TR-98-71, sections 2.3/3.1
//! and appendices A/C: https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/tr98-71.pdf
//! No upstream implementation is copied. Distortion and nonlinear refinement are
//! intentionally absent: these results cannot directly create a capture record.
use crate::stereo::RigidPose;
use nalgebra::{DMatrix, Matrix3, Vector3, linalg::SVD};
use serde::{Deserialize, Serialize};

/// Correspondence on a metric target plane, whose third coordinate is zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanarObservation {
    pub target_xy_mm: [f64; 2],
    pub pixel: [f64; 2],
}

/// One image of the same fixed-intrinsics camera; target coordinates are in mm.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanarView {
    pub observations: Vec<PlanarObservation>,
}

/// An initial pinhole estimate. No lens-domain, distortion or device binding claim.
#[derive(Debug, Clone, Serialize)]
pub struct InitialIntrinsics {
    pub fx: f64,
    pub fy: f64,
    pub cx: f64,
    pub cy: f64,
    pub skew: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InitializedView {
    /// Transforms target [X,Y,0] millimeters to camera +X right,+Y down,+Z forward.
    pub camera_from_target: RigidPose,
    /// RMS Euclidean image distance, not RMS per scalar coordinate.
    pub rms_px: f64,
    pub max_error_px: f64,
}

/// Diagnostic initializer output, deliberately distinct from CameraModel/CalibrationRecord.
#[derive(Debug, Clone, Serialize)]
pub struct PlanarInitialization {
    pub image_size: [u32; 2],
    pub intrinsics: InitialIntrinsics,
    pub views: Vec<InitializedView>,
    pub rms_px: f64,
    pub max_error_px: f64,
    /// Descending singular values of the normalized intrinsic constraint matrix.
    pub constraint_singular_values: [f64; 6],
}

// Numerical policies, not physical accuracy/coverage thresholds. Constraint rows
// and image coordinates are normalized before these relative rank tests.
const MIN_RANK_RATIO: f64 = 1e-8;
const MAX_NULLSPACE_RATIO: f64 = 0.25;

/// Initialize five pinhole intrinsics and per-view target poses from 3..=64 views.
///
/// Each view contains 4..=4096 finite, distinct target points. Diverse plane
/// orientations are required. Assumes zero distortion; no outlier rejection or
/// refinement. A caller-chosen maximum *individual* reprojection error rejects
/// poor fits, but even a low error does not establish correct calibration.
/// Results are not installed, saved as capture calibration, or applied to hardware.
pub fn initialize_planar(
    image_size: [u32; 2],
    views: &[PlanarView],
    max_reprojection_error_px: f64,
) -> Result<PlanarInitialization, String> {
    if image_size.iter().any(|&n| n == 0 || n > 1_000_000)
        || !(3..=64).contains(&views.len())
        || !max_reprojection_error_px.is_finite()
        || max_reprojection_error_px <= 0.
    {
        return Err("invalid image size, view count or reprojection limit".into());
    }
    // A common image similarity keeps the absolute-conic system well scaled.
    let scale = f64::from(image_size[0].max(image_size[1]));
    let cx = (f64::from(image_size[0]) - 1.) / 2.;
    let cy = (f64::from(image_size[1]) - 1.) / 2.;
    let image_normalization = Matrix3::new(
        1. / scale,
        0.,
        -cx / scale,
        0.,
        1. / scale,
        -cy / scale,
        0.,
        0.,
        1.,
    );
    let mut homographies = Vec::with_capacity(views.len());
    let mut constraints = DMatrix::zeros(2 * views.len(), 6);
    for (index, view) in views.iter().enumerate() {
        validate_view(view, image_size).map_err(|e| format!("view {index}: {e}"))?;
        let target: Vec<_> = view.observations.iter().map(|p| p.target_xy_mm).collect();
        let pixels: Vec<_> = view.observations.iter().map(|p| p.pixel).collect();
        let h = image_normalization * homography(&target, &pixels)?;
        let a = intrinsic_constraint(&h, 0, 1);
        let b = intrinsic_constraint(&h, 0, 0) - intrinsic_constraint(&h, 1, 1);
        for (row, values) in [a, b].iter().enumerate() {
            let norm = values.norm();
            if !norm.is_finite() || norm <= f64::MIN_POSITIVE {
                return Err("singular intrinsic constraint".into());
            }
            for col in 0..6 {
                constraints[(index * 2 + row, col)] = values[col] / norm;
            }
        }
        homographies.push(h);
    }
    let (b, singular_values) = nullspace(constraints, 5)?;
    let sign = if b[0] < 0. { -1. } else { 1. };
    let conic = Matrix3::new(b[0], b[1], b[3], b[1], b[2], b[4], b[3], b[4], b[5]) * sign;
    // B=K^-T K^-1. Cholesky's positive diagonal chooses positive focal lengths.
    let factor = conic
        .cholesky()
        .ok_or("intrinsic conic is not positive definite")?;
    let mut k_normalized = factor
        .l()
        .transpose()
        .try_inverse()
        .ok_or("singular intrinsic matrix")?;
    k_normalized /= k_normalized[(2, 2)];
    let k = image_normalization
        .try_inverse()
        .ok_or("image normalization failed")?
        * k_normalized;
    if !k.iter().all(|v| v.is_finite()) || k[(0, 0)] <= 0. || k[(1, 1)] <= 0. {
        return Err("invalid recovered intrinsic matrix".into());
    }
    let k_inverse = k_normalized
        .try_inverse()
        .ok_or("singular recovered intrinsic matrix")?;
    let mut initialized_views = Vec::with_capacity(views.len());
    let mut sum_squared = 0.;
    let mut count = 0usize;
    let mut maximum: f64 = 0.;
    for (view, h) in views.iter().zip(homographies) {
        let pose = recover_pose(k_inverse * h, view.observations[0].target_xy_mm)?;
        let mut view_squared = 0.;
        let mut view_maximum: f64 = 0.;
        for observation in &view.observations {
            let point = Vector3::from(pose.apply([
                observation.target_xy_mm[0],
                observation.target_xy_mm[1],
                0.,
            ]));
            if !point.iter().all(|v| v.is_finite()) || point[2] <= 0. {
                return Err("target observation lies behind the recovered camera".into());
            }
            let projected = k * (point / point[2]);
            let error =
                (projected[0] - observation.pixel[0]).hypot(projected[1] - observation.pixel[1]);
            if !error.is_finite() || error > max_reprojection_error_px {
                return Err("initial pinhole fit exceeds the reprojection limit".into());
            }
            view_squared += error * error;
            view_maximum = view_maximum.max(error);
        }
        if !view_squared.is_finite() {
            return Err("reprojection sum overflow".into());
        }
        sum_squared += view_squared;
        count += view.observations.len();
        maximum = maximum.max(view_maximum);
        initialized_views.push(InitializedView {
            camera_from_target: pose,
            rms_px: (view_squared / view.observations.len() as f64).sqrt(),
            max_error_px: view_maximum,
        });
    }
    if !sum_squared.is_finite() {
        return Err("total reprojection sum overflow".into());
    }
    Ok(PlanarInitialization {
        image_size,
        intrinsics: InitialIntrinsics {
            fx: k[(0, 0)],
            fy: k[(1, 1)],
            cx: k[(0, 2)],
            cy: k[(1, 2)],
            skew: k[(0, 1)],
        },
        views: initialized_views,
        rms_px: (sum_squared / count as f64).sqrt(),
        max_error_px: maximum,
        constraint_singular_values: std::array::from_fn(|i| singular_values[i]),
    })
}

pub(crate) fn validate_view(view: &PlanarView, size: [u32; 2]) -> Result<(), String> {
    if !(4..=4096).contains(&view.observations.len()) {
        return Err("expected 4..=4096 observations".into());
    }
    for p in &view.observations {
        if p.target_xy_mm
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1e9)
            || p.pixel
                .iter()
                .enumerate()
                .any(|(i, v)| !v.is_finite() || *v < -0.5 || *v >= f64::from(size[i]) - 0.5)
        {
            return Err(
                "nonfinite/out-of-bounds observation or target exceeds coordinate limit".into(),
            );
        }
    }
    let mut points: Vec<_> = view.observations.iter().map(|p| p.target_xy_mm).collect();
    points.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap()
            .then(a[1].partial_cmp(&b[1]).unwrap())
    });
    if points.windows(2).any(|w| w[0] == w[1]) {
        return Err("duplicate target point".into());
    }
    Ok(())
}

fn normalize(points: &[[f64; 2]]) -> Result<(Matrix3<f64>, Vec<[f64; 2]>), String> {
    let mut center = [0.; 2];
    for p in points {
        for i in 0..2 {
            center[i] += p[i] / points.len() as f64;
        }
    }
    let distance = points
        .iter()
        .map(|p| (p[0] - center[0]).hypot(p[1] - center[1]) / points.len() as f64)
        .sum::<f64>();
    if !distance.is_finite() || distance <= f64::MIN_POSITIVE {
        return Err("coincident or unrepresentable points".into());
    }
    let s = std::f64::consts::SQRT_2 / distance;
    let transform = Matrix3::new(s, 0., -s * center[0], 0., s, -s * center[1], 0., 0., 1.);
    let normalized: Vec<_> = points
        .iter()
        .map(|p| [(p[0] - center[0]) * s, (p[1] - center[1]) * s])
        .collect();
    if !transform
        .iter()
        .chain(normalized.iter().flatten())
        .all(|v| v.is_finite())
    {
        return Err("point normalization overflow".into());
    }
    Ok((transform, normalized))
}

fn homography(target: &[[f64; 2]], pixels: &[[f64; 2]]) -> Result<Matrix3<f64>, String> {
    let (t, x) = normalize(target)?;
    let (u, y) = normalize(pixels)?;
    // Pad the four-point 8x9 system: thin SVD otherwise omits its null vector.
    let mut a = DMatrix::zeros((2 * target.len()).max(9), 9);
    for (i, (x, y)) in x.iter().zip(y).enumerate() {
        let r = [x[0], x[1], 1.];
        for j in 0..3 {
            a[(2 * i, j)] = r[j];
            a[(2 * i, j + 6)] = -y[0] * r[j];
            a[(2 * i + 1, j + 3)] = r[j];
            a[(2 * i + 1, j + 6)] = -y[1] * r[j];
        }
    }
    let (h, _) = nullspace(a, 8)?;
    let result =
        u.try_inverse().ok_or("singular pixel normalization")? * Matrix3::from_row_slice(&h) * t;
    let norm = result.norm();
    if !norm.is_finite() || norm <= f64::MIN_POSITIVE {
        return Err("unrepresentable homography".into());
    }
    Ok(result / norm)
}

fn nullspace(a: DMatrix<f64>, rank: usize) -> Result<(Vec<f64>, Vec<f64>), String> {
    if !a.iter().all(|v| v.is_finite()) {
        return Err("nonfinite linear system".into());
    }
    let svd =
        SVD::try_new(a, false, true, 1e-14, 10_000).ok_or("calibration SVD did not converge")?;
    let s = svd.singular_values;
    if !s.iter().all(|v| v.is_finite())
        || s[0] <= 0.
        || s[rank - 1] / s[0] < MIN_RANK_RATIO
        || s[rank] / s[rank - 1] > MAX_NULLSPACE_RATIO
    {
        return Err("degenerate, ill-conditioned or inconsistent planar observations".into());
    }
    let vt = svd.v_t.ok_or("missing calibration nullspace")?;
    Ok((
        (0..vt.ncols()).map(|j| vt[(rank, j)]).collect(),
        s.iter().copied().collect(),
    ))
}

fn intrinsic_constraint(h: &Matrix3<f64>, i: usize, j: usize) -> nalgebra::SVector<f64, 6> {
    nalgebra::SVector::from_row_slice(&[
        h[(0, i)] * h[(0, j)],
        h[(0, i)] * h[(1, j)] + h[(1, i)] * h[(0, j)],
        h[(1, i)] * h[(1, j)],
        h[(2, i)] * h[(0, j)] + h[(0, i)] * h[(2, j)],
        h[(2, i)] * h[(1, j)] + h[(1, i)] * h[(2, j)],
        h[(2, i)] * h[(2, j)],
    ])
}

fn recover_pose(h: Matrix3<f64>, visible_target: [f64; 2]) -> Result<RigidPose, String> {
    let length = (h.column(0).norm() + h.column(1).norm()) / 2.;
    if !length.is_finite() || length <= f64::MIN_POSITIVE {
        return Err("singular target pose".into());
    }
    // Choose sign at an observed point; the target coordinate origin need not be visible.
    let visible_depth = h[(2, 0)] * visible_target[0] + h[(2, 1)] * visible_target[1] + h[(2, 2)];
    let scale = if visible_depth < 0. {
        -1. / length
    } else {
        1. / length
    };
    let a = h.column(0) * scale;
    let b = h.column(1) * scale;
    let c = a.cross(&b);
    let rotation = Matrix3::from_columns(&[a, b, c]);
    let svd = SVD::try_new(rotation, true, true, 1e-14, 1000).ok_or("pose SVD did not converge")?;
    let u = svd.u.ok_or("missing pose U")?;
    let vt = svd.v_t.ok_or("missing pose Vt")?;
    let mut correction = Matrix3::identity();
    correction[(2, 2)] = (u * vt).determinant().signum();
    let r = u * correction * vt;
    let pose = RigidPose {
        rotation: std::array::from_fn(|i| std::array::from_fn(|j| r[(i, j)])),
        translation_mm: (h.column(2) * scale).into(),
    };
    pose.validate()?;
    Ok(pose)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Rotation3;

    fn observations(skew: f64, noise: f64) -> (Matrix3<f64>, Vec<PlanarView>, Vec<RigidPose>) {
        let k = Matrix3::new(920., skew, 621., 0., 870., 349., 0., 0., 1.);
        let mut views = Vec::new();
        let mut poses = Vec::new();
        for i in 0..6 {
            let f = i as f64;
            let rotation =
                Rotation3::from_euler_angles(-0.32 + f * 0.11, 0.25 - f * 0.085, -0.12 + f * 0.04);
            let translation = Vector3::new(-20. + f * 7., 12. - f * 4., 420. + f * 35.);
            let mut observations = Vec::new();
            for row in 0..6 {
                for col in 0..8 {
                    let target = [(col as f64 - 3.5) * 18., (row as f64 - 2.5) * 18.];
                    let point = rotation * Vector3::new(target[0], target[1], 0.) + translation;
                    let pixel = k * (point / point[2]);
                    let phase = (row * 8 + col + i * 48) as f64;
                    observations.push(PlanarObservation {
                        target_xy_mm: target,
                        pixel: [
                            pixel[0] + noise * (phase * 1.7).sin(),
                            pixel[1] + noise * (phase * 0.9).cos(),
                        ],
                    });
                }
            }
            views.push(PlanarView { observations });
            poses.push(RigidPose {
                rotation: std::array::from_fn(|a| std::array::from_fn(|b| rotation[(a, b)])),
                translation_mm: translation.into(),
            });
        }
        (k, views, poses)
    }

    #[test]
    fn recovers_asymmetric_intrinsics_skew_and_metric_poses() {
        for skew in [0., 7.5, -4.] {
            let (k, views, poses) = observations(skew, 0.);
            let result = initialize_planar([1280, 720], &views, 1e-6).unwrap();
            let recovered = &result.intrinsics;
            for (actual, expected) in [
                (recovered.fx, k[(0, 0)]),
                (recovered.fy, k[(1, 1)]),
                (recovered.cx, k[(0, 2)]),
                (recovered.cy, k[(1, 2)]),
                (recovered.skew, skew),
            ] {
                assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
            }
            for (view, expected) in result.views.iter().zip(poses) {
                assert!((view.camera_from_target.matrix() - expected.matrix()).norm() < 1e-8);
                assert!(
                    (Vector3::from(view.camera_from_target.translation_mm)
                        - Vector3::from(expected.translation_mm))
                    .norm()
                        < 1e-6
                );
            }
            assert!(result.max_error_px < 1e-7);
        }
    }

    #[test]
    fn supports_minimum_three_views_and_four_points_without_thin_svd_loss() {
        let (_, mut views, _) = observations(3., 0.);
        views.truncate(3);
        for view in &mut views {
            view.observations = [0, 7, 40, 47]
                .map(|i| view.observations[i].clone())
                .to_vec();
        }
        let result = initialize_planar([1280, 720], &views, 1e-6).unwrap();
        assert!((result.intrinsics.fx - 920.).abs() < 1e-6);
        assert!((result.intrinsics.skew - 3.).abs() < 1e-6);
    }

    #[test]
    fn noisy_observations_report_residuals_and_strict_limit_rejects() {
        let (_, views, _) = observations(0., 0.02);
        let result = initialize_planar([1280, 720], &views, 0.2).unwrap();
        assert!(result.rms_px > 0. && result.max_error_px < 0.2);
        assert!((result.intrinsics.fx - 920.).abs() < 2.);
        assert!(initialize_planar([1280, 720], &views, 1e-5).is_err());
    }

    #[test]
    fn rejects_repeated_orientation_collinear_and_duplicate_targets() {
        let (_, views, _) = observations(0., 0.);
        let repeated = vec![views[0].clone(); 4];
        assert!(initialize_planar([1280, 720], &repeated, 1.).is_err());
        let mut collinear = views.clone();
        for view in &mut collinear {
            view.observations.truncate(8);
        }
        assert!(initialize_planar([1280, 720], &collinear, 1.).is_err());
        let mut duplicate = views.clone();
        duplicate[0].observations[1].target_xy_mm = duplicate[0].observations[0].target_xy_mm;
        assert!(initialize_planar([1280, 720], &duplicate, 1.).is_err());
        let mut outlier = views.clone();
        outlier[0].observations[3].pixel[0] += 30.;
        assert!(initialize_planar([1280, 720], &outlier, 0.2).is_err());
    }

    #[test]
    fn metric_scale_and_target_origin_do_not_change_intrinsics() {
        let (_, views, poses) = observations(2., 0.);
        for scale in [0.001, 1000.] {
            let mut scaled = views.clone();
            for view in &mut scaled {
                for p in &mut view.observations {
                    for v in &mut p.target_xy_mm {
                        *v *= scale;
                    }
                }
            }
            let result = initialize_planar([1280, 720], &scaled, 1e-6).unwrap();
            assert!((result.intrinsics.fx - 920.).abs() < 1e-6);
            for (view, pose) in result.views.iter().zip(&poses) {
                let difference = Vector3::from(view.camera_from_target.translation_mm) / scale
                    - Vector3::from(pose.translation_mm);
                assert!(difference.norm() < 1e-5);
            }
        }
        // Move the target origin far outside observations, behind some cameras.
        // Choosing homography sign at that origin would reject a valid dataset.
        let offset = Vector3::new(-5000., 0., 0.);
        let mut shifted = views.clone();
        for view in &mut shifted {
            for p in &mut view.observations {
                p.target_xy_mm[0] -= offset[0];
            }
        }
        let result = initialize_planar([1280, 720], &shifted, 1e-5).unwrap();
        assert!((result.intrinsics.fx - 920.).abs() < 1e-5);
        let mut behind = false;
        for (view, pose) in result.views.iter().zip(&poses) {
            let expected = Vector3::from(pose.translation_mm) + pose.matrix() * offset;
            behind |= expected[2] < 0.;
            assert!(
                (Vector3::from(view.camera_from_target.translation_mm) - expected).norm() < 1e-4
            );
        }
        assert!(behind);
    }

    #[test]
    fn validates_bounds_nonfinite_and_resource_limits() {
        let (_, views, _) = observations(0., 0.);
        for limit in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(initialize_planar([1280, 720], &views, limit).is_err());
        }
        for size in [[0, 720], [1280, 0], [1_000_001, 720]] {
            assert!(initialize_planar(size, &views, 1.).is_err());
        }
        assert!(initialize_planar([1280, 720], &views[..2], 1.).is_err());
        assert!(initialize_planar([1280, 720], &vec![views[0].clone(); 65], 1.).is_err());
        for value in [f64::NAN, f64::INFINITY, 1e10] {
            let mut bad = views.clone();
            bad[0].observations[0].target_xy_mm[0] = value;
            assert!(initialize_planar([1280, 720], &bad, 1.).is_err());
        }
        for value in [f64::NAN, f64::INFINITY, -0.6, 1279.5] {
            let mut bad = views.clone();
            bad[0].observations[0].pixel[0] = value;
            assert!(initialize_planar([1280, 720], &bad, 1.).is_err());
        }
        let mut few = views.clone();
        few[0].observations.truncate(3);
        assert!(initialize_planar([1280, 720], &few, 1.).is_err());
        let mut many = views.clone();
        many[0]
            .observations
            .resize(4097, views[0].observations[0].clone());
        assert!(initialize_planar([1280, 720], &many, 1.).is_err());
    }
}
