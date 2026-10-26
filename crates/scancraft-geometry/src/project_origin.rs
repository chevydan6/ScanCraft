//! Initial reconstruction lineage, retained independently of reversible geometry edits.
//! References are descriptive only; this module never opens external files.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Write};
pub const MAX_ORIGIN_BYTES: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub name: String,
    pub bytes: u64,
    pub crc32: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub schema_version: u32,
    pub kind: String,
    pub bundle_directory_display: String,
    pub calibration: Value,
    pub request: Value,
    pub recording_sources: Value,
    pub artifacts: Vec<Artifact>,
}
impl Origin {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.kind != "initial_recorded_stereo_reconstruction"
            || self.bundle_directory_display.is_empty()
            || self.bundle_directory_display.len() > 4096
            || !self.calibration.is_object()
            || !self.request.is_object()
            || !self.recording_sources.is_object()
        {
            return Err("invalid initial reconstruction provenance".into());
        }
        for flag in [
            "source_device_binding_verified",
            "synchronization_verified",
            "physical_accuracy_qualified",
        ] {
            if self.recording_sources.get(flag).and_then(Value::as_bool) != Some(false) {
                return Err("project origin cannot assert qualified device binding, synchronization or accuracy".into());
            }
        }
        let mut names = std::collections::HashSet::new();
        for artifact in &self.artifacts {
            if ![
                "left-frame.raptor",
                "right-frame.raptor",
                "calibration.rapcal",
                "request.json",
                "recording-sources.json",
            ]
            .contains(&artifact.name.as_str())
                || artifact.bytes == 0
                || artifact.bytes > 128 * 1024 * 1024
                || !names.insert(artifact.name.as_str())
            {
                return Err("invalid or duplicate source artifact reference".into());
            }
        }
        if names.len() != 5 {
            return Err(
                "project origin requires all five reconstruction source artifact references".into(),
            );
        }
        let mut limit = Limit(0);
        serde_json::to_writer(&mut limit, self).map_err(|e| e.to_string())?;
        Ok(())
    }
}
struct Limit(usize);
impl Write for Limit {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_ORIGIN_BYTES - self.0 {
            return Err(io::Error::other("project origin exceeds 8 MiB"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
