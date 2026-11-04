//! Native source/settings ownership for unqualified laser candidate diagnostics.
//! Preserves illumination by default; fixed line-boolean trial is explicit and
//! restored. Optional standard IR gain is also scoped and read back. No
//! mode/energy/mirror/firmware/installed-calibration writes.
use super::{Result, capture_context, capture_statistics, combine};
use scancraft_core::Frame;
use scancraft_device::{
    Capture, SessionLock,
    control::line_trial::ScopedLine,
    ir_gain::{self, ScopedIrGain},
    stream_group::SettingsLease,
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
mod cleanup;
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
/// only changes the fixed line boolean or standard IR gain when explicitly
/// requested, never exports metric geometry. A live caller must run
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
    let options = source_options(plan.source_options())?;
    plan.admit_target(target, work)?;
    work.check()?;
    if stop.load(Ordering::Relaxed) {
        return Err("laser capture stopped before acquiring device ownership".into());
    }
    let lock = SessionLock::acquire().map_err(|e| e.to_string())?;
    let gain = options
        .ir_gain_raw
        .map(|raw| ScopedIrGain::capture(&lock, raw))
        .transpose()
        .map_err(|e| e.to_string())?;
    let lease = SettingsLease::capture(&plan.interfaces()).map_err(|e| e.to_string())?;
    let mut settings = cleanup::Settings::new(lease, gain);
    if let Some(gain) = settings.gain.as_mut()
        && let Err(e) = gain.apply_requested()
    {
        return combine(Err(e.to_string()), settings.finish());
    }
    let mut line = match options.illumination {
        Illumination::Preserve => None,
        Illumination::LineBooleanTrial => match ScopedLine::enable(&lock) {
            Ok(line) => Some(line),
            Err(e) => {
                return combine(Err(e.to_string()), settings.finish());
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
            let interface = profile.interface;
            let capture = Capture::open_recording(profile).map_err(|e| e.to_string())?;
            let mut declaration = declaration.clone();
            if let Some(requested) = options.ir_gain_raw {
                let observed = ir_gain::read(interface)
                    .map_err(|e| e.to_string())
                    .and_then(|value| gain_observation(interface, requested, value));
                match observed {
                    Ok(value) => declaration["ir_gain_trial"] = value,
                    Err(e) => return combine(Err(e), capture.close().map_err(|e| e.to_string())),
                }
            }
            Ok(Native {
                capture,
                stop: token,
                declaration,
            })
        },
        || {
            let verified = line
                .as_ref()
                .map(|v| v.verify_enabled())
                .transpose()
                .map(|_| ())
                .map_err(|e| e.to_string());
            let gain_verified = verify_gain(options.ir_gain_raw, |interface| {
                ir_gain::read(interface).map_err(|e| e.to_string())
            });
            let line_restore = line
                .as_mut()
                .map(|v| v.restore())
                .transpose()
                .map(|_| ())
                .map_err(|e| e.to_string());
            let settings_restore = settings.restore();
            combine(
                combine(combine(verified, gain_verified), line_restore),
                settings_restore,
            )
        },
        preview,
    );
    let line_restore = line
        .as_mut()
        .map(|v| v.restore())
        .transpose()
        .map(|_| ())
        .map_err(|e| e.to_string());
    combine(combine(result, line_restore), settings.finish())
}

fn gain_observation(interface: u8, requested: i32, observed: i32) -> Result<Value> {
    if observed != requested {
        return Err(format!(
            "interface {interface} gain readback differs: requested {requested}, observed {observed}"
        ));
    }
    Ok(
        serde_json::json!({"interface":interface,"requested_raw":requested,
        "observed_after_stream_start_raw":observed,"units":"vendor raw; physical conversion unverified"}),
    )
}

fn verify_gain(requested: Option<i32>, mut read: impl FnMut(u8) -> Result<i32>) -> Result<()> {
    if let Some(requested) = requested {
        let mut errors = Vec::new();
        for interface in [0, 2] {
            if let Err(e) =
                read(interface).and_then(|value| gain_observation(interface, requested, value))
            {
                errors.push(format!("end-of-capture gain verification: {e}"));
            }
        }
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Illumination {
    Preserve,
    LineBooleanTrial,
}
#[derive(Debug, PartialEq, Clone, Copy)]
struct SourceOptions {
    illumination: Illumination,
    ir_gain_raw: Option<i32>,
}
fn source_options(value: &Value) -> Result<SourceOptions> {
    if value.is_null() {
        return Ok(SourceOptions {
            illumination: Illumination::Preserve,
            ir_gain_raw: None,
        });
    }
    let object = value
        .as_object()
        .filter(|v| v.keys().all(|k| k == "illumination" || k == "ir_gain_raw"))
        .ok_or("native laser source_options permits illumination and optional ir_gain_raw only")?;
    let illumination =
        match object.get("illumination").and_then(Value::as_str) {
            Some("preserve") => Illumination::Preserve,
            Some("line_boolean_trial") => Illumination::LineBooleanTrial,
            _ => return Err(
                "native laser source_options illumination must be preserve or line_boolean_trial"
                    .into(),
            ),
        };
    let ir_gain_raw = object
        .get("ir_gain_raw")
        .map(|value| {
            value
                .as_i64()
                .filter(|v| (1000..=15000).contains(v))
                .map(|v| v as i32)
                .ok_or_else(|| "ir_gain_raw must be an integer from 1000 through 15000".to_string())
        })
        .transpose()?;
    Ok(SourceOptions {
        illumination,
        ir_gain_raw,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_source_options_preserve_defaults_and_require_explicit_known_controls() {
        assert_eq!(
            source_options(&Value::Null).unwrap(),
            SourceOptions {
                illumination: Illumination::Preserve,
                ir_gain_raw: None
            }
        );
        assert_eq!(
            source_options(&serde_json::json!({"illumination":"preserve"})).unwrap(),
            SourceOptions {
                illumination: Illumination::Preserve,
                ir_gain_raw: None
            }
        );
        assert_eq!(
            source_options(&serde_json::json!({"illumination":"line_boolean_trial"})).unwrap(),
            SourceOptions {
                illumination: Illumination::LineBooleanTrial,
                ir_gain_raw: None
            }
        );
        for value in [
            serde_json::json!({}),
            serde_json::json!({"illumination":"laser_energy"}),
            serde_json::json!({"illumination":true}),
            serde_json::json!({"illumination":"line_boolean_trial","gain":100}),
            serde_json::json!({"property_id":151,"value":1}),
        ] {
            assert!(source_options(&value).is_err());
        }
    }
    #[test]
    fn explicit_gain_rejects_invalid_values_and_retains_exact_raw_declaration() {
        for mode in ["preserve", "line_boolean_trial"] {
            for raw in [1000, 8000, 15000] {
                let options =
                    source_options(&serde_json::json!({"illumination":mode,"ir_gain_raw":raw}))
                        .unwrap();
                assert_eq!(options.ir_gain_raw, Some(raw));
            }
            for value in [
                Value::Null,
                serde_json::json!(999),
                serde_json::json!(15001),
                serde_json::json!(8000.0),
                serde_json::json!("8000"),
                serde_json::json!(true),
                serde_json::json!(-1),
                serde_json::json!(u64::MAX),
            ] {
                assert!(
                    source_options(&serde_json::json!({"illumination":mode,"ir_gain_raw":value}))
                        .is_err()
                );
            }
        }
        assert!(source_options(&serde_json::json!({"ir_gain_raw":8000})).is_err());
    }
    #[test]
    fn gain_readback_mismatch_is_rejected_and_end_check_checks_both_interfaces() {
        let observation = gain_observation(2, 15000, 15000).unwrap();
        assert_eq!(observation["observed_after_stream_start_raw"], 15000);
        assert_eq!(observation["interface"], 2);
        assert!(gain_observation(0, 15000, 1000).is_err());
        verify_gain(None, |_| panic!("preserve must not read a gain control")).unwrap();
        let mut read_interfaces = Vec::new();
        let error = verify_gain(Some(15000), |interface| {
            read_interfaces.push(interface);
            if interface == 0 {
                Err("disconnected".into())
            } else {
                Ok(1000)
            }
        })
        .unwrap_err();
        assert_eq!(read_interfaces, [0, 2]);
        assert!(error.contains("disconnected") && error.contains("requested 15000, observed 1000"));
        verify_gain(Some(15000), |_| Ok(15000)).unwrap();
    }
}
