use scancraft_geometry::{Cloud, Point, coordinate_frame::Frame, work::WorkControl};
fn point(position: [f64; 3]) -> Point {
    Point {
        position,
        color: Some([1, 2, 3]),
        normal: Some([2., 0., 0.]),
    }
}
#[test]
fn explicit_frame_is_right_handed_and_rotates_positions_and_normals() {
    let frame = Frame::from_points([10., 20., 30.], [10., 22., 30.], [9., 21., 30.]).unwrap();
    let source = Cloud {
        points: vec![point([10., 20., 30.]), point([12., 23., 34.])],
    };
    let original = source.clone();
    let moved = frame.apply(&source, &WorkControl::default()).unwrap();
    assert_eq!(source, original);
    assert_eq!(moved.points[0].position, [0.; 3]);
    assert_eq!(moved.points[1].position, [3., -2., 4.]);
    assert_eq!(moved.points[1].normal, Some([0., -2., 0.]));
    assert_eq!(moved.points[1].color, Some([1, 2, 3]));
    let before =
        scancraft_geometry::distance_mm(source.points[0].position, source.points[1].position)
            .unwrap();
    let after = scancraft_geometry::distance_mm(moved.points[0].position, moved.points[1].position)
        .unwrap();
    assert!((before - after).abs() < 1e-12);
    assert_eq!(frame.axes, [[0., 1., 0.], [-1., 0., 0.], [0., 0., 1.]]);
}
#[test]
fn origin_only_preserves_normal_bits_missing_attributes_and_small_local_offsets() {
    let origin = [1e12, -2e12, 3e12];
    let normal = [-0., f64::from_bits(1), -2.];
    let source = Cloud {
        points: vec![
            Point {
                position: [origin[0] + 1., origin[1] + 2., origin[2] + 3.],
                color: None,
                normal: Some(normal),
            },
            Point {
                position: origin,
                color: None,
                normal: None,
            },
        ],
    };
    let moved = Frame::at_origin(origin)
        .unwrap()
        .apply(&source, &WorkControl::default())
        .unwrap();
    assert_eq!(moved.points[0].position, [1., 2., 3.]);
    assert_eq!(
        moved.points[0].normal.unwrap().map(f64::to_bits),
        normal.map(f64::to_bits)
    );
    assert!(moved.points[1].normal.is_none());
    assert_eq!(moved.points[1].position, [0.; 3]);
}
#[test]
fn reference_scales_are_independent_and_numerically_degenerate_frames_fail() {
    let f = Frame::from_points([0.; 3], [1e300, 0., 0.], [0., 1e-300, 0.]).unwrap();
    assert_eq!(f.axes, [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
    for (x, xy) in [
        ([0.; 3], [0., 1., 0.]),
        ([1., 0., 0.], [2., 0., 0.]),
        ([1., 0., 0.], [1., 1e-9, 0.]),
        ([f64::NAN, 0., 0.], [0., 1., 0.]),
    ] {
        assert!(Frame::from_points([0.; 3], x, xy).is_err());
    }
    assert!(
        Frame {
            origin_mm: [0.; 3],
            axes: [[-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
        }
        .validate()
        .is_err()
    );
}
#[test]
fn cancellation_and_overflow_do_not_change_the_source() {
    let source = Cloud {
        points: vec![point([-f64::MAX, 0., 0.])],
    };
    let before = source.clone();
    assert!(
        Frame::at_origin([f64::MAX, 0., 0.])
            .unwrap()
            .apply(&source, &WorkControl::default())
            .is_err()
    );
    assert_eq!(source, before);
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(
        Frame::at_origin([0.; 3])
            .unwrap()
            .apply(&source, &cancelled)
            .is_err()
    );
    assert_eq!(source, before);
}
#[test]
fn oblique_frame_preserves_nonunit_normal_length() {
    let frame = Frame::from_points([1., 2., 3.], [2., 3., 3.], [0., 3., 3.]).unwrap();
    let source = Cloud {
        points: vec![point([3., 2., 3.])],
    };
    let moved = frame.apply(&source, &WorkControl::default()).unwrap();
    let p = &moved.points[0];
    assert!((p.position[0] - 2_f64.sqrt()).abs() < 1e-12);
    assert!((p.position[1] + 2_f64.sqrt()).abs() < 1e-12);
    assert_eq!(p.position[2], 0.);
    let n = p.normal.unwrap();
    assert!((n[0].hypot(n[1]).hypot(n[2]) - 2.).abs() < 1e-12);
}
