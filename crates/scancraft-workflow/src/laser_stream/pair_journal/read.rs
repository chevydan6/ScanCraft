use super::*;
use crate::scan_project_load::Guard;
use scancraft_geometry::work::WorkControl;
use serde::de::IgnoredAny;
use std::{
    fs,
    io::{BufRead, BufReader, Read},
    path::Path,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Complete,
    Recovery,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordOrigin {
    Indexed,
    Unindexed,
    LegacyVerified,
    LegacyPrefix,
}
#[derive(Debug, Serialize)]
pub struct ReadReport {
    pub schema_version: u32,
    pub format: &'static str,
    /// Observed footer/CRC consistency only. A failed final fsync can leave a
    /// visible footer; this does not prove capture success or durable storage.
    pub journal_complete: bool,
    pub recovery_inspection: bool,
    pub indexed_pairs: u64,
    pub unindexed_complete_pairs: u64,
    pub pairs: u64,
    pub bytes: u64,
    pub segments: usize,
    pub trailing_incomplete_bytes: u64,
    pub physical_accuracy_qualified: bool,
    pub source_restoration_verified: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    recording_role: String,
    frame_index: u64,
    sequence: u32,
    host_timestamp_us: u64,
    flags: u32,
    payload_crc32: u32,
    metadata_crc32: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    schema_version: u32,
    frames: [Reference; 2],
    raw_pts: u32,
    result: IgnoredAny,
    #[serde(default)]
    acquisition: Option<crate::laser_stream::ProgressSnapshot>,
}
#[derive(Default)]
struct Order {
    last: Option<[(u64, u32, u64); 2]>,
}
impl Order {
    fn check(&mut self, line: &[u8]) -> io::Result<()> {
        let h: Header = serde_json::from_slice(line)?;
        if h.schema_version != 1 {
            return Err(invalid("unknown pair-record version"));
        }
        for (i, f) in h.frames.iter().enumerate() {
            if f.recording_role
                != if i == 0 {
                    "first.raptor"
                } else {
                    "second.raptor"
                }
                || f.flags & 0x40 != 0
            {
                return Err(invalid("invalid pair frame reference"));
            }
            if let Some(old) = self.last {
                let delta = f.sequence.wrapping_sub(old[i].1);
                if f.frame_index <= old[i].0
                    || delta == 0
                    || delta >= 1 << 31
                    || f.host_timestamp_us <= old[i].2
                {
                    return Err(invalid("pair frame references regress"));
                }
            }
            if let Some(a) = &h.acquisition
                && (a.journaled_frames[i] <= f.frame_index
                    || a.processing_queue_drops[i] > a.journaled_frames[i])
            {
                return Err(invalid("invalid recorded acquisition counters"));
            }
        }
        let _ = (
            h.raw_pts,
            h.result,
            h.frames
                .each_ref()
                .map(|f| (f.payload_crc32, f.metadata_crc32)),
        );
        self.last = Some(
            h.frames
                .each_ref()
                .map(|f| (f.frame_index, f.sequence, f.host_timestamp_us)),
        );
        Ok(())
    }
}
fn line(input: &mut impl BufRead, maximum: u64, bytes: &mut Vec<u8>) -> io::Result<bool> {
    bytes.clear();
    let n = input.take(maximum + 1).read_until(b'\n', bytes)?;
    if n as u64 > maximum {
        return Err(invalid("journal line exceeds its byte bound"));
    }
    Ok(n != 0)
}
fn manifest_digest(
    root: &Path,
    role: &str,
    guard: &mut Guard,
    work: &WorkControl,
) -> io::Result<Option<(u64, u32)>> {
    let path = root.join("manifest.json");
    if !path.try_exists()? {
        return Ok(None);
    }
    let (bytes, _, _) = guard
        .read(&path, INDEX_LIMIT, true, work)
        .map_err(|e| invalid(&e.to_string()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if !matches!(value["schema_version"].as_u64(), Some(1 | 2))
        || value["kind"] != "laser_candidate_capture_artifacts"
    {
        return Err(invalid("unknown laser artifact manifest"));
    }
    let artifacts = value["artifacts"]
        .as_array()
        .ok_or_else(|| invalid("missing artifact list"))?;
    if artifacts.len() > MAX_SEGMENTS + 8 {
        return Err(invalid("artifact list exceeds bound"));
    }
    let entries = artifacts
        .iter()
        .filter(|a| a["role"] == role)
        .collect::<Vec<_>>();
    if entries.len() != 1 {
        return Err(invalid("missing or duplicate journal artifact"));
    }
    let a = entries[0];
    Ok(Some((
        a["bytes"]
            .as_u64()
            .ok_or_else(|| invalid("missing artifact size"))?,
        a["crc32"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| invalid("invalid artifact CRC"))?,
    )))
}
struct Stream {
    pairs: u64,
    bytes: u64,
    hash: u32,
    tail: u64,
}
struct FilePolicy {
    maximum: u64,
    record_limit: u64,
    recover: bool,
    first: u64,
    origin: RecordOrigin,
}
fn records(
    path: &Path,
    policy: FilePolicy,
    guard: &mut Guard,
    work: &WorkControl,
    order: &mut Order,
    visitor: &mut impl FnMut(u64, &[u8], RecordOrigin) -> Result<(), String>,
) -> io::Result<Stream> {
    let FilePolicy {
        maximum,
        record_limit,
        recover,
        first,
        origin,
    } = policy;
    let file = guard
        .open(path, false, maximum)
        .map_err(|e| invalid(&e.to_string()))?;
    let mut input = BufReader::with_capacity(65536, file);
    let mut buffer = Vec::new();
    let mut hash = crc32fast::Hasher::new();
    let mut bytes = 0u64;
    let mut pairs = 0u64;
    let mut tail = 0;
    while line(&mut input, record_limit, &mut buffer)? {
        work.check().map_err(|e| invalid(&e))?;
        bytes = bytes
            .checked_add(buffer.len() as u64)
            .filter(|&n| n <= maximum)
            .ok_or_else(|| invalid("journal stream exceeds byte budget"))?;
        hash.update(&buffer);
        if !buffer.ends_with(b"\n") {
            if recover {
                tail = buffer.len() as u64;
                break;
            }
            return Err(invalid("unterminated pair record"));
        }
        order.check(&buffer)?;
        visitor(
            first
                .checked_add(pairs)
                .ok_or_else(|| invalid("pair ordinal overflow"))?,
            &buffer,
            origin,
        )
        .map_err(|e| invalid(&e))?;
        pairs += 1;
    }
    Ok(Stream {
        pairs,
        bytes,
        hash: hash.finalize(),
        tail,
    })
}
/// Stream one bounded record at a time. This verifies journal structure/CRC/order,
/// not numerical results, raw-frame linkage or scanner restoration. Recovery is
/// read-only and never turns a failed capture into a successful project.
/// Visitor output is provisional until the entire visit returns Ok; do not publish
/// derived artifacts from the callback before final CRC/identity checks succeed.
pub fn visit(
    root: &Path,
    mode: Mode,
    work: &WorkControl,
    mut visitor: impl FnMut(u64, &[u8], RecordOrigin) -> Result<(), String>,
) -> Result<ReadReport, String> {
    inspect(root, mode, work, &mut visitor).map_err(|e| e.to_string())
}
fn inspect(
    root: &Path,
    mode: Mode,
    work: &WorkControl,
    visitor: &mut impl FnMut(u64, &[u8], RecordOrigin) -> Result<(), String>,
) -> io::Result<ReadReport> {
    work.check().map_err(|e| invalid(&e))?;
    let mut guard = Guard::default();
    guard
        .open(root, true, u64::MAX)
        .map_err(|e| invalid(&e.to_string()))?;
    let mut names = Vec::new();
    for entry in fs::read_dir(root)? {
        if names.len() == MAX_SEGMENTS + 8 {
            return Err(invalid("journal folder entry limit exceeded"));
        }
        names.push(
            entry?
                .file_name()
                .into_string()
                .map_err(|_| invalid("non-UTF8 journal artifact"))?,
        );
    }
    let segmented = names.iter().any(|n| n == INDEX_NAME);
    let legacy = names.iter().any(|n| n == "pairs.ndjson");
    if segmented == legacy {
        return Err(invalid("missing or ambiguous pair journal layout"));
    }
    let mut report = ReadReport {
        schema_version: 1,
        format: if segmented {
            "segmented"
        } else {
            "legacy_single_file"
        },
        journal_complete: false,
        recovery_inspection: mode == Mode::Recovery,
        indexed_pairs: 0,
        unindexed_complete_pairs: 0,
        pairs: 0,
        bytes: 0,
        segments: 0,
        trailing_incomplete_bytes: 0,
        physical_accuracy_qualified: false,
        source_restoration_verified: false,
    };
    let mut order = Order::default();
    if legacy {
        let expected = if mode == Mode::Complete {
            manifest_digest(root, "pairs.ndjson", &mut guard, work)?
        } else {
            None
        };
        if mode == Mode::Complete && expected.is_none() {
            return Err(invalid("complete legacy journal requires artifact digest"));
        }
        let origin = if expected.is_some() {
            RecordOrigin::LegacyVerified
        } else {
            RecordOrigin::LegacyPrefix
        };
        let actual = records(
            &root.join("pairs.ndjson"),
            FilePolicy {
                maximum: 1 << 30,
                record_limit: MAX_RECORD_BYTES,
                recover: mode == Mode::Recovery,
                first: 0,
                origin,
            },
            &mut guard,
            work,
            &mut order,
            visitor,
        )?;
        if let Some(expected) = expected
            && expected != (actual.bytes, actual.hash)
        {
            return Err(invalid("legacy journal digest differs"));
        }
        report.pairs = actual.pairs;
        report.bytes = actual.bytes;
        report.segments = 1;
        report.trailing_incomplete_bytes = actual.tail;
        report.journal_complete = expected.is_some() && actual.tail == 0;
        if report.journal_complete {
            report.indexed_pairs = actual.pairs;
        } else {
            report.unindexed_complete_pairs = actual.pairs;
        }
    } else {
        let (index, _, _) = guard
            .read(&root.join(INDEX_NAME), INDEX_LIMIT, true, work)
            .map_err(|e| invalid(&e.to_string()))?;
        if mode == Mode::Complete
            && let Some(expected) = manifest_digest(root, INDEX_NAME, &mut guard, work)?
            && expected != (index.len() as u64, crc32fast::hash(&index))
        {
            return Err(invalid("journal index digest differs"));
        }
        let mut input = index.as_slice();
        let mut buffer = Vec::new();
        if !line(&mut input, 4096, &mut buffer)? || !buffer.ends_with(b"\n") {
            return Err(invalid("incomplete journal index header"));
        }
        let Line::Header {
            schema_version: 1,
            limits,
        } = serde_json::from_slice(&buffer)?
        else {
            return Err(invalid("unknown journal index header"));
        };
        limits.validate()?;
        let mut prefix = crc32fast::Hasher::new();
        prefix.update(&buffer);
        let mut prefix_bytes = buffer.len() as u64;
        let mut footer = false;
        while line(&mut input, 4096, &mut buffer)? {
            work.check().map_err(|e| invalid(&e))?;
            if !buffer.ends_with(b"\n") {
                if mode == Mode::Recovery {
                    report.trailing_incomplete_bytes += buffer.len() as u64;
                    break;
                }
                return Err(invalid("incomplete index entry"));
            }
            let item: Line = serde_json::from_slice(&buffer)?;
            match item {
                Line::Segment {
                    number,
                    first_pair,
                    pairs,
                    bytes,
                    crc32,
                } => {
                    if number != report.segments
                        || number >= MAX_SEGMENTS
                        || first_pair != report.pairs
                        || pairs == 0
                        || bytes == 0
                        || bytes > limits.segment_bytes
                    {
                        return Err(invalid("invalid segment index span"));
                    }
                    let name = role(number);
                    let actual = records(
                        &root.join(&name),
                        FilePolicy {
                            maximum: limits.segment_bytes,
                            record_limit: limits.maximum_record_bytes,
                            recover: false,
                            first: report.pairs,
                            origin: RecordOrigin::Indexed,
                        },
                        &mut guard,
                        work,
                        &mut order,
                        visitor,
                    )?;
                    if (actual.pairs, actual.bytes, actual.hash) != (pairs, bytes, crc32) {
                        return Err(invalid("segment record count/bytes/CRC differ"));
                    }
                    if mode == Mode::Complete
                        && let Some(expected) = manifest_digest(root, &name, &mut guard, work)?
                        && expected != (bytes, crc32)
                    {
                        return Err(invalid("segment artifact digest differs"));
                    }
                    report.pairs = report
                        .pairs
                        .checked_add(pairs)
                        .ok_or_else(|| invalid("pair count overflow"))?;
                    report.bytes = report
                        .bytes
                        .checked_add(bytes)
                        .filter(|&n| n <= limits.maximum_total_bytes)
                        .ok_or_else(|| invalid("pair journal total quota exceeded"))?;
                    report.segments += 1;
                    report.indexed_pairs = report.pairs;
                }
                Line::Complete {
                    segments,
                    pairs,
                    bytes,
                    prefix_bytes: count,
                    prefix_crc32,
                } => {
                    if (segments, pairs, bytes, count, prefix_crc32)
                        != (
                            report.segments,
                            report.pairs,
                            report.bytes,
                            prefix_bytes,
                            prefix.clone().finalize(),
                        )
                        || !input.is_empty()
                    {
                        return Err(invalid(
                            "journal completion mismatch or trailing index data",
                        ));
                    }
                    footer = true;
                    break;
                }
                _ => return Err(invalid("unexpected journal index record")),
            }
            prefix.update(&buffer);
            prefix_bytes += buffer.len() as u64;
        }
        let segment_names = names
            .iter()
            .filter(|n| n.starts_with("pairs-"))
            .collect::<Vec<_>>();
        if segment_names.len() != report.segments {
            let next = role(report.segments);
            if mode != Mode::Recovery
                || footer
                || report.segments >= MAX_SEGMENTS
                || segment_names.len() != report.segments + 1
                || !segment_names.iter().any(|n| n.as_str() == next)
            {
                return Err(invalid("unindexed, missing or invalid journal segments"));
            }
            let actual = records(
                &root.join(next),
                FilePolicy {
                    maximum: limits.segment_bytes,
                    record_limit: limits.maximum_record_bytes,
                    recover: true,
                    first: report.pairs,
                    origin: RecordOrigin::Unindexed,
                },
                &mut guard,
                work,
                &mut order,
                visitor,
            )?;
            report.unindexed_complete_pairs = actual.pairs;
            report.pairs += actual.pairs;
            report.bytes = report
                .bytes
                .checked_add(actual.bytes)
                .filter(|&n| n <= limits.maximum_total_bytes)
                .ok_or_else(|| invalid("recovery prefix exceeds quota"))?;
            report.trailing_incomplete_bytes += actual.tail;
            report.segments += 1;
        }
        if !footer && mode == Mode::Complete {
            return Err(invalid("pair journal lacks completion footer"));
        }
        report.journal_complete = footer;
    }
    guard.check().map_err(|e| invalid(&e.to_string()))?;
    Ok(report)
}
