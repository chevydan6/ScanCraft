//! Device-free end-to-end process boundary qualification with explicit fixtures.
use std::{
    fs,
    io::BufReader,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn synthetic_process_publishes_verified_recordings_and_retains_synthetic_labels() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = Temp(std::env::temp_dir().join(format!(
        "scancraft-live-process-integration-{}-{stamp}",
        std::process::id()
    )));
    fs::create_dir(&root.0).unwrap();
    let target = root.0.join("scan");
    let output = Command::new(env!("CARGO_BIN_EXE_scancraft"))
        .arg("--synthetic-live-process")
        .arg(&target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
    for value in [&summary, &report] {
        assert_eq!(value["kind"], "synthetic_live_stereo_pipeline");
        assert_eq!(value["source_binding_verified"], false);
        assert_eq!(value["physical_accuracy_qualified"], false);
        assert_eq!(value["synchronization_verified"], false);
    }
    assert_eq!(summary["helper_omitted_previews"], 0);
    assert_eq!(summary["fused_points"], report["fused_points"]);
    assert_eq!(summary["geometry_exported"], report["geometry_exported"]);
    for name in ["left.raptor", "right.raptor"] {
        let mut reader = scancraft_recording::Reader::new(BufReader::new(
            fs::File::open(target.join(name)).unwrap(),
        ))
        .unwrap();
        assert_eq!(
            reader.context.as_ref().unwrap().calibration_status,
            "synthetic_fixture_not_a_scanner_capture"
        );
        let mut frames = 0;
        while reader.read_frame().unwrap().is_some() {
            frames += 1;
        }
        assert_eq!(frames, 2);
    }
    let before = fs::read(target.join("report.json")).unwrap();
    let retry = Command::new(env!("CARGO_BIN_EXE_scancraft"))
        .arg("--synthetic-live-process")
        .arg(&target)
        .output()
        .unwrap();
    assert!(!retry.status.success());
    assert_eq!(fs::read(target.join("report.json")).unwrap(), before);
}
