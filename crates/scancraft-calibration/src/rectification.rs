//! Calibrated horizontal rectification with an explicit output grid. No automatic
//! crop, calibration estimation, exposure synchronization or image correspondence.
use crate::{
    Brown5, CameraModel,
    stereo::{RigidPose, StereoCalibration},
};
use nalgebra::{Matrix3, Vector3};
use serde::Serialize;

fn norm(v: Vector3<f64>) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn pose(matrix: Matrix3<f64>) -> Result<RigidPose, String> {
    let result = RigidPose {
        rotation: std::array::from_fn(|r| std::array::from_fn(|c| matrix[(r, c)])),
        translation_mm: [0.; 3],
    };
    result.validate()?;
    Ok(result)
}
/// Constructed only through validation; serialized diagnostics cannot bypass the
/// constructor by deserializing arbitrary mutually inconsistent matrices.
#[derive(Debug, Clone, Serialize)]
pub struct StereoRectification {
    source: StereoCalibration,
    rectified: StereoCalibration,
    /// Ray rotation from rectified left axes to original left camera axes.
    left_from_rectified: RigidPose,
    /// Ray rotation from rectified right axes to original right camera axes.
    right_from_rectified: RigidPose,
}
impl StereoRectification {
    pub fn build(source: &StereoCalibration, grid: CameraModel) -> Result<Self, String> {
        source.validate()?;
        grid.validate()?;
        if grid.distortion != Brown5::ZERO || grid.skew != 0. {
            return Err(
                "rectified output requires an explicit zero-distortion, zero-skew pinhole grid"
                    .into(),
            );
        }
        let right_rotation = source.right_from_left.matrix();
        let center =
            -right_rotation.transpose() * Vector3::from(source.right_from_left.translation_mm);
        let baseline = norm(center);
        if !baseline.is_finite() || baseline <= 1e-9 {
            return Err("invalid rectification baseline".into());
        }
        // New X axis follows the actual camera-center baseline in original left axes.
        let ex = center / baseline;
        // Resolve roll using the mean of the original forward optical directions.
        let forward = Vector3::new(0., 0., 1.);
        let hint = forward + right_rotation.transpose() * forward;
        let perpendicular = hint - ex * hint.dot(&ex);
        let length = norm(perpendicular);
        if !length.is_finite() || length <= 1e-8 {
            return Err("degenerate rectification: baseline/optical axes do not define a stable common view".into());
        }
        let ez_hint = perpendicular / length;
        let ey_raw = ez_hint.cross(&ex);
        let ey = ey_raw / norm(ey_raw);
        let ez = ex.cross(&ey);
        if ez.dot(&forward) <= 1e-8 || ez.dot(&(right_rotation.transpose() * forward)) <= 1e-8 {
            return Err("rectified forward axis does not face both source cameras".into());
        }
        let left_rotation = Matrix3::from_columns(&[ex, ey, ez]);
        let left_from_rectified = pose(left_rotation)?;
        let right_from_rectified = pose(right_rotation * left_rotation)?;
        let rectified = StereoCalibration {
            left: grid.clone(),
            right: grid,
            right_from_left: RigidPose {
                rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                translation_mm: [-baseline, 0., 0.],
            },
        };
        rectified.validate()?;
        Ok(Self {
            source: source.clone(),
            rectified,
            left_from_rectified,
            right_from_rectified,
        })
    }
    pub fn source(&self) -> &StereoCalibration {
        &self.source
    }
    pub fn geometry(&self) -> &StereoCalibration {
        &self.rectified
    }
    pub fn left_from_rectified(&self) -> &RigidPose {
        &self.left_from_rectified
    }
    pub fn right_from_rectified(&self) -> &RigidPose {
        &self.right_from_rectified
    }
    /// Coordinates in the rectified-left millimeter frame -> original left frame.
    pub fn point_in_source_left(&self, point: [f64; 3]) -> Result<[f64; 3], String> {
        let mapped = self.left_from_rectified.apply(point);
        if !point.iter().chain(mapped.iter()).all(|v| v.is_finite()) {
            return Err("non-finite rectified point transform".into());
        }
        Ok(mapped)
    }
    /// Map ideal matched pixel centers back to original distorted observations.
    /// Triangulate these using the original calibration to retain its axes and
    /// original-camera depth/reprojection limits. Does not prove a match is valid.
    pub fn source_pixels(
        &self,
        left: [f64; 2],
        right: [f64; 2],
    ) -> Result<([f64; 2], [f64; 2]), String> {
        let map = |pixel,
                   ideal: &CameraModel,
                   original: &CameraModel,
                   rotation: &RigidPose|
         -> Result<[f64; 2], String> {
            let ray = ideal.ray(pixel)?;
            let projected = original.project(rotation.apply(ray))?;
            if !original.contains_pixel(projected) {
                return Err("rectified observation maps outside the source image".into());
            }
            Ok(projected)
        };
        Ok((
            map(
                left,
                &self.rectified.left,
                &self.source.left,
                &self.left_from_rectified,
            )?,
            map(
                right,
                &self.rectified.right,
                &self.source.right,
                &self.right_from_rectified,
            )?,
        ))
    }
    pub fn rectify_pixels(
        &self,
        left: [f64; 2],
        right: [f64; 2],
    ) -> Result<([f64; 2], [f64; 2]), String> {
        let map = |pixel,
                   original: &CameraModel,
                   ideal: &CameraModel,
                   rotation: &RigidPose|
         -> Result<[f64; 2], String> {
            let ray = Vector3::from(original.ray(pixel)?);
            let projected = ideal.project((rotation.matrix().transpose() * ray).into())?;
            if !ideal.contains_pixel(projected) {
                return Err("source observation maps outside the rectified image".into());
            }
            Ok(projected)
        };
        Ok((
            map(
                left,
                &self.source.left,
                &self.rectified.left,
                &self.left_from_rectified,
            )?,
            map(
                right,
                &self.source.right,
                &self.rectified.right,
                &self.right_from_rectified,
            )?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stereo::fixtures;
    fn grid() -> CameraModel {
        let mut c = fixtures::rig().left;
        c.distortion = Brown5::ZERO;
        c.skew = 0.;
        c
    }
    #[test]
    fn identity_and_vertical_and_reversed_baselines_preserve_epipolar_geometry() {
        for translation in [
            [-120., 0., 0.],
            [0., -120., 0.],
            [120., 0., 0.],
            [-120., 3., 2.],
        ] {
            let mut source = fixtures::rig();
            source.right_from_left.translation_mm = translation;
            let rect = StereoRectification::build(&source, grid()).unwrap();
            let p = [20., 10., 600.];
            let original_left = source.left.project(p).unwrap();
            let original_right = source
                .right
                .project(source.right_from_left.apply(p))
                .unwrap();
            let (l, r) = rect.rectify_pixels(original_left, original_right).unwrap();
            assert!((l[1] - r[1]).abs() < 1e-8);
            let q = rect
                .geometry()
                .prepare(fixtures::limits())
                .unwrap()
                .triangulate(l, r)
                .unwrap();
            let restored = rect.point_in_source_left(q.position_mm).unwrap();
            assert!(norm(Vector3::from(restored) - Vector3::from(p)) < 1e-7);
            let (sl, sr) = rect.source_pixels(l, r).unwrap();
            for i in 0..2 {
                assert!((sl[i] - original_left[i]).abs() < 1e-8);
                assert!((sr[i] - original_right[i]).abs() < 1e-8);
            }
        }
    }
    #[test]
    fn rotated_asymmetric_distorted_cameras_reconstruct_in_original_axes() {
        let mut source = fixtures::rig();
        let a = 0.07f64;
        source.right_from_left.rotation = [
            [a.cos(), 0., a.sin()],
            [0., 1., 0.],
            [-a.sin(), 0., a.cos()],
        ];
        source.right_from_left.translation_mm = [-120., 5., 2.];
        source.right.fx = 1073.;
        source.right.fy = 1009.;
        source.right.cx = 643.2;
        source.left.skew = 0.23;
        source.right.skew = -0.17;
        let rect = StereoRectification::build(&source, grid()).unwrap();
        for z in [400., 800., 1200.] {
            for x in [-30., 0., 60.] {
                let p = [x, 25., z];
                let lp = source.left.project(p).unwrap();
                let rp = source
                    .right
                    .project(source.right_from_left.apply(p))
                    .unwrap();
                let (l, r) = rect.rectify_pixels(lp, rp).unwrap();
                assert!((l[1] - r[1]).abs() < 1e-8);
                let (sl, sr) = rect.source_pixels(l, r).unwrap();
                let result = source
                    .prepare(fixtures::limits())
                    .unwrap()
                    .triangulate(sl, sr)
                    .unwrap();
                assert!(norm(Vector3::from(result.position_mm) - Vector3::from(p)) < 1e-6);
            }
        }
    }
    #[test]
    fn bad_grids_nonfinite_poses_and_degenerate_baselines_are_rejected() {
        let mut source = fixtures::rig();
        let mut bad = grid();
        bad.skew = 0.1;
        assert!(StereoRectification::build(&source, bad).is_err());
        bad = grid();
        bad.distortion.k1 = 0.01;
        assert!(StereoRectification::build(&source, bad).is_err());
        source.right_from_left.translation_mm = [0., 0., -120.];
        assert!(StereoRectification::build(&source, grid()).is_err());
        source.right_from_left.translation_mm = [f64::NAN, 0., 0.];
        assert!(StereoRectification::build(&source, grid()).is_err());
        source = fixtures::rig();
        source.right_from_left.rotation = [[-1., 0., 0.], [0., 1., 0.], [0., 0., -1.]];
        assert!(StereoRectification::build(&source, grid()).is_err());
    }
    #[test]
    fn unsupported_observations_and_nonfinite_points_fail() {
        let rect = StereoRectification::build(&fixtures::rig(), grid()).unwrap();
        assert!(rect.source_pixels([-10., 0.], [0., 0.]).is_err());
        assert!(rect.rectify_pixels([f64::NAN, 0.], [0., 0.]).is_err());
        assert!(rect.point_in_source_left([f64::INFINITY, 0., 0.]).is_err());
    }
}
