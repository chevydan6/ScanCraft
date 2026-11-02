//! Enumerate stereo trace crossings supported by explicit first-camera laser sheets.
//! Numerical uniqueness is conditional on the supplied traces, sheets and geometry;
//! it does not attest projector identity, camera binding or physical accuracy.
use crate::{
    PreparedCamera,
    epipolar_trace::{self, Intersection, PreparedTrace},
    laser_surface::{QuadraticSheet, RaySheetIntersections},
    stereo::{PreparedStereo, StereoCalibration, TriangulatedPoint, TriangulationLimits},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub intersection: epipolar_trace::Config,
    /// Distance in second-camera pixels between a crossing and a sheet projection.
    pub maximum_sheet_distance_px: f64,
    /// Counts supplied trace vertices and crossing-to-projection comparisons.
    pub maximum_work: usize,
}

impl Config {
    pub fn validate(self) -> Result<(), String> {
        self.intersection.validate()?;
        let config = self;
        if !config.maximum_sheet_distance_px.is_finite()
            || config.maximum_sheet_distance_px <= 0.
            || config.maximum_sheet_distance_px > 32.
            || !(1..=10_000_000).contains(&config.maximum_work)
        {
            return Err("invalid laser trace matching limits".into());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct SheetProjection {
    pub sheet_index: usize,
    /// Index in the sheet's depth-ordered ray intersections, not a projector ID.
    pub root_index: usize,
    pub first_camera_point: [f64; 3],
    pub second_pixel: [f64; 2],
}

#[derive(Debug, Serialize)]
pub struct Support {
    pub projection_index: usize,
    pub distance_px: f64,
}

#[derive(Debug, Serialize)]
pub struct Crossing {
    pub intersection: Intersection,
    /// Rejected depth/angle/gap/reprojection results remain explicit.
    pub triangulation: Result<TriangulatedPoint, String>,
    /// All nearby sheet roots are kept, including duplicate model hypotheses.
    pub support: Vec<Support>,
}

#[derive(Debug, Serialize)]
pub struct TraceCrossings {
    pub trace_index: usize,
    pub crossings: Vec<Crossing>,
    pub ambiguous_segments: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MatchIndex {
    pub trace_index: usize,
    pub crossing_index: usize,
    pub support_index: usize,
}

#[derive(Debug, Serialize)]
pub struct Matches {
    pub projections: Vec<SheetProjection>,
    pub traces: Vec<TraceCrossings>,
    /// Present only for one depth-accepted (crossing, sheet-root) combination
    /// across ALL supplied traces, with no near-coincident segment anywhere.
    pub unique: Option<MatchIndex>,
    pub supported_combinations: usize,
    pub work: usize,
}

pub struct PreparedMatcher<'a> {
    geometry: &'a StereoCalibration,
    first: PreparedCamera<'a>,
    second: PreparedCamera<'a>,
    epipolar: PreparedTrace<'a>,
    stereo: PreparedStereo<'a>,
    depth: [f64; 2],
}

/// Immutable trace-ray cache bound to the exact preparing matcher, input traces,
/// sheets and limits. Borrowing prevents changing these inputs while reusing it.
/// Preparation errors for individual traces are retained and propagated when
/// queried, after the same first-ray/sheet checks as the uncached path.
pub struct PreparedQueries<'m, 'g, 't> {
    matcher: &'m PreparedMatcher<'g>,
    traces: &'t [Vec<[f64; 2]>],
    sheets: &'t [QuadraticSheet],
    config: Config,
    cache: Vec<Result<epipolar_trace::NormalizedTrace<'t>, String>>,
    vertices: usize,
}
struct CachedInputs<'c, 't> {
    traces: &'c [Result<epipolar_trace::NormalizedTrace<'t>, String>],
    first: Option<Vec<SheetProjection>>,
}

/// Projected roots for one first pixel, bound to the exact immutable preparing
/// queries. Consuming execution reuses these roots without recomputing or cloning.
/// This is numerical preparation, not calibration or correspondence attestation.
pub struct PreparedPixel<'p, 'm, 'g, 't> {
    queries: &'p PreparedQueries<'m, 'g, 't>,
    pixel: [f64; 2],
    projections: Vec<SheetProjection>,
}
impl PreparedPixel<'_, '_, '_, '_> {
    /// Conservative bound on Matches::work: vertices + segments * actual roots.
    /// Each segment contributes at most one crossing, and shared endpoints only
    /// reduce that count. Preparation, image processing and CPU time are separate.
    pub fn work_bound(&self) -> usize {
        self.queries.vertices
            + (self.queries.vertices - self.queries.traces.len()) * self.projections.len()
    }
    pub fn execute(self, cancelled: &mut impl FnMut() -> bool) -> Result<Matches, String> {
        self.queries.matcher.match_impl(
            self.pixel,
            self.queries.traces,
            self.queries.sheets,
            self.queries.config,
            Some(CachedInputs {
                traces: &self.queries.cache,
                first: Some(self.projections),
            }),
            cancelled,
        )
    }
}
impl<'m, 'g, 't> PreparedQueries<'m, 'g, 't> {
    /// Preserve every in-range sheet root before bounded batch admission. Failure
    /// occurs before any Matches::work units; callers must retain the error and
    /// separately propagate cancellation rather than treating it as geometry.
    pub fn prepare_pixel<'p>(
        &'p self,
        pixel: [f64; 2],
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<PreparedPixel<'p, 'm, 'g, 't>, String> {
        if cancelled() {
            return Err("laser trace matching cancelled".into());
        }
        let projections = self.matcher.project_sheets(pixel, self.sheets, cancelled)?;
        Ok(PreparedPixel {
            queries: self,
            pixel,
            projections,
        })
    }
    pub fn match_pixel(
        &self,
        first_pixel: [f64; 2],
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Matches, String> {
        self.matcher.match_impl(
            first_pixel,
            self.traces,
            self.sheets,
            self.config,
            Some(CachedInputs {
                traces: &self.cache,
                first: None,
            }),
            cancelled,
        )
    }
}

impl<'a> PreparedMatcher<'a> {
    pub fn new(
        geometry: &'a StereoCalibration,
        limits: TriangulationLimits,
    ) -> Result<Self, String> {
        Ok(Self {
            geometry,
            first: geometry.left.prepare()?,
            second: geometry.right.prepare()?,
            epipolar: PreparedTrace::new(geometry)?,
            stereo: geometry.prepare(limits)?,
            depth: [limits.min_depth_z_mm, limits.max_depth_z_mm],
        })
    }

    /// Prepare second-camera inverse distortion once for repeated first pixels.
    /// Work counters/acceptance budgets keep their uncached meaning. Preparation
    /// checks cancellation per vertex; it never returns a partly prepared cache.
    pub fn prepare_traces<'m, 't>(
        &'m self,
        traces: &'t [Vec<[f64; 2]>],
        sheets: &'t [QuadraticSheet],
        config: Config,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<PreparedQueries<'m, 'a, 't>, String> {
        let mut stopped = false;
        let mut checkpoint = || {
            stopped |= cancelled();
            stopped
        };
        if checkpoint() {
            return Err("laser trace matching cancelled".into());
        }
        let vertices = validate_inputs(traces, sheets, config)?;
        let mut cache = Vec::with_capacity(traces.len());
        for trace in traces {
            cache.push(
                self.epipolar
                    .prepare_trace(trace, config.intersection, &mut checkpoint),
            );
            if checkpoint() {
                return Err("laser trace matching cancelled".into());
            }
        }
        Ok(PreparedQueries {
            matcher: self,
            traces,
            sheets,
            config,
            cache,
            vertices,
        })
    }

    /// Inputs use the supplied camera pixel conventions. Sheets use the first
    /// camera frame and the same length unit as the stereo translation/limits.
    /// No factory parsing, unit inference, reflection or recording validation.
    ///
    /// Bounds: 1..=128 sheets and traces, 2..=4096 vertices per trace, at most
    /// 65536 vertices total and 10000000 work units. Invalid geometry, a failed
    /// sheet/trace query, cancellation or budget exhaustion returns no partial
    /// selection. An out-of-image sheet projection is retained; it is not a
    /// reason to discard a hypothesis silently. Out-of-domain projections fail.
    ///
    /// Selection uses only stereo acceptance and projected sheet support, never
    /// a reference surface. Uniqueness is per first pixel; callers must handle
    /// occlusion, repeated use across first pixels and missing/false input traces.
    pub fn match_pixel(
        &self,
        first_pixel: [f64; 2],
        traces: &[Vec<[f64; 2]>],
        sheets: &[QuadraticSheet],
        config: Config,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Matches, String> {
        self.match_impl(first_pixel, traces, sheets, config, None, cancelled)
    }

    fn match_impl(
        &self,
        first_pixel: [f64; 2],
        traces: &[Vec<[f64; 2]>],
        sheets: &[QuadraticSheet],
        config: Config,
        cache: Option<CachedInputs<'_, '_>>,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Matches, String> {
        let check = |cancelled: &mut dyn FnMut() -> bool| {
            if cancelled() {
                Err("laser trace matching cancelled".to_string())
            } else {
                Ok(())
            }
        };
        check(cancelled)?;
        let vertices = validate_inputs(traces, sheets, config)?;
        let (cache, projections) = match cache {
            Some(cache) => (Some(cache.traces), cache.first),
            None => (None, None),
        };
        let projections = match projections {
            Some(projections) => projections,
            None => self.project_sheets(first_pixel, sheets, cancelled)?,
        };
        let mut results = Vec::with_capacity(traces.len());
        let mut work = vertices;
        let mut supported_combinations = 0;
        let mut selected = None;
        let mut ambiguous = false;
        for (trace_index, trace) in traces.iter().enumerate() {
            check(cancelled)?;
            let intersections = match cache {
                Some(cache) => self.epipolar.intersections_cached(
                    first_pixel,
                    &cache[trace_index],
                    config.intersection,
                    cancelled,
                )?,
                None => self.epipolar.intersections(
                    first_pixel,
                    trace,
                    config.intersection,
                    cancelled,
                )?,
            };
            ambiguous |= !intersections.ambiguous_segments.is_empty();
            let mut crossings = Vec::with_capacity(intersections.intersections.len());
            for (crossing_index, intersection) in
                intersections.intersections.into_iter().enumerate()
            {
                check(cancelled)?;
                let triangulation = self
                    .stereo
                    .triangulate(first_pixel, intersection.right_pixel);
                let mut support = Vec::new();
                for (projection_index, projection) in projections.iter().enumerate() {
                    check(cancelled)?;
                    work += 1;
                    if work > config.maximum_work {
                        return Err("laser trace work limit exceeded".into());
                    }
                    let distance_px = (projection.second_pixel[0] - intersection.right_pixel[0])
                        .hypot(projection.second_pixel[1] - intersection.right_pixel[1]);
                    if distance_px <= config.maximum_sheet_distance_px {
                        if triangulation.is_ok() {
                            supported_combinations += 1;
                            selected = Some(MatchIndex {
                                trace_index,
                                crossing_index,
                                support_index: support.len(),
                            });
                        }
                        support.push(Support {
                            projection_index,
                            distance_px,
                        });
                    }
                }
                crossings.push(Crossing {
                    intersection,
                    triangulation,
                    support,
                });
            }
            results.push(TraceCrossings {
                trace_index,
                crossings,
                ambiguous_segments: intersections.ambiguous_segments,
            });
        }
        check(cancelled)?;
        Ok(Matches {
            projections,
            traces: results,
            unique: if supported_combinations == 1 && !ambiguous {
                selected
            } else {
                None
            },
            supported_combinations,
            work,
        })
    }

    fn project_sheets(
        &self,
        first_pixel: [f64; 2],
        sheets: &[QuadraticSheet],
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Vec<SheetProjection>, String> {
        let ray = self.first.ray(first_pixel)?;
        let mut projections = Vec::new();
        for (sheet_index, sheet) in sheets.iter().enumerate() {
            if cancelled() {
                return Err("laser trace matching cancelled".into());
            }
            let roots = match sheet.intersect([ray[0] / ray[2], ray[1] / ray[2]], self.depth)? {
                RaySheetIntersections::None => Vec::new(),
                RaySheetIntersections::One(p) => vec![p],
                RaySheetIntersections::Two(ps) => ps.to_vec(),
            };
            for (root_index, point) in roots.into_iter().enumerate() {
                let transformed = self.geometry.right_from_left.apply(point);
                projections.push(SheetProjection {
                    sheet_index,
                    root_index,
                    first_camera_point: point,
                    second_pixel: self.second.project(transformed)?,
                });
            }
        }
        Ok(projections)
    }
}

fn validate_inputs(
    traces: &[Vec<[f64; 2]>],
    sheets: &[QuadraticSheet],
    config: Config,
) -> Result<usize, String> {
    config.validate()?;
    if !(1..=128).contains(&traces.len())
        || !(1..=128).contains(&sheets.len())
        || traces.iter().any(|t| !(2..=4096).contains(&t.len()))
    {
        return Err("invalid laser trace matching bounds".into());
    }
    let vertices: usize = traces.iter().map(Vec::len).sum();
    if vertices > 65536 || vertices > config.maximum_work {
        return Err("laser trace vertex or work limit exceeded".into());
    }
    Ok(vertices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Brown5, CameraModel, stereo::RigidPose};

    fn fixture() -> (
        StereoCalibration,
        TriangulationLimits,
        Config,
        QuadraticSheet,
    ) {
        let camera = CameraModel {
            width: 640,
            height: 480,
            fx: 500.,
            fy: 500.,
            cx: 320.,
            cy: 240.,
            skew: 0.,
            distortion: Brown5::ZERO,
            valid_radius: 1.,
        };
        (
            StereoCalibration {
                left: camera.clone(),
                right: camera,
                right_from_left: RigidPose {
                    rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                    translation_mm: [-50., 0., 0.],
                },
            },
            TriangulationLimits {
                min_depth_z_mm: 100.,
                max_depth_z_mm: 1000.,
                min_ray_angle_rad: 0.0001,
                max_ray_gap_mm: 0.01,
                max_reprojection_error_px: 0.01,
            },
            Config {
                intersection: epipolar_trace::Config {
                    maximum_segment_length_px: 16.,
                    normalized_tolerance: 1e-10,
                    minimum_plane_sine: 1e-8,
                },
                maximum_sheet_distance_px: 1.,
                maximum_work: 1000,
            },
            QuadraticSheet {
                quadratic_y: 0.,
                linear_y: 0.,
                linear_z: 0.,
                offset: 20.,
            },
        )
    }
    fn trace(x: f64) -> Vec<[f64; 2]> {
        vec![[x, 249.], [x, 251.]]
    }

    #[test]
    fn prepared_pixels_bound_every_crossing_and_preserve_zero_one_two_root_results() {
        let (rig, limits, mut config, sheet) = fixture();
        config.maximum_sheet_distance_px = 32.;
        let matcher = PreparedMatcher::new(&rig, limits).unwrap();
        let zigzag = vec![[289., 249.], [291., 251.], [289., 249.], [291., 251.]];
        let traces = vec![trace(290.), zigzag, vec![[289., 250.], [291., 250.]]];
        let mut root_counts = Vec::new();
        for sheets in [
            vec![QuadraticSheet {
                offset: 80.,
                ..sheet
            }],
            vec![sheet],
            vec![QuadraticSheet {
                quadratic_y: 1.,
                linear_y: 0.,
                linear_z: -0.36,
                offset: 96.,
            }],
            vec![sheet, sheet],
        ] {
            let queries = matcher
                .prepare_traces(&traces, &sheets, config, &mut || false)
                .unwrap();
            let prepared = queries.prepare_pixel([340., 250.], &mut || false).unwrap();
            let bound = prepared.work_bound();
            let actual = prepared.execute(&mut || false).unwrap();
            let reference = queries.match_pixel([340., 250.], &mut || false).unwrap();
            assert_eq!(
                serde_json::to_value(&actual).unwrap(),
                serde_json::to_value(reference).unwrap()
            );
            let vertices = traces.iter().map(Vec::len).sum::<usize>();
            assert_eq!(
                bound,
                vertices + (vertices - traces.len()) * actual.projections.len()
            );
            assert!(actual.work <= bound);
            assert!(actual.unique.is_none()); // coincident segment and competing crossings retained.
            root_counts.push(actual.projections.len());
        }
        assert_eq!(root_counts, [0, 1, 2, 2]);
    }

    #[test]
    fn prepared_pixel_failures_and_per_query_budget_remain_explicit() {
        let (rig, limits, config, sheet) = fixture();
        let matcher = PreparedMatcher::new(&rig, limits).unwrap();
        let traces = [trace(290.)];
        let sheets = [sheet];
        for maximum_work in [2, 3] {
            let queries = matcher
                .prepare_traces(
                    &traces,
                    &sheets,
                    Config {
                        maximum_work,
                        ..config
                    },
                    &mut || false,
                )
                .unwrap();
            assert!(queries.prepare_pixel([340., 250.], &mut || true).is_err());
            assert!(
                queries
                    .prepare_pixel([340., 250.], &mut || false)
                    .unwrap()
                    .execute(&mut || true)
                    .is_err()
            );
            for pixel in [[f64::NAN, 250.], [99999., 250.], [340., 250.]] {
                let reference = queries.match_pixel(pixel, &mut || false);
                let actual = queries
                    .prepare_pixel(pixel, &mut || false)
                    .and_then(|p| p.execute(&mut || false));
                assert_eq!(
                    serde_json::to_value(actual).unwrap(),
                    serde_json::to_value(reference).unwrap()
                );
            }
        }
    }

    #[test]
    fn known_depth_and_unseeded_distractor_preserve_all_crossings() {
        let (rig, limits, config, sheet) = fixture();
        let matcher = PreparedMatcher::new(&rig, limits).unwrap();
        let result = matcher
            .match_pixel(
                [340., 250.],
                &[trace(270.), trace(290.)],
                &[sheet],
                config,
                &mut || false,
            )
            .unwrap();
        assert_eq!(
            result.unique,
            Some(MatchIndex {
                trace_index: 1,
                crossing_index: 0,
                support_index: 0
            })
        );
        assert_eq!(result.supported_combinations, 1);
        assert_eq!(result.traces.len(), 2);
        assert_eq!(result.traces[0].crossings.len(), 1);
        assert!(result.traces[0].crossings[0].triangulation.is_ok());
        assert!(result.traces[0].crossings[0].support.is_empty());
        let crossing = &result.traces[1].crossings[0];
        for (a, b) in crossing
            .triangulation
            .as_ref()
            .unwrap()
            .position_mm
            .into_iter()
            .zip([20., 10., 500.])
        {
            assert!((a - b).abs() < 1e-8);
        }
        assert!((crossing.intersection.fraction - 0.5).abs() < 1e-10);
        assert_eq!(result.work, 6);
        let shared = vec![[290., 249.], [290., 250.], [290., 251.]];
        let result = matcher
            .match_pixel([340., 250.], &[shared], &[sheet], config, &mut || false)
            .unwrap();
        assert_eq!(result.traces[0].crossings.len(), 1);
        assert_eq!(
            result.traces[0].crossings[0].intersection.vertex_index,
            Some(1)
        );
        assert!(result.unique.is_some());
    }

    #[test]
    fn duplicated_hypotheses_and_coincident_traces_are_not_unique() {
        let (rig, limits, config, sheet) = fixture();
        let matcher = PreparedMatcher::new(&rig, limits).unwrap();
        for (traces, sheets) in [
            (vec![trace(290.), trace(290.)], vec![sheet]),
            (vec![trace(290.)], vec![sheet, sheet]),
        ] {
            let result = matcher
                .match_pixel([340., 250.], &traces, &sheets, config, &mut || false)
                .unwrap();
            assert_eq!(result.supported_combinations, 2);
            assert!(result.unique.is_none());
        }
        let result = matcher
            .match_pixel(
                [340., 250.],
                &[trace(290.), vec![[289., 250.], [291., 250.]]],
                &[sheet],
                config,
                &mut || false,
            )
            .unwrap();
        assert_eq!(result.supported_combinations, 1);
        assert!(result.unique.is_none());
        assert_eq!(result.traces[1].ambiguous_segments, vec![0]);
    }

    #[test]
    fn depth_rejection_and_missing_support_remain_visible() {
        let (rig, mut limits, mut config, sheet) = fixture();
        limits.min_depth_z_mm = 450.;
        limits.max_depth_z_mm = 550.;
        config.maximum_sheet_distance_px = 32.;
        let matcher = PreparedMatcher::new(&rig, limits).unwrap();
        let result = matcher
            .match_pixel(
                [340., 250.],
                &[trace(340. - 25000. / 600.)],
                &[sheet],
                config,
                &mut || false,
            )
            .unwrap();
        assert!(result.unique.is_none());
        assert_eq!(result.supported_combinations, 0);
        assert!(result.traces[0].crossings[0].triangulation.is_err());
        assert_eq!(result.traces[0].crossings[0].support.len(), 1);
        let absent = QuadraticSheet {
            offset: 80.,
            ..sheet
        };
        let result = matcher
            .match_pixel([340., 250.], &[trace(290.)], &[absent], config, &mut || {
                false
            })
            .unwrap();
        assert!(result.projections.is_empty());
        assert!(result.unique.is_none());
        assert!(result.traces[0].crossings[0].triangulation.is_ok());
    }

    #[test]
    fn both_quadratic_roots_remain_separate_support_hypotheses() {
        let (rig, limits, mut config, _) = fixture();
        config.maximum_sheet_distance_px = 32.;
        // Along the supplied ray this gives 0.0004*(Z-400)*(Z-600).
        let sheet = QuadraticSheet {
            quadratic_y: 1.,
            linear_y: 0.,
            linear_z: -0.36,
            offset: 96.,
        };
        let result = PreparedMatcher::new(&rig, limits)
            .unwrap()
            .match_pixel([340., 250.], &[trace(290.)], &[sheet], config, &mut || {
                false
            })
            .unwrap();
        assert_eq!(result.projections.len(), 2);
        for (root, expected) in result.projections.iter().zip([400., 600.]) {
            assert!((root.first_camera_point[2] - expected).abs() < 1e-8);
        }
        assert_eq!(result.projections[0].root_index, 0);
        assert_eq!(result.projections[1].root_index, 1);
        assert_eq!(result.traces[0].crossings[0].support.len(), 2);
        assert_eq!(result.supported_combinations, 2);
        assert!(result.unique.is_none());
    }

    #[test]
    fn distorted_skewed_rotated_geometry_and_curved_sheet_recover_known_point() {
        let (mut rig, limits, config, _) = fixture();
        let angle = 0.03_f64;
        rig.right_from_left.rotation = [
            [angle.cos(), 0., angle.sin()],
            [0., 1., 0.],
            [-angle.sin(), 0., angle.cos()],
        ];
        rig.left.skew = 2.;
        rig.right.skew = -3.;
        rig.left.distortion = Brown5 {
            k1: 0.05,
            k2: -0.002,
            p1: 0.001,
            p2: -0.001,
            k3: 0.0001,
        };
        rig.right.distortion = rig.left.distortion;
        let pixel = rig.left.project([20., 10., 500.]).unwrap();
        let trace = [8., 12.]
            .map(|y| {
                rig.right
                    .project(rig.right_from_left.apply([20., y, 500.]))
                    .unwrap()
            })
            .to_vec();
        let sheet = QuadraticSheet {
            quadratic_y: 0.001,
            linear_y: 0.05,
            linear_z: 0.02,
            offset: 9.4,
        };
        let result = PreparedMatcher::new(&rig, limits)
            .unwrap()
            .match_pixel(pixel, &[trace], &[sheet], config, &mut || false)
            .unwrap();
        assert!(result.unique.is_some());
        for (a, b) in result.traces[0].crossings[0]
            .triangulation
            .as_ref()
            .unwrap()
            .position_mm
            .into_iter()
            .zip([20., 10., 500.])
        {
            assert!((a - b).abs() < 1e-7);
        }
    }

    #[test]
    fn cancellation_invalid_models_and_budgets_fail_without_partial_selection() {
        let (rig, limits, config, sheet) = fixture();
        let matcher = PreparedMatcher::new(&rig, limits).unwrap();
        assert!(
            matcher
                .match_pixel([340., 250.], &[trace(290.)], &[sheet], config, &mut || true)
                .is_err()
        );
        let mut calls = 0;
        assert!(
            matcher
                .match_pixel([340., 250.], &[trace(290.)], &[sheet], config, &mut || {
                    calls += 1;
                    calls > 7
                })
                .is_err()
        );
        for work in [0, 2, 10_000_001] {
            assert!(
                matcher
                    .match_pixel(
                        [340., 250.],
                        &[trace(290.)],
                        &[sheet],
                        Config {
                            maximum_work: work,
                            ..config
                        },
                        &mut || false
                    )
                    .is_err()
            );
        }
        for traces in [
            vec![],
            vec![trace(290.); 129],
            vec![vec![[290., 250.]; 4097]],
            vec![vec![[290., 250.]; 4096]; 17],
            vec![trace(290.), vec![[0., 0.], [100., 100.]]],
        ] {
            assert!(
                matcher
                    .match_pixel([340., 250.], &traces, &[sheet], config, &mut || false)
                    .is_err()
            );
        }
        for sheets in [
            vec![],
            vec![sheet; 129],
            vec![QuadraticSheet {
                offset: f64::NAN,
                ..sheet
            }],
            vec![QuadraticSheet {
                offset: 0.,
                linear_z: 0.04,
                ..sheet
            }],
        ] {
            assert!(
                matcher
                    .match_pixel([340., 250.], &[trace(290.)], &sheets, config, &mut || false)
                    .is_err()
            );
        }
    }

    #[test]
    fn cached_results_preserve_all_crossings_errors_and_ambiguity() {
        let (mut rig, limits, config, sheet) = fixture();
        rig.left.skew = 2.;
        rig.right.skew = -3.;
        rig.left.distortion = Brown5 {
            k1: 0.05,
            k2: -0.002,
            p1: 0.001,
            p2: -0.001,
            k3: 0.0001,
        };
        rig.right.distortion = rig.left.distortion;
        let matcher = PreparedMatcher::new(&rig, limits).unwrap();
        let traces_cases = [
            vec![trace(270.), trace(290.)],
            vec![trace(290.), trace(290.)],
            vec![vec![[289., 250.], [291., 250.]], trace(290.)],
            vec![vec![[290., 249.], [290., 250.], [290., 251.]]],
            vec![vec![[290., 249.], [290., 279.]]],
            vec![vec![[f64::NAN, 249.], [290., 251.]]],
        ];
        for traces in traces_cases {
            for sheets in [
                vec![sheet],
                vec![sheet, sheet],
                vec![QuadraticSheet {
                    offset: 80.,
                    ..sheet
                }],
                vec![QuadraticSheet {
                    offset: f64::NAN,
                    ..sheet
                }],
            ] {
                let cached = matcher
                    .prepare_traces(&traces, &sheets, config, &mut || false)
                    .unwrap();
                for pixel in [[340., 250.], [340., 249.], [340., 280.], [-10., 250.]] {
                    let expected =
                        matcher.match_pixel(pixel, &traces, &sheets, config, &mut || false);
                    let actual = cached.match_pixel(pixel, &mut || false);
                    assert_eq!(
                        serde_json::to_value(actual).unwrap(),
                        serde_json::to_value(expected).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn cache_preserves_budgets_and_cancellation_in_preparation_and_queries() {
        let (rig, limits, config, sheet) = fixture();
        let matcher = PreparedMatcher::new(&rig, limits).unwrap();
        let traces = [trace(290.)];
        let sheets = [sheet];
        assert!(
            matcher
                .prepare_traces(&traces, &sheets, config, &mut || true)
                .is_err()
        );
        let mut calls = 0;
        // A one-shot cancellation must not become a cached numerical error.
        assert!(
            matcher
                .prepare_traces(&traces, &sheets, config, &mut || {
                    calls += 1;
                    calls == 3
                })
                .is_err()
        );
        for budget in [1, 2, 3] {
            let config = Config {
                maximum_work: budget,
                ..config
            };
            let expected =
                matcher.match_pixel([340., 250.], &traces, &sheets, config, &mut || false);
            let actual = match matcher.prepare_traces(&traces, &sheets, config, &mut || false) {
                Ok(cache) => cache.match_pixel([340., 250.], &mut || false),
                Err(e) => Err(e),
            };
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                serde_json::to_value(expected).unwrap()
            );
        }
        let cache = matcher
            .prepare_traces(&traces, &sheets, config, &mut || false)
            .unwrap();
        assert!(cache.match_pixel([340., 250.], &mut || true).is_err());
        let mut calls = 0;
        assert!(
            cache
                .match_pixel([340., 250.], &mut || {
                    calls += 1;
                    calls == 5
                })
                .is_err()
        );
        assert!(
            cache
                .match_pixel([340., 250.], &mut || false)
                .unwrap()
                .unique
                .is_some()
        );
    }

    #[test]
    fn separately_prepared_geometry_and_trace_sets_do_not_share_cached_rays() {
        let (rig, limits, config, sheet) = fixture();
        let mut other = rig.clone();
        other.right.fx *= 1.5;
        other.right_from_left.translation_mm[0] *= 1.2;
        let first = PreparedMatcher::new(&rig, limits).unwrap();
        let second = PreparedMatcher::new(&other, limits).unwrap();
        let traces = [trace(290.)];
        let sheets = [sheet];
        let a = first
            .prepare_traces(&traces, &sheets, config, &mut || false)
            .unwrap();
        let b = second
            .prepare_traces(&traces, &sheets, config, &mut || false)
            .unwrap();
        let actual_a =
            serde_json::to_value(a.match_pixel([340., 250.], &mut || false).unwrap()).unwrap();
        let actual_b =
            serde_json::to_value(b.match_pixel([340., 250.], &mut || false).unwrap()).unwrap();
        assert_eq!(
            actual_a,
            serde_json::to_value(
                first
                    .match_pixel([340., 250.], &traces, &sheets, config, &mut || false)
                    .unwrap()
            )
            .unwrap()
        );
        assert_eq!(
            actual_b,
            serde_json::to_value(
                second
                    .match_pixel([340., 250.], &traces, &sheets, config, &mut || false)
                    .unwrap()
            )
            .unwrap()
        );
        assert_ne!(actual_a, actual_b);
    }
}
