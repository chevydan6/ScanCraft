//! Streaming, aggregate-only SCR diagnostics; capture context is not emitted.
use scancraft_device::scr_observations::observe;
use scancraft_recording::Reader;
use serde::Serialize;
use std::io::{self, Read};

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub frames: u64,
    pub frames_without_scr: u64,
    /// Frames containing at least two distinct known numeric types.
    pub frames_with_conflicting_known_types: u64,
    pub scr_records: u64,
    pub unknown_tag_records: u64,
    /// Per-record counts indexed by numeric vendor type, not frame counts.
    pub vendor_type_record_counts: [u64; 8],
}

/// Read through the recording's verified footer; fail rather than publish partial
/// results on corrupt frames or malformed metadata. Memory is bounded per frame.
pub fn inspect<R: Read>(reader: &mut Reader<R>) -> io::Result<Report> {
    let mut report = Report::default();
    while let Some(frame) = reader.read_frame()? {
        let observations =
            observe(&frame.metadata).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        report.frames += 1;
        report.frames_without_scr += u64::from(observations.is_empty());
        let mut seen = 0u8;
        for observation in observations {
            report.scr_records += 1;
            if let Some(kind) = observation.vendor_light_type {
                report.vendor_type_record_counts[usize::from(kind)] += 1;
                seen |= 1 << kind;
            } else {
                report.unknown_tag_records += 1;
            }
        }
        report.frames_with_conflicting_known_types += u64::from(seen.count_ones() > 1);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_core::{Frame, StreamProfile};
    use scancraft_recording::Writer;

    #[test]
    fn recording_crc_and_conflict_diagnostics() {
        let profile = StreamProfile {
            interface: 0,
            width: 1,
            height: 1,
            fourcc: *b"GREY",
            fps: 30,
        };
        let mut writer = Writer::new(Vec::new(), profile).unwrap();
        let mut metadata = Vec::new();
        for tag in [0xa2, 0xb1, 0] {
            metadata.extend([0; 10]);
            metadata.extend([12, 12]);
            metadata.extend(123u32.to_le_bytes());
            metadata.extend([tag, 0, 0, 0, 0, 0]);
        }
        writer
            .push(&Frame {
                sequence: 0,
                host_timestamp_us: 0,
                flags: 0,
                metadata,
                data: vec![0],
            })
            .unwrap();
        let bytes = writer.finish().unwrap();
        let report = inspect(&mut Reader::new(bytes.as_slice()).unwrap()).unwrap();
        assert_eq!(report.frames, 1);
        assert_eq!(report.scr_records, 3);
        assert_eq!(report.frames_with_conflicting_known_types, 1);
        assert_eq!(report.unknown_tag_records, 1);
        assert_eq!(report.vendor_type_record_counts[1..3], [1, 1]);
        assert!(inspect(&mut Reader::new(&bytes[..bytes.len() - 1]).unwrap()).is_err());
    }
}
