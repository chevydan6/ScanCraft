//! Device-free capacity/compatibility tests. Pacing follows consumed previews, not real hardware speed.
use scancraft_core::Frame;
use scancraft_geometry::{
    formats::{load_project, save_project},
    work::WorkControl,
};
use scancraft_recording::CaptureContext;
use scancraft_workflow::live_stereo::{
    InputMode, Processor, Side,
    control::Control,
    pair_journal,
    runner::{self, Source},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(
            scancraft_workflow::output::staging(&std::env::temp_dir().join("live-capacity"))
                .unwrap(),
        )
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset")
}
fn settings(frames: usize) -> Value {
    json!({"schema_version":2,"session_id":"synthetic-extended-live","fps":30,"maximum_frames_per_camera":frames,
        "maximum_host_delta_us":1000,"tracking_samples":32,"voxel_mm":0.1,"maximum_voxels":1000,
        "tracking":{"max_correspondence_mm":10.0,"max_rms_mm":0.01,"minimum_overlap":0.8,"max_step_translation_mm":1.0,"max_step_rotation_rad":0.1,"iterations":40,"tolerance_mm":1e-8},
        "storage":{"maximum_recording_bytes_per_camera":1024*1024 + frames as u64 * (2048+4096+32) + 32,
            "pair_journal":{"segment_bytes":1024*1024,"maximum_record_bytes":32768,"maximum_total_bytes":128*1024*1024},
            "maximum_bundle_bytes":2*(1024*1024 + frames as u64 * (2048+4096+32) + 32)+640*1024*1024}})
}
fn open(root: &Path, settings: &Value) -> Result<Processor, scancraft_workflow::Error> {
    let path = root.join("settings.json");
    fs::write(&path, serde_json::to_vec(settings).unwrap()).unwrap();
    Processor::open(
        &fixture().join("calibration.rapcal"),
        &fixture().join("request.json"),
        &path,
        InputMode::SyntheticFixture,
        &WorkControl::default(),
    )
}
struct Paced {
    side: Side,
    next: usize,
    maximum: usize,
    pixels: Vec<u8>,
    acknowledged: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
    control: Arc<Control>,
}
impl Source for Paced {
    fn context(&self) -> Result<CaptureContext, String> {
        Ok(CaptureContext {
            controls: json!({"synthetic":true}),
            calibration_status: "synthetic_fixture".into(),
            ..Default::default()
        })
    }
    fn next(&mut self) -> Result<Option<Frame>, String> {
        self.next_until(&|| false)
    }
    fn next_until(&mut self, cancelled: &dyn Fn() -> bool) -> Result<Option<Frame>, String> {
        if self.next == self.maximum {
            return Ok(None);
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.acknowledged.load(Ordering::Acquire) < self.next {
            if cancelled() {
                return Ok(None);
            }
            if let Some(preview) = self.control.take_preview() {
                self.acknowledged
                    .fetch_max(preview.pair_id + 1, Ordering::AcqRel);
            }
            if Instant::now() > deadline {
                return Err("synthetic pacing timed out".into());
            }
            std::thread::sleep(Duration::from_micros(100));
        }
        let index = self.next;
        self.next += 1;
        let pts = 1000 + index as u32 * 33333;
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&pts.to_le_bytes());
        Ok(Some(Frame {
            sequence: index as u32,
            host_timestamp_us: u64::from(pts) + self.side.index() as u64 * 10,
            flags: 0,
            metadata,
            data: self.pixels.clone(),
        }))
    }
    fn statistics(&self) -> Value {
        json!({"synthetic":true})
    }
    fn close(self) -> Result<(), String> {
        self.closed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
fn capture(processor: &mut Processor, target: &Path, count: usize) -> Result<Value, String> {
    let control = Arc::new(Control::new(true));
    let commands = control.clone();
    let acknowledged = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicUsize::new(0));
    let owners = closed.clone();
    runner::bundle_controlled(
        processor,
        target,
        &WorkControl::default(),
        move |side, _| {
            Ok(Paced {
                side,
                next: 0,
                maximum: count,
                pixels: image::open(fixture().join(if side.index() == 0 {
                    "left.png"
                } else {
                    "right.png"
                }))
                .unwrap()
                .to_luma8()
                .into_raw(),
                acknowledged: acknowledged.clone(),
                closed: owners.clone(),
                control: commands.clone(),
            })
        },
        || {
            assert_eq!(closed.load(Ordering::SeqCst), 2);
            assert!(!target.exists());
            Ok(())
        },
        &control,
    )
}
#[test]
fn extended_capture_exceeds_legacy_limit_and_preserves_edit_reopen_export_and_journal() {
    let t = Temp::new();
    let work = WorkControl::default();
    let count = 1001;
    let mut processor = open(&t.0, &settings(count)).unwrap();
    let target = t.0.join("scan");
    let report = capture(&mut processor, &target, count).unwrap();
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["pair_count"], count);
    assert!(report.get("pairs").is_none());
    let fused = report["fused_points"].as_u64().unwrap() as usize;
    assert!(fused >= 3);
    assert!(report["pair_journal"]["segments"].as_u64().unwrap() > 1);
    for source in report["sources"].as_array().unwrap() {
        assert_eq!(source["recorded_frames"], count);
        assert_eq!(source["processing_queue_drops"], 0);
    }
    let bytes = fs::read(target.join("project.orproj")).unwrap();
    assert_eq!(&bytes[..8], b"ORPROJ09");
    let mut editor = load_project(bytes.as_slice()).unwrap();
    assert_eq!(editor.live_session().unwrap().pairs.len(), count);
    let lineage = editor.live_session().unwrap().clone();
    editor.trim(&[0], false).unwrap();
    assert!(editor.undo());
    assert!(editor.redo());
    let mut encoded = vec![];
    save_project(&editor, &mut encoded).unwrap();
    let reloaded = load_project(encoded.as_slice()).unwrap();
    assert_eq!(reloaded.live_session(), Some(&lineage));
    assert_eq!(reloaded.cloud.points.len(), fused - 1);
    assert!(
        scancraft_workflow::source_verification::verify_live(&lineage, &target, &work)
            .unwrap()
            .passed
    );
    let copied = t.0.join("copy");
    scancraft_workflow::project_bundle::export(&reloaded, &target, &copied, &work).unwrap();
    for root in [&target, &copied] {
        let mut raw = ["left.raptor", "right.raptor"].map(|name| {
            scancraft_recording::Reader::new(std::io::BufReader::new(
                fs::File::open(root.join(name)).unwrap(),
            ))
            .unwrap()
        });
        let mut seen = 0;
        let checked = pair_journal::visit(
            root,
            pair_journal::Mode::Complete,
            &work,
            |index, line, _| {
                let record: Value = serde_json::from_slice(line).unwrap();
                assert_eq!(index, seen);
                assert_eq!(record["result"]["pair_id"], index);
                for (side, reader) in raw.iter_mut().enumerate() {
                    let frame = reader.read_frame().unwrap().unwrap();
                    let reference = &record["frames"][side];
                    assert_eq!(reference["frame_index"], index);
                    assert_eq!(reference["sequence"], frame.sequence);
                    assert_eq!(reference["host_timestamp_us"], frame.host_timestamp_us);
                    assert_eq!(reference["payload_crc32"], crc32fast::hash(&frame.data));
                    assert_eq!(
                        reference["metadata_crc32"],
                        crc32fast::hash(&frame.metadata)
                    );
                    assert_eq!(
                        record["raw_pts"],
                        scancraft_core::uvc_pts(&frame.metadata).unwrap().unwrap()
                    );
                }

                seen += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(checked.pairs, count as u64);
        assert!(checked.journal_complete);
        for reader in &mut raw {
            assert!(reader.read_frame().unwrap().is_none());
        }
    }
    // Model interruption before the last segment was indexed, leaving a torn record.
    let index_path = copied.join("pairs.index.ndjson");
    let index = fs::read(&index_path).unwrap();
    let lines: Vec<_> = index.split_inclusive(|b| *b == b'\n').collect();
    let last: Value = serde_json::from_slice(lines[lines.len() - 2]).unwrap();
    let last_segment = copied.join(format!(
        "pairs-{:06}.ndjson",
        last["number"].as_u64().unwrap()
    ));
    fs::write(&index_path, lines[..lines.len() - 2].concat()).unwrap();
    let mut tail = fs::read(&last_segment).unwrap();
    tail.extend_from_slice(b"{xx");
    fs::write(&last_segment, &tail).unwrap();
    let index_before = fs::read(&index_path).unwrap();
    assert!(
        pair_journal::visit(&copied, pair_journal::Mode::Complete, &work, |_, _, _| Ok(
            ()
        ))
        .is_err()
    );
    let recovery = pair_journal::visit(&copied, pair_journal::Mode::Recovery, &work, |_, _, _| {
        Ok(())
    })
    .unwrap();
    assert_eq!(recovery.pairs, count as u64);
    assert_eq!(recovery.trailing_incomplete_bytes, 3);
    assert!(!recovery.journal_complete);
    assert_eq!(fs::read(&index_path).unwrap(), index_before);
    assert_eq!(fs::read(&last_segment).unwrap(), tail);
    let segment = target.join("pairs-000000.ndjson");
    let mut changed = fs::read(&segment).unwrap();
    changed[20] ^= 1;
    fs::write(&segment, changed).unwrap();
    assert!(
        !scancraft_workflow::source_verification::verify_live(&lineage, &target, &work)
            .unwrap()
            .passed
    );
    assert!(
        pair_journal::visit(&target, pair_journal::Mode::Complete, &work, |_, _, _| Ok(
            ()
        ))
        .is_err()
    );
}
#[test]
fn capacity_and_storage_admission_reject_incomplete_or_inconsistent_policies() {
    let t = Temp::new();
    let maximum = settings(100_000);
    open(&t.0, &maximum).unwrap();
    for bad in [
        {
            let mut r = maximum.clone();
            r["maximum_frames_per_camera"] = 100_001.into();
            r
        },
        {
            let mut r = maximum.clone();
            r["schema_version"] = 1.into();
            r
        },
        {
            let mut r = maximum.clone();
            r.as_object_mut().unwrap().remove("storage");
            r
        },
        {
            let mut r = maximum.clone();
            r["storage"]["maximum_bundle_bytes"] = 1.into();
            r
        },
        {
            let mut r = maximum.clone();
            r["storage"]["maximum_recording_bytes_per_camera"] = 1.into();
            r
        },
        {
            let mut r = maximum.clone();
            r["storage"]["pair_journal"]["segment_bytes"] = 1024.into();
            r
        },
    ] {
        assert!(open(&t.0, &bad).is_err());
    }
    let mut legacy = settings(1000);
    legacy["schema_version"] = 1.into();
    legacy.as_object_mut().unwrap().remove("storage");
    open(&t.0, &legacy).unwrap();
    legacy["maximum_frames_per_camera"] = 1001.into();
    assert!(open(&t.0, &legacy).is_err());
}
#[test]
fn pair_budget_failure_closes_both_sources_and_retains_read_only_recovery_prefix() {
    let t = Temp::new();
    let mut request = settings(8);
    request["storage"]["pair_journal"] =
        json!({"segment_bytes":65536,"maximum_record_bytes":1024,"maximum_total_bytes":4096});
    let mut processor = open(&t.0, &request).unwrap();
    let target = t.0.join("failed");
    let error = capture(&mut processor, &target, 8).unwrap_err();
    assert!(error.contains("record exceeds"), "{error}");
    assert!(!target.exists());
    let partial = fs::read_dir(&t.0)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    let before: Vec<_> = fs::read_dir(&partial)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            let bytes = fs::read(&p).unwrap();
            (p, bytes)
        })
        .collect();
    let work = WorkControl::default();
    assert!(
        pair_journal::visit(&partial, pair_journal::Mode::Complete, &work, |_, _, _| Ok(
            ()
        ))
        .is_err()
    );
    let recovered =
        pair_journal::visit(&partial, pair_journal::Mode::Recovery, &work, |_, _, _| {
            Ok(())
        })
        .unwrap();
    assert!(!recovered.journal_complete);
    assert_eq!(recovered.pairs, 0);
    for (path, bytes) in before {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn recovery_rejects_semantically_inconsistent_complete_records_without_needing_a_footer() {
    let t = Temp::new();
    let work = WorkControl::default();
    let target = t.0.join("source");
    let mut processor = open(&t.0, &settings(1)).unwrap();
    capture(&mut processor, &target, 1).unwrap();
    let index = fs::read(target.join("pairs.index.ndjson")).unwrap();
    let header = index.split_inclusive(|b| *b == b'\n').next().unwrap();
    let bytes = fs::read(target.join("pairs-000000.ndjson")).unwrap();
    let record: Value = serde_json::from_slice(&bytes).unwrap();
    let root = t.0.join("recovery");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("pairs.index.ndjson"), header).unwrap();
    fs::write(root.join("pairs-000000.ndjson"), &bytes).unwrap();
    assert_eq!(
        pair_journal::visit(&root, pair_journal::Mode::Recovery, &work, |_, _, _| Ok(()))
            .unwrap()
            .pairs,
        1
    );
    for bad in [
        {
            let mut r = record.clone();
            r["result"]["pair_id"] = 1.into();
            r
        },
        {
            let mut r = record.clone();
            r["result"]["left_index"] = 1.into();
            r
        },
        {
            let mut r = record.clone();
            r["result"]["left_host_us"] = 1.into();
            r
        },
        {
            let mut r = record.clone();
            r["result"]["raw_pts"] = 1.into();
            r
        },
        {
            let mut r = record.clone();
            r["result"]["synchronization_verified"] = true.into();
            r
        },
        {
            let mut r = record.clone();
            r["result"]["physical_accuracy_qualified"] = true.into();
            r
        },
        {
            let mut r = record.clone();
            r["frames"][0]["recording_role"] = "first.raptor".into();
            r
        },
        {
            let mut r = record;
            r["frames"][0]["frame_index"] = 100_000.into();
            r
        },
    ] {
        let mut bytes = serde_json::to_vec(&bad).unwrap();
        bytes.push(b'\n');
        fs::write(root.join("pairs-000000.ndjson"), &bytes).unwrap();
        assert!(
            pair_journal::visit(&root, pair_journal::Mode::Recovery, &work, |_, _, _| Ok(()))
                .is_err()
        );
        assert_eq!(fs::read(root.join("pairs-000000.ndjson")).unwrap(), bytes);
    }
}
