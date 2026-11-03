//! Epipolar intersections with a supplied image polyline, without line identity.
//! Segments are straight in undistorted normalized coordinates, not raw pixels.
use crate::{PreparedCamera, epipolar::raw_field_matrices, stereo::StereoCalibration};
use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};
use std::{cell::OnceCell, collections::BTreeSet};
mod index;
use index::{BLOCK_SEGMENTS, Block, MAX_TRACE_VERTICES};

struct Constraint {
    line: Vector3<f64>,
    config: Config,
}
struct Span<'a> {
    first: usize,
    distances: &'a [f64],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Euclidean raw-image endpoint separation. Longer gaps fail the whole call.
    pub maximum_segment_length_px: f64,
    /// Numerical line-distance tolerance in normalized image coordinates.
    /// This is not a physical measurement or pixel-accuracy tolerance.
    pub normalized_tolerance: f64,
    /// Reject reference rays nearly collinear with the translated baseline.
    pub minimum_plane_sine: f64,
}

impl Config {
    pub fn validate(self) -> Result<(), String> {
        let config = self;
        if !config.maximum_segment_length_px.is_finite()
            || !(0. ..=4096.).contains(&config.maximum_segment_length_px)
            || config.maximum_segment_length_px == 0.
            || !config.normalized_tolerance.is_finite()
            || config.normalized_tolerance <= 0.
            || config.normalized_tolerance > 1e-3
            || !config.minimum_plane_sine.is_finite()
            || config.minimum_plane_sine <= 0.
            || config.minimum_plane_sine > 1.
        {
            return Err("invalid epipolar trace limits".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Intersection {
    pub segment_index: usize,
    /// Fraction along the undistorted normalized segment, in [0,1].
    pub fraction: f64,
    /// Original vertex index for a tolerance-qualified endpoint; None for interpolation.
    pub vertex_index: Option<usize>,
    pub right_pixel: [f64; 2],
    pub normalized_epipolar_distance: f64,
}

#[derive(Debug, Serialize)]
pub struct Intersections {
    /// Multiple spatial crossings are retained. Only a shared vertex is deduplicated.
    pub intersections: Vec<Intersection>,
    /// Both endpoints lie within the numerical tolerance: no unique point is chosen.
    pub ambiguous_segments: Vec<usize>,
}

// Only crate geometry code can construct/use a normalized trace. Public cached
// matching retains the preparing matcher and immutable raw traces together.
pub(crate) struct NormalizedTrace<'t> {
    raw: &'t [[f64; 2]],
    points: Vec<Vector3<f64>>,
    blocks: Vec<Block>,
}

// Lazy per-pixel constraint bound to one immutable preparing model/configuration.
// No trace results, session history or cross-pixel state is cached.
pub(crate) struct PixelQuery<'p, 'g> {
    owner: &'p PreparedTrace<'g>,
    pixel: [f64; 2],
    config: Config,
    line: OnceCell<Result<Vector3<f64>, String>>,
}
impl PixelQuery<'_, '_> {
    pub(crate) fn intersections_cached(
        &self,
        trace: &Result<NormalizedTrace<'_>, String>,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Intersections, String> {
        // Keep the old line-preparation checkpoint even when arithmetic is reused.
        // A pulse cancellation still precedes cached geometry/trace errors.
        if cancelled() {
            return Err("epipolar trace cancelled".into());
        }
        let line = self
            .line
            .get_or_init(|| self.owner.compute_line(self.pixel, self.config))
            .as_ref()
            .map_err(Clone::clone)?;
        self.owner.intersect_line_indexed(
            *line,
            trace.as_ref().map_err(Clone::clone)?,
            self.config,
            cancelled,
        )
    }
}

/// Numerically validated camera geometry; preparation does not qualify calibration.
pub struct PreparedTrace<'a> {
    first: PreparedCamera<'a>,
    second: PreparedCamera<'a>,
    calibration: &'a StereoCalibration,
    essential: Matrix3<f64>,
}

impl<'a> PreparedTrace<'a> {
    pub fn new(calibration: &'a StereoCalibration) -> Result<Self, String> {
        calibration.validate()?;
        let t = calibration.right_from_left.translation_mm;
        let baseline = t[0].hypot(t[1]).hypot(t[2]);
        // Identity intrinsics produce the normalized-coordinate essential matrix.
        // PreparedCamera handles each real intrinsic matrix, including skew.
        let matrices = raw_field_matrices(
            [[1., 1., 0., 0.]; 2],
            calibration.right_from_left.rotation,
            t.map(|v| v / baseline),
        )?;
        Ok(Self {
            first: calibration.left.prepare()?,
            second: calibration.right.prepare()?,
            calibration,
            essential: Matrix3::from_row_slice(&matrices.essential.concat()),
        })
    }

    /// Intersect E * first_ray with every segment of an explicit right-image trace.
    /// Inputs contain 2..=4096 vertices; adjacent vertices must differ and meet the
    /// raw pixel gap bound. All camera-domain, finite, gap and cancellation errors
    /// fail without partial output. Time/storage are O(vertices), with existing
    /// bounded inverse-distortion iterations per vertex. No extrapolation occurs.
    ///
    /// Missing intersections, multiple crossings and near-coincident segments stay
    /// explicit. An isolated hit is not proof of correspondence. The caller must
    /// preserve trace identity, gap/interpolation provenance and ambiguity across
    /// other traces before any independent triangulation/qualification checks.
    pub fn intersections(
        &self,
        first_pixel: [f64; 2],
        trace: &[[f64; 2]],
        config: Config,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Intersections, String> {
        if cancelled() {
            return Err("epipolar trace cancelled".into());
        }
        config.validate()?;
        if !(2..=4096).contains(&trace.len()) {
            return Err("invalid epipolar trace limits".into());
        }
        let line = self.line(first_pixel, config, cancelled)?;
        let trace = self.prepare_trace(trace, config, cancelled)?;
        self.intersect_line_linear(line, &trace, config, cancelled)
    }

    pub(crate) fn prepare_trace<'t>(
        &self,
        trace: &'t [[f64; 2]],
        config: Config,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<NormalizedTrace<'t>, String> {
        fn check(cancelled: &mut impl FnMut() -> bool) -> Result<(), String> {
            if cancelled() {
                Err("epipolar trace cancelled".to_string())
            } else {
                Ok(())
            }
        }
        check(cancelled)?;
        config.validate()?;
        if !(2..=4096).contains(&trace.len()) {
            return Err("invalid epipolar trace limits".into());
        }
        let mut points = Vec::with_capacity(trace.len());
        for (i, &pixel) in trace.iter().enumerate() {
            check(cancelled)?;
            if i > 0 {
                let gap = (pixel[0] - trace[i - 1][0]).hypot(pixel[1] - trace[i - 1][1]);
                if !gap.is_finite() || gap == 0. || gap > config.maximum_segment_length_px {
                    return Err("duplicate or oversized epipolar trace segment".into());
                }
            }
            let ray = self.second.ray(pixel)?;
            let point = Vector3::new(ray[0] / ray[2], ray[1] / ray[2], 1.);
            if !point.iter().all(|v| v.is_finite()) {
                return Err("epipolar trace coordinate overflow".into());
            }
            points.push(point);
        }
        check(cancelled)?;
        Ok(NormalizedTrace {
            raw: trace,
            points,
            blocks: Vec::new(),
        })
    }

    // Raw validation and inverse distortion still visit every vertex. The index
    // only accelerates subsequent queries; it cannot turn an invalid trace valid.
    pub(crate) fn prepare_indexed_trace<'t>(
        &self,
        raw: &'t [[f64; 2]],
        config: Config,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<NormalizedTrace<'t>, String> {
        let mut trace = self.prepare_trace(raw, config, cancelled)?;
        if raw.len() >= 2 * BLOCK_SEGMENTS {
            for first in (0..raw.len() - 1).step_by(BLOCK_SEGMENTS) {
                if cancelled() {
                    return Err("epipolar trace cancelled".into());
                }
                let end = (first + BLOCK_SEGMENTS).min(raw.len() - 1);
                let mut min = [f64::INFINITY; 2];
                let mut max = [f64::NEG_INFINITY; 2];
                for point in &trace.points[first..=end] {
                    for axis in 0..2 {
                        min[axis] = min[axis].min(point[axis]);
                        max[axis] = max[axis].max(point[axis]);
                    }
                }
                trace.blocks.push(Block {
                    first,
                    end,
                    min,
                    max,
                });
            }
        }
        Ok(trace)
    }
    fn line(
        &self,
        first_pixel: [f64; 2],
        config: Config,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Vector3<f64>, String> {
        fn check(cancelled: &mut impl FnMut() -> bool) -> Result<(), String> {
            if cancelled() {
                Err("epipolar trace cancelled".to_string())
            } else {
                Ok(())
            }
        }
        check(cancelled)?;
        self.compute_line(first_pixel, config)
    }
    fn compute_line(&self, first_pixel: [f64; 2], config: Config) -> Result<Vector3<f64>, String> {
        let line = self.essential * Vector3::from(self.first.ray(first_pixel)?);
        let norm = line.x.hypot(line.y).hypot(line.z);
        let xy_norm = line.x.hypot(line.y);
        if !norm.is_finite() || norm < config.minimum_plane_sine || xy_norm == 0. {
            return Err("degenerate reference-ray epipolar constraint".into());
        }
        let line = line / xy_norm;
        if !line.iter().all(|v| v.is_finite()) {
            return Err("epipolar constraint normalization overflow".into());
        }
        Ok(line)
    }

    pub(crate) fn for_pixel(&self, first_pixel: [f64; 2], config: Config) -> PixelQuery<'_, 'a> {
        PixelQuery {
            owner: self,
            pixel: first_pixel,
            config,
            line: OnceCell::new(),
        }
    }

    fn intersect_line_linear(
        &self,
        line: Vector3<f64>,
        trace: &NormalizedTrace<'_>,
        config: Config,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Intersections, String> {
        fn check(cancelled: &mut impl FnMut() -> bool) -> Result<(), String> {
            if cancelled() {
                Err("epipolar trace cancelled".to_string())
            } else {
                Ok(())
            }
        }
        let mut distances = Vec::with_capacity(trace.points.len());
        for point in &trace.points {
            check(cancelled)?;
            let distance = line.dot(point);
            if !distance.is_finite() {
                return Err("epipolar trace coordinate overflow".into());
            }
            distances.push(distance);
        }
        let mut output = Intersections {
            intersections: Vec::new(),
            ambiguous_segments: Vec::new(),
        };
        let mut vertices = BTreeSet::new();
        self.append_span(
            trace,
            &Constraint { line, config },
            Span {
                first: 0,
                distances: &distances,
            },
            &mut output,
            &mut vertices,
            cancelled,
        )?;
        check(cancelled)?;
        Ok(output)
    }
    fn intersect_line_indexed(
        &self,
        line: Vector3<f64>,
        trace: &NormalizedTrace<'_>,
        config: Config,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Intersections, String> {
        const MAX_BLOCKS: usize = (MAX_TRACE_VERTICES - 1).div_ceil(BLOCK_SEGMENTS);
        if trace.blocks.is_empty() || trace.blocks.len() > MAX_BLOCKS {
            return self.intersect_line_linear(line, trace, config, cancelled);
        }
        // Classify every block before any scalar interpolation. If a bound risks
        // overflow, use the original full distance prepass and its error order.
        let mut active = [false; MAX_BLOCKS];
        for (i, block) in trace.blocks.iter().enumerate() {
            if cancelled() {
                return Err("epipolar trace cancelled".into());
            }
            let Some(value) = block.may_cross(line, config.normalized_tolerance) else {
                return self.intersect_line_linear(line, trace, config, cancelled);
            };
            active[i] = value;
        }
        let mut output = Intersections {
            intersections: Vec::new(),
            ambiguous_segments: Vec::new(),
        };
        let mut vertices = BTreeSet::new();
        let mut distances = Vec::with_capacity(BLOCK_SEGMENTS + 1);
        for (block, &active) in trace.blocks.iter().zip(&active) {
            if cancelled() {
                return Err("epipolar trace cancelled".into());
            }
            if !active {
                continue;
            }
            distances.clear();
            for point in &trace.points[block.first..=block.end] {
                if cancelled() {
                    return Err("epipolar trace cancelled".into());
                }
                let distance = line.dot(point);
                if !distance.is_finite() {
                    return Err("epipolar trace coordinate overflow".into());
                }
                distances.push(distance);
            }
            self.append_span(
                trace,
                &Constraint { line, config },
                Span {
                    first: block.first,
                    distances: &distances,
                },
                &mut output,
                &mut vertices,
                cancelled,
            )?;
        }
        if cancelled() {
            return Err("epipolar trace cancelled".into());
        }
        Ok(output)
    }
    fn append_span(
        &self,
        trace: &NormalizedTrace<'_>,
        query: &Constraint,
        span: Span<'_>,
        output: &mut Intersections,
        vertices: &mut BTreeSet<usize>,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<(), String> {
        for local in 0..span.distances.len() - 1 {
            let i = span.first + local;
            if cancelled() {
                return Err("epipolar trace cancelled".into());
            }
            let a = span.distances[local];
            let b = span.distances[local + 1];
            let close_a = a.abs() <= query.config.normalized_tolerance;
            let close_b = b.abs() <= query.config.normalized_tolerance;
            if close_a && close_b {
                output.ambiguous_segments.push(i);
                continue;
            }
            let (fraction, vertex_index) = if close_a {
                (0., Some(i))
            } else if close_b {
                (1., Some(i + 1))
            } else if a.is_sign_positive() == b.is_sign_positive() {
                continue;
            } else {
                // Scaling avoids overflow in a-b while preserving opposite signs.
                let scale = a.abs().max(b.abs());
                let a_scaled = a / scale;
                (a_scaled / (a_scaled - b / scale), None)
            };
            if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
                return Err("invalid epipolar segment intersection fraction".into());
            }
            if let Some(vertex) = vertex_index
                && !vertices.insert(vertex)
            {
                continue;
            }
            let point = trace.points[i] * (1. - fraction) + trace.points[i + 1] * fraction;
            let pixel = match vertex_index {
                Some(vertex) => trace.raw[vertex],
                None => self.second.project(point.into())?,
            };
            let residual = query.line.dot(&point).abs();
            if !residual.is_finite() || !self.calibration.right.contains_pixel(pixel) {
                return Err("epipolar intersection leaves the supported image".into());
            }
            output.intersections.push(Intersection {
                segment_index: i,
                fraction,
                vertex_index,
                right_pixel: pixel,
                normalized_epipolar_distance: residual,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Brown5,
        fixtures::camera,
        stereo::{RigidPose, TriangulationLimits},
    };
    pub(super) fn rig(distorted: bool) -> StereoCalibration {
        let mut left = camera();
        left.skew = 1.3;
        if !distorted {
            left.distortion = Brown5::ZERO;
        }
        StereoCalibration {
            right: left.clone(),
            left,
            right_from_left: RigidPose {
                rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                translation_mm: [-80., 0., 0.],
            },
        }
    }
    pub(super) fn config() -> Config {
        Config {
            maximum_segment_length_px: 64.,
            normalized_tolerance: 1e-10,
            minimum_plane_sine: 1e-8,
        }
    }
    #[test]
    fn analytic_fraction_and_stereo_depth_with_distortion_and_skew() {
        for distorted in [false, true] {
            let mut calibration = rig(distorted);
            let (s, c) = 0.05_f64.sin_cos();
            calibration.right_from_left.rotation = [[c, 0., s], [0., 1., 0.], [-s, 0., c]];
            calibration.right_from_left.translation_mm = [-80., 3., 6.];
            calibration.right.fx *= 1.02;
            calibration.right.fy *= 0.98;
            let prepared = PreparedTrace::new(&calibration).unwrap();
            for point in [[40., -35., 500.], [-60., 50., 700.], [10., 0., 400.]] {
                let right = calibration.right_from_left.apply(point);
                let q = [right[0] / right[2], right[1] / right[2], 1.];
                let endpoints = [
                    [q[0] - 0.005, q[1] - 0.01, 1.],
                    [q[0] + 0.01, q[1] + 0.02, 1.],
                ]
                .map(|v| calibration.right.project(v).unwrap());
                let first = calibration.left.project(point).unwrap();
                let result = prepared
                    .intersections(first, &endpoints, config(), &mut || false)
                    .unwrap();
                assert!(result.ambiguous_segments.is_empty());
                assert_eq!(result.intersections.len(), 1);
                let hit = &result.intersections[0];
                assert!((hit.fraction - 1. / 3.).abs() < 1e-8);
                assert_eq!(hit.vertex_index, None);
                let expected = calibration.right.project(right).unwrap();
                assert!(
                    (hit.right_pixel[0] - expected[0]).hypot(hit.right_pixel[1] - expected[1])
                        < 1e-7
                );
                let triangulated = calibration
                    .prepare(TriangulationLimits {
                        min_depth_z_mm: 100.,
                        max_depth_z_mm: 1000.,
                        min_ray_angle_rad: 0.0001,
                        max_ray_gap_mm: 1.,
                        max_reprojection_error_px: 2.,
                    })
                    .unwrap()
                    .triangulate(first, hit.right_pixel)
                    .unwrap();
                assert!(
                    (Vector3::from(triangulated.position_mm) - Vector3::from(point)).norm() < 1e-6
                );
            }
        }
    }
    #[test]
    fn missing_parallel_coincident_and_repeated_crossings_stay_explicit() {
        let calibration = rig(false);
        let prepared = PreparedTrace::new(&calibration).unwrap();
        let first = calibration.left.project([0., 0., 500.]).unwrap();
        let cx = calibration.right.cx;
        let cy = calibration.right.cy;
        let missing = prepared
            .intersections(
                first,
                &[[cx, cy + 10.], [cx + 20., cy + 10.]],
                config(),
                &mut || false,
            )
            .unwrap();
        assert!(missing.intersections.is_empty() && missing.ambiguous_segments.is_empty());
        let coincident = prepared
            .intersections(first, &[[cx, cy], [cx + 20., cy]], config(), &mut || false)
            .unwrap();
        assert!(coincident.intersections.is_empty());
        assert_eq!(coincident.ambiguous_segments, vec![0]);
        let near = prepared
            .intersections(
                first,
                &[[cx, cy - 0.001], [cx + 20., cy + 0.001]],
                Config {
                    normalized_tolerance: 1e-5,
                    ..config()
                },
                &mut || false,
            )
            .unwrap();
        assert!(near.intersections.is_empty());
        assert_eq!(near.ambiguous_segments, vec![0]);
        let two = prepared
            .intersections(
                first,
                &[[cx, cy - 10.], [cx + 10., cy + 10.], [cx + 20., cy - 10.]],
                config(),
                &mut || false,
            )
            .unwrap();
        assert_eq!(two.intersections.len(), 2);
        let vertex = prepared
            .intersections(
                first,
                &[[cx, cy - 10.], [cx + 10., cy], [cx + 20., cy + 10.]],
                config(),
                &mut || false,
            )
            .unwrap();
        assert_eq!(vertex.intersections.len(), 1);
        assert_eq!(vertex.intersections[0].vertex_index, Some(1));
    }
    #[test]
    fn reversed_trace_and_baseline_scale_preserve_intersection() {
        let mut calibration = rig(true);
        let point = [40., 20., 500.];
        let first = calibration.left.project(point).unwrap();
        let second = calibration.right_from_left.apply(point);
        let q = [second[0] / second[2], second[1] / second[2]];
        let trace = [[-0.002, -0.01], [0.004, 0.02]].map(|d| {
            calibration
                .right
                .project([q[0] + d[0], q[1] + d[1], 1.])
                .unwrap()
        });
        let a = PreparedTrace::new(&calibration)
            .unwrap()
            .intersections(first, &trace, config(), &mut || false)
            .unwrap();
        calibration.right_from_left.translation_mm[0] *= 1000.;
        let b = PreparedTrace::new(&calibration)
            .unwrap()
            .intersections(first, &[trace[1], trace[0]], config(), &mut || false)
            .unwrap();
        assert!((a.intersections[0].fraction + b.intersections[0].fraction - 1.).abs() < 1e-12);
        assert!(
            (Vector3::new(
                a.intersections[0].right_pixel[0],
                a.intersections[0].right_pixel[1],
                0.
            ) - Vector3::new(
                b.intersections[0].right_pixel[0],
                b.intersections[0].right_pixel[1],
                0.
            ))
            .norm()
                < 1e-10
        );
    }
    #[test]
    fn invalid_inputs_cancellation_and_epipole_fail_without_partial_results() {
        let mut calibration = rig(false);
        let prepared = PreparedTrace::new(&calibration).unwrap();
        let first = [640., 400.];
        let trace = [[500., 390.], [500., 410.]];
        assert!(
            prepared
                .intersections(first, &vec![trace[0]; 4097], config(), &mut || false)
                .unwrap_err()
                .contains("invalid epipolar trace limits")
        );
        for bad in [
            vec![],
            vec![trace[0]],
            vec![trace[0], trace[0]],
            vec![[f64::NAN, 390.], trace[1]],
            vec![trace[0], [500., 700.]],
        ] {
            assert!(
                prepared
                    .intersections(first, &bad, config(), &mut || false)
                    .is_err()
            );
        }
        assert!(
            prepared
                .intersections(
                    first,
                    &trace,
                    Config {
                        normalized_tolerance: 0.,
                        ..config()
                    },
                    &mut || false
                )
                .is_err()
        );
        let mut calls = 0;
        assert!(
            prepared
                .intersections(first, &trace, config(), &mut || {
                    calls += 1;
                    calls == 4
                })
                .unwrap_err()
                .contains("cancelled")
        );
        calibration.right_from_left.translation_mm = [0., 0., 80.];
        assert!(
            PreparedTrace::new(&calibration)
                .unwrap()
                .intersections(
                    [calibration.left.cx, calibration.left.cy],
                    &trace,
                    config(),
                    &mut || false
                )
                .unwrap_err()
                .contains("degenerate")
        );
    }
}

#[cfg(test)]
mod index_tests;

#[cfg(test)]
mod query_tests;
