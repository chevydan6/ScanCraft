//! Streaming recording v2. CRC covers the session header and every frame.
pub mod indexed;
pub mod selection;
use scancraft_core::{Frame, StreamProfile, uvc_pts, validate_frame};
use std::io::{self, Read, Write};
const MAGIC: &[u8; 8] = b"RAPTOR02";
const LEGACY_MAGIC: &[u8; 8] = b"RAPTOR01";
const MAX_HEADER: usize = 1024 * 1024;
const MAX_FRAME: usize = 64 * 1024 * 1024;
/// Bound declared decoded image size before frame allocation or image decoding.
const MAX_IMAGE_PIXELS: u64 = 16 * 1024 * 1024;
fn validate_profile(profile: &StreamProfile) -> io::Result<()> {
    if profile.width == 0 || profile.height == 0 || profile.fps == 0 {
        return Err(invalid(
            "recording dimensions and frame rate must be nonzero",
        ));
    }
    if u64::from(profile.width) * u64::from(profile.height) > MAX_IMAGE_PIXELS {
        return Err(invalid("recording image exceeds 16777216-pixel limit"));
    }
    if !matches!(&profile.fourcc, b"GREY" | b"Y14 " | b"MJPG") {
        return Err(invalid("unsupported recording pixel format"));
    }
    Ok(())
}
fn invalid(s: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s.into())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureContext {
    pub device_identity: Option<String>,
    pub calibration_bytes: Option<Vec<u8>>,
    pub calibration_status: String,
    pub controls: serde_json::Value,
}
impl Default for CaptureContext {
    fn default() -> Self {
        Self {
            device_identity: None,
            calibration_bytes: None,
            calibration_status: "not_loaded_or_verified".into(),
            controls: serde_json::Value::Null,
        }
    }
}
impl CaptureContext {
    /// Add typed observations without inferring identity/calibration or replacing trial metadata.
    pub fn add_capture_observations(
        &mut self,
        observations: &impl serde::Serialize,
    ) -> io::Result<()> {
        let value = serde_json::to_value(observations)?;
        if !self.controls.is_null() && !self.controls.is_object() {
            return Err(invalid(
                "control metadata must be an object to add capture observations",
            ));
        }
        if self.controls.get("capture_observations").is_some() {
            return Err(invalid("capture observations already attached"));
        }
        if self.controls.is_null() {
            self.controls = serde_json::json!({});
        }
        self.controls
            .as_object_mut()
            .unwrap()
            .insert("capture_observations".into(), value);
        Ok(())
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    profile: StreamProfile,
    context: CaptureContext,
}

pub struct Writer<W: Write> {
    output: W,
    profile: StreamProfile,
    count: u64,
    previous: Option<(u32, u64, u32)>,
    failed: bool,
}
impl<W: Write> Writer<W> {
    pub fn new(output: W, profile: StreamProfile) -> io::Result<Self> {
        Self::with_context(output, profile, CaptureContext::default())
    }
    pub fn with_context(
        mut output: W,
        profile: StreamProfile,
        context: CaptureContext,
    ) -> io::Result<Self> {
        validate_profile(&profile)?;
        let header = serde_json::to_vec(&Header {
            profile: profile.clone(),
            context,
        })?;
        if header.len() > MAX_HEADER {
            return Err(invalid("profile too large"));
        }
        output.write_all(MAGIC)?;
        output.write_all(&(header.len() as u32).to_le_bytes())?;
        output.write_all(&crc32fast::hash(&header).to_le_bytes())?;
        output.write_all(&header)?;
        Ok(Self {
            output,
            profile,
            count: 0,
            previous: None,
            failed: false,
        })
    }
    pub fn push(&mut self, frame: &Frame) -> io::Result<()> {
        if self.failed {
            return Err(invalid("recording writer unusable after output error"));
        }
        validate_frame(&self.profile, frame).map_err(invalid)?;
        let order = validate_order(self.previous, frame)?;
        let length = 24usize
            .checked_add(frame.metadata.len())
            .and_then(|n| n.checked_add(frame.data.len()))
            .ok_or_else(|| invalid("frame length overflow"))?;
        if frame.metadata.len() > 4096 || length > MAX_FRAME {
            return Err(invalid("frame exceeds size limits"));
        }
        let mut body = Vec::with_capacity(length);
        body.extend_from_slice(&frame.sequence.to_le_bytes());
        body.extend_from_slice(&frame.host_timestamp_us.to_le_bytes());
        body.extend_from_slice(&frame.flags.to_le_bytes());
        body.extend_from_slice(&(frame.metadata.len() as u32).to_le_bytes());
        body.extend_from_slice(&(frame.data.len() as u32).to_le_bytes());
        body.extend_from_slice(&frame.metadata);
        body.extend_from_slice(&frame.data);
        if body.len() > MAX_FRAME {
            return Err(invalid("frame too large"));
        }
        let result = (|| {
            self.output.write_all(&(body.len() as u32).to_le_bytes())?;
            self.output
                .write_all(&crc32fast::hash(&body).to_le_bytes())?;
            self.output.write_all(&body)
        })();
        if let Err(error) = result {
            self.failed = true;
            return Err(error);
        }
        self.count += 1;
        self.previous = Some(order);
        Ok(())
    }
    pub fn finish(mut self) -> io::Result<W> {
        if self.failed {
            return Err(invalid("recording writer unusable after output error"));
        }
        self.output.write_all(&0u32.to_le_bytes())?;
        self.output.write_all(&self.count.to_le_bytes())?;
        self.output.flush()?;
        Ok(self.output)
    }
}

pub struct Reader<R: Read> {
    input: R,
    pub profile: StreamProfile,
    pub context: Option<CaptureContext>,
    pub version: u8,
    count: u64,
    finished: bool,
    previous: Option<(u32, u64, u32)>,
    failed: bool,
}
impl<R: Read> Reader<R> {
    pub fn new(mut input: R) -> io::Result<Self> {
        let mut magic = [0; 8];
        input.read_exact(&mut magic)?;
        if &magic != MAGIC && &magic != LEGACY_MAGIC {
            return Err(invalid("unknown recording version"));
        }
        let n = read_u32(&mut input)? as usize;
        if n > MAX_HEADER {
            return Err(invalid("profile exceeds limit"));
        }
        let expected_crc = if &magic == MAGIC {
            Some(read_u32(&mut input)?)
        } else {
            None
        };
        let mut buf = vec![0; n];
        input.read_exact(&mut buf)?;
        if expected_crc.is_some_and(|crc| crc32fast::hash(&buf) != crc) {
            return Err(invalid("session header CRC mismatch"));
        }
        let (profile, context, version): (StreamProfile, Option<CaptureContext>, u8) =
            if &magic == MAGIC {
                let h: Header = serde_json::from_slice(&buf)?;
                (h.profile, Some(h.context), 2)
            } else {
                (serde_json::from_slice(&buf)?, None, 1)
            };
        validate_profile(&profile)?;
        Ok(Self {
            input,
            profile,
            context,
            version,
            count: 0,
            finished: false,
            previous: None,
            failed: false,
        })
    }
    pub fn read_frame(&mut self) -> io::Result<Option<Frame>> {
        if self.failed {
            return Err(invalid("recording reader unusable after stream error"));
        }
        let result = self.read_frame_inner();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn read_frame_inner(&mut self) -> io::Result<Option<Frame>> {
        if self.finished {
            return Ok(None);
        }
        let n = read_u32(&mut self.input)? as usize;
        if n == 0 {
            let mut b = [0; 8];
            self.input.read_exact(&mut b)?;
            if u64::from_le_bytes(b) != self.count {
                return Err(invalid("footer frame count mismatch"));
            }
            let mut extra = [0];
            if self.input.read(&mut extra)? != 0 {
                return Err(invalid("trailing bytes"));
            }
            self.finished = true;
            return Ok(None);
        }
        if n > MAX_FRAME {
            return Err(invalid("frame exceeds size limit"));
        }
        let crc = read_u32(&mut self.input)?;
        let mut buf = vec![0; n];
        self.input.read_exact(&mut buf)?;
        if crc32fast::hash(&buf) != crc {
            return Err(invalid("frame CRC mismatch"));
        }
        if buf.len() < 24 {
            return Err(invalid("truncated frame header"));
        }
        let metadata_len = u32::from_le_bytes(buf[16..20].try_into().unwrap()) as usize;
        let payload_len = u32::from_le_bytes(buf[20..24].try_into().unwrap()) as usize;
        let declared_length = 24_usize
            .checked_add(metadata_len)
            .and_then(|n| n.checked_add(payload_len));
        if metadata_len > 4096 || declared_length != Some(buf.len()) {
            return Err(invalid("invalid frame lengths"));
        }
        let frame = Frame {
            sequence: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            host_timestamp_us: u64::from_le_bytes(buf[4..12].try_into().unwrap()),
            flags: u32::from_le_bytes(buf[12..16].try_into().unwrap()),
            metadata: buf[24..24 + metadata_len].to_vec(),
            data: buf[24 + metadata_len..].to_vec(),
        };
        validate_frame(&self.profile, &frame).map_err(invalid)?;
        self.previous = Some(validate_order(self.previous, &frame)?);
        self.count += 1;
        Ok(Some(frame))
    }
}
fn validate_order(previous: Option<(u32, u64, u32)>, frame: &Frame) -> io::Result<(u32, u64, u32)> {
    let pts = uvc_pts(&frame.metadata)
        .map_err(invalid)?
        .ok_or_else(|| invalid("missing PTS"))?;
    if let Some((seq, timestamp, old_pts)) = previous {
        let d = frame.sequence.wrapping_sub(seq);
        let dt = pts.wrapping_sub(old_pts);
        if d == 0
            || d >= 0x8000_0000
            || frame.host_timestamp_us <= timestamp
            || dt == 0
            || dt >= 0x8000_0000
        {
            return Err(invalid("non-monotonic recording sequence/timestamp/PTS"));
        }
    }
    Ok((frame.sequence, frame.host_timestamp_us, pts))
}

fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut b = [0; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> (StreamProfile, Frame) {
        let p = StreamProfile {
            interface: 0,
            width: 2,
            height: 2,
            fourcc: *b"GREY",
            fps: 50,
        };
        let mut m = vec![0; 22];
        m[10] = 12;
        m[11] = 0x0c;
        (
            p,
            Frame {
                sequence: 5,
                host_timestamp_us: 100,
                flags: 0,
                metadata: m,
                data: vec![1, 2, 3, 4],
            },
        )
    }
    fn recording() -> Vec<u8> {
        let (p, f) = sample();
        let mut w = Writer::new(Vec::new(), p).unwrap();
        w.push(&f).unwrap();
        w.finish().unwrap()
    }
    #[test]
    fn header_corruption_rejected() {
        let original = recording();
        let n = u32::from_le_bytes(original[8..12].try_into().unwrap()) as usize;
        for i in 12..16 + n {
            let mut b = original.clone();
            b[i] ^= 1;
            assert!(Reader::new(b.as_slice()).is_err());
        }
    }
    #[test]
    fn duplicate_frame_rejected() {
        let (p, f) = sample();
        let mut w = Writer::new(Vec::new(), p).unwrap();
        w.push(&f).unwrap();
        assert!(w.push(&f).is_err());
    }
    #[test]
    fn roundtrip_100_replays() {
        let bytes = recording();
        for _ in 0..100 {
            let mut r = Reader::new(bytes.as_slice()).unwrap();
            assert_eq!(r.read_frame().unwrap().unwrap(), sample().1);
            assert!(r.read_frame().unwrap().is_none());
        }
    }
    #[test]
    fn every_truncation_rejected() {
        let b = recording();
        for n in 0..b.len() {
            let result = (|| {
                let mut r = Reader::new(&b[..n])?;
                while r.read_frame()?.is_some() {}
                Ok::<_, io::Error>(())
            })();
            assert!(result.is_err(), "accepted truncation {n}");
        }
    }
    #[test]
    fn corrupted_payload_rejected() {
        let mut b = recording();
        let n = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
        b[16 + n + 9] ^= 1;
        let mut r = Reader::new(b.as_slice()).unwrap();
        assert!(r.read_frame().is_err());
    }
    #[test]
    fn oversized_record_rejected() {
        let mut b = recording();
        let n = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
        b[16 + n..20 + n].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut r = Reader::new(b.as_slice()).unwrap();
        assert!(r.read_frame().is_err());
    }
    fn empty_with_profile(profile: &StreamProfile, legacy: bool) -> Vec<u8> {
        // Construct independently of Writer so a valid CRC cannot hide bad semantics.
        let header = if legacy {
            serde_json::to_vec(profile).unwrap()
        } else {
            serde_json::to_vec(&Header {
                profile: profile.clone(),
                context: CaptureContext::default(),
            })
            .unwrap()
        };
        let mut bytes = if legacy {
            LEGACY_MAGIC.to_vec()
        } else {
            MAGIC.to_vec()
        };
        bytes.extend_from_slice(&(header.len() as u32).to_le_bytes());
        if !legacy {
            bytes.extend_from_slice(&crc32fast::hash(&header).to_le_bytes());
        }
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes
    }
    #[test]
    fn invalid_profiles_fail_before_writer_output_and_before_empty_replay() {
        let (profile, _) = sample();
        let mut cases = Vec::new();
        for field in 0..6 {
            let mut invalid = profile.clone();
            match field {
                0 => invalid.width = 0,
                1 => invalid.height = 0,
                2 => invalid.fps = 0,
                3 => invalid.fourcc = *b"BAD!",
                4 => {
                    invalid.width = u32::MAX;
                    invalid.height = u32::MAX;
                }
                _ => {
                    invalid.width = 4096;
                    invalid.height = 4097;
                }
            }
            cases.push(invalid);
        }
        for invalid in cases {
            let mut output = vec![1, 2, 3];
            assert!(Writer::new(&mut output, invalid.clone()).is_err());
            assert_eq!(output, [1, 2, 3]);
            for legacy in [false, true] {
                let bytes = empty_with_profile(&invalid, legacy);
                assert!(Reader::new(bytes.as_slice()).is_err());
            }
        }
    }
    #[test]
    fn supported_profiles_and_pixel_limit_accept_empty_recordings_without_allocating_images() {
        let (mut profile, _) = sample();
        profile.width = 4096;
        profile.height = 4096;
        for fourcc in [*b"GREY", *b"Y14 ", *b"MJPG"] {
            profile.fourcc = fourcc;
            for legacy in [false, true] {
                let bytes = empty_with_profile(&profile, legacy);
                let mut reader = Reader::new(bytes.as_slice()).unwrap();
                assert!(reader.read_frame().unwrap().is_none());
            }
            assert!(
                Writer::new(Vec::new(), profile.clone())
                    .unwrap()
                    .finish()
                    .is_ok()
            );
        }
    }
}

pub mod recovery;
