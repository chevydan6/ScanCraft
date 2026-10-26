//! Offline numerical audit only. Does not create or activate capture calibration.
use scancraft_calibration::{CameraModel, factory_brown, factory_named};
use serde_json::json;
use std::{fs::OpenOptions, io::Read, os::unix::fs::OpenOptionsExt};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err(
            "usage: factory_domain_audit PRIVATE_FACTORY_REGION EXPLICIT_TEST_RADIUS".into(),
        );
    }
    let radius: f64 = args[2].parse()?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc_flags())
        .open(&args[1])?;
    if !file.metadata()?.is_file() {
        return Err("input must be regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(8193).read_to_end(&mut bytes)?;
    let lenses = factory_named::inspect(&bytes)?;
    let mut reports = Vec::new();
    for lens in lenses {
        let [fx, fy, cx, cy] = lens.intrinsic_values.map(f64::from);
        let camera = CameraModel {
            width: lens.width,
            height: lens.height,
            fx,
            fy,
            cx,
            cy,
            skew: 0.,
            distortion: factory_brown::brown5_if_supported(lens.brown_values)?,
            valid_radius: radius,
        };
        let prepared = camera.prepare()?;
        let count = u64::from(lens.width) * u64::from(lens.height);
        if count > 4_194_304 {
            return Err("audit exceeds pixel budget".into());
        }
        let mut failed = 0u64;
        let mut worst_error = 0f64;
        let mut max_radius = 0f64;
        let mut first_error = None;
        for y in 0..lens.height {
            for x in 0..lens.width {
                let pixel = [f64::from(x), f64::from(y)];
                let checked = (|| -> Result<(), String> {
                    let ray = prepared.ray(pixel)?;
                    let projected = prepared.project(ray)?;
                    let error = (projected[0] - pixel[0]).hypot(projected[1] - pixel[1]);
                    if !error.is_finite() {
                        return Err("nonfinite roundtrip".into());
                    }
                    worst_error = worst_error.max(error);
                    max_radius = max_radius.max((ray[0] / ray[2]).hypot(ray[1] / ray[2]));
                    if error > 1e-7 {
                        return Err("roundtrip exceeds 1e-7 pixels".into());
                    }
                    Ok(())
                })();
                if let Err(e) = checked {
                    failed += 1;
                    if first_error.is_none() {
                        first_error = Some(json!({"pixel":pixel,"error":e}));
                    }
                }
            }
        }
        reports.push(json!({"label":lens.label,"size":[lens.width,lens.height],"pixel_centers_checked":count,
            "failed_pixel_centers":failed,"first_failure":first_error,"maximum_roundtrip_error_px":worst_error,
            "maximum_inverse_normalized_radius":max_radius,"model_strong_monotonicity_check_passed":true}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"schema_version":1,
        "scope":"offline numerical lens-domain audit; no physical calibration or capture activation",
        "test_radius":radius,"test_radius_is_physical_calibration_coverage":false,
        "pixel_sampling":"every integer pixel center; continuous image rectangle coverage not asserted",
        "translation_units_verified":false,"transform_direction_verified":false,
        "physical_accuracy_qualified":false,"lenses":reports}))?
    );
    Ok(())
}
// Linux O_NONBLOCK | O_CLOEXEC; keep this offline example free of device APIs.
#[cfg(target_os = "linux")]
fn libc_flags() -> i32 {
    0x800 | 0x80000
}
#[cfg(not(target_os = "linux"))]
fn libc_flags() -> i32 {
    0
}
