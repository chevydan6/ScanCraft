//! Integrity-checked frame export retains the selected raw payload and metadata.
use crate::output::{publish, staging};
use image::{ImageEncoder, codecs::png::PngEncoder};
use scancraft_core::uvc_pts;
use scancraft_recording::{Reader, Writer, selection::read_selected};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Write},
    path::Path,
};
type Error = Box<dyn std::error::Error>;
pub fn run(input: &Path, index: u64, target: &Path) -> Result<Value, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("frame output already exists".into());
    }
    let reader = Reader::new(BufReader::new(File::open(input)?))?;
    let version = reader.version;
    let selected = read_selected(reader, index)?;
    if selected.profile.fourcc != *b"GREY" {
        return Err("extract-frame currently supports packed GREY recordings only".into());
    }
    scancraft_image::GrayImage::from_frame(&selected.profile, &selected.frame)?;
    let partial = staging(target)?;
    let manifest = json!({"schema_version":1,"source_recording_version":version,"source_integrity":"all frames and footer validated",
        "source_header_crc_protected":version>=2,"selected_frame_index":index,"source_total_frames":selected.total_frames,
        "profile":selected.profile,"context":selected.context,"sequence":selected.frame.sequence,
        "host_timestamp_us":selected.frame.host_timestamp_us,"sensor_pts":uvc_pts(&selected.frame.metadata)?,
        "flags":selected.frame.flags,"payload_crc32":crc32fast::hash(&selected.frame.data),"metadata_crc32":crc32fast::hash(&selected.frame.metadata),
        "synchronization_verified":false,"physical_accuracy_qualified":false,"raw_frame":"frame.raptor","image":"image.png"});
    let result = (|| -> Result<(), Error> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(partial.join("frame.raptor"))?;
        let mut writer = Writer::with_context(
            BufWriter::new(file),
            selected.profile.clone(),
            selected.context.unwrap_or_default(),
        )?;
        writer.push(&selected.frame)?;
        let out = writer.finish()?;
        out.get_ref().sync_all()?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(partial.join("image.png"))?;
        let mut out = BufWriter::new(file);
        PngEncoder::new(&mut out).write_image(
            &selected.frame.data,
            selected.profile.width,
            selected.profile.height,
            image::ColorType::L8.into(),
        )?;
        out.flush()?;
        out.get_ref().sync_all()?;
        drop(out);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(partial.join("manifest.json"))?;
        let mut out = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut out, &manifest)?;
        out.flush()?;
        out.get_ref().sync_all()?;
        drop(out);
        File::open(&partial)?.sync_all()?;
        publish(&partial, target)?;
        Ok(())
    })();
    result.map_err(|e| {
        format!(
            "{e}; incomplete frame export retained at {}",
            partial.display()
        )
    })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "frame export published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(manifest)
}
