//! Redistributable synthetic preview fixture; never contains scanner data.
use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::{CaptureContext, Writer};
use std::{fs::OpenOptions, io::BufWriter};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: playback_fixture NEW_RECORDING")?;
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let profile = StreamProfile {
        interface: 0,
        width: 320,
        height: 180,
        fourcc: *b"GREY",
        fps: 30,
    };
    let context = CaptureContext {
        device_identity: Some("synthetic playback fixture".into()),
        calibration_status: "synthetic_fixture_not_a_scanner_capture".into(),
        controls: serde_json::json!({"synthetic":true}),
        ..Default::default()
    };
    let mut writer = Writer::with_context(BufWriter::new(file), profile, context)?;
    for i in 0..3u32 {
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&(1000 + i * 33333).to_le_bytes());
        let data = (0..180)
            .flat_map(|y| {
                (0..320).map(move |x| {
                    if y < 30 {
                        if x / 40 % 2 == 0 { 255 } else { 0 }
                    } else {
                        ((x * 255 / 319 + i * 40) % 256) as u8
                    }
                })
            })
            .collect();
        writer.push(&Frame {
            sequence: i,
            host_timestamp_us: 1000 + u64::from(i) * 33333,
            flags: 0,
            metadata,
            data,
        })?;
    }
    writer.finish()?.get_ref().sync_all()?;
    Ok(())
}
