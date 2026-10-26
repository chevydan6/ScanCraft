//! Two calibrated pinhole cameras, with explicit right-from-left SO(3) pose.
use crate::CameraModel;
use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RigidPose {
    pub rotation: [[f64; 3]; 3],
    pub translation_mm: [f64; 3],
}
impl RigidPose {
    pub fn matrix(&self) -> Matrix3<f64> {
        Matrix3::from_row_slice(&self.rotation.concat())
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self
            .rotation
            .iter()
            .flatten()
            .chain(self.translation_mm.iter())
            .all(|v| v.is_finite())
        {
            return Err("non-finite stereo extrinsics".into());
        }
        if self.rotation.iter().flatten().any(|v| v.abs() > 1. + 1e-8) {
            return Err("stereo rotation entries exceed SO(3) bounds".into());
        }
        let r = self.matrix();
        let error = (r.transpose() * r - Matrix3::identity()).norm();
        let determinant = r.determinant();
        if !error.is_finite()
            || !determinant.is_finite()
            || error > 1e-8
            || (determinant - 1.).abs() > 1e-8
        {
            return Err("stereo rotation is not right-handed orthonormal".into());
        }
        Ok(())
    }
    pub fn apply(&self, point: [f64; 3]) -> [f64; 3] {
        (self.matrix() * Vector3::from(point) + Vector3::from(self.translation_mm)).into()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StereoCalibration {
    pub left: CameraModel,
    pub right: CameraModel,
    pub right_from_left: RigidPose,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TriangulationLimits {
    pub min_depth_z_mm: f64,
    pub max_depth_z_mm: f64,
    pub min_ray_angle_rad: f64,
    pub max_ray_gap_mm: f64,
    pub max_reprojection_error_px: f64,
}
impl TriangulationLimits {
    pub fn validate(self) -> Result<(), String> {
        if ![
            self.min_depth_z_mm,
            self.max_depth_z_mm,
            self.min_ray_angle_rad,
            self.max_ray_gap_mm,
            self.max_reprojection_error_px,
        ]
        .iter()
        .all(|v| v.is_finite())
            || self.min_depth_z_mm <= 0.
            || self.max_depth_z_mm <= self.min_depth_z_mm
            || self.min_ray_angle_rad <= 0.
            || self.min_ray_angle_rad >= std::f64::consts::FRAC_PI_2
            || self.max_ray_gap_mm <= 0.
            || self.max_reprojection_error_px <= 0.
        {
            return Err("invalid triangulation acceptance limits".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriangulatedPoint {
    pub position_mm: [f64; 3],
    pub ray_gap_mm: f64,
    pub ray_angle_rad: f64,
    pub left_reprojection_px: f64,
    pub right_reprojection_px: f64,
}
fn norm(v: Vector3<f64>) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
impl StereoCalibration {
    pub fn validate(&self) -> Result<(), String> {
        self.left.validate()?;
        self.right.validate()?;
        self.right_from_left.validate()?;
        let baseline = norm(Vector3::from(self.right_from_left.translation_mm));
        if !baseline.is_finite() || baseline <= 1e-9 {
            return Err("stereo baseline is zero or unrepresentable".into());
        }
        Ok(())
    }
    pub fn prepare(&self, limits: TriangulationLimits) -> Result<PreparedStereo<'_>, String> {
        self.validate()?;
        limits.validate()?;
        let rotation = self.right_from_left.matrix();
        let inverse = rotation.transpose();
        let translation = Vector3::from(self.right_from_left.translation_mm);
        let right_origin = -inverse * translation;
        if !right_origin.iter().all(|v| v.is_finite()) {
            return Err("stereo camera origin overflow".into());
        }
        Ok(PreparedStereo {
            calibration: self,
            rotation,
            inverse,
            right_origin,
            translation,
            limits,
            min_sine_squared: limits.min_ray_angle_rad.sin().powi(2),
        })
    }
}
pub struct PreparedStereo<'a> {
    calibration: &'a StereoCalibration,
    rotation: Matrix3<f64>,
    inverse: Matrix3<f64>,
    right_origin: Vector3<f64>,
    translation: Vector3<f64>,
    limits: TriangulationLimits,
    min_sine_squared: f64,
}
impl PreparedStereo<'_> {
    pub fn triangulate(
        &self,
        left_pixel: [f64; 2],
        right_pixel: [f64; 2],
    ) -> Result<TriangulatedPoint, String> {
        let left = Vector3::from(self.calibration.left.ray_validated(left_pixel)?);
        let right_camera = Vector3::from(self.calibration.right.ray_validated(right_pixel)?);
        let mut right = self.inverse * right_camera;
        let length = norm(right);
        if !length.is_finite() || length == 0. {
            return Err("invalid transformed right ray".into());
        }
        right /= length;
        let cosine = left.dot(&right).clamp(-1., 1.);
        let denominator = 1. - cosine * cosine;
        if !denominator.is_finite() || denominator <= 0. || denominator < self.min_sine_squared {
            return Err("stereo rays have insufficient parallax".into());
        }
        let a = left.dot(&self.right_origin);
        let b = right.dot(&self.right_origin);
        let s = (a - cosine * b) / denominator;
        let t = (cosine * a - b) / denominator;
        if !s.is_finite() || !t.is_finite() || s <= 0. || t <= 0. {
            return Err("stereo match triangulates behind a camera".into());
        }
        let on_left = left * s;
        let on_right = self.right_origin + right * t;
        let midpoint = on_left / 2. + on_right / 2.;
        let right_point = self.rotation * midpoint + self.translation;
        if !midpoint
            .iter()
            .chain(right_point.iter())
            .all(|v| v.is_finite())
            || midpoint[2] < self.limits.min_depth_z_mm
            || midpoint[2] > self.limits.max_depth_z_mm
            || right_point[2] <= 0.
        {
            return Err("triangulated point is non-finite or outside the depth range".into());
        }
        let gap = norm(on_left - on_right);
        if !gap.is_finite() || gap > self.limits.max_ray_gap_mm {
            return Err("stereo rays do not meet within the allowed gap".into());
        }
        let lp = self.calibration.left.project_validated(midpoint.into())?;
        let rp = self
            .calibration
            .right
            .project_validated(right_point.into())?;
        let left_error = (lp[0] - left_pixel[0]).hypot(lp[1] - left_pixel[1]);
        let right_error = (rp[0] - right_pixel[0]).hypot(rp[1] - right_pixel[1]);
        if !left_error.is_finite()
            || !right_error.is_finite()
            || left_error > self.limits.max_reprojection_error_px
            || right_error > self.limits.max_reprojection_error_px
        {
            return Err("stereo reprojection error exceeds limit".into());
        }
        Ok(TriangulatedPoint {
            position_mm: midpoint.into(),
            ray_gap_mm: gap,
            ray_angle_rad: cosine.acos(),
            left_reprojection_px: left_error,
            right_reprojection_px: right_error,
        })
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    pub fn rig() -> StereoCalibration {
        StereoCalibration {
            left: crate::fixtures::camera(),
            right: crate::fixtures::camera(),
            right_from_left: RigidPose {
                rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                translation_mm: [-120., 0., 0.],
            },
        }
    }
    pub fn limits() -> TriangulationLimits {
        TriangulationLimits {
            min_depth_z_mm: 10.,
            max_depth_z_mm: 2000.,
            min_ray_angle_rad: 0.0001,
            max_ray_gap_mm: 0.1,
            max_reprojection_error_px: 0.1,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_grid_reconstruction_uses_real_extrinsic_units() {
        let rig = fixtures::rig();
        let prepared = rig.prepare(fixtures::limits()).unwrap();
        for z in [300., 500., 1000.] {
            for x in [-40., 0., 80.] {
                for y in [-50., 0., 50.] {
                    let p = [x, y, z];
                    let left = rig.left.project(p).unwrap();
                    let right = rig.right.project(rig.right_from_left.apply(p)).unwrap();
                    let point = prepared.triangulate(left, right).unwrap();
                    assert!(norm(Vector3::from(point.position_mm) - Vector3::from(p)) < 1e-7);
                    assert!(point.ray_gap_mm < 1e-7);
                }
            }
        }
    }
    #[test]
    fn rotated_asymmetric_camera_pair() {
        let mut rig = fixtures::rig();
        let a = 0.03f64;
        rig.right_from_left.rotation = [
            [a.cos(), 0., a.sin()],
            [0., 1., 0.],
            [-a.sin(), 0., a.cos()],
        ];
        rig.right.fx = 1100.;
        rig.right.fy = 1050.;
        rig.right.cx = 630.;
        rig.right_from_left.translation_mm = [-120., 2., 0.5];
        let p = [20., 30., 600.];
        let result = rig
            .prepare(fixtures::limits())
            .unwrap()
            .triangulate(
                rig.left.project(p).unwrap(),
                rig.right.project(rig.right_from_left.apply(p)).unwrap(),
            )
            .unwrap();
        assert!(norm(Vector3::from(result.position_mm) - Vector3::from(p)) < 1e-7);
    }
    #[test]
    fn reject_parallel_behind_bad_match_and_bad_rotation() {
        let rig = fixtures::rig();
        let prep = rig.prepare(fixtures::limits()).unwrap();
        assert!(prep.triangulate([639.5, 399.5], [639.5, 399.5]).is_err());
        assert!(prep.triangulate([600., 399.5], [700., 399.5]).is_err());
        assert!(prep.triangulate([639.5, 399.5], [400., 600.]).is_err());
        let mut invalid = rig;
        invalid.right_from_left.rotation[0][0] = -1.;
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn overflowed_or_nonfinite_rotations_are_rejected() {
        let mut rig = fixtures::rig();
        rig.right_from_left.rotation = [[1e308; 3]; 3];
        assert!(rig.validate().is_err());
        rig.right_from_left.rotation[0][0] = f64::NAN;
        assert!(rig.validate().is_err());
    }
    #[test]
    fn depth_and_reprojection_limits_are_enforced_separately() {
        let mut rig = fixtures::rig();
        rig.left.distortion = crate::Brown5::ZERO;
        rig.right.distortion = crate::Brown5::ZERO;
        assert!(
            rig.prepare(fixtures::limits())
                .unwrap()
                .triangulate([639.5, 399.5], [629.5, 399.5])
                .unwrap_err()
                .contains("depth range")
        );
        let mut limits = fixtures::limits();
        limits.max_ray_gap_mm = 1000.;
        limits.max_reprojection_error_px = 1e-8;
        assert!(
            rig.prepare(limits)
                .unwrap()
                .triangulate([639.5, 399.5], [399.5, 401.])
                .unwrap_err()
                .contains("reprojection")
        );
    }
    #[test]
    fn rectified_disparity_depth_matches_analytic_formula() {
        let mut rig = fixtures::rig();
        rig.left.distortion = crate::Brown5::ZERO;
        rig.right.distortion = crate::Brown5::ZERO;
        let result = rig
            .prepare(fixtures::limits())
            .unwrap()
            .triangulate([639.5, 399.5], [399.5, 399.5])
            .unwrap();
        assert!((result.position_mm[2] - 1000. * 120. / 240.).abs() < 1e-8);
    }
}
