//! Whole native scan helper: device/settings ownership remains outside the GUI.
use crate::{
    capture_ipc::{self, Decoder, Message},
    capture_process::{OwnedChild, nonblocking, private_log},
};
use scancraft_geometry::work::WorkControl;
use scancraft_workflow::live_stereo::control::Control;
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
const ABORT_GRACE: Duration = Duration::from_secs(2);

/// Parent-side native scan. Graceful stop may drain/finalize beyond the abort
/// grace; only WorkControl cancellation starts the bounded-abort wait policy.
pub(crate) fn run_native(
    paths: &[PathBuf; 4],
    work: &WorkControl,
    control: &Arc<Control>,
) -> Result<Value, String> {
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command.arg("--live-helper-native").args(paths);
    run_command(command, work, control)
}
fn run_synthetic(
    target: &Path,
    work: &WorkControl,
    control: &Arc<Control>,
) -> Result<Value, String> {
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command.arg("--live-helper-synthetic").arg(target);
    run_command(command, work, control)
}
fn run_command(
    mut command: Command,
    work: &WorkControl,
    control: &Arc<Control>,
) -> Result<Value, String> {
    work.check()?;
    let (log_path, log) = private_log().map_err(|e| e.to_string())?;
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log));
    let mut helper = OwnedChild::spawn(&mut command).map_err(|e| {
        format!(
            "Could not launch scan helper: {e}; diagnostics: {}",
            log_path.display()
        )
    })?;
    let pid = helper.child().id();
    supervise(&mut helper, work, control, ABORT_GRACE).map_err(|e| {
        format!(
            "{e}; scan helper PID {pid}; cleanup diagnostics: {}",
            log_path.display()
        )
    })
}
fn supervise(
    helper: &mut OwnedChild,
    work: &WorkControl,
    control: &Control,
    grace: Duration,
) -> Result<Value, String> {
    let mut output = helper
        .child()
        .stdout
        .take()
        .ok_or("scan helper stdout missing")?;
    nonblocking(&output).map_err(|e| e.to_string())?;
    nonblocking(
        helper
            .child()
            .stdin
            .as_ref()
            .ok_or("scan helper stdin missing")?,
    )
    .map_err(|e| e.to_string())?;
    let mut decoder = Decoder::default();
    let mut complete = None;
    let mut aborting = None;
    let mut sent = None;
    let mut eof = false;
    let mut bytes = [0; 65536];
    loop {
        if aborting.is_none() && work.check().is_err() {
            helper.child().stdin.take();
            aborting = Some(Instant::now());
        }
        if aborting.is_none() {
            let state =
                u8::from(control.pause_requested()) | (u8::from(control.stop_requested()) << 1);
            if sent != Some(state) {
                let input = helper
                    .child()
                    .stdin
                    .as_mut()
                    .ok_or("scan helper control pipe closed")?;
                match input.write(&[state]) {
                    Ok(1) => sent = Some(state),
                    Ok(_) => return Err("scan helper control pipe made no progress".into()),
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) => {}
                    Err(e) if e.kind() == io::ErrorKind::BrokenPipe => (), // Drain the child's final result/exit.
                    Err(e) => return Err(format!("scan helper command failed: {e}")),
                }
            }
        }
        for _ in 0..64 {
            match output.read(&mut bytes) {
                Ok(0) => {
                    eof = true;
                    break;
                }
                Ok(n) => decoder.push(&bytes[..n]).map_err(|e| e.to_string())?,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(format!("scan helper read failed: {e}")),
            }
            while let Some(message) = decoder.next().map_err(|e| e.to_string())? {
                match message {
                    Message::LiveProgress(update) => control.apply_display_update(update)?,
                    Message::LiveComplete(result) => complete = Some(result),
                    _ => return Err("preview packet in live-scan helper stream".into()),
                }
            }
        }
        if let Some(status) = helper.child().try_wait().map_err(|e| e.to_string())?
            && eof
        {
            decoder.finish().map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!("Scan helper exited {status}; result: {complete:?}"));
            }
            return complete.ok_or("Scan helper omitted its result")?;
        }
        if aborting.is_some_and(|start| start.elapsed() >= grace) {
            return Err("Scan helper is still running; cleanup/restoration is NOT confirmed. Any acquired device handles and locks remain owned by that helper until cleanup finishes".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
/// Control EOF is abort, not graceful publication. Explicit Stop is latched and
/// does not cancel WorkControl, so normal stop can drain/fuse/publish fully.
fn read_controls(
    mut input: impl Read,
    work: &WorkControl,
    control: &Control,
) -> Result<(), String> {
    let result = (|| {
        let mut bytes = [0; 32];
        loop {
            let n = match input.read(&mut bytes) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                result => result.map_err(|e| e.to_string())?,
            };
            if n == 0 {
                return Ok(());
            }
            for state in &bytes[..n] {
                if *state > 3 {
                    return Err("invalid live helper control byte".into());
                }
                if state & 2 != 0 {
                    control.request_stop();
                }
                if !control.stop_requested() {
                    let _ = control.set_paused(state & 1 != 0);
                }
            }
        }
    })();
    work.cancel();
    control.request_stop();
    result
}
fn summary(report: &Value) -> Result<Value, String> {
    let kind = report["kind"].as_str().ok_or("missing scan report kind")?;
    let points = report["fused_points"]
        .as_u64()
        .ok_or("missing scan point count")?;
    let exported = report["geometry_exported"]
        .as_bool()
        .ok_or("missing scan export status")?;
    Ok(
        json!({"schema_version":1,"kind":kind,"fused_points":points,"geometry_exported":exported,
        "source_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,
        "details":"report.json in the published scan folder"}),
    )
}
fn child_session(paths: &[PathBuf], synthetic: bool) -> Result<Value, String> {
    eprintln!(
        "{}",
        json!({"helper_pid":std::process::id(),"status":"starting",
        "kind":if synthetic {"synthetic_fixture"} else {"native_scan_candidate"}})
    );
    let work = Arc::new(WorkControl::default());
    let control = Arc::new(Control::new(true));
    let w = work.clone();
    let c = control.clone();
    std::thread::Builder::new()
        .name("scancraft-scan-helper-control".into())
        .spawn(move || {
            if let Err(error) = read_controls(io::stdin(), &w, &c) {
                eprintln!("Control failed: {error}");
            }
        })
        .map_err(|e| e.to_string())?;
    let done = Arc::new(AtomicBool::new(false));
    let d = done.clone();
    let w = work.clone();
    let c = control.clone();
    let progress = std::thread::Builder::new()
        .name("scancraft-scan-helper-display".into())
        .spawn(move || {
            let mut output = io::stdout().lock();
            loop {
                let finished = d.load(Ordering::Acquire);
                if let Err(error) =
                    capture_ipc::write_live_progress(&mut output, &c.take_display_update())
                {
                    w.cancel();
                    c.request_stop();
                    return Err(error.to_string());
                }
                if finished {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        })
        .map_err(|e| e.to_string())?;
    let outcome = if synthetic {
        crate::live_fixture::run_limited(&paths[0], &work, &control, 2)
    } else {
        scancraft_acquisition::scan(&paths[0], &paths[1], &paths[2], &paths[3], &work, &control)
    };
    done.store(true, Ordering::Release);
    let transport = progress
        .join()
        .map_err(|_| "live display helper panicked".to_string())?;
    eprintln!(
        "{}",
        json!({"scan_result":outcome.as_ref().map(|_| "completed").map_err(|e|e.as_str()),"transport_error":transport.as_ref().err()})
    );
    match (outcome, transport) {
        (Ok(report), Ok(())) => summary(&report),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(format!(
            "Scan completed but display transport failed: {error}; inspect target report.json"
        )),
        (Err(error), Err(transport)) => Err(format!(
            "{error}; display transport also failed: {transport}"
        )),
    }
}
/// Internal helper entry points. The explicit synthetic fixture opens no scanner
/// and retains the engine's synthetic provenance labels in persisted reports.
pub(crate) fn helper_entry(args: &[OsString]) -> Option<i32> {
    if args
        .first()
        .is_some_and(|arg| arg == "--synthetic-live-process")
    {
        let result = if args.len() == 2 {
            run_synthetic(
                Path::new(&args[1]),
                &WorkControl::default(),
                &Arc::new(Control::new_remote_display(true)),
            )
        } else {
            Err("synthetic-live-process requires a new output directory".into())
        };
        match &result {
            Ok(report) => println!("{}", report),
            Err(error) => eprintln!("Synthetic process diagnostic failed: {error}"),
        }
        return Some(if result.is_ok() { 0 } else { 1 });
    }
    let synthetic = match args.first().and_then(|s| s.to_str()) {
        Some("--live-helper-native") => false,
        Some("--live-helper-synthetic") => true,
        _ => return None,
    };
    let result = if args.len() != if synthetic { 2 } else { 5 } {
        Err("invalid live helper argument count".into())
    } else {
        let paths: Vec<PathBuf> = args[1..].iter().map(PathBuf::from).collect();
        child_session(&paths, synthetic)
    };
    eprintln!("{}", json!({"live_helper_result":result}));
    let write = capture_ipc::write_live_complete(&mut io::stdout().lock(), &result);
    if let Err(error) = &write {
        eprintln!("Final IPC failed: {error}");
    }
    Some(if result.is_ok() && write.is_ok() {
        0
    } else {
        1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_resume_and_stop_are_distinct_from_eof_abort() {
        use std::os::unix::net::UnixStream;
        let (input, mut output) = UnixStream::pair().unwrap();
        let work = Arc::new(WorkControl::default());
        let control = Arc::new(Control::new(true));
        let w = work.clone();
        let c = control.clone();
        let reader = std::thread::spawn(move || read_controls(input, &w, &c));
        let deadline = Instant::now() + Duration::from_secs(5);
        let wait = |condition: &dyn Fn() -> bool| {
            while !condition() {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
        };
        output.write_all(&[1]).unwrap();
        wait(&|| control.pause_requested());
        output.write_all(&[0]).unwrap();
        wait(&|| !control.pause_requested());
        output.write_all(&[2]).unwrap();
        wait(&|| control.stop_requested());
        assert!(
            work.check().is_ok(),
            "graceful stop must not abort publication"
        );
        drop(output);
        reader.join().unwrap().unwrap();
        assert!(work.check().is_err(), "parent disconnect must abort");
    }
    fn completion_script(delay: &str) -> String {
        let mut bytes = Vec::new();
        capture_ipc::write_live_complete(&mut bytes, &Ok(json!({"geometry_exported":false})))
            .unwrap();
        let escaped = bytes
            .iter()
            .map(|b| format!("\\{b:03o}"))
            .collect::<String>();
        format!("sleep {delay}; printf '{escaped}'")
    }
    #[test]
    fn graceful_stop_can_finalize_beyond_the_abort_grace() {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg(completion_script("0.1"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut helper = OwnedChild::spawn(&mut command).unwrap();
        let control = Control::new_remote_display(true);
        control.request_stop();
        let start = Instant::now();
        let result = supervise(
            &mut helper,
            &WorkControl::default(),
            &control,
            Duration::from_millis(10),
        )
        .unwrap();
        assert_eq!(result["geometry_exported"], false);
        assert!(start.elapsed() >= Duration::from_millis(50));
    }
    #[test]
    fn abort_stops_waiting_for_a_stalled_helper_without_killing_it() {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("exec sleep 1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut helper = OwnedChild::spawn(&mut command).unwrap();
        let work = WorkControl::default();
        work.cancel();
        let start = Instant::now();
        let error = supervise(
            &mut helper,
            &work,
            &Control::new_remote_display(true),
            Duration::from_millis(20),
        )
        .unwrap_err();
        assert!(error.contains("NOT confirmed"));
        assert!(helper.child().try_wait().unwrap().is_none());
        assert!(start.elapsed() < Duration::from_millis(800));
    }
    #[test]
    fn summary_stays_small_and_never_attests_hardware() {
        let report = json!({"kind":"synthetic_live_stereo_pipeline", "fused_points":3,
            "geometry_exported":true, "pairs":vec!["private per-pair detail";10_000]});
        let small = summary(&report).unwrap();
        assert!(small.get("pairs").is_none());
        assert_eq!(small["physical_accuracy_qualified"], false);
        assert_eq!(small["kind"], "synthetic_live_stereo_pipeline");
        assert!(serde_json::to_vec(&small).unwrap().len() < 1024);
        assert!(summary(&Value::Null).is_err());
        let work = WorkControl::default();
        let control = Control::new(true);
        assert!(read_controls([255].as_slice(), &work, &control).is_err());
        assert!(work.check().is_err());
    }
}
