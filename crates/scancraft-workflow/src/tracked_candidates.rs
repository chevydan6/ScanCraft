//! Experimental recorded marker tracking and laser-only fusion in unverified units.
//! No metric project/PLY conversion; source bundles remain required for provenance.
pub mod bundle;
mod markers;
mod recovery;
#[cfg(test)]
mod tests;
use crate::{
    ArtifactWriter, Error, artifact, bounded, bounded_json::Items, candidate_bundle, output,
};
use scancraft_geometry::{
    Cloud, Point, session::ScanSession, tracking::TrackingConfig, work::WorkControl,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MarkerConfig {
    threshold_exclusive: u8,
    minimum_area: u32,
    maximum_area: u32,
    maximum_components: usize,
    maximum_shape_rms_px: f64,
    minimum_depth_model_units: f64,
    maximum_depth_model_units: f64,
    minimum_ray_angle_rad: f64,
    maximum_ray_gap_model_units: f64,
    maximum_reprojection_error_px: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tracking {
    max_correspondence_model_units: f64,
    max_rms_model_units: f64,
    minimum_overlap: f64,
    max_step_translation_model_units: f64,
    max_step_rotation_rad: f64,
    iterations: usize,
    tolerance_model_units: f64,
}
impl Tracking {
    fn config(&self) -> TrackingConfig {
        // Reuse the unit-homogeneous numerical kernel without asserting physical mm.
        TrackingConfig {
            max_correspondence_mm: self.max_correspondence_model_units,
            max_rms_mm: self.max_rms_model_units,
            minimum_overlap: self.minimum_overlap,
            max_step_translation_mm: self.max_step_translation_model_units,
            max_step_rotation_rad: self.max_step_rotation_rad,
            iterations: self.iterations,
            tolerance_mm: self.tolerance_model_units,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    id: u64,
    bundle: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    session_id: String,
    markers: MarkerConfig,
    tracking: Tracking,
    voxel_model_units: f64,
    maximum_voxels: usize,
    maximum_stereo_attempts: usize,
    /// Experimental local consistency, not marker identification or relocalization.
    #[serde(default)]
    temporal_marker_support: bool,
    /// Opt-in recovery against session-owned retained marker references.
    #[serde(default)]
    retained_reference_recovery: bool,
    frames: Items<Frame, 256>,
}
fn cloud(positions: Vec<[f64; 3]>) -> Cloud {
    Cloud {
        points: positions
            .into_iter()
            .map(|position| Point {
                position,
                color: None,
                normal: None,
            })
            .collect(),
    }
}
fn line(out: &mut impl Write, value: &Value) -> Result<(), Error> {
    serde_json::to_writer(&mut *out, value)?;
    out.write_all(b"\n")?;
    Ok(())
}

/// Read immutable single-pair candidate bundles, derive marker observations from
/// their exact retained raw frames, and fuse only their laser surface candidates.
/// At most 256 frames, 512 observations per image, 1M stereo attempts and 1M voxels.
/// All image/geometry failures abort publication; insufficient observations and
/// rejected poses are explicit frame outcomes. Empty surface frames retain tracking.
/// Partial diagnostics survive failure/cancellation; the target is never overwritten.
pub fn run(settings: &Path, target: &Path, work: &WorkControl) -> Result<Value, Error> {
    work.check()?;
    if fs::symlink_metadata(target).is_ok() {
        return Err("tracked candidate output already exists".into());
    }
    let bytes = bounded(settings, 1_048_576, work)?;
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.schema_version != 1
        || request.frames.0.is_empty()
        || request.frames.0.windows(2).any(|f| f[0].id >= f[1].id)
        || !(1..=1_000_000).contains(&request.maximum_stereo_attempts)
        || !(1..=1_000_000).contains(&request.maximum_voxels)
        || !request.markers.maximum_shape_rms_px.is_finite()
        || request.markers.maximum_shape_rms_px <= 0.
        || (request.retained_reference_recovery && !request.temporal_marker_support)
    {
        return Err("invalid tracked candidate schema, order or resource/shape limits".into());
    }
    let mut session = ScanSession::new(
        request.session_id.clone(),
        request.tracking.config(),
        request.voxel_model_units,
        request.maximum_voxels,
        request.frames.0.len(),
    )?;
    session.start()?;
    let partial = output::staging(target)?;
    let result = (|| -> Result<Value, Error> {
        let mut artifacts = vec![artifact(&partial, "request.json", work, |out| {
            out.write_all(&bytes)?;
            Ok(())
        })?];
        let mut frames_out = ArtifactWriter::open(&partial, "frames.ndjson", work)?;
        let mut sources = ArtifactWriter::open(&partial, "sources.ndjson", work)?;
        let mut marker_file = ArtifactWriter::open(&partial, "markers.ndjson", work)?;
        let mut markers = recovery::LimitedWriter::new(&mut marker_file, recovery::MARKER_BYTES);
        let mut journal = request
            .retained_reference_recovery
            .then(|| ArtifactWriter::open(&partial, "recovery.ndjson", work))
            .transpose()?;
        let mut budget = request.maximum_stereo_attempts;
        let mut reference_geometry = None;
        let mut reference_interfaces = None;
        let mut last_timestamps = None;
        let mut integrated = 0;
        let mut tracking_only = 0;
        let mut rejected = 0;
        let mut unavailable = 0;
        let mut surface_points = 0;
        let mut marker_prior = None;
        let base = settings.parent().unwrap_or(Path::new("."));
        for frame in &request.frames.0 {
            work.check()?;
            let path = std::path::absolute(base.join(&frame.bundle))?;
            let pair = candidate_bundle::load_recorded_pair(&path, work)?;
            let interfaces = pair.frames.each_ref().map(|f| f.profile.interface);
            let timestamps = pair.frames.each_ref().map(|f| f.frame.host_timestamp_us);
            if reference_geometry
                .as_ref()
                .is_some_and(|g| g != &pair.geometry)
                || reference_interfaces.is_some_and(|v| v != interfaces)
                || last_timestamps
                    .is_some_and(|v: [u64; 2]| timestamps[0] <= v[0] || timestamps[1] <= v[1])
            {
                return Err("sequence geometry/interface order changed or source timestamps did not advance".into());
            }
            if reference_geometry.is_none() {
                artifacts.push(artifact(&partial, "geometry.json", work, |out| {
                    serde_json::to_writer_pretty(out, &pair.geometry)?;
                    Ok(())
                })?);
                reference_geometry = Some(pair.geometry.clone());
                reference_interfaces = Some(interfaces);
            }
            last_timestamps = Some(timestamps);
            line(
                &mut sources,
                &json!({"frame_id":frame.id,"bundle":path,"manifest":pair.manifest,
                "validation":pair.candidates.report,"first_observations":pair.candidates.first_observations,
                "frames":pair.frames.each_ref().map(|f|json!({"profile":f.profile,
                    "sequence":f.frame.sequence,"host_timestamp_us":f.frame.host_timestamp_us,
                    "sensor_pts":scancraft_core::uvc_pts(&f.frame.metadata).ok(),
                    "payload_crc32":crc32fast::hash(&f.frame.data),"metadata_crc32":crc32fast::hash(&f.frame.metadata)}))}),
            )?;
            line(
                &mut markers,
                &json!({"kind":"frame_start","frame_id":frame.id}),
            )?;
            let surface = cloud(pair.candidates.positions);
            surface_points += surface.points.len();
            let mut row = json!({"frame_id":frame.id,"surface_points":surface.points.len(),"integrated":false});
            let (step, points, error) = if let Some(journal) = &mut journal {
                let prepared = markers::prepare(
                    &pair.frames,
                    &pair.geometry,
                    &request.markers,
                    &mut budget,
                    &mut markers,
                    work,
                )?;
                let processed = recovery::process(
                    &mut session,
                    frame.id,
                    &surface,
                    &prepared,
                    &request,
                    (&mut markers, journal),
                    work,
                )?;
                row["recovery"] = processed.summary;
                (processed.step, processed.points, processed.error)
            } else {
                let observations = markers::reconstruct(
                    &pair.frames,
                    &pair.geometry,
                    &request.markers,
                    marker_prior
                        .as_ref()
                        .map(|prior| (prior, request.tracking.max_correspondence_model_units)),
                    &mut budget,
                    &mut markers,
                    work,
                )?;
                let count = observations.points.len();
                if count < 3 {
                    (None, count, None)
                } else {
                    let step = session
                        .process_with_tracking_cloud(frame.id, &surface, &observations, work)?
                        .clone();
                    if request.temporal_marker_support && step.tracking.accepted {
                        marker_prior = Some(markers::Prior {
                            frame_id: frame.id,
                            positions: observations.points.iter().map(|p| p.position).collect(),
                        });
                    }
                    (Some(step), count, None)
                }
            };
            row["tracking_points"] = json!(points);
            if let Some(step) = step {
                let t = &step.tracking;
                let pose=t.world_from_frame.as_ref().map(|p|json!({
                    "rotation":(0..3).map(|i|(0..3).map(|j|p.rotation[(i,j)]).collect::<Vec<_>>()).collect::<Vec<_>>(),
                    "translation_model_units":p.translation.as_slice()}));
                let status = if step.fusion.is_some() {
                    integrated += 1;
                    "integrated"
                } else if t.accepted {
                    tracking_only += 1;
                    "tracking_only"
                } else {
                    rejected += 1;
                    "tracking_rejected"
                };
                row["status"] = json!(status);
                row["integrated"] = json!(step.fusion.is_some());
                row["accepted"] = json!(t.accepted);
                row["reason"] = json!(t.reason);
                row["pose"] = json!(pose);
                row["rms_model_units"] = json!(t.rms_mm);
                row["source_overlap"] = json!(t.source_overlap);
                row["target_overlap"] = json!(t.target_overlap);
                row["fusion"] =
                    json!(step.fusion.as_ref().map(
                        |f| json!({"input_points":f.input_points,"total_voxels":f.total_voxels})
                    ));
            } else {
                unavailable += 1;
                row["status"] = json!(if points < 3 {
                    "insufficient_tracking_observations"
                } else {
                    "tracking_error"
                });
                if points >= 3 {
                    row["reason"] = json!(error);
                }
                row["pose"] = Value::Null;
                // The session did not observe a pose; retain its last accepted
                // reference, never substitute it as this frame's measurement.
            }
            row["last_integrated"] = json!(session.last_integrated());
            line(&mut frames_out, &row)?;
        }
        session.stop()?;
        artifacts.extend([
            frames_out.finish()?,
            sources.finish()?,
            marker_file.finish()?,
        ]);
        if let Some(journal) = journal {
            artifacts.push(journal.finish()?);
        }
        let fused = session.cloud(work)?;
        let positions: Vec<_> = fused.points.iter().map(|p| p.position).collect();
        artifacts.push(artifact(&partial,"fused-candidates.json",work,|out|{
            serde_json::to_writer(out,&json!({"schema_version":1,"coordinate_frame":"first accepted tracking frame",
                "positions_model_units":positions,"units_verified":false,"physical_accuracy_qualified":false,
                "marker_points_fused":false}))?;Ok(())})?);
        let summary = json!({"schema_version":1,"kind":"recorded_marker_laser_fusion",
            "frames":request.frames.0.len(),"integrated_frames":integrated,"tracking_only_frames":tracking_only,
            "tracking_rejected_frames":rejected,"tracking_unavailable_frames":unavailable,
            "surface_candidates":surface_points,"fused_candidates":positions.len(),
            "stereo_attempts":request.maximum_stereo_attempts-budget,
            "units":"unverified_supplied_geometry_units","units_verified":false,
            "physical_accuracy_qualified":false,"synchronization_verified":false,
            "marker_identity_verified":false,"projector_line_identity_verified":false,
            "marker_points_fused":false,"source_bundles_required":true,"artifacts":artifacts});
        artifact(&partial, "manifest.json", work, |out| {
            serde_json::to_writer_pretty(out, &summary)?;
            Ok(())
        })?;
        File::open(&partial)?.sync_all()?;
        work.check()?;
        output::publish(&partial, target)?;
        Ok(summary)
    })();
    let summary =
        result.map_err(|e| format!("{e}; incomplete output preserved at {}", partial.display()))?;
    File::open(
        target
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?
    .sync_all()
    .map_err(|e| {
        format!(
            "bundle published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(summary)
}
