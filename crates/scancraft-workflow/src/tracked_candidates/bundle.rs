//! Read-only fused diagnostics with artifact checks and original source snapshots.
mod rows;
#[cfg(test)]
mod tests;
use super::{Error, Request};
use crate::{
    bounded_json::Items,
    candidate_bundle::{self, FrameView},
    scan_project_load::Guard,
};
pub use rows::{FrameOutcome, FrameStatus};
use scancraft_calibration::stereo::StereoCalibration;
use scancraft_geometry::work::WorkControl;
use serde::Deserialize;
use std::{collections::HashSet, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    role: String,
    bytes: u64,
    crc32: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    kind: String,
    frames: usize,
    integrated_frames: usize,
    tracking_only_frames: usize,
    tracking_rejected_frames: usize,
    tracking_unavailable_frames: usize,
    surface_candidates: usize,
    fused_candidates: usize,
    stereo_attempts: usize,
    units: String,
    units_verified: bool,
    physical_accuracy_qualified: bool,
    synchronization_verified: bool,
    marker_identity_verified: bool,
    projector_line_identity_verified: bool,
    marker_points_fused: bool,
    source_bundles_required: bool,
    artifacts: Items<Artifact, 6>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Points {
    schema_version: u32,
    coordinate_frame: String,
    positions_model_units: Items<[f64; 3], 1_000_000>,
    units_verified: bool,
    physical_accuracy_qualified: bool,
    marker_points_fused: bool,
}
#[derive(Debug)]
pub struct SessionView {
    pub frames: Vec<FrameOutcome>,
    pub selected_frame: usize,
    pub integrated: usize,
    pub tracking_only: usize,
    pub rejected: usize,
    pub unavailable: usize,
}
pub struct Loaded {
    pub positions: Vec<[f64; 3]>,
    pub cameras: [FrameView; 2],
    pub session: SessionView,
}

/// Validate fixed-layout output and source snapshots, without recomputing any
/// marker, pose, laser or voxel mathematics. Never creates a metric Editor.
/// Original source bundles must still exist at their recorded absolute paths.
/// Frame selection affects camera thumbnails, not the complete fused point view.
pub fn load(directory: &Path, index: usize, work: &WorkControl) -> Result<Loaded, Error> {
    load_inner(directory, index, work, &|| Ok(()))
}
fn load_inner(
    directory: &Path,
    index: usize,
    work: &WorkControl,
    before_final: &dyn Fn() -> Result<(), Error>,
) -> Result<Loaded, Error> {
    work.check()?;
    let mut guard = Guard::default();
    guard.open(directory, true, 0)?;
    let manifest: Manifest = serde_json::from_slice(
        &guard
            .read(&directory.join("manifest.json"), 1_048_576, true, work)?
            .0,
    )?;
    if manifest.schema_version != 1
        || manifest.kind != "recorded_marker_laser_fusion"
        || !(1..=256).contains(&manifest.frames)
        || index >= manifest.frames
        || manifest.fused_candidates > 1_000_000
        || manifest.stereo_attempts > 1_000_000
        || manifest.surface_candidates > 256 * 4096
        || manifest.units != "unverified_supplied_geometry_units"
        || !manifest.source_bundles_required
        || [
            manifest.units_verified,
            manifest.physical_accuracy_qualified,
            manifest.synchronization_verified,
            manifest.marker_identity_verified,
            manifest.projector_line_identity_verified,
            manifest.marker_points_fused,
        ]
        .into_iter()
        .any(|v| v)
        || [
            manifest.integrated_frames,
            manifest.tracking_only_frames,
            manifest.tracking_rejected_frames,
            manifest.tracking_unavailable_frames,
        ]
        .into_iter()
        .any(|n| n > manifest.frames)
        || manifest.integrated_frames
            + manifest.tracking_only_frames
            + manifest.tracking_rejected_frames
            + manifest.tracking_unavailable_frames
            != manifest.frames
    {
        return Err("invalid fused candidate schema, index, counts or qualification flags".into());
    }
    let mut seen = HashSet::new();
    let mut request = None;
    let mut geometry = None;
    let mut points = None;
    for a in manifest.artifacts.0 {
        work.check()?;
        let (limit, retain) = match a.role.as_str() {
            "request.json" | "geometry.json" => (1_048_576, true),
            "frames.ndjson" => (2 * 1024 * 1024, false),
            "sources.ndjson" => (512 * 1024 * 1024, false),
            "markers.ndjson" => (1024 * 1024 * 1024, false),
            "fused-candidates.json" => (128 * 1024 * 1024, true),
            _ => return Err("unexpected fused candidate artifact role/path".into()),
        };
        if !seen.insert(a.role.clone()) || a.bytes > limit {
            return Err("duplicate or oversized fused candidate artifact".into());
        }
        let (bytes, size, crc) = guard.read(&directory.join(&a.role), limit, retain, work)?;
        if size != a.bytes || crc != a.crc32 {
            return Err(format!("fused artifact length/CRC mismatch: {}", a.role).into());
        }
        match a.role.as_str() {
            "request.json" => request = Some(bytes),
            "geometry.json" => geometry = Some(bytes),
            "fused-candidates.json" => points = Some(bytes),
            _ => {}
        }
    }
    if seen.len() != 6 {
        return Err("fused bundle requires exactly six artifacts".into());
    }
    let request: Request = serde_json::from_slice(&request.ok_or("missing fused request")?)?;
    if request.schema_version != 1
        || request.frames.0.len() != manifest.frames
        || request.frames.0.windows(2).any(|w| w[0].id >= w[1].id)
        || !(1..=1_000_000).contains(&request.maximum_stereo_attempts)
        || !(1..=1_000_000).contains(&request.maximum_voxels)
        || manifest.fused_candidates > request.maximum_voxels
        || manifest.stereo_attempts > request.maximum_stereo_attempts
    {
        return Err("fused request and manifest disagree".into());
    }
    // Validate the same tracking/fusion quality gates and capacity without running it.
    let _session = scancraft_geometry::session::ScanSession::new(
        request.session_id.clone(),
        request.tracking.config(),
        request.voxel_model_units,
        request.maximum_voxels,
        manifest.frames,
    )?;
    let geometry: StereoCalibration =
        serde_json::from_slice(&geometry.ok_or("missing fused geometry")?)?;
    geometry.validate()?;
    let points: Points = serde_json::from_slice(&points.ok_or("missing fused points")?)?;
    if points.schema_version != 1
        || points.coordinate_frame != "first accepted tracking frame"
        || points.units_verified
        || points.physical_accuracy_qualified
        || points.marker_points_fused
        || points.positions_model_units.0.len() != manifest.fused_candidates
        || points
            .positions_model_units
            .0
            .iter()
            .flatten()
            .any(|v| !v.is_finite())
    {
        return Err("invalid fused coordinates, count or qualification flags".into());
    }
    work.check()?;
    let mut frames = Vec::new();
    rows::read(
        guard.open(&directory.join("frames.ndjson"), false, 2 * 1024 * 1024)?,
        256,
        8192,
        work,
        |row: FrameOutcome| {
            frames.push(row);
            Ok(())
        },
    )?;
    let mut counts = [0usize; 4];
    let mut total_surface = 0;
    let mut last = None;
    let mut voxels = 0;
    if frames.len() != manifest.frames {
        return Err("fused frame row count mismatch".into());
    }
    for (frame, input) in frames.iter().zip(&request.frames.0) {
        work.check()?;
        frame.validate(input.id, &request, &mut last, &mut voxels)?;
        counts[frame.status.counter()] += 1;
        total_surface += frame.surface_points;
    }
    if counts
        != [
            manifest.integrated_frames,
            manifest.tracking_only_frames,
            manifest.tracking_rejected_frames,
            manifest.tracking_unavailable_frames,
        ]
        || total_surface != manifest.surface_candidates
        || voxels != manifest.fused_candidates
    {
        return Err("fused frame outcomes and manifest totals disagree".into());
    }
    let mut selected = None;
    let mut source_index = 0;
    let mut interfaces = None;
    let mut timestamps = None;
    rows::read(
        guard.open(&directory.join("sources.ndjson"), false, 512 * 1024 * 1024)?,
        256,
        2 * 1024 * 1024,
        work,
        |source: rows::Source| {
            let frame = frames
                .get(source_index)
                .ok_or("too many fused source rows")?;
            if source.frame_id != frame.frame_id || !source.bundle.is_absolute() {
                return Err("invalid fused source order or path".into());
            }
            let pair = candidate_bundle::load_recorded_pair(&source.bundle, work)?;
            if pair.manifest != source.manifest
                || pair.geometry != geometry
                || pair.candidates.report != source.validation
                || pair.candidates.first_observations != source.first_observations.0
                || pair.candidates.positions.len() != frame.surface_points
            {
                return Err(
                    "fused source bundle changed or disagrees with recorded provenance".into(),
                );
            }
            let order = pair.frames.each_ref().map(|f| f.profile.interface);
            let time = pair.frames.each_ref().map(|f| f.frame.host_timestamp_us);
            if interfaces.is_some_and(|v| v != order)
                || timestamps.is_some_and(|v: [u64; 2]| time[0] <= v[0] || time[1] <= v[1])
            {
                return Err(
                    "fused source interface order changed or timestamps did not advance".into(),
                );
            }
            interfaces = Some(order);
            timestamps = Some(time);
            for (actual, expected) in pair.frames.iter().zip(source.frames) {
                expected.check(actual)?;
            }
            if source_index == index {
                selected = Some(pair.candidates.frames);
            }
            source_index += 1;
            Ok(())
        },
    )?;
    if source_index != manifest.frames {
        return Err("fused source row count mismatch".into());
    }
    before_final()?;
    guard.check()?;
    work.check()?;
    Ok(Loaded {
        positions: points.positions_model_units.0,
        cameras: selected.ok_or("missing selected fused source")?,
        session: SessionView {
            frames,
            selected_frame: index,
            integrated: counts[0],
            tracking_only: counts[1],
            rejected: counts[2],
            unavailable: counts[3],
        },
    })
}
