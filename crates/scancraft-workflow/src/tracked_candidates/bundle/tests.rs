use super::*;
use crate::tracked_candidates::tests::{Temp, fixture, request, write};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
fn setup(t: &Temp) -> PathBuf {
    let paths = vec![
        fixture(&t.0, 0, true, true, 0),
        fixture(&t.0, 1, false, true, 0),
        fixture(&t.0, 2, true, false, 0),
        fixture(&t.0, 3, true, true, 10),
        fixture(&t.0, 4, true, true, 0),
    ];
    let settings = t.0.join("settings.json");
    write(&settings, &request(&paths));
    let target = t.0.join("result");
    crate::tracked_candidates::run(&settings, &target, &WorkControl::default()).unwrap();
    target
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn redigest(root: &Path, name: &str) {
    let path = root.join("manifest.json");
    let mut m = read(&path);
    let b = fs::read(root.join(name)).unwrap();
    let a = m["artifacts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["role"] == name)
        .unwrap();
    a["bytes"] = json!(b.len());
    a["crc32"] = json!(crc32fast::hash(&b));
    write(&path, &m);
}
#[test]
fn loads_fused_coordinates_all_outcomes_and_selected_original_camera_pair() {
    let t = Temp::new();
    let root = setup(&t);
    let a = load(&root, 0, &WorkControl::default()).unwrap();
    let b = load(&root, 2, &WorkControl::default()).unwrap();
    assert_eq!(a.positions, b.positions);
    assert_eq!(a.positions.len(), 1);
    assert_ne!(a.cameras[0].pixels, b.cameras[0].pixels);
    assert_eq!(
        (
            b.session.integrated,
            b.session.tracking_only,
            b.session.rejected,
            b.session.unavailable
        ),
        (2, 1, 1, 1)
    );
    assert_eq!(b.session.selected_frame, 2);
    assert_eq!(b.session.frames[3].status, FrameStatus::TrackingRejected);
    assert!(load(&root, 5, &WorkControl::default()).is_err());
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(load(&root, 0, &cancelled).is_err());
}
#[test]
fn detects_corruption_qualification_changes_unsafe_roles_and_missing_sources() {
    let t = Temp::new();
    let root = setup(&t);
    let manifest = read(&root.join("manifest.json"));
    for (key, value) in [
        ("units_verified", json!(true)),
        ("fused_candidates", json!(2)),
        ("frames", json!(257)),
        ("tracking_only_frames", json!(9)),
    ] {
        let mut m = manifest.clone();
        m[key] = value;
        write(&root.join("manifest.json"), &m);
        assert!(load(&root, 0, &WorkControl::default()).is_err());
    }
    for role in ["../request.json", "/etc/passwd", "geometry.json"] {
        let mut m = manifest.clone();
        m["artifacts"][0]["role"] = json!(role);
        write(&root.join("manifest.json"), &m);
        assert!(load(&root, 0, &WorkControl::default()).is_err());
    }
    write(&root.join("manifest.json"), &manifest);
    let marker = root.join("markers.ndjson");
    let bytes = fs::read(&marker).unwrap();
    fs::write(&marker, b"corrupt\n").unwrap();
    assert!(
        load(&root, 0, &WorkControl::default())
            .err()
            .unwrap()
            .to_string()
            .contains("CRC")
    );
    fs::write(&marker, bytes).unwrap();
    fs::rename(t.0.join("pair-4"), t.0.join("moved-pair")).unwrap();
    assert!(load(&root, 0, &WorkControl::default()).is_err());
}
#[test]
fn inconsistent_frame_rows_even_with_fresh_crc_do_not_load() {
    let t = Temp::new();
    let root = setup(&t);
    let path = root.join("frames.ndjson");
    let original = fs::read_to_string(&path).unwrap();
    let rows: Vec<Value> = original
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    for (index, key, value) in [
        (0, "accepted", json!(false)),
        (1, "surface_points", json!(1)),
        (
            2,
            "pose",
            json!({"rotation":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"translation_model_units":[0.,0.,0.]}),
        ),
        (3, "last_integrated", json!(3)),
        (4, "frame_id", json!(0)),
    ] {
        let mut modified = rows.clone();
        modified[index][key] = value;
        fs::write(
            &path,
            modified
                .iter()
                .map(|v| serde_json::to_string(v).unwrap() + "\n")
                .collect::<String>(),
        )
        .unwrap();
        redigest(&root, "frames.ndjson");
        assert!(
            load(&root, 0, &WorkControl::default()).is_err(),
            "{index} {key}"
        );
    }
}
#[test]
fn source_changes_symlinks_unterminated_rows_and_mutation_before_commit_are_errors() {
    let t = Temp::new();
    let root = setup(&t);
    let path = root.join("sources.ndjson");
    let original = fs::read(&path).unwrap();
    fs::write(&path, &original[..original.len() - 1]).unwrap();
    redigest(&root, "sources.ndjson");
    assert!(load(&root, 0, &WorkControl::default()).is_err());
    fs::write(&path, &original).unwrap();
    redigest(&root, "sources.ndjson");
    let moved = t.0.join("sources.saved");
    fs::rename(&path, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &path).unwrap();
    assert!(load(&root, 0, &WorkControl::default()).is_err());
    fs::remove_file(&path).unwrap();
    fs::rename(&moved, &path).unwrap();
    assert!(
        load_inner(&root, 0, &WorkControl::default(), &|| {
            fs::write(root.join("fused-candidates.json"), b"changed")?;
            Ok(())
        })
        .err()
        .unwrap()
        .to_string()
        .contains("changed")
    );
}
