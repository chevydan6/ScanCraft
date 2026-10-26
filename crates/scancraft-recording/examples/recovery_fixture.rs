//! Redistributable synthetic raw-frame fixtures; no device or proprietary assets.
use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::{CaptureContext, Writer};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: recovery_fixture NEW_DIRECTORY")?,
    );
    std::fs::create_dir(&root)?;
    let context = CaptureContext {
        device_identity: Some("synthetic fixture".into()),
        calibration_bytes: None,
        calibration_status: "synthetic_fixture_not_a_scanner_capture".into(),
        controls: serde_json::json!({"synthetic":true}),
    };
    let mut writer = Writer::with_context(
        Vec::new(),
        StreamProfile {
            interface: 0,
            width: 2,
            height: 2,
            fourcc: *b"GREY",
            fps: 30,
        },
        context,
    )?;
    for i in 0..3_u32 {
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&(1000 + 33337 * i).to_le_bytes());
        writer.push(&Frame {
            sequence: i,
            host_timestamp_us: 1 + 33337 * i as u64,
            flags: 0,
            metadata,
            data: vec![i as u8; 4],
        })?;
    }
    let complete = writer.finish()?;
    std::fs::write(root.join("complete.raptor"), &complete)?;
    std::fs::write(
        root.join("interrupted.raptor.partial"),
        &complete[..complete.len() - 12],
    )?;
    let header = u32::from_le_bytes(complete[8..12].try_into()?) as usize;
    let first = 16 + header;
    let size = u32::from_le_bytes(complete[first..first + 4].try_into()?) as usize;
    let second = first + 8 + size;
    let mut corrupt = complete;
    corrupt[second + 8] ^= 1;
    std::fs::write(root.join("corrupt-middle.raptor"), corrupt)?;
    Ok(())
}
