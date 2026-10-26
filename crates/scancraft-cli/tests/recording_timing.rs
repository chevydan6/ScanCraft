use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::Writer;
use std::{fs, process::Command};
struct Fixture(std::path::PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn native_timing_command_reports_unqualified_candidates_without_modifying_recordings() {
    let root = std::env::temp_dir().join(format!("scancraft-cli-timing-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    let mut sources = Vec::new();
    for (name, interface, offset) in [("left.raptor", 0, 0), ("right.raptor", 2, 1)] {
        let path = fixture.0.join(name);
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut writer = Writer::new(
            file,
            StreamProfile {
                interface,
                width: 2,
                height: 2,
                fourcc: *b"GREY",
                fps: 50,
            },
        )
        .unwrap();
        for i in 0..3_u32 {
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&(100 + i).to_le_bytes());
            writer
                .push(&Frame {
                    sequence: i,
                    host_timestamp_us: 1000 + u64::from(i) * 20 + offset,
                    flags: 0,
                    metadata,
                    data: vec![20; 4],
                })
                .unwrap();
        }
        writer.finish().unwrap();
        sources.push((path.clone(), fs::read(path).unwrap()));
    }
    let run = |limit: &str| {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("recording-timing")
            .arg(&sources[0].0)
            .arg(&sources[1].0)
            .arg(limit)
            .output()
            .unwrap()
    };
    let result = run("1");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["unique_nearest_within_limit"], 3);
    for key in [
        "clock_domains_verified",
        "synchronization_verified",
        "physical_accuracy_qualified",
    ] {
        assert_eq!(report[key], false);
    }
    assert!(!run("-1").status.success());
    for (path, bytes) in &sources {
        assert_eq!(fs::read(path).unwrap(), *bytes);
    }
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 2);
}
