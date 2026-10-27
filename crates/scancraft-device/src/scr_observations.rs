//! Passive Raptor light-type observations from Linux UVC metadata.
//!
//! Numeric values are protocol observations, not qualified blue/NIR mode names.
//! This module neither changes device state nor determines frame acceptance.
use serde::Serialize;

/// One payload's six-byte source-clock reference and optional vendor type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScrObservation {
    /// Zero-based Linux metadata record index, including records without SCR.
    pub record_index: usize,
    pub scr: [u8; 6],
    /// Unknown tags remain `None`; no guessed mode is substituted.
    pub vendor_light_type: Option<u8>,
}

/// Decode every available SCR, preserving record order and disagreements.
///
/// Linux prefixes each UVC header with an eight-byte host timestamp and two-byte
/// SOF value. PTS, when present, precedes SCR. Malformed records are rejected even
/// after an earlier valid observation. Missing SCR is permitted. Per-record
/// results intentionally do not imply a single consistent frame mode.
///
/// The Raptor tag table was independently described through protocol research;
/// see `docs/RAPTOR_SCR_OBSERVATIONS.md` for provenance and limitations.
pub fn observe(metadata: &[u8]) -> Result<Vec<ScrObservation>, &'static str> {
    let mut remaining = metadata;
    let mut record_index = 0;
    let mut observations = Vec::new();
    while !remaining.is_empty() {
        if remaining.len() < 12 {
            return Err("truncated UVC metadata record");
        }
        let length = usize::from(remaining[10]);
        if length < 2 || length > remaining.len() - 10 {
            return Err("invalid UVC header length");
        }
        let header = &remaining[10..10 + length];
        let flags = header[1];
        if flags & 0x40 != 0 {
            return Err("device reported a UVC frame error");
        }
        let scr_offset = if flags & 4 != 0 { 6 } else { 2 };
        if header.len() < scr_offset {
            return Err("truncated UVC PTS");
        }
        if flags & 8 != 0 {
            let bytes = header
                .get(scr_offset..scr_offset + 6)
                .ok_or("truncated UVC SCR")?;
            let scr: [u8; 6] = bytes.try_into().map_err(|_| "invalid UVC SCR")?;
            let tag = if matches!(scr[3], 0xc7 | 0xc8) {
                scr[3]
            } else {
                scr[0]
            };
            let vendor_light_type = match tag {
                0xff => Some(0),
                0xa2 => Some(1),
                0xb1 => Some(2),
                0xb2 => Some(3),
                0xb4 => Some(4),
                0xc7 => Some(6),
                0xc8 => Some(7),
                _ => None,
            };
            observations.push(ScrObservation {
                record_index,
                scr,
                vendor_light_type,
            });
        }
        remaining = &remaining[10 + length..];
        record_index += 1;
    }
    Ok(observations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(pts: bool, scr: Option<[u8; 6]>) -> Vec<u8> {
        let mut bytes = vec![0; 10];
        bytes.extend([
            2 + 4 * u8::from(pts) + 6 * u8::from(scr.is_some()),
            4 * u8::from(pts) + 8 * u8::from(scr.is_some()),
        ]);
        if pts {
            bytes.extend(123u32.to_le_bytes());
        }
        if let Some(scr) = scr {
            bytes.extend(scr);
        }
        bytes
    }

    #[test]
    fn known_unknown_and_override_tags_with_optional_pts() {
        for pts in [false, true] {
            for (tag, expected) in [
                (0xff, Some(0)),
                (0xa2, Some(1)),
                (0xb1, Some(2)),
                (0xb2, Some(3)),
                (0xb4, Some(4)),
                (0xc7, Some(6)),
                (0xc8, Some(7)),
                (0, None),
            ] {
                let result = observe(&record(pts, Some([tag, 0, 0, 0, 0, 0]))).unwrap();
                assert_eq!(result[0].vendor_light_type, expected);
                for (override_tag, expected) in [(0xc7, 6), (0xc8, 7)] {
                    let result =
                        observe(&record(pts, Some([tag, 0, 0, override_tag, 0, 0]))).unwrap();
                    assert_eq!(result[0].vendor_light_type, Some(expected));
                }
            }
        }
    }

    #[test]
    fn missing_unknown_and_conflicting_records_are_not_collapsed() {
        let mut bytes = record(true, None);
        bytes.extend(record(false, Some([0xa2, 0, 0, 0, 0, 0])));
        bytes.extend(record(true, Some([0xb1, 0, 0, 0, 0, 0])));
        bytes.extend(record(false, Some([0, 0, 0, 0, 0, 0])));
        let result = observe(&bytes).unwrap();
        assert_eq!(
            result
                .iter()
                .map(|o| (o.record_index, o.vendor_light_type))
                .collect::<Vec<_>>(),
            [(1, Some(1)), (2, Some(2)), (3, None)]
        );
        assert!(observe(&record(false, None)).unwrap().is_empty());
        assert!(observe(&[]).unwrap().is_empty());
    }

    #[test]
    fn truncated_headers_and_flagged_errors_are_rejected() {
        for pts in [false, true] {
            let valid = record(pts, Some([0xa2; 6]));
            for end in 1..valid.len() {
                assert!(observe(&valid[..end]).is_err());
            }
            let mut error = valid.clone();
            error[11] |= 0x40;
            assert!(observe(&error).is_err());
            let mut tail = valid;
            tail.push(0);
            assert!(observe(&tail).is_err());
        }
        for length in 2..12 {
            let mut bytes = vec![0; 10 + length];
            bytes[10] = length as u8;
            bytes[11] = 12;
            assert!(observe(&bytes).is_err());
        }
    }
}
