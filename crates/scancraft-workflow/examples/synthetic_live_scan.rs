//! Device-free execution of the same live runner. All images/calibration are synthetic.
use scancraft_core::{Frame, StreamProfile};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::CaptureContext;
use scancraft_workflow::live_stereo::{
    InputMode, Processor, Side,
    runner::{self, Source},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
struct Fixture(Option<Frame>);
impl Source for Fixture {
    fn context(&self) -> Result<CaptureContext, String> {
        Ok(CaptureContext {
            controls: json!({"synthetic":true}),
            calibration_status: "synthetic_fixture_not_a_scanner_capture".into(),
            ..Default::default()
        })
    }
    fn next(&mut self) -> Result<Option<Frame>, String> {
        Ok(self.0.take())
    }
    fn statistics(&self) -> Value {
        json!({"source":"synthetic_fixture","hardware_accessed":false})
    }
    fn close(self) -> Result<(), String> {
        Ok(())
    }
}
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn source(side: Side, _profile: StreamProfile) -> Result<Fixture, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
    let pixels = image::open(root.join(if side.index() == 0 {
        "left.png"
    } else {
        "right.png"
    }))
    .map_err(|e| e.to_string())?
    .to_luma8()
    .into_raw();
    let mut metadata = vec![0; 16];
    metadata[10] = 6;
    metadata[11] = 4;
    metadata[12..16].copy_from_slice(&1000u32.to_le_bytes());
    Ok(Fixture(Some(Frame {
        sequence: 0,
        host_timestamp_us: 1000 + side.index() as u64 * 10,
        flags: 0,
        metadata,
        data: pixels,
    })))
}
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("expected a NEW_SYNTHETIC_OUTPUT_DIRECTORY".into());
    }
    let target = Path::new(&args[1]);
    runner::require_new_target(target)?;
    let temporary = Temporary(scancraft_workflow::output::staging(
        &std::env::temp_dir().join("synthetic-live-inputs"),
    )?);
    let settings = temporary.0.join("scan.json");
    fs::write(
        &settings,
        serde_json::to_vec_pretty(
            &json!({"schema_version":1,"session_id":"synthetic-live-reference","fps":30,"maximum_frames_per_camera":1,
        "maximum_host_delta_us":1000,"tracking_samples":32,"voxel_mm":0.1,"maximum_voxels":1000,
        "tracking":{"max_correspondence_mm":10.0,"max_rms_mm":0.01,"minimum_overlap":0.8,"max_step_translation_mm":1.0,
        "max_step_rotation_rad":0.1,"iterations":40,"tolerance_mm":1e-8}}),
        )?,
    )?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
    let work = WorkControl::default();
    let mut processor = Processor::open(
        &root.join("calibration.rapcal"),
        &root.join("request.json"),
        &settings,
        InputMode::SyntheticFixture,
        &work,
    )?;
    let report = runner::bundle(&mut processor, target, &work, source, || Ok(()))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
