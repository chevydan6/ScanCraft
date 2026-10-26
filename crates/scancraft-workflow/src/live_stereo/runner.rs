//! Reusable finite acquisition/recording/reconstruction runner. Hardware adapters
//! own their own device semantics; the caller restores group settings after all
//! sources close and before publication. Driver shutdown is not time bounded.
use super::{
    Association, Processor, Sample, Side,
    control::{self, Control, Phase},
};
use crate::output::{publish, staging};
use scancraft_core::{Frame, StreamProfile, uvc_pts};
use scancraft_geometry::{Editor, formats::save_project, work::WorkControl, write_ply};
use scancraft_recording::{CaptureContext, Reader, Writer};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread,
    time::Duration,
};
type Result<T> = std::result::Result<T, String>;
/// Source is constructed inside its owning acquisition thread. `close` must
/// complete cleanup; errors are retained alongside processing failures.
pub trait Source {
    fn context(&self) -> Result<CaptureContext>;
    fn next(&mut self) -> Result<Option<Frame>>;
    fn statistics(&self) -> Value;
    fn close(self) -> Result<()>;
}
#[derive(Serialize)]
struct SourceReport {
    interface: u8,
    recorded_frames: usize,
    queued_frames: usize,
    processing_queue_drops: usize,
    statistics: Value,
}
struct Event {
    side: Side,
    sample: Sample,
}
fn combine<T>(result: Result<T>, cleanup: Result<()>) -> Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(e), Err(c)) => Err(format!("{e}; cleanup: {c}")),
    }
}
#[cfg(test)]
fn record<S: Source>(
    profile: StreamProfile,
    path: &Path,
    maximum: usize,
    stop: &AtomicBool,
    sender: SyncSender<Event>,
    side: Side,
    open: impl FnOnce() -> Result<S>,
) -> Result<SourceReport> {
    record_controlled(
        profile,
        path,
        maximum,
        (stop, &Control::default()),
        sender,
        side,
        open,
    )
}
fn record_controlled<S: Source>(
    profile: StreamProfile,
    path: &Path,
    maximum: usize,
    signals: (&AtomicBool, &Control),
    sender: SyncSender<Event>,
    side: Side,
    open: impl FnOnce() -> Result<S>,
) -> Result<SourceReport> {
    let (stop, control) = signals;
    let mut source = open()?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        let mut writer =
            Writer::with_context(BufWriter::new(file), profile.clone(), source.context()?)
                .map_err(|e| e.to_string())?;
        let mut report = SourceReport {
            interface: profile.interface,
            recorded_frames: 0,
            queued_frames: 0,
            processing_queue_drops: 0,
            statistics: Value::Null,
        };
        for index in 0..maximum {
            if stop.load(Ordering::Relaxed) || control.stop_requested() {
                break;
            }
            let Some(frame) = source.next()? else {
                break;
            };
            writer.push(&frame).map_err(|e| e.to_string())?;
            report.recorded_frames += 1;
            control.recorded[side.index()].fetch_add(1, Ordering::Relaxed);
            match sender.try_send(Event {
                side,
                sample: Sample { index, frame },
            }) {
                Ok(()) => report.queued_frames += 1,
                Err(TrySendError::Full(_)) => {
                    report.processing_queue_drops += 1;
                    control.queue_drops[side.index()].fetch_add(1, Ordering::Relaxed);
                }
                Err(TrySendError::Disconnected(_)) => {
                    report.processing_queue_drops += 1;
                    control.queue_drops[side.index()].fetch_add(1, Ordering::Relaxed);
                    break;
                }
            }
        }
        let out = writer.finish().map_err(|e| e.to_string())?;
        out.get_ref().sync_all().map_err(|e| e.to_string())?;
        report.statistics = source.statistics();
        Ok(report)
    }))
    .unwrap_or_else(|_| Err("source acquisition or recording panicked".into()));
    let cleanup = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| source.close()))
        .unwrap_or_else(|_| Err("source cleanup panicked".into()));
    combine(result, cleanup)
}
fn file(path: &Path, write: impl FnOnce(&mut BufWriter<File>) -> Result<()>) -> Result<()> {
    let f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let mut out = BufWriter::new(f);
    write(&mut out)?;
    out.flush().map_err(|e| e.to_string())?;
    out.get_ref().sync_all().map_err(|e| e.to_string())
}
fn json_file(path: &Path, value: &Value) -> Result<()> {
    file(path, |out| {
        serde_json::to_writer_pretty(out, value).map_err(|e| e.to_string())
    })
}
/// Reject existing paths (including dangling symlinks) before hardware changes.
pub fn require_new_target(target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !fs::metadata(parent).map_err(|e| e.to_string())?.is_dir() {
        return Err("scan output parent must be an existing directory".into());
    }
    match fs::symlink_metadata(target) {
        Ok(_) => Err("scan output already exists (including symlinks)".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}
fn verify_recording(path: &Path, expected: usize, work: &WorkControl) -> Result<()> {
    let input = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| e.to_string())?;
    if !input.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("raw recording must be a regular file".into());
    }
    let mut reader = Reader::new(BufReader::new(input)).map_err(|e| e.to_string())?;
    let mut count = 0;
    let mut pts = HashSet::new();
    while let Some(frame) = reader.read_frame().map_err(|e| e.to_string())? {
        work.check()?;

        let value = uvc_pts(&frame.metadata)
            .map_err(str::to_string)?
            .ok_or("missing recorded PTS")?;
        if !pts.insert(value) {
            return Err(
                "repeated raw PTS in complete recording makes candidate association ambiguous"
                    .into(),
            );
        }
        count += 1;
        if count > 1000 {
            return Err("raw recording exceeds live frame cap".into());
        }
    }
    if count != expected {
        return Err("live raw-recording replay count differs from capture count".into());
    }
    Ok(())
}
struct Workers {
    stop: Arc<AtomicBool>,
    handles: Vec<thread::JoinHandle<Result<SourceReport>>>,
}
impl Drop for Workers {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for handle in self.handles.drain(..) {
            match handle.join() {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => eprintln!("live capture cleanup during unwind: {error}"),
                Err(_) => eprintln!("live capture worker panicked during unwind"),
            }
        }
    }
}
fn execute<S: Source + 'static>(
    processor: &mut Processor,
    partial: &Path,
    work: &WorkControl,
    open: impl Fn(Side, StreamProfile) -> Result<S> + Send + Sync + 'static,
    control: &Arc<Control>,
) -> Result<(Value, Option<Editor>)> {
    work.check()?;
    let mut association = Association::new(
        processor.profiles().clone(),
        processor.config().maximum_frames_per_camera,
        processor.config().maximum_host_delta_us,
    )?;
    let stop = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::sync_channel(4);
    let open = Arc::new(open);
    let mut workers = Workers {
        stop: stop.clone(),
        handles: Vec::new(),
    };
    let mut spawn_error = None;
    for side in [Side::Left, Side::Right] {
        let sender = sender.clone();
        let stop = stop.clone();
        let open = open.clone();
        let control = control.clone();
        let profile = processor.profiles()[side.index()].clone();
        let path = partial.join(if side.index() == 0 {
            "left.raptor"
        } else {
            "right.raptor"
        });
        let maximum = processor.config().maximum_frames_per_camera;
        match thread::Builder::new()
            .name(format!("scancraft-live-{}", side.index()))
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    record_controlled(
                        profile.clone(),
                        &path,
                        maximum,
                        (&stop, &control),
                        sender,
                        side,
                        || open(side, profile),
                    )
                }))
                .unwrap_or_else(|_| {
                    Err("capture worker panicked; recording may require recovery".into())
                });
                if result.is_err() {
                    stop.store(true, Ordering::Relaxed);
                }
                result
            }) {
            Ok(handle) => workers.handles.push(handle),
            Err(e) => {
                spawn_error = Some(e.to_string());
                break;
            }
        }
    }
    drop(sender);
    let mut latest_pair = None;
    let processing = (|| {
        if let Some(error) = spawn_error {
            return Err(error);
        }
        let mut reports = Vec::new();
        loop {
            work.check()?;
            let paused = control.pause_requested();
            processor.set_paused(paused)?;
            if control.stop_requested() {
                stop.store(true, Ordering::Relaxed);
                control.set_phase(Phase::Stopping);
            } else {
                control.set_phase(if paused {
                    Phase::Paused
                } else {
                    Phase::Running
                });
            }
            match receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(event) => {
                    if let Some(pair) = association.push(event.side, event.sample)? {
                        let result = processor.process(&pair, work).map_err(|e| e.to_string())?;
                        if control.wants_preview() {
                            let (total, cloud) =
                                processor.preview_cloud(work).map_err(|e| e.to_string())?;
                            control.offer(control::snapshot(
                                &pair,
                                processor.profiles(),
                                reports.len(),
                                cloud,
                                total,
                                result["tracking"].clone(),
                            ));
                        }
                        let name = format!("pair-{:04}.json", reports.len());
                        json_file(&partial.join(&name), &result)?;
                        reports.push(json!({"pair_id":result["pair_id"],"left_index":result["left_index"],"right_index":result["right_index"],
                        "raw_pts":result["raw_pts"],"tracking":result["tracking"],"report":name}));
                        if control.preview_enabled() {
                            latest_pair = Some(pair);
                        }
                        eprintln!(
                            "{}",
                            json!({"event":"live_stereo_pair","pairs":reports.len(),"tracking":result["tracking"]})
                        );
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(reports)
    })();
    stop.store(true, Ordering::Relaxed);
    drop(receiver);
    // Never detach a camera owner: all cleanup completes before settings restore.
    // Driver setup/close remain synchronous and have no established time bound.
    let mut sources = Vec::new();
    let mut errors = Vec::new();
    for handle in workers.handles.drain(..) {
        match handle.join() {
            Ok(Ok(report)) => sources.push(report),
            Ok(Err(e)) => errors.push(e),
            Err(_) => errors.push("capture thread join failed".into()),
        }
    }
    let reports = combine(
        processing,
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        },
    )?;
    control.set_phase(Phase::Finishing);
    for source in &sources {
        verify_recording(
            &partial.join(if source.interface == 0 {
                "left.raptor"
            } else {
                "right.raptor"
            }),
            source.recorded_frames,
            work,
        )?;
    }
    if let Some(pair) = latest_pair {
        let (total, cloud) = processor.preview_cloud(work).map_err(|e| e.to_string())?;
        let tracking = reports
            .last()
            .map(|r| r["tracking"].clone())
            .unwrap_or(Value::Null);
        control.offer(control::snapshot(
            &pair,
            processor.profiles(),
            reports.len() - 1,
            cloud,
            total,
            tracking,
        ));
    }
    let cloud = processor.finish(work).map_err(|e| e.to_string())?;
    if !cloud.points.is_empty() {
        file(&partial.join("points.ply"), |out| {
            write_ply(&cloud, out).map_err(|e| e.to_string())
        })?;
    }
    let count = cloud.points.len();
    let editor = if count > 0 {
        Some(Editor::new(cloud, 32)?)
    } else {
        None
    };
    Ok((
        json!({"schema_version":1,"kind":if matches!(processor.source(),scancraft_depth::MatchSource::SyntheticFixture){"synthetic_live_stereo_pipeline"}else{"native_live_stereo_candidate_scan"},"declared_source":processor.source(),"session_id":processor.config().session_id,
        "source_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,
        "pair_policy":"unique equal raw PTS and explicit host window; not verified exposures",
        "graceful_stop_requested":control.stop_requested(),"pause_requested_at_end":control.pause_requested(),"skipped_display_previews":control.skipped_previews(),"pending_capacity_per_side":2,"processing_channel_capacity":4,"association":association.counts,"unmatched_pending":association.remaining(),
        "sources":sources,"pairs":reports,"fused_points":count,"geometry_exported":count>0,
        "project_scope":"derived editable cloud with initial live-session references; external raw recordings and reports required for verification; not resumable fusion"}),
        editor,
    ))
}

/// Append raw frames before bounded processing delivery; restore all sources
/// before atomic no-overwrite publication. Failures retain the staging folder.
/// Cancellation checks are cooperative; device calls are not interrupted here.
pub fn bundle<S: Source + 'static>(
    processor: &mut Processor,
    target: &Path,
    work: &WorkControl,
    open: impl Fn(Side, StreamProfile) -> Result<S> + Send + Sync + 'static,
    restore: impl FnOnce() -> Result<()>,
) -> Result<Value> {
    bundle_controlled(
        processor,
        target,
        work,
        open,
        restore,
        &Arc::new(Control::default()),
    )
}
/// Single-session pause/resume, graceful stop and bounded display snapshots.
pub fn bundle_controlled<S: Source + 'static>(
    processor: &mut Processor,
    target: &Path,
    work: &WorkControl,
    open: impl Fn(Side, StreamProfile) -> Result<S> + Send + Sync + 'static,
    restore: impl FnOnce() -> Result<()>,
    control: &Arc<Control>,
) -> Result<Value> {
    control.begin()?;
    let outcome = bundle_inner(processor, target, work, open, restore, control);
    control.set_phase(if outcome.is_ok() {
        Phase::Complete
    } else {
        Phase::Failed
    });
    outcome
}
fn fingerprint(path: &Path, work: &WorkControl) -> Result<Value> {
    let mut input = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| e.to_string())?;
    if !input.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("live artifacts must be regular files".into());
    }
    let mut hash = crc32fast::Hasher::new();
    let mut bytes = 0;
    let mut buffer = [0; 65536];
    loop {
        work.check()?;
        let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        bytes += n as u64;
    }
    Ok(
        json!({"name":path.file_name().ok_or("artifact lacks name")?.to_string_lossy(),"bytes":bytes,"crc32":hash.finalize()}),
    )
}
fn bundle_inner<S: Source + 'static>(
    processor: &mut Processor,
    target: &Path,
    work: &WorkControl,
    open: impl Fn(Side, StreamProfile) -> Result<S> + Send + Sync + 'static,
    restore: impl FnOnce() -> Result<()>,
    control: &Arc<Control>,
) -> Result<Value> {
    require_new_target(target)?;
    let partial = staging(target).map_err(|e| e.to_string())?;
    let mut published = false;
    let result = (|| {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for (name, bytes) in processor.input_bytes() {
                file(&partial.join(name), |out| {
                    out.write_all(bytes).map_err(|e| e.to_string())
                })?;
            }
            execute(processor, &partial, work, open, control)
        }))
        .unwrap_or_else(|_| Err("live reconstruction coordinator panicked".into()));
        // Restore and verify group settings before publishing a successful scan.
        let result = combine(outcome, restore());
        let (mut report, editor) = result?;
        report["reported_stream_settings_restoration"] = json!("PASS");
        json_file(&partial.join("report.json"), &report)?;
        let mut artifacts = Vec::new();
        let mut names: Vec<String> = [
            "calibration.rapcal",
            "request.json",
            "scan-request.json",
            "left.raptor",
            "right.raptor",
            "report.json",
        ]
        .map(String::from)
        .to_vec();
        for index in 0..report["pairs"]
            .as_array()
            .ok_or("missing pair reports")?
            .len()
        {
            names.push(format!("pair-{index:04}.json"));
        }
        if report["geometry_exported"] == true {
            names.push("points.ply".into());
        }
        let actual: HashSet<_> = fs::read_dir(&partial)
            .map_err(|e| e.to_string())?
            .map(|entry| {
                entry
                    .map_err(|e| e.to_string())?
                    .file_name()
                    .into_string()
                    .map_err(|_| "unexpected non-UTF8 live artifact".to_string())
            })
            .collect::<Result<_>>()?;
        if actual != names.iter().cloned().collect() {
            return Err(format!(
                "unexpected live artifact set; partial retained at {}",
                partial.display()
            ));
        }
        for name in names {
            work.check()?;
            artifacts.push(fingerprint(&partial.join(name), work)?);
        }
        if let Some(mut editor) = editor {
            use scancraft_geometry::{
                live_references::LiveSession,
                sequence_references::{Artifact, Frame},
            };
            let pairs = report["pairs"]
                .as_array()
                .ok_or("missing live pairs")?
                .iter()
                .map(|pair| -> Result<Frame> {
                    Ok(Frame {
                        id: pair["pair_id"].as_u64().ok_or("invalid pair id")?,
                        left_index: usize::try_from(
                            pair["left_index"].as_u64().ok_or("invalid left index")?,
                        )
                        .map_err(|e| e.to_string())?,
                        right_index: usize::try_from(
                            pair["right_index"].as_u64().ok_or("invalid right index")?,
                        )
                        .map_err(|e| e.to_string())?,
                        integrated: pair["tracking"]["integrated"]
                            .as_bool()
                            .ok_or("missing pair integration decision")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let mut recorded_frames = [0; 2];
            for source in report["sources"].as_array().ok_or("missing live sources")? {
                let index = match source["interface"].as_u64() {
                    Some(0) => 0,
                    Some(2) => 1,
                    _ => return Err("invalid live interface".into()),
                };
                recorded_frames[index] = usize::try_from(
                    source["recorded_frames"]
                        .as_u64()
                        .ok_or("invalid raw count")?,
                )
                .map_err(|e| e.to_string())?;
            }
            let references: Vec<Artifact> = artifacts
                .iter()
                .cloned()
                .map(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
                .collect::<Result<_>>()?;
            editor.attach_live_session(LiveSession {
                schema_version: 1,
                directory_display: std::path::absolute(target)
                    .map_err(|e| e.to_string())?
                    .display()
                    .to_string(),
                session_id: processor.config().session_id.clone(),
                synthetic: matches!(
                    processor.source(),
                    scancraft_depth::MatchSource::SyntheticFixture
                ),
                recorded_frames,
                pairs,
                artifacts: references,
                source_device_binding_verified: false,
                synchronization_verified: false,
                physical_accuracy_qualified: false,
            })?;
            file(&partial.join("project.orproj"), |out| {
                save_project(&editor, out).map_err(|e| e.to_string())
            })?;
            artifacts.push(fingerprint(&partial.join("project.orproj"), work)?);
        }
        artifacts.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        json_file(
            &partial.join("manifest.json"),
            &json!({"schema_version":1,"artifacts":artifacts}),
        )?;
        File::open(&partial)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        work.check()?;

        publish(&partial, target).map_err(|e| e.to_string())?;
        published = true;
        let parent = target
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
            format!(
                "scan published at {}, parent sync failed: {e}",
                target.display()
            )
        })?;
        Ok(report)
    })();
    result.map_err(|e| {
        if published {
            e
        } else {
            format!("{e}; incomplete scan retained at {}", partial.display())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, path::PathBuf, sync::atomic::AtomicUsize};
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            Self(staging(&std::env::temp_dir().join("live-native-test")).unwrap())
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    struct Fake {
        frames: VecDeque<Frame>,
        closed: Arc<AtomicUsize>,
        fail_read: bool,
        fail_close: bool,
    }
    impl Source for Fake {
        fn context(&self) -> Result<CaptureContext> {
            Ok(CaptureContext {
                controls: json!({"synthetic":true}),
                calibration_status: "synthetic_fixture_not_a_scanner_capture".into(),
                ..Default::default()
            })
        }
        fn next(&mut self) -> Result<Option<Frame>> {
            if self.fail_read {
                Err("synthetic read failure".into())
            } else {
                Ok(self.frames.pop_front())
            }
        }
        fn statistics(&self) -> Value {
            json!({"synthetic":true})
        }
        fn close(self) -> Result<()> {
            self.closed.fetch_add(1, Ordering::Relaxed);
            if self.fail_close {
                Err("synthetic close failure".into())
            } else {
                Ok(())
            }
        }
    }
    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset")
    }
    fn image(side: Side) -> Vec<u8> {
        image::open(fixture().join(if side.index() == 0 {
            "left.png"
        } else {
            "right.png"
        }))
        .unwrap()
        .to_luma8()
        .into_raw()
    }
    fn frame(index: usize, side: Side) -> Frame {
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&(1000 + index as u32 * 33333).to_le_bytes());
        Frame {
            sequence: index as u32,
            host_timestamp_us: 1000 + index as u64 * 33333 + side.index() as u64 * 10,
            flags: 0,
            metadata,
            data: image(side),
        }
    }
    fn profile() -> StreamProfile {
        StreamProfile {
            interface: 0,
            width: 64,
            height: 32,
            fourcc: *b"GREY",
            fps: 30,
        }
    }
    fn processor(root: &Path, work: &WorkControl) -> Processor {
        let settings = root.join("settings.json");
        fs::write(&settings,serde_json::to_vec(&json!({"schema_version":1,"session_id":"synthetic-pipeline","fps":30,"maximum_frames_per_camera":8,
            "maximum_host_delta_us":1000,"tracking_samples":32,"voxel_mm":0.1,"maximum_voxels":1000,
            "tracking":{"max_correspondence_mm":10.0,"max_rms_mm":0.01,"minimum_overlap":0.8,"max_step_translation_mm":1.0,"max_step_rotation_rad":0.1,"iterations":40,"tolerance_mm":1e-8}})).unwrap()).unwrap();
        Processor::open(
            &fixture().join("calibration.rapcal"),
            &fixture().join("request.json"),
            &settings,
            super::super::InputMode::SyntheticFixture,
            work,
        )
        .unwrap()
    }
    #[test]
    fn saturated_processing_queue_keeps_every_raw_frame_and_reports_drops() {
        let t = Temp::new();
        let closed = Arc::new(AtomicUsize::new(0));
        let count = closed.clone();
        let (sender, _receiver) = mpsc::sync_channel(1);
        let stop = AtomicBool::new(false);
        let path = t.0.join("raw.raptor");
        let frames = (0..8).map(|i| frame(i, Side::Left)).collect();
        let report = record(profile(), &path, 8, &stop, sender, Side::Left, || {
            Ok(Fake {
                frames,
                closed: count,
                fail_read: false,
                fail_close: false,
            })
        })
        .unwrap();
        assert_eq!(report.recorded_frames, 8);
        assert_eq!(report.queued_frames, 1);
        assert_eq!(report.processing_queue_drops, 7);
        assert_eq!(closed.load(Ordering::Relaxed), 1);
        verify_recording(&path, 8, &WorkControl::default()).unwrap();
    }
    #[test]
    fn read_and_close_errors_are_both_preserved() {
        let t = Temp::new();
        let closed = Arc::new(AtomicUsize::new(0));
        let count = closed.clone();
        let (sender, _receiver) = mpsc::sync_channel(1);
        let error = record(
            profile(),
            &t.0.join("partial.raptor"),
            8,
            &AtomicBool::new(false),
            sender,
            Side::Left,
            || {
                Ok(Fake {
                    frames: VecDeque::new(),
                    closed: count,
                    fail_read: true,
                    fail_close: true,
                })
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("read failure"));
        assert!(error.contains("close failure"));
        assert_eq!(closed.load(Ordering::Relaxed), 1);
    }
    #[test]
    fn complete_injected_pipeline_restores_before_publish_and_validates_every_artifact() {
        let t = Temp::new();
        let work = WorkControl::default();
        let mut processor = processor(&t.0, &work);
        let closed = Arc::new(AtomicUsize::new(0));
        let count = closed.clone();
        let target = t.0.join("scan");
        let report = bundle(
            &mut processor,
            &target,
            &work,
            move |side, _| {
                Ok(Fake {
                    frames: VecDeque::from([frame(0, side)]),
                    closed: count.clone(),
                    fail_read: false,
                    fail_close: false,
                })
            },
            || {
                assert_eq!(closed.load(Ordering::Relaxed), 2);
                assert!(!target.exists());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(report["kind"], "synthetic_live_stereo_pipeline");
        assert_eq!(report["fused_points"], 3);
        assert_eq!(report["geometry_exported"], true);
        assert_eq!(report["synchronization_verified"], false);
        assert_eq!(report["source_binding_verified"], false);
        let manifest: Value =
            serde_json::from_slice(&fs::read(target.join("manifest.json")).unwrap()).unwrap();
        for artifact in manifest["artifacts"].as_array().unwrap() {
            let bytes = fs::read(target.join(artifact["name"].as_str().unwrap())).unwrap();
            assert_eq!(artifact["bytes"], bytes.len() as u64);
            assert_eq!(artifact["crc32"], crc32fast::hash(&bytes));
        }
        let project = fs::read(target.join("project.orproj")).unwrap();
        let editor = scancraft_geometry::formats::load_project(project.as_slice()).unwrap();
        assert_eq!(editor.cloud.points.len(), 3);
        let mut again = processor_for_retry(&t.0, &work);
        let opened = Arc::new(AtomicUsize::new(0));
        let attempted = opened.clone();
        assert!(
            bundle(
                &mut again,
                &target,
                &work,
                move |_, _| -> Result<Fake> {
                    attempted.fetch_add(1, Ordering::Relaxed);
                    Err("must not open".into())
                },
                || Ok(())
            )
            .is_err()
        );
        assert_eq!(opened.load(Ordering::Relaxed), 0);
        assert_eq!(fs::read(target.join("project.orproj")).unwrap(), project);
        assert_eq!(&project[..8], b"ORPROJ08");
        let lineage = editor.live_session().unwrap();
        assert_eq!(lineage.recorded_frames, [1, 1]);
        assert_eq!(lineage.pairs.len(), 1);
        assert!(lineage.pairs[0].integrated);
        assert_eq!(lineage.directory_display, target.display().to_string());
        assert_eq!(lineage.artifacts.len(), 8);
        let moved = t.0.join("moved-scan");
        fs::rename(&target, &moved).unwrap();
        let mut edited = editor;
        edited.trim(&[0], false).unwrap();
        let mut saved = Vec::new();
        save_project(&edited, &mut saved).unwrap();
        let reopened = scancraft_geometry::formats::load_project(saved.as_slice()).unwrap();
        let live = reopened.live_session().unwrap();
        assert!(
            crate::source_verification::verify_live(live, &moved, &work)
                .unwrap()
                .passed
        );
        let standalone = t.0.join("edited.orproj");
        fs::write(&standalone, &saved).unwrap();
        assert!(
            crate::source_verification::verify_project(&standalone, &moved, &work)
                .unwrap()
                .passed
        );
        for name in [
            "left.raptor",
            "calibration.rapcal",
            "report.json",
            "pair-0000.json",
        ] {
            let path = moved.join(name);
            let original = fs::read(&path).unwrap();
            let mut altered = original.clone();
            altered[0] ^= 1;
            fs::write(&path, altered).unwrap();
            assert!(
                !crate::source_verification::verify_live(live, &moved, &work)
                    .unwrap()
                    .passed
            );
            fs::write(&path, original).unwrap();
        }
        let raw = moved.join("left.raptor");
        let retained = t.0.join("retained-raw");
        fs::rename(&raw, &retained).unwrap();
        std::os::unix::fs::symlink(&retained, &raw).unwrap();
        assert!(
            !crate::source_verification::verify_live(live, &moved, &work)
                .unwrap()
                .passed
        );
        fs::remove_file(&raw).unwrap();
        fs::rename(&retained, &raw).unwrap();
        assert!(
            crate::source_verification::verify_live(live, &moved, &work)
                .unwrap()
                .passed
        );
    }
    fn processor_for_retry(root: &Path, work: &WorkControl) -> Processor {
        processor(root, work)
    }
    #[test]
    fn worker_failure_closes_both_sources_restores_and_retains_partial() {
        let t = Temp::new();
        let work = WorkControl::default();
        let mut p = processor(&t.0, &work);
        let closed = Arc::new(AtomicUsize::new(0));
        let count = closed.clone();
        let target = t.0.join("failed");
        let error = bundle(
            &mut p,
            &target,
            &work,
            move |side, _| {
                Ok(Fake {
                    frames: VecDeque::from([frame(0, side)]),
                    closed: count.clone(),
                    fail_read: side.index() == 0,
                    fail_close: false,
                })
            },
            || {
                assert_eq!(closed.load(Ordering::Relaxed), 2);
                Ok(())
            },
        )
        .unwrap_err();
        assert!(!target.exists());
        assert!(error.contains("read failure"));
        assert!(error.contains("incomplete scan retained at"));
    }
    #[test]
    fn coordinator_unwind_joins_worker_instead_of_detaching() {
        let closed = Arc::new(AtomicBool::new(false));
        let count = closed.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let signal = stop.clone();
        let outcome = std::panic::catch_unwind(|| {
            let handle = thread::spawn(move || {
                while !signal.load(Ordering::Relaxed) {
                    thread::yield_now();
                }
                count.store(true, Ordering::Relaxed);
                Err("synthetic cleanup diagnostic".into())
            });
            let _workers = Workers {
                stop,
                handles: vec![handle],
            };
            panic!("synthetic coordinator failure");
        });
        assert!(outcome.is_err());
        assert!(closed.load(Ordering::Relaxed));
    }
    #[test]
    fn restoration_failure_and_late_cancellation_never_publish() {
        for cancel in [false, true] {
            let t = Temp::new();
            let work = WorkControl::default();
            let mut p = processor(&t.0, &work);
            let target = t.0.join("scan");
            let closed = Arc::new(AtomicUsize::new(0));
            let count = closed.clone();
            let error = bundle(
                &mut p,
                &target,
                &work,
                move |side, _| {
                    Ok(Fake {
                        frames: VecDeque::from([frame(0, side)]),
                        closed: count.clone(),
                        fail_read: false,
                        fail_close: false,
                    })
                },
                || {
                    assert_eq!(closed.load(Ordering::Relaxed), 2);
                    if cancel {
                        work.cancel();
                        Ok(())
                    } else {
                        Err("synthetic restoration mismatch".into())
                    }
                },
            )
            .unwrap_err();
            assert!(!target.exists());
            assert!(error.contains("incomplete scan retained at"));
            assert!(error.contains(if cancel {
                "cancelled"
            } else {
                "restoration mismatch"
            }));
        }
    }
    #[test]
    fn absent_parent_and_dangling_target_fail_before_any_source_open() {
        let t = Temp::new();
        let work = WorkControl::default();
        let mut p = processor(&t.0, &work);
        let link = t.0.join("existing-link");
        std::os::unix::fs::symlink("missing", &link).unwrap();
        for target in [link, t.0.join("absent-parent/scan")] {
            let opened = Arc::new(AtomicUsize::new(0));
            let count = opened.clone();
            assert!(
                bundle(
                    &mut p,
                    &target,
                    &work,
                    move |_, _| -> Result<Fake> {
                        count.fetch_add(1, Ordering::Relaxed);
                        Err("must not open".into())
                    },
                    || Ok(())
                )
                .is_err()
            );
            assert_eq!(opened.load(Ordering::Relaxed), 0);
        }
    }
    #[test]
    fn blank_scan_publishes_raw_diagnostics_without_a_fabricated_model() {
        let t = Temp::new();
        let work = WorkControl::default();
        let mut p = processor(&t.0, &work);
        let target = t.0.join("blank");
        let closed = Arc::new(AtomicUsize::new(0));
        let count = closed.clone();
        let report = bundle(
            &mut p,
            &target,
            &work,
            move |side, _| {
                let mut sample = frame(0, side);
                sample.data.fill(0);
                Ok(Fake {
                    frames: VecDeque::from([sample]),
                    closed: count.clone(),
                    fail_read: false,
                    fail_close: false,
                })
            },
            || Ok(()),
        )
        .unwrap();
        assert_eq!(report["fused_points"], 0);
        assert_eq!(report["geometry_exported"], false);
        assert!(target.join("left.raptor").is_file());
        assert!(target.join("right.raptor").is_file());
        assert!(target.join("pair-0000.json").is_file());
        assert!(!target.join("project.orproj").exists());
        assert!(!target.join("points.ply").exists());
        assert_eq!(closed.load(Ordering::Relaxed), 2);
    }
    #[test]
    fn complete_raw_replay_rejects_repeated_pts_even_when_processing_delivery_dropped_it() {
        let t = Temp::new();
        let closed = Arc::new(AtomicUsize::new(0));
        let (sender, _receiver) = mpsc::sync_channel(1);
        let frames = (0..9)
            .map(|i| {
                let mut f = frame(i, Side::Left);
                f.metadata[12..16]
                    .copy_from_slice(&(i as u32).wrapping_mul(0x60000000).to_le_bytes());
                f
            })
            .collect();
        let path = t.0.join("ambiguous.raptor");
        let report = record(
            profile(),
            &path,
            9,
            &AtomicBool::new(false),
            sender,
            Side::Left,
            || {
                Ok(Fake {
                    frames,
                    closed,
                    fail_read: false,
                    fail_close: false,
                })
            },
        )
        .unwrap();
        assert_eq!(report.recorded_frames, 9);
        assert_eq!(report.processing_queue_drops, 8);
        let error = verify_recording(&path, 9, &WorkControl::default()).unwrap_err();
        assert!(error.contains("repeated raw PTS"));
    }
    #[test]
    fn source_read_panic_still_attempts_explicit_close() {
        struct Panicking(Fake);
        impl Source for Panicking {
            fn context(&self) -> Result<CaptureContext> {
                self.0.context()
            }
            fn next(&mut self) -> Result<Option<Frame>> {
                panic!("synthetic source panic")
            }
            fn statistics(&self) -> Value {
                self.0.statistics()
            }
            fn close(self) -> Result<()> {
                self.0.close()
            }
        }
        let t = Temp::new();
        let closed = Arc::new(AtomicUsize::new(0));
        let count = closed.clone();
        let (sender, _receiver) = mpsc::sync_channel(1);
        let error = record(
            profile(),
            &t.0.join("partial.raptor"),
            1,
            &AtomicBool::new(false),
            sender,
            Side::Left,
            || {
                Ok(Panicking(Fake {
                    frames: VecDeque::new(),
                    closed: count,
                    fail_read: false,
                    fail_close: false,
                }))
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("panicked"));
        assert_eq!(closed.load(Ordering::Relaxed), 1);
    }
}
