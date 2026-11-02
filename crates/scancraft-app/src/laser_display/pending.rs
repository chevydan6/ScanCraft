//! Owned, bounded display work. No scanner, document or required journal ownership.
use super::{Origin, Snapshot};
use scancraft_core::StreamProfile;
use scancraft_geometry::work::WorkControl;
use scancraft_image::GrayImage;
use scancraft_workflow::{
    candidate_bundle::FrameView,
    laser_stream::{
        CameraSummary, MAX_FRAME_METADATA_BYTES, Pair, ProgressSnapshot, Sample,
        bundle::PreviewInput,
    },
};
struct Outcome {
    positions: Vec<[f64; 3]>,
    queries: usize,
    query_failures: usize,
    without_unique: usize,
    cameras: Option<[CameraSummary; 2]>,
    processing_error: Option<String>,
}
pub(crate) struct Pending {
    origin: Origin,
    pair: Pair,
    profiles: [StreamProfile; 2],
    outcome: Outcome,
    progress: ProgressSnapshot,
}
impl Pending {
    /// Only validated raw copies and compact result fields cross the mailbox.
    /// Full histograms/thumbnails/encoding happen after ownership transfer.
    /// At most two MAX_PIXELS payloads and two 4096-byte metadata arrays are copied.
    pub fn capture(
        input: PreviewInput<'_>,
        origin: Origin,
        work: &WorkControl,
    ) -> Result<Self, String> {
        work.check()?;
        for (profile, sample) in input.profiles.iter().zip(&input.pair.samples) {
            if sample.frame.metadata.len() > MAX_FRAME_METADATA_BYTES {
                return Err("laser preview metadata exceeds its byte budget".into());
            }
            GrayImage::from_frame(profile, &sample.frame)?;
        }
        let outcome = match input.outcome {
            Ok(r) => {
                if r.candidates.len() > 4096 || r.queries > 4096 {
                    return Err("laser preview result exceeds its point/query budget".into());
                }
                Outcome {
                    positions: r
                        .candidates
                        .iter()
                        .map(|c| c.position_in_supplied_geometry_units)
                        .collect(),
                    queries: r.queries,
                    query_failures: r.query_failures,
                    without_unique: r.queries_without_unique_match,
                    cameras: Some(r.cameras),
                    processing_error: None,
                }
            }
            Err(e) => Outcome {
                positions: Vec::new(),
                queries: 0,
                query_failures: 0,
                without_unique: 0,
                cameras: None,
                processing_error: Some(e.chars().take(1024).collect()),
            },
        };
        let copy = |side: usize| Sample {
            index: input.pair.samples[side].index,
            frame: input.pair.samples[side].frame.clone(),
        };
        let first = copy(0);
        work.check()?;
        let second = copy(1);
        work.check()?;
        Ok(Self {
            origin,
            pair: Pair {
                samples: [first, second],
                raw_pts: input.pair.raw_pts,
            },
            profiles: input.profiles.clone(),
            outcome,
            progress: input.progress,
        })
    }
    pub fn finish(self, work: &WorkControl) -> Result<Snapshot, String> {
        work.check()?;
        let frames = [
            FrameView::from_frame(&self.profiles[0], &self.pair.samples[0].frame, work)
                .map_err(|e| e.to_string())?,
            FrameView::from_frame(&self.profiles[1], &self.pair.samples[1].frame, work)
                .map_err(|e| e.to_string())?,
        ];
        let snapshot = Snapshot {
            schema_version: 1,
            origin: self.origin,
            indices: self.pair.samples.each_ref().map(|v| v.index),
            host_timestamp_us: self
                .pair
                .samples
                .each_ref()
                .map(|v| v.frame.host_timestamp_us),
            raw_pts: self.pair.raw_pts,
            frames,
            positions: self.outcome.positions,
            queries: self.outcome.queries,
            query_failures: self.outcome.query_failures,
            without_unique: self.outcome.without_unique,
            cameras: self.outcome.cameras,
            processing_error: self.outcome.processing_error,
            acquisition: self.progress,
            helper_omissions: 0,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
}
