//! Bounded display subset of one complete stored pair; never a geometry import.
use crate::bounded_json::Items;
use scancraft_depth::laser_pair::{DomainSummary, MAX_QUERIES};
use serde::Deserialize;
use std::collections::HashSet;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reference {
    pub recording_role: String,
    pub frame_index: u64,
    pub sequence: u32,
    pub host_timestamp_us: u64,
    pub flags: u32,
    pub payload_crc32: u32,
    pub metadata_crc32: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub schema_version: u32,
    pub frames: [Reference; 2],
    pub raw_pts: u32,
    pub result: Result<StoredReport, String>,
    #[serde(default)]
    pub acquisition: Option<crate::laser_stream::ProgressSnapshot>,
}
#[derive(Deserialize)]
struct Camera {
    interface: u8,
    observations: usize,
}
#[derive(Deserialize)]
struct Candidate {
    first_observation: usize,
    position_in_supplied_geometry_units: [f64; 3],
}
#[derive(Deserialize)]
struct Query {
    first_observation: usize,
    outcome: Outcome,
}
#[derive(Deserialize)]
enum Outcome {
    Candidate {
        candidate_index: usize,
    },
    NoUnique {
        supported_combinations: usize,
        ambiguous_segments: usize,
    },
    Error(String),
}
#[derive(Deserialize)]
pub(super) struct StoredReport {
    cameras: [Camera; 2],
    queries: usize,
    query_failures: usize,
    queries_without_unique_match: usize,
    #[serde(default)]
    domain_rejections: Option<DomainSummary>,
    candidates: Items<Candidate, MAX_QUERIES>,
    outcomes: Items<Query, MAX_QUERIES>,
    raw_sensor_pts_equal: bool,
    synchronization_verified: bool,
    geometry_binding_verified: bool,
    units_verified: bool,
    projector_line_identity_verified: bool,
    physical_accuracy_qualified: bool,
}
pub(super) struct Display {
    pub positions: Vec<[f64; 3]>,
    pub first_observations: Vec<usize>,
    pub queries: usize,
    pub failures: usize,
    pub without_unique: usize,
    pub error: Option<String>,
    pub domain_rejections: Option<DomainSummary>,
}
impl Record {
    pub(super) fn display(
        &self,
        interfaces: [u8; 2],
        retain_valid_domain_segments: bool,
    ) -> Result<Display, String> {
        if self.schema_version != 1 {
            return Err("unsupported stored pair version".into());
        }
        let report = match &self.result {
            Ok(r) => r,
            Err(error) if error.len() <= 8192 => {
                return Ok(Display {
                    positions: Vec::new(),
                    first_observations: Vec::new(),
                    queries: 0,
                    failures: 0,
                    without_unique: 0,
                    error: Some(error.clone()),
                    domain_rejections: None,
                });
            }
            Err(_) => return Err("stored pair error exceeds display bound".into()),
        };
        if report.queries > MAX_QUERIES
            || report.outcomes.0.len() != report.queries
            || report
                .cameras
                .iter()
                .enumerate()
                .any(|(i, c)| c.interface != interfaces[i] || c.observations > 100000)
            || !report.raw_sensor_pts_equal
            || [
                report.synchronization_verified,
                report.geometry_binding_verified,
                report.units_verified,
                report.projector_line_identity_verified,
                report.physical_accuracy_qualified,
            ]
            .into_iter()
            .any(|v| v)
        {
            return Err("invalid or falsely qualified stored pair report".into());
        }
        if report.domain_rejections.is_some() != retain_valid_domain_segments
            || report.domain_rejections.is_some_and(|summary| {
                summary.rejected_trace_vertices > 65536
                    || summary.rejected_trace_segments > 65536
                    || summary.rejected_sheet_projections > report.queries * 256
            })
        {
            return Err("stored pair domain policy and exclusions disagree".into());
        }
        let mut used = HashSet::new();
        let mut candidate_indices = HashSet::new();
        let mut failures = 0;
        let mut without_unique = 0;
        for q in &report.outcomes.0 {
            if q.first_observation >= report.cameras[0].observations
                || !used.insert(q.first_observation)
            {
                return Err("invalid stored query reference".into());
            }
            match &q.outcome {
                Outcome::Candidate { candidate_index } => {
                    let c = report
                        .candidates
                        .0
                        .get(*candidate_index)
                        .ok_or("invalid stored candidate index")?;
                    if !candidate_indices.insert(*candidate_index)
                        || c.first_observation != q.first_observation
                    {
                        return Err("stored query/candidate references disagree".into());
                    }
                }
                Outcome::NoUnique {
                    supported_combinations,
                    ambiguous_segments,
                } => {
                    let _ = (supported_combinations, ambiguous_segments);
                    without_unique += 1;
                }
                Outcome::Error(error) => {
                    if error.len() > 8192 {
                        return Err("stored query error exceeds bound".into());
                    }
                    failures += 1;
                }
            }
        }
        if candidate_indices.len() != report.candidates.0.len()
            || failures != report.query_failures
            || without_unique != report.queries_without_unique_match
        {
            return Err("stored pair counters disagree".into());
        }
        let mut positions = Vec::with_capacity(report.candidates.0.len());
        let mut first_observations = Vec::with_capacity(report.candidates.0.len());
        for c in &report.candidates.0 {
            if !c
                .position_in_supplied_geometry_units
                .iter()
                .all(|v| v.is_finite())
            {
                return Err("non-finite stored candidate".into());
            }
            positions.push(c.position_in_supplied_geometry_units);
            first_observations.push(c.first_observation);
        }
        Ok(Display {
            positions,
            first_observations,
            queries: report.queries,
            failures,
            without_unique,
            error: None,
            domain_rejections: report.domain_rejections,
        })
    }
}
