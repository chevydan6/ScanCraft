//! Native source/settings ownership for unqualified laser candidate diagnostics.
//! Preserves illumination by default; fixed line-boolean trial is explicit and
//! restored. No mode/energy/gain/mirror/firmware/installed-calibration writes.
use super::{Result, capture_context, capture_statistics, combine};
use scancraft_core::Frame;
use scancraft_device::{
    Capture, SessionLock, control::line_trial::ScopedLine, stream_group::SettingsLease,
};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::CaptureContext;
use scancraft_workflow::{
    laser_stream::{
        StopToken,
        bundle::{Plan, PreviewInput, PreviewStatus},
    },
    live_stereo::runner::{Source, require_new_target},
};
use serde_json::Value;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
struct Native {
    capture: Capture,
    stop: StopToken,
    declaration: Value,
}
impl Source for Native {
    fn context(&self) -> Result<CaptureContext> {
        let mut context = capture_context(&self.capture)?;
        if context.controls.is_null() {
            context.controls = serde_json::json!({});
        }
        context
            .controls
            .as_object_mut()
            .ok_or("capture controls must be an object")?
            .insert("laser_source_declaration".into(), self.declaration.clone());
        Ok(context)
    }
    fn next(&mut self) -> Result<Option<Frame>> {
        self.capture
            .next_frame_until(|| self.stop.stop_requested())
            .map_err(|e| e.to_string())
    }
    fn statistics(&self) -> Value {
        capture_statistics(&self.capture)
    }
    fn close(self) -> Result<()> {
        self.capture.close().map_err(|e| e.to_string())
    }
}
/// Geometry declarations are accepted as unqualified numerical hypotheses. This
/// only enables a fixed line boolean when explicitly requested, never exports
/// metric geometry. A live caller must run
/// the complete operation in its device-owning helper and supervise it honestly.
pub fn scan(
    geometry: &Path,
    request: &Path,
    target: &Path,
    work: &Arc<WorkControl>,
    stop: &Arc<AtomicBool>,
    preview: impl FnMut(PreviewInput<'_>) -> PreviewStatus,
) -> Result<Value> {
    require_new_target(target)?;
    let plan = Plan::open(geometry, request, work)?;
    if ![5, 10, 15, 20, 25, 30, 40, 50].contains(&plan.fps()) {
        return Err("native laser capture requires an enumerated IR frame rate".into());
    }
    let illumination = illumination(plan.source_options())?;
    plan.admit_target(target, work)?;
    work.check()?;
    if stop.load(Ordering::Relaxed) {
        return Err("laser capture stopped before acquiring device ownership".into());
    }
    let lock = SessionLock::acquire().map_err(|e| e.to_string())?;
    let mut lease = SettingsLease::capture(&plan.interfaces()).map_err(|e| e.to_string())?;
    let mut line = match illumination {
        Illumination::Preserve => None,
        Illumination::LineBooleanTrial => match ScopedLine::enable(&lock) {
            Ok(line) => Some(line),
            Err(e) => {
                return combine(
                    Err(e.to_string()),
                    lease.restore().map_err(|e| e.to_string()),
                );
            }
        },
    };
    let declaration = if let Some(line) = &line {
        serde_json::json!({"illumination":"line_boolean_trial","property_id":151,"state_before_capture":line.state(),"enabled_before_open_readback":true,"physical_emission_verified":false,"wavelength_verified":false})
    } else {
        serde_json::json!({"illumination":"preserve"})
    };
    let result = plan.capture(
        target,
        work,
        stop,
        move |_, profile, token| {
            Capture::open_recording(profile)
                .map(|capture| Native {
                    capture,
                    stop: token,
                    declaration: declaration.clone(),
                })
                .map_err(|e| e.to_string())
        },
        || {
            let verified = line
                .as_ref()
                .map(|v| v.verify_enabled())
                .transpose()
                .map(|_| ())
                .map_err(|e| e.to_string());
            let line_restore = line
                .as_mut()
                .map(|v| v.restore())
                .transpose()
                .map(|_| ())
                .map_err(|e| e.to_string());
            let stream_restore = lease.restore().map_err(|e| e.to_string());
            combine(combine(verified, line_restore), stream_restore)
        },
        preview,
    );
    let line_restore = line
        .as_mut()
        .map(|v| v.restore())
        .transpose()
        .map(|_| ())
        .map_err(|e| e.to_string());
    let stream_restore = lease.restore().map_err(|e| e.to_string());
    combine(combine(result, line_restore), stream_restore)
}

#[derive(Debug, PartialEq)]
enum Illumination {
    Preserve,
    LineBooleanTrial,
}
fn illumination(value: &Value) -> Result<Illumination> {
    if value.is_null() {
        return Ok(Illumination::Preserve);
    }
    let object = value
        .as_object()
        .filter(|v| v.len() == 1)
        .ok_or("native laser source_options requires only an illumination declaration")?;
    match object.get("illumination").and_then(Value::as_str) {
        Some("preserve") => Ok(Illumination::Preserve),
        Some("line_boolean_trial") => Ok(Illumination::LineBooleanTrial),
        _ => Err(
            "native laser source_options illumination must be preserve or line_boolean_trial"
                .into(),
        ),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_fixed_line_trial_can_request_a_property_change() {
        assert_eq!(illumination(&Value::Null).unwrap(), Illumination::Preserve);
        assert_eq!(
            illumination(&serde_json::json!({"illumination":"preserve"})).unwrap(),
            Illumination::Preserve
        );
        assert_eq!(
            illumination(&serde_json::json!({"illumination":"line_boolean_trial"})).unwrap(),
            Illumination::LineBooleanTrial
        );
        for value in [
            serde_json::json!({}),
            serde_json::json!({"illumination":"laser_energy"}),
            serde_json::json!({"illumination":true}),
            serde_json::json!({"illumination":"line_boolean_trial","gain":100}),
            serde_json::json!({"property_id":151,"value":1}),
        ] {
            assert!(illumination(&value).is_err());
        }
    }
}
