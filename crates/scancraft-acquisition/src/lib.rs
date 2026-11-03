//! Native capture/session/settings ownership shared by desktop and CLI.
//! Processing stays device-independent in scancraft-workflow. The laser entry
//! supports an explicit scoped line-boolean trial; firmware, mode, energy, mirror,
//! gain and installed calibration are not changed.
pub mod laser;
use scancraft_core::Frame;
use scancraft_device::{Capture, SessionLock, stream_group::SettingsLease};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::CaptureContext;
use scancraft_workflow::live_stereo::{Processor, control::Control, runner::Source};
use serde_json::{Value, json};
use std::{path::Path, sync::Arc};
type Result<T> = std::result::Result<T, String>;
struct Native(Capture, Arc<Control>);
impl Source for Native {
    fn context(&self) -> Result<CaptureContext> {
        capture_context(&self.0)
    }
    fn next(&mut self) -> Result<Option<Frame>> {
        self.next_until(&|| false)
    }
    fn next_until(&mut self, cancelled: &dyn Fn() -> bool) -> Result<Option<Frame>> {
        self.0
            .next_frame_until(|| cancelled() || self.1.stop_requested())
            .map_err(|e| e.to_string())
    }
    fn statistics(&self) -> Value {
        capture_statistics(&self.0)
    }
    fn close(self) -> Result<()> {
        self.0.close().map_err(|e| e.to_string())
    }
}
fn capture_context(capture: &Capture) -> Result<CaptureContext> {
    let mut context = CaptureContext::default();
    if let Some(snapshot) = capture.recording_observations() {
        context
            .add_capture_observations(snapshot)
            .map_err(|e| e.to_string())?;
    }
    Ok(context)
}
fn capture_statistics(capture: &Capture) -> Value {
    json!({"rejected_buffers":capture.rejected,"rejected_before_first_valid":capture.rejected_before_first_valid,"rejected_after_first_valid":capture.rejected_after_first_valid,"rejection_counter_unit":"V4L2 buffers; not necessarily physical exposures","stopped_incomplete_video_buffers":capture.stopped_incomplete_video_buffers,"metadata_missing":capture.metadata_missing,
        "sequence_gaps":capture.sequence_drops,"duplicate_sensor_pts":capture.duplicate_sensor_pts,
        "rejection_samples":capture.rejection_samples,"ongoing_rejection_samples":capture.ongoing_rejection_samples})
}
fn combine<T>(result: Result<T>, cleanup: Result<()>) -> Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(e), Err(c)) => Err(format!("{e}; cleanup: {c}")),
    }
}
/// Run an explicitly configured native session; declarations do not attest hardware binding.
pub fn scan(
    calibration: &Path,
    request: &Path,
    settings: &Path,
    target: &Path,
    work: &WorkControl,
    control: &Arc<Control>,
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
    processor.admit_storage(target)?;
    work.check()?;
    let _lock = SessionLock::acquire().map_err(|e| e.to_string())?;
    let mut lease = SettingsLease::capture(&[0, 2]).map_err(|e| e.to_string())?;
    let source_control = control.clone();
    let result = scancraft_workflow::live_stereo::runner::bundle_controlled(
        &mut processor,
        target,
        work,
        move |_, profile| {
            Capture::open_recording(profile)
                .map(|capture| Native(capture, source_control.clone()))
                .map_err(|e| e.to_string())
        },
        || lease.restore().map_err(|e| e.to_string()),
        control,
    );
    combine(result, lease.restore().map_err(|e| e.to_string()))
}
