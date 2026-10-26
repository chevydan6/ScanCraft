//! Reopen exact mesh histories and publish original-preserving undo/redo steps.
use super::{
    Error, artifact, bounded, digest, mesh_document,
    output::{publish, staging},
};
use scancraft_geometry::{mesh_project, work::WorkControl};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};
pub fn step(input: &Path, undo: bool, target: &Path, work: &WorkControl) -> Result<Value, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("mesh history output already exists".into());
    }
    let bytes = bounded(input, mesh_project::MAX_PAYLOAD + 16, work)?;
    let mut editor = mesh_project::load(bytes.as_slice(), work)?;
    let before = editor.history_depth();
    let changed = if undo {
        editor.undo(work)?
    } else {
        editor.redo(work)?
    };
    if !changed {
        return Err("no mesh history step available".into());
    }
    let after = editor.history_depth();
    let topology = editor.mesh().topology(work)?;
    let report = json!({"schema_version":1,"input":digest("previous.meshproj",&bytes),"operation":if undo{"undo"}else{"redo"},"units":"millimeters",
        "history_before":{"undo":before.0,"redo":before.1},"history_after":{"undo":after.0,"redo":after.1},
        "vertices":editor.mesh().vertices.len(),"triangles":editor.mesh().triangles.len(),"topology":topology,
        "physical_accuracy_qualified":false,"watertightness_qualified":false});
    publish_editor(&editor, Some(&bytes), report, target, work)
}
pub fn load_input(
    input: &Path,
    work: &WorkControl,
) -> Result<(scancraft_geometry::mesh_edit::MeshEditor, bool), Error> {
    let bytes = bounded(input, mesh_project::MAX_PAYLOAD + 16, work)?;
    if bytes.starts_with(mesh_project::MAGIC) {
        Ok((mesh_project::load(bytes.as_slice(), work)?, true))
    } else {
        Ok((
            scancraft_geometry::mesh_edit::MeshEditor::new(
                mesh_document::decode(&bytes, work)?.mesh,
                32,
                work,
            )?,
            false,
        ))
    }
}
pub fn save_snapshot(
    editor: &scancraft_geometry::mesh_edit::MeshEditor,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("mesh history output already exists".into());
    }
    editor.validate_history(work)?;
    let depth = editor.history_depth();
    let report = json!({"schema_version":1,"operation":"save_history_snapshot","units":"millimeters","history_after":{"undo":depth.0,"redo":depth.1},"physical_accuracy_qualified":false});
    publish_editor(editor, None, report, target, work)
}
fn publish_editor(
    editor: &scancraft_geometry::mesh_edit::MeshEditor,
    previous: Option<&[u8]>,
    report: Value,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("mesh history output already exists".into());
    }
    let after = editor.history_depth();
    let partial = staging(target)?;
    let outcome = (|| -> Result<Value, Error> {
        let mut artifacts = vec![artifact(&partial, "report.json", work, |out| {
            serde_json::to_writer_pretty(out, &report)?;
            Ok(())
        })?];
        if let Some(bytes) = previous {
            artifacts.push(artifact(&partial, "previous.meshproj", work, |out| {
                out.write_all(bytes)?;
                Ok(())
            })?);
        }
        artifacts.push(artifact(&partial, "project.meshproj", work, |out| {
            mesh_project::save(editor, out, work)?;
            Ok(())
        })?);
        artifacts.push(artifact(&partial, "mesh.json", work, |out| {
            serde_json::to_writer_pretty(
                out,
                &json!({"schema_version":1,"units":"millimeters","mesh":editor.mesh()}),
            )?;
            Ok(())
        })?);
        if !editor.mesh().triangles.is_empty() {
            artifacts.push(artifact(&partial, "mesh.stl", work, |out| {
                editor.mesh().write_stl(out, work)?;
                Ok(())
            })?);
            artifacts.push(artifact(&partial, "mesh.obj", work, |out| {
                editor.mesh().write_obj(out, work)?;
                Ok(())
            })?);
        }
        let summary = json!({"schema_version":1,"vertices":editor.mesh().vertices.len(),"triangles":editor.mesh().triangles.len(),"undo_steps":after.0,"redo_steps":after.1,"physical_accuracy_qualified":false,"artifacts":artifacts});
        artifact(&partial, "manifest.json", work, |out| {
            serde_json::to_writer_pretty(out, &summary)?;
            Ok(())
        })?;
        File::open(&partial)?.sync_all()?;
        work.check()?;
        publish(&partial, target)?;
        Ok(summary)
    })();
    let summary = outcome.map_err(|e| {
        format!(
            "{e}; incomplete mesh history bundle retained at {}",
            partial.display()
        )
    })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "mesh history bundle published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(summary)
}
