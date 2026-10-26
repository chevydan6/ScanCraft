//! Synthetic aligned project retaining its exact geometry undo and measurement.
use scancraft_geometry::{
    Cloud, Editor, Point, coordinate_frame::Frame, formats::save_project,
    measurement_snapshot::Snapshot, work::WorkControl,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: coordinate_frame_fixture NEW_PROJECT")?;
    let positions = [
        [10., 20., 30.],
        [10., 22., 30.],
        [9., 21., 30.],
        [12., 23., 34.],
    ];
    let work = WorkControl::default();
    let mut editor = Editor::new(
        Cloud {
            points: positions
                .iter()
                .map(|&position| Point {
                    position,
                    color: Some([90, 190, 230]),
                    normal: Some([2., 0., 0.]),
                })
                .collect(),
        },
        8,
    )?;
    editor.add_measurement(Snapshot::distance(
        "Original reference length".into(),
        positions[..2].to_vec(),
        &work,
    )?)?;
    let frame = Frame::from_points(positions[0], positions[1], positions[2])?;
    editor.replace(frame.apply(&editor.cloud, &work)?)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    save_project(&editor, &mut file)?;
    file.sync_all()?;
    Ok(())
}
