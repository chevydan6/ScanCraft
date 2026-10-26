use scancraft_geometry::{Cloud, Editor, Point, statistical_filter, work::WorkControl};
fn cloud(points: Vec<[f64; 3]>) -> Cloud {
    Cloud {
        points: points
            .into_iter()
            .enumerate()
            .map(|(i, position)| Point {
                position,
                color: Some([i as u8, 180, 240]),
                normal: Some([0., 0., 1.]),
            })
            .collect(),
    }
}
#[test]
fn analytic_sample_statistics_exclude_self_keep_equality_and_preserve_attributes() {
    let c = cloud(vec![[0., 0., 0.], [1., 0., 0.], [10., 0., 0.]]);
    let original = c.clone();
    let work = WorkControl::default();
    let (filtered, r) = statistical_filter::filter(&c, 1, 1., &work).unwrap();
    assert_eq!(r.mean_neighbor_distance_mm, vec![1., 1., 9.]);
    assert!((r.global_mean_mm - 11. / 3.).abs() < 1e-14);
    assert!((r.sample_standard_deviation_mm - (64f64 / 3.).sqrt()).abs() < 1e-14);
    assert_eq!(r.outlier_indices, vec![2]);
    assert_eq!(filtered.points, c.points[..2]);
    assert_eq!(c, original);
    let duplicates = cloud(vec![[0.; 3]; 5]);
    let r = statistical_filter::analyze(&duplicates, 4, 0., &work).unwrap();
    assert_eq!(r.threshold_mm, 0.);
    assert!(r.outlier_indices.is_empty());
    let pair = cloud(vec![[0.; 3], [1., 0., 0.]]);
    assert!(
        statistical_filter::analyze(&pair, 1, 0., &work)
            .unwrap()
            .outlier_indices
            .is_empty()
    );
}
#[test]
fn labeled_dense_patch_is_preserved_and_removal_has_exact_project_undo() {
    let mut positions: Vec<_> = (0..10)
        .flat_map(|x| (0..10).map(move |y| [x as f64, y as f64, 0.]))
        .collect();
    positions.extend([[100., 100., 100.], [-100., -100., -100.]]);
    let c = cloud(positions);
    let (filtered, r) = statistical_filter::filter(&c, 4, 1., &WorkControl::default()).unwrap();
    assert_eq!(r.outlier_indices, vec![100, 101]);
    assert_eq!(filtered.points, c.points[..100]);
    let mut e = Editor::new(c.clone(), 8).unwrap();
    e.replace(filtered.clone()).unwrap();
    let mut bytes = Vec::new();
    scancraft_geometry::formats::save_project(&e, &mut bytes).unwrap();
    let mut loaded = scancraft_geometry::formats::load_project(bytes.as_slice()).unwrap();
    assert!(loaded.undo());
    assert_eq!(loaded.cloud, c);
    assert!(loaded.redo());
    assert_eq!(loaded.cloud, filtered);
}
#[test]
fn invalid_parameters_cancellation_and_distance_overflow_return_no_result() {
    let c = cloud(vec![[0.; 3], [1., 0., 0.], [2., 0., 0.]]);
    let work = WorkControl::default();
    for (k, sigma) in [
        (0, 1.),
        (3, 1.),
        (1025, 1.),
        (1, -1.),
        (1, f64::INFINITY),
        (1, f64::NAN),
    ] {
        assert!(statistical_filter::analyze(&c, k, sigma, &work).is_err());
    }
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(statistical_filter::analyze(&c, 1, 1., &cancelled).is_err());
    let extreme = cloud(vec![[1e308, 0., 0.], [-1e308, 0., 0.]]);
    assert!(statistical_filter::analyze(&extreme, 1, 1., &work).is_err());
    assert!(statistical_filter::analyze(&cloud(vec![]), 1, 1., &work).is_err());
}
