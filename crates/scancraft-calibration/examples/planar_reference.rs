//! Synthetic observations only. No device access or calibration-file creation.
use nalgebra::{Matrix3, Rotation3, Vector3};
use scancraft_calibration::{
    planar::{PlanarObservation, PlanarView, initialize_planar},
    stereo::RigidPose,
};
use serde_json::json;

fn main() -> Result<(), String> {
    let mut cases = Vec::new();
    for (name, skew, noise) in [
        ("exact_zero_skew", 0., 0.),
        ("exact_nonzero_skew", 7.5, 0.),
        ("noisy_zero_skew", 0., 0.02),
    ] {
        let k = Matrix3::new(920., skew, 621., 0., 870., 349., 0., 0., 1.);
        let mut views = Vec::new();
        let mut poses = Vec::new();
        for i in 0..8 {
            let f = i as f64;
            let rotation =
                Rotation3::from_euler_angles(-0.4 + f * 0.105, 0.32 - f * 0.08, -0.15 + f * 0.045);
            let translation = Vector3::new(-25. + f * 6., 14. - f * 3., 390. + f * 32.);
            let mut observations = Vec::new();
            for row in 0..7 {
                for col in 0..9 {
                    let target = [(col as f64 - 4.) * 17., (row as f64 - 3.) * 17.];
                    let point = rotation * Vector3::new(target[0], target[1], 0.) + translation;
                    let pixel = k * (point / point[2]);
                    let phase = (row * 9 + col + i * 63) as f64;
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
        let estimate = initialize_planar([1280, 720], &views, 0.2)?;
        cases.push(json!({ "name":name,"noise_amplitude_px":noise,"truth":{"fx":920.,"fy":870.,"cx":621.,"cy":349.,"skew":skew,"poses":poses},"observations":views,"estimate":estimate }));
    }
    println!("{}", serde_json::to_string_pretty(&json!({"schema":1,"provenance":"synthetic_fixture_not_scanner_calibration","cases":cases})).map_err(|e| e.to_string())?);
    Ok(())
}
