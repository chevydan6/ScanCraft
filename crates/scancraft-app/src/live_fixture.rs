//! Explicit synthetic engine sessions shared by GUI and process diagnostics.
//! This module opens no scanner and preserves SyntheticFixture provenance.
use scancraft_geometry::work::WorkControl;
use scancraft_workflow::live_stereo::control::Control;
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};

// Device-free, explicitly labelled startup source for reproducible GUI validation.
pub(crate) fn run(
    target: &std::path::Path,
    work: &WorkControl,
    control: &Arc<Control>,
) -> Result<Value, String> {
    run_limited(target, work, control, 100)
}
pub(crate) fn run_limited(
    target: &std::path::Path,
    work: &WorkControl,
    control: &Arc<Control>,
    maximum_frames: u32,
) -> Result<Value, String> {
    use scancraft_core::{Frame, StreamProfile};
    use scancraft_recording::CaptureContext;
    use scancraft_workflow::live_stereo::{
        InputMode, Processor, Side,
        runner::{self, Source},
    };
    use serde_json::json;
    struct Fixture {
        index: u32,
        maximum_frames: u32,
        side: Side,
        pixels: Vec<u8>,
    }
    impl Source for Fixture {
        fn context(&self) -> Result<CaptureContext, String> {
            Ok(CaptureContext {
                controls: json!({"synthetic":true}),
                calibration_status: "synthetic_fixture_not_a_scanner_capture".into(),
                ..Default::default()
            })
        }
        fn next(&mut self) -> Result<Option<Frame>, String> {
            if self.index == self.maximum_frames {
                return Ok(None);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
            let index = self.index;
            self.index += 1;
            let pts = 1000 + index * 33333;
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&pts.to_le_bytes());
            Ok(Some(Frame {
                sequence: index,
                host_timestamp_us: u64::from(pts) + self.side.index() as u64 * 10,
                flags: 0,
                metadata,
                data: self.pixels.clone(),
            }))
        }
        fn statistics(&self) -> Value {
            json!({"synthetic":true,"hardware_accessed":false})
        }
        fn close(self) -> Result<(), String> {
            Ok(())
        }
    }
    struct Temporary(PathBuf);
    impl Drop for Temporary {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let temp = Temporary(
        scancraft_workflow::output::staging(
            &std::env::temp_dir().join("scancraft-live-gui-fixture"),
        )
        .map_err(|e| e.to_string())?,
    );
    let settings = temp.0.join("scan.json");
    std::fs::write(&settings,serde_json::to_vec(&json!({"schema_version":1,"session_id":"synthetic-gui","fps":30,"maximum_frames_per_camera":maximum_frames,"maximum_host_delta_us":1000,
        "tracking_samples":32,"voxel_mm":0.1,"maximum_voxels":1000,"tracking":{"max_correspondence_mm":10.0,"max_rms_mm":0.01,"minimum_overlap":0.8,"max_step_translation_mm":1.0,"max_step_rotation_rad":0.1,"iterations":40,"tolerance_mm":1e-8}})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
    let mut processor = Processor::open(
        &root.join("calibration.rapcal"),
        &root.join("request.json"),
        &settings,
        InputMode::SyntheticFixture,
        work,
    )
    .map_err(|e| e.to_string())?;
    runner::bundle_controlled(
        &mut processor,
        target,
        work,
        move |side, _profile: StreamProfile| {
            let pixels = image::open(root.join(if side.index() == 0 {
                "left.png"
            } else {
                "right.png"
            }))
            .map_err(|e| e.to_string())?
            .to_luma8()
            .into_raw();
            Ok(Fixture {
                index: 0,
                maximum_frames,
                side,
                pixels,
            })
        },
        || Ok(()),
        control,
    )
}
