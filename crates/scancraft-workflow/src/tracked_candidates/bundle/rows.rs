use super::{Error, Request};
use crate::bounded_json::Items;
use scancraft_geometry::work::WorkControl;
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::PathBuf,
};
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FrameStatus {
    Integrated,
    TrackingOnly,
    TrackingRejected,
    InsufficientTrackingObservations,
}
impl FrameStatus {
    pub(super) fn counter(self) -> usize {
        match self {
            Self::Integrated => 0,
            Self::TrackingOnly => 1,
            Self::TrackingRejected => 2,
            Self::InsufficientTrackingObservations => 3,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Integrated => "Integrated",
            Self::TrackingOnly => "Tracking only",
            Self::TrackingRejected => "Tracking rejected",
            Self::InsufficientTrackingObservations => "Tracking unavailable",
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pose {
    rotation: [[f64; 3]; 3],
    translation_model_units: [f64; 3],
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fusion {
    input_points: usize,
    total_voxels: usize,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameOutcome {
    pub frame_id: u64,
    pub surface_points: usize,
    pub tracking_points: usize,
    pub status: FrameStatus,
    integrated: bool,
    accepted: Option<bool>,
    pub reason: Option<String>,
    pose: Option<Pose>,
    pub rms_model_units: Option<f64>,
    source_overlap: Option<f64>,
    target_overlap: Option<f64>,
    fusion: Option<Fusion>,
    last_integrated: Option<u64>,
}
impl FrameOutcome {
    pub(super) fn validate(
        &self,
        id: u64,
        request: &Request,
        last: &mut Option<u64>,
        voxels: &mut usize,
    ) -> Result<(), Error> {
        if self.frame_id != id
            || self.surface_points > 4096
            || self.tracking_points > 512
            || self
                .rms_model_units
                .is_some_and(|v| !v.is_finite() || v < 0.)
            || [self.source_overlap, self.target_overlap]
                .into_iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0. ..=1.).contains(&v))
            || self.reason.as_ref().is_some_and(|v| v.len() > 4096)
        {
            return Err("invalid fused frame diagnostics".into());
        }
        let accepted = matches!(
            self.status,
            FrameStatus::Integrated | FrameStatus::TrackingOnly
        );
        let unavailable = self.status == FrameStatus::InsufficientTrackingObservations;
        if self.integrated != (self.status == FrameStatus::Integrated)
            || self.accepted != if unavailable { None } else { Some(accepted) }
            || self.pose.is_some() != accepted
            || self.fusion.is_some() != self.integrated
            || (unavailable
                && (self.tracking_points >= 3
                    || self.rms_model_units.is_some()
                    || self.source_overlap.is_some()
                    || self.target_overlap.is_some()
                    || self.reason.is_some()))
            || (!unavailable
                && (self.tracking_points < 3
                    || self.reason.is_none()
                    || self.source_overlap.is_none()
                    || self.target_overlap.is_none()))
            || (accepted
                && (self.rms_model_units.is_none()
                    || self
                        .rms_model_units
                        .is_some_and(|v| v > request.tracking.max_rms_model_units)))
            || (self.status == FrameStatus::TrackingOnly && self.surface_points != 0)
        {
            return Err("inconsistent fused frame status".into());
        }
        if let Some(p) = &self.pose {
            scancraft_calibration::stereo::RigidPose {
                rotation: p.rotation,
                translation_mm: p.translation_model_units,
            }
            .validate()?;
        }
        if let Some(f) = &self.fusion {
            if f.input_points == 0
                || f.input_points != self.surface_points
                || f.total_voxels < *voxels
                || f.total_voxels == 0
                || f.total_voxels > request.maximum_voxels
            {
                return Err("inconsistent fused voxel counts".into());
            }
            *last = Some(id);
            *voxels = f.total_voxels;
        }
        if self.last_integrated != *last {
            return Err("inconsistent last integrated frame".into());
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    pub frame_id: u64,
    pub bundle: PathBuf,
    pub manifest: serde_json::Value,
    pub validation: serde_json::Value,
    pub first_observations: Items<usize, 4096>,
    pub frames: [SourceFrame; 2],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceFrame {
    profile: scancraft_core::StreamProfile,
    sequence: u32,
    host_timestamp_us: u64,
    sensor_pts: u32,
    payload_crc32: u32,
    metadata_crc32: u32,
}
impl SourceFrame {
    pub fn check(
        &self,
        frame: &scancraft_recording::selection::SelectedFrame,
    ) -> Result<(), Error> {
        if self.profile != frame.profile
            || self.sequence != frame.frame.sequence
            || self.host_timestamp_us != frame.frame.host_timestamp_us
            || Some(self.sensor_pts) != scancraft_core::uvc_pts(&frame.frame.metadata)?
            || self.payload_crc32 != crc32fast::hash(&frame.frame.data)
            || self.metadata_crc32 != crc32fast::hash(&frame.frame.metadata)
        {
            return Err("fused source frame binding mismatch".into());
        }
        Ok(())
    }
}
/// Read one bounded NDJSON object at a time; cap before allocating another line.
pub(super) fn read<T: DeserializeOwned>(
    file: &mut File,
    maximum: usize,
    line_limit: usize,
    work: &WorkControl,
    mut accept: impl FnMut(T) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut input = BufReader::new(file);
    let mut count = 0;
    loop {
        work.check()?;
        let mut bytes = Vec::new();
        let n = Read::by_ref(&mut input)
            .take(line_limit as u64 + 1)
            .read_until(b'\n', &mut bytes)?;
        if n == 0 {
            return Ok(());
        }
        if n > line_limit || count == maximum || bytes.last() != Some(&b'\n') {
            return Err("fused diagnostic row budget or termination failure".into());
        }
        accept(serde_json::from_slice(&bytes)?)?;
        count += 1;
    }
}
