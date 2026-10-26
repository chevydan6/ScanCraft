use scancraft_core::{Frame, StreamProfile};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::{CaptureContext, Reader, Writer};
use scancraft_workflow::{
    FileInputs,
    recording_reconstruction::{self, Inputs},
};
use std::{
    fs,
    path::{Path, PathBuf},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(
            scancraft_workflow::output::staging(&std::env::temp_dir().join("recorded-stereo-test"))
                .unwrap(),
        )
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn original() -> FileInputs {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
    FileInputs {
        calibration: root.join("calibration.rapcal"),
        left: root.join("left.png"),
        right: root.join("right.png"),
        settings: root.join("request.json"),
    }
}
fn write_recording(path: &Path, interface: u8, image_path: &Path, context: CaptureContext) {
    let image = image::open(image_path).unwrap().to_luma8();
    let mut writer = Writer::with_context(
        Vec::new(),
        StreamProfile {
            interface,
            width: image.width(),
            height: image.height(),
            fourcc: *b"GREY",
            fps: 30,
        },
        context,
    )
    .unwrap();
    for i in 0..3u32 {
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&(1000 + i * 33333).to_le_bytes());
        writer
            .push(&Frame {
                sequence: 10 + i,
                host_timestamp_us: 1000 + u64::from(i) * 33333,
                flags: 0,
                metadata,
                data: image.as_raw().clone(),
            })
            .unwrap();
    }
    fs::write(path, writer.finish().unwrap()).unwrap();
}
fn synthetic() -> CaptureContext {
    CaptureContext {
        controls: serde_json::json!({"synthetic":true}),
        calibration_status: "synthetic_fixture_not_a_scanner_capture".into(),
        ..Default::default()
    }
}
fn fixture(root: &Path) -> Inputs {
    let old = original();
    let left = root.join("left.raptor");
    let right = root.join("right.raptor");
    write_recording(&left, 0, &old.left, synthetic());
    write_recording(&right, 2, &old.right, synthetic());
    Inputs {
        calibration: old.calibration,
        left,
        left_index: 1,
        right,
        right_index: 2,
        settings: old.settings,
    }
}
#[test]
fn same_pixels_produce_same_geometry_and_publish_exact_selected_raw_frames() {
    let t = Temp::new();
    let inputs = fixture(&t.0);
    let work = WorkControl::default();
    let png = scancraft_workflow::prepare(&original(), &work).unwrap();
    let recorded = recording_reconstruction::prepare(&inputs, &work).unwrap();
    assert_eq!(
        serde_json::to_value(png.report()).unwrap(),
        serde_json::to_value(recorded.report()).unwrap()
    );
    let target = t.0.join("result");
    let summary = scancraft_workflow::publish_prepared(&recorded, &target, &work).unwrap();
    assert_eq!(summary.points, 3);
    let project_bytes = fs::read(target.join("project.orproj")).unwrap();
    assert_eq!(&project_bytes[..8], b"ORPROJ05");
    let mut saved = scancraft_geometry::formats::load_project(project_bytes.as_slice()).unwrap();
    let live = recorded.create_editor(&target).unwrap();
    assert_eq!(saved.origin(), live.origin());
    let origin = saved.origin().unwrap().clone();
    for reference in &origin.artifacts {
        let bytes = fs::read(target.join(&reference.name)).unwrap();
        assert_eq!(bytes.len() as u64, reference.bytes);
        assert_eq!(crc32fast::hash(&bytes), reference.crc32);
    }
    let original = saved.cloud.clone();
    saved.trim(&[0], true).unwrap();
    let mut edited = Vec::new();
    scancraft_geometry::formats::save_project(&saved, &mut edited).unwrap();
    let mut reopened = scancraft_geometry::formats::load_project(edited.as_slice()).unwrap();
    assert_eq!(reopened.origin(), Some(&origin));
    assert!(reopened.undo());
    assert_eq!(reopened.cloud, original);
    assert!(reopened.redo());
    assert_eq!(reopened.origin(), Some(&origin));

    assert_eq!(
        fs::read(target.join("calibration.rapcal")).unwrap(),
        fs::read(&inputs.calibration).unwrap()
    );
    assert_eq!(
        fs::read(target.join("request.json")).unwrap(),
        fs::read(&inputs.settings).unwrap()
    );
    assert!(!summary.synchronization_verified);
    assert!(!summary.physical_accuracy_qualified);
    let sources: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join("recording-sources.json")).unwrap()).unwrap();
    assert_eq!(sources["left"]["selected_frame_index"], 1);
    assert_eq!(sources["right"]["selected_frame_index"], 2);
    assert_eq!(sources["left"]["source_total_frames"], 3);
    for (name, source, index) in [
        ("left-frame.raptor", &inputs.left, 1),
        ("right-frame.raptor", &inputs.right, 2),
    ] {
        let selected = scancraft_recording::selection::read_selected(
            Reader::new(fs::File::open(source).unwrap()).unwrap(),
            index,
        )
        .unwrap();
        let mut output = Reader::new(fs::File::open(target.join(name)).unwrap()).unwrap();
        let frame = output.read_frame().unwrap().unwrap();
        assert_eq!(frame.data, selected.frame.data);
        assert_eq!(frame.metadata, selected.frame.metadata);
        assert_eq!(frame.sequence, selected.frame.sequence);
        assert_eq!(frame.host_timestamp_us, selected.frame.host_timestamp_us);
        assert_eq!(
            serde_json::to_value(output.context.as_ref()).unwrap(),
            serde_json::to_value(selected.context).unwrap()
        );
        assert!(output.read_frame().unwrap().is_none());
    }
    assert!(scancraft_workflow::publish_prepared(&recorded, &target, &work).is_err());
}
#[test]
fn corrupt_later_frame_wrong_interface_and_out_of_range_never_prepare_geometry() {
    let t = Temp::new();
    let mut inputs = fixture(&t.0);
    let work = WorkControl::default();
    let mut bytes = fs::read(&inputs.left).unwrap();
    bytes.pop();
    fs::write(&inputs.left, bytes).unwrap();
    inputs.left_index = 0;
    assert!(recording_reconstruction::prepare(&inputs, &work).is_err());
    inputs = fixture(&t.0);
    inputs.left_index = 3;
    assert!(recording_reconstruction::prepare(&inputs, &work).is_err());
    inputs = fixture(&t.0);
    std::mem::swap(&mut inputs.left, &mut inputs.right);
    assert!(
        recording_reconstruction::prepare(&inputs, &work)
            .err()
            .unwrap()
            .to_string()
            .contains("interface")
    );
    inputs = fixture(&t.0);
    work.cancel();
    assert!(recording_reconstruction::prepare(&inputs, &work).is_err());
    assert!(!t.0.join("result").exists());
}
#[test]
fn unknown_real_context_cannot_be_silently_processed_as_synthetic() {
    let t = Temp::new();
    let inputs = fixture(&t.0);
    write_recording(&inputs.left, 0, &original().left, CaptureContext::default());
    assert!(
        recording_reconstruction::prepare(&inputs, &WorkControl::default())
            .err()
            .unwrap()
            .to_string()
            .contains("explicitly synthetic")
    );
}
#[test]
fn embedded_calibration_mismatch_is_rejected() {
    let t = Temp::new();
    let inputs = fixture(&t.0);
    let mut calibration =
        scancraft_calibration::storage::read(fs::File::open(&inputs.calibration).unwrap()).unwrap();
    calibration.notes.push_str(" changed");
    let mut bytes = Vec::new();
    scancraft_calibration::storage::write(&calibration, &mut bytes).unwrap();
    let mut context = synthetic();
    context.calibration_bytes = Some(bytes);
    write_recording(&inputs.left, 0, &original().left, context);
    assert!(
        recording_reconstruction::prepare(&inputs, &WorkControl::default())
            .err()
            .unwrap()
            .to_string()
            .contains("embedded recording calibration")
    );
}
#[test]
fn physical_declaration_checks_calibration_binding_and_known_recording_identity() {
    let t = Temp::new();
    let mut inputs = fixture(&t.0);
    let mut calibration =
        scancraft_calibration::storage::read(fs::File::open(&inputs.calibration).unwrap()).unwrap();
    let binding = scancraft_calibration::storage::DeviceBinding {
        serial: "test-device".into(),
        firmware: "test-firmware".into(),
        mode: "test-mode".into(),
        left_size: [64, 32],
        right_size: [64, 32],
    };
    calibration.provenance = scancraft_calibration::storage::Provenance::Imported;
    calibration.binding = Some(binding.clone());
    inputs.calibration = t.0.join("declared.rapcal");
    scancraft_calibration::storage::write(
        &calibration,
        fs::File::create(&inputs.calibration).unwrap(),
    )
    .unwrap();
    let mut request: serde_json::Value =
        serde_json::from_slice(&fs::read(&inputs.settings).unwrap()).unwrap();
    request["source"] =
        serde_json::to_value(scancraft_depth::MatchSource::Observations(binding)).unwrap();
    inputs.settings = t.0.join("declared.json");
    fs::write(&inputs.settings, serde_json::to_vec(&request).unwrap()).unwrap();
    // These are synthetic numerical pixels with test-only declarations, not hardware qualification.
    assert!(
        recording_reconstruction::prepare(&inputs, &WorkControl::default())
            .err()
            .unwrap()
            .to_string()
            .contains("synthetic recording")
    );
    let context = CaptureContext {
        device_identity: Some("other-device".into()),
        ..Default::default()
    };
    write_recording(&inputs.left, 0, &original().left, context);
    write_recording(
        &inputs.right,
        2,
        &original().right,
        CaptureContext::default(),
    );
    assert!(
        recording_reconstruction::prepare(&inputs, &WorkControl::default())
            .err()
            .unwrap()
            .to_string()
            .contains("device identity")
    );
    write_recording(
        &inputs.left,
        0,
        &original().left,
        CaptureContext {
            controls: serde_json::json!({"mode":"different-mode"}),
            ..Default::default()
        },
    );
    assert!(
        recording_reconstruction::prepare(&inputs, &WorkControl::default())
            .err()
            .unwrap()
            .to_string()
            .contains("recording mode")
    );
    write_recording(&inputs.left, 0, &original().left, CaptureContext::default());
    let prepared = recording_reconstruction::prepare(&inputs, &WorkControl::default()).unwrap();
    assert!(!prepared.report().physical_accuracy_qualified);
    assert!(!prepared.report().synchronization_verified);
    let bytes = fs::read(&inputs.left).unwrap();
    let header_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let header: serde_json::Value = serde_json::from_slice(&bytes[16..16 + header_len]).unwrap();
    let profile = serde_json::to_vec(&header["profile"]).unwrap();
    let mut legacy = b"RAPTOR01".to_vec();
    legacy.extend_from_slice(&(profile.len() as u32).to_le_bytes());
    legacy.extend_from_slice(&profile);
    legacy.extend_from_slice(&bytes[16 + header_len..]);
    fs::write(&inputs.left, legacy).unwrap();
    let prepared = recording_reconstruction::prepare(&inputs, &WorkControl::default()).unwrap();
    let output = t.0.join("legacy-result");
    scancraft_workflow::publish_prepared(&prepared, &output, &WorkControl::default()).unwrap();
    let lineage: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("recording-sources.json")).unwrap()).unwrap();
    assert_eq!(lineage["left"]["source_version"], 1);
    assert_eq!(lineage["left"]["source_header_crc_verified"], false);
    assert!(lineage["left"]["context"].is_null());
}

#[test]
fn cancellation_after_recorded_preparation_never_publishes_bundle() {
    let t = Temp::new();
    let inputs = fixture(&t.0);
    let work = WorkControl::default();
    let prepared = recording_reconstruction::prepare(&inputs, &work).unwrap();
    work.cancel();
    let output = t.0.join("cancelled");
    assert!(scancraft_workflow::publish_prepared(&prepared, &output, &work).is_err());
    assert!(!output.exists());
}

#[test]
fn cached_recording_processor_reuses_pairs_without_weakening_source_guards() {
    let t = Temp::new();
    let mut inputs = fixture(&t.0);
    let work = WorkControl::default();
    let mut processor = recording_reconstruction::RecordingProcessor::open(&inputs, &work).unwrap();
    for (left, right) in [(0, 0), (2, 1), (1, 2)] {
        let cached = processor.prepare_pair(left, right, &work).unwrap();
        inputs.left_index = left;
        inputs.right_index = right;
        let independent = recording_reconstruction::prepare(&inputs, &work).unwrap();
        assert_eq!(
            serde_json::to_value(cached.report()).unwrap(),
            serde_json::to_value(independent.report()).unwrap()
        );
    }
    assert!(processor.prepare_pair(3, 0, &work).is_err());
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&inputs.left)
        .unwrap();
    std::io::Write::write_all(&mut file, b"changed").unwrap();
    assert!(processor.prepare_pair(0, 0, &work).is_err());
    assert!(processor.check_sources(&work).is_err());
}
fn sequence_settings(path: &Path) {
    fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version":1,"session_id":"synthetic-recorded-sequence",
        "tracking":{"max_correspondence_mm":2.,"max_rms_mm":0.01,"minimum_overlap":0.8,
            "max_step_translation_mm":1.,"max_step_rotation_rad":0.1,"iterations":20,"tolerance_mm":1e-8},
        "voxel_mm":0.1,"maximum_voxels":1000,
        "pairs":[{"id":1,"left_index":0,"right_index":0,"tracking_indices":[0,1,2]},
            {"id":2,"left_index":1,"right_index":1,"tracking_indices":[0,1,2]},
            {"id":3,"left_index":2,"right_index":2,"tracking_indices":[0,1,2]}]
    })).unwrap()).unwrap();
}
#[test]
fn recorded_sequence_retains_raw_pairs_fuses_and_reopens_without_overwriting_inputs() {
    let t = Temp::new();
    let inputs = fixture(&t.0);
    let settings = t.0.join("sequence.json");
    sequence_settings(&settings);
    let originals: Vec<_> = [
        &inputs.left,
        &inputs.right,
        &inputs.calibration,
        &inputs.settings,
        &settings,
    ]
    .into_iter()
    .map(|p| (p.clone(), fs::read(p).unwrap()))
    .collect();
    let target = t.0.join("sequence");
    let summary = scancraft_workflow::recorded_sequence::run(
        &inputs,
        &settings,
        &target,
        &WorkControl::default(),
    )
    .unwrap();
    assert_eq!(summary["integrated_frames"], 3);
    assert_eq!(summary["rejected_frames"], 0);
    assert!(summary["points"].as_u64().unwrap() >= 3);
    assert_eq!(summary["physical_accuracy_qualified"], false);
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["synchronization_verified"], false);
    assert_eq!(report["source_device_binding_verified"], false);
    assert_eq!(
        report["reconstruction_context"]["source"],
        "synthetic_fixture"
    );
    for index in 0..3 {
        let dir = target.join(format!("frame-{index:04}"));
        let source: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join("recording-sources.json")).unwrap()).unwrap();
        assert_eq!(source["left"]["selected_frame_index"], index);
        let mut reader =
            Reader::new(fs::File::open(dir.join("left-frame.raptor")).unwrap()).unwrap();
        assert_eq!(
            reader.read_frame().unwrap().unwrap().sequence,
            10 + index as u32
        );
        assert!(reader.read_frame().unwrap().is_none());
        assert!(dir.join("report.json").is_file());
    }
    let editor = scancraft_geometry::formats::load_project(
        fs::File::open(target.join("project.orproj")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        editor.cloud.points.len() as u64,
        summary["points"].as_u64().unwrap()
    );
    // Analytic stationary plane coordinates in the original left frame. Tiny
    // fitted-pose perturbations can straddle voxel boundaries, so validate
    // geometry/coverage instead of assuming exactly three occupied cells.
    let expected = [[-57.5, -27.5, 400.], [22.5, 22.5, 400.], [92.5, 42.5, 400.]];
    let close = |a: [f64; 3], b: [f64; 3]| {
        a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f64>() < 1e-14
    };
    assert!(
        editor
            .cloud
            .points
            .iter()
            .all(|p| expected.iter().any(|&q| close(p.position, q)))
    );
    assert!(
        expected
            .iter()
            .all(|&p| editor.cloud.points.iter().any(|q| close(p, q.position)))
    );
    let original_project = fs::read(target.join("project.orproj")).unwrap();
    assert!(
        scancraft_workflow::recorded_sequence::run(
            &inputs,
            &settings,
            &target,
            &WorkControl::default()
        )
        .is_err()
    );
    assert_eq!(
        fs::read(target.join("project.orproj")).unwrap(),
        original_project
    );
    for (path, bytes) in originals {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}
#[test]
fn recorded_sequence_errors_and_cancellation_never_publish_partial_success() {
    let t = Temp::new();
    let inputs = fixture(&t.0);
    let settings = t.0.join("sequence.json");
    sequence_settings(&settings);
    let target = t.0.join("never-published");
    let work = WorkControl::default();
    work.cancel();
    assert!(
        scancraft_workflow::recorded_sequence::run(&inputs, &settings, &target, &work).is_err()
    );
    assert!(!target.exists());
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
    value["pairs"][2]["left_index"] = 99.into();
    fs::write(&settings, serde_json::to_vec(&value).unwrap()).unwrap();
    let error = scancraft_workflow::recorded_sequence::run(
        &inputs,
        &settings,
        &target,
        &WorkControl::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("incomplete recorded sequence retained"),
        "{error}"
    );
    assert!(!target.exists());
    assert!(
        t.0.read_dir()
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.file_type().unwrap().is_dir()
                && entry.path().join("frame-0000/left-frame.raptor").is_file())
    );
    value["pairs"][2]["left_index"] = 2.into();
    value["pairs"][1]["id"] = 1.into();
    fs::write(&settings, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        scancraft_workflow::recorded_sequence::run(
            &inputs,
            &settings,
            &target,
            &WorkControl::default()
        )
        .is_err()
    );
    assert!(!target.exists());
}

#[test]
fn recorded_sequence_rejects_bad_alignment_and_recovers_without_integrating_it() {
    let t = Temp::new();
    let inputs = fixture(&t.0);
    let original = original();
    let left = image::open(&original.left).unwrap().to_luma8();
    let right = image::open(&original.right).unwrap().to_luma8();
    let mut shifted = vec![0; right.as_raw().len()];
    for y in 0..32usize {
        for x in 0..56usize {
            shifted[y * 64 + x] = 2 * left.as_raw()[y * 64 + x + 8] + 20;
        }
    }
    // The bundled synthetic right sensor is rotated 180 degrees.
    shifted.reverse();
    let mut writer = Writer::with_context(
        Vec::new(),
        StreamProfile {
            interface: 2,
            width: 64,
            height: 32,
            fourcc: *b"GREY",
            fps: 30,
        },
        synthetic(),
    )
    .unwrap();
    for i in 0..3u32 {
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&(1000 + i * 33333).to_le_bytes());
        writer
            .push(&Frame {
                sequence: 10 + i,
                host_timestamp_us: 1000 + u64::from(i) * 33333,
                flags: 0,
                metadata,
                data: if i == 1 {
                    shifted.clone()
                } else {
                    right.as_raw().clone()
                },
            })
            .unwrap();
    }
    fs::write(&inputs.right, writer.finish().unwrap()).unwrap();
    let settings = t.0.join("sequence.json");
    sequence_settings(&settings);
    let mut request: serde_json::Value =
        serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
    for i in 0..3 {
        request["pairs"][i]["right_index"] = i.into();
    }
    fs::write(&settings, serde_json::to_vec(&request).unwrap()).unwrap();
    let target = t.0.join("sequence");
    let summary = scancraft_workflow::recorded_sequence::run(
        &inputs,
        &settings,
        &target,
        &WorkControl::default(),
    )
    .unwrap();
    assert_eq!(summary["integrated_frames"], 2);
    assert_eq!(summary["rejected_frames"], 1);
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["frames"][1]["tracking_state"], "Lost");
    assert_eq!(report["frames"][1]["integrated"], false);
    assert!(report["frames"][1]["world_from_frame"].is_null());
    assert_eq!(report["frames"][2]["tracking_state"], "Tracking");
    assert_eq!(report["frames"][2]["integrated"], true);
    let cloud = scancraft_geometry::formats::read_ply(
        std::io::BufReader::new(fs::File::open(target.join("points.ply")).unwrap()),
        None,
    )
    .unwrap();
    assert!(
        cloud
            .points
            .iter()
            .all(|p| (p.position[2] - 400.).abs() < 1e-7)
    );
}

#[test]
fn recorded_sequence_no_geometry_keeps_current_raw_inputs_and_rejection_diagnostics() {
    let t = Temp::new();
    let mut inputs = fixture(&t.0);
    let mut request: serde_json::Value =
        serde_json::from_slice(&fs::read(&inputs.settings).unwrap()).unwrap();
    request["seeds"] = serde_json::json!([[0, 0]]);
    inputs.settings = t.0.join("empty-request.json");
    fs::write(&inputs.settings, serde_json::to_vec(&request).unwrap()).unwrap();
    let settings = t.0.join("sequence.json");
    sequence_settings(&settings);
    let target = t.0.join("no-geometry");
    let error = scancraft_workflow::recorded_sequence::run(
        &inputs,
        &settings,
        &target,
        &WorkControl::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("no geometry"), "{error}");
    assert!(!target.exists());
    let partial =
        t.0.read_dir()
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| p.join("frame-0000/report.json").is_file())
            .unwrap();
    let frame = partial.join("frame-0000");
    assert!(frame.join("left-frame.raptor").is_file());
    assert!(frame.join("right-frame.raptor").is_file());
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(frame.join("report.json")).unwrap()).unwrap();
    assert!(report["reconstruction"].is_null());
    assert_eq!(report["matching"]["rejected"].as_array().unwrap().len(), 1);
    assert!(!frame.join("points.ply").exists());
}

#[test]
fn recorded_sequence_deferred_publication_rechecks_cancellation_and_sources() {
    let t = Temp::new();
    let inputs = fixture(&t.0);
    let settings = t.0.join("sequence.json");
    sequence_settings(&settings);
    let target = t.0.join("deferred");
    let prepared = scancraft_workflow::recorded_sequence::prepare(
        &inputs,
        &settings,
        &target,
        &WorkControl::default(),
    )
    .unwrap();
    assert!(!target.exists());
    assert!(!prepared.cloud().points.is_empty());
    let partial = prepared.staging_directory().to_path_buf();
    assert!(partial.join("project.orproj").exists());
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(
        scancraft_workflow::recorded_sequence::publish_prepared(&prepared, &cancelled).is_err()
    );
    assert!(!target.exists());
    assert!(partial.exists());
    std::io::Write::write_all(
        &mut fs::OpenOptions::new()
            .append(true)
            .open(&inputs.left)
            .unwrap(),
        b"changed",
    )
    .unwrap();
    assert!(
        scancraft_workflow::recorded_sequence::publish_prepared(&prepared, &WorkControl::default())
            .is_err()
    );
    assert!(!target.exists());
    assert!(partial.exists());
}

#[test]
fn sequence_project_references_survive_edit_reopen_and_explicitly_verify_moved_sources() {
    let t = Temp::new();
    let inputs = fixture(&t.0);
    let settings = t.0.join("sequence.json");
    sequence_settings(&settings);
    let target = t.0.join("bundle");
    scancraft_workflow::recorded_sequence::run(
        &inputs,
        &settings,
        &target,
        &WorkControl::default(),
    )
    .unwrap();
    let mut editor = scancraft_geometry::formats::load_project(
        fs::File::open(target.join("project.orproj")).unwrap(),
    )
    .unwrap();
    let references = editor.sequence().unwrap().clone();
    assert_eq!(references.frames.len(), 3);
    assert_eq!(references.artifacts.len(), 27);
    assert_eq!(references.source_kind, "synthetic_fixture");
    assert_eq!(references.frames.iter().filter(|f| f.integrated).count(), 3);
    editor.trim(&[0], false).unwrap();
    assert!(editor.undo());
    assert!(editor.redo());
    let saved = t.0.join("edited.orproj");
    scancraft_geometry::formats::save_project(&editor, fs::File::create(&saved).unwrap()).unwrap();
    let moved = t.0.join("moved");
    fs::rename(&target, &moved).unwrap();
    let reopened =
        scancraft_geometry::formats::load_project(fs::File::open(&saved).unwrap()).unwrap();
    assert_eq!(reopened.sequence(), Some(&references));
    let checked = scancraft_workflow::source_verification::verify_project(
        &saved,
        &moved,
        &WorkControl::default(),
    )
    .unwrap();
    assert!(checked.passed);
    assert_eq!(checked.artifacts.len(), 27);
    assert!(!checked.synchronization_verified && !checked.physical_accuracy_qualified);
    let changed = moved.join("frame-0001/left-frame.raptor");
    std::io::Write::write_all(
        &mut fs::OpenOptions::new().append(true).open(changed).unwrap(),
        b"changed",
    )
    .unwrap();
    assert!(
        !scancraft_workflow::source_verification::verify_sequence(
            &references,
            &moved,
            &WorkControl::default()
        )
        .unwrap()
        .passed
    );
    let parent = moved.join("frame-0000");
    let elsewhere = t.0.join("elsewhere");
    fs::rename(&parent, &elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &parent).unwrap();
    assert!(
        scancraft_workflow::source_verification::verify_sequence(
            &references,
            &moved,
            &WorkControl::default()
        )
        .is_err()
    );
}
