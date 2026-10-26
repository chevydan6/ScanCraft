//! Independent indicators for a rejected buffer; none relax frame acceptance.
use scancraft_core::{Frame, StreamProfile, uvc_pts};
use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
pub struct RejectedBufferInfo {
    pub sequence: u32,
    pub host_timestamp_us: u64,
    pub video_bytes: usize,
    pub metadata_bytes: usize,
    pub video_flags: u32,
    pub metadata_flags: u32,
    pub raw_sensor_pts: Option<u32>,
    /// First failure retained for compatibility, not necessarily the root cause.
    pub reason: String,
    pub video_error_flag: bool,
    pub metadata_error_flag: bool,
    pub expected_uncompressed_video_bytes: Option<usize>,
    pub uncompressed_video_length_matches: Option<bool>,
    /// First raw UVC header flag byte; not a validated exposure/pair marker.
    pub raw_first_uvc_flags: Option<u8>,
    /// Structural metadata parsing independent of the V4L2 flags above.
    pub metadata_pts_parse_error: Option<String>,
}
impl RejectedBufferInfo {
    pub(crate) fn new(
        profile: &StreamProfile,
        frame: &Frame,
        metadata_flags: u32,
        reason: &str,
    ) -> Self {
        let pixels = (profile.width as usize).checked_mul(profile.height as usize);
        let expected = match &profile.fourcc {
            b"GREY" => pixels,
            b"Y14 " => pixels.and_then(|x| x.checked_mul(2)),
            _ => None,
        };
        let raw_first_uvc_flags =
            (frame.metadata.len() >= 12 && frame.metadata[10] >= 2).then(|| frame.metadata[11]);
        let raw_sensor_pts =
            if frame.metadata.len() >= 16 && frame.metadata[10] >= 6 && frame.metadata[11] & 4 != 0
            {
                Some(u32::from_le_bytes(
                    frame.metadata[12..16].try_into().unwrap(),
                ))
            } else {
                None
            };
        let metadata_pts_parse_error = match uvc_pts(&frame.metadata) {
            Ok(Some(_)) => None,
            Ok(None) => Some("missing sensor PTS".into()),
            Err(e) => Some(e.into()),
        };
        Self {
            sequence: frame.sequence,
            host_timestamp_us: frame.host_timestamp_us,
            video_bytes: frame.data.len(),
            metadata_bytes: frame.metadata.len(),
            video_flags: frame.flags,
            metadata_flags,
            raw_sensor_pts,
            reason: reason.into(),
            video_error_flag: frame.flags & 0x40 != 0,
            metadata_error_flag: metadata_flags & 0x40 != 0,
            expected_uncompressed_video_bytes: expected,
            uncompressed_video_length_matches: expected.map(|n| n == frame.data.len()),
            raw_first_uvc_flags,
            metadata_pts_parse_error,
        }
    }
}
/// Keep the legacy first sample and an independent post-first-valid sample.
/// Metadata/strings only: image payloads are never retained here.
pub(crate) fn retain(
    first: &mut Vec<RejectedBufferInfo>,
    ongoing: &mut Vec<RejectedBufferInfo>,
    after_first_valid: bool,
    make: impl FnOnce() -> RejectedBufferInfo,
) {
    let keep_first = first.len() < 32;
    let keep_ongoing = after_first_valid && ongoing.len() < 32;
    if keep_first || keep_ongoing {
        let info = make();
        if keep_first {
            first.push(info.clone());
        }
        if keep_ongoing {
            ongoing.push(info);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (StreamProfile, Frame) {
        let profile = StreamProfile {
            interface: 0,
            width: 2,
            height: 2,
            fourcc: *b"GREY",
            fps: 30,
        };
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&123_u32.to_le_bytes());
        (
            profile,
            Frame {
                sequence: 2,
                host_timestamp_us: 100,
                flags: 0,
                metadata,
                data: vec![0; 4],
            },
        )
    }
    #[test]
    fn startup_saturation_cannot_hide_later_rejections_and_both_samples_stay_bounded() {
        let (profile, mut frame) = fixture();
        let mut first = Vec::new();
        let mut ongoing = Vec::new();
        let mut built = 0;
        for i in 0..200 {
            frame.sequence = i;
            retain(&mut first, &mut ongoing, false, || {
                built += 1;
                RejectedBufferInfo::new(&profile, &frame, 0x40, "startup")
            });
        }
        assert_eq!(first.len(), 32);
        assert!(ongoing.is_empty());
        assert_eq!(built, 32);
        for i in 200..400 {
            frame.sequence = i;
            retain(&mut first, &mut ongoing, true, || {
                built += 1;
                RejectedBufferInfo::new(&profile, &frame, 0x40, "ongoing")
            });
        }
        assert_eq!(first.len(), 32);
        assert_eq!(ongoing.len(), 32);
        assert_eq!(built, 64);
        assert_eq!(first[0].sequence, 0);
        assert_eq!(ongoing[0].sequence, 200);
        assert_eq!(ongoing[31].sequence, 231);
    }
    #[test]
    fn flag_propagation_and_short_image_do_not_imply_bad_pts_structure() {
        let (profile, mut frame) = fixture();
        frame.flags = 0x40;
        frame.data.pop();
        let info = RejectedBufferInfo::new(&profile, &frame, 0x40, "metadata buffer error flag");
        assert!(info.video_error_flag && info.metadata_error_flag);
        assert_eq!(info.expected_uncompressed_video_bytes, Some(4));
        assert_eq!(info.uncompressed_video_length_matches, Some(false));
        assert_eq!(info.raw_first_uvc_flags, Some(4));
        assert_eq!(info.raw_sensor_pts, Some(123));
        assert_eq!(info.metadata_pts_parse_error, None);
        assert!(scancraft_core::validate_frame(&profile, &frame).is_err());
        let output = serde_json::to_value(&info).unwrap();
        assert_eq!(output["reason"], "metadata buffer error flag");
        assert_eq!(output["video_bytes"], 3);
    }
    #[test]
    fn device_uvc_error_is_distinct_from_v4l2_flags() {
        let (profile, mut frame) = fixture();
        frame.metadata[11] |= 0x40;
        let info = RejectedBufferInfo::new(&profile, &frame, 0, "PTS failure");
        assert!(!info.video_error_flag && !info.metadata_error_flag);
        assert_eq!(info.uncompressed_video_length_matches, Some(true));
        assert_eq!(
            info.metadata_pts_parse_error.as_deref(),
            Some("device reported a UVC frame error")
        );
        assert!(scancraft_core::validate_frame(&profile, &frame).is_err());
    }
    #[test]
    fn missing_truncated_and_inconsistent_metadata_are_separate_indicators() {
        let (profile, mut frame) = fixture();
        frame.metadata.clear();
        let info = RejectedBufferInfo::new(&profile, &frame, 0, "missing");
        assert_eq!(info.raw_first_uvc_flags, None);
        assert_eq!(
            info.metadata_pts_parse_error.as_deref(),
            Some("missing sensor PTS")
        );
        frame.metadata = vec![0; 11];
        assert_eq!(
            RejectedBufferInfo::new(&profile, &frame, 0, "bad")
                .metadata_pts_parse_error
                .as_deref(),
            Some("truncated UVC metadata record")
        );
        let (_, mut frame) = fixture();
        let mut second = frame.metadata.clone();
        second[12..16].copy_from_slice(&124_u32.to_le_bytes());
        frame.metadata.extend(second);
        assert_eq!(
            RejectedBufferInfo::new(&profile, &frame, 0, "bad")
                .metadata_pts_parse_error
                .as_deref(),
            Some("inconsistent per-frame PTS")
        );
    }
    #[test]
    fn expected_lengths_respect_format_and_compressed_unknown_size() {
        let (mut profile, frame) = fixture();
        profile.fourcc = *b"Y14 ";
        let info = RejectedBufferInfo::new(&profile, &frame, 0, "length");
        assert_eq!(info.expected_uncompressed_video_bytes, Some(8));
        assert_eq!(info.uncompressed_video_length_matches, Some(false));
        profile.fourcc = *b"MJPG";
        let info = RejectedBufferInfo::new(&profile, &frame, 0, "jpeg");
        assert_eq!(info.expected_uncompressed_video_bytes, None);
        assert_eq!(info.uncompressed_video_length_matches, None);
    }
}
