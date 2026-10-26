//! Synthetic two-depth correspondence fixture; not Raptor data or calibration.
use scancraft_calibration::{
    Brown5, CameraModel,
    stereo::{RigidPose, StereoCalibration, TriangulationLimits},
    storage::{CalibrationRecord, Provenance},
};
use scancraft_depth::{
    MatchSource,
    matching::{MatchConfig, MatchedFeature, match_rectified},
    reconstruct,
};
use scancraft_geometry::work::WorkControl;
use scancraft_image::MaskedImage;
use serde_json::json;
fn noise(mut state: u32) -> Vec<f32> {
    (0..64 * 32)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            10. + (state % 91) as f32
        })
        .collect()
}
fn main() {
    let left = noise(0xabc12345);
    let mut right = noise(0x12345abc);
    for y in 0..32 {
        let d = if y < 16 { 4 } else { 8 };
        for x in 0..64 - d {
            right[y * 64 + x] = 2. * left[y * 64 + x + d] + 20.;
        }
    }
    let left = MaskedImage::new(64, 32, left, vec![true; 2048]).unwrap();
    let right = MaskedImage::new(64, 32, right, vec![true; 2048]).unwrap();
    let camera = CameraModel {
        width: 64,
        height: 32,
        fx: 80.,
        fy: 80.,
        cx: 31.5,
        cy: 15.5,
        skew: 0.,
        distortion: Brown5::ZERO,
        valid_radius: 1.,
    };
    let rig = StereoCalibration {
        left: camera.clone(),
        right: camera,
        right_from_left: RigidPose {
            rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            translation_mm: [-20., 0., 0.],
        },
    };
    let seeds: Vec<_> = [6, 11, 21, 26]
        .into_iter()
        .flat_map(|y| (16..55).step_by(6).map(move |x| [x, y]))
        .collect();
    let config = MatchConfig {
        patch_radius: 2,
        min_disparity: 1,
        max_disparity: 12,
        min_patch_stddev: 5.,
        min_correlation: 0.9,
        min_uniqueness_margin: 0.05,
        subpixel: None,
    };
    let matched =
        match_rectified(&rig, &left, &right, &seeds, config, &WorkControl::default()).unwrap();
    let record = CalibrationRecord {
        schema_version: 1,
        record_id: "synthetic-two-depth-images".into(),
        units: "millimeters".into(),
        camera_axes: "x_right_y_down_z_forward".into(),
        provenance: Provenance::SyntheticFixture,
        binding: None,
        geometry: rig,
        notes: "synthetic fixture only".into(),
    };
    let limits = TriangulationLimits {
        min_depth_z_mm: 10.,
        max_depth_z_mm: 1000.,
        min_ray_angle_rad: 0.0001,
        max_ray_gap_mm: 0.01,
        max_reprojection_error_px: 0.01,
    };
    let points: Vec<_> = matched
        .accepted
        .iter()
        .map(MatchedFeature::pixel_match)
        .collect();
    let reconstructed = reconstruct(
        &record,
        MatchSource::SyntheticFixture,
        &points,
        limits,
        &WorkControl::default(),
    )
    .unwrap();
    println!("{}",serde_json::to_string(&json!({"provenance":"synthetic_fixture_not_scanner_calibration", "rig":record.geometry,
        "left_image":left.intensity(),"right_image":right.intensity(),"seeds":seeds,"matching":matched,"reconstruction":reconstructed})).unwrap());
}
