//! Synthetic Brown5 fitting observations only; never device calibration.
use nalgebra::Rotation3;
use scancraft_calibration::{
    Brown5, CameraModel,
    planar::{PlanarObservation, PlanarView, initialize_planar},
    refinement::{RefinementOptions, refine_planar},
    stereo::RigidPose,
};
use serde_json::json;
fn main() -> Result<(), String> {
    let mut cases = Vec::new();
    for (name, skew, noise) in [
        ("exact_zero_skew", 0., 0.),
        ("exact_nonzero_skew", 6., 0.),
        ("noisy_zero_skew", 0., 0.02),
    ] {
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
                    let mut pixel = camera.project(pose.apply([target[0], target[1], 0.]))?;
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
        let options = RefinementOptions {
            estimate_skew: skew != 0.,
            max_reprojection_error_px: 0.1,
            ..Default::default()
        };
        let initial = initialize_planar([1280, 800], &views, 50.)?;
        let fit = refine_planar(&views, &initial, &options, &|| false)?;
        cases.push(json!({"name":name,"noise_amplitude_px":noise,"truth":{"camera":camera,"poses":poses},
            "request":{"schema_version":1,"provenance":"synthetic_fixture","image_size":[1280,800],"views":views,"initial_max_reprojection_error_px":50.,"refinement":options},
            "initialization":initial,"refinement":fit}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"schema":1,"provenance":"synthetic_fixture_not_scanner_calibration","cases":cases})).map_err(|e|e.to_string())?);
    Ok(())
}
