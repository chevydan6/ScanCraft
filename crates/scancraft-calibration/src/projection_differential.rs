//! Shared Brown projection and local rigid-pose derivatives for calibration fitters.
use crate::{Brown5, planar::InitialIntrinsics, stereo::RigidPose};
use nalgebra::{Matrix3, Rotation3, SMatrix, SVector, Vector2, Vector3};
/// Forward projection and derivative with respect to camera-frame millimeters.
pub(crate) struct Projection {
    pub pixel: Vector2<f64>,
    pub normalized: [f64; 2],
    pub distorted: [f64; 2],
    pub point_jacobian: SMatrix<f64, 2, 3>,
}
/// Project a positive-depth point. Lens-domain checks remain the caller's policy.
pub(crate) fn project(
    k: &InitialIntrinsics,
    distortion: Brown5,
    point: Vector3<f64>,
) -> Result<Projection, String> {
    if !point.iter().all(|v| v.is_finite())
        || point[2] <= 0.
        || ![k.fx, k.fy, k.cx, k.cy, k.skew]
            .iter()
            .all(|v| v.is_finite())
        || k.fx <= 0.
        || k.fy <= 0.
    {
        return Err("invalid camera point or intrinsics".into());
    }
    let x = point[0] / point[2];
    let y = point[1] / point[2];
    let (d, j) = distortion.map(x, y)?;
    let pixel = Vector2::new(k.fx * d[0] + k.skew * d[1] + k.cx, k.fy * d[1] + k.cy);
    let image_normalized = SMatrix::<f64, 2, 2>::new(
        k.fx * j[0][0] + k.skew * j[1][0],
        k.fx * j[0][1] + k.skew * j[1][1],
        k.fy * j[1][0],
        k.fy * j[1][1],
    );
    let normalized_point = SMatrix::<f64, 2, 3>::new(
        1. / point[2],
        0.,
        -x / point[2],
        0.,
        1. / point[2],
        -y / point[2],
    );
    let point_jacobian = image_normalized * normalized_point;
    if !pixel
        .iter()
        .chain(point_jacobian.iter())
        .all(|v| v.is_finite())
    {
        return Err("projection differential overflow".into());
    }
    Ok(Projection {
        pixel,
        normalized: [x, y],
        distorted: d,
        point_jacobian,
    })
}
/// Derivative for R'=exp(dw)R, t'=t+dt; rotated excludes translation.
pub(crate) fn point_pose_jacobian(rotated: Vector3<f64>) -> SMatrix<f64, 3, 6> {
    let mut result = SMatrix::<f64, 3, 6>::zeros();
    result.fixed_view_mut::<3, 3>(0, 0).copy_from(&Matrix3::new(
        0.,
        rotated[2],
        -rotated[1],
        -rotated[2],
        0.,
        rotated[0],
        rotated[1],
        -rotated[0],
        0.,
    ));
    result
        .fixed_view_mut::<3, 3>(0, 3)
        .copy_from(&Matrix3::identity());
    result
}
/// Apply a finite local SO(3)/translation increment without changing pose conventions.
pub(crate) fn step_pose(pose: &RigidPose, step: SVector<f64, 6>) -> Result<RigidPose, String> {
    if !step.iter().all(|v| v.is_finite())
        || step[0].hypot(step[1]).hypot(step[2]) > std::f64::consts::PI
    {
        return Err("invalid or nonlocal pose step".into());
    }
    let r = Rotation3::new(Vector3::new(step[0], step[1], step[2])).matrix() * pose.matrix();
    let result = RigidPose {
        rotation: std::array::from_fn(|i| std::array::from_fn(|j| r[(i, j)])),
        translation_mm: std::array::from_fn(|i| pose.translation_mm[i] + step[i + 3]),
    };
    result.validate()?;
    Ok(result)
}
