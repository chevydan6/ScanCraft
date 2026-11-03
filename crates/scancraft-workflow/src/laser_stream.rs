//! Bounded raw-journal-first laser processing, independent of device/UI ownership.
//! Equal raw PTS and inferred modular progression are not verified exposures.
mod association;
pub mod bundle;
pub mod pair_journal;
mod runner;
pub use association::{Association, AssociationCounts, Camera, Pair, Sample};
pub use runner::{Failure, Journal, ProgressSnapshot, SourceReport, StopToken, Summary};
use scancraft_calibration::stereo::StereoCalibration;
use scancraft_core::StreamProfile;
pub use scancraft_depth::laser_pair::CameraSummary;
use scancraft_depth::laser_pair::{self, Settings};

pub(crate) type Result<T> = std::result::Result<T, String>;
pub const PROCESSING_QUEUE_CAPACITY: usize = 4;
pub const PENDING_PER_CAMERA: usize = 2;
pub const MAX_FRAME_METADATA_BYTES: usize = 4096;
fn validate_sample(profile: &StreamProfile, frame: &scancraft_core::Frame) -> Result<()> {
    if frame.metadata.len() > MAX_FRAME_METADATA_BYTES {
        return Err("laser frame metadata exceeds 4096-byte bound".into());
    }
    scancraft_core::validate_frame(profile, frame).map_err(str::to_string)
}

/// Immutable processing configuration. This neither opens a scanner nor qualifies
/// supplied geometry, projector identities, units or synchronization.
pub struct Pipeline<'a> {
    processor: laser_pair::Processor<'a>,
    profiles: [StreamProfile; 2],
    maximum_host_delta_us: u64,
}
impl<'a> Pipeline<'a> {
    pub fn new(geometry: &'a StereoCalibration, settings: &'a Settings, fps: u32) -> Result<Self> {
        if fps == 0 {
            return Err("laser stream frame rate must be nonzero".into());
        }
        let processor = laser_pair::Processor::new(geometry, settings)?;
        let profiles = [&geometry.left, &geometry.right].map(|camera| StreamProfile {
            interface: 0,
            width: camera.width,
            height: camera.height,
            fourcc: *b"GREY",
            fps,
        });
        let mut pipeline = Self {
            processor,
            profiles,
            maximum_host_delta_us: settings.maximum_host_delta_us,
        };
        for (i, profile) in pipeline.profiles.iter_mut().enumerate() {
            profile.interface = settings.interfaces[i];
        }
        Ok(pipeline)
    }
    pub fn profiles(&self) -> &[StreamProfile; 2] {
        &self.profiles
    }
}

#[cfg(test)]
mod tests;
