use scancraft_geometry::{
    Cloud, Editor, Point,
    formats::{load_project, save_project},
    measurement_snapshot::{ResultValue, Snapshot},
    work::WorkControl,
};
fn samples() -> Vec<[f64; 3]> {
    vec![[3., 0., 0.], [0., 0., 0.], [0., 4., 0.]]
}
#[test]
fn direct_evidence_survives_deletion_history_and_project_reopen() {
    let work = WorkControl::default();
    let distance =
        Snapshot::distance("Distance".into(), vec![samples()[0], samples()[2]], &work).unwrap();
    let angle = Snapshot::angle("Angle".into(), samples(), 1, &work).unwrap();
    let original = serde_json::to_vec(&angle).unwrap();
    let mut editor = Editor::new(
        Cloud {
            points: samples()
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
    .unwrap();
    editor.add_measurement(distance).unwrap();
    editor.add_measurement(angle).unwrap();
    editor.trim(&[0, 1, 2], false).unwrap();
    editor.remove_measurement(1).unwrap();
    let mut bytes = Vec::new();
    save_project(&editor, &mut bytes).unwrap();
    let mut reopened = load_project(bytes.as_slice()).unwrap();
    assert!(reopened.cloud.points.is_empty());
    assert!(reopened.undo());
    assert_eq!(reopened.cloud.points.len(), 3);
    assert_eq!(reopened.measurements().len(), 1);
    reopened.undo_measurement().unwrap();
    assert_eq!(
        serde_json::to_vec(&reopened.measurements()[1]).unwrap(),
        original
    );
    assert!(matches!(
        reopened.measurements()[0].result,
        ResultValue::Distance { distance_mm: 5. }
    ));
    reopened
        .rename_measurement(1, "Restored angle".into())
        .unwrap();
    assert!(!reopened.can_redo_measurement());
    assert_eq!(reopened.measurements()[1].source_points_mm, samples());
}
#[test]
fn invalid_or_inconsistent_direct_evidence_is_rejected() {
    let work = WorkControl::default();
    let mut angle = Snapshot::angle("Angle".into(), samples(), 1, &work).unwrap();
    angle.result = ResultValue::Angle {
        degrees: 45.,
        vertex: 1,
    };
    assert!(angle.validate().is_err());
    angle.result = ResultValue::Angle {
        degrees: 90.,
        vertex: 9,
    };
    assert!(angle.validate().is_err());
    angle.result = ResultValue::Angle {
        degrees: 90.,
        vertex: 1,
    };
    angle.schema_version = 1;
    assert!(angle.validate().is_err());
    let mut distance =
        Snapshot::distance("Distance".into(), vec![samples()[0], samples()[2]], &work).unwrap();
    distance.result = ResultValue::Distance { distance_mm: 4. };
    assert!(distance.validate().is_err());
    distance.result = ResultValue::Distance {
        distance_mm: f64::NAN,
    };
    assert!(distance.validate().is_err());
    assert!(Snapshot::angle("bad".into(), vec![[0.; 3]; 3], 1, &work).is_err());
    assert!(Snapshot::distance("bad".into(), samples(), &work).is_err());
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(Snapshot::angle("bad".into(), samples(), 1, &cancelled).is_err());
}
#[test]
fn crc_valid_corruption_is_rejected_on_project_load() {
    let mut editor = Editor::new(Cloud { points: vec![] }, 8).unwrap();
    editor
        .add_measurement(
            Snapshot::angle("Angle".into(), samples(), 1, &WorkControl::default()).unwrap(),
        )
        .unwrap();
    let mut bytes = Vec::new();
    save_project(&editor, &mut bytes).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes[16..]).unwrap();
    value["editor"]["measurements"][0]["result"]["degrees"] = serde_json::json!(45.);
    let payload = serde_json::to_vec(&value).unwrap();
    bytes.truncate(8);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&crc32fast::hash(&payload).to_le_bytes());
    bytes.extend(payload);
    assert!(load_project(bytes.as_slice()).is_err());
}
