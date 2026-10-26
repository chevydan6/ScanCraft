use scancraft_geometry::{mesh::Mesh, mesh_project, mesh_simplify::Config, work::WorkControl};
use scancraft_workflow::mesh_trim::{prepare_simplify, publish_prepared};
use std::{fs, path::PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "scancraft-simplify-workflow-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Mesh {
    let n = 8;
    let vertices = (0..n)
        .flat_map(|y| (0..n).map(move |x| [x as f64, y as f64, 0.]))
        .collect();
    let mut triangles = Vec::new();
    for y in 0..n - 1 {
        for x in 0..n - 1 {
            let a = (y * n + x) as u32;
            triangles.extend([
                [a, a + 1, a + n as u32 + 1],
                [a, a + n as u32 + 1, a + n as u32],
            ]);
        }
    }
    Mesh {
        vertices,
        triangles,
    }
}
fn config() -> Config {
    Config {
        target_triangles: 70,
        maximum_displacement_mm: 1.5,
        maximum_normal_change_degrees: 30.,
        preserve_crease_degrees: 30.,
        maximum_collapses: 100,
    }
}
#[test]
fn published_simplification_preserves_original_bytes_and_exact_reopened_undo() {
    let temp = Temp::new();
    let source = fixture();
    let input = temp.0.join("source.json");
    let bytes = serde_json::to_vec_pretty(
        &serde_json::json!({"schema_version":1,"units":"millimeters","mesh":source}),
    )
    .unwrap();
    fs::write(&input, &bytes).unwrap();
    let work = WorkControl::default();
    let prepared = prepare_simplify(&input, config(), &work).unwrap();
    assert_eq!(prepared.original_mesh(), &source);
    assert_eq!(fs::read(&input).unwrap(), bytes);
    let target = temp.0.join("result");
    assert!(!target.exists());
    let summary = publish_prepared(&prepared, &target, &work).unwrap();
    assert_eq!(summary["operation"], "simplify");
    assert_eq!(fs::read(target.join("original.mesh.json")).unwrap(), bytes);
    assert_eq!(fs::read(&input).unwrap(), bytes);
    let mut editor = mesh_project::load(
        fs::File::open(target.join("project.meshproj")).unwrap(),
        &work,
    )
    .unwrap();
    assert!(editor.mesh().triangles.len() <= 70);
    assert!(editor.undo(&work).unwrap());
    assert_eq!(editor.mesh(), &source);
    assert!(editor.redo(&work).unwrap());
    assert_eq!(editor.mesh(), prepared.mesh());
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    assert_eq!(
        report["parameters"]["source_to_output_vertices"]
            .as_array()
            .unwrap()
            .len(),
        source.vertices.len()
    );
    assert_eq!(report["parameters"]["decimation"]["target_reached"], true);
    assert_eq!(report["physical_accuracy_qualified"], false);
    let sentinel = target.join("sentinel");
    fs::write(&sentinel, b"keep").unwrap();
    assert!(publish_prepared(&prepared, &target, &work).is_err());
    assert_eq!(fs::read(sentinel).unwrap(), b"keep");
}
#[test]
fn cancelled_publication_has_no_final_destination_and_retains_source() {
    let temp = Temp::new();
    let input = temp.0.join("source.json");
    let bytes = serde_json::to_vec(
        &serde_json::json!({"schema_version":1,"units":"millimeters","mesh":fixture()}),
    )
    .unwrap();
    fs::write(&input, &bytes).unwrap();
    let work = WorkControl::default();
    let prepared = prepare_simplify(&input, config(), &work).unwrap();
    work.cancel();
    let output = temp.0.join("cancelled");
    assert!(publish_prepared(&prepared, &output, &work).is_err());
    assert!(!output.exists());
    assert_eq!(fs::read(input).unwrap(), bytes);
}
