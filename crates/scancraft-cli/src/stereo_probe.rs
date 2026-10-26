//! Timestamp diagnostics only; no claim of exposure synchronization or matching image content.
use crate::{
    output::{publish, staging},
    termination,
};
use scancraft_core::{StreamProfile, uvc_pts};
use scancraft_device::{Capture, SessionLock};
use scancraft_recording::{Reader, Writer};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Instant,
};
#[derive(Clone)]
struct Sample {
    sequence: u32,
    host_us: u64,
    pts: u32,
}
struct Report {
    interface: u8,
    requested_fps: u32,
    samples: Vec<Sample>,
    rejected: u64,
    rejected_before_first_valid: u64,
    rejected_after_first_valid: u64,
    missing: u64,
    gaps: u64,
    duplicate_pts: u64,
    rejection_samples: Vec<scancraft_device::RejectedBufferInfo>,
    ongoing_rejection_samples: Vec<scancraft_device::RejectedBufferInfo>,
}
#[derive(Clone, Copy)]
enum IlluminationTrial {
    None,
    Flood,
    Line,
}
impl IlluminationTrial {
    fn key(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Flood => Some("flood_trial"),
            Self::Line => Some("line_laser_trial"),
        }
    }
}
fn capture(
    interface: u8,
    requested_fps: u32,
    count: usize,
    cancel: &AtomicBool,
    output: Option<PathBuf>,
    gain: Option<i32>,
    illumination: IlluminationTrial,
) -> Result<Report, String> {
    termination::check()?;
    let profile = StreamProfile {
        interface,
        width: 1920,
        height: 1200,
        fourcc: *b"GREY",
        fps: requested_fps,
    };
    let mut camera = if output.is_some() {
        Capture::open_recording(profile.clone())
    } else {
        Capture::open(profile.clone())
    }
    .map_err(|e| e.to_string())?;
    termination::check()?;
    let mut context = scancraft_recording::CaptureContext::default();
    if let Some(requested) = gain {
        let observed = scancraft_device::ir_gain::read(interface).map_err(|e| e.to_string())?;
        if observed != requested {
            return Err(format!(
                "interface {interface} gain changed during stream startup: requested {requested}, observed {observed}"
            ));
        }
        context.controls = json!({"gain_trial":{"requested_raw":requested,"observed_after_stream_start_raw":observed,"units":"vendor raw; physical conversion unverified"}});
    }
    if let Some(key) = illumination.key() {
        if context.controls.is_null() {
            context.controls = json!({});
        }
        context.controls[key] =
            json!({"reported_enabled_before_stream_setup":true,"physical_emission_verified":false});
    }
    if let Some(observations) = camera.recording_observations() {
        context
            .add_capture_observations(observations)
            .map_err(|e| e.to_string())?;
    }
    let mut writer = output
        .map(|path| -> Result<_, String> {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| e.to_string())?;
            Writer::with_context(BufWriter::new(file), profile.clone(), context)
                .map_err(|e| e.to_string())
        })
        .transpose()?;
    let mut samples = Vec::with_capacity(count);
    let capture_start = Instant::now();
    for _ in 0..count {
        termination::check()?;
        if cancel.load(Ordering::Relaxed) {
            return Err("stereo probe cancelled because another worker failed".into());
        }
        let acquired = camera.next_frame();
        termination::check()?;
        let frame = acquired.map_err(|e| e.to_string())?;
        let pts = uvc_pts(&frame.metadata)
            .map_err(str::to_string)?
            .ok_or("missing sensor PTS")?;
        if let Some(writer) = writer.as_mut() {
            writer.push(&frame).map_err(|e| e.to_string())?;
        }
        samples.push(Sample {
            sequence: frame.sequence,
            host_us: frame.host_timestamp_us,
            pts,
        });
        if samples.len().is_multiple_of(500) {
            eprintln!(
                "{}",
                json!({"event":"stereo_progress","interface":interface,"accepted_buffers":samples.len(),"target_buffers":count,"rejected_buffers":camera.rejected,"rejected_before_first_valid":camera.rejected_before_first_valid,"rejected_after_first_valid":camera.rejected_after_first_valid,"sequence_gaps":camera.sequence_drops,"elapsed_seconds":capture_start.elapsed().as_secs_f64()})
            );
        }
    }
    let report = Report {
        interface,
        requested_fps,
        samples,
        rejected: camera.rejected,
        rejected_before_first_valid: camera.rejected_before_first_valid,
        rejected_after_first_valid: camera.rejected_after_first_valid,
        missing: camera.metadata_missing,
        gaps: camera.sequence_drops,
        duplicate_pts: camera.duplicate_sensor_pts,
        rejection_samples: camera.rejection_samples.clone(),
        ongoing_rejection_samples: camera.ongoing_rejection_samples.clone(),
    };
    camera.close().map_err(|e| e.to_string())?;
    if let Some(writer) = writer {
        let out = writer.finish().map_err(|e| e.to_string())?;
        out.get_ref().sync_all().map_err(|e| e.to_string())?;
    }
    Ok(report)
}
fn summary(r: &Report) -> Value {
    let first = r.samples.first().unwrap();
    let last = r.samples.last().unwrap();
    let ticks: u64 = r
        .samples
        .windows(2)
        .map(|pair| u64::from(pair[1].pts.wrapping_sub(pair[0].pts)))
        .sum();
    let host = last.host_us.saturating_sub(first.host_us);
    json!({"interface":r.interface,"requested_fps":r.requested_fps,"valid_frames":r.samples.len(),"rejected_frames":r.rejected,"rejected_buffers":r.rejected,"rejected_before_first_valid":r.rejected_before_first_valid,"rejected_after_first_valid":r.rejected_after_first_valid,"rejection_counter_unit":"V4L2 buffers; not necessarily physical exposures","metadata_missing":r.missing,"sequence_gaps":r.gaps,"duplicate_sensor_pts":r.duplicate_pts,"rejection_samples":r.rejection_samples,"ongoing_rejection_samples":r.ongoing_rejection_samples,"first_sequence":first.sequence,"last_sequence":last.sequence,"first_pts":first.pts,"last_pts":last.pts,"accepted_host_span_seconds":host as f64/1e6,"accepted_buffer_rate_hz":if host>0 {Some((r.samples.len()-1) as f64*1e6/host as f64)}else{None},"observed_ticks_per_host_microsecond":if host>0{Some(ticks as f64/host as f64)}else{None},"samples":r.samples.iter().map(|s|json!({"sequence":s.sequence,"host_us":s.host_us,"sensor_pts":s.pts})).collect::<Vec<_>>()})
}
fn nearest_host_deltas(
    left: &[Sample],
    right: &[Sample],
    overlap_start: u64,
    overlap_end: u64,
) -> Vec<(i128, i32)> {
    if right.is_empty() {
        return Vec::new();
    }
    let mut deltas = Vec::new();
    let mut j = 0;
    for l in left
        .iter()
        .filter(|s| s.host_us >= overlap_start && s.host_us <= overlap_end)
    {
        while j + 1 < right.len()
            && right[j + 1].host_us.abs_diff(l.host_us) < right[j].host_us.abs_diff(l.host_us)
        {
            j += 1;
        }
        let r = &right[j];
        deltas.push((
            r.host_us as i128 - l.host_us as i128,
            r.pts.wrapping_sub(l.pts) as i32,
        ));
    }
    deltas
}

fn validate_request(count: usize, fps: u32) -> Result<(), String> {
    if !(10..=30000).contains(&count) || ![5, 10, 15, 20, 25, 30, 40, 50].contains(&fps) {
        return Err("stereo request requires 10..30000 buffers and an enumerated IR rate (5,10,15,20,25,30,40,50)".into());
    }
    Ok(())
}
pub fn run(count: usize, fps: u32) -> Result<Value, String> {
    validate_request(count, fps)?;
    let _termination = termination::Guard::install().map_err(|e| e.to_string())?;
    termination::check()?;
    let _lock = SessionLock::acquire().map_err(|e| e.to_string())?;
    let result = run_inner(count, fps, None, None, IlluminationTrial::None);
    termination::check()?;
    result
}
fn run_inner(
    count: usize,
    fps: u32,
    output: Option<&Path>,
    gain: Option<i32>,
    illumination: IlluminationTrial,
) -> Result<Value, String> {
    let mut settings = scancraft_device::stream_group::SettingsLease::capture(&[0, 2])
        .map_err(|e| e.to_string())?;
    let acquired = run_workers(count, fps, output, gain, illumination);
    let restored = settings.restore().map_err(|e| e.to_string());
    match (acquired, restored) {
        (Ok(mut report), Ok(())) => {
            report["reported_stream_settings_restoration"] = json!("PASS");
            Ok(report)
        }
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(restoration)) => Err(format!("{error}; {restoration}")),
    }
}
fn run_workers(
    count: usize,
    fps: u32,
    output: Option<&Path>,
    gain: Option<i32>,
    illumination: IlluminationTrial,
) -> Result<Value, String> {
    let cancel = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();
    let start = Instant::now();
    for interface in [0, 2] {
        let stop = cancel.clone();
        let path = output.map(|p| {
            p.join(if interface == 0 {
                "left.raptor"
            } else {
                "right.raptor"
            })
        });
        match thread::Builder::new()
            .name(format!("scancraft-probe-{interface}"))
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    capture(interface, fps, count, &stop, path, gain, illumination)
                }))
                .unwrap_or_else(|_| Err(format!("stereo worker {interface} panicked")));
                if result.is_err() {
                    stop.store(true, Ordering::Relaxed);
                }
                result
            }) {
            Ok(handle) => handles.push(handle),
            Err(e) => {
                cancel.store(true, Ordering::Relaxed);
                for handle in handles {
                    let _ = handle.join();
                }
                return Err(format!("could not start stereo worker: {e}"));
            }
        }
    }
    let mut reports = Vec::new();
    let mut errors = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(Ok(report)) => reports.push(report),
            Ok(Err(e)) => errors.push(e),
            Err(_) => {
                cancel.store(true, Ordering::Relaxed);
                errors.push("stereo capture worker panicked".into());
            }
        }
    }
    termination::check()?;
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    reports.sort_by_key(|r| r.interface);
    let left = &reports[0];
    let right = &reports[1];

    let overlap_start = left
        .samples
        .first()
        .unwrap()
        .host_us
        .max(right.samples.first().unwrap().host_us);
    let overlap_end = left
        .samples
        .last()
        .unwrap()
        .host_us
        .min(right.samples.last().unwrap().host_us);
    let right_pts: std::collections::HashSet<_> = right.samples.iter().map(|s| s.pts).collect();
    let exact_pts_matches = left
        .samples
        .iter()
        .filter(|s| right_pts.contains(&s.pts))
        .count();
    let deltas = nearest_host_deltas(&left.samples, &right.samples, overlap_start, overlap_end);
    let mut host_deltas: Vec<_> = deltas.iter().map(|p| p.0).collect();
    host_deltas.sort_unstable();
    let mut pts_deltas: Vec<_> = deltas.iter().map(|p| p.1).collect();
    pts_deltas.sort_unstable();
    Ok(
        json!({"diagnostic_only":true,"synchronization_verified":false,"exposure_timing_verified":false,"native_rust":true,"wine_used":false,"elapsed_seconds":start.elapsed().as_secs_f64(),"left":summary(left),"right":summary(right),"exact_sensor_pts_matches":exact_pts_matches,"association_host_overlap_us":[overlap_start,overlap_end],"nearest_host_pair_differences":{"count":deltas.len(),"host_us_min":host_deltas.first(),"host_us_median":host_deltas.get(host_deltas.len()/2),"host_us_max":host_deltas.last(),"sensor_ticks_min":pts_deltas.first(),"sensor_ticks_median":pts_deltas.get(pts_deltas.len()/2),"sensor_ticks_max":pts_deltas.last()},"interpretation":"Nearest host timestamps are diagnostic associations, not validated synchronized image pairs."}),
    )
}

/// Capture both native streams and publish only after footer/CRC replay succeeds.
pub fn record(count: usize, fps: u32, target: &Path) -> Result<Value, String> {
    record_gain(count, fps, target, None)
}
/// A temporary gain trial restores both controls before publishing its result.
pub fn record_gain(
    count: usize,
    fps: u32,
    target: &Path,
    gain: Option<i32>,
) -> Result<Value, String> {
    record_trial(count, fps, target, gain, IlluminationTrial::None)
}
pub fn record_flood(count: usize, fps: u32, target: &Path) -> Result<Value, String> {
    record_trial(count, fps, target, None, IlluminationTrial::Flood)
}
pub fn record_line(count: usize, fps: u32, target: &Path) -> Result<Value, String> {
    record_trial(count, fps, target, None, IlluminationTrial::Line)
}
fn record_trial(
    count: usize,
    fps: u32,
    target: &Path,
    gain: Option<i32>,
    illumination: IlluminationTrial,
) -> Result<Value, String> {
    validate_request(count, fps)?;
    let _termination = termination::Guard::install().map_err(|e| e.to_string())?;
    termination::check()?;
    let lock = SessionLock::acquire().map_err(|e| e.to_string())?;
    if fs::symlink_metadata(target).is_ok() {
        return Err("stereo output already exists".into());
    }
    let partial = staging(target).map_err(|e| e.to_string())?;
    let result = (|| -> Result<Value, String> {
        let mut lease = gain
            .map(|raw| scancraft_device::ir_gain::ScopedIrGain::apply(&lock, raw))
            .transpose()
            .map_err(|e| e.to_string())?;
        termination::check()?;
        let mut flood_lease = if matches!(illumination, IlluminationTrial::Flood) {
            Some(
                scancraft_device::control::flood_trial::ScopedFlood::enable(&lock)
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        termination::check()?;
        let mut line_lease = if matches!(illumination, IlluminationTrial::Line) {
            Some(
                scancraft_device::control::line_trial::ScopedLine::enable(&lock)
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        let acquired = run_inner(count, fps, Some(&partial), gain, illumination);
        let line_verified = line_lease
            .as_ref()
            .map(|lease| lease.verify_enabled())
            .transpose();
        let line_restored = line_lease.as_mut().map(|lease| lease.restore()).transpose();
        let flood_verified = flood_lease
            .as_ref()
            .map(|lease| lease.verify_enabled())
            .transpose();
        let flood_restored = flood_lease
            .as_mut()
            .map(|lease| lease.restore())
            .transpose();
        if let Some(lease) = lease.as_mut() {
            lease.restore().map_err(|e| e.to_string())?;
        }
        line_restored.map_err(|e| e.to_string())?;
        line_verified.map_err(|e| e.to_string())?;
        flood_restored.map_err(|e| e.to_string())?;
        flood_verified.map_err(|e| e.to_string())?;
        termination::check()?;
        let mut report = acquired?;
        if let Some(lease) = line_lease.as_ref() {
            report["line_laser_trial"] = json!({"property_id":151,"sdk_symbol":"OB_PROP_LASER_LINE_BOOL","state":lease.state(),"enabled_at_end_of_capture_readback":true,"restoration_readback":"PASS","physical_emission_verified":false,"wavelength_verified":false,"mode_changed":false});
        }
        if let Some(lease) = flood_lease.as_ref() {
            report["flood_trial"] = json!({"state":lease.state(),"enabled_at_end_of_capture_readback":true,"restoration_readback":"PASS","physical_emission_verified":false});
        }
        if let Some(lease) = lease.as_ref() {
            report["standard_gain_trial"] = json!({"changes":lease.changes(),"restoration_readback":"PASS","units":"vendor raw; physical conversion unverified"});
        }
        for name in ["left.raptor", "right.raptor"] {
            let file = File::open(partial.join(name)).map_err(|e| e.to_string())?;
            let mut reader = Reader::new(BufReader::new(file)).map_err(|e| e.to_string())?;
            let mut n = 0;
            while reader.read_frame().map_err(|e| e.to_string())?.is_some() {
                termination::check()?;
                n += 1;
            }
            if n != count {
                return Err("recording replay count mismatch".into());
            }
        }
        report["schema_version"] = json!(1);
        report["raw_recordings"] = json!({"left":"left.raptor","right":"right.raptor","saved_frames_each":count,"full_crc_footer_replay":"PASS","capture_context":"GET-only setup observations recorded; binding, per-frame settings and calibration remain unverified","setup_observations_recorded":true});
        if matches!(illumination, IlluminationTrial::Line) {
            report["raw_recordings"]["capture_context"] = json!(
                "SDK line-laser boolean reported enabled before stream setup and verified after capture; physical emission, wavelength and calibration binding unverified"
            );
        }
        if matches!(illumination, IlluminationTrial::Flood) {
            report["raw_recordings"]["capture_context"] = json!(
                "flood reported on before stream setup and verified after capture; physical emission and other calibration/control state remain unknown"
            );
        }
        if gain.is_some() {
            report["raw_recordings"]["capture_context"] = json!(
                "gain readback after stream startup saved in each recording header; other device calibration/control state remains unknown"
            );
        }
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(partial.join("manifest.json"))
            .map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &report).map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        writer.get_ref().sync_all().map_err(|e| e.to_string())?;
        drop(writer);
        File::open(&partial)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        termination::check()?;
        publish(&partial, target).map_err(|e| e.to_string())?;
        Ok(report)
    })();
    let report = result.map_err(|e| {
        format!(
            "{e}; incomplete stereo capture retained at {}",
            partial.display()
        )
    })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "capture published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linear_association_matches_brute_force_with_wrap_and_overlap() {
        let left: Vec<_> = (0..300)
            .map(|i| Sample {
                sequence: i,
                host_us: 80 + u64::from(i) * 23,
                pts: u32::MAX.wrapping_sub(100).wrapping_add(i * 23),
            })
            .collect();
        let right: Vec<_> = (0..200)
            .map(|i| Sample {
                sequence: i,
                host_us: 100 + u64::from(i) * 37,
                pts: u32::MAX.wrapping_sub(80).wrapping_add(i * 37),
            })
            .collect();
        let actual = nearest_host_deltas(&left, &right, 100, 6900);
        let expected: Vec<_> = left
            .iter()
            .filter(|l| l.host_us >= 100 && l.host_us <= 6900)
            .map(|l| {
                let r = right
                    .iter()
                    .min_by_key(|r| r.host_us.abs_diff(l.host_us))
                    .unwrap();
                (
                    r.host_us as i128 - l.host_us as i128,
                    r.pts.wrapping_sub(l.pts) as i32,
                )
            })
            .collect();
        assert_eq!(actual, expected);
    }
    #[test]
    fn accepted_rate_uses_host_span_and_not_device_sequence_gaps() {
        let report = Report {
            interface: 0,
            requested_fps: 50,
            samples: vec![
                Sample {
                    sequence: 1,
                    host_us: 1000,
                    pts: 100,
                },
                Sample {
                    sequence: 100,
                    host_us: 21000,
                    pts: 20100,
                },
            ],
            rejected: 2,
            rejected_before_first_valid: 1,
            rejected_after_first_valid: 1,
            missing: 0,
            gaps: 98,
            duplicate_pts: 0,
            rejection_samples: vec![],
            ongoing_rejection_samples: vec![],
        };
        let data = summary(&report);
        assert_eq!(data["accepted_buffer_rate_hz"], 50.);
        assert_eq!(data["accepted_host_span_seconds"], 0.02);
    }
    #[test]
    fn unsupported_capture_rates_and_unbounded_counts_fail_validation() {
        for (count, fps) in [
            (0, 50),
            (9, 50),
            (30001, 50),
            (100, 0),
            (100, 17),
            (100, 60),
        ] {
            assert!(validate_request(count, fps).is_err());
        }
        assert!(validate_request(10, 30).is_ok());
        assert!(validate_request(30000, 50).is_ok());
    }
    #[test]
    fn empty_or_nonoverlapping_associations_are_controlled() {
        let sample = Sample {
            sequence: 0,
            host_us: 10,
            pts: 10,
        };
        let samples = std::slice::from_ref(&sample);
        assert!(nearest_host_deltas(samples, &[], 0, 20).is_empty());
        assert!(nearest_host_deltas(samples, samples, 20, 30).is_empty());
    }
}
