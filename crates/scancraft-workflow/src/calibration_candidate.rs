//! Fit supplied stereo observations and publish a reconstruction-readable candidate.
//! Explicit units/binding are declarations; this never installs or qualifies a calibration.
use crate::{
    Error, artifact, bounded,
    calibration_bundle::{ObservationProvenance, check, ensure_new},
    calibration_observations::TargetUnits,
    output::{publish, staging},
    stereo_calibration,
};
use scancraft_calibration::{
    stereo::StereoCalibration,
    stereo_fit::fit_stereo_extrinsics,
    storage::{self, CalibrationRecord, DeviceBinding, Provenance},
};
use scancraft_geometry::work::WorkControl;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs::File, io::Write, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub record_id: String,
    pub target_units: TargetUnits,
    /// Describe the supplied coordinate unit reference, not an inferred scale.
    pub unit_reference: String,
    pub binding: Option<DeviceBinding>,
    pub observations: stereo_calibration::Request,
}

/// Keep exact inputs beside an unqualified RAPCAL01 record; no hardware is opened.
/// Unknown units are rejected rather than silently becoming millimeters. The
/// supplied coordinate values, lens domains and fit limits are never rescaled.
pub fn run(
    input: &Path,
    target: &Path,
    work: &WorkControl,
    cancel: &dyn Fn() -> bool,
) -> Result<Value, Error> {
    check(work, cancel)?;
    ensure_new(target)?;
    let bytes = bounded(input, stereo_calibration::MAX_REQUEST_BYTES, work)?;
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.schema_version != 1
        || request.observations.schema_version != 1
        || request.target_units != TargetUnits::DeclaredMillimeters
        || request.unit_reference.trim().is_empty()
        || request.unit_reference.len() > 2048
    {
        return Err("candidate requires schema1, explicitly declared millimeters and a nonempty unit reference of at most 2048 bytes; no scale is inferred".into());
    }
    let provenance = match request.observations.provenance {
        ObservationProvenance::SyntheticFixture => Provenance::SyntheticFixture,
        ObservationProvenance::UserSuppliedUnverified => Provenance::EstimatedFromObservations,
    };
    let fit = fit_stereo_extrinsics(
        &request.observations.fixed_left,
        &request.observations.fixed_right,
        &request.observations.views,
        &request.observations.fitting,
        &|| work.check().is_err() || cancel(),
    )?;
    let record = CalibrationRecord {
        schema_version: 1,
        record_id: request.record_id,
        units: "millimeters".into(),
        camera_axes: "x_right_y_down_z_forward".into(),
        provenance,
        binding: request.binding,
        geometry: StereoCalibration {
            left: fit.fixed_left.clone(),
            right: fit.fixed_right.clone(),
            right_from_left: fit.right_from_left.clone(),
        },
        notes: format!(
            "Unqualified fitted candidate; units and device binding are supplied declarations, not verified measurements. Input request.json: {} bytes, CRC32 {:08x}. Retain the complete candidate bundle. Unit reference: {}",
            bytes.len(),
            crc32fast::hash(&bytes),
            request.unit_reference
        ),
    };
    let mut calibration = Vec::new();
    storage::write(&record, &mut calibration)?;
    // Round-trip the exact reconstruction input before publishing any candidate.
    if storage::read(calibration.as_slice())? != record {
        return Err("candidate calibration round-trip differs".into());
    }
    check(work, cancel)?;
    let report = json!({"schema_version":1,"scope":"fitted calibration candidate; not activated or physically qualified",
        "provenance":record.provenance,"target_units":request.target_units,"unit_reference":request.unit_reference,
        "units_verified":false,"device_binding_verified":false,"synchronization_verified":false,
        "physical_accuracy_qualified":false,"calibration_activated":false,"fit":fit});
    let partial = staging(target)?;
    let result = (|| -> Result<Value, Error> {
        let mut artifacts = Vec::new();
        for (name, data) in [
            ("request.json", bytes),
            ("calibration.rapcal", calibration),
            ("fit.json", serde_json::to_vec_pretty(&report)?),
        ] {
            check(work, cancel)?;
            artifacts.push(artifact(&partial, name, work, |out| {
                out.write_all(&data)?;
                Ok(())
            })?);
        }
        let manifest = json!({"schema_version":1,"kind":"fitted_calibration_candidate","artifacts":artifacts,
            "record_id":record.record_id,"provenance":record.provenance,"units":"millimeters",
            "units_verified":false,"device_binding_verified":false,"synchronization_verified":false,
            "physical_accuracy_qualified":false,"calibration_activated":false});
        artifact(&partial, "manifest.json", work, |out| {
            serde_json::to_writer_pretty(out, &manifest)?;
            Ok(())
        })?;
        File::open(&partial)?.sync_all()?;
        check(work, cancel)?;
        publish(&partial, target)?;
        Ok(manifest)
    })()
    .map_err(|e| format!("{e}; incomplete calibration candidate retained at {}", partial.display()))?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "calibration candidate published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(result)
}

#[cfg(test)]
mod tests;
