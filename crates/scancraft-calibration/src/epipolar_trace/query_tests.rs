use super::tests::{config, rig};
use super::*;
fn traces(rig: &StereoCalibration) -> Vec<Vec<[f64; 2]>> {
    let right = rig.right_from_left.apply([40., -30., 500.]);
    let [x, y] = [right[0] / right[2], right[1] / right[2]];
    vec![
        (0..193)
            .map(|i| {
                rig.right
                    .project([x, y + (i as f64 - 96.) * 0.0002, 1.])
                    .unwrap()
            })
            .collect(),
        (0..193)
            .map(|i| {
                rig.right
                    .project([x + (i as f64 - 96.) * 0.0002, y, 1.])
                    .unwrap()
            })
            .collect(),
        (0..33)
            .map(|i| {
                rig.right
                    .project([
                        x + i as f64 * 0.0001,
                        y + if i % 2 == 0 { -0.001 } else { 0.001 },
                        1.,
                    ])
                    .unwrap()
            })
            .collect(),
        vec![[300., 300.], [300., 300.]],
    ]
}
#[test]
fn cached_constraint_preserves_complete_scalar_results_and_per_trace_checkpoints() {
    for distorted in [false, true] {
        let geometry = rig(distorted);
        let prepared = PreparedTrace::new(&geometry).unwrap();
        let pixel = geometry.left.project([40., -30., 500.]).unwrap();
        let query = prepared.for_pixel(pixel, config());
        for (i, raw) in traces(&geometry).iter().enumerate() {
            let trace = prepared.prepare_indexed_trace(raw, config(), &mut || false);
            let mut cached_calls = 0;
            let actual = query.intersections_cached(&trace, &mut || {
                cached_calls += 1;
                false
            });
            let fresh = prepared.for_pixel(pixel, config());
            let mut fresh_calls = 0;
            let expected = fresh.intersections_cached(&trace, &mut || {
                fresh_calls += 1;
                false
            });
            assert_eq!(cached_calls, fresh_calls);
            assert_eq!(
                serde_json::to_vec(&actual).unwrap(),
                serde_json::to_vec(&expected).unwrap()
            );
            let scalar = prepared.intersections(pixel, raw, config(), &mut || false);
            assert_eq!(
                serde_json::to_vec(&actual).unwrap(),
                serde_json::to_vec(&scalar).unwrap()
            );
            if i == 1 {
                assert!(!actual.unwrap().ambiguous_segments.is_empty());
            }
        }
    }
}
#[test]
fn cached_constraint_keeps_pulse_cancellation_and_geometry_before_trace_error() {
    let geometry = rig(true);
    let prepared = PreparedTrace::new(&geometry).unwrap();
    let data = traces(&geometry);
    let trace = prepared.prepare_indexed_trace(&data[0], config(), &mut || false);
    let pixel = geometry.left.project([40., -30., 500.]).unwrap();
    let cached = prepared.for_pixel(pixel, config());
    cached.intersections_cached(&trace, &mut || false).unwrap();
    for stop in 1..512 {
        let mut a = 0;
        let actual = cached.intersections_cached(&trace, &mut || {
            a += 1;
            a == stop
        });
        let fresh = prepared.for_pixel(pixel, config());
        let mut b = 0;
        let expected = fresh.intersections_cached(&trace, &mut || {
            b += 1;
            b == stop
        });
        assert_eq!(a, b);
        assert_eq!(
            serde_json::to_vec(&actual).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
    }
    cached.intersections_cached(&trace, &mut || false).unwrap();
    for pixel in [[f64::NAN, 0.], [-1e6, 0.]] {
        let query = prepared.for_pixel(pixel, config());
        let broken = Err("bad trace preparation".to_owned());
        assert!(
            query
                .intersections_cached(&broken, &mut || false)
                .unwrap_err()
                .contains("outside")
        );
        assert_eq!(
            query
                .intersections_cached(&broken, &mut || true)
                .unwrap_err(),
            "epipolar trace cancelled"
        );
    }
    let mut forward = rig(false);
    forward.right_from_left.translation_mm = [0., 0., 80.];
    let prepared = PreparedTrace::new(&forward).unwrap();
    let query = prepared.for_pixel([forward.left.cx, forward.left.cy], config());
    assert!(
        query
            .intersections_cached(&Err("bad trace preparation".into()), &mut || false)
            .unwrap_err()
            .contains("degenerate")
    );
}
#[test]
fn query_state_is_per_pixel_and_does_not_change_shared_matcher_traits() {
    fn shared<T: Send + Sync>() {}
    shared::<PreparedTrace<'_>>();
    shared::<crate::laser_trace::PreparedMatcher<'_>>();
    let geometry = rig(true);
    let prepared = PreparedTrace::new(&geometry).unwrap();
    let data = traces(&geometry);
    let trace = prepared.prepare_indexed_trace(&data[0], config(), &mut || false);
    let first = prepared.for_pixel(geometry.left.project([40., -30., 500.]).unwrap(), config());
    let expected = first.intersections_cached(&trace, &mut || false).unwrap();
    assert!(!expected.intersections.is_empty());
    let other = prepared.for_pixel(geometry.left.project([-30., 25., 600.]).unwrap(), config());
    assert!(
        other
            .intersections_cached(&trace, &mut || false)
            .unwrap()
            .intersections
            .is_empty()
    );
    assert_eq!(
        serde_json::to_vec(&first.intersections_cached(&trace, &mut || false).unwrap()).unwrap(),
        serde_json::to_vec(&expected).unwrap()
    );
}
