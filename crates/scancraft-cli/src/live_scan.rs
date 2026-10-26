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
    scancraft_acquisition::scan(
        calibration,
        request,
        settings,
        target,
        &work,
        &Arc::new(scancraft_workflow::live_stereo::control::Control::default()),
    )
}
