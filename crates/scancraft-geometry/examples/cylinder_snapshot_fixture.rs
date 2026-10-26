//! Redistributable tilted cylinder with historical fitted evidence.
use scancraft_geometry::{
    Cloud, Editor, Point, formats::save_project, measurement_snapshot::Snapshot, work::WorkControl,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: cylinder_snapshot_fixture NEW_PROJECT")?;
    let rotation = nalgebra::Rotation3::from_axis_angle(&nalgebra::Vector3::x_axis(), 0.4);
    let axis = rotation * nalgebra::Vector3::y();
    let positions: Vec<[f64; 3]> = [-6., 0., 6.]
        .into_iter()
        .flat_map(|height| {
            (0..32).map(move |i| {
                let t = i as f64 * std::f64::consts::TAU / 32.;
                (rotation * nalgebra::Vector3::new(3. * t.cos(), height, 3. * t.sin())).into()
            })
        })
        .collect();
    let cloud = Cloud {
        points: positions
            .iter()
            .map(|&position| {
                let p = nalgebra::Vector3::from(position);
                Point {
                    position,
                    color: Some([90, 190, 230]),
                    normal: Some(((p - axis * axis.dot(&p)) / 3.).into()),
                }
            })
            .collect(),
    };
    let mut editor = Editor::new(cloud, 8)?;
    editor.add_measurement(Snapshot::cylinder(
        "Analytical cylinder".into(),
        positions,
        1e-6,
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
