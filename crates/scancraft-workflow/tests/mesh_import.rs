use scancraft_geometry::{
    mesh_import::{Config, Format, Units},
    mesh_project,
    work::WorkControl,
};
use scancraft_workflow::{mesh_document, mesh_trim};
use std::{fs, path::PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "scancraft-import-{}-{}",
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
#[test]
fn import_preserves_bytes_units_reopened_geometry_and_further_edit_history() {
    let temp = Temp::new();
    let input = temp.0.join("input.obj");
    let bytes =
        b"# source units explicitly supplied as inches\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
    fs::write(&input, bytes).unwrap();
    let target = temp.0.join("imported");
    let work = WorkControl::default();
    let config = Config {
        format: Format::Obj,
        units: Units::Inches,
        weld_exact: false,
    };
    let summary = mesh_trim::import_mesh(&input, config.clone(), &target, &work).unwrap();
    assert_eq!(summary["operation"], "import_mesh");
    assert_eq!(fs::read(target.join("original.obj")).unwrap(), bytes);
    assert_eq!(fs::read(&input).unwrap(), bytes);
    let loaded = mesh_document::load(&target.join("mesh.json"), &work).unwrap();
    assert_eq!(loaded.mesh.vertices[1], [25.4, 0., 0.]);
    let mut editor = mesh_project::load(
        fs::File::open(target.join("project.meshproj")).unwrap(),
        &work,
    )
    .unwrap();
    assert_eq!(editor.mesh(), &loaded.mesh);
    assert!(!editor.undo(&work).unwrap());
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["parameters"]["configuration"]["units"], "inches");
    assert_eq!(report["physical_accuracy_qualified"], false);
    let prepared =
        mesh_trim::prepare(&target.join("project.meshproj"), [1., 0., 0.], -12.7, &work).unwrap();
    let edited = temp.0.join("edited");
    mesh_trim::publish_prepared(&prepared, &edited, &work).unwrap();
    let mut editor = mesh_project::load(
        fs::File::open(edited.join("project.meshproj")).unwrap(),
        &work,
    )
    .unwrap();
    assert!(editor.undo(&work).unwrap());
    assert_eq!(editor.mesh(), &loaded.mesh);
    assert!(mesh_trim::import_mesh(&input, config, &target, &work).is_err());
    assert_eq!(fs::read(target.join("original.obj")).unwrap(), bytes);
}
#[test]
fn invalid_and_cancelled_import_never_publish_or_modify_source() {
    let temp = Temp::new();
    let input = temp.0.join("bad.obj");
    let bytes = b"v 0 0 0\nf 1 2 3";
    fs::write(&input, bytes).unwrap();
    let work = WorkControl::default();
    let c = Config {
        format: Format::Obj,
        units: Units::Millimeters,
        weld_exact: false,
    };
    let out = temp.0.join("out");
    assert!(mesh_trim::import_mesh(&input, c.clone(), &out, &work).is_err());
    assert!(!out.exists());
    assert_eq!(fs::read(&input).unwrap(), bytes);
    work.cancel();
    assert!(mesh_trim::import_mesh(&input, c, &out, &work).is_err());
    assert!(!out.exists());
}
