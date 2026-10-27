//! Explicit offline target measurements; never installs or activates calibration.
use crate::{
    Error, artifact, bounded,
    calibration_bundle::{ObservationProvenance, check, ensure_new},
    gray_png,
    output::{publish, staging},
};
use scancraft_calibration::planar::{PlanarObservation, PlanarView};
use scancraft_geometry::work::{Stage, WorkControl};
use scancraft_image::{
    GrayImage,
    corners::{Options, refine_corner},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashSet, fs::File, io::Write, path::Path};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub radius: u32,
    pub max_iterations: u32,
    pub epsilon_px: f64,
    pub max_shift_px: f64,
    pub min_eigenvalue: f64,
    pub min_eigenvalue_ratio: f64,
}
impl From<&Settings> for Options {
    fn from(s: &Settings) -> Self {
        Self {
            radius: s.radius,
            max_iterations: s.max_iterations,
            epsilon_px: s.epsilon_px,
            max_shift_px: s.max_shift_px,
            min_eigenvalue: s.min_eigenvalue,
            min_eigenvalue_ratio: s.min_eigenvalue_ratio,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub provenance: ObservationProvenance,
    pub image_size: [u32; 2],
    /// Supplied target associations, not automatically detected board coordinates.
    pub view: PlanarView,
    pub settings: Settings,
}
/// Retain exact PNG/request and publish refined measurements without overwriting.
/// All supplied corners must succeed; no silent dropping/reordering of associations.
/// Decoding and filesystem calls are bounded in size but not wall-clock-preempted.
pub fn run(
    image_path: &Path,
    input: &Path,
    target: &Path,
    work: &WorkControl,
    cancelled: &dyn Fn() -> bool,
) -> Result<Value, Error> {
    check(work, cancelled)?;
    ensure_new(target)?;
    let request_bytes = bounded(input, 4 * 1024 * 1024, work)?;
    let request: Request = serde_json::from_slice(&request_bytes)?;
    if request.schema_version != 1 || !(1..=4096).contains(&request.view.observations.len()) {
        return Err("invalid corner request schema or observation count".into());
    }
    let mut targets = HashSet::new();
    for observation in &request.view.observations {
        let xy = observation.target_xy_mm;
        if !xy.iter().all(|x| x.is_finite() && x.abs() <= 1e9)
            || !targets.insert(xy.map(|x| if x == 0. { 0 } else { x.to_bits() }))
        {
            return Err("nonfinite, excessive or duplicate target association".into());
        }
    }
    let image_bytes = bounded(image_path, 16 * 1024 * 1024, work)?;
    let pixels = gray_png(&image_bytes, request.image_size)?;
    let image = GrayImage::new(request.image_size[0], request.image_size[1], &pixels)?;
    let mut observations = Vec::with_capacity(request.view.observations.len());
    let mut details = Vec::with_capacity(request.view.observations.len());
    for (index, observation) in request.view.observations.iter().enumerate() {
        check(work, cancelled)?;
        work.update(Stage::Processing, index, request.view.observations.len())?;
        let refined = refine_corner(
            image,
            observation.pixel,
            Options::from(&request.settings),
            &|| work.check().is_err() || cancelled(),
        )
        .map_err(|e| format!("corner {index}: {e}"))?;
        observations.push(PlanarObservation {
            target_xy_mm: observation.target_xy_mm,
            pixel: refined.pixel,
        });
        details.push(
            json!({"observation_index":index,"seed":observation.pixel,"pixel":refined.pixel,
            "iterations":refined.iterations,"min_eigenvalue":refined.min_eigenvalue,
            "eigenvalue_ratio":refined.eigenvalue_ratio}),
        );
    }
    check(work, cancelled)?;
    let view = PlanarView { observations };
    let diagnostics = json!({"schema_version":1,"scope":"offline seeded corner measurements only",
        "provenance":request.provenance,"device_binding_verified":false,"physical_accuracy_qualified":false,
        "target_associations_verified":false,"measurements":details});
    let partial = staging(target)?;
    let result=(||->Result<Value,Error> {
        let image_digest=artifact(&partial,"image.png",work,|out|{out.write_all(&image_bytes)?;Ok(())})?;
        check(work,cancelled)?;
        let request_digest=artifact(&partial,"request.json",work,|out|{out.write_all(&request_bytes)?;Ok(())})?;
        let view_digest=artifact(&partial,"refined_view.json",work,|out|{serde_json::to_writer_pretty(out,&view)?;Ok(())})?;
        let diagnostic_digest=artifact(&partial,"diagnostics.json",work,|out|{serde_json::to_writer_pretty(out,&diagnostics)?;Ok(())})?;
        let manifest=json!({"schema_version":1,"kind":"offline_seeded_target_corner_measurements",
            "artifacts":[image_digest,request_digest,view_digest,diagnostic_digest],
            "device_binding_verified":false,"physical_accuracy_qualified":false,"target_associations_verified":false});
        artifact(&partial,"manifest.json",work,|out|{serde_json::to_writer_pretty(out,&manifest)?;Ok(())})?;
        File::open(&partial)?.sync_all()?;check(work,cancelled)?;publish(&partial,target)?;
        Ok(manifest)
    })().map_err(|e|format!("{e}; incomplete corner measurements retained at {}",partial.display()))?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent)?.sync_all().map_err(|e| {
        format!(
            "measurements published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    struct Temp(std::path::PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let p = std::env::temp_dir().join(format!(
                "scancraft-corners-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn inputs(temp: &Temp) -> (std::path::PathBuf, std::path::PathBuf, Request) {
        let image_path = temp.0.join("input.png");
        let request_path = temp.0.join("input.json");
        let image = image::GrayImage::from_fn(64, 64, |x, y| {
            let a = ((f64::from(x) - 30.3) / 1.2).tanh();
            let b = ((f64::from(y) - 31.7) / 1.2).tanh();
            image::Luma([(128. + 95. * a * b).round() as u8])
        });
        image.save(&image_path).unwrap();
        let request = Request {
            schema_version: 1,
            provenance: ObservationProvenance::SyntheticFixture,
            image_size: [64, 64],
            view: PlanarView {
                observations: vec![PlanarObservation {
                    target_xy_mm: [10., 20.],
                    pixel: [31., 31.],
                }],
            },
            settings: Settings {
                radius: 5,
                max_iterations: 40,
                epsilon_px: 1e-4,
                max_shift_px: 4.,
                min_eigenvalue: 1e-5,
                min_eigenvalue_ratio: 0.01,
            },
        };
        fs::write(&request_path, serde_json::to_vec_pretty(&request).unwrap()).unwrap();
        (image_path, request_path, request)
    }
    #[test]
    fn publishes_exact_sources_refined_association_and_checked_artifacts() {
        let temp = Temp::new();
        let (image, input, _) = inputs(&temp);
        let target = temp.0.join("out");
        let result = run(&image, &input, &target, &WorkControl::default(), &|| false).unwrap();
        assert_eq!(
            fs::read(&image).unwrap(),
            fs::read(target.join("image.png")).unwrap()
        );
        assert_eq!(
            fs::read(&input).unwrap(),
            fs::read(target.join("request.json")).unwrap()
        );
        let view: PlanarView =
            serde_json::from_slice(&fs::read(target.join("refined_view.json")).unwrap()).unwrap();
        assert_eq!(view.observations[0].target_xy_mm, [10., 20.]);
        assert!((view.observations[0].pixel[0] - 30.3).abs() < 0.04);
        assert_eq!(result["physical_accuracy_qualified"], false);
        assert_eq!(result["device_binding_verified"], false);
        for entry in result["artifacts"].as_array().unwrap() {
            let bytes = fs::read(target.join(entry["role"].as_str().unwrap())).unwrap();
            assert_eq!(entry["bytes"].as_u64().unwrap(), bytes.len() as u64);
            assert_eq!(
                entry["crc32"].as_u64().unwrap(),
                u64::from(crc32fast::hash(&bytes))
            );
        }
        let saved = fs::read(target.join("refined_view.json")).unwrap();
        assert!(run(&image, &input, &target, &WorkControl::default(), &|| false).is_err());
        assert_eq!(saved, fs::read(target.join("refined_view.json")).unwrap());
    }
    #[test]
    fn rejection_and_cancellation_never_publish_partial_measurements() {
        let temp = Temp::new();
        let (image, input, mut request) = inputs(&temp);
        let target = temp.0.join("out");
        assert!(run(&image, &input, &target, &WorkControl::default(), &|| true).is_err());
        assert!(!target.exists());
        request.view.observations[0].pixel = [1., 1.];
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        assert!(run(&image, &input, &target, &WorkControl::default(), &|| false).is_err());
        assert!(!target.exists());
        request
            .view
            .observations
            .push(request.view.observations[0].clone());
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        assert!(
            run(&image, &input, &target, &WorkControl::default(), &|| false)
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );
        assert!(!target.exists());
    }
}
