//! IR pair records use the existing bounded segmented journal container.
//! Complete/recovery inspection is read-only; CRC/order do not establish geometry or synchronization.
use super::{Pair, storage::MAXIMUM_FRAMES};
pub use crate::laser_stream::pair_journal::{Mode, ReadReport, RecordOrigin};
use scancraft_geometry::work::WorkControl;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct Reference {
    recording_role: String,
    frame_index: usize,
    sequence: u32,
    host_timestamp_us: u64,
    flags: u32,
    payload_crc32: u32,
    metadata_crc32: u32,
}
#[derive(Serialize)]
pub(super) struct Record<'a> {
    schema_version: u32,
    frames: [Reference; 2],
    raw_pts: u32,
    result: &'a Value,
}
pub(super) fn record<'a>(pair: &Pair, result: &'a Value) -> Record<'a> {
    Record {
        schema_version: 1,
        frames: [(&pair.left, "left.raptor"), (&pair.right, "right.raptor")].map(
            |(sample, role)| Reference {
                recording_role: role.into(),
                frame_index: sample.index,
                sequence: sample.frame.sequence,
                host_timestamp_us: sample.frame.host_timestamp_us,
                flags: sample.frame.flags,
                payload_crc32: crc32fast::hash(&sample.frame.data),
                metadata_crc32: crc32fast::hash(&sample.frame.metadata),
            },
        ),
        raw_pts: pair.pts,
        result,
    }
}
#[derive(Deserialize)]
struct Summary {
    pair_id: u64,
    left_index: usize,
    right_index: usize,
    left_host_us: u64,
    right_host_us: u64,
    raw_pts: u32,
    synchronization_verified: bool,
    physical_accuracy_qualified: bool,
    tracking: Tracking,
}
#[derive(Deserialize)]
struct Tracking {
    integrated: bool,
}
#[derive(Deserialize)]
struct CheckedRecord {
    frames: [Reference; 2],
    raw_pts: u32,
    result: Summary,
}
/// Visitors receive provisional records until final CRC and file identity checks pass.
/// Recovery never changes input bytes or promotes a failed capture to a successful project.
pub fn visit(
    root: &Path,
    mode: Mode,
    work: &WorkControl,
    mut visitor: impl FnMut(u64, &[u8], RecordOrigin) -> Result<(), String>,
) -> Result<ReadReport, String> {
    crate::laser_stream::pair_journal::visit_live(root, mode, work, |index, bytes, origin| {
        let r: CheckedRecord = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let s = r.result;
        if index >= MAXIMUM_FRAMES as u64
            || s.pair_id != index
            || r.frames.iter().any(|f| f.frame_index >= MAXIMUM_FRAMES)
            || [s.left_index, s.right_index] != r.frames.each_ref().map(|f| f.frame_index)
            || [s.left_host_us, s.right_host_us] != r.frames.each_ref().map(|f| f.host_timestamp_us)
            || s.raw_pts != r.raw_pts
            || s.synchronization_verified
            || s.physical_accuracy_qualified
        {
            return Err("invalid or qualified live pair journal linkage".into());
        }
        let _ = s.tracking.integrated;
        visitor(index, bytes, origin)
    })
}
