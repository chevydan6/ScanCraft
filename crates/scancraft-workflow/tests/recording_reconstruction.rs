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
