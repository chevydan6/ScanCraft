//! Read-only, identity-guarded offline recording access and bounded decoding.
use image::{ImageDecoder, ImageReader, RgbImage};
use scancraft_geometry::work::{Stage, WorkControl};
use scancraft_recording::indexed::{IndexedRecording, MAX_INDEX_FRAMES};
use std::{
    fs::{self, Metadata, OpenOptions},
    io::{self, BufReader, Cursor},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
type Stamp = (u64, u64, u64, i64, i64, i64, i64);
fn stamp(m: &Metadata) -> Stamp {
    (
        m.dev(),
        m.ino(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    )
}
fn error(e: impl Into<String>) -> io::Error {
    io::Error::other(e.into())
}
pub struct Playback {
    recording: IndexedRecording<BufReader<std::fs::File>>,
    path: PathBuf,
    initial: Stamp,
}
pub struct View {
    pub image: RgbImage,
    pub index: usize,
    pub total: usize,
    pub sequence: u32,
    pub host_timestamp_us: u64,
    pub sensor_pts: Option<u32>,
    pub description: String,
}
impl Playback {
    pub fn open(path: &Path, work: &WorkControl) -> io::Result<Self> {
        work.check().map_err(error)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > (1_u64 << 40) {
            return Err(error(
                "playback requires a regular file no larger than 1 TiB",
            ));
        }
        let initial = stamp(&metadata);
        let recording =
            IndexedRecording::build(BufReader::new(file), MAX_INDEX_FRAMES, &|count| {
                // The total frame count is unknown until the footer is validated.
                work.update(Stage::Indexing, count as usize, 0)
                    .map_err(error)
            })?;
        let playback = Self {
            recording,
            path: path.into(),
            initial,
        };
        playback.check_source()?;
        work.check().map_err(error)?;
        if playback.recording.is_empty() {
            return Err(error("recording contains no frames"));
        }
        Ok(playback)
    }
    /// Profile validated while indexing; does not attest the physical sensor.
    pub fn profile(&self) -> &scancraft_core::StreamProfile {
        self.recording.profile()
    }
    /// Cached observations from full indexing; source guards remain mandatory.
    pub fn timings(
        &self,
    ) -> impl ExactSizeIterator<Item = scancraft_recording::indexed::FrameTiming> + '_ {
        self.recording.timings()
    }
    pub fn len(&self) -> usize {
        self.recording.len()
    }
    pub fn is_empty(&self) -> bool {
        self.recording.is_empty()
    }
    pub fn check_source(&self) -> io::Result<()> {
        let current = fs::symlink_metadata(&self.path)?;
        // Check both the open inode and the current path, including replacement.
        if !current.is_file()
            || stamp(&current) != self.initial
            || stamp(&self.recording.source().get_ref().metadata()?) != self.initial
        {
            return Err(error(
                "recording source changed; reopen and validate it again",
            ));
        }
        Ok(())
    }
    pub fn version(&self) -> u8 {
        self.recording.version()
    }
    /// Select original payload/metadata, preserving capture context. No display conversion.
    pub fn select_raw(
        &mut self,
        index: usize,
        work: &WorkControl,
    ) -> io::Result<scancraft_recording::selection::SelectedFrame> {
        work.check().map_err(error)?;
        self.check_source()?;
        let frame = self.recording.read_frame(index)?;
        work.check().map_err(error)?;
        self.check_source()?;
        Ok(scancraft_recording::selection::SelectedFrame {
            profile: self.recording.profile().clone(),
            context: self.recording.context().cloned(),
            frame_index: index as u64,
            total_frames: self.len() as u64,
            frame,
        })
    }
    pub fn read(&mut self, index: usize, work: &WorkControl) -> io::Result<View> {
        work.check().map_err(error)?;
        self.check_source()?;
        let frame = self.recording.read_frame(index)?;
        work.check().map_err(error)?;
        let p = self.recording.profile();
        let image = match &p.fourcc {
            b"GREY" => image::DynamicImage::ImageLuma8(
                image::GrayImage::from_raw(p.width, p.height, frame.data)
                    .ok_or_else(|| error("invalid GREY image length"))?,
            )
            .to_rgb8(),
            b"Y14 " => {
                // Display only: full 16-bit container range, no physical scaling.
                let gray: Vec<u8> = frame
                    .data
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| (u16::from_le_bytes([b[0], b[1]]) >> 8) as u8)
                    .collect();
                image::DynamicImage::ImageLuma8(
                    image::GrayImage::from_raw(p.width, p.height, gray)
                        .ok_or_else(|| error("invalid Y14 image length"))?,
                )
                .to_rgb8()
            }
            b"MJPG" => {
                let mut limits = image::Limits::default();
                limits.max_image_width = Some(p.width);
                limits.max_image_height = Some(p.height);
                limits.max_alloc = Some(128 * 1024 * 1024);
                let mut reader =
                    ImageReader::with_format(Cursor::new(&frame.data), image::ImageFormat::Jpeg);
                reader.limits(limits);
                let decoder = reader.into_decoder().map_err(|e| error(e.to_string()))?;
                if decoder.dimensions() != (p.width, p.height) {
                    return Err(error("JPEG dimensions disagree with recording profile"));
                }
                image::DynamicImage::from_decoder(decoder)
                    .map_err(|e| error(e.to_string()))?
                    .to_rgb8()
            }
            _ => return Err(error("unsupported playback pixel format")),
        };
        work.check().map_err(error)?;
        self.check_source()?;
        let context = self.recording.context();
        let description = format!(
            "{} × {} · {} · v{} · {} · calibration: {}{}",
            p.width,
            p.height,
            String::from_utf8_lossy(&p.fourcc),
            self.recording.version(),
            if self.recording.version() >= 2 {
                "header CRC verified"
            } else {
                "legacy header has no CRC"
            },
            context
                .map(|c| c.calibration_status.as_str())
                .unwrap_or("unknown"),
            if &p.fourcc == b"Y14 " {
                " · display uses 16-bit container range"
            } else {
                ""
            }
        );
        Ok(View {
            image,
            index,
            total: self.len(),
            sequence: frame.sequence,
            host_timestamp_us: frame.host_timestamp_us,
            sensor_pts: scancraft_core::uvc_pts(&frame.metadata).map_err(error)?,
            description,
        })
    }
}
