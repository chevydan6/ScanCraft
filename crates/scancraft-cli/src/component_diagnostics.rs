//! Raw component measurements with complete recording integrity validation.
use scancraft_geometry::work::WorkControl;
use scancraft_image::{
    GrayImage,
    components::{Config, detect},
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
    config: Config,
    shape_rms_limit: Option<f64>,
) -> Result<Value, Error> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err("recording must be a regular file".into());
    }
    analyze(BufReader::new(file), index, config, shape_rms_limit)
}
fn analyze(
    input: impl Read,
    index: u64,
    config: Config,
    shape_rms_limit: Option<f64>,
) -> Result<Value, Error> {
    if shape_rms_limit.is_some_and(|v| !v.is_finite() || v <= 0. || v > 1e5) {
        return Err("shape RMS limit must be finite and in (0, 100000] pixels".into());
    }
    let reader = Reader::new(input)?;
    let version = reader.version;
    let selected = read_selected(reader, index)?;
    let detections = detect(
        GrayImage::from_frame(&selected.profile, &selected.frame)?,
        config,
        &WorkControl::default(),
    )?;
    if shape_rms_limit.is_some() && detections.observations.len() > 512 {
        return Err("shape diagnostics limited to 512 components per image".into());
    }
    let image = GrayImage::from_frame(&selected.profile, &selected.frame)?;
    let observations: Vec<_> = detections
        .observations
        .iter()
        .map(|o| {
            let mut row = json!({
                "pixel":o.pixel,"area":o.area,"bounds_inclusive":o.bounds,
                "peak":o.peak,"saturated_pixels":o.saturated_pixels
            });
            if let Some(limit) = shape_rms_limit {
                row["shape"] = match scancraft_image::shape::measure(
                    image,
                    config.threshold,
                    o,
                    &WorkControl::default(),
                ) {
                    Ok(fit) => json!({"rms_gate_passed":fit.rms_sampson<=limit,"fit":fit,
                        "physical_center_verified":false}),
                    Err(error) => json!({"rms_gate_passed":false,"error":error,
                        "physical_center_verified":false}),
                };
            }
            row
        })
        .collect();
    let mut report = json!({
        "schema_version":1,"source_recording_version":version,
        "source_integrity":"all frames and footer validated","source_header_crc_protected":version>=2,
        "selected_frame_index":index,"source_total_frames":selected.total_frames,
        "profile":selected.profile,"context":selected.context,
        "sequence":selected.frame.sequence,"host_timestamp_us":selected.frame.host_timestamp_us,
        "sensor_pts":scancraft_core::uvc_pts(&selected.frame.metadata)?,
        "payload_crc32":crc32fast::hash(&selected.frame.data),"metadata_crc32":crc32fast::hash(&selected.frame.metadata),
        "parameters":{"threshold_exclusive":config.threshold,"minimum_area":config.minimum_area,
            "maximum_area":config.maximum_area,"maximum_components":config.maximum_components},
        "method":"eight-connected foreground; binary area centroids; area rejection then border rejection",
        "components_examined":detections.components_examined,"foreground_pixels":detections.foreground_pixels,
        "rejections":{"area":detections.rejected_area,"border":detections.rejected_border},
        "accepted_observations":observations.len(),"observations":observations,
        "coordinate_system":"raw image pixel centers; x right, y down",
        "marker_identity_verified":false,"calibration_activated":false,
        "synchronization_verified":false,"physical_accuracy_qualified":false
    });
    if let Some(limit) = shape_rms_limit {
        report["shape_parameters"] = json!({"maximum_rms_sampson_px":limit,
            "boundary":"foreground pixel centers adjacent to cardinal background, including holes",
            "fit":"normalized general-conic SVD; reject nonellipses",
            "raw_observations_retained":true,"subpixel_boundary":false});
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let profile = scancraft_core::StreamProfile {
            interface: 2,
            width: 7,
            height: 7,
            fourcc: *b"GREY",
            fps: 30,
        };
        let mut writer = scancraft_recording::Writer::new(Vec::new(), profile).unwrap();
        for sequence in 0..2 {
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&(100u32 + sequence * 30).to_le_bytes());
            let mut data = vec![0; 49];
            data[2 * 7 + 2] = 150;
            data[3 * 7 + 3] = 255;
            writer
                .push(&scancraft_core::Frame {
                    sequence,
                    host_timestamp_us: 1000 + u64::from(sequence) * 30,
                    flags: 0,
                    metadata,
                    data,
                })
                .unwrap();
        }
        writer.finish().unwrap()
    }
    fn config() -> Config {
        Config {
            threshold: 100,
            minimum_area: 2,
            maximum_area: 20,
            maximum_components: 100,
        }
    }
    #[test]
    fn reports_raw_observations_and_selected_frame_provenance() {
        let bytes = fixture();
        let report = analyze(bytes.as_slice(), 1, config(), None).unwrap();
        assert_eq!(
            report["source_integrity"],
            "all frames and footer validated"
        );
        assert_eq!(report["source_total_frames"], 2);
        assert_eq!(report["selected_frame_index"], 1);
        assert_eq!(report["profile"]["interface"], 2);
        assert_eq!(report["sensor_pts"], 130);
        assert_eq!(report["source_header_crc_protected"], true);
        assert_eq!(report["accepted_observations"], 1);
        assert_eq!(report["observations"][0]["pixel"], json!([2.5, 2.5]));
        assert_eq!(report["observations"][0]["saturated_pixels"], 1);
        assert_eq!(report["marker_identity_verified"], false);
        assert_eq!(report["physical_accuracy_qualified"], false);
    }
    #[test]
    fn early_selection_never_hides_corruption_after_selected_frame() {
        let mut bytes = fixture();
        let n = bytes.len();
        assert!(analyze(&bytes[..n - 1], 0, config(), None).is_err());
        assert!(analyze(bytes.as_slice(), 2, config(), None).is_err());
        bytes[n - 13] ^= 1;
        assert!(analyze(bytes.as_slice(), 0, config(), None).is_err());
        assert!(analyze(bytes.as_slice(), 0, config(), Some(0.5)).is_err());
    }
    #[test]
    fn failed_shape_keeps_raw_observation_and_requires_an_explicit_valid_limit() {
        let bytes = fixture();
        let report = analyze(bytes.as_slice(), 0, config(), Some(0.5)).unwrap();
        assert_eq!(report["accepted_observations"], 1);
        assert_eq!(report["observations"][0]["pixel"], json!([2.5, 2.5]));
        assert_eq!(report["observations"][0]["shape"]["rms_gate_passed"], false);
        assert!(report["observations"][0]["shape"]["error"].is_string());
        assert_eq!(
            report["observations"][0]["shape"]["physical_center_verified"],
            false
        );
        for limit in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(analyze(bytes.as_slice(), 0, config(), Some(limit)).is_err());
        }
    }
}
