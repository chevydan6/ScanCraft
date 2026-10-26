//! Epipolar constraints on undistorted pinhole pixel coordinates. Distorted
//! image pixels must be undistorted with the appropriate camera model first.
use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpipolarMatrices {
    pub essential: [[f64; 3]; 3],
    pub fundamental: [[f64; 3]; 3],
}
fn rows(m: Matrix3<f64>) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| m[(i, j)]))
}
/// Pure numerical formula on explicitly ordered fields. Unlike
/// `from_calibration`, this diagnostic does not validate SO(3), assign camera
/// interfaces or translation units, or verify physical calibration. Zero
/// translation is allowed here and gives zero matrices (no usable constraint).
pub fn raw_field_matrices(
    intrinsics: [[f64; 4]; 2],
    rotation: [[f64; 3]; 3],
    translation: [f64; 3],
) -> Result<EpipolarMatrices, String> {
    if !intrinsics
        .iter()
        .flatten()
        .chain(rotation.iter().flatten())
        .chain(&translation)
        .all(|v| v.is_finite())
        || intrinsics.iter().any(|k| k[0] <= 0. || k[1] <= 0.)
    {
        return Err("invalid epipolar numeric fields".into());
    }
    let inverse = |k: [f64; 4]| {
        Matrix3::new(
            1. / k[0],
            0.,
            -k[2] / k[0],
            0.,
            1. / k[1],
            -k[3] / k[1],
            0.,
            0.,
            1.,
        )
    };
    let [x, y, z] = translation;
    let cross = Matrix3::new(0., -z, y, z, 0., -x, -y, x, 0.);
    let e = cross * Matrix3::from_row_slice(&rotation.concat());
    let f = inverse(intrinsics[1]).transpose() * e * inverse(intrinsics[0]);
    if !e.iter().chain(f.iter()).all(|v| v.is_finite()) {
        return Err("epipolar matrix overflow".into());
    }
    Ok(EpipolarMatrices {
        essential: rows(e),
        fundamental: rows(f),
    })
}
impl EpipolarMatrices {
    /// Validates the full application's calibration first. Translation is
    /// normalized since its magnitude does not affect epipolar constraints.
    pub fn from_calibration(
        calibration: &crate::stereo::StereoCalibration,
    ) -> Result<Self, String> {
        calibration.validate()?;
        if calibration.left.skew != 0. || calibration.right.skew != 0. {
            return Err("epipolar constraints currently require zero camera skew".into());
        }
        let t = Vector3::from(calibration.right_from_left.translation_mm);
        let baseline = t[0].hypot(t[1]).hypot(t[2]);
        raw_field_matrices(
            [&calibration.left, &calibration.right].map(|c| [c.fx, c.fy, c.cx, c.cy]),
            calibration.right_from_left.rotation,
            (t / baseline).into(),
        )
    }
    /// First-order Sampson distance in undistorted pixel units. This is a
    /// correspondence gate, not a reprojection error or physical accuracy claim.
    pub fn sampson_distance_pixels(
        &self,
        first: [f64; 2],
        second: [f64; 2],
    ) -> Result<f64, String> {
        if !first
            .iter()
            .chain(&second)
            .chain(self.fundamental.iter().flatten())
            .all(|v| v.is_finite())
        {
            return Err("nonfinite epipolar constraint input".into());
        }
        let scale = self
            .fundamental
            .iter()
            .flatten()
            .map(|v| v.abs())
            .fold(0f64, f64::max);
        if scale == 0. {
            return Err("degenerate epipolar matrix".into());
        }
        let f = Matrix3::from_row_slice(&self.fundamental.concat()) / scale;
        let a = Vector3::new(first[0], first[1], 1.);
        let b = Vector3::new(second[0], second[1], 1.);
        let line_b = f * a;
        let line_a = f.transpose() * b;
        let denominator = line_b[0].hypot(line_b[1]).hypot(line_a[0]).hypot(line_a[1]);
        let numerator = b.dot(&line_b).abs();
        if !denominator.is_finite() || denominator == 0. || !numerator.is_finite() {
            return Err("degenerate or overflowing epipolar distance".into());
        }
        let distance = numerator / denominator;
        if !distance.is_finite() {
            return Err("epipolar distance overflow".into());
        }
        Ok(distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Deserialize)]
    struct ReferenceCase {
        intrinsics: [[f64; 4]; 2],
        rotation: [[f64; 3]; 3],
        translation_raw_units: [f64; 3],
        essential: [[f64; 3]; 3],
        fundamental: [[f64; 3]; 3],
    }
    #[test]
    fn matches_six_synthetic_reference_cases_without_transposing() {
        let cases: Vec<ReferenceCase> = serde_json::from_str(include_str!(
            "../tests/fixtures/factory-laser-epipolar.json"
        ))
        .unwrap();
        assert_eq!(cases.len(), 6);
        for c in cases {
            let computed =
                raw_field_matrices(c.intrinsics, c.rotation, c.translation_raw_units).unwrap();
            for (a, b) in computed
                .essential
                .iter()
                .flatten()
                .zip(c.essential.iter().flatten())
            {
                assert!((a - b).abs() < 3e-6);
            }
            for (a, b) in computed
                .fundamental
                .iter()
                .flatten()
                .zip(c.fundamental.iter().flatten())
            {
                assert!((a - b).abs() < 4e-6);
            }
        }
    }
    #[test]
    fn projected_correspondences_and_pixel_outliers_obey_constraint() {
        let k = [[800., 900., 320., 240.], [850., 950., 300., 250.]];
        let r = [[0.8, 0., 0.6], [0., 1., 0.], [-0.6, 0., 0.8]];
        let t = [-100., 2., 5.];
        let camera = |k: [f64; 4]| crate::CameraModel {
            width: 640,
            height: 480,
            fx: k[0],
            fy: k[1],
            cx: k[2],
            cy: k[3],
            skew: 0.,
            distortion: crate::Brown5::ZERO,
            valid_radius: 1.,
        };
        let mut calibration = crate::stereo::StereoCalibration {
            left: camera(k[0]),
            right: camera(k[1]),
            right_from_left: crate::stereo::RigidPose {
                rotation: r,
                translation_mm: t,
            },
        };
        let matrix = EpipolarMatrices::from_calibration(&calibration).unwrap();
        calibration.right_from_left.rotation[0][0] = 1.2;
        assert!(EpipolarMatrices::from_calibration(&calibration).is_err());
        for point in [[0., 0., 500.], [35., -27., 600.], [-45., 71., 900.]] {
            let second =
                Matrix3::from_row_slice(&r.concat()) * Vector3::from(point) + Vector3::from(t);
            let project =
                |p: [f64; 3], k: [f64; 4]| [k[0] * p[0] / p[2] + k[2], k[1] * p[1] / p[2] + k[3]];
            let a = project(point, k[0]);
            let b = project(second.into(), k[1]);
            assert!(matrix.sampson_distance_pixels(a, b).unwrap() < 1e-10);
            let bad = [b[0], b[1] + 10.];
            assert!(matrix.sampson_distance_pixels(a, bad).unwrap() > 4.);
            let mut scaled = matrix.clone();
            for value in scaled.fundamental.iter_mut().flatten() {
                *value *= -1e100;
            }
            assert!(
                (matrix.sampson_distance_pixels(a, bad).unwrap()
                    - scaled.sampson_distance_pixels(a, bad).unwrap())
                .abs()
                    < 1e-10
            );
        }
    }
    #[test]
    fn rejects_invalid_and_degenerate_inputs() {
        let r = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let k = [[800., 900., 320., 240.]; 2];
        let zero = raw_field_matrices(k, r, [0.; 3]).unwrap();
        assert!(zero.sampson_distance_pixels([1.; 2], [2.; 2]).is_err());
        for v in [f64::NAN, f64::INFINITY, 0., -1.] {
            let mut bad = k;
            bad[0][0] = v;
            assert!(raw_field_matrices(bad, r, [1., 0., 0.]).is_err());
        }
        let good = raw_field_matrices(k, r, [1., 0., 0.]).unwrap();
        assert!(
            good.sampson_distance_pixels([f64::NAN, 0.], [2.; 2])
                .is_err()
        );
    }
}
