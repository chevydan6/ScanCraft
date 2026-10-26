use scancraft_geometry::{
    Cloud, Editor, Point,
    formats::{load_project, save_project},
    project_origin::{Artifact, MAX_ORIGIN_BYTES, Origin},
};
use serde_json::{Value, json};
fn origin() -> Origin {
    Origin {
        schema_version: 1,
        kind: "initial_recorded_stereo_reconstruction".into(),
        bundle_directory_display: "/example/bundle".into(),
        calibration: json!({"record_id":"synthetic"}),
        request: json!({"source":"synthetic_fixture"}),
        recording_sources: json!({"source_device_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,"left":{"selected_frame_index":1},"right":{"selected_frame_index":2}}),
        artifacts: [
            "left-frame.raptor",
            "right-frame.raptor",
            "calibration.rapcal",
            "request.json",
            "recording-sources.json",
        ]
        .into_iter()
        .map(|name| Artifact {
            name: name.into(),
            bytes: 12,
            crc32: 123,
        })
        .collect(),
    }
}
fn editor() -> Editor {
    Editor::new(
        Cloud {
            points: vec![
                Point {
                    position: [1., 2., 3.],
                    color: None,
                    normal: None,
                },
                Point {
                    position: [2., 3., 4.],
                    color: None,
                    normal: None,
                },
            ],
        },
        8,
    )
    .unwrap()
}
fn bytes(editor: &Editor) -> Vec<u8> {
    let mut bytes = Vec::new();
    save_project(editor, &mut bytes).unwrap();
    bytes
}
fn reframe(value: &Value, version: u8) -> Vec<u8> {
    let data = serde_json::to_vec(value).unwrap();
    let mut bytes = format!("ORPROJ0{version}").into_bytes();
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&crc32fast::hash(&data).to_le_bytes());
    bytes.extend_from_slice(&data);
    bytes
}
#[test]
fn provenance_survives_edit_save_reopen_undo_redo_without_external_files() {
    let mut editor = editor();
    let original = editor.cloud.clone();
    editor.attach_origin(origin()).unwrap();
    assert!(editor.attach_origin(origin()).is_err());
    editor.trim(&[0], true).unwrap();
    let encoded = bytes(&editor);
    assert_eq!(&encoded[..8], b"ORPROJ05");
    let mut loaded = load_project(encoded.as_slice()).unwrap();
    assert_eq!(loaded.origin(), Some(&origin()));
    assert!(loaded.undo());
    assert_eq!(loaded.cloud, original);
    assert!(loaded.redo());
    assert_eq!(loaded.cloud.points.len(), 1);
    assert_eq!(loaded.origin(), Some(&origin()));
    assert_eq!(bytes(&loaded), encoded);
}
#[test]
fn old_versions_cannot_hide_provenance_and_new_version_requires_it() {
    let mut editor = editor();
    assert_eq!(&bytes(&editor)[..8], b"ORPROJ02");
    editor.attach_origin(origin()).unwrap();
    let encoded = bytes(&editor);
    let mut value: Value = serde_json::from_slice(&encoded[16..]).unwrap();
    value["version"] = 4.into();
    assert!(load_project(reframe(&value, 4).as_slice()).is_err());
    value["version"] = 5.into();
    value["editor"].as_object_mut().unwrap().remove("origin");
    assert!(load_project(reframe(&value, 5).as_slice()).is_err());
}
#[test]
fn crc_valid_invalid_provenance_is_rejected() {
    let mut editor = editor();
    editor.attach_origin(origin()).unwrap();
    let encoded = bytes(&editor);
    let valid: Value = serde_json::from_slice(&encoded[16..]).unwrap();
    for flag in [
        "source_device_binding_verified",
        "synchronization_verified",
        "physical_accuracy_qualified",
    ] {
        let mut bad = valid.clone();
        bad["editor"]["origin"]["recording_sources"][flag] = true.into();
        assert!(load_project(reframe(&bad, 5).as_slice()).is_err());
    }
    let mut bad = valid.clone();
    bad["editor"]["origin"]["artifacts"][0]["name"] = "../escape".into();
    assert!(load_project(reframe(&bad, 5).as_slice()).is_err());
    let mut bad = valid.clone();
    bad["editor"]["origin"]["unexpected"] = true.into();
    assert!(load_project(reframe(&bad, 5).as_slice()).is_err());
    let mut invalid = origin();
    invalid.artifacts[1] = invalid.artifacts[0].clone();
    assert!(invalid.validate().is_err());
    let mut invalid = origin();
    invalid.request = json!({"oversized":"x".repeat(MAX_ORIGIN_BYTES)});
    assert!(invalid.validate().unwrap_err().contains("8 MiB"));
}

#[test]
fn v5_preserves_measurement_history_and_refuses_late_origin_attachment() {
    let mut edited = editor();
    edited.trim(&[0], true).unwrap();
    assert!(edited.attach_origin(origin()).is_err());
    let mut e = editor();
    e.attach_origin(origin()).unwrap();
    let snapshot = scancraft_geometry::measurement_snapshot::Snapshot::distance(
        "initial distance".into(),
        vec![[1., 2., 3.], [2., 3., 4.]],
        &scancraft_geometry::work::WorkControl::default(),
    )
    .unwrap();
    e.add_measurement(snapshot).unwrap();
    let encoded = bytes(&e);
    assert_eq!(&encoded[..8], b"ORPROJ05");
    let mut loaded = load_project(encoded.as_slice()).unwrap();
    assert_eq!(loaded.measurements().len(), 1);
    assert!(loaded.undo_measurement().unwrap());
    assert!(loaded.measurements().is_empty());
    assert!(loaded.redo_measurement().unwrap());
    assert_eq!(loaded.origin(), Some(&origin()));
    assert_eq!(bytes(&loaded), encoded);
}

#[test]
fn v5_rejects_every_truncated_envelope_and_single_byte_corruption() {
    let mut e = editor();
    e.attach_origin(origin()).unwrap();
    let encoded = bytes(&e);
    for end in 0..encoded.len() {
        assert!(load_project(&encoded[..end]).is_err());
    }
    for index in 0..encoded.len() {
        let mut changed = encoded.clone();
        changed[index] ^= 1;
        assert!(load_project(changed.as_slice()).is_err());
    }
}

fn recording_editor() -> (Editor, scancraft_geometry::recording_references::Recordings) {
    use scancraft_geometry::recording_references::{File, Recordings};
    let mut editor = editor();
    let mut origin = origin();
    for side in ["left", "right"] {
        origin.recording_sources[side]["source_total_frames"] = json!(3);
        origin.recording_sources[side]["source_version"] = json!(2);
    }
    editor.attach_origin(origin).unwrap();
    let refs = Recordings {
        schema_version: 1,
        directory_display: "/missing-synthetic-recordings-not-opened-by-project-parser".into(),
        files: [
            File {
                name: "left.raptor".into(),
                bytes: 123,
                crc32: 45,
                frames: 3,
                version: 2,
                selected_frame_index: 1,
            },
            File {
                name: "right.raptor".into(),
                bytes: 124,
                crc32: 46,
                frames: 3,
                version: 2,
                selected_frame_index: 2,
            },
        ],
    };
    (editor, refs)
}
#[test]
fn v6_recording_references_survive_edits_save_load_and_exact_undo_without_external_reads() {
    let (editor, refs) = recording_editor();
    let original = editor.cloud.clone();
    let mut editor = editor.copied_with_recordings(refs.clone()).unwrap();
    editor
        .add_measurement(
            scancraft_geometry::measurement_snapshot::Snapshot::distance(
                "reference distance".into(),
                vec![[1., 2., 3.], [2., 3., 4.]],
                &scancraft_geometry::work::WorkControl::default(),
            )
            .unwrap(),
        )
        .unwrap();
    editor.trim(&[0], true).unwrap();
    let encoded = bytes(&editor);
    assert_eq!(&encoded[..8], b"ORPROJ06");
    let mut loaded = load_project(encoded.as_slice()).unwrap();
    assert_eq!(loaded.recordings(), Some(&refs));
    assert!(loaded.undo());
    assert_eq!(loaded.cloud, original);
    assert!(loaded.redo());
    assert_eq!(loaded.recordings(), Some(&refs));
    assert!(loaded.undo_measurement().unwrap());
    assert!(loaded.measurements().is_empty());
    assert!(loaded.redo_measurement().unwrap());
    assert_eq!(loaded.measurements().len(), 1);
    assert_eq!(loaded.recordings(), Some(&refs));
    assert_eq!(bytes(&loaded), encoded);
}
#[test]
fn version_gates_prevent_recording_references_hiding_in_old_projects() {
    let (editor, refs) = recording_editor();
    let editor = editor.copied_with_recordings(refs).unwrap();
    let encoded = bytes(&editor);
    let value: Value = serde_json::from_slice(&encoded[16..]).unwrap();
    for version in 2..=5 {
        let mut old = value.clone();
        old["version"] = json!(version);
        assert!(load_project(reframe(&old, version).as_slice()).is_err());
    }
    let mut no_refs = value.clone();
    no_refs["editor"]
        .as_object_mut()
        .unwrap()
        .remove("recordings");
    assert!(load_project(reframe(&no_refs, 6).as_slice()).is_err());
    let mut no_origin = value;
    no_origin["editor"]
        .as_object_mut()
        .unwrap()
        .remove("origin");
    assert!(load_project(reframe(&no_origin, 6).as_slice()).is_err());
}
#[test]
fn invalid_recording_reference_paths_budgets_roles_and_lineage_are_rejected_atomically() {
    let (editor, refs) = recording_editor();
    let before = bytes(&editor);
    for (field, bad) in [
        ("name", json!("../left.raptor")),
        ("bytes", json!(0)),
        ("bytes", json!((1u64 << 40) + 1)),
        ("frames", json!(1_000_001)),
        ("frames", json!(2)),
        ("version", json!(3)),
        ("selected_frame_index", json!(0)),
    ] {
        let mut value = serde_json::to_value(&refs).unwrap();
        value["files"][0][field] = bad;
        let invalid = serde_json::from_value(value).unwrap();
        assert!(editor.copied_with_recordings(invalid).is_err());
        assert_eq!(bytes(&editor), before);
    }
    let mut invalid = refs.clone();
    invalid.directory_display.clear();
    assert!(editor.copied_with_recordings(invalid).is_err());
    let mut invalid = refs;
    invalid.files.swap(0, 1);
    assert!(editor.copied_with_recordings(invalid).is_err());
    assert!(
        Editor::new(editor.cloud.clone(), 8)
            .unwrap()
            .copied_with_recordings(recording_editor().1)
            .is_err()
    );
}
