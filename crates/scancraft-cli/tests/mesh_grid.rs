use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "scancraft-grid-mesh-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn request(&self, hole: bool) -> Value {
        let samples: Vec<_> = (0..3)
            .flat_map(|y| {
                (0..3).map(move |x| {
                    if hole && x == 1 && y == 1 {
                        Value::Null
                    } else {
                        json!([x as f64, y as f64, 10. + x as f64 * 0.1 + y as f64 * 0.2])
                    }
                })
            })
            .collect();
        let request = json!({"schema_version":1,"units":"millimeters","source":"synthetic_fixture","width":3,"height":3,"samples":samples,"maximum_edge_mm":2.});
        self.write(&request);
        request
    }
    fn write(&self, request: &Value) {
        fs::write(
            self.0.join("grid.json"),
            serde_json::to_vec_pretty(request).unwrap(),
        )
        .unwrap();
    }
    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("mesh-grid")
            .arg(self.0.join("grid.json"))
            .arg(self.0.join("result"))
            .output()
            .unwrap()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn cli_exports_analytic_plane_surface_and_integrity_checked_mesh_artifacts() {
    let s = Scratch::new();
    s.request(false);
    let out = s.run();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let summary: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(summary["triangles"], 8);
    assert_eq!(summary["vertices"], 9);
    assert_eq!(summary["status"], "surface_produced");
    for a in summary["artifacts"].as_array().unwrap() {
        let b = fs::read(s.0.join("result").join(a["role"].as_str().unwrap())).unwrap();
        assert_eq!(a["bytes"], b.len() as u64);
        assert_eq!(a["crc32"], u64::from(crc32fast::hash(&b)));
    }
    let report: Value =
        serde_json::from_slice(&fs::read(s.0.join("result/report.json")).unwrap()).unwrap();
    assert_eq!(report["topology"]["boundary_edges"], 8);
    assert_eq!(report["physical_accuracy_qualified"], false);
    assert_eq!(report["watertightness_qualified"], false);
    let json: Value =
        serde_json::from_slice(&fs::read(s.0.join("result/mesh.json")).unwrap()).unwrap();
    let mesh: scancraft_geometry::mesh::Mesh =
        serde_json::from_value(json["mesh"].clone()).unwrap();
    mesh.validate(&scancraft_geometry::work::WorkControl::default())
        .unwrap();
    let mut area = 0.;
    for t in &mesh.triangles {
        let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
        let u: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
        let v: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        area += n[0].hypot(n[1]).hypot(n[2]) / 2.;
    }
    assert!((area - 4. * 1.05f64.sqrt()).abs() < 1e-12);
    let stl = fs::read_to_string(s.0.join("result/mesh.stl")).unwrap();
    assert_eq!(
        stl.lines()
            .filter(|l| l.starts_with("facet normal"))
            .count(),
        8
    );
    let before = fs::read(s.0.join("result/manifest.json")).unwrap();
    assert!(!s.run().status.success());
    assert_eq!(fs::read(s.0.join("result/manifest.json")).unwrap(), before);
}
#[test]
fn incomplete_grid_retains_diagnostics_and_invalid_input_never_publishes() {
    let s = Scratch::new();
    s.request(true);
    let out = s.run();
    assert!(out.status.success());
    let summary: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(summary["status"], "no_surface");
    assert!(!s.0.join("result/mesh.stl").exists());
    for kind in 0..3 {
        let s = Scratch::new();
        let mut request = s.request(false);
        match kind {
            0 => request["units"] = json!("unspecified"),
            1 => request["samples"] = json!([]),
            _ => request["maximum_edge_mm"] = json!(-1.),
        };
        s.write(&request);
        assert!(!s.run().status.success());
        assert!(!s.0.join("result").exists());
    }
    let s = Scratch::new();
    s.request(false);
    let work = scancraft_geometry::work::WorkControl::default();
    work.cancel();
    assert!(
        scancraft_workflow::meshing::run(&s.0.join("grid.json"), &s.0.join("result"), &work)
            .is_err()
    );
    assert!(!s.0.join("result").exists());
}

#[test]
fn cli_plane_trim_saves_original_exactly_and_preserves_input_on_failures() {
    let s = Scratch::new();
    s.request(false);
    assert!(s.run().status.success());
    let input = s.0.join("result/mesh.json");
    let original = fs::read(&input).unwrap();
    let target = s.0.join("trimmed");
    let run = |normal: &str, output: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("trim-mesh")
            .arg(&input)
            .args([normal, "0", "0", "-0.5"])
            .arg(output)
            .output()
            .unwrap()
    };
    let output = run("1", &target);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["triangles"], 6);
    assert_eq!(summary["cut_edges"], 5);
    assert_eq!(
        fs::read(target.join("original.mesh.json")).unwrap(),
        original
    );
    assert_eq!(fs::read(&input).unwrap(), original);
    let trimmed = scancraft_workflow::mesh_document::load(
        &target.join("mesh.json"),
        &scancraft_geometry::work::WorkControl::default(),
    )
    .unwrap();
    assert!(trimmed.mesh.vertices.iter().all(|p| p[0] <= 0.5));
    assert_eq!(trimmed.topology.boundary_edges, 8);
    for a in summary["artifacts"].as_array().unwrap() {
        let bytes = fs::read(target.join(a["role"].as_str().unwrap())).unwrap();
        assert_eq!(a["crc32"], u64::from(crc32fast::hash(&bytes)));
    }
    let before = fs::read(target.join("manifest.json")).unwrap();
    assert!(!run("1", &target).status.success());
    assert_eq!(fs::read(target.join("manifest.json")).unwrap(), before);
    let failed = s.0.join("failed");
    assert!(!run("0", &failed).status.success());
    assert!(!failed.exists());
    let work = scancraft_geometry::work::WorkControl::default();
    work.cancel();
    assert!(
        scancraft_workflow::mesh_trim::run(&input, [1., 0., 0.], -0.5, &failed, &work).is_err()
    );
    assert!(!failed.exists());
}

#[test]
fn persisted_mesh_projects_chain_trims_and_restore_undo_redo_after_reopening() {
    let s = Scratch::new();
    s.request(false);
    assert!(s.run().status.success());
    let work = scancraft_geometry::work::WorkControl::default();
    let input = s.0.join("result/mesh.json");
    let original = scancraft_workflow::mesh_document::load(&input, &work)
        .unwrap()
        .mesh;
    let trim = |input: &std::path::Path, offset: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("trim-mesh")
            .arg(input)
            .args(["1", "0", "0", offset])
            .arg(target)
            .output()
            .unwrap()
    };
    let first = s.0.join("first");
    let out = trim(&input, "-0.5", &first);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let first_bytes = fs::read(first.join("project.meshproj")).unwrap();
    let first_editor =
        scancraft_geometry::mesh_project::load(first_bytes.as_slice(), &work).unwrap();
    assert_eq!(first_editor.history_depth(), (1, 0));
    let second = s.0.join("second");
    let out = trim(&first.join("project.meshproj"), "-0.25", &second);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read(second.join("original.meshproj")).unwrap(),
        first_bytes
    );
    let second_bytes = fs::read(second.join("project.meshproj")).unwrap();
    let second_editor =
        scancraft_geometry::mesh_project::load(second_bytes.as_slice(), &work).unwrap();
    assert_eq!(second_editor.history_depth(), (2, 0));
    let history = |input: &std::path::Path, action: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("mesh-history")
            .arg(input)
            .arg(action)
            .arg(target)
            .output()
            .unwrap()
    };
    let undo1 = s.0.join("undo1");
    assert!(
        history(&second.join("project.meshproj"), "undo", &undo1)
            .status
            .success()
    );
    let bytes = fs::read(undo1.join("project.meshproj")).unwrap();
    let restored = scancraft_geometry::mesh_project::load(bytes.as_slice(), &work).unwrap();
    assert_eq!(restored.mesh(), first_editor.mesh());
    assert_eq!(restored.history_depth(), (1, 1));
    let undo2 = s.0.join("undo2");
    assert!(
        history(&undo1.join("project.meshproj"), "undo", &undo2)
            .status
            .success()
    );
    let bytes = fs::read(undo2.join("project.meshproj")).unwrap();
    assert_eq!(
        scancraft_geometry::mesh_project::load(bytes.as_slice(), &work)
            .unwrap()
            .mesh(),
        &original
    );
    assert!(
        !history(
            &undo2.join("project.meshproj"),
            "undo",
            &s.0.join("unavailable")
        )
        .status
        .success()
    );
    assert!(!s.0.join("unavailable").exists());
    let redo1 = s.0.join("redo1");
    assert!(
        history(&undo2.join("project.meshproj"), "redo", &redo1)
            .status
            .success()
    );
    let redo2 = s.0.join("redo2");
    assert!(
        history(&redo1.join("project.meshproj"), "redo", &redo2)
            .status
            .success()
    );
    let bytes = fs::read(redo2.join("project.meshproj")).unwrap();
    let restored = scancraft_geometry::mesh_project::load(bytes.as_slice(), &work).unwrap();
    assert_eq!(restored.mesh(), second_editor.mesh());
    assert_eq!(restored.history_depth(), (2, 0));
    let mut corrupt = bytes;
    corrupt.push(0);
    fs::write(s.0.join("corrupt.meshproj"), corrupt).unwrap();
    assert!(
        !history(&s.0.join("corrupt.meshproj"), "undo", &s.0.join("bad"))
            .status
            .success()
    );
    assert!(!s.0.join("bad").exists());
}

#[test]
fn cli_cleanup_options_and_project_chaining_preserve_exact_original_and_undo() {
    let s = Scratch::new();
    let input = s.0.join("source.json");
    let bytes=br#"{"schema_version":1,"units":"millimeters","mesh":{"vertices":[[0,0,0],[1,0,0],[0,1,0],[9,9,9]],"triangles":[[0,2,1],[2,1,0],[1,2,0]]}}"#;
    fs::write(&input, bytes).unwrap();
    let run = |input: &std::path::Path, mode: &str, output: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("cleanup-mesh")
            .arg(input)
            .arg(mode)
            .arg(output)
            .output()
            .unwrap()
    };
    let first = s.0.join("first");
    let out = run(&input, "duplicates", &first);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let summary: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(summary["removed_faces"], 2);
    assert_eq!(summary["removed_vertices"], 0);
    assert_eq!(summary["vertices"], 4);
    assert_eq!(fs::read(first.join("original.mesh.json")).unwrap(), bytes);
    let second = s.0.join("second");
    assert!(
        run(&first.join("project.meshproj"), "unused", &second)
            .status
            .success()
    );
    let work = scancraft_geometry::work::WorkControl::default();
    let data = fs::read(second.join("project.meshproj")).unwrap();
    let mut editor = scancraft_geometry::mesh_project::load(data.as_slice(), &work).unwrap();
    assert_eq!(editor.mesh().vertices.len(), 3);
    assert_eq!(editor.history_depth(), (2, 0));
    editor.undo(&work).unwrap();
    assert_eq!(editor.mesh().vertices.len(), 4);
    editor.undo(&work).unwrap();
    assert_eq!(editor.mesh().triangles.len(), 3);
    let before = fs::read(second.join("manifest.json")).unwrap();
    assert!(!run(&input, "both", &second).status.success());
    assert_eq!(fs::read(second.join("manifest.json")).unwrap(), before);
    assert!(!run(&input, "unknown", &s.0.join("bad")).status.success());
    assert!(!s.0.join("bad").exists());
}

#[test]
fn cli_component_analysis_keep_remove_and_persisted_undo_are_exact() {
    let s = Scratch::new();
    let input = s.0.join("source.json");
    let bytes=br#"{"schema_version":1,"units":"millimeters","mesh":{"vertices":[[0,0,0],[1,0,0],[0,1,0],[10,0,0],[11,0,0],[10,1,0]],"triangles":[[0,1,2],[3,4,5]]}}"#;
    fs::write(&input, bytes).unwrap();
    let analysis = Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
        .arg("mesh-components")
        .arg(&input)
        .output()
        .unwrap();
    assert!(analysis.status.success());
    let report: Value = serde_json::from_slice(&analysis.stdout).unwrap();
    assert_eq!(report["component_count"], 2);
    assert_eq!(report["components"][1]["first_triangle"], 1);
    let run = |mode: &str, ids: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("select-mesh-components")
            .arg(&input)
            .arg(mode)
            .arg(ids)
            .arg(target)
            .output()
            .unwrap()
    };
    let kept = s.0.join("kept");
    assert!(run("keep", "1", &kept).status.success());
    let work = scancraft_geometry::work::WorkControl::default();
    let mesh = scancraft_workflow::mesh_document::load(&kept.join("mesh.json"), &work)
        .unwrap()
        .mesh;
    assert_eq!(
        mesh.vertices,
        vec![[10., 0., 0.], [11., 0., 0.], [10., 1., 0.]]
    );
    assert_eq!(fs::read(kept.join("original.mesh.json")).unwrap(), bytes);
    let removed = s.0.join("removed");
    assert!(run("remove", "0", &removed).status.success());
    assert_eq!(
        fs::read(removed.join("mesh.json")).unwrap(),
        fs::read(kept.join("mesh.json")).unwrap()
    );
    let data = fs::read(kept.join("project.meshproj")).unwrap();
    let mut editor = scancraft_geometry::mesh_project::load(data.as_slice(), &work).unwrap();
    editor.undo(&work).unwrap();
    assert_eq!(editor.mesh().triangles.len(), 2);
    for ids in ["", "1,1", "99"] {
        let path = s.0.join(format!("bad-{}", ids.len()));
        assert!(!run("keep", ids, &path).status.success());
        assert!(!path.exists());
    }
    assert!(!run("keep", "1", &kept).status.success());
}
