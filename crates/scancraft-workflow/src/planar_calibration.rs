//! Explicit, offline-only observation fitting. Never installs capture calibration.
use crate::{
    Error, bounded,
    calibration_bundle::{check, ensure_new, publish_fit},
};
use scancraft_calibration::{
    planar::{PlanarView, initialize_planar},
    refinement::{RefinementOptions, refine_planar},
};
use scancraft_geometry::work::WorkControl;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[cfg(test)]
use std::fs::{self, File};
use std::path::Path;

pub const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
pub use crate::calibration_bundle::ObservationProvenance;
/// Provenance is a supplied declaration, never device/accuracy attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub provenance: ObservationProvenance,
    pub image_size: [u32; 2],
    pub views: Vec<PlanarView>,
    /// A coarse seed limit; final acceptance uses refinement's individual limit.
    pub initial_max_reprojection_error_px: f64,
    pub refinement: RefinementOptions,
}
/// Retain exact request bytes and a diagnostic fit in an atomic no-overwrite bundle.
/// Filesystem I/O and linear-initializer SVD are not preempted inside their calls;
/// refinement checks cancellation per view/256 observations and damping trial.
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
        return Err("unsupported planar observation schema".into());
    }
    check(work, cancel)?;
    let initial = initialize_planar(
        request.image_size,
        &request.views,
        request.initial_max_reprojection_error_px,
    )?;
    check(work, cancel)?;
    let fit = refine_planar(&request.views, &initial, &request.refinement, &|| {
        work.check().is_err() || cancel()
    })?;
    check(work, cancel)?;
    let report = json!({"schema_version":1,"scope":"offline numerical fit of supplied observations; not qualified or installed capture calibration",
        "provenance":request.provenance,"device_binding_verified":false,"physical_accuracy_qualified":false,
        "initialization":initial,"refinement":fit});
    publish_fit(
        &bytes,
        &report,
        target,
        work,
        cancel,
        "offline planar Brown5 fit only",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_calibration::{Brown5, CameraModel, planar::PlanarObservation};
    use std::sync::atomic::{AtomicU64, Ordering};
    struct Temporary(std::path::PathBuf);
    impl Temporary {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let p = std::env::temp_dir().join(format!(
                "scancraft-planar-workflow-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
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
        for i in 0..8 {
            let (sx, cx) = (0.4 * (i as f64 * 0.8).sin()).sin_cos();
            let (sy, cy) = (0.35 * (i as f64 * 0.6).cos()).sin_cos();
            let mut observations = Vec::new();
            for y in 0..7 {
                for x in 0..9 {
                    let xy = [(x as f64 - 4.) * 20., (y as f64 - 3.) * 20.];
                    let point = [
                        cy * xy[0] + sy * sx * xy[1] - 15. + i as f64 * 3.,
                        cx * xy[1],
                        -sy * xy[0] + cy * sx * xy[1] + 240. + i as f64 * 12.,
                    ];
                    observations.push(PlanarObservation {
                        target_xy_mm: xy,
                        pixel: camera.project(point).unwrap(),
                    });
                }
            }
            views.push(PlanarView { observations });
        }
        Request {
            schema_version: 1,
            provenance: ObservationProvenance::SyntheticFixture,
            image_size: [800, 600],
            views,
            initial_max_reprojection_error_px: 10.,
            refinement: RefinementOptions::default(),
        }
    }
    #[test]
    fn publishes_exact_request_and_unqualified_fit_without_overwriting() {
        let temp = Temporary::new();
        let input = temp.0.join("input.json");
        let target = temp.0.join("result");
        let bytes = serde_json::to_vec_pretty(&request()).unwrap();
        fs::write(&input, &bytes).unwrap();
        let report = run(&input, &target, &WorkControl::default(), &|| false).unwrap();
        assert_eq!(fs::read(target.join("observations.json")).unwrap(), bytes);
        assert_eq!(fs::read(&input).unwrap(), bytes);
        let fit: Value =
            serde_json::from_slice(&fs::read(target.join("fit.json")).unwrap()).unwrap();
        assert_eq!(fit["physical_accuracy_qualified"], false);
        assert_eq!(fit["device_binding_verified"], false);
        assert!(fit["refinement"]["rms_px"].as_f64().unwrap() < 1e-6);
        for entry in report["artifacts"].as_array().unwrap() {
            let b = fs::read(target.join(entry["role"].as_str().unwrap())).unwrap();
            assert_eq!(entry["bytes"].as_u64().unwrap(), b.len() as u64);
            assert_eq!(
                entry["crc32"].as_u64().unwrap(),
                u64::from(crc32fast::hash(&b))
            );
        }
        let before = fs::read(target.join("fit.json")).unwrap();
        assert!(run(&input, &target, &WorkControl::default(), &|| false).is_err());
        assert_eq!(fs::read(target.join("fit.json")).unwrap(), before);
    }
    #[test]
    fn invalid_schema_oversize_and_cancellation_do_not_publish() {
        let temp = Temporary::new();
        let input = temp.0.join("input.json");
        let target = temp.0.join("result");
        let mut r = request();
        r.schema_version = 2;
        fs::write(&input, serde_json::to_vec(&r).unwrap()).unwrap();
        assert!(run(&input, &target, &WorkControl::default(), &|| false).is_err());
        assert!(!target.exists());
        r.schema_version = 1;
        fs::write(&input, serde_json::to_vec(&r).unwrap()).unwrap();
        assert!(run(&input, &target, &WorkControl::default(), &|| true).is_err());
        assert!(!target.exists());
        let file = File::create(&input).unwrap();
        file.set_len(MAX_REQUEST_BYTES as u64 + 1).unwrap();
        drop(file);
        assert!(run(&input, &target, &WorkControl::default(), &|| false).is_err());
        assert!(!target.exists());
    }
}
