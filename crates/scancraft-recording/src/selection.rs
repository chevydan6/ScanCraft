//! Retain a chosen frame only after validating the complete recording/footer.
use crate::{CaptureContext, Reader};
use scancraft_core::{Frame, StreamProfile};
use std::io::{self, Read};
#[derive(Debug, Clone)]
pub struct SelectedFrame {
    pub profile: StreamProfile,
    pub context: Option<CaptureContext>,
    pub frame_index: u64,
    pub total_frames: u64,
    pub frame: Frame,
}
/// Selection never skips CRC, frame-order or footer checks after the requested
/// frame. At most the selected frame and current decoded raw frame are retained.
pub fn read_selected<R: Read>(
    mut reader: Reader<R>,
    frame_index: u64,
) -> io::Result<SelectedFrame> {
    let mut selected = None;
    let mut total = 0u64;
    while let Some(frame) = reader.read_frame()? {
        if total == frame_index {
            selected = Some(frame);
        }
        total = total
            .checked_add(1)
            .ok_or_else(|| io::Error::other("recording frame count overflow"))?;
    }
    Ok(SelectedFrame {
        profile: reader.profile,
        context: reader.context,
        frame_index,
        total_frames: total,
        frame: selected.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "requested frame index is outside recording",
            )
        })?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Writer;
    fn recording() -> Vec<u8> {
        let profile = StreamProfile {
            interface: 0,
            width: 2,
            height: 2,
            fourcc: *b"GREY",
            fps: 50,
        };
        let mut writer = Writer::new(Vec::new(), profile).unwrap();
        for i in 0..3u32 {
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&(100 + i * 20).to_le_bytes());
            writer
                .push(&Frame {
                    sequence: i,
                    host_timestamp_us: u64::from(i) * 20 + 1000,
                    flags: 0,
                    metadata,
                    data: vec![i as u8; 4],
                })
                .unwrap();
        }
        writer.finish().unwrap()
    }
    #[test]
    fn selected_frame_retains_raw_metadata_context_and_zero_based_index() {
        let bytes = recording();
        let selected = read_selected(Reader::new(bytes.as_slice()).unwrap(), 1).unwrap();
        assert_eq!(selected.frame_index, 1);
        assert_eq!(selected.total_frames, 3);
        assert_eq!(selected.frame.sequence, 1);
        assert_eq!(selected.frame.data, [1; 4]);
        assert_eq!(
            scancraft_core::uvc_pts(&selected.frame.metadata).unwrap(),
            Some(120)
        );
        assert_eq!(
            selected.context.unwrap().calibration_status,
            "not_loaded_or_verified"
        );
        assert!(read_selected(Reader::new(bytes.as_slice()).unwrap(), 3).is_err());
    }
    #[test]
    fn selected_early_frame_does_not_hide_later_corruption_or_missing_footer() {
        let mut bytes = recording();
        let len = bytes.len();
        assert!(read_selected(Reader::new(&bytes[..len - 1]).unwrap(), 0).is_err());
        bytes[len - 13] ^= 1;
        assert!(read_selected(Reader::new(bytes.as_slice()).unwrap(), 0).is_err());
    }
}
