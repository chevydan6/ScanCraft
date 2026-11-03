//! Explicit recorded stereo pairs -> reconstruction -> existing tracking/fusion.
//! Offline only. Pair selection, declared calibration and local ICP acceptance
//! never establish synchronized exposures or physically accurate scanner geometry.
use crate::{
    Error, Prepared, artifact, bounded,
    output::{publish, staging},
    recording_reconstruction::{Inputs, RecordingProcessor},
};
use scancraft_geometry::{
    Editor, formats::save_project, session::ScanSession, tracking::TrackingConfig,
    work::WorkControl, write_ply,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair {
    id: u64,
    left_index: usize,
    right_index: usize,
    /// Indices in this frame's accepted reconstruction cloud, not seed indices.
    tracking_indices: Vec<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    session_id: String,
    tracking: TrackingConfig,
    voxel_mm: f64,
    maximum_voxels: usize,
    pairs: Vec<Pair>,
}

fn write_frame(
    prepared: &Prepared,
    path: &Path,
    work: &WorkControl,
) -> Result<Vec<crate::Digest>, Error> {
    fs::create_dir(path)?;
    let mut artifacts = vec![artifact(path, "report.json", work, |out| {
        serde_json::to_writer_pretty(out, prepared.report())?;
        Ok(())
    })?];
    prepared
        .recording_sources
        .as_ref()
        .ok_or("recorded sequence is missing frame lineage")?
        .write_artifacts(path, work, &mut artifacts)?;
    let points = prepared.cloud().map_or(0, |cloud| cloud.points.len());
    if let Some(cloud) = prepared.cloud() {
        artifacts.push(artifact(path, "points.ply", work, |out| {
            write_ply(cloud, out)?;
            Ok(())
        })?);
    }
    let manifest = artifact(path, "manifest.json", work, |out| {
        serde_json::to_writer_pretty(
            out,
            &json!({"schema_version":1,"points":points,"artifacts":artifacts,
            "synchronization_verified":false,"physical_accuracy_qualified":false}),
        )?;
        Ok(())
    })?;
    File::open(path)?.sync_all()?;
    artifacts.push(manifest);
    Ok(artifacts)
}

/// Complete staged sequence whose final publication is deferred until callers
/// have prepared their view/document. Dropping it retains staging for recovery.
pub struct PreparedSequence {
    editor: Editor,
    summary: Value,
    partial: PathBuf,
    target: PathBuf,
    processor: RecordingProcessor,
}
impl PreparedSequence {
    pub fn cloud(&self) -> &scancraft_geometry::Cloud {
        &self.editor.cloud
    }
    /// Preserve the same initial sequence references in the saved and GUI document.
    pub fn create_editor(&self) -> Result<Editor, Error> {
        self.editor.validate_history()?;
        Ok(self.editor.clone())
    }
    pub fn staging_directory(&self) -> &Path {
        &self.partial
    }
}
pub fn run(
    inputs: &Inputs,
    settings: &Path,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    let prepared = prepare(inputs, settings, target, work)?;
    publish_prepared(&prepared, work)
}
/// Processes 1..1000 explicit pairs with one validated source/index/map set.
/// Retains only the current reconstruction plus bounded session/fusion state in
/// memory. Each pair's raw sources and diagnostics are persisted in staging.
/// Errors/cancellation retain incomplete staging. Success returns an unpublished
/// sequence for view preparation before explicit publish_prepared. No accepted
/// frame is silently dropped or re-anchored.
pub fn prepare(
    inputs: &Inputs,
    settings: &Path,
    target: &Path,
    work: &WorkControl,
) -> Result<PreparedSequence, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("recorded sequence output already exists".into());
    }
    let bytes = bounded(settings, 4 * 1024 * 1024, work)?;
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.schema_version != 1
        || !(1..=1000).contains(&request.pairs.len())
        || request.pairs.windows(2).any(|p| {
            p[0].id >= p[1].id
                || p[0].left_index >= p[1].left_index
                || p[0].right_index >= p[1].right_index
        })
        || request
            .pairs
            .iter()
            .any(|p| !(3..=512).contains(&p.tracking_indices.len()))
    {
        return Err("recorded sequence requires schema1, 1..1000 increasing frame IDs/left indices/right indices and 3..512 explicit tracking indices per pair".into());
    }
    let mut session = ScanSession::new(
        request.session_id.clone(),
        request.tracking,
        request.voxel_mm,
        request.maximum_voxels,
        request.pairs.len(),
    )?;
    let mut processor = RecordingProcessor::open(inputs, work)?;
    session.start()?;
    let partial = staging(target)?;
    let result = (|| -> Result<(Value, Editor), Error> {
        let request_artifact = artifact(&partial, "sequence-request.json", work, |out| {
            std::io::Write::write_all(out, &bytes)?;
            Ok(())
        })?;
        let mut frames = Vec::with_capacity(request.pairs.len());
        let mut reconstruction_context = Value::Null;
        let mut sequence_frames = Vec::with_capacity(request.pairs.len());
        let mut retained = Vec::new();
        for (index, pair) in request.pairs.iter().enumerate() {
            work.check()?;
            let prepared = processor.prepare_pair(pair.left_index, pair.right_index, work)?;
            if index == 0 {
                let report = prepared.report();
                reconstruction_context = json!({"source":report.source,
                    "calibration_record_id":report.calibration.record_id,
                    "calibration_provenance":report.calibration.provenance});
            }
            if prepared.surface.is_some() || prepared.normal_cloud.is_some() {
                return Err("recorded sequence currently requires reconstruction settings without meshing or surface normals".into());
            }
            let directory = format!("frame-{index:04}");
            let references = write_frame(&prepared, &partial.join(&directory), work)?;
            retained.extend(references.into_iter().map(|d| {
                scancraft_geometry::sequence_references::Artifact {
                    name: format!("{directory}/{}", d.role),
                    bytes: d.bytes,
                    crc32: d.crc32,
                }
            }));
            let cloud = prepared.cloud().ok_or("selected stereo pair produced no geometry; sequence stopped without inventing tracking")?;
            let update = session.process(pair.id, cloud, &pair.tracking_indices, work)?;
            sequence_frames.push(scancraft_geometry::sequence_references::Frame {
                id: pair.id,
                left_index: pair.left_index,
                right_index: pair.right_index,
                integrated: update.fusion.is_some(),
            });
            let tracking = &update.tracking;
            let pose = tracking.world_from_frame.as_ref().map(|p|json!({
                "rotation":(0..3).map(|r|(0..3).map(|c|p.rotation[(r,c)]).collect::<Vec<_>>()).collect::<Vec<_>>(),
                "translation_mm":[p.translation[0],p.translation[1],p.translation[2]]}));
            frames.push(json!({"frame_id":pair.id,"left_index":pair.left_index,"right_index":pair.right_index,
                "tracking_indices":pair.tracking_indices,"artifact_directory":directory,
                "tracking_state":format!("{:?}",tracking.state),"accepted":tracking.accepted,
                "reason":tracking.reason,"world_from_frame":pose,"rms_mm":tracking.rms_mm,
                "source_overlap":tracking.source_overlap,"target_overlap":tracking.target_overlap,
                "integrated":update.fusion.is_some()}));
        }
        session.stop()?;
        let cloud = session.cloud(work)?;
        let integrated = frames.iter().filter(|f| f["integrated"] == true).count();
        let report = json!({"schema_version":1,"kind":"offline_recorded_stereo_sequence","session_id":session.id(),
            "units":"millimeters","world_frame":"first accepted reconstruction cloud",
            "pair_selection":"explicit indices; no inferred exposure pairing","tracking":request.tracking,
            "registration_method":"point_to_point","voxel_mm":request.voxel_mm,"maximum_voxels":request.maximum_voxels,
            "reconstruction_context":reconstruction_context,"frames":frames,"integrated_frames":integrated,"rejected_frames":request.pairs.len()-integrated,
            "synchronization_verified":false,"physical_accuracy_qualified":false,"source_device_binding_verified":false,
            "project_lineage":"initial sequence references retained in project; raw sources and complete reports remain in external bundle",
            "source_integrity":"complete recordings indexed once; selected fingerprints and final filesystem guards checked"});
        let mut artifacts = vec![
            request_artifact,
            artifact(&partial, "report.json", work, |out| {
                serde_json::to_writer_pretty(out, &report)?;
                Ok(())
            })?,
        ];
        artifacts.push(artifact(&partial, "points.ply", work, |out| {
            write_ply(&cloud, out)?;
            Ok(())
        })?);
        retained.extend(artifacts.iter().map(|d| {
            scancraft_geometry::sequence_references::Artifact {
                name: d.role.clone(),
                bytes: d.bytes,
                crc32: d.crc32,
            }
        }));
        let mut editor = Editor::new(cloud, 32)?;
        editor.attach_sequence(scancraft_geometry::sequence_references::Sequence {
            schema_version: 1,
            directory_display: std::path::absolute(target)?.display().to_string(),
            session_id: request.session_id.clone(),
            source_kind: if reconstruction_context["source"] == "synthetic_fixture" {
                "synthetic_fixture"
            } else {
                "declared_observations"
            }
            .into(),
            calibration_record_id: reconstruction_context["calibration_record_id"]
                .as_str()
                .ok_or("missing sequence calibration ID")?
                .into(),
            frames: sequence_frames,
            artifacts: retained,
            source_device_binding_verified: false,
            synchronization_verified: false,
            physical_accuracy_qualified: false,
        })?;
        artifacts.push(artifact(&partial, "project.orproj", work, |out| {
            save_project(&editor, out)?;
            Ok(())
        })?);
        let summary = json!({"schema_version":1,"points":editor.cloud.points.len(),"frames":request.pairs.len(),
            "integrated_frames":integrated,"rejected_frames":request.pairs.len()-integrated,
            "physical_accuracy_qualified":false,"synchronization_verified":false,"artifacts":artifacts});
        artifact(&partial, "manifest.json", work, |out| {
            serde_json::to_writer_pretty(out, &summary)?;
            Ok(())
        })?;
        processor.check_sources(work)?;
        File::open(&partial)?.sync_all()?;
        work.check()?;
        Ok((summary, editor))
    })();
    let (summary, editor) = result.map_err(|e| {
        format!(
            "{e}; incomplete recorded sequence retained at {}",
            partial.display()
        )
    })?;
    Ok(PreparedSequence {
        editor,
        summary,
        partial,
        target: target.into(),
        processor,
    })
}
/// Rechecks sources/cancellation and atomically commits a fully prepared sequence.
/// No fallible view/document construction should follow successful publication.
pub fn publish_prepared(prepared: &PreparedSequence, work: &WorkControl) -> Result<Value, Error> {
    let commit = || -> Result<(), Error> {
        if fs::symlink_metadata(&prepared.target).is_ok() {
            return Err("recorded sequence output already exists".into());
        }
        prepared.processor.check_sources(work)?;
        work.check()?;
        publish(&prepared.partial, &prepared.target)?;
        Ok(())
    };
    commit().map_err(|e| {
        format!(
            "{e}; recorded sequence staging path: {}",
            prepared.partial.display()
        )
    })?;
    let parent = prepared
        .target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|e| {
            format!(
                "recorded sequence published at {}, parent sync failed: {e}",
                prepared.target.display()
            )
        })?;
    Ok(prepared.summary.clone())
}
