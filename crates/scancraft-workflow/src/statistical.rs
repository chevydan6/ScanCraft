//! Explicit original-preserving statistical filtering of imported/project clouds.
use crate::{
    Error, artifact, bounded,
    output::{publish, staging},
};
use scancraft_geometry::{
    Editor,
    formats::{load_project, read_ply, save_project},
    statistical_filter,
    work::WorkControl,
    write_ply,
};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    path::Path,
};
pub fn run(
    input: &Path,
    neighbors: usize,
    sigma: f64,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    work.check()?;
    if fs::symlink_metadata(target).is_ok() {
        return Err("statistical filter output already exists".into());
    }
    let bytes = bounded(input, 64 * 1024 * 1024 + 16, work)?;
    let mut editor = if bytes.starts_with(b"ORPROJ") {
        load_project(bytes.as_slice())?
    } else {
        Editor::new(read_ply(bytes.as_slice(), None)?, 8)?
    };
    let before = editor.cloud.points.len();
    let (next, statistics) = statistical_filter::filter(&editor.cloud, neighbors, sigma, work)?;
    editor.replace(next)?;
    work.check()?;
    let partial = staging(target)?;
    let result=(||->Result<Value,Error>{
        let mut artifacts=Vec::new();artifacts.push(artifact(&partial,"points.ply",work,|out|{write_ply(&editor.cloud,out)?;Ok(())})?);
        let project=artifact(&partial,"project.orproj",work,|out|{save_project(&editor,out)?;Ok(())})?;
        let reopened=load_project(File::open(partial.join("project.orproj"))?)?;
        let mut canonical=Vec::new();save_project(&reopened,&mut canonical)?;
        if canonical.len() as u64!=project.bytes || crc32fast::hash(&canonical)!=project.crc32 {return Err("statistical project canonical replay mismatch".into());}
        let undo_available=reopened.clone().undo();
        artifacts.push(project);
        let report=json!({"schema_version":1,"scope":"explicit statistical distance filtering; not inferred physical noise labels","input_read_crc32":crc32fast::hash(&bytes),"input_points":before,"retained_points":editor.cloud.points.len(),"removed_points":statistics.outlier_indices.len(),"statistics":statistics,"self_excluded_by_index":true,"coincident_other_points_count":true,"variance_denominator":"N-1","threshold_equality_retained":true,"source_file_written":false,"project_full_state_replay":"PASS","undo_available_after_filter":undo_available,"physical_accuracy_qualified":false});
        artifacts.push(artifact(&partial,"report.json",work,|out|{serde_json::to_writer_pretty(out,&report)?;Ok(())})?);
        let summary=json!({"schema_version":1,"input_points":before,"retained_points":editor.cloud.points.len(),"removed_points":statistics.outlier_indices.len(),"artifacts":artifacts,"physical_accuracy_qualified":false});
        artifact(&partial,"manifest.json",work,|out|{serde_json::to_writer_pretty(out,&summary)?;Ok(())})?;
        File::open(&partial)?.sync_all()?;work.check()?;publish(&partial,target)?;Ok(summary)
    })().map_err(|e|format!("{e}; incomplete statistical filter retained at {}",partial.display()))?;
    File::open(
        target
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?
    .sync_all()
    .map_err(|e| {
        format!(
            "statistical result published at {}; parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(result)
}
