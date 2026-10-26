//! Descriptive full-recording fingerprints. Validation never accesses the filesystem.
use crate::project_origin::Origin;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub name: String,
    pub bytes: u64,
    pub crc32: u32,
    pub frames: u64,
    pub version: u8,
    pub selected_frame_index: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recordings {
    pub schema_version: u32,
    pub directory_display: String,
    pub files: [File; 2],
}
impl Recordings {
    pub fn validate(&self, origin: &Origin) -> Result<(), String> {
        origin.validate()?;
        if self.schema_version != 1
            || self.directory_display.is_empty()
            || self.directory_display.len() > 4096
        {
            return Err("invalid recording reference schema or directory".into());
        }
        for (file, (role, name)) in self
            .files
            .iter()
            .zip([("left", "left.raptor"), ("right", "right.raptor")])
        {
            let source = &origin.recording_sources[role];
            if file.name != name
                || file.bytes == 0
                || file.bytes > 1u64 << 40
                || file.frames == 0
                || file.frames > 1_000_000
                || ![1, 2].contains(&file.version)
                || file.selected_frame_index >= file.frames
                || source["source_total_frames"].as_u64() != Some(file.frames)
                || source["source_version"].as_u64() != Some(u64::from(file.version))
                || source["selected_frame_index"].as_u64() != Some(file.selected_frame_index)
            {
                return Err(
                    "recording reference disagrees with initial reconstruction lineage".into(),
                );
            }
        }
        Ok(())
    }
}
