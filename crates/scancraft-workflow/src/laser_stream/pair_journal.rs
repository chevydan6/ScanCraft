//! Versioned bounded pair segments. CRC is integrity, not authenticity or geometry proof.
mod read;
mod write;
pub(crate) use read::visit_live;
pub use read::{Mode, ReadReport, RecordOrigin, visit};
use serde::{Deserialize, Serialize};
use std::io;
pub(crate) use write::Journal;
pub const MAX_SEGMENTS: usize = 256;
pub const MAX_RECORD_BYTES: u64 = 8 * 1024 * 1024;
pub(crate) const INDEX_LIMIT: u64 = 1_048_576;
pub(crate) const INDEX_NAME: &str = "pairs.index.ndjson";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub segment_bytes: u64,
    pub maximum_record_bytes: u64,
    pub maximum_total_bytes: u64,
}
impl Limits {
    pub fn validate(self) -> io::Result<()> {
        if !(1024..=MAX_RECORD_BYTES).contains(&self.maximum_record_bytes)
            || self.segment_bytes < self.maximum_record_bytes
            || self.segment_bytes > 1 << 30
            || self.maximum_total_bytes < self.maximum_record_bytes
            || !(4096..=256 * (1u64 << 30)).contains(&self.maximum_total_bytes)
            || self.maximum_total_bytes
                > (self.segment_bytes - self.maximum_record_bytes + 1) * MAX_SEGMENTS as u64
        {
            return Err(invalid(
                "invalid segmented pair-log limits or segment-slot admission",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Line {
    Header {
        schema_version: u32,
        limits: Limits,
    },
    Segment {
        number: usize,
        first_pair: u64,
        pairs: u64,
        bytes: u64,
        crc32: u32,
    },
    Complete {
        segments: usize,
        pairs: u64,
        bytes: u64,
        prefix_bytes: u64,
        prefix_crc32: u32,
    },
}
fn role(number: usize) -> String {
    format!("pairs-{number:06}.ndjson")
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
#[cfg(test)]
mod encoding_tests;
#[cfg(test)]
mod tests;
