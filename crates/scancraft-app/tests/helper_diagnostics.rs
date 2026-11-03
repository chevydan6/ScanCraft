//! Failure and successful-publication process tests; every request is device-free.
use serde_json::Value;
use std::{
    fs,
    io::BufReader,
    path::PathBuf,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "scancraft-helper-log-integration-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn packets(bytes: &[u8]) -> Vec<(u8, Value)> {
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        assert!(bytes.len() - offset >= 5);
        let n = u32::from_le_bytes(bytes[offset + 1..offset + 5].try_into().unwrap()) as usize;
        assert!(n <= 16 * 1024 * 1024 && n <= bytes.len() - offset - 5);
        result.push((
            bytes[offset],
            serde_json::from_slice(&bytes[offset + 5..offset + 5 + n]).unwrap(),
        ));
        offset += 5 + n;
    }
    result
}
#[test]
fn full_stderr_does_not_prevent_invalid_request_completion_or_change_wire_bytes() {
    for (argument, kind) in [
        ("--laser-helper-native", 6),
        ("--laser-helper-replay", 6),
        ("--live-helper-native", 4),
    ] {
        let normal = Command::new(env!("CARGO_BIN_EXE_scancraft"))
            .arg(argument)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let broken = Command::new(env!("CARGO_BIN_EXE_scancraft"))
            .arg(argument)
            .stdin(Stdio::null())
            .stderr(
                fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .unwrap(),
            )
            .output()
            .unwrap();
        assert_eq!(normal.status.code(), Some(1));
        assert_eq!(broken.status.code(), Some(1));
        assert_eq!(normal.stdout, broken.stdout);
        let values = packets(&broken.stdout);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].0, kind);
        assert!(values[0].1["Err"].as_str().unwrap().contains("invalid"));
    }
}
#[test]
fn diagnostic_setup_failure_precedes_synthetic_helper_launch_and_publication() {
    let temp = Temp::new();
    let state = temp.0.join("state-file");
    fs::write(&state, b"keep").unwrap();
    let target = temp.0.join("scan");
    let output = Command::new(env!("CARGO_BIN_EXE_scancraft"))
        .args(["--synthetic-live-process"])
        .arg(&target)
        .env("XDG_STATE_HOME", &state)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!target.exists());
    assert_eq!(fs::read(&state).unwrap(), b"keep");
    assert!(String::from_utf8_lossy(&output.stderr).contains("helper not launched"));
}
#[test]
fn full_stderr_does_not_prevent_successful_synthetic_capture_and_verified_recordings() {
    let temp = Temp::new();
    let target = temp.0.join("scan");
    let mut child = Command::new(env!("CARGO_BIN_EXE_scancraft"))
        .arg("--live-helper-synthetic")
        .arg(&target)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(
            fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .unwrap(),
        )
        .spawn()
        .unwrap();
    let controls = child.stdin.take();
    let output = child.wait_with_output().unwrap();
    drop(controls);
    assert!(output.status.success());
    let values = packets(&output.stdout);
    assert_eq!(values.last().unwrap().0, 4);
    assert!(values.last().unwrap().1["Ok"].is_object());
    let report: Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["kind"], "synthetic_live_stereo_pipeline");
    assert_eq!(report["physical_accuracy_qualified"], false);
    for name in ["left.raptor", "right.raptor"] {
        let mut reader = scancraft_recording::Reader::new(BufReader::new(
            fs::File::open(target.join(name)).unwrap(),
        ))
        .unwrap();
        assert_eq!(
            reader.context.as_ref().unwrap().calibration_status,
            "synthetic_fixture_not_a_scanner_capture"
        );
        let mut count = 0;
        while reader.read_frame().unwrap().is_some() {
            count += 1;
        }
        assert_eq!(count, 2);
    }
}
