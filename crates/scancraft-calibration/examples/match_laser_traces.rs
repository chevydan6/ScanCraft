//! Bounded diagnostic example; supplied geometry is not an installed calibration.
use scancraft_calibration::{
    laser_surface::QuadraticSheet,
    laser_trace::{Config, PreparedMatcher},
    stereo::{StereoCalibration, TriangulationLimits},
};
use serde::Deserialize;
use std::io::Read;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    calibration: StereoCalibration,
    limits: TriangulationLimits,
    first_pixel: [f64; 2],
    right_traces: Vec<Vec<[f64; 2]>>,
    sheets: Vec<QuadraticSheet>,
    config: Config,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    std::io::stdin().take(1_048_577).read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err("laser trace input exceeds 1 MiB".into());
    }
    let input: Input = serde_json::from_slice(&bytes)?;
    let result = PreparedMatcher::new(&input.calibration, input.limits)?.match_pixel(
        input.first_pixel,
        &input.right_traces,
        &input.sheets,
        input.config,
        &mut || false,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version":1,"result":result,"calibration_activated":false,
            "physical_accuracy_qualified":false,"units_verified":false,
            "projector_line_identity_verified":false,"source_recording_integrity_verified":false
        }))?
    );
    Ok(())
}
