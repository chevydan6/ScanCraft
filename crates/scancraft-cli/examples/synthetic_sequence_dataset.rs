//! Reproducible synthetic tracking loss/recovery input; no scanner data.
use scancraft_geometry::{Cloud, Point, write_ply};
use serde_json::json;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("usage: synthetic_sequence_dataset NEW_DIRECTORY".into());
    }
    let dir = PathBuf::from(&args[1]);
    fs::create_dir(&dir)?;
    let world = [
        [0.25, 0.25, 0.25],
        [10.25, 0.25, 0.25],
        [0.25, 20.25, 0.25],
        [0.25, 0.25, 30.25],
        [4.25, 5.25, 8.25],
        [-6.75, 2.25, 4.25],
    ];
    let mut frames = Vec::new();
    for (id, shift) in [0., -0.2, 1000., -0.4].into_iter().enumerate() {
        let cloud = Cloud {
            points: world
                .iter()
                .map(|p| Point {
                    position: [p[0] + shift, p[1], p[2]],
                    color: Some([80, 200, 240]),
                    normal: None,
                })
                .collect(),
        };
        let path = format!("frame-{id}.ply");
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(dir.join(&path))?;
        write_ply(&cloud, &mut file)?;
        file.sync_all()?;
        frames.push(json!({"id":id,"path":path,"tracking_indices":[0,1,2,3,4,5]}));
    }
    let request = json!({"schema_version":1,"session_id":"synthetic-sequence","source":"synthetic_fixture",
        "voxel_mm":1.,"maximum_voxels":100,
        "tracking":{"max_correspondence_mm":2.,"max_rms_mm":0.01,"minimum_overlap":0.8,
        "max_step_translation_mm":1.,"max_step_rotation_rad":0.1,"iterations":20,"tolerance_mm":1e-8},"frames":frames});
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(dir.join("sequence.json"))?;
    serde_json::to_writer_pretty(&mut file, &request)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!(
        "Synthetic dataset written to {}; expected 3 integrated frames, 1 tracking loss, 6 fused points",
        dir.display()
    );
    Ok(())
}
