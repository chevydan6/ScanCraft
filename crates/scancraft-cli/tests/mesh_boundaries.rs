use serde_json::{Value, json};
use std::{fs, process::Command};
#[test]
fn fan_split_cli_preserves_indexed_face_positions_and_history_without_overwriting() {
    let root = std::env::temp_dir().join(format!("scancraft-cli-fan-split-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let path = root.join("input.json");
    let bytes = include_bytes!("../../../evidence/mesh-vertex-pinch-input.json");
    fs::write(&path, bytes).unwrap();
    let target = root.join("split");
    let run = |ids: &str, out: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .args([
                "split-mesh-vertex-fans",
                path.to_str().unwrap(),
                ids,
                out.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let result = run("0", &target);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["parameters"]["vertices_added"], 1);
    assert_eq!(report["parameters"]["geometric_gap_created"], false);
    assert_eq!(report["result_topology"]["nonmanifold_vertices"], 0);
    assert_eq!(report["watertightness_qualified"], false);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let saved = fs::read(target.join("project.meshproj")).unwrap();
    let work = scancraft_geometry::work::WorkControl::default();
    let mut history = scancraft_geometry::mesh_project::load(saved.as_slice(), &work).unwrap();
    assert_eq!(history.mesh().vertices.len(), 8);
    history.undo(&work).unwrap();
    let document: Value = serde_json::from_slice(bytes).unwrap();
    let original: scancraft_geometry::mesh::Mesh =
        serde_json::from_value(document["mesh"].clone()).unwrap();
    assert_eq!(history.mesh(), &original);
    assert!(!run("0", &target).status.success());
    assert_eq!(fs::read(target.join("project.meshproj")).unwrap(), saved);
    let rejected = root.join("invalid");
    assert!(!run("1", &rejected).status.success());
    assert!(!rejected.exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn topology_cli_reports_pinched_vertex_despite_closed_edges_and_keeps_source() {
    let root =
        std::env::temp_dir().join(format!("scancraft-cli-vertex-links-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let path = root.join("input.json");
    let bytes = include_bytes!("../../../evidence/mesh-vertex-pinch-input.json");
    fs::write(&path, bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
        .arg("mesh-topology")
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let t = &report["topology"];
    assert_eq!(t["edge_closed"], true);
    assert_eq!(t["closed_manifold_candidate"], false);
    assert_eq!(t["nonmanifold_vertex_ids"], json!([0]));
    assert_eq!(report["watertightness_qualified"], false);
    assert_eq!(fs::read(path).unwrap(), bytes);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn orientation_cli_retains_originals_and_exact_history_and_reports_explicit_sign() {
    let root =
        std::env::temp_dir().join(format!("scancraft-cli-orientation-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let path = root.join("input.json");
    let bytes = include_bytes!("../../../evidence/mesh-orientation-input.json");
    fs::write(&path, bytes).unwrap();
    let target = root.join("result");
    let run = |ids: &str, out: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .args([
                "orient-mesh",
                path.to_str().unwrap(),
                ids,
                out.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let result = run("none", &target);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["parameters"]["faces_flipped"], 1);
    assert_eq!(report["parameters"]["outward_direction_verified"], false);
    assert_eq!(report["result_topology"]["inconsistent_winding_edges"], 0);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let work = scancraft_geometry::work::WorkControl::default();
    let saved = fs::read(target.join("project.meshproj")).unwrap();
    let mut history = scancraft_geometry::mesh_project::load(saved.as_slice(), &work).unwrap();
    history.undo(&work).unwrap();
    let document: Value = serde_json::from_slice(bytes).unwrap();
    let original: scancraft_geometry::mesh::Mesh =
        serde_json::from_value(document["mesh"].clone()).unwrap();
    assert_eq!(history.mesh(), &original);
    assert!(!run("none", &target).status.success());
    assert_eq!(fs::read(target.join("project.meshproj")).unwrap(), saved);
    let reversed = root.join("reversed");
    assert!(run("0", &reversed).status.success());
    let r: Value =
        serde_json::from_slice(&fs::read(reversed.join("report.json")).unwrap()).unwrap();
    assert_eq!(r["parameters"]["faces_flipped"], 7);
    let rejected = root.join("invalid");
    assert!(!run("9", &rejected).status.success());
    assert!(!rejected.exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn explicit_fill_publishes_reversible_bundle_and_rejects_overlap_and_overwrite() {
    let root = std::env::temp_dir().join(format!("scancraft-cli-fill-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let input = root.join("ring.json");
    let bytes = include_bytes!("../../../evidence/mesh-boundary-ring-input.json");
    fs::write(&input, bytes).unwrap();
    let target = root.join("filled");
    let run = |id: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .args([
                "fill-mesh-boundary",
                input.to_str().unwrap(),
                id,
                "0.001",
                target.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let output = run("1", &target);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["parameters"]["added_faces"], 2);
    assert_eq!(report["result_topology"]["boundary_edges"], 4);
    assert_eq!(fs::read(&input).unwrap(), bytes);
    let saved = fs::read(target.join("project.meshproj")).unwrap();
    let work = scancraft_geometry::work::WorkControl::default();
    let mut history = scancraft_geometry::mesh_project::load(saved.as_slice(), &work).unwrap();
    assert_eq!(history.mesh().triangles.len(), 10);
    history.undo(&work).unwrap();
    let document: Value = serde_json::from_slice(bytes).unwrap();
    let original: scancraft_geometry::mesh::Mesh =
        serde_json::from_value(document["mesh"].clone()).unwrap();
    assert_eq!(history.mesh(), &original);
    assert!(!run("1", &target).status.success());
    assert_eq!(fs::read(target.join("project.meshproj")).unwrap(), saved);
    let refused = root.join("overlap");
    assert!(!run("0", &refused).status.success());
    assert!(!refused.exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn cli_reports_both_ring_boundaries_preserves_input_and_rejects_bad_units() {
    let root =
        std::env::temp_dir().join(format!("scancraft-cli-boundaries-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let bytes = include_bytes!("../../../evidence/mesh-boundary-ring-input.json");
    let path = root.join("mesh.json");
    fs::write(&path, bytes).unwrap();
    let run = |path: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("mesh-boundaries")
            .arg(path)
            .output()
            .unwrap()
    };
    let output = run(&path);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["analysis"]["boundary_edges"], 8);
    assert_eq!(report["analysis"]["loops"].as_array().unwrap().len(), 2);
    assert_eq!(report["analysis"]["loops"][0]["perimeter_mm"], 40.);
    assert_eq!(report["analysis"]["loops"][1]["perimeter_mm"], 16.);
    assert_eq!(report["physical_accuracy_qualified"], false);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let work = scancraft_geometry::work::WorkControl::default();
    let document: Value = serde_json::from_slice(bytes).unwrap();
    let mesh = serde_json::from_value(document["mesh"].clone()).unwrap();
    let editor = scancraft_geometry::mesh_edit::MeshEditor::new(mesh, 32, &work).unwrap();
    let mut project = Vec::new();
    scancraft_geometry::mesh_project::save(&editor, &mut project, &work).unwrap();
    let project_path = root.join("history.meshproj");
    fs::write(&project_path, &project).unwrap();
    let reopened = run(&project_path);
    assert!(reopened.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&reopened.stdout).unwrap(),
        report
    );
    assert_eq!(fs::read(&project_path).unwrap(), project);
    let mut bad: Value = serde_json::from_slice(bytes).unwrap();
    bad["units"] = json!("unknown");
    fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(!run(&path).status.success());
}
