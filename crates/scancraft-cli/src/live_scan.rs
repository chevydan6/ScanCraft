//! Process-local signal ownership for native acquisition shared with the GUI.
use crate::termination;
use scancraft_geometry::work::WorkControl;
use serde_json::Value;
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

/// Finite native IR acquisition. No illumination, mirror, gain, firmware or calibration writes.
pub fn run(calibration: &Path, request: &Path, settings: &Path, target: &Path) -> Result<Value> {
    let _termination = termination::Guard::install().map_err(|e| e.to_string())?;
    let work = Arc::new(WorkControl::default());
    let control = Arc::new(scancraft_workflow::live_stereo::control::Control::default());
    let stop = control.clone();
    let cancellation = work.clone();
    let _monitor = monitor(move || {
        cancellation.cancel();
        stop.request_stop();
    })?;
    scancraft_acquisition::scan(calibration, request, settings, target, &work, &control)
}

fn monitor(notify: impl FnOnce() + Send + 'static) -> Result<CancellationMonitor> {
    let finished = Arc::new(AtomicBool::new(false));
    let done = finished.clone();
    let handle = thread::Builder::new()
        .name("scancraft-live-cancel".into())
        .spawn(move || {
            while !done.load(Ordering::Relaxed) {
                if termination::check().is_err() {
                    notify();
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(CancellationMonitor {
        finished,
        handle: Some(handle),
    })
}
/// Signal requests graceful stop: retain journals and finish cleanup/verification.
pub fn run_laser(geometry: &Path, request: &Path, target: &Path) -> Result<Value> {
    let _termination = termination::Guard::install().map_err(|e| e.to_string())?;
    let work = Arc::new(WorkControl::default());
    let stop = Arc::new(AtomicBool::new(false));
    let requested = stop.clone();
    let _monitor = monitor(move || requested.store(true, Ordering::Relaxed))?;
    scancraft_acquisition::laser::scan(geometry, request, target, &work, &stop, |_| {
        scancraft_workflow::laser_stream::bundle::PreviewStatus::NotRequested
    })
}

/// Read-only completed-capture inspection with the existing scoped signal monitor.
pub fn inspect_laser_capture(directory: &Path, index: u64) -> Result<Value> {
    let _termination = termination::Guard::install().map_err(|e| e.to_string())?;
    let work = Arc::new(WorkControl::default());
    let cancellation = work.clone();
    let _monitor = monitor(move || cancellation.cancel())?;
    let loaded = scancraft_workflow::laser_stream::bundle::load_pair(directory, index, &work)
        .map_err(|e| e.to_string())?;
    Ok(loaded.report)
}

/// Summarize only after all provisional pairs and final source checks succeed.
pub fn inspect_all_laser_pairs(directory: &Path) -> Result<Value> {
    let _termination = termination::Guard::install().map_err(|e| e.to_string())?;
    let work = Arc::new(WorkControl::default());
    let cancellation = work.clone();
    let _monitor = monitor(move || cancellation.cancel())?;
    let mut candidates = 0_u64;
    let mut empty_pairs = 0_u64;
    let mut processing_failures = 0_u64;
    let pairs =
        scancraft_workflow::laser_stream::bundle::visit_pairs(directory, &work, |_, pair| {
            candidates += pair.positions.len() as u64;
            empty_pairs += u64::from(pair.positions.is_empty());
            processing_failures += u64::from(!pair.report["stored_pair_error"].is_null());
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    work.check()?;
    Ok(
        serde_json::json!({"schema_version":1,"kind":"complete_laser_capture_inspection",
        "pairs":pairs,"candidate_points":candidates,"empty_pairs":empty_pairs,
        "stored_processing_failures":processing_failures,"artifact_integrity_verified":true,
        "all_retained_frame_bindings_verified":true,"bundled_recording_integrity_verified":true,
        "numerical_recomputation_verified":false,"geometry_binding_verified":false,
        "units_verified":false,"physical_accuracy_qualified":false,
        "tracking_verified":false,"geometry_exported":false}),
    )
}
