//! Explicitly retain valid contiguous portions of traces without enlarging camera domains.
//! This opt-in policy preserves rejected vertices/segments/roots. Numerical uniqueness
//! is conditional on the retained calibrated domain, never physical line identity.
use super::*;

#[derive(Debug, Serialize)]
pub struct RejectedVertex {
    pub trace_index: usize,
    pub vertex_index: usize,
    pub pixel: [f64; 2],
    pub error: String,
}
#[derive(Debug, Serialize)]
pub struct RejectedSegment {
    pub trace_index: usize,
    pub segment_index: usize,
    /// Original endpoint indices that failed inverse distortion.
    pub rejected_vertices: Vec<usize>,
}
#[derive(Debug, Serialize)]
pub struct ValidRun {
    pub trace_index: usize,
    pub first_vertex: usize,
    pub vertex_count: usize,
}
#[derive(Debug, Default, Serialize)]
pub struct TraceRejections {
    pub vertices: Vec<RejectedVertex>,
    pub segments: Vec<RejectedSegment>,
    /// Includes isolated valid vertices (count=1), which cannot form a segment.
    pub valid_runs: Vec<ValidRun>,
}
#[derive(Debug, Serialize)]
pub struct RejectedProjection {
    pub sheet_index: usize,
    pub root_index: usize,
    pub first_camera_point: [f64; 3],
    pub second_camera_point: [f64; 3],
    pub error: String,
}
struct Fragment<'t> {
    first: usize,
    normalized: Result<epipolar_trace::NormalizedTrace<'t>, String>,
}
/// Immutable cache bound to original traces/model. Every original vertex and
/// segment still contributes to the conservative work budget, including rejects.
pub struct PreparedDomainQueries<'m, 'g, 't> {
    matcher: &'m PreparedMatcher<'g>,
    sheets: &'t [QuadraticSheet],
    config: Config,
    fragments: Vec<Vec<Fragment<'t>>>,
    vertices: usize,
    rejections: TraceRejections,
}
pub struct DomainOutcome {
    pub result: Result<Matches, String>,
    pub rejected_projections: Vec<RejectedProjection>,
}
pub struct PreparedDomainPixel<'p, 'm, 'g, 't> {
    queries: &'p PreparedDomainQueries<'m, 'g, 't>,
    pixel: [f64; 2],
    projections: Result<Vec<SheetProjection>, String>,
    rejections: Vec<RejectedProjection>,
}
fn check(cancelled: &mut impl FnMut() -> bool) -> Result<(), String> {
    if cancelled() {
        Err("laser trace matching cancelled".into())
    } else {
        Ok(())
    }
}
impl<'a> PreparedMatcher<'a> {
    /// Explicit alternative to the unchanged strict prepare_traces contract.
    /// Malformed pixels/gaps, cancellation and limits still fail the operation.
    /// Finite in-image vertices whose inverse cannot be evaluated are recorded,
    /// and both adjacent segments are excluded. No gap is bridged or extrapolated.
    pub fn prepare_domain_traces<'m, 't>(
        &'m self,
        traces: &'t [Vec<[f64; 2]>],
        sheets: &'t [QuadraticSheet],
        config: Config,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<PreparedDomainQueries<'m, 'a, 't>, String> {
        check(cancelled)?;
        let vertices = validate_inputs(traces, sheets, config)?;
        if sheets.iter().any(|s| {
            ![s.quadratic_y, s.linear_y, s.linear_z, s.offset]
                .iter()
                .all(|v| v.is_finite())
        }) {
            return Err("nonfinite laser sheet coefficients".into());
        }
        let mut fragments = Vec::with_capacity(traces.len());
        let mut rejections = TraceRejections::default();
        for (trace_index, raw) in traces.iter().enumerate() {
            let mut valid = Vec::with_capacity(raw.len());
            for (i, &pixel) in raw.iter().enumerate() {
                check(cancelled)?;
                if !self.geometry.right.contains_pixel(pixel) {
                    return Err("observation is outside the calibrated image".into());
                }
                if i > 0 {
                    let gap = (pixel[0] - raw[i - 1][0]).hypot(pixel[1] - raw[i - 1][1]);
                    if !gap.is_finite()
                        || gap == 0.
                        || gap > config.intersection.maximum_segment_length_px
                    {
                        return Err("duplicate or oversized epipolar trace segment".into());
                    }
                }
                match self.second.ray(pixel) {
                    Ok(_) => valid.push(true),
                    Err(error) => {
                        valid.push(false);
                        rejections.vertices.push(RejectedVertex {
                            trace_index,
                            vertex_index: i,
                            pixel,
                            error,
                        });
                    }
                }
            }
            for (i, pair) in valid.windows(2).enumerate() {
                check(cancelled)?;
                if !pair[0] || !pair[1] {
                    rejections.segments.push(RejectedSegment {
                        trace_index,
                        segment_index: i,
                        rejected_vertices: (i..=i + 1).filter(|&j| !valid[j]).collect(),
                    });
                }
            }
            let mut parts = Vec::new();
            let mut i = 0;
            while i < valid.len() {
                check(cancelled)?;
                if !valid[i] {
                    i += 1;
                    continue;
                }
                let first = i;
                while i < valid.len() && valid[i] {
                    check(cancelled)?;
                    i += 1;
                }
                rejections.valid_runs.push(ValidRun {
                    trace_index,
                    first_vertex: first,
                    vertex_count: i - first,
                });
                if i - first >= 2 {
                    // Use the existing bounded, indexed geometry implementation.
                    let normalized = self.epipolar.prepare_indexed_trace(
                        &raw[first..i],
                        config.intersection,
                        cancelled,
                    )?;
                    parts.push(Fragment {
                        first,
                        normalized: Ok(normalized),
                    });
                }
            }
            fragments.push(parts);
        }
        check(cancelled)?;
        Ok(PreparedDomainQueries {
            matcher: self,
            sheets,
            config,
            fragments,
            vertices,
            rejections,
        })
    }
}
impl<'m, 'g, 't> PreparedDomainQueries<'m, 'g, 't> {
    pub fn rejections(&self) -> &TraceRejections {
        &self.rejections
    }
    /// Expected root-domain rejections remain alongside success or later failure.
    /// Intersection/numerical overflow errors are fatal; only finite roots outside
    /// the second camera's forward calibrated domain are excluded explicitly.
    pub fn prepare_pixel<'p>(
        &'p self,
        pixel: [f64; 2],
        cancelled: &mut impl FnMut() -> bool,
    ) -> PreparedDomainPixel<'p, 'm, 'g, 't> {
        let mut rejections = Vec::new();
        let projections = (|| {
            check(cancelled)?;
            let ray = self.matcher.first.ray(pixel)?;
            let mut projections = Vec::new();
            for (sheet_index, sheet) in self.sheets.iter().enumerate() {
                check(cancelled)?;
                let points = match sheet
                    .intersect([ray[0] / ray[2], ray[1] / ray[2]], self.matcher.depth)?
                {
                    RaySheetIntersections::None => vec![],
                    RaySheetIntersections::One(p) => vec![p],
                    RaySheetIntersections::Two(ps) => ps.to_vec(),
                };
                for (root_index, point) in points.into_iter().enumerate() {
                    check(cancelled)?;
                    let transformed = self.matcher.geometry.right_from_left.apply(point);
                    if !transformed.iter().all(|v| v.is_finite()) {
                        return Err("sheet transformation overflow".into());
                    }
                    let error = if transformed[2] <= 0. {
                        Some("point must be finite and in front of the camera")
                    } else {
                        let x = transformed[0] / transformed[2];
                        let y = transformed[1] / transformed[2];
                        if !x.is_finite() || !y.is_finite() {
                            return Err("sheet projection normalization overflow".into());
                        }
                        (x.hypot(y) > self.matcher.geometry.right.valid_radius)
                            .then_some("point is outside the calibrated lens domain")
                    };
                    if let Some(error) = error {
                        rejections.push(RejectedProjection {
                            sheet_index,
                            root_index,
                            first_camera_point: point,
                            second_camera_point: transformed,
                            error: error.into(),
                        });
                    } else {
                        projections.push(SheetProjection {
                            sheet_index,
                            root_index,
                            first_camera_point: point,
                            second_pixel: self.matcher.second.project(transformed)?,
                        });
                    }
                }
            }
            check(cancelled)?;
            Ok(projections)
        })();
        PreparedDomainPixel {
            queries: self,
            pixel,
            projections,
            rejections,
        }
    }
}
impl PreparedDomainPixel<'_, '_, '_, '_> {
    pub fn preparation_failed(&self) -> bool {
        self.projections.is_err()
    }
    pub fn work_bound(&self) -> usize {
        self.projections.as_ref().map_or(0, |p| {
            self.queries.vertices + (self.queries.vertices - self.queries.fragments.len()) * p.len()
        })
    }
    pub fn execute(self, cancelled: &mut impl FnMut() -> bool) -> DomainOutcome {
        let result = (|| {
            check(cancelled)?;
            let projections = self.projections?;
            let queries = self.queries;
            let constraint = queries
                .matcher
                .epipolar
                .for_pixel(self.pixel, queries.config.intersection);
            queries.matcher.match_intersections(
                MatchingInput {
                    first_pixel: self.pixel,
                    trace_count: queries.fragments.len(),
                    vertices: queries.vertices,
                    projections,
                    config: queries.config,
                },
                |index, cancelled| {
                    let mut output = epipolar_trace::Intersections {
                        intersections: Vec::new(),
                        ambiguous_segments: Vec::new(),
                    };
                    for fragment in &queries.fragments[index] {
                        check(cancelled)?;
                        let mut hits =
                            constraint.intersections_cached(&fragment.normalized, cancelled)?;
                        for hit in &mut hits.intersections {
                            hit.segment_index += fragment.first;
                            if let Some(i) = &mut hit.vertex_index {
                                *i += fragment.first;
                            }
                        }
                        output.intersections.extend(hits.intersections);
                        output.ambiguous_segments.extend(
                            hits.ambiguous_segments
                                .into_iter()
                                .map(|i| i + fragment.first),
                        );
                    }
                    Ok(output)
                },
                cancelled,
            )
        })();
        DomainOutcome {
            result,
            rejected_projections: self.rejections,
        }
    }
}

#[cfg(test)]
mod tests;
