mod common;
use std::{
    fs::{self, File},
    io::{BufReader, Cursor},
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "scancraft-offline-stereo-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("reconstruct-stereo")
            .args([
                self.0.join("calibration.rapcal"),
                self.0.join("left.png"),
                self.0.join("right.png"),
                self.0.join("request.json"),
                self.0.join("result"),
            ])
            .output()
            .unwrap()
    }
}
#[test]
fn explicit_surface_normals_export_supported_points_with_exact_cloud_lineage() {
    let s = Scratch::new();
    common::dataset(&s.0, false);
    let path = s.0.join("request.json");
    let mut request: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    request.as_object_mut().unwrap().remove("seeds");
    request["automatic_sampling"] = serde_json::json!({"spacing":4,"maximum_seeds":100});
    request["meshing"] = serde_json::json!({"spacing":4,"maximum_edge_mm":35.});
    request["surface_normals"] =
        serde_json::json!({"maximum_deviation_rad":0.3,"viewpoint_mm":[0.,0.,0.]});
    fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let output = s.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let full = scancraft_geometry::formats::read_ply(
        Cursor::new(fs::read(s.0.join("result/points.ply")).unwrap()),
        None,
    )
    .unwrap();
    let supported = scancraft_geometry::formats::read_ply(
        Cursor::new(fs::read(s.0.join("result/surface-normal-points.ply")).unwrap()),
        None,
    )
    .unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(s.0.join("result/mesh-report.json")).unwrap()).unwrap();
    let normal_report = &report["surface_normals"];
    assert!(!supported.points.is_empty());
    assert_eq!(normal_report["estimated"], supported.points.len());
    assert_eq!(normal_report["physical_accuracy_qualified"], false);
    assert!(full.points.iter().all(|p| p.normal.is_none()));
    for (point, index) in supported
        .points
        .iter()
        .zip(normal_report["cloud_indices"].as_array().unwrap())
    {
        let original = &full.points[index.as_u64().unwrap() as usize];
        assert_eq!(point.position, original.position);
        assert_eq!(point.color, original.color);
        let normal = point.normal.unwrap();
        assert!((normal[2] + 1.).abs() < 1e-8);
        assert!(normal[0].abs() < 1e-8 && normal[1].abs() < 1e-8);
    }
}

#[test]
fn surface_normals_without_declared_meshing_fail_before_publication() {
    let s = Scratch::new();
    common::dataset(&s.0, false);
    let path = s.0.join("request.json");
    let mut request: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    request["surface_normals"] = serde_json::json!({"maximum_deviation_rad":0.3});
    fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let output = s.run();
    assert!(!output.status.success());
    assert!(!s.0.join("result").exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires explicit meshing"));
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn cli_reconstructs_rotated_images_and_publishes_verified_editable_artifacts() {
    let s = Scratch::new();
    common::dataset(&s.0, false);
    let out = s.run();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let summary: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(summary["points"], 3);
    assert_eq!(summary["status"], "geometry_produced");
    assert_eq!(summary["synchronization_verified"], false);
    assert_eq!(summary["physical_accuracy_qualified"], false);
    let dir = s.0.join("result");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest, summary);
    for artifact in manifest["artifacts"].as_array().unwrap() {
        let bytes = fs::read(dir.join(artifact["role"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, artifact["bytes"].as_u64().unwrap());
        assert_eq!(
            u64::from(crc32fast::hash(&bytes)),
            artifact["crc32"].as_u64().unwrap()
        );
    }
    let cloud = scancraft_geometry::formats::read_ply(
        BufReader::new(File::open(dir.join("points.ply")).unwrap()),
        None,
    )
    .unwrap();
    assert_eq!(cloud.points.len(), 3);
    let expected = [[-57.5, -27.5, 400.], [22.5, 22.5, 400.], [92.5, 42.5, 400.]];
    for (p, expected) in cloud.points.iter().zip(expected) {
        for (actual, expected) in p.position.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-7);
        }
    }
    let editor =
        scancraft_geometry::formats::load_project(File::open(dir.join("project.orproj")).unwrap())
            .unwrap();
    assert_eq!(editor.cloud, cloud);
    let mut bytes = Vec::new();
    scancraft_geometry::formats::save_project(&editor, &mut bytes).unwrap();
    assert!(!bytes.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["calibration"]["provenance"], "synthetic_fixture");
    assert_eq!(report["matching"]["rejected"][0]["seed_index"], 0);
    let before = fs::read(dir.join("manifest.json")).unwrap();
    assert!(!s.run().status.success());
    assert_eq!(fs::read(dir.join("manifest.json")).unwrap(), before);
}
#[test]
fn blank_pair_retains_diagnostics_and_does_not_publish_fake_geometry() {
    let s = Scratch::new();
    common::dataset(&s.0, true);
    let out = s.run();
    assert!(out.status.success());
    let summary: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(summary["status"], "no_geometry");
    assert_eq!(summary["points"], 0);
    assert!(!s.0.join("result/points.ply").exists());
    assert!(!s.0.join("result/project.orproj").exists());
    assert!(s.0.join("result/report.json").exists());
}
#[test]
fn malformed_inputs_and_existing_empty_targets_leave_no_published_bundle() {
    let s = Scratch::new();
    common::dataset(&s.0, false);
    fs::create_dir(s.0.join("result")).unwrap();
    assert!(!s.run().status.success());
    assert!(s.0.join("result").read_dir().unwrap().next().is_none());
    fs::remove_dir(s.0.join("result")).unwrap();
    fs::write(s.0.join("request.json"), b"{}").unwrap();
    assert!(!s.run().status.success());
    assert!(!s.0.join("result").exists());
}

#[test]
fn selected_recording_frame_preserves_payload_and_rejects_corruption_after_selection() {
    let s = Scratch::new();
    let profile = scancraft_core::StreamProfile {
        interface: 0,
        width: 2,
        height: 2,
        fourcc: *b"GREY",
        fps: 50,
    };
    let mut writer = scancraft_recording::Writer::new(Vec::new(), profile).unwrap();
    for i in 0..3u32 {
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&(100 + i * 20).to_le_bytes());
        writer
            .push(&scancraft_core::Frame {
                sequence: i,
                host_timestamp_us: 1000 + u64::from(i) * 20,
                flags: 0,
                metadata,
                data: vec![i as u8; 4],
            })
            .unwrap();
    }
    let bytes = writer.finish().unwrap();
    let input = s.0.join("source.raptor");
    fs::write(&input, &bytes).unwrap();
    let invoke = |target: &str| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("extract-frame")
            .arg(&input)
            .arg("1")
            .arg(s.0.join(target))
            .output()
            .unwrap()
    };
    let out = invoke("frame");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(manifest["source_total_frames"], 3);
    assert_eq!(manifest["selected_frame_index"], 1);
    assert_eq!(manifest["sensor_pts"], 120);
    let image = image::open(s.0.join("frame/image.png"))
        .unwrap()
        .into_luma8();
    assert_eq!(image.as_raw(), &[1; 4]);
    let selected = scancraft_recording::selection::read_selected(
        scancraft_recording::Reader::new(File::open(s.0.join("frame/frame.raptor")).unwrap())
            .unwrap(),
        0,
    )
    .unwrap();
    assert_eq!(selected.frame.sequence, 1);
    assert_eq!(selected.frame.data, [1; 4]);
    assert_eq!(selected.total_frames, 1);
    let mut corrupt = bytes;
    let n = corrupt.len();
    corrupt[n - 13] ^= 1;
    fs::write(&input, &corrupt).unwrap();
    assert!(!invoke("invalid").status.success());
    assert!(!s.0.join("invalid").exists());
}

#[test]
fn automatic_sampling_reconstructs_without_manual_pixels_and_records_seed_lineage() {
    for blank in [false, true] {
        let s = Scratch::new();
        common::dataset(&s.0, blank);
        let path = s.0.join("request.json");
        let mut request: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        request.as_object_mut().unwrap().remove("seeds");
        request["automatic_sampling"] = serde_json::json!({"spacing":8,"maximum_seeds":100});
        fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let output = s.run();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(s.0.join("result/report.json")).unwrap()).unwrap();
        let seeds = report["seeds"].as_array().unwrap();
        assert_eq!(report["automatic_sampling"]["spacing"], 8);
        if blank {
            assert!(seeds.is_empty());
            assert!(report["reconstruction"].is_null());
            assert!(!s.0.join("result/points.ply").exists());
        } else {
            assert!(!seeds.is_empty() && seeds.len() <= 100);
            let cloud = scancraft_geometry::formats::read_ply(
                BufReader::new(File::open(s.0.join("result/points.ply")).unwrap()),
                None,
            )
            .unwrap();
            assert!(!cloud.points.is_empty());
            assert!(
                cloud
                    .points
                    .iter()
                    .all(|p| (p.position[2] - 400.).abs() < 1e-7)
            );
            for m in report["matching"]["accepted"].as_array().unwrap() {
                assert_eq!(
                    m["left_pixel"],
                    seeds[m["seed_index"].as_u64().unwrap() as usize]
                );
            }
        }
    }
}

#[test]
fn automatic_sampling_rejects_conflicting_seeds_and_invalid_resource_limits() {
    for sampling in [
        serde_json::json!({"spacing":0,"maximum_seeds":100}),
        serde_json::json!({"spacing":8,"maximum_seeds":100001}),
        serde_json::json!({"spacing":8,"maximum_seeds":100}),
    ] {
        let s = Scratch::new();
        common::dataset(&s.0, false);
        let path = s.0.join("request.json");
        let mut request: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if sampling["spacing"] == 0 || sampling["maximum_seeds"] == 100001 {
            request.as_object_mut().unwrap().remove("seeds");
        }
        request["automatic_sampling"] = sampling;
        fs::write(path, serde_json::to_vec(&request).unwrap()).unwrap();
        assert!(!s.run().status.success());
        assert!(!s.0.join("result").exists());
    }
}

#[test]
fn stereo_meshing_preserves_original_frame_coordinates_seed_lineage_and_missing_samples() {
    for blank in [false, true] {
        let s = Scratch::new();
        common::dataset(&s.0, blank);
        let path = s.0.join("request.json");
        let mut request: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        request.as_object_mut().unwrap().remove("seeds");
        request["automatic_sampling"] = serde_json::json!({"spacing":4,"maximum_seeds":100});
        request["meshing"] = serde_json::json!({"spacing":4,"maximum_edge_mm":35.});
        fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let out = s.run();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let summary: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let mesh_report: serde_json::Value =
            serde_json::from_slice(&fs::read(s.0.join("result/mesh-report.json")).unwrap())
                .unwrap();
        assert_eq!(mesh_report["physical_accuracy_qualified"], false);
        assert_eq!(
            mesh_report["lattice_origin_rectified_pixel"],
            serde_json::json!([2, 2])
        );
        assert_eq!(mesh_report["grid_size"], serde_json::json!([15, 7]));
        if blank {
            assert_eq!(summary["mesh_triangles"], 0);
            assert!(!s.0.join("result/mesh.stl").exists());
            continue;
        }
        assert!(summary["mesh_triangles"].as_u64().unwrap() > 0);
        let mesh_json: serde_json::Value =
            serde_json::from_slice(&fs::read(s.0.join("result/mesh.json")).unwrap()).unwrap();
        let mesh: scancraft_geometry::mesh::Mesh =
            serde_json::from_value(mesh_json["mesh"].clone()).unwrap();
        assert!(mesh.vertices.iter().all(|p| (p[2] - 400.).abs() < 1e-7));
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(s.0.join("result/report.json")).unwrap()).unwrap();
        for (vertex, seed) in mesh
            .vertices
            .iter()
            .zip(mesh_report["vertex_seed_indices"].as_array().unwrap())
        {
            let mapped = report["mapped_matches"]
                .as_array()
                .unwrap()
                .iter()
                .position(|m| m["seed_index"] == *seed)
                .unwrap();
            let point = report["reconstruction"]["accepted"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["source_index"] == mapped)
                .unwrap();
            let expected: [f64; 3] =
                serde_json::from_value(point["geometry"]["position_mm"].clone()).unwrap();
            assert_eq!(*vertex, expected);
        }
        for artifact in summary["artifacts"].as_array().unwrap() {
            let bytes =
                fs::read(s.0.join("result").join(artifact["role"].as_str().unwrap())).unwrap();
            assert_eq!(artifact["crc32"], u64::from(crc32fast::hash(&bytes)));
        }
    }
}
#[test]
fn stereo_meshing_rejects_conflicting_lattice_but_accepts_explicit_sparse_grid() {
    for valid in [false, true] {
        let s = Scratch::new();
        common::dataset(&s.0, false);
        let path = s.0.join("request.json");
        let mut request: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        request["meshing"] = serde_json::json!({"spacing":4,"maximum_edge_mm":35.});
        if valid {
            request["seeds"] = serde_json::json!([[10, 10], [14, 10], [10, 14], [14, 14]]);
        }
        fs::write(path, serde_json::to_vec(&request).unwrap()).unwrap();
        let out = s.run();
        if valid {
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let summary: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(summary["mesh_triangles"], 2);
        } else {
            assert!(!out.status.success());
            assert!(!s.0.join("result").exists());
        }
    }
}
