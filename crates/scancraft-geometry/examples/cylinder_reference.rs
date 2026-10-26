//! Bounded batch diagnostic for independent numerical comparison, not app runtime.
use scancraft_geometry::{cylinder, work::WorkControl};
use serde::Deserialize;
use std::{collections::BTreeSet, io::Read};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    points: Vec<[f64; 3]>,
    maximum_rms_mm: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    units: String,
    cases: Vec<Case>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: cylinder_reference REQUEST_JSON")?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("reference request exceeds 8 MiB".into());
    }
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.schema_version != 1
        || request.units != "millimeters"
        || request.cases.is_empty()
        || request.cases.len() > 64
    {
        return Err("unsupported reference request schema/units/case count".into());
    }
    let mut names = BTreeSet::new();
    let mut reports = Vec::new();
    for case in request.cases {
        if case.name.is_empty() || case.name.len() > 128 || !names.insert(case.name.clone()) {
            return Err("invalid/duplicate reference case name".into());
        }
        let result = match cylinder::fit(&case.points, case.maximum_rms_mm, &WorkControl::default())
        {
            Ok(fit) => serde_json::json!({"name":case.name,"status":"ok","fit":fit}),
            Err(error) => serde_json::json!({"name":case.name,"status":"rejected","error":error}),
        };
        reports.push(result);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"schema_version":1,"units":"millimeters","physical_accuracy_qualified":false,"input_crc32":crc32fast::hash(&bytes),"cases":reports})
        )?
    );
    Ok(())
}
