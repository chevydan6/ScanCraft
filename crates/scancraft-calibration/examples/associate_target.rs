//! Offline pattern association probe. Input contains no implicit physical units.
use scancraft_calibration::target::{Config, Point, associate};
use serde::Deserialize;
use std::io::Read;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    target: Vec<Point>,
    observed: Vec<Point>,
    config: Config,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    std::io::stdin().take(1_048_577).read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err("association input exceeds 1 MiB".into());
    }
    let input: Input = serde_json::from_slice(&bytes)?;
    let result = associate(&input.target, &input.observed, input.config, || false)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1,
            "method": "four_anchor_planar_pattern_association",
            "association": result,
            "calibration_activated": false,
            "physical_accuracy_qualified": false,
            "units_verified": false,
            "source_recording_integrity_verified": false
        }))?
    );
    Ok(())
}
