//! Retained raw-frame summaries and bounded display copies. Never reconstruction input.
use crate::Error;
use scancraft_geometry::work::WorkControl;
use scancraft_image::{
    GrayImage,
    diagnostics::{self, GrayStatistics},
};
use scancraft_recording::selection::SelectedFrame;

#[derive(Debug)]
pub struct FrameView {
    pub interface: u8,
    pub source_size: [u32; 2],
    /// Exact statistics over every original pixel, before thumbnail subsampling.
    pub statistics: GrayStatistics,
    pub size: [usize; 2],
    /// Nearest samples from the original frame, with no reflection or tone mapping.
    pub pixels: Vec<u8>,
}

pub(super) fn prepare(frame: &SelectedFrame, work: &WorkControl) -> Result<FrameView, Error> {
    work.check()?;
    let image = GrayImage::from_frame(&frame.profile, &frame.frame)?;
    let statistics = diagnostics::statistics(image);
    work.check()?;
    let [width, height] = image.size().map(|n| n as usize);
    let stride = width.div_ceil(320).max(height.div_ceil(240)).max(1);
    let size = [width.div_ceil(stride), height.div_ceil(stride)];
    let mut pixels = Vec::with_capacity(size[0] * size[1]);
    for y in 0..size[1] {
        work.check()?;
        for x in 0..size[0] {
            pixels.push(frame.frame.data[y * stride * width + x * stride]);
        }
    }
    Ok(FrameView {
        interface: frame.profile.interface,
        source_size: image.size(),
        statistics,
        size,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_core::{Frame, StreamProfile};
    #[test]
    fn thumbnail_caps_and_exact_full_frame_statistics_preserve_source_and_orientation() {
        let width = 641;
        let height = 481;
        let data = (0..width * height)
            .map(|n| (n % 256) as u8)
            .collect::<Vec<_>>();
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&100u32.to_le_bytes());
        let frame = SelectedFrame {
            profile: StreamProfile {
                interface: 2,
                width,
                height,
                fourcc: *b"GREY",
                fps: 30,
            },
            frame: Frame {
                sequence: 0,
                host_timestamp_us: 1000,
                flags: 0,
                metadata,
                data: data.clone(),
            },
            context: None,
            frame_index: 0,
            total_frames: 1,
        };
        let view = prepare(&frame, &WorkControl::default()).unwrap();
        assert_eq!(view.interface, 2);
        assert_eq!(view.source_size, [641, 481]);
        assert_eq!(view.size, [214, 161]);
        assert_eq!(view.pixels.len(), 214 * 161);
        for y in 0..view.size[1] {
            for x in 0..view.size[0] {
                assert_eq!(view.pixels[y * view.size[0] + x], data[y * 3 * 641 + x * 3]);
            }
        }
        assert_eq!(view.statistics.pixels, data.len());
        assert_eq!(
            view.statistics.white_clipped,
            data.iter().filter(|&&n| n == 255).count()
        );
        assert_eq!(frame.frame.data, data);
        let work = WorkControl::default();
        work.cancel();
        assert!(prepare(&frame, &work).is_err());
    }
}
