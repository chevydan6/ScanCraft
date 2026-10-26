//! Explicit numerical image/basis reflection. Does not establish device binding,
//! physical axes, calibration coverage, units, or accuracy. Never activates a model.
use crate::CameraModel;

/// Reflect a pixel center in a finite image domain. Applying twice is identity
/// up to floating-point subtraction rounding. Does not alter source frame bytes.
pub fn reflect_pixel_x(size: [u32; 2], pixel: [f64; 2]) -> Result<[f64; 2], String> {
    if size.contains(&0)
        || !pixel.iter().all(|v| v.is_finite())
        || (0..2).any(|i| pixel[i] < 0. || pixel[i] > f64::from(size[i] - 1))
    {
        return Err("reflection requires a pixel center within a nonempty image".into());
    }
    Ok([f64::from(size[0] - 1) - pixel[0], pixel[1]])
}

/// Change both image X and camera-basis X signs. For S=diag(-1,1,1),
/// project(reflected_model, S*point) = [width-1-project(model,point).x, y].
/// S has determinant -1: it is not a camera rotation or a physical pose.
/// Input and output are numerically validated; provenance is not inferred.
pub fn reflect_camera_x(camera: &CameraModel) -> Result<CameraModel, String> {
    camera.validate()?;
    let mut reflected = camera.clone();
    reflected.cx = f64::from(camera.width - 1) - camera.cx;
    reflected.skew = -camera.skew;
    reflected.distortion.p2 = -camera.distortion.p2;
    reflected.validate()?;
    Ok(reflected)
}

/// Reflect both camera bases and the left-camera scene basis in X. Transforms
/// R to S R S and t to S t; the resulting relative rotation remains in SO(3).
/// Reconstructed scene coordinates are reflected, including handedness. This is
/// an explicit numerical conversion, not a calibration/binding qualification.
pub fn reflect_stereo_basis_x(
    stereo: &crate::stereo::StereoCalibration,
) -> Result<crate::stereo::StereoCalibration, String> {
    stereo.validate()?;
    let sign = [-1., 1., 1.];
    let mut reflected = stereo.clone();
    reflected.left = reflect_camera_x(&stereo.left)?;
    reflected.right = reflect_camera_x(&stereo.right)?;
    for i in 0..3 {
        for j in 0..3 {
            reflected.right_from_left.rotation[i][j] =
                sign[i] * stereo.right_from_left.rotation[i][j] * sign[j];
        }
        reflected.right_from_left.translation_mm[i] =
            sign[i] * stereo.right_from_left.translation_mm[i];
    }
    reflected.validate()?;
    Ok(reflected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Brown5;

    #[test]
    fn distorted_projection_and_inverse_rays_commute_with_reflection() {
        let camera = CameraModel {
            width: 1920,
            height: 1200,
            fx: 1700.,
            fy: 1720.,
            cx: 945.3,
            cy: 603.2,
            skew: 7.5,
            distortion: Brown5 {
                k1: 0.02,
                k2: -0.003,
                k3: 0.0002,
                p1: 0.001,
                p2: -0.002,
            },
            valid_radius: 0.8,
        };
        let reflected = reflect_camera_x(&camera).unwrap();
        for x in -4..=4 {
            for y in -3..=3 {
                for depth in [200., 750.] {
                    let point = [
                        f64::from(x) * 0.05 * depth,
                        f64::from(y) * 0.05 * depth,
                        depth,
                    ];
                    let pixel = camera.project(point).unwrap();
                    let expected = reflect_pixel_x([1920, 1200], pixel).unwrap();
                    let actual = reflected.project([-point[0], point[1], point[2]]).unwrap();
                    for i in 0..2 {
                        assert!((expected[i] - actual[i]).abs() < 1e-10);
                    }
                    let ray = camera.ray(pixel).unwrap();
                    let other = reflected.ray(expected).unwrap();
                    for (a, b) in other.into_iter().zip([-ray[0], ray[1], ray[2]]) {
                        assert!((a - b).abs() < 1e-9);
                    }
                }
            }
        }
        let restored = reflect_camera_x(&reflected).unwrap();
        assert!((restored.cx - camera.cx).abs() < 1e-12);
        assert_eq!(restored.skew, camera.skew);
        assert_eq!(restored.distortion, camera.distortion);
    }

    #[test]
    fn pixel_endpoints_and_invalid_domains() {
        assert_eq!(
            reflect_pixel_x([1920, 1200], [0., 1199.]).unwrap(),
            [1919., 1199.]
        );
        assert_eq!(reflect_pixel_x([1, 1], [0., 0.]).unwrap(), [0., 0.]);
        for (size, pixel) in [
            ([0, 1], [0., 0.]),
            ([1, 0], [0., 0.]),
            ([1920, 1200], [1919.5, 1.]),
            ([1920, 1200], [1., 1200.]),
            ([1920, 1200], [-0.1, 1.]),
            ([1920, 1200], [f64::NAN, 1.]),
        ] {
            assert!(reflect_pixel_x(size, pixel).is_err());
        }
    }

    #[test]
    fn stereo_pose_projection_and_triangulation_reflect_together() {
        use crate::stereo::{RigidPose, StereoCalibration, TriangulationLimits};
        let camera = CameraModel {
            width: 1920,
            height: 1200,
            fx: 1700.,
            fy: 1710.,
            cx: 945.,
            cy: 602.,
            skew: 3.,
            distortion: Brown5 {
                k1: 0.01,
                k2: 0.001,
                k3: 0.,
                p1: 0.001,
                p2: -0.002,
            },
            valid_radius: 0.8,
        };
        let (s, c) = 0.08f64.sin_cos();
        let stereo = StereoCalibration {
            left: camera.clone(),
            right: camera,
            right_from_left: RigidPose {
                rotation: [[c, 0., s], [0., 1., 0.], [-s, 0., c]],
                translation_mm: [-100., 2., 5.],
            },
        };
        let reflected = reflect_stereo_basis_x(&stereo).unwrap();
        let limits = TriangulationLimits {
            min_depth_z_mm: 100.,
            max_depth_z_mm: 1000.,
            min_ray_angle_rad: 0.001,
            max_ray_gap_mm: 1e-5,
            max_reprojection_error_px: 1e-5,
        };
        let prepared = reflected.prepare(limits).unwrap();
        for x in [-30., 0., 30.] {
            for y in [-20., 20.] {
                for z in [400., 700.] {
                    let point = [x, y, z];
                    let right = stereo.right_from_left.apply(point);
                    let left_pixel =
                        reflect_pixel_x([1920, 1200], stereo.left.project(point).unwrap()).unwrap();
                    let right_pixel =
                        reflect_pixel_x([1920, 1200], stereo.right.project(right).unwrap())
                            .unwrap();
                    let new_right = reflected.right_from_left.apply([-x, y, z]);
                    for (a, b) in new_right.into_iter().zip([-right[0], right[1], right[2]]) {
                        assert!((a - b).abs() < 1e-12);
                    }
                    let reconstructed = prepared.triangulate(left_pixel, right_pixel).unwrap();
                    for (a, b) in reconstructed.position_mm.into_iter().zip([-x, y, z]) {
                        assert!((a - b).abs() < 1e-6);
                    }
                }
            }
        }
        let restored = reflect_stereo_basis_x(&reflected).unwrap();
        assert_eq!(restored, stereo);
        let mut invalid = stereo;
        invalid.right_from_left.rotation[0][0] = -1.;
        assert!(reflect_stereo_basis_x(&invalid).is_err());
    }
}
