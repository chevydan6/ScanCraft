use scancraft_geometry::{
    Cloud, Editor, Point,
    formats::{load_project, save_project},
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "scancraft-statistical-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn original_preserving_cli_keeps_attributes_and_exact_undo_then_refuses_overwrite() {
    let t = Temp::new();
    let cloud = Cloud {
        points: vec![[0., 0., 0.], [1., 0., 0.], [10., 0., 0.]]
            .into_iter()
            .map(|position| Point {
                position,
                color: Some([1, 2, 3]),
                normal: Some([0., 0., 1.]),
            })
            .collect(),
    };
    let input = t.0.join("input.orproj");
    let mut bytes = Vec::new();
    save_project(&Editor::new(cloud.clone(), 8).unwrap(), &mut bytes).unwrap();
    fs::write(&input, &bytes).unwrap();
    let target = t.0.join("result");
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .args([
                "filter-statistical",
                input.to_str().unwrap(),
                "1",
                "1",
                target.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let out = run();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut editor = load_project(fs::File::open(target.join("project.orproj")).unwrap()).unwrap();
    assert_eq!(editor.cloud.points, cloud.points[..2]);
    assert!(editor.undo());
    assert_eq!(editor.cloud, cloud);
    assert_eq!(fs::read(&input).unwrap(), bytes);
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    assert_eq!(
        report["statistics"]["outlier_indices"],
        serde_json::json!([2])
    );
    assert!(!run().status.success());
}
#[test]
fn invalid_parameters_never_publish_and_empty_removal_respects_disabled_history() {
    let t = Temp::new();
    let input = t.0.join("input.orproj");
    let cloud = Cloud {
        points: vec![
            Point {
                position: [0.; 3],
                color: None,
                normal: None
            };
            3
        ],
    };
    let mut bytes = Vec::new();
    save_project(&Editor::new(cloud, 0).unwrap(), &mut bytes).unwrap();
    fs::write(&input, &bytes).unwrap();
    let target = t.0.join("bad");
    let out = Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
        .args([
            "filter-statistical",
            input.to_str().unwrap(),
            "3",
            "1",
            target.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!target.exists());
    let target = t.0.join("valid");
    let out = Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
        .args([
            "filter-statistical",
            input.to_str().unwrap(),
            "1",
            "1",
            target.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["removed_points"], 0);
    assert_eq!(report["undo_available_after_filter"], false);
    assert_eq!(fs::read(input).unwrap(), bytes);
}
