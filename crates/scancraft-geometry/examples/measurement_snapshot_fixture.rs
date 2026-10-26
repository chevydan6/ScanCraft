//! Redistributable analytical fixture for project/UI regression checks.
use scancraft_geometry::{
    Cloud, Editor, Point, formats::save_project, measurement_snapshot::Snapshot, work::WorkControl,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: measurement_snapshot_fixture NEW_PROJECT")?;
    let points = vec![
        [5., 0., 0.],
        [-5., 0., 0.],
        [0., 5., 0.],
        [0., -5., 0.],
        [0., 0., 5.],
        [0., 0., -5.],
    ];
    let cloud = Cloud {
        points: points
            .iter()
            .map(|p| Point {
                position: *p,
                color: Some([90, 190, 230]),
                normal: Some(p.map(|x| x / 5.)),
            })
            .collect(),
    };
    let mut editor = Editor::new(cloud, 8)?;
    editor.add_measurement(Snapshot::sphere(
        "Analytical sphere".into(),
        points,
        &WorkControl::default(),
    )?)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    save_project(&editor, &mut file)?;
    file.sync_all()?;
    Ok(())
}
