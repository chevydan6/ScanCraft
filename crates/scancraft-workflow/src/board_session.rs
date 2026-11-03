//! Explicit multi-pose observation collection and optional independent camera fitting.
//! Validation poses are retained separately and never enter the fitted requests.
use crate::{
    Error, artifact, bounded,
    calibration_bundle::{check, ensure_new},
    calibration_observations::{self, TargetUnits},
    digest,
    output::{publish, staging},
    planar_calibration,
};
use scancraft_calibration::refinement::RefinementOptions;
use scancraft_geometry::work::WorkControl;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    /// Paths to explicit single-pair observation requests, relative to this file.
    pub training: Vec<PathBuf>,
    pub validation: Vec<PathBuf>,
    pub fit_intrinsics: bool,
    pub initial_max_reprojection_error_px: f64,
    pub refinement: RefinementOptions,
}
/// Creates a new session with complete per-pose raw snapshots and exact requests.
/// Numerical fitting requires declared millimeters;
/// This remains a declaration,
/// not physical verification. Camera domains/stereo pose are not guessed here.
pub fn run(
    input: &Path,
    target: &Path,
    work: &WorkControl,
    cancel: &dyn Fn() -> bool,
) -> Result<Value, Error> {
    check(work, cancel)?;
    ensure_new(target)?;
    let bytes = bounded(input, 1024 * 1024, work)?;
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.schema_version != 1
        || !(3..=32).contains(&request.training.len())
        || request.validation.len() > 32
        || !request.initial_max_reprojection_error_px.is_finite()
        || request.initial_max_reprojection_error_px <= 0.
    {
        return Err("board session requires 3..32 training poses, at most 32 validation poses and a positive initializer error limit".into());
    }
    let parent = input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut unique_paths = HashSet::new();
    for path in request.training.iter().chain(&request.validation) {
        check(work, cancel)?;
        if !unique_paths.insert(std::fs::canonicalize(parent.join(path))?) {
            return Err("training/validation must not reuse the same observation request".into());
        }
    }
    let partial = staging(target)?;
    let result = prepare(&request, &bytes, parent, &partial, work, cancel)
        .and_then(|manifest| {
            File::open(&partial)?.sync_all()?;
            check(work, cancel)?;
            publish(&partial, target)?;
            Ok(manifest)
        })
        .map_err(|e| {
            format!(
                "{e}; incomplete board session retained at {}",
                partial.display()
            )
        })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "board session published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(result)
}
fn prepare(
    request: &Request,
    bytes: &[u8],
    parent: &Path,
    partial: &Path,
    work: &WorkControl,
    cancel: &dyn Fn() -> bool,
) -> Result<Value, Error> {
    let mut artifacts = vec![artifact(partial, "session-request.json", work, |out| {
        out.write_all(bytes)?;
        Ok(())
    })?];
    let mut reference = None;
    let mut profiles = None;
    let mut fingerprints: [HashSet<String>; 2] = Default::default();
    let mut training = [Vec::new(), Vec::new()];
    let mut validation = [Vec::new(), Vec::new()];
    let mut stereo_training = Vec::new();
    let mut stereo_validation = Vec::new();
    let mut poses = Vec::new();
    for (split, paths) in [
        ("training", &request.training),
        ("validation", &request.validation),
    ] {
        for (index, path) in paths.iter().enumerate() {
            check(work, cancel)?;
            let name = format!("{split}-{index:02}");
            let completed = calibration_observations::run_with_frames(
                &parent.join(path),
                &partial.join(&name),
                work,
                cancel,
            )?;
            let current_profiles = [
                completed.frames[0].profile.clone(),
                completed.frames[1].profile.clone(),
            ];
            if let Some(expected) = &profiles {
                if expected != &current_profiles {
                    return Err("camera profiles changed across board session poses".into());
                }
            } else {
                profiles = Some(current_profiles);
            }
            if let Some(expected) = &reference {
                let expected: &calibration_observations::Request = expected;
                if expected.target != completed.request.target
                    || expected.target_units != completed.request.target_units
                {
                    return Err(
                        "target definition or unit declaration changed across board session poses"
                            .into(),
                    );
                }
            }
            let identity:Vec<_>=completed.frames.iter().map(|s|json!({"profile":s.profile,"sequence":s.frame.sequence,"host_timestamp_us":s.frame.host_timestamp_us,"sensor_pts":scancraft_core::uvc_pts(&s.frame.metadata).ok().flatten(),"metadata_crc32":crc32fast::hash(&s.frame.metadata),"payload_crc32":crc32fast::hash(&s.frame.data)})).collect();
            for side in 0..2 {
                if !fingerprints[side].insert(serde_json::to_string(&identity[side])?) {
                    return Err(
                        "training/validation reuse the same recorded camera-frame fingerprint"
                            .into(),
                    );
                }
            }
            let r = &completed.measurements;
            for side in 0..2 {
                let value = json!({"pose":name,"observations":r["views"][side]});
                if split == "training" {
                    training[side].push(value);
                } else {
                    validation[side].push(value);
                }
            }
            let stereo = json!({"pose":name,"observations":r["stereo_observations"]});
            if split == "training" {
                stereo_training.push(stereo);
            } else {
                stereo_validation.push(stereo);
            }
            poses.push(json!({"directory":name,"split":split,"request_path_display":parent.join(path).display().to_string(),"source_recordings":r["source_recordings"],"minimum_fit_point_count_met":r["minimum_fit_point_count_met"]}));
            let manifest_bytes = bounded(
                &partial.join(&name).join("manifest.json"),
                1024 * 1024,
                work,
            )?;
            artifacts.push(digest(&format!("{name}/manifest.json"), &manifest_bytes));
            if reference.is_none() {
                reference = Some(completed.request);
            }
        }
    }
    let reference = reference.ok_or("missing board session poses")?;
    let profiles = profiles.ok_or("missing board session profiles")?;
    let dataset = json!({"schema_version":1,"target_units":reference.target_units,"units_verified":false,
            "device_binding_verified":false,"physical_accuracy_qualified":false,"synchronization_verified":false,
            "profiles":profiles,"poses":poses,"training":training,"validation":validation,
            "stereo_training":stereo_training,"stereo_validation":stereo_validation,
            "validation_used_in_fitting":false});
    artifacts.push(artifact(partial, "dataset.json", work, |out| {
        serde_json::to_writer_pretty(out, &dataset)?;
        Ok(())
    })?);
    let fit_ready = reference.target_units == TargetUnits::DeclaredMillimeters
        && training.iter().all(|views| {
            views
                .iter()
                .all(|v| v["observations"].as_array().is_some_and(|o| o.len() >= 4))
        });
    if request.fit_intrinsics && !fit_ready {
        return Err("intrinsic fitting requires declared millimeters and at least four retained marks in every training camera view; observations retained, no units inferred".into());
    }
    if fit_ready {
        for (side, label) in ["left", "right"].into_iter().enumerate() {
            let views:Vec<_>=training[side].iter().map(|v|json!({"observations":v["observations"].as_array().unwrap().iter().map(|o|json!({"target_xy_mm":o["coordinates"],"pixel":o["pixel"]})).collect::<Vec<_>>()})).collect();
            let fit_request = json!({"schema_version":1,"provenance":"user_supplied_unverified","image_size":[profiles[side].width,profiles[side].height],"views":views,
                    "initial_max_reprojection_error_px":request.initial_max_reprojection_error_px,"refinement":request.refinement});
            let file = format!("{label}-intrinsics-request.json");
            artifacts.push(artifact(partial, &file, work, |out| {
                serde_json::to_writer_pretty(out, &fit_request)?;
                Ok(())
            })?);
            if request.fit_intrinsics {
                check(work, cancel)?;
                planar_calibration::run(
                    &partial.join(&file),
                    &partial.join(format!("{label}-fit")),
                    work,
                    cancel,
                )?;
                let name = format!("{label}-fit/manifest.json");
                artifacts.push(digest(
                    &name,
                    &bounded(&partial.join(&name), 1024 * 1024, work)?,
                ));
            }
        }
    }
    let manifest = json!({"schema_version":1,"kind":"recorded_board_session","artifacts":artifacts,
            "training_poses":request.training.len(),"validation_poses":request.validation.len(),
            "intrinsic_requests_exported":fit_ready,"intrinsics_fitted":request.fit_intrinsics,
            "target_units":reference.target_units,"units_verified":false,"physical_accuracy_qualified":false,
            "calibration_activated":false,"validation_used_in_fitting":false});
    artifact(partial, "manifest.json", work, |out| {
        serde_json::to_writer_pretty(out, &manifest)?;
        Ok(())
    })?;
    Ok(manifest)
}
#[cfg(test)]
#[path = "board_session/tests.rs"]
mod tests;
