//! Bounded display DTOs. These never create a metric Cloud or editable document.
use scancraft_geometry::work::WorkControl;
use scancraft_workflow::{
    candidate_bundle::FrameView,
    laser_stream::{CameraSummary, ProgressSnapshot, bundle::PreviewInput},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Native,
    RecordingReplay,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub origin: Origin,
    pub indices: [u64; 2],
    pub host_timestamp_us: [u64; 2],
    pub raw_pts: u32,
    pub frames: [FrameView; 2],
    #[serde(deserialize_with = "read_positions")]
    pub positions: Vec<[f64; 3]>,
    pub queries: usize,
    pub query_failures: usize,
    pub without_unique: usize,
    pub cameras: Option<[CameraSummary; 2]>,
    pub processing_error: Option<String>,
    pub acquisition: ProgressSnapshot,
    pub helper_omissions: u64,
}
impl Snapshot {
    pub fn prepare(
        input: PreviewInput<'_>,
        origin: Origin,
        work: &WorkControl,
    ) -> Result<Self, String> {
        let frames = [
            FrameView::from_frame(&input.profiles[0], &input.pair.samples[0].frame, work)
                .map_err(|e| e.to_string())?,
            FrameView::from_frame(&input.profiles[1], &input.pair.samples[1].frame, work)
                .map_err(|e| e.to_string())?,
        ];
        let mut s = Self {
            schema_version: 1,
            origin,
            indices: input.pair.samples.each_ref().map(|v| v.index),
            host_timestamp_us: input
                .pair
                .samples
                .each_ref()
                .map(|v| v.frame.host_timestamp_us),
            raw_pts: input.pair.raw_pts,
            frames,
            positions: Vec::new(),
            queries: 0,
            query_failures: 0,
            without_unique: 0,
            cameras: None,
            processing_error: None,
            acquisition: input.progress,
            helper_omissions: 0,
        };
        match input.outcome {
            Ok(r) => {
                s.positions = r
                    .candidates
                    .iter()
                    .map(|c| c.position_in_supplied_geometry_units)
                    .collect();
                s.queries = r.queries;
                s.query_failures = r.query_failures;
                s.without_unique = r.queries_without_unique_match;
                s.cameras = Some(r.cameras);
            }
            Err(e) => s.processing_error = Some(e.chars().take(1024).collect()),
        };
        s.validate()?;
        Ok(s)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.positions.len() > 4096
            || self.positions.iter().flatten().any(|v| !v.is_finite())
            || self.queries > 4096
            || self.query_failures > self.queries
            || self.without_unique > self.queries
            || self.positions.len() + self.query_failures + self.without_unique != self.queries
            || self
                .processing_error
                .as_ref()
                .is_some_and(|s| s.len() > 8192)
            || self.frames[0].interface == self.frames[1].interface
        {
            return Err("invalid laser diagnostic preview".into());
        }
        for (i, f) in self.frames.iter().enumerate() {
            f.validate_display()?;
            if self.acquisition.journaled_frames[i] <= self.indices[i]
                || self.acquisition.processing_queue_drops[i] > self.acquisition.journaled_frames[i]
            {
                return Err("invalid laser preview acquisition counters".into());
            }
        }
        if let Some(cameras) = &self.cameras {
            for (i, c) in cameras.iter().enumerate() {
                if c.interface != self.frames[i].interface
                    || c.observations > 100000
                    || c.supported_traces > c.observations
                    || c.supported_observations > c.observations
                    || c.continuity_ambiguous_observations > c.observations
                    || c.continuity_comparisons > 20000000
                    || [
                        c.rejected_border_runs,
                        c.rejected_width_runs,
                        c.rejected_peak_runs,
                        c.rejected_saturation_runs,
                    ]
                    .iter()
                    .any(|&n| n > scancraft_image::MAX_PIXELS)
                {
                    return Err("invalid detector preview counters".into());
                }
            }
        }
        if self.processing_error.is_some() != self.cameras.is_none() {
            return Err("inconsistent laser preview outcome".into());
        }
        Ok(())
    }
}
fn read_positions<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<[f64; 3]>, D::Error> {
    struct Positions;
    impl<'de> serde::de::Visitor<'de> for Positions {
        type Value = Vec<[f64; 3]>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "at most 4096 positions")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut a: A,
        ) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(v) = a.next_element()? {
                if out.len() == 4096 {
                    return Err(serde::de::Error::custom("laser preview point cap exceeded"));
                }
                out.push(v);
            }
            Ok(out)
        }
    }
    d.deserialize_seq(Positions)
}

#[cfg(test)]
pub(crate) mod tests;
