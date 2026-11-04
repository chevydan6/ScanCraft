//! Borrowed raw stereo frames -> bounded laser candidates, without files or JSON.
//! Supplied geometry/units/exposure synchronization remain physically unqualified.
use scancraft_calibration::{
    laser_surface::QuadraticSheet,
    laser_trace::{self, MatchIndex, Matches, PreparedMatcher},
    orientation::reflect_pixel_x,
    stereo::{StereoCalibration, TriangulationLimits},
};
use scancraft_core::{Frame, StreamProfile};
use scancraft_geometry::work::{Stage, WorkControl};
use scancraft_image::{
    GrayImage,
    laser::{self, LineConfig, LineDetections},
    laser_continuity::{self, Traces},
};
use serde::{Deserialize, Serialize};
mod detection;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Detection {
    pub background: u8,
    pub minimum_peak: u8,
    pub maximum_width: u32,
    pub saturation: u8,
    pub maximum_observations: usize,
    pub opening_radius: Option<u32>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Continuity {
    pub maximum_row_gap: u32,
    pub maximum_horizontal_step: f64,
    pub maximum_slope: f64,
    pub minimum_span_rows: u32,
    pub minimum_observations: usize,
}
pub struct Observations {
    pub detections: LineDetections,
    pub traces: Traces,
    pub membership: Vec<(usize, usize)>,
}
pub fn detect(
    image: GrayImage<'_>,
    detection: &Detection,
    continuity: &Continuity,
    work: &WorkControl,
) -> Result<Observations, String> {
    let c = detection;
    let config = LineConfig {
        background: c.background,
        minimum_peak: c.minimum_peak,
        maximum_width: c.maximum_width,
        saturation: c.saturation,
        maximum_observations: c.maximum_observations,
    };
    let detections = match c.opening_radius {
        Some(radius) => laser::detect_rows_with_opening(image, config, radius, work)?,
        None => laser::detect_rows(image, config, work)?,
    };
    let c = continuity;
    let traces = laser_continuity::trace_rows(
        image.size(),
        &detections.observations,
        laser_continuity::Config {
            maximum_row_gap: c.maximum_row_gap,
            maximum_horizontal_step: c.maximum_horizontal_step,
            maximum_slope: c.maximum_slope,
            minimum_span_rows: c.minimum_span_rows,
            minimum_observations: c.minimum_observations,
            maximum_comparisons: 20_000_000,
        },
        work,
    )?;
    let mut membership = vec![(0, 0); detections.observations.len()];
    for (trace_id, trace) in traces.traces.iter().enumerate() {
        work.check()?;
        for (offset, &index) in trace.observation_indices.iter().enumerate() {
            membership[index] = (trace_id, offset);
        }
    }
    Ok(Observations {
        detections,
        traces,
        membership,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub interfaces: [u8; 2],
    pub reflect_x: [bool; 2],
    pub maximum_host_delta_us: u64,
    pub detection: Detection,
    pub continuity: Continuity,
    pub sheets: Vec<QuadraticSheet>,
    pub triangulation: TriangulationLimits,
    pub matching: laser_trace::Config,
    pub maximum_queries: usize,
    pub maximum_total_work: usize,
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        self.matching.validate()?;
        if !matches!(self.interfaces, [0, 2] | [2, 0])
            || self.maximum_host_delta_us > 1_000_000
            || !(1..=128).contains(&self.sheets.len())
            || !(1..=4096).contains(&self.maximum_queries)
            || !(1..=100_000_000).contains(&self.maximum_total_work)
            || self.sheets.iter().any(|s| {
                ![s.quadratic_y, s.linear_y, s.linear_z, s.offset]
                    .iter()
                    .all(|v| v.is_finite())
            })
        {
            return Err("invalid laser-pair processing settings".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
pub struct FrameInput<'a> {
    pub profile: &'a StreamProfile,
    pub frame: &'a Frame,
}

/// Borrowed only during Sink::begin; no recording history or identity is attested.
pub struct PairInfo<'a> {
    pub sides: &'a [Observations; 2],
    pub first_query_observation_indices: &'a [usize],
    pub second_matcher_trace_ids: &'a [usize],
    pub second_matcher_trace_pixels: &'a [Vec<[f64; 2]>],
    pub admission: Admission,
    pub domain_rejections: Option<&'a laser_trace::domain::TraceRejections>,
}
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Admission {
    pub coarse_total_work_bound: usize,
    pub conservative_total_work_bound: usize,
    pub uses_projected_roots: bool,
    pub projected_root_preparation_failures: usize,
}
pub struct Query {
    pub first_observation: usize,
    pub first_calibration_pixel: [f64; 2],
    pub result: Result<Matches, String>,
    pub rejected_projections: Option<Vec<laser_trace::domain::RejectedProjection>>,
}
/// A required journal may return an error; the processing call stops without
/// cancelling WorkControl or touching a device. Optional display consumers should
/// omit rejected previews themselves, rather than returning a journal failure.
pub trait Sink {
    fn begin(&mut self, _info: PairInfo<'_>) -> Result<(), String> {
        Ok(())
    }
    fn query(&mut self, _query: &Query) -> Result<(), String> {
        Ok(())
    }
}
impl Sink for () {}

#[derive(Debug, Serialize)]
pub struct Candidate {
    pub first_observation: usize,
    pub first_trace: usize,
    pub selection: MatchIndex,
    pub second_trace: usize,
    pub second_calibration_pixel: [f64; 2],
    pub second_raw_image_pixel: [f64; 2],
    pub sheet_index: usize,
    pub sheet_root_index: usize,
    pub position_in_supplied_geometry_units: [f64; 3],
}
#[derive(Debug, Serialize)]
pub enum Outcome {
    Candidate {
        candidate_index: usize,
    },
    NoUnique {
        supported_combinations: usize,
        ambiguous_segments: usize,
    },
    Error(String),
}
#[derive(Debug, Serialize)]
pub struct QueryOutcome {
    pub first_observation: usize,
    pub first_calibration_pixel: [f64; 2],
    pub outcome: Outcome,
}
/// Explicit exclusions; numerical uniqueness is conditional on the retained domain.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainSummary {
    pub rejected_trace_vertices: usize,
    pub rejected_trace_segments: usize,
    pub rejected_sheet_projections: usize,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub cameras: [CameraSummary; 2],
    pub queries: usize,
    pub query_failures: usize,
    pub queries_without_unique_match: usize,
    pub admission: Admission,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_rejections: Option<DomainSummary>,
    pub candidates: Vec<Candidate>,
    /// One outcome per query. Full crossings/supports are streamed to Sink;
    /// using () explicitly discards those details, not these errors/counters.
    pub outcomes: Vec<QueryOutcome>,
    pub raw_sensor_pts_equal: bool,
    pub synchronization_verified: bool,
    pub geometry_binding_verified: bool,
    pub units_verified: bool,
    pub projector_line_identity_verified: bool,
    pub physical_accuracy_qualified: bool,
}

/// Detector-run and continuity counters, not whole-image clipping or a quality pass.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraSummary {
    pub interface: u8,
    pub observations: usize,
    pub supported_traces: usize,
    pub supported_observations: usize,
    pub rejected_border_runs: usize,
    pub rejected_width_runs: usize,
    pub rejected_peak_runs: usize,
    pub rejected_saturation_runs: usize,
    pub continuity_ambiguous_observations: usize,
    pub continuity_comparisons: usize,
}
/// Reusable immutable numerical model/settings. No scanner, file, Cloud/Editor,
/// registration state, recording provenance or calibration activation is owned.
pub struct Processor<'a> {
    geometry: &'a StereoCalibration,
    settings: &'a Settings,
    matcher: PreparedMatcher<'a>,
    parallel_detection: bool,
    retain_valid_domain_segments: bool,
}
impl<'a> Processor<'a> {
    pub fn new(geometry: &'a StereoCalibration, settings: &'a Settings) -> Result<Self, String> {
        settings.validate()?;
        Ok(Self {
            geometry,
            settings,
            matcher: PreparedMatcher::new(geometry, settings.triangulation)?,
            retain_valid_domain_segments: false,
            parallel_detection: std::thread::available_parallelism().is_ok_and(|n| n.get() > 1),
        })
    }
    /// Opt in to explicitly journaled domain exclusions. Strict matching remains
    /// the default. Excluded observations never become inferred geometry.
    pub fn with_valid_domain_segments(mut self, enabled: bool) -> Self {
        self.retain_valid_domain_segments = enabled;
        self
    }
    /// Validate both raw GREY frames, PTS equality and host window; compute all
    /// supported observations with bounded output. Per-query numerical errors
    /// remain outcomes. Cancellation, admission or sink failure returns no report.
    /// Raw samples are borrowed unchanged. CRC/history checks belong to adapters.
    pub fn process(
        &self,
        frames: [FrameInput<'_>; 2],
        work: &WorkControl,
        sink: &mut impl Sink,
    ) -> Result<Report, String> {
        work.check()?;
        for (side, f) in frames.iter().enumerate() {
            let camera = if side == 0 {
                &self.geometry.left
            } else {
                &self.geometry.right
            };
            if f.profile.interface != self.settings.interfaces[side]
                || f.profile.fourcc != *b"GREY"
                || [f.profile.width, f.profile.height] != [camera.width, camera.height]
            {
                return Err("laser pair frame disagrees with configured camera profile".into());
            }
        }
        let images = [
            GrayImage::from_frame(frames[0].profile, frames[0].frame)?,
            GrayImage::from_frame(frames[1].profile, frames[1].frame)?,
        ];
        if scancraft_core::uvc_pts(&frames[0].frame.metadata)?
            != scancraft_core::uvc_pts(&frames[1].frame.metadata)?
            || frames[0]
                .frame
                .host_timestamp_us
                .abs_diff(frames[1].frame.host_timestamp_us)
                > self.settings.maximum_host_delta_us
        {
            return Err("selected frames fail raw-PTS/host-window pairing requirements".into());
        }
        let sides = detection::pair(images, self.settings, work, self.parallel_detection)?;
        let map_pixel = |side: usize, pixel: [f64; 2]| -> Result<[f64; 2], String> {
            if self.settings.reflect_x[side] {
                reflect_pixel_x(
                    [frames[side].profile.width, frames[side].profile.height],
                    pixel,
                )
            } else {
                Ok(pixel)
            }
        };
        let first_indices = sides[0]
            .traces
            .traces
            .iter()
            .filter(|t| t.continuity_supported)
            .flat_map(|t| t.observation_indices.iter().copied())
            .collect::<Vec<_>>();
        if first_indices.len() > self.settings.maximum_queries {
            return Err(
                "automatic trace query limit exceeded; no observations were silently sampled"
                    .into(),
            );
        }
        let trace_ids = sides[1]
            .traces
            .traces
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.continuity_supported.then_some(i))
            .collect::<Vec<_>>();
        if trace_ids.len() > 128 {
            return Err("automatic second-trace limit exceeded".into());
        }
        let mut traces = Vec::new();
        let mut vertices = 0usize;
        let mut segments = 0usize;
        for &id in &trace_ids {
            work.check()?;
            let indices = &sides[1].traces.traces[id].observation_indices;
            if !(2..=4096).contains(&indices.len()) {
                return Err("unsupported second-trace vertex count".into());
            }
            vertices += indices.len();
            segments += indices.len() - 1;
            traces.push(
                indices
                    .iter()
                    .map(|&i| map_pixel(1, sides[1].detections.observations[i].pixel))
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        if vertices > 65536 {
            return Err("automatic trace vertex limit exceeded".into());
        }
        let domain_queries = if self.retain_valid_domain_segments && !traces.is_empty() {
            Some(self.matcher.prepare_domain_traces(
                &traces,
                &self.settings.sheets,
                self.settings.matching,
                &mut || work.check().is_err(),
            )?)
        } else {
            None
        };
        let mut domain_rejections = self.retain_valid_domain_segments.then(|| {
            let rejected = domain_queries.as_ref().map(|q| q.rejections());
            DomainSummary {
                rejected_trace_vertices: rejected.map_or(0, |r| r.vertices.len()),
                rejected_trace_segments: rejected.map_or(0, |r| r.segments.len()),
                rejected_sheet_projections: 0,
            }
        });
        let prepared_queries = if traces.is_empty() || self.retain_valid_domain_segments {
            None
        } else {
            Some(self.matcher.prepare_traces(
                &traces,
                &self.settings.sheets,
                self.settings.matching,
                &mut || work.check().is_err(),
            ))
        };
        let per_query_bound = vertices + segments * 2 * self.settings.sheets.len();
        let coarse_work_bound = per_query_bound
            .checked_mul(first_indices.len())
            .ok_or("automatic work bound overflow")?;
        // Preserve the cheap admission path and its existing output for ordinary
        // inputs. When it rejects, count actual projected roots before any matching.
        let mut total_work_bound = coarse_work_bound;
        let mut prepared_pixels = None;
        let mut domain_pixels = None;
        let mut projection_failures = 0usize;
        if coarse_work_bound > self.settings.maximum_total_work
            && let Some(prepared) = &domain_queries
        {
            total_work_bound = 0;
            let mut pixels = Vec::with_capacity(first_indices.len());
            for (query, &index) in first_indices.iter().enumerate() {
                work.update(Stage::Validating, query, first_indices.len())?;
                let pixel = map_pixel(0, sides[0].detections.observations[index].pixel)?;
                let prepared = prepared.prepare_pixel(pixel, &mut || work.check().is_err());
                work.check()?;
                projection_failures += usize::from(prepared.preparation_failed());
                total_work_bound = total_work_bound
                    .checked_add(prepared.work_bound())
                    .ok_or("automatic work bound overflow")?;
                if total_work_bound > self.settings.maximum_total_work {
                    return Err("automatic total work budget exceeded before matching".into());
                }
                pixels.push(prepared);
            }
            domain_pixels = Some(pixels.into_iter());
        } else if coarse_work_bound > self.settings.maximum_total_work {
            let Some(Ok(prepared)) = &prepared_queries else {
                return Err("automatic total work budget exceeded before matching".into());
            };
            total_work_bound = 0;
            let mut pixels = Vec::with_capacity(first_indices.len());
            for (query, &index) in first_indices.iter().enumerate() {
                work.update(Stage::Validating, query, first_indices.len())?;
                let pixel = map_pixel(0, sides[0].detections.observations[index].pixel)?;
                let prepared = prepared.prepare_pixel(pixel, &mut || work.check().is_err());
                work.check()?;
                match &prepared {
                    Ok(pixel) => {
                        total_work_bound = total_work_bound
                            .checked_add(pixel.work_bound())
                            .ok_or("automatic work bound overflow")?;
                    }
                    // These errors precede any counted matching work and are retained.
                    Err(_) => projection_failures += 1,
                }
                if total_work_bound > self.settings.maximum_total_work {
                    return Err("automatic total work budget exceeded before matching".into());
                }
                pixels.push(prepared);
            }
            prepared_pixels = Some(pixels.into_iter());
        }

        let admission = Admission {
            coarse_total_work_bound: coarse_work_bound,
            conservative_total_work_bound: total_work_bound,
            uses_projected_roots: prepared_pixels.is_some() || domain_pixels.is_some(),
            projected_root_preparation_failures: projection_failures,
        };
        sink.begin(PairInfo {
            sides: &sides,
            first_query_observation_indices: &first_indices,
            second_matcher_trace_ids: &trace_ids,
            second_matcher_trace_pixels: &traces,
            admission,
            domain_rejections: domain_queries.as_ref().map(|q| q.rejections()),
        })?;
        work.check()?;
        let mut candidates = Vec::new();
        let mut outcomes = Vec::with_capacity(first_indices.len());
        let mut failures = 0usize;
        let mut no_unique = 0usize;
        for (query, &index) in first_indices.iter().enumerate() {
            work.update(Stage::Matching, query, first_indices.len())?;
            let pixel = map_pixel(0, sides[0].detections.observations[index].pixel)?;
            let domain_result = if let Some(pixels) = &mut domain_pixels {
                Some(
                    pixels
                        .next()
                        .ok_or("inconsistent prepared domain query count")?
                        .execute(&mut || work.check().is_err()),
                )
            } else {
                domain_queries.as_ref().map(|q| {
                    q.prepare_pixel(pixel, &mut || work.check().is_err())
                        .execute(&mut || work.check().is_err())
                })
            };
            let (result, rejected_projections) = if let Some(output) = domain_result {
                if let Some(summary) = &mut domain_rejections {
                    summary.rejected_sheet_projections += output.rejected_projections.len();
                }
                (output.result, Some(output.rejected_projections))
            } else {
                (
                    match &mut prepared_pixels {
                        Some(pixels) => {
                            match pixels.next().ok_or("inconsistent prepared query count")? {
                                Ok(pixel) => pixel.execute(&mut || work.check().is_err()),
                                Err(error) => Err(error),
                            }
                        }
                        None => match &prepared_queries {
                            None => Err("no supported second-camera traces".into()),
                            Some(Err(error)) => Err(error.clone()),
                            Some(Ok(prepared)) => {
                                prepared.match_pixel(pixel, &mut || work.check().is_err())
                            }
                        },
                    },
                    self.retain_valid_domain_segments.then(Vec::new),
                )
            };
            work.check()?;
            let outcome = match &result {
                Ok(matches) => {
                    if let Some(selected) = matches.unique {
                        let crossing = &matches.traces[selected.trace_index].crossings
                            [selected.crossing_index];
                        let point = crossing
                            .triangulation
                            .as_ref()
                            .map_err(|e| format!("inconsistent selected triangulation: {e}"))?;
                        let support = &crossing.support[selected.support_index];
                        let projection = &matches.projections[support.projection_index];
                        let candidate_index = candidates.len();
                        candidates.push(Candidate {
                            first_observation: index,
                            first_trace: sides[0].membership[index].0,
                            selection: selected,
                            second_trace: trace_ids[selected.trace_index],
                            second_calibration_pixel: crossing.intersection.right_pixel,
                            second_raw_image_pixel: map_pixel(
                                1,
                                crossing.intersection.right_pixel,
                            )?,
                            sheet_index: projection.sheet_index,
                            sheet_root_index: projection.root_index,
                            position_in_supplied_geometry_units: point.position_mm,
                        });
                        Outcome::Candidate { candidate_index }
                    } else {
                        no_unique += 1;
                        Outcome::NoUnique {
                            supported_combinations: matches.supported_combinations,
                            ambiguous_segments: matches
                                .traces
                                .iter()
                                .map(|t| t.ambiguous_segments.len())
                                .sum(),
                        }
                    }
                }
                Err(error) => {
                    failures += 1;
                    Outcome::Error(error.clone())
                }
            };
            sink.query(&Query {
                first_observation: index,
                first_calibration_pixel: pixel,
                result,
                rejected_projections,
            })?;
            work.check()?;
            outcomes.push(QueryOutcome {
                first_observation: index,
                first_calibration_pixel: pixel,
                outcome,
            });
        }
        work.update(Stage::Matching, first_indices.len(), first_indices.len())?;
        let cameras = std::array::from_fn(|index| {
            let side = &sides[index];
            CameraSummary {
                interface: frames[index].profile.interface,
                observations: side.detections.observations.len(),
                supported_traces: side
                    .traces
                    .traces
                    .iter()
                    .filter(|t| t.continuity_supported)
                    .count(),
                supported_observations: side
                    .traces
                    .traces
                    .iter()
                    .filter(|t| t.continuity_supported)
                    .map(|t| t.observation_indices.len())
                    .sum(),
                rejected_border_runs: side.detections.rejected_border,
                rejected_width_runs: side.detections.rejected_width,
                rejected_peak_runs: side.detections.rejected_peak,
                rejected_saturation_runs: side.detections.rejected_saturation,
                continuity_ambiguous_observations: side.traces.ambiguous_observations,
                continuity_comparisons: side.traces.comparisons,
            }
        });
        work.check()?;
        Ok(Report {
            cameras,
            queries: first_indices.len(),
            query_failures: failures,
            queries_without_unique_match: no_unique,
            admission,
            domain_rejections,
            candidates,
            outcomes,
            raw_sensor_pts_equal: true,
            synchronization_verified: false,
            geometry_binding_verified: false,
            units_verified: false,
            projector_line_identity_verified: false,
            physical_accuracy_qualified: false,
        })
    }
}

#[cfg(test)]
mod tests;
