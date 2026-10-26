//! Bounded, fully validated recording index for repeated offline frame access.
//!
//! Building reads every frame and the footer before returning any image. Seeking
//! rechecks the selected frame against its original fingerprint. The caller must
//! keep the source immutable: this does not detect changes to unselected bytes
//! after indexing (file workflows should also check source identity/timestamps).
use crate::{CaptureContext, Reader, invalid};
use scancraft_core::{Frame, StreamProfile};
use std::io::{self, Read, Seek, SeekFrom};

pub const MAX_INDEX_FRAMES: usize = 1_000_000;
/// Timestamp values observed during complete recording validation. Clock domains,
/// tick frequency and exposure timing are not established by these numbers.
#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
pub struct FrameTiming {
    pub sequence: u32,
    pub host_timestamp_us: u64,
    pub sensor_pts: u32,
}
struct Entry {
    timing: FrameTiming,
    offset: u64,
    end: u64,
    previous: Option<(u32, u64, u32)>,
    fingerprint: u32,
}
/// Owns the indexed source; no persistent index is trusted from disk.
pub struct IndexedRecording<R: Read + Seek> {
    reader: Reader<R>,
    entries: Vec<Entry>,
}
impl<R: Read + Seek> IndexedRecording<R> {
    /// `checkpoint` is called before and after each frame and after footer
    /// validation; errors abort construction, including cancellation.
    pub fn build(
        input: R,
        maximum_frames: usize,
        checkpoint: &dyn Fn(u64) -> io::Result<()>,
    ) -> io::Result<Self> {
        if maximum_frames == 0 || maximum_frames > MAX_INDEX_FRAMES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "index frame budget must be 1..=1000000",
            ));
        }
        checkpoint(0)?;
        let mut reader = Reader::new(input)?;
        let mut entries = Vec::new();
        loop {
            checkpoint(entries.len() as u64)?;
            let offset = reader.input.stream_position()?;
            let previous = reader.previous;
            let Some(frame) = reader.read_frame()? else {
                break;
            };
            if entries.len() == maximum_frames {
                return Err(invalid("recording exceeds index frame budget"));
            }
            entries.push(Entry {
                timing: FrameTiming {
                    sequence: frame.sequence,
                    host_timestamp_us: frame.host_timestamp_us,
                    sensor_pts: scancraft_core::uvc_pts(&frame.metadata)
                        .map_err(invalid)?
                        .ok_or_else(|| invalid("missing sensor PTS"))?,
                },
                offset,
                end: reader.input.stream_position()?,
                previous,
                fingerprint: fingerprint(&frame),
            });
            checkpoint(entries.len() as u64)?;
        }
        checkpoint(entries.len() as u64)?;
        Ok(Self { reader, entries })
    }
    /// Read-only access for source identity checks; callers must not mutate it.
    pub fn source(&self) -> &R {
        &self.reader.input
    }
    pub fn profile(&self) -> &StreamProfile {
        &self.reader.profile
    }
    pub fn context(&self) -> Option<&CaptureContext> {
        self.reader.context.as_ref()
    }
    pub fn version(&self) -> u8 {
        self.reader.version
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Historical timing observations from indexing, without rereading payloads.
    /// Caller must guard source identity; this is not a fresh integrity check.
    pub fn timings(&self) -> impl ExactSizeIterator<Item = FrameTiming> + '_ {
        self.entries.iter().map(|entry| entry.timing)
    }
    /// Read one frame, without rescanning other image payloads. CRC/order and
    /// fingerprint checks remain mandatory. A failed seek/read can be retried;
    /// it never returns the stale previously selected image as a new result.
    pub fn read_frame(&mut self, index: usize) -> io::Result<Frame> {
        let entry = self.entries.get(index).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "requested frame index is outside recording",
            )
        })?;
        self.reader.input.seek(SeekFrom::Start(entry.offset))?;
        self.reader.finished = false;
        self.reader.failed = false;
        self.reader.previous = entry.previous;
        self.reader.count = index as u64;
        let frame = self
            .reader
            .read_frame()?
            .ok_or_else(|| invalid("indexed frame replaced by footer"))?;
        if self.reader.input.stream_position()? != entry.end
            || fingerprint(&frame) != entry.fingerprint
        {
            return Err(invalid("indexed frame changed since validation"));
        }
        Ok(frame)
    }
}
fn fingerprint(frame: &Frame) -> u32 {
    let mut h = crc32fast::Hasher::new();
    h.update(&frame.sequence.to_le_bytes());
    h.update(&frame.host_timestamp_us.to_le_bytes());
    h.update(&frame.flags.to_le_bytes());
    h.update(&(frame.metadata.len() as u32).to_le_bytes());
    h.update(&(frame.data.len() as u32).to_le_bytes());
    h.update(&frame.metadata);
    h.update(&frame.data);
    h.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Writer;
    use std::{cell::Cell, io::Cursor, rc::Rc};
    fn fixture() -> Vec<u8> {
        let profile = StreamProfile {
            interface: 0,
            width: 2,
            height: 2,
            fourcc: *b"GREY",
            fps: 50,
        };
        let mut writer = Writer::new(Vec::new(), profile).unwrap();
        for i in 0..10u32 {
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&(100 + i * 20).to_le_bytes());
            writer
                .push(&Frame {
                    sequence: i,
                    host_timestamp_us: 1000 + u64::from(i) * 20,
                    flags: 0,
                    metadata,
                    data: vec![i as u8; 4],
                })
                .unwrap();
        }
        writer.finish().unwrap()
    }
    struct Counted {
        source: Cursor<Vec<u8>>,
        bytes: Rc<Cell<usize>>,
    }
    impl Read for Counted {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let n = self.source.read(out)?;
            self.bytes.set(self.bytes.get() + n);
            Ok(n)
        }
    }
    impl Seek for Counted {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.source.seek(pos)
        }
    }
    #[test]
    fn out_of_order_navigation_reads_only_selected_record() {
        let bytes = Rc::new(Cell::new(0));
        let source = Counted {
            source: Cursor::new(fixture()),
            bytes: bytes.clone(),
        };
        let mut index = IndexedRecording::build(source, 10, &|_| Ok(())).unwrap();
        assert_eq!(index.len(), 10);
        assert_eq!(index.version(), 2);
        assert!(index.context().is_some());
        let timings: Vec<_> = index.timings().collect();
        assert_eq!(
            timings[5],
            FrameTiming {
                sequence: 5,
                host_timestamp_us: 1100,
                sensor_pts: 200
            }
        );
        for i in [9, 0, 5, 9] {
            let before = bytes.get();
            let frame = index.read_frame(i).unwrap();
            assert_eq!(frame.data, vec![i as u8; 4]);
            assert_eq!(bytes.get() - before, 52);
        }
        let before = bytes.get();
        assert_eq!(index.timings().collect::<Vec<_>>(), timings);
        assert_eq!(
            bytes.get(),
            before,
            "timing inspection must not reread payloads"
        );
        assert!(index.read_frame(10).is_err());
        assert_eq!(index.read_frame(0).unwrap().sequence, 0);
    }
    #[test]
    fn index_rejects_bad_suffix_even_when_first_frame_valid() {
        let original = fixture();
        for removed in 1..=12 {
            assert!(
                IndexedRecording::build(
                    Cursor::new(&original[..original.len() - removed]),
                    10,
                    &|_| Ok(())
                )
                .is_err()
            );
        }
        let mut corrupt = original.clone();
        let last_payload = corrupt.len() - 13;
        corrupt[last_payload] ^= 1;
        assert!(IndexedRecording::build(Cursor::new(corrupt), 10, &|_| Ok(())).is_err());
        let mut trailing = original;
        trailing.push(0);
        assert!(IndexedRecording::build(Cursor::new(trailing), 10, &|_| Ok(())).is_err());
    }
    #[test]
    fn budgets_and_cancellation_are_errors_not_partial_indexes() {
        for maximum in [0, 9, MAX_INDEX_FRAMES + 1] {
            assert!(IndexedRecording::build(Cursor::new(fixture()), maximum, &|_| Ok(())).is_err());
        }
        let result = IndexedRecording::build(Cursor::new(fixture()), 10, &|count| {
            if count >= 3 {
                Err(io::Error::other("cancelled"))
            } else {
                Ok(())
            }
        });
        assert!(matches!(result, Err(e) if e.to_string() == "cancelled"));
    }
    #[test]
    fn rejects_changed_frame_even_with_recomputed_crc() {
        let mut index = IndexedRecording::build(Cursor::new(fixture()), 10, &|_| Ok(())).unwrap();
        let entry = &index.entries[5];
        let start = entry.offset as usize;
        let end = entry.end as usize;
        let data = index.reader.input.get_mut();
        data[end - 1] ^= 1;
        let crc = crc32fast::hash(&data[start + 8..end]);
        data[start + 4..start + 8].copy_from_slice(&crc.to_le_bytes());
        assert!(
            index
                .read_frame(5)
                .unwrap_err()
                .to_string()
                .contains("changed since validation")
        );
        assert_eq!(index.read_frame(4).unwrap().sequence, 4);
    }
}
