//! Convert the bundled synthetic PNG regression pair into three-frame recordings.
//! This never captures hardware and refuses to overwrite an existing directory.
use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::{CaptureContext, Writer};
use std::{fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: recorded_stereo_fixture NEW_DIRECTORY")?,
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
    let calibration =
        scancraft_calibration::storage::read(fs::File::open(root.join("calibration.rapcal"))?)?;
    if calibration.provenance != scancraft_calibration::storage::Provenance::SyntheticFixture {
        return Err("fixture calibration must be synthetic".into());
    }
    fs::create_dir(&target)?;
    for name in ["calibration.rapcal", "request.json"] {
        fs::copy(root.join(name), target.join(name))?;
    }
    for (name, interface) in [("left", 0), ("right", 2)] {
        let image = image::open(root.join(format!("{name}.png")))?.to_luma8();
        let context = CaptureContext {
            device_identity: Some("synthetic recorded stereo fixture".into()),
            calibration_status: "synthetic_fixture_not_a_scanner_capture".into(),
            controls: serde_json::json!({"synthetic":true}),
            ..Default::default()
        };
        let mut writer = Writer::with_context(
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target.join(format!("{name}.raptor")))?,
            StreamProfile {
                interface,
                width: image.width(),
                height: image.height(),
                fourcc: *b"GREY",
                fps: 30,
            },
            context,
        )?;
        for i in 0..3u32 {
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&(1000 + i * 33333).to_le_bytes());
            writer.push(&Frame {
                sequence: 10 + i,
                host_timestamp_us: 1000 + u64::from(i) * 33333,
                flags: 0,
                metadata,
                data: image.as_raw().clone(),
            })?;
        }
        writer.finish()?.sync_all()?;
    }
    Ok(())
}
