use scancraft_geometry::{
    Cloud, Editor, Point,
    formats::{load_project, save_project},
    measurement_snapshot::{ResultValue, Snapshot},
    work::WorkControl,
};
fn points() -> Vec<[f64; 3]> {
    [-6., 0., 6.]
        .into_iter()
        .flat_map(|z| {
            (0..24).map(move |i| {
                let t = i as f64 * std::f64::consts::TAU / 24.;
                [3. * t.cos(), 3. * t.sin(), z]
            })
        })
        .collect()
}
fn snapshot() -> Snapshot {
    Snapshot::cylinder("Cylinder".into(), points(), 1e-6, &WorkControl::default()).unwrap()
}
#[test]
fn source_evidence_and_deleted_cylinder_history_survive_geometry_edits_and_reopen() {
    let source = points();
    let mut editor = Editor::new(
        Cloud {
            points: source
                .iter()
                .map(|&position| Point {
                    position,
                    color: None,
                    normal: None,
                })
                .collect(),
        },
        8,
    )
    .unwrap();
    let original = snapshot();
    assert_eq!(original.schema_version, 4);
    let source_bytes = serde_json::to_vec(&original.source_points_mm).unwrap();
    editor.add_measurement(original).unwrap();
    editor
        .rename_measurement(0, "Reference cylinder".into())
        .unwrap();
    editor.remove_measurement(0).unwrap();
    editor
        .trim(&(0..source.len()).collect::<Vec<_>>(), false)
        .unwrap();
    let mut bytes = Vec::new();
    save_project(&editor, &mut bytes).unwrap();
    assert_eq!(&bytes[..8], b"ORPROJ04");
    let mut reopened = load_project(bytes.as_slice()).unwrap();
    assert!(reopened.cloud.points.is_empty());
    assert!(reopened.measurements().is_empty());
    assert!(reopened.undo_measurement().unwrap());
    let record = &reopened.measurements()[0];
    assert_eq!(record.label, "Reference cylinder");
    assert_eq!(
        serde_json::to_vec(&record.source_points_mm).unwrap(),
        source_bytes
    );
    assert!(
        matches!(&record.result,ResultValue::Cylinder {fit,..} if (fit.diameter_mm-6.).abs()<1e-6)
    );
    assert!(reopened.undo());
    assert_eq!(reopened.cloud.points.len(), source.len());
    assert!(reopened.redo_measurement().unwrap());
    assert!(reopened.measurements().is_empty());
}
#[test]
fn malformed_cylinder_parameters_and_schema_are_rejected() {
    let valid = snapshot();
    let mut wrong = valid.clone();
    wrong.schema_version = 3;
    assert!(wrong.validate().is_err());
    wrong = valid.clone();
    wrong.physical_accuracy_qualified = true;
    assert!(wrong.validate().is_err());
    for kind in 0..10 {
        let mut bad = valid.clone();
        if let ResultValue::Cylinder {
            fit,
            maximum_rms_mm,
        } = &mut bad.result
        {
            match kind {
                0 => fit.axis_direction = [0.; 3],
                1 => fit.radius_mm = -1.,
                2 => fit.diameter_mm = 0.,
                3 => fit.iterations = 0,
                4 => fit.starts_converged = 4,
                5 => fit.points += 1,
                6 => fit.axial_range_mm = [1., -1.],
                7 => fit.jacobian_condition_number = 1e9,
                8 => *maximum_rms_mm = -1.,
                _ => fit.maximum_absolute_residual_mm = f64::INFINITY,
            }
        }
        assert!(bad.validate().is_err(), "{kind}");
    }
    let work = WorkControl::default();
    work.cancel();
    assert!(Snapshot::cylinder("Cancelled".into(), points(), 1e-6, &work).is_err());
}
#[test]
fn crc_valid_invalid_fit_is_rejected_by_project_loader() {
    let mut editor = Editor::new(Cloud { points: vec![] }, 8).unwrap();
    editor.add_measurement(snapshot()).unwrap();
    let mut bytes = Vec::new();
    save_project(&editor, &mut bytes).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes[16..]).unwrap();
    value["editor"]["measurements"][0]["result"]["fit"]["iterations"] = serde_json::json!(0);
    let payload = serde_json::to_vec(&value).unwrap();
    bytes.truncate(8);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&crc32fast::hash(&payload).to_le_bytes());
    bytes.extend(payload);
    assert!(load_project(bytes.as_slice()).is_err());
}
