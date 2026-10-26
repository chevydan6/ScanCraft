use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::Writer;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "scancraft-image-export-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn run(&self) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("export-image")
            .arg(self.0.join("input.raptor"))
            .arg(self.0.join("output.png"))
            .output()
            .unwrap()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn recording() -> Vec<u8> {
    let profile = StreamProfile {
        interface: 0,
        width: 2,
        height: 2,
        fourcc: *b"GREY",
        fps: 30,
    };
    let mut writer = Writer::new(Vec::new(), profile).unwrap();
    for sequence in 0..2u32 {
        let mut metadata = vec![0; 22];
        metadata[10] = 12;
        metadata[11] = 0x0c;
        metadata[12..16].copy_from_slice(&(1000 + sequence).to_le_bytes());
        writer
            .push(&Frame {
                sequence,
                host_timestamp_us: 1000 + u64::from(sequence),
                flags: 0,
                metadata,
                data: if sequence == 0 {
                    vec![1, 2, 3, 4]
                } else {
                    vec![10, 20, 30, 40]
                },
            })
            .unwrap();
    }
    writer.finish().unwrap()
}

#[test]
fn validates_all_frames_before_publishing_exact_first_image() {
    let s = Scratch::new();
    let bytes = recording();
    fs::write(s.0.join("input.raptor"), &bytes).unwrap();
    let output = s.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["frames"], 2);
    assert_eq!(summary["integrity"], "PASS");
    assert_eq!(
        image::open(s.0.join("output.png"))
            .unwrap()
            .to_luma8()
            .as_raw(),
        &[1, 2, 3, 4]
    );
    assert_eq!(fs::read(s.0.join("input.raptor")).unwrap(), bytes);
    assert_eq!(fs::read_dir(&s.0).unwrap().count(), 2);
}

#[test]
fn later_payload_corruption_and_footer_errors_never_publish_or_stage_an_image() {
    let original = recording();
    let mut corrupt = original.clone();
    let offset = corrupt
        .windows(4)
        .rposition(|w| w == [10, 20, 30, 40])
        .unwrap();
    corrupt[offset] ^= 1;
    let mut bad_footer = original.clone();
    *bad_footer.last_mut().unwrap() ^= 1;
    for bytes in [corrupt, bad_footer, original[..original.len() - 1].to_vec()] {
        let s = Scratch::new();
        fs::write(s.0.join("input.raptor"), bytes).unwrap();
        assert!(!s.run().status.success());
        assert!(!s.0.join("output.png").exists());
        assert_eq!(fs::read_dir(&s.0).unwrap().count(), 1);
    }
}

#[test]
fn existing_file_and_dangling_symlink_are_preserved() {
    let s = Scratch::new();
    fs::write(s.0.join("input.raptor"), recording()).unwrap();
    let output = s.0.join("output.png");
    fs::write(&output, b"keep this").unwrap();
    assert!(!s.run().status.success());
    assert_eq!(fs::read(&output).unwrap(), b"keep this");
    fs::remove_file(&output).unwrap();
    std::os::unix::fs::symlink("missing.png", &output).unwrap();
    assert!(!s.run().status.success());
    assert_eq!(
        fs::read_link(&output).unwrap(),
        PathBuf::from("missing.png")
    );
    assert!(!s.0.join("missing.png").exists());
}
