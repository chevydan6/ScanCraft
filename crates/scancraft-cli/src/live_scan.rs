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
    scancraft_acquisition::laser::scan(geometry, request, target, &work, &stop, |_, _| {
        scancraft_workflow::laser_stream::bundle::PreviewStatus::NotRequested
    })
}
