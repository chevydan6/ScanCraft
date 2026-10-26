//! Synthetic cylindrical wall and independent fit report; no scanner data.
use scancraft_geometry::{Cloud, Point, cylinder, work::WorkControl, write_ply};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: cylinder_fixture NEW_PLY")?;
    let positions: Vec<_> = [-6., 0., 6.]
        .into_iter()
        .flat_map(|z| {
            (0..32).map(move |i| {
                let t = i as f64 * std::f64::consts::TAU / 32.;
                [3. * t.cos(), 3. * t.sin(), z]
            })
        })
        .collect();
    let fit = cylinder::fit(&positions, 1e-6, &WorkControl::default())?;
    let cloud = Cloud {
        points: positions
            .into_iter()
            .map(|position| Point {
                normal: Some([position[0] / 3., position[1] / 3., 0.]),
                position,
                color: Some([90, 190, 230]),
            })
            .collect(),
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    write_ply(&cloud, &mut file)?;
    file.sync_all()?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"schema_version":1,"units":"millimeters","physical_accuracy_qualified":false,"fit":fit})
        )?
    );
    Ok(())
}
