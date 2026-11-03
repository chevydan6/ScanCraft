use super::super::tests::{Files, segmented};
use super::*;
use crate::laser_stream::{bundle::PreviewStatus, tests::Fake};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize},
};
fn capture(schema: u32) -> (Files, std::path::PathBuf) {
    let f = Files::new();
    f.change(|r| r["maximum_frames_per_camera"] = json!(2));
    if schema == 2 {
        segmented(&f, 2);
    }
    let frames = f.frames.clone();
    let target = f.root.join("capture");
    let report = f
        .plan()
        .capture(
            &target,
            &Arc::new(WorkControl::default()),
            &Arc::new(AtomicBool::new(false)),
            move |c, _, token| {
                Ok(Fake {
                    template: frames[c.index()].clone(),
                    next: 0,
                    count: 2,
                    gate: None,
                    gate_once: false,
                    token,
                    closed: Arc::new(AtomicUsize::new(0)),
                    fail_close: false,
                    fail_next: false,
                })
            },
            || Ok(()),
            |_| PreviewStatus::NotRequested,
        )
        .unwrap();
    assert_eq!(report["summary"]["processed_pairs"], 2);
    (f, target)
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn redigest(root: &Path, name: &str) {
    let mut m = read(&root.join("manifest.json"));
    let bytes = fs::read(root.join(name)).unwrap();
    let a = m["artifacts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["role"] == name)
        .unwrap();
    a["bytes"] = json!(bytes.len());
    a["crc32"] = json!(crc32fast::hash(&bytes));
    write(&root.join("manifest.json"), &m);
}
fn edit_pair(root: &Path, index: usize, edit: impl FnOnce(&mut Value)) {
    let mut lines = fs::read_to_string(root.join("pairs.ndjson"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str::<Value>(s).unwrap())
        .collect::<Vec<_>>();
    edit(&mut lines[index]);
    let mut bytes = Vec::new();
    for r in lines {
        serde_json::to_writer(&mut bytes, &r).unwrap();
        bytes.push(b'\n');
    }
    fs::write(root.join("pairs.ndjson"), bytes).unwrap();
    redigest(root, "pairs.ndjson");
}
#[test]
fn complete_legacy_and_segmented_captures_select_raw_bound_pairs_without_mutation() {
    for schema in [1, 2] {
        let (_f, root) = capture(schema);
        let originals = fs::read_dir(&root)
            .unwrap()
            .map(|e| {
                let f = e.unwrap();
                (f.file_name(), fs::read(f.path()).unwrap())
            })
            .collect::<Vec<_>>();
        for index in [0, 1] {
            let loaded =
                crate::candidate_bundle::load_at(&root, index, &WorkControl::default()).unwrap();
            assert_eq!(loaded.positions.len(), 4);
            assert_eq!(loaded.first_observations.len(), 4);
            assert_eq!(loaded.report["pair_count"], 2);
            assert_eq!(loaded.report["pair_index"], index);
            assert_eq!(loaded.report["source_frame_indices"], json!([index, index]));
            assert_eq!(loaded.report["numerical_recomputation_verified"], false);
            assert_eq!(loaded.report["physical_accuracy_qualified"], false);
            assert_eq!(loaded.report["units_verified"], false);
            assert_eq!(loaded.frames.each_ref().map(|f| f.interface), [0, 2]);
        }
        assert!(load_pair(&root, 2, &WorkControl::default()).is_err());
        for (name, bytes) in originals {
            assert_eq!(fs::read(root.join(name)).unwrap(), bytes);
        }
    }
}
#[test]
fn selected_record_reference_counters_and_qualification_changes_fail_after_redigest() {
    for kind in 0..6 {
        let (_f, root) = capture(1);
        edit_pair(&root, 1, |r| match kind {
            0 => r["frames"][0]["payload_crc32"] = json!(0),
            1 => r["raw_pts"] = json!(0),
            2 => r["result"]["Ok"]["physical_accuracy_qualified"] = json!(true),
            3 => r["result"]["Ok"]["queries"] = json!(4097),
            4 => {
                r["result"]["Ok"]["outcomes"][0]["outcome"]["Candidate"]["candidate_index"] =
                    json!(4096)
            }
            _ => r["frames"][0]["frame_index"] = json!(2),
        });
        assert!(
            load_pair(&root, 1, &WorkControl::default()).is_err(),
            "kind {kind}"
        );
    }
}
#[test]
fn missing_extra_symlink_and_duplicate_or_traversal_roles_fail_closed() {
    for kind in 0..5 {
        let (_f, root) = capture(1);
        if kind == 0 {
            fs::write(root.join("failure.json"), b"{}").unwrap();
        } else if kind == 1 {
            fs::rename(
                root.join("first.raptor"),
                root.parent().unwrap().join("saved.raptor"),
            )
            .unwrap();
            std::os::unix::fs::symlink("../saved.raptor", root.join("first.raptor")).unwrap();
        } else {
            let mut m = read(&root.join("manifest.json"));
            match kind {
                2 => {
                    m["artifacts"].as_array_mut().unwrap().pop();
                }
                3 => {
                    m["artifacts"][0]["role"] = json!("../geometry.json");
                }
                _ => {
                    m["artifacts"][0]["role"] = m["artifacts"][1]["role"].clone();
                }
            }
            write(&root.join("manifest.json"), &m);
        }
        assert!(
            load_pair(&root, 0, &WorkControl::default()).is_err(),
            "kind {kind}"
        );
    }
}
#[test]
fn corrupt_late_journal_or_raw_recording_and_false_terminal_report_are_rejected() {
    for kind in 0..4 {
        let (_f, root) = capture(1);
        if kind == 0 {
            let mut bytes = fs::read(root.join("pairs.ndjson")).unwrap();
            let i = bytes.len() - 2;
            bytes[i] = b' ';
            fs::write(root.join("pairs.ndjson"), bytes).unwrap();
        } else if kind == 1 {
            let mut bytes = fs::read(root.join("second.raptor")).unwrap();
            bytes.pop();
            fs::write(root.join("second.raptor"), bytes).unwrap();
            redigest(&root, "second.raptor");
        } else {
            let mut r = read(&root.join("report.json"));
            if kind == 2 {
                r["summary"]["processed_pairs"] = json!(1);
            } else {
                r["geometry_exported"] = json!(true);
            }
            write(&root.join("report.json"), &r);
            redigest(&root, "report.json");
        }
        assert!(
            load_pair(&root, 0, &WorkControl::default()).is_err(),
            "kind {kind}"
        );
    }
}
#[test]
fn cancellation_and_final_path_mutation_return_no_view() {
    let (_f, root) = capture(2);
    let work = WorkControl::default();
    work.cancel();
    assert!(load_pair(&root, 0, &work).is_err());
    let late = WorkControl::default();
    assert!(
        load_inner(&root, 0, &late, &|| {
            late.cancel();
            Ok(())
        })
        .is_err()
    );
    assert!(
        load_inner(&root, 0, &WorkControl::default(), &|| {
            fs::write(root.join("geometry.json"), b"changed")?;
            Ok(())
        })
        .unwrap_err()
        .to_string()
        .contains("changed")
    );
}
#[test]
fn stored_processing_failure_is_visible_with_images_and_empty_geometry() {
    let (_f, root) = capture(1);
    edit_pair(&root, 1, |r| {
        r["result"] = json!({"Err":"expected processing failure"})
    });
    let mut report = read(&root.join("report.json"));
    report["summary"]["failed_processing_pairs"] = json!(1);
    write(&root.join("report.json"), &report);
    redigest(&root, "report.json");
    let loaded = load_pair(&root, 1, &WorkControl::default()).unwrap();
    assert!(loaded.positions.is_empty());
    assert_eq!(
        loaded.report["stored_pair_error"],
        "expected processing failure"
    );
    assert_eq!(loaded.frames[0].source_size, [10, 5]);
}
#[test]
fn selected_candidate_and_outcome_arrays_are_bounded_during_parsing() {
    for field in ["candidates", "outcomes"] {
        let (_f, root) = capture(1);
        // Admit the enlarged byte fixture so the typed item bound is exercised.
        let mut request = read(&root.join("stream-request.json"));
        request["maximum_pair_log_bytes"] = json!(8 * 1024 * 1024);
        write(&root.join("stream-request.json"), &request);
        redigest(&root, "stream-request.json");
        edit_pair(&root, 1, |r| {
            let item = r["result"]["Ok"][field][0].clone();
            r["result"]["Ok"][field] = json!(vec![item; 4097]);
        });
        assert!(
            load_pair(&root, 1, &WorkControl::default())
                .unwrap_err()
                .to_string()
                .contains("item limit")
        );
    }
}

#[test]
fn older_legacy_restoration_metadata_remains_unknown_without_weakening_schema_two() {
    for schema in [1, 2] {
        let (_f, root) = capture(schema);
        let mut report = read(&root.join("report.json"));
        report
            .as_object_mut()
            .unwrap()
            .remove("reported_source_restoration");
        write(&root.join("report.json"), &report);
        redigest(&root, "report.json");
        if schema == 1 {
            let loaded = load_pair(&root, 0, &WorkControl::default()).unwrap();
            assert_eq!(loaded.report["source_restoration_reported"], false);
        } else {
            assert!(load_pair(&root, 0, &WorkControl::default()).is_err());
        }
        report["reported_source_restoration"] = json!("FAIL");
        write(&root.join("report.json"), &report);
        redigest(&root, "report.json");
        assert!(load_pair(&root, 0, &WorkControl::default()).is_err());
    }
}
