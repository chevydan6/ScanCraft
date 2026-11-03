//! Offline fixed-camera pose validation; arbitrary target units remain explicit.
use crate::{
    Error, bounded,
    calibration_bundle::{ObservationProvenance, check, ensure_new, publish_fit},
};
use scancraft_calibration::{
    CameraModel,
    planar_pose::{Observation, Options, fit},
};
use scancraft_geometry::work::WorkControl;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub provenance: ObservationProvenance,
    pub fixed_camera: CameraModel,
    pub observations: Vec<Observation>,
    pub fitting: Options,
}
/// Exact request and fit report are retained in a new no-overwrite bundle.
/// Supplied observations alone do not attest independence from lens training.
pub fn run(
    input: &Path,
    target: &Path,
    work: &WorkControl,
    cancel: &dyn Fn() -> bool,
) -> Result<Value, Error> {
    check(work, cancel)?;
    ensure_new(target)?;
    let bytes = bounded(input, MAX_REQUEST_BYTES, work)?;
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.schema_version != 1 {
        return Err("unsupported planar pose schema".into());
    }
    let result = fit(
        &request.fixed_camera,
        &request.observations,
        &request.fitting,
        &|| work.check().is_err() || cancel(),
    )?;
    let report = json!({"schema_version":1,"scope":"offline six-parameter planar pose fit with fixed camera; target units supplied, not assumed millimeters",
        "provenance":request.provenance,"intrinsics_fixed":true,"distortion_fixed":true,
        "physical_units_verified":false,"physical_accuracy_qualified":false,"held_out_independence_verified":false,
        "device_binding_verified":false,"fit":result});
    publish_fit(
        &bytes,
        &report,
        target,
        work,
        cancel,
        "offline fixed-camera planar pose validation only",
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_calibration::Brown5;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    #[test]
    fn retains_units_request_digests_and_refuses_overwrite_invalid_and_cancelled() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "scancraft-pose-workflow-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let camera = CameraModel {
            width: 800,
            height: 600,
            fx: 520.,
            fy: 500.,
            cx: 391.,
            cy: 299.,
            skew: 0.,
            distortion: Brown5::ZERO,
            valid_radius: 1.,
        };
        let mut observations = Vec::new();
        for y in 0..5 {
            for x in 0..7 {
                let xy = [(x as f64 - 3.) * 20., (y as f64 - 2.) * 20.];
                observations.push(Observation {
                    target_xy: xy,
                    pixel: camera.project([xy[0], xy[1], 300.]).unwrap(),
                });
            }
        }
        let mut request = Request {
            schema_version: 1,
            provenance: ObservationProvenance::SyntheticFixture,
            fixed_camera: camera,
            observations,
            fitting: Options::default(),
        };
        let input = root.join("input.json");
        let output = root.join("fit");
        let bytes = serde_json::to_vec_pretty(&request).unwrap();
        fs::write(&input, &bytes).unwrap();
        let manifest = run(&input, &output, &WorkControl::default(), &|| false).unwrap();
        assert_eq!(fs::read(output.join("observations.json")).unwrap(), bytes);
        let report: Value =
            serde_json::from_slice(&fs::read(output.join("fit.json")).unwrap()).unwrap();
        assert!(report["fit"].get("translation_mm").is_none());
        assert!(
            (report["fit"]["translation_in_supplied_target_units"][2]
                .as_f64()
                .unwrap()
                - 300.)
                .abs()
                < 1e-6
        );
        assert_eq!(report["held_out_independence_verified"], false);
        assert_eq!(report["physical_units_verified"], false);
        for entry in manifest["artifacts"].as_array().unwrap() {
            let data = fs::read(output.join(entry["role"].as_str().unwrap())).unwrap();
            assert_eq!(
                entry["crc32"].as_u64().unwrap(),
                u64::from(crc32fast::hash(&data))
            );
            assert_eq!(entry["bytes"].as_u64().unwrap(), data.len() as u64);
        }
        assert!(run(&input, &output, &WorkControl::default(), &|| false).is_err());
        let absent = root.join("absent");
        assert!(run(&input, &absent, &WorkControl::default(), &|| true).is_err());
        request.schema_version = 2;
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        assert!(run(&input, &absent, &WorkControl::default(), &|| false).is_err());
        assert!(!absent.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
