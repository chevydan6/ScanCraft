use super::*;
use crate::laser_stream::tests::{Fake, fixture};
use std::{io::Write, sync::atomic::AtomicUsize};
struct Files {
    root: PathBuf,
    geometry: PathBuf,
    request: PathBuf,
    frames: [scancraft_core::Frame; 2],
}
impl Files {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "scancraft-laser-bundle-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let (geometry, processing, _, frames) = fixture();
        let g = root.join("geometry.json");
        let r = root.join("request.json");
        fs::write(&g, serde_json::to_vec(&geometry).unwrap()).unwrap();
        fs::write(&r,serde_json::to_vec(&json!({"schema_version":1,"fps":30,"maximum_frames_per_camera":1,"maximum_recording_bytes_per_camera":2_000_000,"maximum_pair_log_bytes":1_000_000,"processing":processing})).unwrap()).unwrap();
        Self {
            root,
            geometry: g,
            request: r,
            frames,
        }
    }
    fn plan(&self) -> Plan {
        Plan::open(&self.geometry, &self.request, &WorkControl::default()).unwrap()
    }
    fn change(&self, edit: impl FnOnce(&mut Value)) {
        let mut r: Value = serde_json::from_slice(&fs::read(&self.request).unwrap()).unwrap();
        edit(&mut r);
        fs::write(&self.request, serde_json::to_vec(&r).unwrap()).unwrap();
    }
    fn partial(&self) -> PathBuf {
        fs::read_dir(&self.root)
            .unwrap()
            .map(|v| v.unwrap().path())
            .find(|p| p.is_dir())
            .unwrap()
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn source(
    files: &Files,
    closed: Arc<AtomicUsize>,
) -> impl Fn(Camera, StreamProfile, StopToken) -> Result<Fake> + Send + Sync + 'static {
    let frames = files.frames.clone();
    move |c, _, token| {
        Ok(Fake {
            template: frames[c.index()].clone(),
            next: 0,
            count: 1,
            gate: None,
            gate_once: false,
            token,
            closed: closed.clone(),
            fail_close: false,
            fail_next: false,
        })
    }
}
#[test]
fn bundle_verifies_raw_lineage_checksums_and_no_metric_exports_before_publication() {
    let f = Files::new();
    let plan = f.plan();
    let closed = Arc::new(AtomicUsize::new(0));
    let target = f.root.join("capture");
    let mut restored = false;
    let mut previewed = 0;
    let report = plan
        .capture(
            &target,
            &Arc::new(WorkControl::default()),
            &Arc::new(AtomicBool::new(false)),
            source(&f, closed.clone()),
            || {
                assert_eq!(closed.load(Ordering::Relaxed), 2);
                restored = true;
                Ok(())
            },
            |input| {
                assert_eq!(input.progress.journaled_frames, [1; 2]);
                assert_eq!(input.progress.processing_queue_drops, [0; 2]);
                assert_eq!(input.profiles.each_ref().map(|p| p.interface), [0, 2]);
                let r = input.outcome;
                assert_eq!(r.as_ref().unwrap().candidates.len(), 4);
                previewed += 1;
                PreviewStatus::Accepted
            },
        )
        .unwrap();
    assert!(restored);
    assert_eq!(previewed, 1);
    assert_eq!(report["geometry_exported"], false);
    assert_eq!(report["raw_replay_integrity"], "PASS");
    assert_eq!(report["summary"]["sources"][0]["journaled_frames"], 1);
    let manifest: Value =
        serde_json::from_slice(&fs::read(target.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["artifacts"].as_array().unwrap().len(), 6);
    for a in manifest["artifacts"].as_array().unwrap() {
        let bytes = fs::read(target.join(a["role"].as_str().unwrap())).unwrap();
        assert_eq!(a["bytes"], bytes.len());
        assert_eq!(a["crc32"], crc32fast::hash(&bytes));
    }
    let records = fs::read_to_string(target.join("pairs.ndjson")).unwrap();
    let line: Value = serde_json::from_str(records.trim()).unwrap();
    assert_eq!(
        line["result"]["Ok"]["candidates"].as_array().unwrap().len(),
        4
    );
    assert_eq!(line["acquisition"]["journaled_frames"], json!([1, 1]));
    assert_eq!(line["acquisition"]["processing_queue_drops"], json!([0, 0]));
    for i in 0..2 {
        assert_eq!(line["frames"][i]["frame_index"], 0);
        assert_eq!(
            line["frames"][i]["payload_crc32"],
            crc32fast::hash(&f.frames[i].data)
        );
        let data = fs::read(target.join(if i == 0 {
            "first.raptor"
        } else {
            "second.raptor"
        }))
        .unwrap();
        let mut reader = scancraft_recording::Reader::new(data.as_slice()).unwrap();
        let frame = reader.read_frame().unwrap().unwrap();
        assert_eq!(
            line["frames"][i]["metadata_crc32"],
            crc32fast::hash(&frame.metadata)
        );
        assert_eq!(line["frames"][i]["sequence"], frame.sequence);
        assert_eq!(
            line["frames"][i]["host_timestamp_us"],
            frame.host_timestamp_us
        );
        assert_eq!(line["frames"][i]["flags"], frame.flags);
        assert!(reader.read_frame().unwrap().is_none());
    }
    assert!(!target.join("points.ply").exists());
    assert!(!target.join("failure.json").exists());
}
#[test]
fn optional_preview_omission_and_panic_do_not_fail_raw_recording_or_cancel_work() {
    for panic in [false, true] {
        let f = Files::new();
        let plan = f.plan();
        let work = Arc::new(WorkControl::default());
        let r = plan
            .capture(
                &f.root.join("out"),
                &work,
                &Arc::new(AtomicBool::new(false)),
                source(&f, Arc::new(AtomicUsize::new(0))),
                || Ok(()),
                |_| {
                    if panic {
                        panic!("injected display panic");
                    }
                    PreviewStatus::Omitted
                },
            )
            .unwrap();
        assert_eq!(r["optional_previews"]["omitted"], 1);
        assert!(work.check().is_ok());
        assert_eq!(r["summary"]["processed_pairs"], 1);
    }
}
#[test]
fn restoration_failure_retains_raw_journals_and_terminal_evidence_without_publication() {
    let f = Files::new();
    let plan = f.plan();
    let target = f.root.join("out");
    let closed = Arc::new(AtomicUsize::new(0));
    let e = plan
        .capture(
            &target,
            &Arc::new(WorkControl::default()),
            &Arc::new(AtomicBool::new(false)),
            source(&f, closed.clone()),
            || {
                assert_eq!(closed.load(Ordering::Relaxed), 2);
                Err("injected restoration failure".into())
            },
            |_| PreviewStatus::NotRequested,
        )
        .unwrap_err();
    assert!(e.contains("restoration failure"));
    assert!(!target.exists());
    let p = f.partial();
    assert!(p.join("first.raptor").exists());
    let failure: Value =
        serde_json::from_slice(&fs::read(p.join("failure.json")).unwrap()).unwrap();
    assert_eq!(failure["stream_result"]["Ok"]["processed_pairs"], 1);
    assert!(
        failure["settings_restoration"]["Err"]
            .as_str()
            .unwrap()
            .contains("restoration")
    );
}
#[test]
fn invalid_policy_changed_inputs_and_existing_target_never_open_sources() {
    for change in ["schema", "bytes", "detection", "continuity", "opening"] {
        let f = Files::new();
        f.change(|r| match change {
            "schema" => r["schema_version"] = json!(2),
            "bytes" => r["maximum_recording_bytes_per_camera"] = json!(1),
            "detection" => r["processing"]["detection"]["background"] = json!(250),
            "continuity" => r["processing"]["continuity"]["maximum_row_gap"] = json!(0),
            _ => r["processing"]["detection"]["opening_radius"] = json!(128),
        });
        assert!(Plan::open(&f.geometry, &f.request, &WorkControl::default()).is_err());
    }
    for changed in [false, true] {
        let f = Files::new();
        let plan = f.plan();
        let target = f.root.join("out");
        if changed {
            fs::write(&f.geometry, b"{}").unwrap();
        } else {
            fs::create_dir(&target).unwrap();
        }
        let mut restored = false;
        let e = plan
            .capture::<Fake, _, _>(
                &target,
                &Arc::new(WorkControl::default()),
                &Arc::new(AtomicBool::new(false)),
                |_, _, _| panic!("must not open source"),
                || {
                    restored = true;
                    Ok(())
                },
                |_| panic!("must not preview"),
            )
            .unwrap_err();
        assert!(restored);
        assert!(e.contains(if changed { "changed" } else { "already exists" }));
    }
}
#[test]
fn publication_race_and_input_mutation_after_capture_retain_diagnostics() {
    for race in [false, true] {
        let f = Files::new();
        let plan = f.plan();
        let target = f.root.join("out");
        let changed = f.geometry.clone();
        let e = plan
            .capture(
                &target,
                &Arc::new(WorkControl::default()),
                &Arc::new(AtomicBool::new(false)),
                source(&f, Arc::new(AtomicUsize::new(0))),
                || {
                    if race {
                        fs::create_dir(&target).unwrap();
                        fs::write(target.join("owner"), b"preserve").unwrap();
                    } else {
                        fs::write(changed, b"{}").unwrap();
                    }
                    Ok(())
                },
                |_| PreviewStatus::NotRequested,
            )
            .unwrap_err();
        assert!(e.contains("incomplete capture retained"));
        if race {
            assert_eq!(fs::read(target.join("owner")).unwrap(), b"preserve");
        } else {
            assert!(!target.exists());
        }
    }
}
#[test]
fn capped_writer_enforces_exact_budget_and_readback_detects_mutation() {
    let f = Files::new();
    let mut out = CappedFile::new(&f.root, "bounded", 3).unwrap();
    out.write_all(b"abc").unwrap();
    assert!(out.write_all(b"d").is_err());
    let digest = out.finish().unwrap();
    io::verify(
        &f.root,
        &digest,
        &mut Guard::default(),
        &WorkControl::default(),
    )
    .unwrap();
    fs::write(f.root.join("bounded"), b"abd").unwrap();
    assert!(
        io::verify(
            &f.root,
            &digest,
            &mut Guard::default(),
            &WorkControl::default()
        )
        .unwrap_err()
        .contains("differs")
    );
}

#[test]
fn required_pair_log_budget_failure_keeps_finalized_raw_prefix_and_cleanup_evidence() {
    let f = Files::new();
    f.change(|r| {
        r["maximum_frames_per_camera"] = json!(2);
        r["maximum_pair_log_bytes"] = json!(4096);
    });
    let plan = f.plan();
    let frames = f.frames.clone();
    let closed = Arc::new(AtomicUsize::new(0));
    let close = closed.clone();
    let target = f.root.join("out");
    let e = plan
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
                    closed: close.clone(),
                    fail_close: false,
                    fail_next: false,
                })
            },
            || Ok(()),
            |_| PreviewStatus::NotRequested,
        )
        .unwrap_err();
    assert!(e.contains("byte budget"));
    assert_eq!(closed.load(Ordering::Relaxed), 2);
    assert!(!target.exists());
    let p = f.partial();
    let failure: Value =
        serde_json::from_slice(&fs::read(p.join("failure.json")).unwrap()).unwrap();
    for source in failure["stream_result"]["Err"]["partial"]["sources"]
        .as_array()
        .unwrap()
    {
        assert_eq!(source["cleanup_confirmed"], true);
        assert_eq!(source["journal_finalized"], true);
        let i = source["camera_index"].as_u64().unwrap();
        let bytes = fs::read(p.join(if i == 0 {
            "first.raptor"
        } else {
            "second.raptor"
        }))
        .unwrap();
        let mut reader = scancraft_recording::Reader::new(bytes.as_slice()).unwrap();
        let mut n = 0;
        while reader.read_frame().unwrap().is_some() {
            n += 1;
        }
        assert_eq!(source["journaled_frames"], n);
    }
}
#[test]
fn raw_mutation_after_cleanup_is_detected_before_publication() {
    let f = Files::new();
    let plan = f.plan();
    let target = f.root.join("out");
    let error = plan
        .capture(
            &target,
            &Arc::new(WorkControl::default()),
            &Arc::new(AtomicBool::new(false)),
            source(&f, Arc::new(AtomicUsize::new(0))),
            || {
                let p = f.partial().join("first.raptor");
                let mut bytes = fs::read(&p).unwrap();
                let i = bytes.len() - 1;
                bytes[i] ^= 1;
                fs::write(p, bytes).unwrap();
                Ok(())
            },
            |_| PreviewStatus::NotRequested,
        )
        .unwrap_err();
    assert!(!target.exists());
    assert!(error.contains("incomplete capture retained"));
    assert!(f.partial().join("failure.json").exists());
}

#[test]
fn buffered_pair_records_preserve_bytes_checksums_and_exact_budget_across_flushes() {
    let f = Files::new();
    let value = json!({"observations":(0..4096).map(|i|json!({"index":i,"position":[i as f64/7.,-0.,1e-50],"error":"full diagnostic \u{03bb}"})).collect::<Vec<_>>()});
    let mut expected = Vec::new();
    for _ in 0..2 {
        serde_json::to_writer(&mut expected, &value).unwrap();
        expected.push(b'\n');
    }
    for (role, maximum, ok) in [
        ("exact", expected.len() as u64, true),
        ("one_short", expected.len() as u64 - 1, false),
    ] {
        let mut out = CappedFile::new(&f.root, role, maximum).unwrap();
        io::pair_record(&mut out, &value).unwrap();
        assert_eq!(io::pair_record(&mut out, &value).is_ok(), ok);
        let digest = out.finish().unwrap();
        let actual = fs::read(f.root.join(role)).unwrap();
        assert!(actual.len() as u64 <= maximum);
        assert_eq!(actual, expected[..actual.len()]);
        assert_eq!(digest.bytes, actual.len() as u64);
        assert_eq!(digest.crc32, crc32fast::hash(&actual));
        if ok {
            assert_eq!(actual, expected);
        }
        io::verify(
            &f.root,
            &digest,
            &mut Guard::default(),
            &WorkControl::default(),
        )
        .unwrap();
    }
}
#[test]
fn buffered_record_errors_never_retry_pending_bytes_on_drop() {
    struct Partial {
        bytes: Vec<u8>,
        calls: usize,
    }
    impl Write for Partial {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.calls += 1;
            if self.calls == 1 {
                let n = bytes.len().min(17);
                self.bytes.extend_from_slice(&bytes[..n]);
                Ok(n)
            } else {
                Err(std::io::Error::other("injected partial write failure"))
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let value = json!({"observations":vec![12345;30000]});
    let mut expected = serde_json::to_vec(&value).unwrap();
    expected.push(b'\n');
    let mut out = Partial {
        bytes: Vec::new(),
        calls: 0,
    };
    let e = io::pair_record(&mut out, &value).unwrap_err();
    assert!(e.to_string().contains("partial write"));
    assert_eq!(
        out.calls, 2,
        "error return must not trigger a Drop write retry"
    );
    assert_eq!(out.bytes, expected[..17]);
    struct BrokenFlush;
    impl Write for BrokenFlush {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("flush failed"))
        }
    }
    assert!(
        io::pair_record(&mut BrokenFlush, &json!([1, 2]))
            .unwrap_err()
            .to_string()
            .contains("flush failed")
    );
}
#[test]
fn serialization_failure_discards_unwritten_tail_and_success_is_visible_before_preview() {
    struct Incomplete;
    impl Serialize for Incomplete {
        fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
            use serde::ser::SerializeSeq;
            let mut seq = s.serialize_seq(None)?;
            seq.serialize_element("prefix")?;
            Err(serde::ser::Error::custom("injected serialization failure"))
        }
    }
    let mut bytes = Vec::new();
    assert!(io::pair_record(&mut bytes, &Incomplete).is_err());
    assert!(bytes.is_empty());
    let f = Files::new();
    let target = f.root.join("complete");
    let mut previews = 0;
    f.plan()
        .capture(
            &target,
            &Arc::new(WorkControl::default()),
            &Arc::new(AtomicBool::new(false)),
            source(&f, Arc::new(AtomicUsize::new(0))),
            || Ok(()),
            |input| {
                let staged = f.partial();
                let bytes = fs::read(staged.join("pairs.ndjson")).unwrap();
                assert_eq!(bytes.last(), Some(&b'\n'));
                let record: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(
                    record["result"]["Ok"]["queries"],
                    input.outcome.as_ref().unwrap().queries
                );
                previews += 1;
                PreviewStatus::Accepted
            },
        )
        .unwrap();
    assert_eq!(previews, 1);
}

fn segmented(files: &Files, frames: u64) {
    files.change(|r| {
        r["schema_version"] = json!(2);
        r["maximum_frames_per_camera"] = json!(frames);
        r["maximum_pair_log_bytes"] = json!(100000);
        r["pair_log_segment_bytes"] = json!(8192);
        r["maximum_pair_record_bytes"] = json!(4096);
        r["maximum_bundle_bytes"] = json!(16 * 1024 * 1024);
    });
}
#[test]
fn segmented_capture_publishes_all_pairs_raw_data_index_and_compatible_report() {
    use crate::live_stereo::runner::Source;
    use std::sync::mpsc;
    struct Paced {
        inner: Fake,
        permit: mpsc::Receiver<()>,
    }
    impl Source for Paced {
        fn context(&self) -> Result<scancraft_recording::CaptureContext> {
            self.inner.context()
        }
        fn next(&mut self) -> Result<Option<scancraft_core::Frame>> {
            if self.inner.next > 0 && self.inner.next < self.inner.count {
                loop {
                    if self.inner.token.stop_requested() {
                        return Ok(None);
                    }
                    match self
                        .permit
                        .recv_timeout(std::time::Duration::from_millis(10))
                    {
                        Ok(()) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => return Err("test permit disconnected".into()),
                    }
                }
            }
            self.inner.next()
        }
        fn statistics(&self) -> Value {
            self.inner.statistics()
        }
        fn close(self) -> Result<()> {
            self.inner.close()
        }
    }
    let f = Files::new();
    segmented(&f, 8);
    let plan = f.plan();
    let target = f.root.join("capture");
    let channels = [mpsc::channel(), mpsc::channel()];
    let (senders, receivers): (Vec<_>, Vec<_>) = channels.into_iter().unzip();
    let receivers = Arc::new(
        receivers
            .into_iter()
            .map(|r| Mutex::new(Some(r)))
            .collect::<Vec<_>>(),
    );
    let closed = Arc::new(AtomicUsize::new(0));
    let close = closed.clone();
    let frames = f.frames.clone();
    let mut offered = 0;
    let report = plan
        .capture(
            &target,
            &Arc::new(WorkControl::default()),
            &Arc::new(AtomicBool::new(false)),
            move |c, _, token| {
                Ok(Paced {
                    inner: Fake {
                        template: frames[c.index()].clone(),
                        next: 0,
                        count: 8,
                        gate: None,
                        gate_once: false,
                        token,
                        closed: close.clone(),
                        fail_close: false,
                        fail_next: false,
                    },
                    permit: receivers[c.index()].lock().unwrap().take().unwrap(),
                })
            },
            || {
                assert_eq!(closed.load(Ordering::Relaxed), 2);
                Ok(())
            },
            |_| {
                offered += 1;
                if offered < 8 {
                    for s in &senders {
                        s.send(()).unwrap();
                    }
                }
                PreviewStatus::Accepted
            },
        )
        .unwrap();
    assert_eq!(offered, 8);
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["summary"]["processed_pairs"], 8);
    assert!(report["pair_journal"]["segments"].as_u64().unwrap() > 1);
    assert!(!target.join("pairs.ndjson").exists());
    let checked = super::super::pair_journal::visit(
        &target,
        super::super::pair_journal::Mode::Complete,
        &WorkControl::default(),
        |_, _, _| Ok(()),
    )
    .unwrap();
    assert_eq!(checked.pairs, 8);
    for name in ["first.raptor", "second.raptor"] {
        let mut reader =
            scancraft_recording::Reader::new(fs::File::open(target.join(name)).unwrap()).unwrap();
        let mut n = 0;
        while reader.read_frame().unwrap().is_some() {
            n += 1;
        }
        assert_eq!(n, 8);
    }
}
#[test]
fn segmented_policy_rejects_ambiguous_options_and_insufficient_raw_bundle_admission() {
    for bad in [
        "null",
        "legacy_extra",
        "segment",
        "record",
        "slots",
        "bundle",
        "raw",
    ] {
        let f = Files::new();
        segmented(&f, 8);
        f.change(|r| match bad {
            "null" => r["maximum_pair_record_bytes"] = Value::Null,
            "legacy_extra" => r["schema_version"] = json!(1),
            "segment" => r["pair_log_segment_bytes"] = json!(1),
            "record" => r["maximum_pair_record_bytes"] = json!(9 * 1024 * 1024),
            "slots" => r["maximum_pair_log_bytes"] = json!(1u64 << 38),
            "bundle" => r["maximum_bundle_bytes"] = json!(100),
            _ => r["maximum_frames_per_camera"] = json!(u64::MAX),
        });
        assert!(
            Plan::open(&f.geometry, &f.request, &WorkControl::default()).is_err(),
            "{bad}"
        );
    }
    let f = Files::new();
    segmented(&f, 8);
    let plan = f.plan();
    let target = f.root.join("missing-parent/capture");
    let mut restored = false;
    assert!(
        plan.capture::<Fake, _, _>(
            &target,
            &Arc::new(WorkControl::default()),
            &Arc::new(AtomicBool::new(false)),
            |_, _, _| panic!("must reject admission before source"),
            || {
                restored = true;
                Ok(())
            },
            |_| PreviewStatus::NotRequested
        )
        .is_err()
    );
    assert!(restored);
    assert!(!target.exists());
}
#[test]
fn segmented_required_record_failure_retains_raw_cleanup_and_recoverable_journal_prefix() {
    let f = Files::new();
    segmented(&f, 1);
    f.change(|r| r["maximum_pair_record_bytes"] = json!(1024));
    let target = f.root.join("out");
    let closed = Arc::new(AtomicUsize::new(0));
    let error = f
        .plan()
        .capture(
            &target,
            &Arc::new(WorkControl::default()),
            &Arc::new(AtomicBool::new(false)),
            source(&f, closed.clone()),
            || Ok(()),
            |_| panic!("failed journal must not preview"),
        )
        .unwrap_err();
    assert!(error.contains("record exceeds"));
    assert_eq!(closed.load(Ordering::Relaxed), 2);
    assert!(!target.exists());
    let partial = f.partial();
    let fail: Value =
        serde_json::from_slice(&fs::read(partial.join("failure.json")).unwrap()).unwrap();
    assert_eq!(fail["settings_restoration"]["Ok"], Value::Null);
    let journal = super::super::pair_journal::visit(
        &partial,
        super::super::pair_journal::Mode::Recovery,
        &WorkControl::default(),
        |_, _, _| Ok(()),
    )
    .unwrap();
    assert!(!journal.journal_complete);
    assert_eq!(journal.pairs, 0);
}
