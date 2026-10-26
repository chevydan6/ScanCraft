//! Synthetic 3-4-5 triangle with retained measurements for native UI inspection.
use scancraft_geometry::{
    Cloud, Editor, Point, formats::save_project, measurement_snapshot::Snapshot, work::WorkControl,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: direct_measurement_fixture NEW_PROJECT")?;
    let points = vec![[0., 0., 0.], [3., 0., 0.], [0., 4., 0.]];
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
    let work = WorkControl::default();
    editor.add_measurement(Snapshot::distance(
        "Hypotenuse".into(),
        vec![points[1], points[2]],
        &work,
    )?)?;
    editor.add_measurement(Snapshot::angle("Right angle".into(), points, 0, &work)?)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    save_project(&editor, &mut file)?;
    file.sync_all()?;
    Ok(())
}
