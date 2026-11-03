//! Retained raw-frame summaries and bounded display copies. Never reconstruction input.
use crate::Error;
use scancraft_geometry::work::WorkControl;
use scancraft_image::{
    GrayImage,
    diagnostics::{self, GrayStatistics},
};
use scancraft_recording::selection::SelectedFrame;

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameView {
    pub interface: u8,
    pub source_size: [u32; 2],
    /// Exact statistics over every original pixel, before thumbnail subsampling.
    pub statistics: GrayStatistics,
    pub size: [usize; 2],
    /// Nearest samples from the original frame, with no reflection or tone mapping.
    #[serde(deserialize_with = "read_pixels")]
    pub pixels: Vec<u8>,
}

pub(super) fn prepare(frame: &SelectedFrame, work: &WorkControl) -> Result<FrameView, Error> {
    FrameView::from_frame(&frame.profile, &frame.frame, work)
}
impl FrameView {
    /// Display-only borrowed raw input; no recording identity is attested.
    pub fn from_frame(
        profile: &scancraft_core::StreamProfile,
        frame: &scancraft_core::Frame,
        work: &WorkControl,
    ) -> Result<Self, Error> {
        work.check()?;
        let image = GrayImage::from_frame(profile, frame)?;
        let statistics = diagnostics::statistics(image);
        work.check()?;
        let [width, height] = image.size().map(|n| n as usize);
        let stride = width.div_ceil(320).max(height.div_ceil(240)).max(1);
        let size = [width.div_ceil(stride), height.div_ceil(stride)];
        let mut pixels = Vec::with_capacity(size[0] * size[1]);
        for y in 0..size[1] {
            work.check()?;
            for x in 0..size[0] {
                pixels.push(frame.data[y * stride * width + x * stride]);
            }
        }
        Ok(FrameView {
            interface: profile.interface,
            source_size: image.size(),
            statistics,
            size,
            pixels,
        })
    }

    pub fn validate_display(&self) -> Result<(), String> {
        let [w, h] = self.source_size.map(|v| v as usize);
        let pixels = w
            .checked_mul(h)
            .filter(|&n| n > 0 && n <= scancraft_image::MAX_PIXELS)
            .ok_or("invalid preview source dimensions")?;
        let stride = w.div_ceil(320).max(h.div_ceil(240)).max(1);
        let stats = &self.statistics;
        if ![0, 2].contains(&self.interface)
            || self.size != [w.div_ceil(stride), h.div_ceil(stride)]
            || self.pixels.len() != self.size[0] * self.size[1]
            || stats.pixels != pixels
            || stats.black_clipped > pixels
            || stats.white_clipped > pixels
            || stats.black_clipped + stats.white_clipped > pixels
            || !stats.mean.is_finite()
            || stats.mean < f64::from(stats.minimum)
            || stats.mean > f64::from(stats.maximum)
            || [
                stats.minimum,
                stats.p01,
                stats.median,
                stats.p99,
                stats.maximum,
            ]
            .windows(2)
            .any(|v| v[0] > v[1])
        {
            return Err("invalid display thumbnail or reported statistics".into());
        }
        Ok(())
    }
}
fn read_pixels<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    struct Pixels;
    impl<'de> serde::de::Visitor<'de> for Pixels {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "at most 76800 thumbnail pixels")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut a: A,
        ) -> Result<Self::Value, A::Error> {
            let mut pixels = Vec::new();
            while let Some(pixel) = a.next_element()? {
                if pixels.len() == 320 * 240 {
                    return Err(serde::de::Error::custom("thumbnail pixel budget exceeded"));
                }
                pixels.push(pixel);
            }
            Ok(pixels)
        }
    }
    d.deserialize_seq(Pixels)
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
