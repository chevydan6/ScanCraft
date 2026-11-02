//! Private helper protocol: bounded binary frames and bounded JSON completion.
use scancraft_core::Frame;
use std::io::{self, Write};
const FRAME: u8 = 1;
const COMPLETE: u8 = 2;
const LIVE_PROGRESS: u8 = 3;
const LIVE_COMPLETE: u8 = 4;
const LASER_PROGRESS: u8 = 5;
const LASER_COMPLETE: u8 = 6;
const MAX_LASER_PROGRESS: usize = 2 * 1024 * 1024;
pub(crate) const MAX_PROGRESS: usize = 16 * 1024 * 1024;
const MAX_FRAME: usize = 64 * 1024 * 1024;
const MAX_RESULT: usize = 64 * 1024;
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
#[derive(Debug, PartialEq)]
pub(crate) enum Message {
    Frame(Frame),
    LaserProgress(Box<crate::laser_display::Snapshot>),
    LaserComplete(Result<crate::laser_process::Completion, String>),
    Complete(Result<String, String>),
    LiveProgress(scancraft_workflow::live_stereo::control::display::DisplayUpdate),
    LiveComplete(Result<serde_json::Value, String>),
}
fn prefix(output: &mut impl Write, kind: u8, length: usize) -> io::Result<()> {
    output.write_all(&[kind])?;
    output.write_all(&(length as u32).to_le_bytes())
}
/// Send and flush a complete frame; a buffered tail must not await the next frame.
pub(crate) fn write_frame(output: &mut impl Write, frame: &Frame) -> io::Result<()> {
    let length = 24usize
        .checked_add(frame.metadata.len())
        .and_then(|n| n.checked_add(frame.data.len()))
        .ok_or_else(|| invalid("IPC frame overflow"))?;
    if length > MAX_FRAME {
        return Err(invalid("IPC frame exceeds limit"));
    }
    prefix(output, FRAME, length)?;
    output.write_all(&frame.sequence.to_le_bytes())?;
    output.write_all(&frame.host_timestamp_us.to_le_bytes())?;
    output.write_all(&frame.flags.to_le_bytes())?;
    output.write_all(&(frame.metadata.len() as u32).to_le_bytes())?;
    output.write_all(&(frame.data.len() as u32).to_le_bytes())?;
    output.write_all(&frame.metadata)?;
    output.write_all(&frame.data)?;
    output.flush()
}
struct LimitedBytes {
    bytes: Vec<u8>,
    maximum: usize,
}
impl Write for LimitedBytes {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.bytes.len().saturating_add(data.len()) > self.maximum {
            return Err(invalid("IPC JSON exceeds its size limit"));
        }
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn prepare_json(
    kind: u8,
    maximum: usize,
    serialize: impl FnOnce(&mut LimitedBytes) -> serde_json::Result<()>,
) -> io::Result<Vec<u8>> {
    let mut bounded = LimitedBytes {
        bytes: vec![kind, 0, 0, 0, 0],
        maximum: maximum + 5,
    };
    serialize(&mut bounded)?;
    let length = bounded.bytes.len() - 5;
    bounded.bytes[1..5].copy_from_slice(&(length as u32).to_le_bytes());
    Ok(bounded.bytes)
}
fn write_json(
    output: &mut impl Write,
    kind: u8,
    maximum: usize,
    serialize: impl FnOnce(&mut LimitedBytes) -> serde_json::Result<()>,
) -> io::Result<()> {
    let packet = prepare_json(kind, maximum, serialize)?;
    output.write_all(&packet)?;
    output.flush()
}
pub(crate) fn write_complete(
    output: &mut impl Write,
    result: &Result<String, String>,
) -> io::Result<()> {
    write_json(output, COMPLETE, MAX_RESULT, |bytes| {
        serde_json::to_writer(bytes, result)
    })
}
/// Validate/encode entirely before pipe I/O, allowing display-only rejections
/// to be handled independently from a disconnected parent or broken pipe.
pub(crate) fn prepare_live_progress(
    update: &scancraft_workflow::live_stereo::control::display::DisplayUpdate,
) -> io::Result<Vec<u8>> {
    update.validate().map_err(|e| invalid(&e))?;
    prepare_json(LIVE_PROGRESS, MAX_PROGRESS, |bytes| {
        serde_json::to_writer(bytes, update)
    })
}
pub(crate) fn write_live_complete(
    output: &mut impl Write,
    result: &Result<serde_json::Value, String>,
) -> io::Result<()> {
    write_json(output, LIVE_COMPLETE, MAX_RESULT, |bytes| {
        serde_json::to_writer(bytes, result)
    })
}
pub(crate) fn prepare_laser_progress(
    snapshot: &crate::laser_display::Snapshot,
) -> io::Result<Vec<u8>> {
    snapshot.validate().map_err(|e| invalid(&e))?;
    prepare_json(LASER_PROGRESS, MAX_LASER_PROGRESS, |bytes| {
        serde_json::to_writer(bytes, snapshot)
    })
}
pub(crate) fn write_laser_complete(
    out: &mut impl Write,
    result: &Result<crate::laser_process::Completion, String>,
) -> io::Result<()> {
    write_json(out, LASER_COMPLETE, MAX_RESULT, |bytes| {
        serde_json::to_writer(bytes, result)
    })
}
#[derive(Default)]
pub(crate) struct Decoder {
    bytes: Vec<u8>,
    completed: bool,
}
impl Decoder {
    /// One 64KiB read may follow a nearly complete maximum-sized packet.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.completed && !bytes.is_empty() {
            return Err(invalid("IPC data after completion"));
        }
        if self.bytes.len().saturating_add(bytes.len()) > MAX_FRAME + 5 + 65536 {
            return Err(invalid("IPC buffered data exceeds limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    pub(crate) fn next(&mut self) -> io::Result<Option<Message>> {
        if self.completed && !self.bytes.is_empty() {
            return Err(invalid("IPC data after completion"));
        }
        if self.bytes.len() < 5 {
            return Ok(None);
        }
        let kind = self.bytes[0];
        let length = u32::from_le_bytes(self.bytes[1..5].try_into().unwrap()) as usize;
        match kind {
            FRAME if (24..=MAX_FRAME).contains(&length) => (),
            COMPLETE | LIVE_COMPLETE | LASER_COMPLETE if (1..=MAX_RESULT).contains(&length) => (),
            LIVE_PROGRESS if (1..=MAX_PROGRESS).contains(&length) => (),
            LASER_PROGRESS if (1..=MAX_LASER_PROGRESS).contains(&length) => (),
            _ => return Err(invalid("invalid IPC kind or declared length")),
        }
        if self.bytes.len() < 5 + length {
            return Ok(None);
        }
        let body = &self.bytes[5..5 + length];
        let message = if kind == FRAME {
            let metadata = u32::from_le_bytes(body[16..20].try_into().unwrap()) as usize;
            let data = u32::from_le_bytes(body[20..24].try_into().unwrap()) as usize;
            if metadata.checked_add(data).and_then(|n| n.checked_add(24)) != Some(length) {
                return Err(invalid("IPC frame lengths disagree"));
            }
            Message::Frame(Frame {
                sequence: u32::from_le_bytes(body[..4].try_into().unwrap()),
                host_timestamp_us: u64::from_le_bytes(body[4..12].try_into().unwrap()),
                flags: u32::from_le_bytes(body[12..16].try_into().unwrap()),
                metadata: body[24..24 + metadata].to_vec(),
                data: body[24 + metadata..].to_vec(),
            })
        } else if kind == LASER_PROGRESS {
            let preview: crate::laser_display::Snapshot = serde_json::from_slice(body)?;
            preview.validate().map_err(|e| invalid(&e))?;
            Message::LaserProgress(Box::new(preview))
        } else if kind == LIVE_PROGRESS {
            let update: scancraft_workflow::live_stereo::control::display::DisplayUpdate =
                serde_json::from_slice(body)?;
            update.validate().map_err(|e| invalid(&e))?;
            Message::LiveProgress(update)
        } else {
            self.completed = true;
            if kind == LASER_COMPLETE {
                let result: Result<crate::laser_process::Completion, String> =
                    serde_json::from_slice(body)?;
                if let Ok(c) = &result {
                    c.validate().map_err(|e| invalid(&e))?;
                }
                Message::LaserComplete(result)
            } else if kind == COMPLETE {
                Message::Complete(serde_json::from_slice(body)?)
            } else {
                Message::LiveComplete(serde_json::from_slice(body)?)
            }
        };
        self.bytes.drain(..5 + length);
        Ok(Some(message))
    }
    pub(crate) fn finish(&self) -> io::Result<()> {
        if self.completed && self.bytes.is_empty() {
            Ok(())
        } else {
            Err(invalid("IPC ended without a complete final result"))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn laser_packets_fragmented_and_validated_before_display_or_completion() {
        let snapshot =
            crate::laser_display::tests::snapshot(crate::laser_display::Origin::Native, 0);
        let mut wire = prepare_laser_progress(&snapshot).unwrap();
        write_laser_complete(&mut wire, &Err("restoration failed".into())).unwrap();
        for size in [1, 3, 17, 1024, 65536] {
            let mut d = Decoder::default();
            let mut messages = Vec::new();
            for chunk in wire.chunks(size) {
                d.push(chunk).unwrap();
                while let Some(m) = d.next().unwrap() {
                    messages.push(m);
                }
            }
            d.finish().unwrap();
            assert_eq!(messages.len(), 2);
            assert!(matches!(&messages[0],Message::LaserProgress(s) if s.as_ref()==&snapshot));
            assert!(
                matches!(&messages[1],Message::LaserComplete(Err(e)) if e=="restoration failed")
            );
            assert!(d.push(&[1]).is_err());
        }
        let mut bad =
            crate::laser_display::tests::snapshot(crate::laser_display::Origin::Native, 0);
        bad.frames[0].pixels.clear();
        assert!(prepare_laser_progress(&bad).is_err());
        for (kind, length) in [
            (LASER_PROGRESS, MAX_LASER_PROGRESS + 1),
            (LASER_COMPLETE, MAX_RESULT + 1),
        ] {
            let mut d = Decoder::default();
            let mut header = vec![kind];
            header.extend((length as u32).to_le_bytes());
            d.push(&header).unwrap();
            assert!(d.next().is_err());
        }
        let mut d = Decoder::default();
        let invalid = serde_json::json!({"Ok":{"schema_version":1,"origin":"native","recorded":[1,1],"drops":[2,0],"processed":0,"failed":0,"pending":[0,0],"helper_omissions":0}});
        let packet = prepare_json(LASER_COMPLETE, MAX_RESULT, |out| {
            serde_json::to_writer(out, &invalid)
        })
        .unwrap();
        d.push(&packet).unwrap();
        assert!(d.next().is_err());
        assert!(d.finish().is_err());
    }
    #[test]
    fn fragmented_frame_and_completion_preserve_exact_data() {
        let frame = Frame {
            sequence: u32::MAX,
            host_timestamp_us: u64::MAX,
            flags: 17,
            metadata: vec![1, 2, 3],
            data: vec![4, 5, 6, 7],
        };
        let mut wire = Vec::new();
        write_frame(&mut wire, &frame).unwrap();
        write_complete(&mut wire, &Err("close failed".into())).unwrap();
        for chunk in 1..=wire.len() {
            let mut decoder = Decoder::default();
            let mut messages = Vec::new();
            for bytes in wire.chunks(chunk) {
                decoder.push(bytes).unwrap();
                while let Some(message) = decoder.next().unwrap() {
                    messages.push(message);
                }
            }
            decoder.finish().unwrap();
            assert_eq!(
                messages,
                [
                    Message::Frame(frame.clone()),
                    Message::Complete(Err("close failed".into()))
                ]
            );
        }
    }
    #[test]
    fn one_frame_is_visible_without_a_second_frame_or_completion() {
        let frame = Frame {
            sequence: 1,
            host_timestamp_us: 2,
            flags: 0,
            metadata: vec![3],
            data: vec![4; 16],
        };
        let mut buffered = io::BufWriter::with_capacity(4096, Vec::new());
        write_frame(&mut buffered, &frame).unwrap();
        let mut decoder = Decoder::default();
        // Inspect only bytes already delivered to the underlying pipe/writer.
        decoder.push(buffered.get_ref()).unwrap();
        assert_eq!(decoder.next().unwrap(), Some(Message::Frame(frame)));
    }

    #[test]
    fn frame_flush_failure_is_a_transport_failure() {
        struct BrokenFlush;
        impl Write for BrokenFlush {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
            }
        }
        let frame = Frame {
            sequence: 1,
            host_timestamp_us: 2,
            flags: 0,
            metadata: vec![],
            data: vec![3],
        };
        assert_eq!(
            write_frame(&mut BrokenFlush, &frame).unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
    }

    #[test]
    fn fragmented_live_updates_and_typed_completion_roundtrip() {
        let update =
            scancraft_workflow::live_stereo::control::Control::new(true).take_display_update();
        let complete = Ok(
            serde_json::json!({"kind":"synthetic_live_stereo_pipeline","geometry_exported":false}),
        );
        let mut wire = Vec::new();
        wire.extend(prepare_live_progress(&update).unwrap());
        write_live_complete(&mut wire, &complete).unwrap();
        for size in 1..=wire.len() {
            let mut decoder = Decoder::default();
            let mut messages = Vec::new();
            for chunk in wire.chunks(size) {
                decoder.push(chunk).unwrap();
                while let Some(message) = decoder.next().unwrap() {
                    messages.push(message);
                }
            }
            decoder.finish().unwrap();
            assert_eq!(messages.len(), 2);
            match &messages[0] {
                Message::LiveProgress(value) => assert_eq!(value, &update),
                _ => panic!("wrong progress type"),
            }
            match &messages[1] {
                Message::LiveComplete(value) => assert_eq!(value, &complete),
                _ => panic!("wrong completion type"),
            }
        }
    }
    #[test]
    fn live_json_limits_fail_before_any_partial_packet_is_published() {
        let mut wire = Vec::new();
        assert!(write_live_complete(&mut wire, &Err("x".repeat(MAX_RESULT + 1))).is_err());
        assert!(wire.is_empty());
        for (kind, length) in [
            (LIVE_PROGRESS, MAX_PROGRESS + 1),
            (LIVE_COMPLETE, MAX_RESULT + 1),
        ] {
            let mut header = vec![kind];
            header.extend((length as u32).to_le_bytes());
            let mut decoder = Decoder::default();
            decoder.push(&header).unwrap();
            assert!(decoder.next().is_err());
        }
        let mut frame = vec![FRAME];
        frame.extend(24u32.to_le_bytes());
        frame.extend([0; 24]);
        frame[5 + 16..5 + 20].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut decoder = Decoder::default();
        decoder.push(&frame).unwrap();
        assert!(decoder.next().is_err());
    }

    #[test]
    fn corrupt_sizes_truncation_and_trailing_data_never_succeed() {
        for kind in [FRAME, COMPLETE, 0, 255] {
            let mut decoder = Decoder::default();
            decoder.push(&[kind, 255, 255, 255, 255]).unwrap();
            assert!(decoder.next().is_err());
        }
        let mut wire = Vec::new();
        write_complete(&mut wire, &Ok("done".into())).unwrap();
        for end in 0..wire.len() {
            let mut decoder = Decoder::default();
            decoder.push(&wire[..end]).unwrap();
            let _ = decoder.next();
            assert!(decoder.finish().is_err());
        }
        let mut decoder = Decoder::default();
        wire.push(0);
        decoder.push(&wire).unwrap();
        assert!(decoder.next().unwrap().is_some());
        assert!(decoder.next().is_err());
        assert!(decoder.finish().is_err());
    }
}
