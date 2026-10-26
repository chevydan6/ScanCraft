//! Versioned sensor data. Coordinates remain undefined until calibrated reconstruction.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamProfile {
    pub interface: u8,
    pub width: u32,
    pub height: u32,
    pub fourcc: [u8; 4],
    pub fps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Frame {
    pub sequence: u32,
    pub host_timestamp_us: u64,
    pub flags: u32,
    /// Exact Linux UVC metadata records, paired by V4L2 sequence.
    pub metadata: Vec<u8>,
    pub data: Vec<u8>,
}

pub fn uvc_pts(metadata: &[u8]) -> Result<Option<u32>, &'static str> {
    let mut pos = 0;
    let mut pts = None;
    while pos < metadata.len() {
        if metadata.len() - pos < 12 {
            return Err("truncated UVC metadata record");
        }
        let length = metadata[pos + 10] as usize;
        if length < 2 || pos + 10 + length > metadata.len() {
            return Err("invalid UVC header length");
        }
        let flags = metadata[pos + 11];
        if flags & 0x40 != 0 {
            return Err("device reported a UVC frame error");
        }
        if flags & 0x04 != 0 {
            if length < 6 {
                return Err("truncated UVC PTS");
            }
            let value = u32::from_le_bytes(metadata[pos + 12..pos + 16].try_into().unwrap());
            if pts.is_some_and(|p| p != value) {
                return Err("inconsistent per-frame PTS");
            }
            pts = Some(value);
        }
        pos += 10 + length;
    }
    Ok(pts)
}

pub fn validate_frame(profile: &StreamProfile, frame: &Frame) -> Result<(), &'static str> {
    if frame.flags & 0x40 != 0 {
        return Err("V4L2 error flag");
    }
    let pixels = (profile.width as usize)
        .checked_mul(profile.height as usize)
        .ok_or("image size overflow")?;
    match &profile.fourcc {
        b"GREY" if frame.data.len() == pixels => (),
        b"Y14 " if pixels.checked_mul(2) == Some(frame.data.len()) => (),
        b"MJPG" if valid_jpeg_envelope(&frame.data) => {}
        _ => return Err("unsupported format or invalid frame payload length/markers"),
    }
    if uvc_pts(&frame.metadata)?.is_none() {
        return Err("missing sensor PTS");
    }
    Ok(())
}

/// Raptor MJPEG buffers may contain zero padding after the EOI marker.
/// This validates framing only; image decoding is a separate validation stage.
fn valid_jpeg_envelope(data: &[u8]) -> bool {
    data.starts_with(&[0xff, 0xd8])
        && data
            .windows(2)
            .rposition(|w| w == [0xff, 0xd9])
            .is_some_and(|end| data[end + 2..].iter().all(|b| *b == 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jpeg_padding_and_corruption() {
        assert!(valid_jpeg_envelope(&[255, 216, 1, 2, 255, 217, 0, 0]));
        assert!(!valid_jpeg_envelope(&[255, 216, 1, 2, 255, 217, 0, 1]));
        assert!(!valid_jpeg_envelope(&[255, 216, 1, 2, 0, 0]));
    }
    #[test]
    fn malformed_metadata_never_panics() {
        for n in 0..100 {
            assert!(uvc_pts(&vec![255; n]).is_err() || n == 0);
        }
    }
    #[test]
    fn genuine_record_layout_pts() {
        let mut m = vec![0; 22];
        m[10] = 12;
        m[11] = 0x0c;
        m[12..16].copy_from_slice(&123u32.to_le_bytes());
        assert_eq!(uvc_pts(&m), Ok(Some(123)));
        m[11] |= 0x40;
        assert!(uvc_pts(&m).is_err());
    }
}
