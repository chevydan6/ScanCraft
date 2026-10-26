use scancraft_geometry::{Cloud, Point, normal_edit::flip_selected, work::WorkControl};
fn cloud() -> Cloud {
    Cloud {
        points: vec![
            Point {
                position: [-0., 1., 2.],
                color: Some([1, 2, 3]),
                normal: Some([-0., 2., 0.]),
            },
            Point {
                position: [3., 4., 5.],
                color: None,
                normal: Some([0., 0., -7.]),
            },
            Point {
                position: [6., 7., 8.],
                color: None,
                normal: None,
            },
        ],
    }
}
#[test]
fn explicit_selected_flips_are_bit_exact_involutions_without_geometry_changes() {
    let cloud = cloud();
    let flipped = flip_selected(&cloud, &[1, 0], &WorkControl::default()).unwrap();
    assert_eq!(flipped.points[0].normal.unwrap(), [0., -2., -0.]);
    assert_eq!(flipped.points[1].normal.unwrap(), [-0., -0., 7.]);
    for (original, after) in cloud.points.iter().zip(&flipped.points) {
        assert_eq!(
            original.position.map(f64::to_bits),
            after.position.map(f64::to_bits)
        );
        assert_eq!(original.color, after.color);
    }
    assert_eq!(cloud.points[2], flipped.points[2]);
    let restored = flip_selected(&flipped, &[0, 1], &WorkControl::default()).unwrap();
    for (original, after) in cloud.points.iter().zip(restored.points) {
        assert_eq!(
            original.normal.map(|n| n.map(f64::to_bits)),
            after.normal.map(|n| n.map(f64::to_bits))
        );
    }
    let one = flip_selected(&cloud, &[0], &WorkControl::default()).unwrap();
    assert_eq!(one.points[1], cloud.points[1]);
}
#[test]
fn invalid_selections_missing_zero_normals_and_cancellation_refuse_the_whole_edit() {
    let original = cloud();
    for indices in [vec![], vec![0, 0], vec![0, 99], vec![0, 2]] {
        assert!(flip_selected(&original, &indices, &WorkControl::default()).is_err());
        assert_eq!(original, cloud());
    }
    let mut zero = cloud();
    zero.points[1].normal = Some([0.; 3]);
    assert!(flip_selected(&zero, &[0, 1], &WorkControl::default()).is_err());
    let work = WorkControl::default();
    work.cancel();
    assert!(flip_selected(&original, &[0], &work).is_err());
}
