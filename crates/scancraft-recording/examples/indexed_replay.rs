//! Diagnostic for full validation followed by repeated nonsequential frame access.
//! Reports checksums/timing only; never emits image payloads or device identity.
use scancraft_recording::indexed::{IndexedRecording, MAX_INDEX_FRAMES};
use std::{fs::File, io::BufReader, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: indexed_replay RECORDING")?;
    let start = Instant::now();
    let mut recording =
        IndexedRecording::build(BufReader::new(File::open(path)?), MAX_INDEX_FRAMES, &|_| {
            Ok(())
        })?;
    let index_seconds = start.elapsed().as_secs_f64();
    let count = recording.len();
    if count == 0 {
        return Err("recording contains no frames".into());
    }
    let start = Instant::now();
    let mut frames = Vec::new();
    for i in [count - 1, 0, count / 2, count - 1] {
        let frame = recording.read_frame(i)?;
        frames.push(serde_json::json!({"index":i,"sequence":frame.sequence,"payload_bytes":frame.data.len(),"payload_crc32":crc32fast::hash(&frame.data),"metadata_crc32":crc32fast::hash(&frame.metadata)}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"frames":count,"version":recording.version(),"profile":recording.profile(),"index_seconds":index_seconds,"seek_seconds":start.elapsed().as_secs_f64(),"selected":frames,"physical_accuracy_qualified":false})
        )?
    );
    Ok(())
}
