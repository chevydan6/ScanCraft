//! Fixed-correspondence diagnostic for independent robust-registration comparisons.
use scancraft_geometry::{
    point_plane::{self, Loss},
    registration::Rigid,
    work::WorkControl,
};
use serde::Deserialize;
use std::io::Read;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    source: Vec<[f64; 3]>,
    target: Vec<[f64; 3]>,
    normals: Vec<[f64; 3]>,
    delta_mm: f64,
    iterations: usize,
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
        .ok_or("usage: robust_plane_reference REQUEST_JSON")?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("request exceeds 8 MiB".into());
    }
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.schema_version != 1
        || request.units != "millimeters"
        || !(1..=64).contains(&request.cases.len())
    {
        return Err("unsupported reference schema/units/count".into());
    }
    let mut output = Vec::new();
    let mut names = std::collections::HashSet::new();
    for case in request.cases {
        if case.name.is_empty()
            || case.name.len() > 128
            || !names.insert(case.name.clone())
            || !(1..=100).contains(&case.iterations)
        {
            return Err("invalid reference case name/iterations".into());
        }
        let mut pose = Rigid::identity();
        let mut rms = 0.;
        let mut trace = Vec::new();
        for _ in 0..case.iterations {
            let points: Vec<_> = case.source.iter().map(|p| pose.apply(*p)).collect();
            let (delta, residual) = point_plane::step_with_loss(
                &points,
                &case.target,
                &case.normals,
                0.5,
                Loss::Huber {
                    delta_mm: case.delta_mm,
                },
                &WorkControl::default(),
            )?;
            let motion = points
                .iter()
                .map(|p| {
                    let q = delta.apply(*p);
                    (q[0] - p[0]).hypot(q[1] - p[1]).hypot(q[2] - p[2])
                })
                .fold(0f64, f64::max);
            pose = delta.compose(&pose);
            rms = residual;
            trace.push(serde_json::json!({"rotation":(0..3).map(|i|(0..3).map(|j|pose.rotation[(i,j)]).collect::<Vec<_>>()).collect::<Vec<_>>(),"translation_mm":pose.translation.as_slice()}));
            if motion < 1e-10 {
                break;
            }
        }
        output.push(serde_json::json!({"name":case.name,"iterations":trace.len(),"point_to_plane_rms_mm":rms,"trace":trace,"rotation":(0..3).map(|i|(0..3).map(|j|pose.rotation[(i,j)]).collect::<Vec<_>>()).collect::<Vec<_>>(),"translation_mm":pose.translation.as_slice()}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"schema_version":1,"units":"millimeters","physical_accuracy_qualified":false,"cases":output})
        )?
    );
    Ok(())
}
