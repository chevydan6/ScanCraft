use scancraft_geometry::{
    Cloud, Editor,
    formats::{load_project, save_project},
    measurement_snapshot::Snapshot,
    work::WorkControl,
};
fn snapshot(label: &str) -> Snapshot {
    Snapshot::sphere(
        label.into(),
        vec![
            [5., 0., 0.],
            [-5., 0., 0.],
            [0., 5., 0.],
            [0., -5., 0.],
            [0., 0., 5.],
            [0., 0., -5.],
        ],
        &WorkControl::default(),
    )
    .unwrap()
}
fn bytes(editor: &Editor) -> Vec<u8> {
    let mut out = Vec::new();
    save_project(editor, &mut out).unwrap();
    out
}
fn reframe(value: &serde_json::Value, magic: &[u8; 8]) -> Vec<u8> {
    let body = serde_json::to_vec(value).unwrap();
    let mut out = magic.to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&crc32fast::hash(&body).to_le_bytes());
    out.extend(body);
    out
}
#[test]
fn deleted_source_evidence_and_metadata_history_survive_save_reopen_and_branching() {
    let mut editor = Editor::new(Cloud { points: vec![] }, 8).unwrap();
    let original = snapshot("Ball");
    let source = serde_json::to_vec(&original.source_points_mm).unwrap();
    editor.add_measurement(original).unwrap();
    editor
        .rename_measurement(0, "Reference ball".into())
        .unwrap();
    editor.remove_measurement(0).unwrap();
    assert!(editor.measurements().is_empty());
    let encoded = bytes(&editor);
    assert_eq!(&encoded[..8], b"ORPROJ04");
    let mut editor = load_project(encoded.as_slice()).unwrap();
    assert!(editor.undo_measurement().unwrap());
    assert_eq!(editor.measurements()[0].label, "Reference ball");
    assert_eq!(
        serde_json::to_vec(&editor.measurements()[0].source_points_mm).unwrap(),
        source
    );
    assert!(editor.undo_measurement().unwrap());
    assert_eq!(editor.measurements()[0].label, "Ball");
    assert!(editor.redo_measurement().unwrap());
    assert_eq!(editor.measurements()[0].label, "Reference ball");
    assert!(editor.redo_measurement().unwrap());
    assert!(editor.measurements().is_empty());
    assert!(editor.undo_measurement().unwrap());
    editor.add_measurement(snapshot("Other")).unwrap();
    assert!(!editor.can_redo_measurement());
    assert_eq!(editor.measurements().len(), 2);
    assert!(editor.cloud.points.is_empty());
    assert!(load_project(bytes(&editor).as_slice()).is_ok());
}
#[test]
fn failed_changes_are_atomic_and_history_is_bounded() {
    let mut editor = Editor::new(Cloud { points: vec![] }, 8).unwrap();
    editor.add_measurement(snapshot("Ball")).unwrap();
    editor.add_measurement(snapshot("Other")).unwrap();
    let before = bytes(&editor);
    for label in ["", "Other", "Ball", "bad\nlabel"] {
        assert!(editor.rename_measurement(0, label.into()).is_err());
        assert_eq!(bytes(&editor), before);
    }
    assert!(editor.remove_measurement(99).is_err());
    assert_eq!(bytes(&editor), before);
    for i in 0..40 {
        editor.rename_measurement(0, format!("Ball {i}")).unwrap();
    }
    let mut count = 0;
    while editor.undo_measurement().unwrap() {
        count += 1;
    }
    assert_eq!(count, 16);
    assert!(load_project(bytes(&editor).as_slice()).is_ok());
}
#[test]
fn version_three_migration_and_crc_valid_inconsistent_history_are_checked() {
    let mut editor = Editor::new(Cloud { points: vec![] }, 8).unwrap();
    editor.add_measurement(snapshot("Ball")).unwrap();
    let encoded = bytes(&editor);
    let mut value: serde_json::Value = serde_json::from_slice(&encoded[16..]).unwrap();
    value["editor"]["measurement_undo"][0]["label"] = "Wrong label".into();
    assert!(load_project(reframe(&value, b"ORPROJ04").as_slice()).is_err());
    value = serde_json::from_slice(&encoded[16..]).unwrap();
    value["version"] = 3.into();
    assert!(load_project(reframe(&value, b"ORPROJ03").as_slice()).is_err());
    value["editor"]
        .as_object_mut()
        .unwrap()
        .remove("measurement_undo");
    let mut migrated = load_project(reframe(&value, b"ORPROJ03").as_slice()).unwrap();
    assert_eq!(migrated.measurements()[0].label, "Ball");
    assert!(!migrated.has_measurement_history());
    assert_eq!(&bytes(&migrated)[..8], b"ORPROJ03");
    migrated.rename_measurement(0, "Renamed".into()).unwrap();
    assert_eq!(&bytes(&migrated)[..8], b"ORPROJ04");
}
