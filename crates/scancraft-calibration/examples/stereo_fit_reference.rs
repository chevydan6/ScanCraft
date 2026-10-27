//! Synthetic stereo-fitting and holdout triangulation evidence; no scanner access.
use nalgebra::Rotation3;
use scancraft_calibration::{
    Brown5, CameraModel,
    stereo::{RigidPose, StereoCalibration, TriangulationLimits},
    stereo_fit::{StereoFitOptions, StereoObservation, StereoView, fit_stereo_extrinsics},
};
use serde_json::json;
fn main() -> Result<(), String> {
    let mut cases = Vec::new();
    for (name, skew, noise) in [
        ("exact_zero_skew", false, 0.),
        ("exact_nonzero_skew", true, 0.),
        ("noisy_zero_skew", false, 0.02),
    ] {
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
                    let mut lp = left.project(point)?;
                    let mut rp = right.project(rig.apply(point))?;
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
        let options = StereoFitOptions {
            max_reprojection_error_px: 0.1,
            ..Default::default()
        };
        let fit = fit_stereo_extrinsics(&left, &right, &views, &options, &|| false)?;
        let fitted = StereoCalibration {
            left: left.clone(),
            right: right.clone(),
            right_from_left: fit.right_from_left.clone(),
        };
        let prepared = fitted.prepare(TriangulationLimits {
            min_depth_z_mm: 10.,
            max_depth_z_mm: 2000.,
            min_ray_angle_rad: 1e-5,
            max_ray_gap_mm: 1.,
            max_reprojection_error_px: 1.,
        })?;
        let mut holdouts = Vec::new();
        for z in [400., 650., 900.] {
            for x in [-50., 50.] {
                for y in [-30., 30.] {
                    let point = [x, y, z];
                    let lp = left.project(point)?;
                    let rp = right.project(rig.apply(point))?;
                    let reconstructed = prepared.triangulate(lp, rp)?;
                    holdouts.push(json!({"ground_truth_mm":point,"left_pixel":lp,"right_pixel":rp,"reconstructed_mm":reconstructed.position_mm}));
                }
            }
        }
        cases.push(json!({"name":name,"noise_amplitude_px":noise,"truth":{"right_from_left":rig,"left_from_target":poses},"request":{"schema_version":1,"provenance":"synthetic_fixture","fixed_left":left,"fixed_right":right,"views":views,"fitting":options},"fit":fit,"holdouts":holdouts}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"schema":1,"provenance":"synthetic_fixture_not_scanner_calibration","cases":cases})).map_err(|e|e.to_string())?);
    Ok(())
}
