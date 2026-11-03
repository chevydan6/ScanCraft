//! Analytic band-limited synthetic scene, not sensor data or physical calibration.
use scancraft_calibration::{
    Brown5, CameraModel,
    stereo::{RigidPose, StereoCalibration},
};
use scancraft_depth::matching::{MatchConfig, SubpixelConfig};
pub fn fixture(disparity: f64) -> (StereoCalibration, Vec<f32>, Vec<f32>) {
    let camera = CameraModel {
        width: 96,
        height: 48,
        fx: 120.,
        fy: 120.,
        cx: 47.5,
        cy: 23.5,
        skew: 0.,
        distortion: Brown5::ZERO,
        valid_radius: 1.,
    };
    let rig = StereoCalibration {
        left: camera.clone(),
        right: camera,
        right_from_left: RigidPose {
            rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            translation_mm: [-20. * disparity.signum(), 0., 0.],
        },
    };
    let signal = |x: f64, y: f64| {
        125. + 30. * (0.31 * x + 0.17 * y).sin()
            + 25. * (0.49 * x - 0.21 * y).sin()
            + 12. * (0.13 * x + 0.57 * y).sin()
            + 8. * (0.53 * x + 0.23 * y).sin()
    };
    let mut left = Vec::new();
    let mut right = Vec::new();
    for y in 0..48 {
        for x in 0..96 {
            left.push(signal(x as f64, y as f64) as f32);
            right.push((1.1 * signal(x as f64 + disparity, y as f64) + 7.) as f32);
        }
    }
    (rig, left, right)
}
pub fn config(disparity: f64) -> MatchConfig {
    MatchConfig {
        patch_radius: 7,
        min_disparity: if disparity > 0. { 1 } else { -8 },
        max_disparity: if disparity > 0. { 8 } else { -1 },
        min_patch_stddev: 2.,
        min_correlation: 0.85,
        min_uniqueness_margin: 0.001,
        subpixel: Some(SubpixelConfig {
            min_curvature: 0.001,
            max_bidirectional_error_px: 0.15,
        }),
    }
}
