//! Explicit fixed-lens stereo fitting, never automatic physical/capture calibration.
pub use crate::calibration_bundle::ObservationProvenance;
use crate::{
    Error, bounded,
    calibration_bundle::{check, ensure_new, publish_fit},
};
use scancraft_calibration::{
    CameraModel,
    stereo_fit::{StereoFitOptions, StereoView, fit_stereo_extrinsics},
};
use scancraft_geometry::work::WorkControl;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
pub const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
/// Declared lenses and common target correspondences; no device/source binding implied.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub provenance: ObservationProvenance,
    pub fixed_left: CameraModel,
    pub fixed_right: CameraModel,
    pub views: Vec<StereoView>,
    pub fitting: StereoFitOptions,
}
/// Fit fixed-lens stereo geometry and retain exact bytes in a new diagnostic bundle.
/// Lens domains and target pairing are supplied declarations; hardware is never opened.
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
        return Err("unsupported stereo observation schema".into());
    }
    check(work, cancel)?;
    let fit = fit_stereo_extrinsics(
        &request.fixed_left,
        &request.fixed_right,
        &request.views,
        &request.fitting,
        &|| work.check().is_err() || cancel(),
    )?;
    check(work, cancel)?;
    let report = json!({"schema_version":1,"scope":"offline fixed-intrinsic stereo fit; not qualified or installed capture calibration","provenance":request.provenance,
        "intrinsics_fixed":true,"device_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,"fit":fit});
    publish_fit(
        &bytes,
        &report,
        target,
        work,
        cancel,
        "offline fixed-intrinsic stereo fit only",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_calibration::{Brown5, stereo_fit::StereoObservation};
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    struct Temporary(std::path::PathBuf);
    impl Temporary {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "scancraft-stereo-fit-workflow-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temporary {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn request() -> Request {
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
        let mut views = Vec::new();
        for i in 0..5 {
            let (s, c) = (i as f64 * 0.08 - 0.16).sin_cos();
            let mut observations = Vec::new();
            for y in 0..5 {
                for x in 0..7 {
                    let xy = [(x as f64 - 3.) * 20., (y as f64 - 2.) * 20.];
                    let point = [
                        c * xy[0] + i as f64 * 3.,
                        xy[1],
                        -s * xy[0] + 280. + i as f64 * 15.,
                    ];
                    observations.push(StereoObservation {
                        target_xy_mm: xy,
                        left_pixel: camera.project(point).unwrap(),
                        right_pixel: camera
                            .project([point[0] - 60., point[1] + 1., point[2]])
                            .unwrap(),
                    });
                }
            }
            views.push(StereoView { observations });
        }
        Request {
            schema_version: 1,
            provenance: ObservationProvenance::SyntheticFixture,
            fixed_left: camera.clone(),
            fixed_right: camera,
            views,
            fitting: StereoFitOptions::default(),
        }
    }
    #[test]
    fn retains_fixed_models_exact_request_and_existing_output() {
        let temp = Temporary::new();
        let input = temp.0.join("request.json");
        let target = temp.0.join("fit");
        let bytes = serde_json::to_vec_pretty(&request()).unwrap();
        fs::write(&input, &bytes).unwrap();
        let manifest = run(&input, &target, &WorkControl::default(), &|| false).unwrap();
        assert_eq!(fs::read(&input).unwrap(), bytes);
        assert_eq!(fs::read(target.join("observations.json")).unwrap(), bytes);
        let report: Value =
            serde_json::from_slice(&fs::read(target.join("fit.json")).unwrap()).unwrap();
        assert_eq!(report["intrinsics_fixed"], true);
        for key in [
            "device_binding_verified",
            "synchronization_verified",
            "physical_accuracy_qualified",
        ] {
            assert_eq!(report[key], false);
        }
        assert!(
            (report["fit"]["right_from_left"]["translation_mm"][0]
                .as_f64()
                .unwrap()
                + 60.)
                .abs()
                < 1e-5
        );
        for entry in manifest["artifacts"].as_array().unwrap() {
            let bytes = fs::read(target.join(entry["role"].as_str().unwrap())).unwrap();
            assert_eq!(
                entry["crc32"].as_u64().unwrap(),
                u64::from(crc32fast::hash(&bytes))
            );
            assert_eq!(entry["bytes"].as_u64().unwrap(), bytes.len() as u64);
        }
        let original = fs::read(target.join("fit.json")).unwrap();
        assert!(run(&input, &target, &WorkControl::default(), &|| false).is_err());
        assert_eq!(fs::read(target.join("fit.json")).unwrap(), original);
    }
    #[test]
    fn invalid_request_and_cancellation_do_not_publish() {
        let temp = Temporary::new();
        let input = temp.0.join("request.json");
        let target = temp.0.join("fit");
        let mut r = request();
        r.schema_version = 2;
        fs::write(&input, serde_json::to_vec(&r).unwrap()).unwrap();
        assert!(run(&input, &target, &WorkControl::default(), &|| false).is_err());
        assert!(!target.exists());
        r.schema_version = 1;
        fs::write(&input, serde_json::to_vec(&r).unwrap()).unwrap();
        assert!(run(&input, &target, &WorkControl::default(), &|| true).is_err());
        assert!(!target.exists());
        r.views[0].observations[0].right_pixel[0] = 10000.;
        fs::write(&input, serde_json::to_vec(&r).unwrap()).unwrap();
        assert!(run(&input, &target, &WorkControl::default(), &|| false).is_err());
        assert!(!target.exists());
    }
}
