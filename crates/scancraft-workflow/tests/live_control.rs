//! Scripted source events make command boundaries deterministic without scanner I/O.
use scancraft_core::{Frame, StreamProfile};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::CaptureContext;
use scancraft_workflow::{
    live_stereo::{
        InputMode, Processor, Side,
        control::{Control, Phase, Snapshot},
        runner::{self, Source},
    },
    output::staging,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};
struct Script {
    input: mpsc::Receiver<Frame>,
}
impl Source for Script {
    fn context(&self) -> Result<CaptureContext, String> {
        Ok(CaptureContext {
            controls: json!({"synthetic":true}),
            calibration_status: "synthetic_fixture_not_a_scanner_capture".into(),
            ..Default::default()
        })
    }
    fn next(&mut self) -> Result<Option<Frame>, String> {
        Ok(self.input.recv().ok())
    }
    fn statistics(&self) -> Value {
        json!({"synthetic":true})
    }
    fn close(self) -> Result<(), String> {
        Ok(())
    }
}
struct Harness {
    root: PathBuf,
    target: PathBuf,
    control: Arc<Control>,
    work: Arc<WorkControl>,
    senders: Option<[mpsc::Sender<Frame>; 2]>,
    job: Option<thread::JoinHandle<Result<Value, String>>>,
}
impl Harness {
    fn start() -> Self {
        let root = staging(&std::env::temp_dir().join("live-control-test")).unwrap();
        let target = root.join("scan");
        let config = root.join("scan.json");
        fs::write(&config,serde_json::to_vec(&json!({"schema_version":1,"session_id":"controlled-fixture","fps":30,"maximum_frames_per_camera":8,"maximum_host_delta_us":1000,
   "tracking_samples":32,"voxel_mm":0.1,"maximum_voxels":1000,"tracking":{"max_correspondence_mm":10.0,"max_rms_mm":0.01,"minimum_overlap":0.8,"max_step_translation_mm":1.0,"max_step_rotation_rad":0.1,"iterations":40,"tolerance_mm":1e-8}})).unwrap()).unwrap();
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
        let work = Arc::new(WorkControl::default());
        let mut processor = Processor::open(
            &fixture.join("calibration.rapcal"),
            &fixture.join("request.json"),
            &config,
            InputMode::SyntheticFixture,
            &work,
        )
        .unwrap();
        let (left, lr) = mpsc::channel();
        let (right, rr) = mpsc::channel();
        let receivers = Arc::new([Mutex::new(Some(lr)), Mutex::new(Some(rr))]);
        let control = Arc::new(Control::new(true));
        let c = control.clone();
        let w = work.clone();
        let out = target.clone();
        let job = thread::spawn(move || {
            runner::bundle_controlled(
                &mut processor,
                &out,
                &w,
                move |side: Side, _: StreamProfile| {
                    Ok(Script {
                        input: receivers[side.index()].lock().unwrap().take().unwrap(),
                    })
                },
                || Ok(()),
                &c,
            )
        });
        Self {
            root,
            target,
            control,
            work,
            senders: Some([left, right]),
            job: Some(job),
        }
    }
    fn pair(&self, index: u32) {
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
        for side in 0..2 {
            let pixels =
                image::open(fixture.join(if side == 0 { "left.png" } else { "right.png" }))
                    .unwrap()
                    .to_luma8()
                    .into_raw();
            let pts = 1000 + index * 33333;
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&pts.to_le_bytes());
            self.senders.as_ref().unwrap()[side]
                .send(Frame {
                    sequence: index,
                    host_timestamp_us: u64::from(pts) + side as u64 * 10,
                    flags: 0,
                    metadata,
                    data: pixels,
                })
                .unwrap();
        }
    }
    fn snapshot(&self) -> Snapshot {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(value) = self.control.take_preview() {
                return value;
            }
            assert!(Instant::now() < deadline, "no preview delivered");
            thread::sleep(Duration::from_millis(1));
        }
    }
    fn finish(&mut self) -> Result<Value, String> {
        self.senders.take();
        self.job.take().unwrap().join().unwrap()
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.control.request_stop();
        self.work.cancel();
        self.senders.take();
        if let Some(job) = self.job.take() {
            let _ = job.join();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}
#[test]
fn pause_resume_and_graceful_stop_keep_raw_data_and_publish_final_preview() {
    let mut h = Harness::start();
    h.pair(0);
    let first = h.snapshot();
    assert_eq!(first.fused_points, 3);
    h.control.set_paused(true).unwrap();
    h.pair(1);
    let paused = h.snapshot();
    assert_eq!(paused.tracking["state"], "paused");
    assert_eq!(paused.fused_points, first.fused_points);
    assert_eq!(h.control.phase(), Phase::Paused);
    h.control.set_paused(false).unwrap();
    h.pair(2);
    assert_eq!(h.snapshot().tracking["integrated"], true);
    h.control.request_stop();
    let report = h.finish().unwrap();
    assert_eq!(h.control.phase(), Phase::Complete);
    assert_eq!(report["graceful_stop_requested"], true);
    assert_eq!(report["sources"][0]["recorded_frames"], 3);
    assert_eq!(report["sources"][1]["recorded_frames"], 3);
    assert_eq!(report["pairs"][1]["tracking"]["state"], "paused");
    let final_preview = h.control.take_preview().unwrap();
    assert_eq!(final_preview.pair_id, 2);
    assert_eq!(
        final_preview.fused_points as u64,
        report["fused_points"].as_u64().unwrap()
    );
    assert!(h.target.join("project.orproj").is_file());
    assert!(h.control.set_paused(false).is_err());
}
#[test]
fn abort_retains_partial_and_does_not_publish() {
    let mut h = Harness::start();
    h.pair(0);
    h.snapshot();
    h.work.cancel();
    let error = h.finish().unwrap_err();
    assert!(error.contains("incomplete scan retained"));
    assert!(!h.target.exists());
    assert_eq!(h.control.phase(), Phase::Failed);
}
#[test]
fn slow_display_keeps_one_snapshot_and_final_state_replaces_stale_preview() {
    let mut h = Harness::start();
    h.pair(0);
    h.pair(1);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let ready = fs::read_dir(&h.root)
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.path().join("pair-0001.json").is_file());
        if ready {
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    h.control.request_stop();
    let report = h.finish().unwrap();
    let snapshot = h.control.take_preview().unwrap();
    assert_eq!(snapshot.pair_id, 1);
    assert_eq!(
        snapshot.fused_points as u64,
        report["fused_points"].as_u64().unwrap()
    );
    assert!(h.control.skipped_previews() >= 1);
    assert!(h.control.take_preview().is_none());
}
