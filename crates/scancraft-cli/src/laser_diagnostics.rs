//! Raw observations only, after validation of the entire source recording.
use scancraft_geometry::work::WorkControl;
use scancraft_image::{
    GrayImage,
    laser::{LineConfig, detect_rows, detect_rows_with_opening},
};
use scancraft_recording::{Reader, selection::read_selected};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::{BufReader, Read},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
type Error = Box<dyn std::error::Error>;
pub fn run(
    path: &Path,
    index: u64,
    config: LineConfig,
    opening_radius: Option<u32>,
) -> Result<Value, Error> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err("recording must be a regular file".into());
    }
    analyze(BufReader::new(file), index, config, opening_radius)
}
fn analyze(
    input: impl Read,
    index: u64,
    config: LineConfig,
    opening_radius: Option<u32>,
) -> Result<Value, Error> {
    let reader = Reader::new(input)?;
    let version = reader.version;
    let selected = read_selected(reader, index)?;
    let image = GrayImage::from_frame(&selected.profile, &selected.frame)?;
    let result = match opening_radius {
        Some(radius) => detect_rows_with_opening(image, config, radius, &WorkControl::default())?,
        None => detect_rows(image, config, &WorkControl::default())?,
    };
    let observations: Vec<_> = result
        .observations
        .iter()
        .map(|o| {
            json!({
                "pixel":o.pixel,"first_x":o.first_x,"last_x":o.last_x,"peak":o.peak,
                "integrated_signal":o.integrated_signal,"variance_x":o.variance_x
            })
        })
        .collect();
    let mut report = json!({
        "schema_version":1,"source_recording_version":version,
        "source_integrity":"all frames and footer validated",
        "source_header_crc_protected":version>=2,
        "selected_frame_index":index,"source_total_frames":selected.total_frames,
        "profile":selected.profile,"context":selected.context,
        "sequence":selected.frame.sequence,"host_timestamp_us":selected.frame.host_timestamp_us,
        "sensor_pts":scancraft_core::uvc_pts(&selected.frame.metadata)?,
        "payload_crc32":crc32fast::hash(&selected.frame.data),
        "metadata_crc32":crc32fast::hash(&selected.frame.metadata),
        "parameters":{"background":config.background,"minimum_peak":config.minimum_peak,
            "maximum_width":config.maximum_width,"saturation":config.saturation,
            "maximum_observations":config.maximum_observations},
        "accepted_observations":observations.len(),"observations":observations,
        "rejections":{"border":result.rejected_border,"width":result.rejected_width,
            "saturation":result.rejected_saturation,"peak":result.rejected_peak},
        "coordinate_system":"raw image pixel centers; x right, y down",
        "projector_line_identity_verified":false,"synchronization_verified":false,
        "physical_accuracy_qualified":false
    });
    if let Some(radius) = opening_radius {
        report["parameters"]["opening_radius"] = json!(radius);
        report["signal_units"] = json!("raw minus row morphological opening, 8-bit contrast");
        report["saturation_units"] = json!("original raw 8-bit intensity");
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut writer = scancraft_recording::Writer::new(
            Vec::new(),
            scancraft_core::StreamProfile {
                interface: 0,
                width: 5,
                height: 1,
                fourcc: *b"GREY",
                fps: 30,
            },
        )
        .unwrap();
        for sequence in 0..2 {
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&(100u32 + sequence).to_le_bytes());
            writer
                .push(&scancraft_core::Frame {
                    sequence,
                    host_timestamp_us: 1000 + u64::from(sequence),
                    flags: 0,
                    metadata,
                    data: vec![10, 20, 40, 30, 10],
                })
                .unwrap();
        }
        writer.finish().unwrap()
    }
    fn config() -> LineConfig {
        LineConfig {
            background: 10,
            minimum_peak: 30,
            maximum_width: 3,
            saturation: 255,
            maximum_observations: 10,
        }
    }
    #[test]
    fn diagnostic_retains_lineage_and_rejects_late_corruption() {
        let bytes = fixture();
        let result = analyze(bytes.as_slice(), 0, config(), None).unwrap();
        assert_eq!(result["source_total_frames"], 2);
        assert_eq!(result["accepted_observations"], 1);
        assert_eq!(result["sensor_pts"], 100);
        assert_eq!(result["physical_accuracy_qualified"], false);
        assert!((result["observations"][0]["pixel"][0].as_f64().unwrap() - 13. / 6.).abs() < 1e-14);
        assert!(analyze(&bytes[..bytes.len() - 1], 0, config(), None).is_err());
        let mut corrupted = bytes.clone();
        let n = corrupted.len();
        corrupted[n - 13] ^= 1;
        assert!(analyze(corrupted.as_slice(), 0, config(), None).is_err());
        assert!(analyze(bytes.as_slice(), 2, config(), None).is_err());
    }
    #[test]
    fn local_diagnostic_labels_contrast_and_preserves_original_frame_crc() {
        let bytes = fixture();
        let raw = analyze(bytes.as_slice(), 0, config(), None).unwrap();
        let local_config = LineConfig {
            background: 0,
            minimum_peak: 10,
            ..config()
        };
        let local = analyze(bytes.as_slice(), 0, local_config, Some(1)).unwrap();
        assert_eq!(local["payload_crc32"], raw["payload_crc32"]);
        assert_eq!(local["metadata_crc32"], raw["metadata_crc32"]);
        assert_eq!(local["parameters"]["opening_radius"], 1);
        assert_eq!(local["accepted_observations"], 1);
        assert_eq!(local["saturation_units"], "original raw 8-bit intensity");
        assert!(analyze(&bytes[..bytes.len() - 1], 0, local_config, Some(1)).is_err());
    }
}
