//! Completed capture -> one read-only diagnostic pair. Never opens scanner hardware.
mod record;
use super::{MAX_INPUT, MAX_TERMINAL, Plan, log};
use crate::{
    Error,
    bounded_json::Items,
    candidate_bundle::{FrameView, Loaded},
    laser_stream::pair_journal::{self, Limits, Mode},
    recording_playback::Playback,
    scan_project_load::Guard,
};
use scancraft_geometry::work::WorkControl;
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashSet, fs, path::Path};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    role: String,
    bytes: u64,
    crc32: u32,
}
const MAX_ARTIFACTS: usize = pair_journal::MAX_SEGMENTS + 6;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    kind: String,
    artifacts: Items<Artifact, MAX_ARTIFACTS>,
}
#[derive(Deserialize)]
struct Source {
    camera_index: usize,
    interface: u8,
    opened: bool,
    cleanup_confirmed: bool,
    journal_finalized: bool,
    journaled_frames: u64,
    queued_frames: u64,
    processing_queue_drops: u64,
    outcome: Result<(), String>,
}
#[derive(Deserialize)]
struct Summary {
    sources: [Source; 2],
    processed_pairs: u64,
    failed_processing_pairs: u64,
    physical_accuracy_qualified: bool,
    synchronization_verified: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalSummary {
    format: String,
    index: String,
    segments: usize,
    pairs: u64,
    bytes: u64,
}
#[derive(Deserialize)]
struct CaptureReport {
    schema_version: u32,
    kind: String,
    summary: Summary,
    raw_replay_integrity: String,
    #[serde(default)]
    reported_source_restoration: Option<String>,
    reported_stream_settings_restoration: String,
    geometry_binding_verified: bool,
    units_verified: bool,
    projector_line_identity_verified: bool,
    synchronization_verified: bool,
    physical_accuracy_qualified: bool,
    geometry_exported: bool,
    #[serde(default)]
    pair_journal: Option<JournalSummary>,
}
fn segment_number(name: &str) -> Option<usize> {
    let n = name.strip_prefix("pairs-")?.strip_suffix(".ndjson")?;
    if n.len() != 6 || !n.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    n.parse::<usize>()
        .ok()
        .filter(|&n| n < pair_journal::MAX_SEGMENTS)
}

/// Verify a complete schema-1/2 capture, its raw recordings and journal, then
/// bind one stored pair to its exact raw frames. No numerical recomputation,
/// physical attestation, recovery/publication or editing state is performed.
/// Reads are linear in capture size, on a caller-owned cancellable worker.
pub fn load_pair(directory: &Path, pair_index: u64, work: &WorkControl) -> Result<Loaded, Error> {
    load_inner(directory, pair_index, work, &|| Ok(()))
}
fn load_inner(
    directory: &Path,
    pair_index: u64,
    work: &WorkControl,
    before_final: &dyn Fn() -> Result<(), Error>,
) -> Result<Loaded, Error> {
    let mut loaded = None;
    read_inner(
        directory,
        Some(pair_index),
        work,
        &mut |_, pair| {
            loaded = Some(pair);
            Ok(())
        },
        before_final,
    )?;
    loaded.ok_or_else(|| "selected pair was not found".into())
}

/// Visit every stored pair in order, using one raw index per camera. Checks the
/// complete capture/journal before callbacks, then binds each pair to raw frames.
/// Memory retained by this reader is bounded by the two recording indexes, one
/// record and one pair of frames/views; callback-owned memory is the caller's duty.
///
/// Callbacks are PROVISIONAL: a later pair, callback, cancellation or final source
/// identity check can fail. Stage outputs and publish only after this returns Ok.
/// Does not recompute matching, qualify units, export geometry or access hardware.
/// Empty captures are rejected as by load_pair; filesystem reads have no deadline.
pub fn visit_pairs(
    directory: &Path,
    work: &WorkControl,
    mut visitor: impl FnMut(u64, Loaded) -> Result<(), Error>,
) -> Result<u64, Error> {
    read_inner(directory, None, work, &mut visitor, &|| Ok(()))
}

fn read_inner(
    directory: &Path,
    pair_index: Option<u64>,
    work: &WorkControl,
    visitor: &mut dyn FnMut(u64, Loaded) -> Result<(), Error>,
    before_final: &dyn Fn() -> Result<(), Error>,
) -> Result<u64, Error> {
    work.check()?;
    let mut guard = Guard::default();
    guard.open(directory, true, 0)?;
    let manifest: Manifest = serde_json::from_slice(
        &guard
            .read(&directory.join("manifest.json"), MAX_TERMINAL, true, work)?
            .0,
    )?;
    if !matches!(manifest.schema_version, 1 | 2)
        || manifest.kind != "laser_candidate_capture_artifacts"
    {
        return Err("unsupported completed laser capture".into());
    }
    let plan = Plan::open(
        &directory.join("geometry.json"),
        &directory.join("stream-request.json"),
        work,
    )?;
    if plan.request.schema_version != manifest.schema_version {
        return Err("capture manifest/request versions disagree".into());
    }
    let limits = log::limits(&plan.request)?;
    let manifest_schema = manifest.schema_version;
    let (report, segments) = verify_artifacts(directory, manifest, &plan, &mut guard, work)?;
    let summary = &report.summary;
    if report.schema_version != manifest_schema
        || report.kind != "laser_candidate_stream_diagnostics"
        || report.raw_replay_integrity != "PASS"
        || !matches!(
            (
                report.reported_source_restoration.as_deref(),
                manifest_schema
            ),
            (Some("PASS"), _) | (None, 1)
        )
        || report.reported_stream_settings_restoration != "PASS"
        || [
            report.geometry_binding_verified,
            report.units_verified,
            report.projector_line_identity_verified,
            report.synchronization_verified,
            report.physical_accuracy_qualified,
            report.geometry_exported,
            summary.physical_accuracy_qualified,
            summary.synchronization_verified,
        ]
        .into_iter()
        .any(|v| v)
        || summary.failed_processing_pairs > summary.processed_pairs
    {
        return Err("incomplete or falsely qualified capture report".into());
    }
    let pipeline = crate::laser_stream::Pipeline::new(
        &plan.geometry,
        &plan.request.processing,
        plan.request.fps,
    )?;
    let profiles = pipeline.profiles();
    let mut counts = [None; 2];
    let mut drops = [0; 2];
    for s in &summary.sources {
        if s.camera_index > 1
            || counts[s.camera_index].is_some()
            || !s.opened
            || !s.cleanup_confirmed
            || !s.journal_finalized
            || s.outcome.is_err()
            || s.interface != profiles[s.camera_index].interface
            || s.journaled_frames > plan.request.maximum_frames_per_camera
            || s.queued_frames.checked_add(s.processing_queue_drops) != Some(s.journaled_frames)
        {
            return Err("invalid capture source counters or finalization".into());
        }
        counts[s.camera_index] = Some(s.journaled_frames);
        drops[s.camera_index] = s.processing_queue_drops;
    }
    let counts = [
        counts[0].ok_or("missing first source")?,
        counts[1].ok_or("missing second source")?,
    ];
    if summary.processed_pairs == 0
        || summary.processed_pairs > counts[0].min(counts[1])
        || pair_index.is_some_and(|index| index >= summary.processed_pairs)
    {
        return Err("saved pair index is outside the completed capture".into());
    }
    let mut selected = None;
    let journal = pair_journal::visit(directory, Mode::Complete, work, |i, bytes, _| {
        if Some(i) == pair_index {
            selected =
                Some(serde_json::from_slice::<record::Record>(bytes).map_err(|e| e.to_string())?);
        }
        Ok(())
    })?;
    if !journal.journal_complete
        || journal.pairs != summary.processed_pairs
        || journal.bytes > plan.request.maximum_pair_log_bytes
        || journal.segments != if limits.is_some() { segments } else { 1 }
    {
        return Err("capture report/journal counters disagree".into());
    }
    match (&report.pair_journal, limits) {
        (Some(j), Some(_))
            if j.format == "segmented_ndjson_v1"
                && j.index == "pairs.index.ndjson"
                && (j.segments, j.pairs, j.bytes)
                    == (journal.segments, journal.pairs, journal.bytes) => {}
        (None, None) => {}
        _ => return Err("capture journal summary disagrees".into()),
    }
    let mut sources = [
        Playback::open(&directory.join("first.raptor"), work)?,
        Playback::open(&directory.join("second.raptor"), work)?,
    ];
    let mut reader = PairReader {
        plan: &plan,
        report: &report,
        sources: &mut sources,
        counts,
        drops,
        pair_count: journal.pairs,
        profiles,
    };
    let mut failed_pairs = 0;
    let mut deliver = |index, record: record::Record| -> Result<(), Error> {
        failed_pairs += u64::from(record.result.is_err());
        let loaded = reader.load(&record, index, work)?;
        visitor(index, loaded)?;
        work.check()?;
        Ok(())
    };
    if let Some(index) = pair_index {
        deliver(index, selected.ok_or("selected pair was not found")?)?;
    } else {
        // Reuse the complete streaming reader; no whole-journal Value/record array
        // or per-pair raw-file re-indexing. Its final checks remain mandatory too.
        let replay = pair_journal::visit(directory, Mode::Complete, work, |index, bytes, _| {
            let record =
                serde_json::from_slice::<record::Record>(bytes).map_err(|e| e.to_string())?;
            deliver(index, record).map_err(|e| e.to_string())
        })?;
        if (replay.pairs, replay.bytes, replay.segments)
            != (journal.pairs, journal.bytes, journal.segments)
        {
            return Err("capture journal changed during pair traversal".into());
        }
    }
    if pair_index.is_none() && failed_pairs != summary.failed_processing_pairs {
        return Err("capture failed-processing count disagrees with stored pairs".into());
    }
    before_final()?;
    for source in &sources {
        source.check_source()?;
    }
    plan.guard.check()?;
    guard.check()?;
    work.check()?;
    Ok(journal.pairs)
}

struct PairReader<'a> {
    plan: &'a Plan,
    report: &'a CaptureReport,
    sources: &'a mut [Playback; 2],
    counts: [u64; 2],
    drops: [u64; 2],
    pair_count: u64,
    profiles: &'a [scancraft_core::StreamProfile; 2],
}
impl PairReader<'_> {
    fn load(
        &mut self,
        selected: &record::Record,
        pair_index: u64,
        work: &WorkControl,
    ) -> Result<Loaded, Error> {
        let Self {
            plan,
            report,
            sources,
            counts,
            drops,
            pair_count,
            profiles,
        } = self;
        let display =
            selected.display(plan.interfaces(), plan.request.retain_valid_domain_segments)?;
        let mut frames = Vec::with_capacity(2);
        for (i, source) in sources.iter_mut().enumerate() {
            work.check()?;
            let reference = &selected.frames[i];
            if source.profile() != &profiles[i]
                || source.len() as u64 != counts[i]
                || reference.recording_role
                    != if i == 0 {
                        "first.raptor"
                    } else {
                        "second.raptor"
                    }
            {
                return Err("capture raw profile/count/role disagrees".into());
            }
            let frame = source.select_raw(usize::try_from(reference.frame_index)?, work)?;
            if reference.sequence != frame.frame.sequence
                || reference.host_timestamp_us != frame.frame.host_timestamp_us
                || reference.flags != frame.frame.flags
                || reference.payload_crc32 != crc32fast::hash(&frame.frame.data)
                || reference.metadata_crc32 != crc32fast::hash(&frame.frame.metadata)
                || scancraft_core::uvc_pts(&frame.frame.metadata)? != Some(selected.raw_pts)
            {
                return Err("selected capture pair raw-frame binding differs".into());
            }
            if let Some(a) = &selected.acquisition
                && (a.journaled_frames[i] <= reference.frame_index
                    || a.journaled_frames[i] > counts[i]
                    || a.processing_queue_drops[i] > drops[i])
            {
                return Err("selected capture progress counters disagree".into());
            }
            frames.push(frame);
        }
        if frames[0]
            .frame
            .host_timestamp_us
            .abs_diff(frames[1].frame.host_timestamp_us)
            > plan.request.processing.maximum_host_delta_us
        {
            return Err("selected capture pair exceeds host window".into());
        }
        let views = [
            FrameView::from_frame(&frames[0].profile, &frames[0].frame, work)?,
            FrameView::from_frame(&frames[1].profile, &frames[1].frame, work)?,
        ];
        let domain_rejections = display.domain_rejections;
        let mut loaded = Loaded {
            positions: display.positions,
            first_observations: display.first_observations,
            frames: views,
            report: json!({"queries":display.queries,"candidate_points":display.queries-display.failures-display.without_unique,
            "query_failures":display.failures,"queries_without_unique_match":display.without_unique,
            "stored_pair_error":display.error,"pair_index":pair_index,"pair_count":pair_count,
            "source_frame_indices":selected.frames.each_ref().map(|r|r.frame_index),
            "artifact_integrity_verified":true,"retained_frame_bindings_verified":true,
            "bundled_recording_integrity_verified":true,"historical_original_recordings_verified":false,
            "numerical_recomputation_verified":false,"geometry_binding_verified":false,
            "source_restoration_reported":report.reported_source_restoration.as_deref() == Some("PASS"),"physical_accuracy_qualified":false,"units_verified":false}),
        };
        if let Some(summary) = domain_rejections {
            loaded.report["domain_rejections"] = serde_json::to_value(summary)?;
            loaded.report["uniqueness_scope"] = json!("retained_valid_domain_segments");
        }
        Ok(loaded)
    }
}
fn verify_artifacts(
    directory: &Path,
    manifest: Manifest,
    plan: &Plan,
    guard: &mut Guard,
    work: &WorkControl,
) -> Result<(CaptureReport, usize), Error> {
    let limits = log::limits(&plan.request)?;
    let mut roles = HashSet::new();
    let mut report = None;
    let mut segments = 0;
    for a in manifest.artifacts.0 {
        work.check()?;
        let (maximum, retain, journal) = match a.role.as_str() {
            "geometry.json" | "stream-request.json" => (MAX_INPUT, false, false),
            "report.json" => (MAX_TERMINAL, true, false),
            "first.raptor" | "second.raptor" => (
                plan.request.maximum_recording_bytes_per_camera,
                false,
                false,
            ),
            "pairs.ndjson" if limits.is_none() => {
                (plan.request.maximum_pair_log_bytes, false, true)
            }
            "pairs.index.ndjson" if limits.is_some() => (MAX_TERMINAL, true, false),
            name if limits.is_some() && segment_number(name).is_some() => {
                segments += 1;
                (limits.unwrap().segment_bytes, false, true)
            }
            _ => return Err("unexpected capture artifact role/path".into()),
        };
        if a.bytes > maximum || !roles.insert(a.role.clone()) {
            return Err("invalid or duplicate capture artifact".into());
        }
        let path = directory.join(&a.role);
        if journal {
            // Hold identity across the existing streaming journal verifier; it
            // performs the record/order/whole-file CRC checks, without retaining history.
            if guard.open(&path, false, maximum)?.metadata()?.len() != a.bytes {
                return Err("capture journal length differs".into());
            }
        } else {
            let (bytes, length, crc) = guard.read(&path, maximum, retain, work)?;
            if (length, crc) != (a.bytes, a.crc32) {
                return Err(format!("capture artifact length/CRC mismatch: {}", a.role).into());
            }
            if a.role == "report.json" {
                report = Some(serde_json::from_slice::<CaptureReport>(&bytes)?);
            }
            if a.role == "pairs.index.ndjson" {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Header {
                    kind: String,
                    schema_version: u32,
                    limits: Limits,
                }
                let h: Header = serde_json::from_slice(
                    bytes
                        .split(|&b| b == b'\n')
                        .next()
                        .ok_or("missing journal header")?,
                )?;
                if h.kind != "header" || h.schema_version != 1 || Some(h.limits) != limits {
                    return Err("capture request/journal limits disagree".into());
                }
            }
        }
    }
    for required in [
        "geometry.json",
        "stream-request.json",
        "report.json",
        "first.raptor",
        "second.raptor",
    ] {
        if !roles.contains(required) {
            return Err("capture is missing a required artifact".into());
        }
    }
    if !roles.contains(if limits.is_some() {
        "pairs.index.ndjson"
    } else {
        "pairs.ndjson"
    }) {
        return Err("capture is missing its required journal".into());
    }
    let mut actual = HashSet::new();
    for entry in fs::read_dir(directory)? {
        work.check()?;
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF8 capture artifact")?;
        if actual.len() > MAX_ARTIFACTS || !actual.insert(name) {
            return Err("capture folder entry bound exceeded".into());
        }
    }
    let mut expected = roles.clone();
    expected.insert("manifest.json".into());
    if actual != expected {
        return Err("unexpected or missing capture artifacts".into());
    }
    let report = report.ok_or("missing capture report")?;
    Ok((report, segments))
}

#[cfg(test)]
mod tests;
