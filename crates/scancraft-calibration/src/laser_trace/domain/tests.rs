use super::*;
use crate::laser_trace::tests::fixture;

#[test]
fn valid_only_matches_are_identical_including_ambiguity_and_work() {
    let (rig, limits, config, sheet) = fixture();
    let matcher = PreparedMatcher::new(&rig, limits).unwrap();
    for traces in [
        vec![vec![[290., 249.], [290., 251.]]],
        vec![
            vec![[289., 249.], [291., 251.], [289., 249.]],
            vec![[289., 250.], [291., 250.]],
        ],
    ] {
        for sheets in [
            vec![sheet],
            vec![sheet, sheet],
            vec![QuadraticSheet {
                offset: 80.,
                ..sheet
            }],
        ] {
            let strict = matcher
                .prepare_traces(&traces, &sheets, config, &mut || false)
                .unwrap()
                .match_pixel([340., 250.], &mut || false)
                .unwrap();
            let domain = matcher
                .prepare_domain_traces(&traces, &sheets, config, &mut || false)
                .unwrap();
            assert!(domain.rejections().vertices.is_empty());
            let query = domain.prepare_pixel([340., 250.], &mut || false);
            let bound = query.work_bound();
            let output = query.execute(&mut || false);
            assert!(output.rejected_projections.is_empty());
            let matched = output.result.unwrap();
            assert!(matched.work <= bound);
            assert_eq!(
                serde_json::to_value(matched).unwrap(),
                serde_json::to_value(strict).unwrap()
            );
        }
    }
}

#[test]
fn rejected_vertices_split_runs_preserve_indices_and_never_bridge_holes() {
    let (mut rig, limits, mut config, sheet) = fixture();
    rig.right.valid_radius = 0.1;
    config.intersection.maximum_segment_length_px = 100.;
    let matcher = PreparedMatcher::new(&rig, limits).unwrap();
    let sheets = [sheet];
    let traces = vec![
        vec![[290., 249.], [260., 250.], [290., 251.]], // hole across epipolar line
        vec![[260., 250.], [290., 249.], [290., 251.]], // valid run offset=1
    ];
    assert!(
        matcher
            .prepare_traces(&traces, &sheets, config, &mut || false)
            .unwrap()
            .match_pixel([340., 250.], &mut || false)
            .is_err()
    );
    let domain = matcher
        .prepare_domain_traces(&traces, &sheets, config, &mut || false)
        .unwrap();
    let rejected = domain.rejections();
    assert_eq!(
        rejected
            .vertices
            .iter()
            .map(|v| (v.trace_index, v.vertex_index))
            .collect::<Vec<_>>(),
        [(0, 1), (1, 0)]
    );
    assert_eq!(
        rejected
            .segments
            .iter()
            .map(|v| (v.trace_index, v.segment_index, v.rejected_vertices.clone()))
            .collect::<Vec<_>>(),
        [(0, 0, vec![1]), (0, 1, vec![1]), (1, 0, vec![0])]
    );
    assert_eq!(
        rejected
            .valid_runs
            .iter()
            .map(|v| (v.trace_index, v.first_vertex, v.vertex_count))
            .collect::<Vec<_>>(),
        [(0, 0, 1), (0, 2, 1), (1, 1, 2)]
    );
    let result = domain
        .prepare_pixel([340., 250.], &mut || false)
        .execute(&mut || false)
        .result
        .unwrap();
    assert!(result.traces[0].crossings.is_empty());
    assert_eq!(result.unique.unwrap().trace_index, 1);
    assert_eq!(result.traces[1].crossings[0].intersection.segment_index, 1);
    // A root exactly at an original vertex is also rebased, not renumbered as zero.
    let result = domain
        .prepare_pixel([340., 249.], &mut || false)
        .execute(&mut || false)
        .result
        .unwrap();
    assert_eq!(
        result.traces[1].crossings[0].intersection.vertex_index,
        Some(1)
    );
}

#[test]
fn excluded_roots_are_recorded_and_all_valid_alternatives_remain() {
    let (mut rig, limits, config, sheet) = fixture();
    rig.right.valid_radius = 0.1;
    let matcher = PreparedMatcher::new(&rig, limits).unwrap();
    let traces = vec![vec![[290., 249.], [290., 251.]]];
    let bad = QuadraticSheet {
        offset: 4.,
        ..sheet
    };
    for sheets in [vec![bad, sheet], vec![bad, sheet, sheet]] {
        let domain = matcher
            .prepare_domain_traces(&traces, &sheets, config, &mut || false)
            .unwrap();
        let query = domain.prepare_pixel([340., 250.], &mut || false);
        let bound = query.work_bound();
        let output = query.execute(&mut || false);
        assert_eq!(output.rejected_projections.len(), 1);
        let rejected = &output.rejected_projections[0];
        assert_eq!((rejected.sheet_index, rejected.root_index), (0, 0));
        assert_eq!(
            rejected.error,
            "point is outside the calibrated lens domain"
        );
        let result = output.result.unwrap();
        assert_eq!(result.projections.len(), sheets.len() - 1);
        assert_eq!(result.projections[0].sheet_index, 1);
        assert_eq!(result.supported_combinations, sheets.len() - 1);
        assert_eq!(result.unique.is_some(), sheets.len() == 2);
        assert!(result.work <= bound);
    }
}

#[test]
fn malformed_inputs_limits_and_first_ray_errors_still_fail() {
    let (mut rig, limits, config, sheet) = fixture();
    rig.left.valid_radius = 0.1;
    rig.right.valid_radius = 0.1;
    let matcher = PreparedMatcher::new(&rig, limits).unwrap();
    let sheets = [sheet];
    for traces in [
        vec![vec![[290., 249.], [290., 249.]]],
        vec![vec![[290., 249.], [260., 250.]]], // oversized even though invalid inverse
        vec![vec![[290., 249.], [f64::NAN, 250.]]],
        vec![vec![[290., 249.], [-1., 250.]]],
    ] {
        assert!(
            matcher
                .prepare_domain_traces(&traces, &sheets, config, &mut || false)
                .is_err()
        );
    }
    let traces = vec![vec![[290., 249.], [290., 251.]]];
    let domain = matcher
        .prepare_domain_traces(&traces, &sheets, config, &mut || false)
        .unwrap();
    let query = domain.prepare_pixel([500., 250.], &mut || false);
    assert!(query.preparation_failed());
    assert_eq!(query.work_bound(), 0);
    assert!(query.execute(&mut || false).result.is_err());
    let budget = Config {
        maximum_work: 2,
        ..config
    };
    let domain = matcher
        .prepare_domain_traces(&traces, &sheets, budget, &mut || false)
        .unwrap();
    assert!(
        domain
            .prepare_pixel([340., 250.], &mut || false)
            .execute(&mut || false)
            .result
            .unwrap_err()
            .contains("work limit")
    );
    assert!(
        matcher
            .prepare_domain_traces(
                &traces,
                &sheets,
                Config {
                    maximum_work: 1,
                    ..config
                },
                &mut || false
            )
            .is_err()
    );
}

#[test]
fn cancellation_pulses_cannot_be_hidden_as_domain_rejections() {
    let (rig, limits, config, sheet) = fixture();
    let matcher = PreparedMatcher::new(&rig, limits).unwrap();
    let traces = vec![vec![[290., 249.], [290., 251.]]];
    let sheets = [sheet];
    let mut count = 0;
    matcher
        .prepare_domain_traces(&traces, &sheets, config, &mut || {
            count += 1;
            false
        })
        .unwrap();
    for pulse in 1..=count {
        let mut seen = 0;
        assert!(
            matcher
                .prepare_domain_traces(&traces, &sheets, config, &mut || {
                    seen += 1;
                    seen == pulse
                })
                .is_err()
        );
    }
    let domain = matcher
        .prepare_domain_traces(&traces, &sheets, config, &mut || false)
        .unwrap();
    let mut count = 0;
    domain.prepare_pixel([340., 250.], &mut || {
        count += 1;
        false
    });
    for pulse in 1..=count {
        let mut seen = 0;
        assert!(
            domain
                .prepare_pixel([340., 250.], &mut || {
                    seen += 1;
                    seen == pulse
                })
                .execute(&mut || false)
                .result
                .unwrap_err()
                .contains("cancelled")
        );
    }
    let mut count = 0;
    domain
        .prepare_pixel([340., 250.], &mut || false)
        .execute(&mut || {
            count += 1;
            false
        })
        .result
        .unwrap();
    for pulse in 1..=count {
        let mut seen = 0;
        assert!(
            domain
                .prepare_pixel([340., 250.], &mut || false)
                .execute(&mut || {
                    seen += 1;
                    seen == pulse
                })
                .result
                .unwrap_err()
                .contains("cancelled")
        );
    }
}
