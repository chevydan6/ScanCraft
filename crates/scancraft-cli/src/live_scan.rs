//! Native adapter for the reusable live stereo runner; no illumination/firmware writes.
use crate::termination;
use scancraft_core::Frame;
use scancraft_device::{Capture, SessionLock, stream_group::SettingsLease};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::CaptureContext;
use scancraft_workflow::live_stereo::{Processor, runner::Source};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
type Result<T> = std::result::Result<T, String>;
struct Native(Capture);
impl Source for Native {
    fn context(&self) -> Result<CaptureContext> {
        let mut context = CaptureContext::default();
        if let Some(snapshot) = self.0.recording_observations() {
            context
                .add_capture_observations(snapshot)
                .map_err(|e| e.to_string())?;
        }
        Ok(context)
    }
    fn next(&mut self) -> Result<Option<Frame>> {
        self.0.next_frame().map(Some).map_err(|e| e.to_string())
    }
    fn statistics(&self) -> Value {
        json!({"rejected_buffers":self.0.rejected,"metadata_missing":self.0.metadata_missing,
        "sequence_gaps":self.0.sequence_drops,"duplicate_sensor_pts":self.0.duplicate_sensor_pts,
        "rejection_samples":self.0.rejection_samples,"ongoing_rejection_samples":self.0.ongoing_rejection_samples})
    }
    fn close(self) -> Result<()> {
        self.0.close().map_err(|e| e.to_string())
    }
}
fn combine<T>(result: Result<T>, cleanup: Result<()>) -> Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(e), Err(c)) => Err(format!("{e}; cleanup: {c}")),
    }
}
struct CancellationMonitor {
    finished: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}
impl Drop for CancellationMonitor {
    fn drop(&mut self) {
        self.finished.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Finite native acquisition. No illumination, mirror, gain, firmware or calibration writes.
pub fn run(calibration: &Path, request: &Path, settings: &Path, target: &Path) -> Result<Value> {
    let _termination = termination::Guard::install().map_err(|e| e.to_string())?;
    let work = Arc::new(WorkControl::default());
    let finished = Arc::new(AtomicBool::new(false));
    let cancellation = work.clone();
    let done = finished.clone();
    let monitor = thread::Builder::new()
        .name("scancraft-live-cancel".into())
        .spawn(move || {
            while !done.load(Ordering::Relaxed) {
                if termination::check().is_err() {
                    cancellation.cancel();
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        })
        .map_err(|e| e.to_string())?;
    let _monitor = CancellationMonitor {
        finished,
        handle: Some(monitor),
    };
    run_inner(calibration, request, settings, target, &work)
}
fn run_inner(
    calibration: &Path,
    request: &Path,
    settings: &Path,
    target: &Path,
    work: &WorkControl,
) -> Result<Value> {
    scancraft_workflow::live_stereo::runner::require_new_target(target)?;
    let mut processor = Processor::open(
        calibration,
        request,
        settings,
        scancraft_workflow::live_stereo::InputMode::PhysicalCapture,
        work,
    )
    .map_err(|e| e.to_string())?;
    termination::check()?;
    let _lock = SessionLock::acquire().map_err(|e| e.to_string())?;
    let mut lease = SettingsLease::capture(&[0, 2]).map_err(|e| e.to_string())?;
    let result = scancraft_workflow::live_stereo::runner::bundle(
        &mut processor,
        target,
        work,
        |_, profile| {
            Capture::open_recording(profile)
                .map(Native)
                .map_err(|e| e.to_string())
        },
        || lease.restore().map_err(|e| e.to_string()),
    );
    combine(result, lease.restore().map_err(|e| e.to_string()))
}
