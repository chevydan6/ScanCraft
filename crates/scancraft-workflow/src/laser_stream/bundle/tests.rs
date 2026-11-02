use super::*;
use crate::laser_stream::tests::{Fake, fixture};
use std::sync::atomic::AtomicUsize;
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
            |_, r| {
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
                |_, _| {
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
            |_, _| PreviewStatus::NotRequested,
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
                |_, _| panic!("must not preview"),
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
                |_, _| PreviewStatus::NotRequested,
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
            |_, _| PreviewStatus::NotRequested,
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
            |_, _| PreviewStatus::NotRequested,
        )
        .unwrap_err();
    assert!(!target.exists());
    assert!(error.contains("incomplete capture retained"));
    assert!(f.partial().join("failure.json").exists());
}
