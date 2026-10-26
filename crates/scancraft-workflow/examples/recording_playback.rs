//! Exercise the same guarded, bounded decoder used by native GUI playback.
use scancraft_geometry::work::WorkControl;
use scancraft_workflow::recording_playback::Playback;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: recording_playback RECORDING")?;
    let work = WorkControl::default();
    let mut playback = Playback::open(std::path::Path::new(&path), &work)?;
    let count = playback.len();
    let mut views = Vec::new();
    for index in [count - 1, 0, count / 2, count - 1] {
        let view = playback.read(index, &work)?;
        views.push(serde_json::json!({"index":index,"sequence":view.sequence,"sensor_pts":view.sensor_pts,"size":[view.image.width(),view.image.height()],"rgb_crc32":crc32fast::hash(view.image.as_raw()),"description":view.description}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"frames":count,"views":views,"physical_accuracy_qualified":false})
        )?
    );
    Ok(())
}
