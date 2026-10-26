//! Redistributable analytical point-to-plane measurement fixture.
use scancraft_geometry::{
    Cloud, Editor, Point, formats::save_project, measurement_snapshot::Snapshot, work::WorkControl,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: point_plane_measurement_fixture NEW_PROJECT")?;
    let points = vec![
        [-2., 0., -2.],
        [-2., 0., 2.],
        [2., 0., -2.],
        [2., 0., 2.],
        [0., 5., 0.],
    ];
    let mut editor = Editor::new(
        Cloud {
            points: points
                .iter()
                .map(|&position| Point {
                    position,
                    color: Some([90, 190, 230]),
                    normal: None,
                })
                .collect(),
        },
        8,
    )?;
    editor.add_measurement(Snapshot::point_plane(
        "Probe height".into(),
        points,
        4,
        0.,
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
