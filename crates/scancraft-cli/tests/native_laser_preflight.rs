//! Device-free CLI boundary checks; no successful hardware request is issued.
mod common;
use std::{
    fs::{self, File},
    process::Command,
};
#[test]
fn native_laser_cli_rejects_existing_targets_and_invalid_fps_before_hardware() {
    let root =
        std::env::temp_dir().join(format!("scancraft-native-laser-cli-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    common::dataset(&root, false);
    let calibration =
        scancraft_calibration::storage::read(File::open(root.join("calibration.rapcal")).unwrap())
            .unwrap();
    let geometry = root.join("geometry.json");
    fs::write(
        &geometry,
        serde_json::to_vec(&calibration.geometry).unwrap(),
    )
    .unwrap();
    let request = root.join("laser-request.json");
    let r = serde_json::json!({"schema_version":1,"fps":31,"maximum_frames_per_camera":1,"maximum_recording_bytes_per_camera":2_000_000,"maximum_pair_log_bytes":100_000,"processing":{"interfaces":[0,2],"reflect_x":[false,false],"maximum_host_delta_us":1000,"detection":{"background":10,"minimum_peak":30,"maximum_width":3,"saturation":255,"maximum_observations":100},"continuity":{"maximum_row_gap":1,"maximum_horizontal_step":1.,"maximum_slope":0.,"minimum_span_rows":2,"minimum_observations":2},"sheets":[{"quadratic_y":0.,"linear_y":0.,"linear_z":0.,"offset":1.}],"triangulation":{"min_depth_z_mm":1.,"max_depth_z_mm":1000.,"min_ray_angle_rad":0.0001,"max_ray_gap_mm":1.,"max_reprojection_error_px":1.},"matching":{"intersection":{"maximum_segment_length_px":3.,"normalized_tolerance":1e-10,"minimum_plane_sine":1e-8},"maximum_sheet_distance_px":1.,"maximum_work":1000},"maximum_queries":100,"maximum_total_work":100_000}});
    fs::write(&request, serde_json::to_vec(&r).unwrap()).unwrap();
    let target = root.join("out");
    let output = Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
        .arg("scan-live-laser")
        .args([&geometry, &request, &target])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("enumerated IR frame rate"));
    assert!(!target.exists());
    let mut invalid = r.clone();
    invalid["fps"] = serde_json::json!(30);
    invalid["source_options"] = serde_json::json!({"illumination":"line_boolean_trial","gain":99});
    fs::write(&request, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
        .arg("scan-live-laser")
        .args([&geometry, &request, &target])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("source_options"));
    assert!(!target.exists());
    fs::create_dir(&target).unwrap();
    fs::write(target.join("owner"), b"preserve").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
        .arg("scan-live-laser")
        .args([&geometry, &request, &target])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
    assert_eq!(fs::read(target.join("owner")).unwrap(), b"preserve");
    fs::remove_dir_all(&root).unwrap();
}
