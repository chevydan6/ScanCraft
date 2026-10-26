//! Reproducible edited synthetic project with retained reconstruction sources.
use std::{fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: project_bundle_fixture NEW_DIRECTORY")?,
    );
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evidence/project-origin-gui-render/result");
    let mut editor =
        scancraft_geometry::formats::load_project(fs::File::open(source.join("project.orproj"))?)?;
    let origin = editor.origin().ok_or("fixture requires provenance")?;
    for side in ["left", "right"] {
        if origin.recording_sources[side]["context"]["controls"]["synthetic"] != true {
            return Err("fixture source must be synthetic".into());
        }
    }
    fs::create_dir(&target)?;
    for artifact in &origin.artifacts {
        fs::copy(source.join(&artifact.name), target.join(&artifact.name))?;
    }
    let points = editor
        .cloud
        .points
        .iter()
        .take(2)
        .map(|p| p.position)
        .collect();
    editor.add_measurement(
        scancraft_geometry::measurement_snapshot::Snapshot::distance(
            "Synthetic source distance".into(),
            points,
            &scancraft_geometry::work::WorkControl::default(),
        )?,
    )?;
    editor.trim(&[0], true)?;
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target.join("project.orproj"))?;
    scancraft_geometry::formats::save_project(&editor, &file)?;
    file.sync_all()?;
    Ok(())
}
