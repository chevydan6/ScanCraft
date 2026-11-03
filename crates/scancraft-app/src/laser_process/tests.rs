use super::*;
use crate::{capture_ipc, laser_display::tests::snapshot};
fn completion() -> Completion {
    Completion {
        schema_version: 1,
        origin: Origin::RecordingReplay,
        recorded: [1; 2],
        drops: [0; 2],
        processed: 1,
        failed: 0,
        pending: [0; 2],
        helper_omissions: 0,
    }
}
fn spawn_child(script: &str, wire: &[u8]) -> OwnedChild {
    let hex = wire.iter().map(|v| format!("{v:02x}")).collect::<String>();
    let mut command = Command::new("python3");
    command
        .arg("-c")
        .arg(script)
        .arg(hex)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    OwnedChild::spawn(&mut command).unwrap()
}
#[test]
fn slow_graceful_stop_retains_success_and_final_preview_beyond_abort_grace() {
    let mut wire =
        capture_ipc::prepare_laser_progress(&snapshot(Origin::RecordingReplay, 0)).unwrap();
    capture_ipc::write_laser_complete(&mut wire, &Ok(completion())).unwrap();
    let mut child = spawn_child(
        "import sys,time; assert sys.stdin.buffer.read(1)==b'\\x01'; time.sleep(.06); sys.stdout.buffer.write(bytes.fromhex(sys.argv[1])); sys.stdout.buffer.flush()",
        &wire,
    );
    let c = Control::new(Origin::RecordingReplay);
    c.stop.store(true, Ordering::Relaxed);
    let start = Instant::now();
    let result = monitor(&mut child, &c, Duration::from_millis(1)).unwrap();
    assert!(start.elapsed() >= Duration::from_millis(60));
    assert_eq!(result, completion());
    assert!(child.child().try_wait().unwrap().is_some());
    assert_eq!(c.take().unwrap().indices, [0; 2]);
    assert!(c.work.check().is_ok());
}
#[test]
fn protocol_failure_and_explicit_abort_keep_guard_until_actual_exit() {
    let mut child = spawn_child(
        "import sys,time; sys.stdout.buffer.write(b'\\xff\\x01\\x00\\x00\\x00'); sys.stdout.buffer.flush(); sys.stdin.buffer.read(); time.sleep(.06)",
        &[],
    );
    let c = Control::new(Origin::Native);
    let start = Instant::now();
    assert!(monitor(&mut child, &c, Duration::from_millis(1)).is_err());
    assert!(start.elapsed() >= Duration::from_millis(60));
    assert!(child.child().try_wait().unwrap().is_some());
    let mut wire = Vec::new();
    capture_ipc::write_laser_complete(&mut wire, &Err("cancelled".into())).unwrap();
    let mut child = spawn_child(
        "import sys,time; assert sys.stdin.buffer.read()==b''; time.sleep(.06); sys.stdout.buffer.write(bytes.fromhex(sys.argv[1])); sys.stdout.buffer.flush()",
        &wire,
    );
    let c = Control::new(Origin::Native);
    c.work.cancel();
    let start = Instant::now();
    assert!(
        monitor(&mut child, &c, Duration::from_millis(1))
            .unwrap_err()
            .contains("cancelled")
    );
    assert!(start.elapsed() >= Duration::from_millis(60));
    assert!(child.child().try_wait().unwrap().is_some());
}
#[test]
fn only_teardown_can_abandon_pending_cleanup_without_claiming_exit() {
    let mut child = spawn_child(
        "import sys,time; sys.stdin.buffer.read(); time.sleep(.3)",
        &[],
    );
    let c = Control::new(Origin::Native);
    c.teardown.store(true, Ordering::Relaxed);
    let e = monitor(&mut child, &c, Duration::from_millis(1)).unwrap_err();
    assert!(e.contains("NOT confirmed"));
    assert!(child.child().try_wait().unwrap().is_none());
    child.child().wait().unwrap();
}
#[test]
fn latest_mailbox_counts_overwrites_and_rejects_wrong_origin_or_regression() {
    let c = Control::new(Origin::Native);
    c.offer(snapshot(Origin::Native, 0)).unwrap();
    c.offer(snapshot(Origin::Native, 1)).unwrap();
    assert_eq!(c.parent_omissions.load(Ordering::Relaxed), 1);
    assert!(c.offer(snapshot(Origin::RecordingReplay, 2)).is_err());
    assert!(c.offer(snapshot(Origin::Native, 0)).is_err());
    assert_eq!(c.take().unwrap().indices, [1; 2]);
    assert!(c.take().is_none());
    assert!(c.offer(snapshot(Origin::Native, 1)).is_err());
}

#[test]
fn missing_laser_completion_retains_zero_nonzero_and_signal_exit_status() {
    for (script, expected) in [
        ("import sys; sys.exit(0)", "exit status: 0"),
        ("import sys; sys.exit(42)", "exit status: 42"),
        (
            "import os,signal; os.kill(os.getpid(),signal.SIGTERM)",
            "signal: 15",
        ),
    ] {
        // All children are synthetic scripts; no device or native helper is opened.
        let mut child = spawn_child(script, &[]);
        let control = Control::new(Origin::RecordingReplay);
        let error = monitor(&mut child, &control, Duration::from_secs(1)).unwrap_err();
        assert!(error.contains("complete final result"), "{error}");
        assert!(error.contains(expected), "{error}");
        assert!(child.child().try_wait().unwrap().is_some());
    }
}
