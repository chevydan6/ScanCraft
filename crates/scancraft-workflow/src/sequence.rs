//! Offline PLY-sequence tracking and fusion through the shared session controller.
use super::{
    Error, artifact, bounded, digest,
    output::{publish, staging},
};
use scancraft_geometry::{
    Editor,
    formats::{read_ply, save_project},
    session::ScanSession,
    tracking::{RegistrationMethod, TrackingConfig},
    work::WorkControl,
    write_ply,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::Cursor,
    path::{Path, PathBuf},
};
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Source {
    SyntheticFixture,
    UnverifiedImports,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    id: u64,
    path: PathBuf,
    tracking_indices: Vec<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    session_id: String,
    source: Source,
    tracking: TrackingConfig,
    #[serde(default)]
    registration_method: RegistrationMethod,
    voxel_mm: f64,
    maximum_voxels: usize,
    frames: Vec<Frame>,
}
pub struct PreparedSequence {
    cloud: scancraft_geometry::Cloud,
    report: Value,
}
impl PreparedSequence {
    pub fn cloud(&self) -> &scancraft_geometry::Cloud {
        &self.cloud
    }
}
pub fn run(settings: &Path, target: &Path, work: &WorkControl) -> Result<Value, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("sequence output already exists".into());
    }
    let prepared = prepare(settings, work)?;
    publish_prepared(&prepared, target, work)
}
pub fn prepare(settings: &Path, work: &WorkControl) -> Result<PreparedSequence, Error> {
    let bytes = bounded(settings, 4 * 1024 * 1024, work)?;
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.schema_version != 1 || !(1..=1000).contains(&request.frames.len()) {
        return Err("unsupported sequence schema or frame count (1..1000)".into());
    }
    let source = match request.source {
        Source::SyntheticFixture => "synthetic_fixture",
        Source::UnverifiedImports => "unverified_imports",
    };
    let mut session = ScanSession::new_with_method(
        request.session_id,
        request.tracking,
        request.voxel_mm,
        request.maximum_voxels,
        request.frames.len(),
        request.registration_method,
    )?;
    session.start()?;
    let mut inputs = vec![digest("settings", &bytes)];
    let base = settings.parent().unwrap_or(Path::new("."));
    for frame in &request.frames {
        let path = base.join(&frame.path);
        let bytes = bounded(&path, 64 * 1024 * 1024, work)?;
        let cloud = read_ply(Cursor::new(&bytes), None)?;
        session.process(frame.id, &cloud, &frame.tracking_indices, work)?;
        inputs.push(digest(&format!("frame_{}", frame.id), &bytes));
    }
    session.stop()?;
    let cloud = session.cloud(work)?;
    let frames:Vec<_>=session.history().iter().enumerate().map(|(index,frame)| {
        let tracking=&frame.tracking;
        let pose=tracking.world_from_frame.as_ref().map(|p|json!({
            "rotation":(0..3).map(|i|(0..3).map(|j|p.rotation[(i,j)]).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "translation_mm":[p.translation[0],p.translation[1],p.translation[2]]
        }));
        json!({"frame_id":tracking.frame_id,"tracking_indices":request.frames[index].tracking_indices,"accepted":tracking.accepted,
            "tracking_state":format!("{:?}",tracking.state),"reason":tracking.reason,
            "world_from_frame":pose,"rms_mm":tracking.rms_mm,
            "source_overlap":tracking.source_overlap,"target_overlap":tracking.target_overlap,
            "integrated":frame.fusion.is_some(),"total_voxels":frame.fusion.as_ref().map(|f|f.total_voxels)})
    }).collect();
    let accepted = frames.iter().filter(|f| f["integrated"] == true).count();
    let report = json!({"schema_version":1,"session_id":session.id(),"source":source,
        "session_state":"Stopped","units":"millimeters","world_frame":"first accepted input cloud",
        "tracking":request.tracking,"registration_method":request.registration_method,"voxel_mm":request.voxel_mm,"maximum_voxels":request.maximum_voxels,
        "frames":frames,"points":cloud.points.len(),"integrated_frames":accepted,
        "rejected_frames":session.history().len()-accepted,"physical_accuracy_qualified":false,
        "synchronization_verified":false,"inputs":inputs});
    work.check()?;
    Ok(PreparedSequence { cloud, report })
}
pub fn publish_prepared(
    prepared: &PreparedSequence,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("sequence output already exists".into());
    }
    work.check()?;
    let cloud = &prepared.cloud;
    let report = &prepared.report;
    let partial = staging(target)?;
    let result = (|| -> Result<Value, Error> {
        let mut artifacts = vec![artifact(&partial, "report.json", work, |out| {
            serde_json::to_writer_pretty(out, &report)?;
            Ok(())
        })?];
        artifacts.push(artifact(&partial, "points.ply", work, |out| {
            write_ply(cloud, out)?;
            Ok(())
        })?);
        let editor = Editor::new(cloud.clone(), 32)?;
        artifacts.push(artifact(&partial, "project.orproj", work, |out| {
            save_project(&editor, out)?;
            Ok(())
        })?);
        let summary = json!({"schema_version":1,"session_id":report["session_id"],"points":editor.cloud.points.len(),
            "integrated_frames":report["integrated_frames"],"rejected_frames":report["rejected_frames"],
            "physical_accuracy_qualified":false,"artifacts":artifacts});
        artifact(&partial, "manifest.json", work, |out| {
            serde_json::to_writer_pretty(out, &summary)?;
            Ok(())
        })?;
        File::open(&partial)?.sync_all()?;
        work.check()?;
        publish(&partial, target)?;
        Ok(summary)
    })();
    let summary = result.map_err(|e| {
        format!(
            "{e}; incomplete sequence bundle retained at {}",
            partial.display()
        )
    })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "sequence bundle published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(summary)
}
