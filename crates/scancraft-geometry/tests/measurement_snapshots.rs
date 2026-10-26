use scancraft_geometry::{
    Cloud, Editor, Point,
    formats::{load_project, save_project},
    measurement_snapshot::Snapshot,
    work::WorkControl,
};
fn points() -> Vec<[f64; 3]> {
    vec![
        [5., 0., 0.],
        [-5., 0., 0.],
        [0., 5., 0.],
        [0., -5., 0.],
        [0., 0., 5.],
        [0., 0., -5.],
    ]
}
fn editor() -> Editor {
    Editor::new(
        Cloud {
            points: points()
                .into_iter()
                .map(|position| Point {
                    position,
                    color: None,
                    normal: None,
                })
                .collect(),
        },
        8,
    )
    .unwrap()
}
#[test]
fn snapshots_upgrade_project_and_survive_geometry_edits_undo_and_reopen() {
    let mut editor = editor();
    let mut old = Vec::new();
    save_project(&editor, &mut old).unwrap();
    assert_eq!(&old[..8], b"ORPROJ02");
    assert!(
        load_project(old.as_slice())
            .unwrap()
            .measurements()
            .is_empty()
    );
    let snapshot = Snapshot::sphere("Ball".into(), points(), &WorkControl::default()).unwrap();
    let expected = serde_json::to_vec(&snapshot).unwrap();
    editor.add_measurement(snapshot).unwrap();
    editor.trim(&[0], false).unwrap();
    assert_eq!(editor.measurements()[0].source_points_mm.len(), 6);
    let mut bytes = Vec::new();
    save_project(&editor, &mut bytes).unwrap();
    assert_eq!(&bytes[..8], b"ORPROJ04");
    let mut loaded = load_project(bytes.as_slice()).unwrap();
    assert_eq!(loaded.cloud.points.len(), 5);
    assert_eq!(
        serde_json::to_vec(&loaded.measurements()[0]).unwrap(),
        expected
    );
    assert!(loaded.undo());
    assert_eq!(loaded.cloud.points.len(), 6);
    assert_eq!(loaded.measurements().len(), 1);
}
#[test]
fn circle_settings_duplicate_labels_and_snapshot_bounds_are_checked() {
    let mut editor = editor();
    let circle = Snapshot::circle(
        "Rim".into(),
        points()[..4].to_vec(),
        0.01,
        &WorkControl::default(),
    )
    .unwrap();
    editor.add_measurement(circle.clone()).unwrap();
    assert!(editor.add_measurement(circle.clone()).is_err());
    for i in 1..16 {
        let mut snapshot = circle.clone();
        snapshot.label = format!("Rim {i}");
        editor.add_measurement(snapshot).unwrap();
    }
    let mut seventeenth = circle;
    seventeenth.label = "Too many".into();
    assert!(editor.add_measurement(seventeenth).is_err());
    assert_eq!(editor.measurements().len(), 16);
    let mut bytes = Vec::new();
    save_project(&editor, &mut bytes).unwrap();
    assert_eq!(
        load_project(bytes.as_slice()).unwrap().measurements().len(),
        16
    );
}
#[test]
fn crc_valid_malformed_snapshots_and_version_mismatches_are_rejected() {
    let mut editor = editor();
    editor
        .add_measurement(
            Snapshot::sphere("Ball".into(), points(), &WorkControl::default()).unwrap(),
        )
        .unwrap();
    let mut bytes = Vec::new();
    save_project(&editor, &mut bytes).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&bytes[16..]).unwrap();
    for case in 0..4 {
        let mut value = original.clone();
        match case {
            0 => value["editor"]["measurements"][0]["physical_accuracy_qualified"] = true.into(),
            1 => value["editor"]["measurements"][0]["result"]["fit"]["radius_mm"] = (-1.).into(),
            2 => value["version"] = 3.into(),
            _ => value["editor"]["measurements"][0]["source_points_mm"] = serde_json::json!([]),
        }
        let body = serde_json::to_vec(&value).unwrap();
        let mut corrupt = b"ORPROJ04".to_vec();
        corrupt.extend_from_slice(&(body.len() as u32).to_le_bytes());
        corrupt.extend_from_slice(&crc32fast::hash(&body).to_le_bytes());
        corrupt.extend(body);
        assert!(load_project(corrupt.as_slice()).is_err());
    }
}
