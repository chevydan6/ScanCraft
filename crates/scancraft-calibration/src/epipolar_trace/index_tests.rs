//! Scalar-versus-indexed regressions, with no physical-calibration claim.
use super::{
    tests::{config, rig},
    *,
};

fn same(a: Result<Intersections, String>, b: Result<Intersections, String>) {
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
}

#[test]
fn indexed_preserves_varied_long_distorted_and_reversed_traces() {
    for distorted in [false, true] {
        let mut calibration = rig(distorted);
        let (s, c) = 0.05_f64.sin_cos();
        calibration.right_from_left.rotation = [[c, 0., s], [0., 1., 0.], [-s, 0., c]];
        calibration.right_from_left.translation_mm = [-80., 3., 6.];
        let prepared = PreparedTrace::new(&calibration).unwrap();
        for len in [127, 128, 129, 257, 4096] {
            let trace = (0..len)
                .map(|i| {
                    let t = i as f64 / (len - 1) as f64;
                    calibration
                        .right
                        .project([-0.15 + 0.3 * t, 0.1 * (t * 23.).sin(), 1.])
                        .unwrap()
                })
                .collect::<Vec<_>>();
            for reversed in [false, true] {
                let trace = if reversed {
                    trace.iter().copied().rev().collect()
                } else {
                    trace.clone()
                };
                let normalized = prepared
                    .prepare_indexed_trace(&trace, config(), &mut || false)
                    .unwrap();
                assert_eq!(normalized.blocks.is_empty(), len < 128);
                for i in 0..41 {
                    let first = calibration
                        .left
                        .project([0.01, -0.13 + i as f64 * 0.0065, 1.])
                        .unwrap();
                    let line = prepared.line(first, config(), &mut || false).unwrap();
                    same(
                        prepared.intersect_line_linear(line, &normalized, config(), &mut || false),
                        prepared.intersect_line_indexed(line, &normalized, config(), &mut || false),
                    );
                }
            }
        }
    }
}

#[test]
fn shared_block_endpoint_and_coincident_segments_keep_global_lineage() {
    let calibration = rig(false);
    let prepared = PreparedTrace::new(&calibration).unwrap();
    let line = Vector3::new(0., 1., 0.);
    let tolerance = config().normalized_tolerance;
    // Hit vertex64 from both adjacent blocks; retain near-coincident segments
    // across boundary128, another interpolated crossing, and original indices.
    let trace = (0..193)
        .map(|i| {
            let y = match i {
                64 => 0.,
                127..=129 => tolerance / 4.,
                160.. => -0.01,
                _ => 0.01,
            };
            calibration
                .right
                .project([-0.1 + i as f64 * 0.001, y, 1.])
                .unwrap()
        })
        .collect::<Vec<_>>();
    let normalized = prepared
        .prepare_indexed_trace(&trace, config(), &mut || false)
        .unwrap();
    let scalar = prepared.intersect_line_linear(line, &normalized, config(), &mut || false);
    let indexed = prepared.intersect_line_indexed(line, &normalized, config(), &mut || false);
    let result = indexed.as_ref().unwrap();
    assert_eq!(result.ambiguous_segments, vec![127, 128]);
    assert_eq!(
        result
            .intersections
            .iter()
            .filter(|p| p.vertex_index == Some(64))
            .count(),
        1
    );
    assert!(
        result
            .intersections
            .iter()
            .any(|p| p.segment_index == 159 && p.vertex_index.is_none())
    );
    same(scalar, indexed);
}

#[test]
fn risky_bounds_fall_back_before_any_result_or_numerical_error() {
    let calibration = rig(false);
    let prepared = PreparedTrace::new(&calibration).unwrap();
    let raw = vec![[100., 100.]; 129];
    // Internal adversarial normalized values exercise overflow ordering without
    // pretending these could pass public camera-domain preparation.
    let mut points = vec![Vector3::new(0., 1., 1.); 129];
    points[0].y = -1.;
    points[0].x = 1e10;
    points[1].x = 1e10;
    points[128].y = f64::MAX;
    let normalized = NormalizedTrace {
        raw: &raw,
        points,
        blocks: vec![
            Block {
                first: 0,
                end: 64,
                min: [0., -1.],
                max: [1e10, 1.],
            },
            Block {
                first: 64,
                end: 128,
                min: [0., 1.],
                max: [0., f64::MAX],
            },
        ],
    };
    for line in [Vector3::new(0., 1., 0.), Vector3::new(0., 2., 0.)] {
        let scalar = prepared.intersect_line_linear(line, &normalized, config(), &mut || false);
        let expected_error = if line.y == 1. {
            "point is outside the calibrated lens domain"
        } else {
            "epipolar trace coordinate overflow"
        };
        assert_eq!(scalar.as_ref().unwrap_err(), expected_error);
        same(
            scalar,
            prepared.intersect_line_indexed(line, &normalized, config(), &mut || false),
        );
    }
}

#[test]
fn indexed_cancellation_checks_cover_preparation_skipped_and_retained_work() {
    let calibration = rig(false);
    let prepared = PreparedTrace::new(&calibration).unwrap();
    let trace = (0..4096)
        .map(|i| {
            calibration
                .right
                .project([-0.1 + i as f64 * 0.00004, 0.01, 1.])
                .unwrap()
        })
        .collect::<Vec<_>>();
    let mut calls = 0;
    let normalized = prepared
        .prepare_indexed_trace(&trace, config(), &mut || {
            calls += 1;
            false
        })
        .unwrap();
    let prepare_calls = calls;
    for stop in [1, 2048, 4099, prepare_calls] {
        let mut calls = 0;
        assert!(
            prepared
                .prepare_indexed_trace(&trace, config(), &mut || {
                    calls += 1;
                    calls >= stop
                })
                .is_err()
        );
    }
    for line in [Vector3::new(0., 1., 0.), Vector3::new(0., 1., -0.01)] {
        let mut calls = 0;
        prepared
            .intersect_line_indexed(line, &normalized, config(), &mut || {
                calls += 1;
                false
            })
            .unwrap();
        let total = calls;
        // The first line skips every block, yet checks classification, each skip
        // and final completion. Coincident line retains every scalar segment.
        assert!(total > 2 * normalized.blocks.len());
        for stop in [
            1,
            normalized.blocks.len(),
            normalized.blocks.len() + 1,
            total / 2,
            total,
        ] {
            let mut calls = 0;
            assert!(
                prepared
                    .intersect_line_indexed(line, &normalized, config(), &mut || {
                        calls += 1;
                        calls >= stop
                    })
                    .unwrap_err()
                    .contains("cancelled")
            );
            assert_eq!(calls, stop);
        }
    }
}
