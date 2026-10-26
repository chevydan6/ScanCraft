//! Native CLI adapter for the shared reconstruction workflow.
use std::path::PathBuf;
pub fn run(paths: Vec<PathBuf>) -> Result<scancraft_workflow::Summary, scancraft_workflow::Error> {
    if paths.len() != 5 {
        return Err("five reconstruction paths required".into());
    }
    if std::fs::symlink_metadata(&paths[4]).is_ok() {
        return Err("output directory already exists; choose a new path".into());
    }
    let work = scancraft_geometry::work::WorkControl::default();
    let inputs = scancraft_workflow::FileInputs {
        calibration: paths[0].clone(),
        left: paths[1].clone(),
        right: paths[2].clone(),
        settings: paths[3].clone(),
    };
    let prepared = scancraft_workflow::prepare(&inputs, &work)?;
    scancraft_workflow::publish_prepared(&prepared, &paths[4], &work)
}
