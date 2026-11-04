//! Capped, verified, no-overwrite capture bundles. Supplied geometry is unqualified.
mod load;
pub use load::{load_pair, visit_pairs};
pub(crate) mod io;
mod log;
use self::io::{CappedFile, RawDigests, RawJournal};
use super::{Association, Camera, Pair, Pipeline, Result, StopToken};
use crate::{
    live_stereo::runner::{Source, require_new_target},
    output::{publish, staging},
    scan_project_load::Guard,
};
use scancraft_calibration::stereo::StereoCalibration;
use scancraft_core::StreamProfile;
use scancraft_depth::laser_pair::{self, Settings};
use scancraft_geometry::work::WorkControl;
use scancraft_image::GrayImage;
use scancraft_recording::Writer;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
const MAX_INPUT: u64 = 1_048_576;
const MAX_TERMINAL: u64 = 1_048_576;
pub struct PreviewInput<'a> {
    pub pair: &'a Pair,
    pub profiles: &'a [StreamProfile; 2],
    pub outcome: &'a PairOutcome,
    pub progress: super::ProgressSnapshot,
}
pub type PairOutcome = std::result::Result<laser_pair::Report, String>;
#[derive(Clone, Copy)]
pub enum PreviewStatus {
    NotRequested,
    Accepted,
    Omitted,
}
#[derive(Default, Serialize)]
struct PreviewCounts {
    not_requested: u64,
    accepted: u64,
    omitted: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    fps: u32,
    maximum_frames_per_camera: u64,
    maximum_recording_bytes_per_camera: u64,
    maximum_pair_log_bytes: u64,
    processing: Settings,
    #[serde(default)]
    retain_valid_domain_segments: bool,
    #[serde(default, deserialize_with = "optional_quota")]
    pair_log_segment_bytes: Option<u64>,
    #[serde(default, deserialize_with = "optional_quota")]
    maximum_pair_record_bytes: Option<u64>,
    #[serde(default, deserialize_with = "optional_quota")]
    maximum_bundle_bytes: Option<u64>,
    #[serde(default)]
    source_options: Value,
}
fn optional_quota<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<u64>, D::Error> {
    u64::deserialize(d).map(Some)
}
pub struct Plan {
    geometry: StereoCalibration,
    request: Request,
    bytes: [Vec<u8>; 2],
    guard: Guard,
}
fn caught<T>(call: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(call))
        .unwrap_or_else(|_| Err("laser capture coordinator/cleanup panicked".into()))
}
fn combine<T>(outcome: Result<T>, cleanup: Result<()>) -> Result<T> {
    match (outcome, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(e), Err(c)) => Err(format!("{e}; restoration: {c}")),
    }
}
#[derive(Serialize)]
struct FrameReference {
    recording_role: &'static str,
    frame_index: u64,
    sequence: u32,
    host_timestamp_us: u64,
    flags: u32,
    payload_crc32: u32,
    metadata_crc32: u32,
}
#[derive(Serialize)]
struct PairRecord<'a> {
    schema_version: u32,
    frames: [FrameReference; 2],
    raw_pts: u32,
    result: &'a PairOutcome,
    acquisition: super::ProgressSnapshot,
}
fn record<'a>(
    pair: &Pair,
    result: &'a PairOutcome,
    acquisition: super::ProgressSnapshot,
) -> PairRecord<'a> {
    PairRecord {
        schema_version: 1,
        acquisition,
        raw_pts: pair.raw_pts,
        result,
        frames: std::array::from_fn(|i| FrameReference {
            recording_role: if i == 0 {
                "first.raptor"
            } else {
                "second.raptor"
            },
            frame_index: pair.samples[i].index,
            sequence: pair.samples[i].frame.sequence,
            host_timestamp_us: pair.samples[i].frame.host_timestamp_us,
            flags: pair.samples[i].frame.flags,
            payload_crc32: crc32fast::hash(&pair.samples[i].frame.data),
            metadata_crc32: crc32fast::hash(&pair.samples[i].frame.metadata),
        }),
    }
}
impl Plan {
    /// Retain exact bounded regular-file inputs and identity guards. All numerical
    /// and detector policies are checked before callers acquire/open hardware.
    pub fn open(geometry: &Path, request: &Path, work: &WorkControl) -> Result<Self> {
        work.check()?;
        let mut guard = Guard::default();
        let (g, _, _) = guard
            .read(geometry, MAX_INPUT, true, work)
            .map_err(|e| e.to_string())?;
        let (r, _, _) = guard
            .read(request, MAX_INPUT, true, work)
            .map_err(|e| e.to_string())?;
        let geometry: StereoCalibration = serde_json::from_slice(&g).map_err(|e| e.to_string())?;
        let request: Request = serde_json::from_slice(&r).map_err(|e| e.to_string())?;
        if request.maximum_frames_per_camera == 0
            || request.maximum_recording_bytes_per_camera > 1 << 40
        {
            return Err("invalid laser capture schema/frame/byte limits".into());
        }
        let segmented = log::limits(&request).map_err(|e| e.to_string())?.is_some();
        let pipeline = Pipeline::new(&geometry, &request.processing, request.fps)?
            .with_valid_domain_segments(request.retain_valid_domain_segments);
        Association::new(
            pipeline.profiles().clone(),
            request.maximum_frames_per_camera,
            request.processing.maximum_host_delta_us,
        )?;
        for profile in pipeline.profiles() {
            let pixels = u64::from(profile.width) * u64::from(profile.height);
            if request.maximum_recording_bytes_per_camera < MAX_INPUT + pixels + 4096 + 64 {
                return Err(
                    "raw byte budget must accommodate one maximum header/frame/footer".into(),
                );
            }
            if segmented {
                let required = (pixels + 4096 + 32)
                    .checked_mul(request.maximum_frames_per_camera)
                    .and_then(|n| n.checked_add(MAX_INPUT + 32))
                    .ok_or("raw frame admission overflow")?;
                if required > request.maximum_recording_bytes_per_camera {
                    return Err(
                        "schema2 raw quota cannot admit configured frame count and metadata bound"
                            .into(),
                    );
                }
            }
            // Calls the existing validators/math; no duplicate detector rules or
            // synthetic image is represented as physical calibration evidence.
            let blank = vec![0; pixels as usize];
            laser_pair::detect(
                GrayImage::new(profile.width, profile.height, &blank)?,
                &request.processing.detection,
                &request.processing.continuity,
                work,
            )?;
        }
        guard.check().map_err(|e| e.to_string())?;
        Ok(Self {
            geometry,
            request,
            bytes: [g, r],
            guard,
        })
    }
    /// Schema2 checks an available-space snapshot before native ownership is acquired.
    /// This is not physical disk reservation; later I/O errors still fail safely.
    pub fn admit_target(&self, target: &Path, work: &WorkControl) -> Result<()> {
        work.check()?;
        require_new_target(target)?;
        let parent = target
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        log::admit(&self.request, parent).map_err(|e| e.to_string())
    }
    pub fn fps(&self) -> u32 {
        self.request.fps
    }
    /// Opaque backend declaration retained exactly in stream-request.json. The
    /// device adapter validates/interprets it before acquiring hardware ownership.
    pub fn source_options(&self) -> &Value {
        &self.request.source_options
    }
    pub fn interfaces(&self) -> [u8; 2] {
        self.request.processing.interfaces
    }
    /// Source owners close before restore is called, including failure/cancel.
    /// Optional previews never control required journaling success. Callers own
    /// session locks/settings leases; no driver calls or restoration are detached.
    pub fn capture<S: Source + 'static, O, P>(
        &self,
        target: &Path,
        work: &Arc<WorkControl>,
        stop: &Arc<AtomicBool>,
        open: O,
        restore: impl FnOnce() -> Result<()>,
        mut preview: P,
    ) -> Result<Value>
    where
        O: Fn(Camera, StreamProfile, StopToken) -> Result<S> + Send + Sync + 'static,
        P: FnMut(PreviewInput<'_>) -> PreviewStatus,
    {
        let mut partial: Option<PathBuf> = None;
        let mut terminal = None;
        let mut preview_counts = PreviewCounts::default();
        let outcome = caught(|| {
            work.check()?;
            self.admit_target(target, work)?;
            self.guard.check().map_err(|e| e.to_string())?;
            if stop.load(Ordering::Relaxed) {
                return Err("laser capture stopped before opening sources".into());
            }
            let directory = staging(target).map_err(|e| e.to_string())?;
            partial = Some(directory.clone());
            let mut artifacts = vec![
                io::bytes(&directory, "geometry.json", &self.bytes[0])
                    .map_err(|e| e.to_string())?,
                io::bytes(&directory, "stream-request.json", &self.bytes[1])
                    .map_err(|e| e.to_string())?,
            ];
            let pipeline =
                Pipeline::new(&self.geometry, &self.request.processing, self.request.fps)?
                    .with_valid_domain_segments(self.request.retain_valid_domain_segments);
            let mut pairs = log::Log::open(&directory, &self.request).map_err(|e| e.to_string())?;
            let raw_digests: RawDigests = Arc::new(Mutex::new([None, None]));
            let slots = raw_digests.clone();
            let raw_dir = directory.clone();
            let raw_limit = self.request.maximum_recording_bytes_per_camera;
            let result = pipeline.run_progress(
                self.request.maximum_frames_per_camera,
                work,
                stop,
                open,
                move |camera, profile, context| {
                    let writer = CappedFile::new(
                        &raw_dir,
                        if camera.index() == 0 {
                            "first.raptor"
                        } else {
                            "second.raptor"
                        },
                        raw_limit,
                    )
                    .map_err(|e| e.to_string())?;
                    Ok(RawJournal {
                        writer: Writer::with_context(writer, profile, context)
                            .map_err(|e| e.to_string())?,
                        digests: slots.clone(),
                        camera: camera.index(),
                    })
                },
                |pair, result, progress| {
                    pairs
                        .push(&record(pair, &result, progress), work)
                        .map_err(|e| e.to_string())?;
                    let offered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        preview(PreviewInput {
                            pair,
                            profiles: pipeline.profiles(),
                            outcome: &result,
                            progress,
                        })
                    }))
                    .unwrap_or(PreviewStatus::Omitted);
                    match offered {
                        PreviewStatus::NotRequested => preview_counts.not_requested += 1,
                        PreviewStatus::Accepted => preview_counts.accepted += 1,
                        PreviewStatus::Omitted => preview_counts.omitted += 1,
                    };
                    Ok(())
                },
            );
            terminal = Some(match &result {
                Ok(summary) => json!({"Ok":summary}),
                Err(failure) => json!({"Err":failure}),
            });
            let pair_digest = pairs.finish().map_err(|e| e.to_string());
            let summary = combine(
                result.map_err(|e| e.reason),
                pair_digest.as_ref().map(|_| ()).map_err(Clone::clone),
            )?;
            let pair_log = pair_digest?;
            artifacts.extend(pair_log.artifacts);
            let mut slots = raw_digests
                .lock()
                .map_err(|_| "raw digest state poisoned")?;
            for slot in slots.iter_mut() {
                artifacts.push(slot.take().ok_or("missing complete raw artifact")?);
            }
            Ok((
                directory,
                artifacts,
                summary,
                pipeline.profiles().clone(),
                pair_log.details,
            ))
        });
        let restoration = caught(restore);
        let restoration_value = restoration.clone();
        let captured = combine(outcome, restoration);
        let result=captured.and_then(|(directory,mut artifacts,summary,profiles,pair_log)| {
            work.check()?;let mut guard=Guard::default();
            for digest in &artifacts {
                if digest.role=="first.raptor" || digest.role=="second.raptor" {
                    let side=usize::from(digest.role=="second.raptor");
                    let source=summary.sources.iter().find(|s|s.camera_index==side).ok_or("missing raw source report")?;
                    if !source.journal_finalized || !source.cleanup_confirmed {return Err("raw source finalization/cleanup unconfirmed".into());}
                    io::verify_raw(&directory,digest,&profiles[side],source.journaled_frames,&mut guard,work)?;
                }else {io::verify(&directory,digest,&mut guard,work)?;}
            }
            if pair_log.is_some() {
                let checked=super::pair_journal::visit(&directory,super::pair_journal::Mode::Complete,work,|_,_,_|Ok(()))?;
                if checked.pairs!=summary.processed_pairs {return Err("segmented journal pair count differs from processing summary".into());}
            }
            let mut report=json!({"schema_version":self.request.schema_version,"kind":"laser_candidate_stream_diagnostics","geometry_binding_verified":false,"units_verified":false,"projector_line_identity_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,"geometry_exported":false,"reported_stream_settings_restoration":"PASS","reported_source_restoration":"PASS","raw_replay_integrity":"PASS","optional_previews":preview_counts,"summary":summary});
            if let Some(pair_log)=pair_log {report["pair_journal"]=pair_log;}
            let digest=io::json(&directory,"report.json",&report,MAX_TERMINAL).map_err(|e|e.to_string())?;io::verify(&directory,&digest,&mut guard,work)?;artifacts.push(digest);
            artifacts.sort_by(|a,b|a.role.cmp(&b.role));
            let manifest=io::json(&directory,"manifest.json",&json!({"schema_version":self.request.schema_version,"kind":"laser_candidate_capture_artifacts","artifacts":artifacts}),MAX_TERMINAL).map_err(|e|e.to_string())?;
            io::verify(&directory,&manifest,&mut guard,work)?;
            let mut expected=artifacts.iter().map(|d|d.role.clone()).collect::<Vec<_>>();expected.push("manifest.json".into());expected.sort();
            let mut actual=fs::read_dir(&directory).map_err(|e|e.to_string())?.map(|e|e.map(|v|v.file_name().to_string_lossy().into_owned()).map_err(|e|e.to_string())).collect::<Result<Vec<_>>>()?;actual.sort();if actual!=expected {return Err("unexpected laser capture artifacts".into());}
            guard.open(&directory,true,u64::MAX).map_err(|e|e.to_string())?;File::open(&directory).and_then(|f|f.sync_all()).map_err(|e|e.to_string())?;
            work.check()?;guard.check().map_err(|e|e.to_string())?;self.guard.check().map_err(|e|e.to_string())?;
            publish(&directory,target).map_err(|e|e.to_string())?;partial=None;
            let parent=target.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
            File::open(parent).and_then(|f|f.sync_all()).map_err(|e|format!("capture published at {}, parent sync failed: {e}",target.display()))?;
            Ok(report)
        });
        result.map_err(|reason| {
            if let Some(directory)=partial {
                let failure=json!({"schema_version":1,"kind":"incomplete_laser_candidate_capture","error":reason,"stream_result":terminal,"settings_restoration":restoration_value,"optional_previews":preview_counts});
                let note=match io::json(&directory,"failure.json",&failure,MAX_TERMINAL) {Ok(_)=>String::new(),Err(e)=>format!("; failure report write failed: {e}")};
                format!("{reason}{note}; incomplete capture retained at {}",directory.display())
            }else {reason}
        })
    }
}

#[cfg(test)]
mod tests;
