//! Redistributable synthetic image fixtures, never scanner calibration.
use scancraft_calibration::{
    Brown5, CameraModel,
    stereo::{RigidPose, StereoCalibration},
    storage::{self, CalibrationRecord, Provenance},
};
use std::{
    fs::{self, File},
    path::Path,
};
pub fn dataset(dir: &Path, flat: bool) {
    let grid = CameraModel {
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
    let calibration = CalibrationRecord {
        schema_version: 1,
        record_id: "synthetic-cli-reconstruction".into(),
        units: "millimeters".into(),
        camera_axes: "x_right_y_down_z_forward".into(),
        provenance: Provenance::SyntheticFixture,
        binding: None,
        geometry: StereoCalibration {
            left: grid.clone(),
            right: grid.clone(),
            right_from_left: RigidPose {
                rotation: [[-1., 0., 0.], [0., -1., 0.], [0., 0., 1.]],
                translation_mm: [20., 0., 0.],
            },
        },
        notes: "Synthetic rotated-camera test. Not Raptor calibration.".into(),
    };
    storage::write(
        &calibration,
        File::create(dir.join("calibration.rapcal")).unwrap(),
    )
    .unwrap();
    let mut state = 0x12345678u32;
    let left: Vec<u8> = (0..2048)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            if flat { 20 } else { 10 + (state % 91) as u8 }
        })
        .collect();
    let mut right = vec![0; 2048];
    for y in 0..32 {
        for x in 0..60 {
            right[y * 64 + x] = 2 * left[y * 64 + x + 4] + 20;
        }
    }
    right.reverse();
    image::save_buffer(dir.join("left.png"), &left, 64, 32, image::ColorType::L8).unwrap();
    image::save_buffer(dir.join("right.png"), &right, 64, 32, image::ColorType::L8).unwrap();
    let request = serde_json::json!({"schema_version":1,"source":"synthetic_fixture","output_grid":grid,
        "seeds":[[0,0],[20,10],[36,20],[50,24]],
        "matching":{"patch_radius":2,"min_disparity":1,"max_disparity":8,"min_patch_stddev":5.,"min_correlation":0.9,"min_uniqueness_margin":0.05},
        "triangulation":{"min_depth_z_mm":10.,"max_depth_z_mm":1000.,"min_ray_angle_rad":0.0001,"max_ray_gap_mm":0.01,"max_reprojection_error_px":0.01}});
    fs::write(
        dir.join("request.json"),
        serde_json::to_vec_pretty(&request).unwrap(),
    )
    .unwrap();
}
