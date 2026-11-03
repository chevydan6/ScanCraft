//! Bounded offline geometry example. Caller supplies calibration and trace lineage.
use scancraft_calibration::{
    epipolar_trace::{Config, PreparedTrace},
    stereo::StereoCalibration,
};
use serde::Deserialize;
use std::io::Read;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    calibration: StereoCalibration,
    first_pixel: [f64; 2],
    right_trace: Vec<[f64; 2]>,
    config: Config,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    std::io::stdin().take(1_048_577).read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err("epipolar trace input exceeds 1 MiB".into());
    }
    let input: Input = serde_json::from_slice(&bytes)?;
    let result = PreparedTrace::new(&input.calibration)?.intersections(
        input.first_pixel,
        &input.right_trace,
        input.config,
        &mut || false,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version":1,
            "method":"epipolar_intersection_with_undistorted_normalized_polyline",
            "result":result,
            "calibration_activated":false,
            "physical_accuracy_qualified":false,
            "units_verified":false,
            "projector_line_identity_verified":false,
            "source_recording_integrity_verified":false
        }))?
    );
    Ok(())
}
