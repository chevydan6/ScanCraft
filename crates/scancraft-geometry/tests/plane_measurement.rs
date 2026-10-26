use scancraft_geometry::{
    Cloud, Editor,
    formats::{load_project, save_project},
    measurement_snapshot::{ResultValue, Snapshot},
    plane_measurement::measure,
    work::WorkControl,
};
fn points() -> Vec<[f64; 3]> {
    vec![
        [-1., -1., 0.],
        [-1., 1., 0.],
        [1., -1., 0.],
        [1., 1., 0.],
        [0., 0., 5.],
    ]
}
#[test]
fn excluded_probe_known_distance_foot_and_sign() {
    let work = WorkControl::default();
    let fit = measure(&points(), 4, 0., &work).unwrap();
    assert_eq!(fit.distance_mm, 5.);
    assert_eq!(fit.signed_distance_mm, 5.);
    assert_eq!(fit.foot_mm, [0.; 3]);
    assert_eq!(fit.normal, [0., 0., 1.]);
    assert_eq!(fit.plane_rms_mm, 0.);
    assert_eq!(fit.plane_points, 4);
    let mut data = points();
    data[4][2] = -5.;
    assert_eq!(
        measure(&data, 4, 0., &work).unwrap().signed_distance_mm,
        -5.
    );
    data.swap(0, 4);
    assert_eq!(
        measure(&data, 0, 0., &work).unwrap().signed_distance_mm,
        -5.
    );
}
#[test]
fn tilted_plane_and_extreme_finite_scales() {
    let mut data = Vec::new();
    for x in [-1., 1.] {
        for y in [-1., 1.] {
            data.push([x, 0.8 * y, -0.6 * y]);
        }
    }
    data.push([0., 3., 4.]);
    let fit = measure(&data, 4, 1e-12, &WorkControl::default()).unwrap();
    assert!((fit.distance_mm - 5.).abs() < 1e-12);
    assert!(fit.foot_mm.iter().all(|v| v.abs() < 1e-12));
    for scale in [1e-100, 1e100, 1e200] {
        let data: Vec<_> = points().iter().map(|p| p.map(|v| v * scale)).collect();
        let fit = measure(&data, 4, 0., &WorkControl::default()).unwrap();
        assert!((fit.distance_mm / scale - 5.).abs() < 1e-12);
    }
    let data: Vec<_> = points()
        .iter()
        .map(|p| std::array::from_fn(|a| p[a] + [1e12, -2e12, 3e12][a]))
        .collect();
    assert_eq!(
        measure(&data, 4, 0., &WorkControl::default())
            .unwrap()
            .distance_mm,
        5.
    );
}
#[test]
fn rms_gate_and_invalid_geometry_are_explicit() {
    let mut noisy = points();
    for (i, p) in noisy[..4].iter_mut().enumerate() {
        p[2] = if i == 0 || i == 3 { 0.1 } else { -0.1 };
    }
    let fit = measure(&noisy, 4, 0.11, &WorkControl::default()).unwrap();
    assert!((fit.plane_rms_mm - 0.1).abs() < 1e-12);
    assert!(measure(&noisy, 4, 0.09, &WorkControl::default()).is_err());
    let mut line = vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [0., 0., 1.]];
    assert!(measure(&line, 3, 1., &WorkControl::default()).is_err());
    line[0][0] = f64::NAN;
    assert!(measure(&line, 3, 1., &WorkControl::default()).is_err());
    let mut isotropic = vec![
        [1., 0., 0.],
        [-1., 0., 0.],
        [0., 1., 0.],
        [0., -1., 0.],
        [0., 0., 1.],
        [0., 0., -1.],
    ];
    isotropic.push([0., 0., 5.]);
    assert!(measure(&isotropic, 6, 100., &WorkControl::default()).is_err());
    for limit in [-1., f64::NAN, f64::INFINITY] {
        assert!(measure(&points(), 4, limit, &WorkControl::default()).is_err());
    }
    assert!(measure(&points(), 99, 0., &WorkControl::default()).is_err());
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(measure(&points(), 4, 0., &cancelled).is_err());
}
#[test]
fn saved_plane_evidence_and_history_reopen_without_current_cloud() {
    let s =
        Snapshot::point_plane("Height".into(), points(), 4, 0., &WorkControl::default()).unwrap();
    let evidence = serde_json::to_vec(&s).unwrap();
    let mut editor = Editor::new(Cloud { points: vec![] }, 8).unwrap();
    editor.add_measurement(s).unwrap();
    editor.remove_measurement(0).unwrap();
    let mut bytes = Vec::new();
    save_project(&editor, &mut bytes).unwrap();
    let mut reopened = load_project(bytes.as_slice()).unwrap();
    reopened.undo_measurement().unwrap();
    assert_eq!(
        serde_json::to_vec(&reopened.measurements()[0]).unwrap(),
        evidence
    );
    let mut invalid = reopened.measurements()[0].clone();
    if let ResultValue::PointPlane { fit, .. } = &mut invalid.result {
        fit.normal = [0.; 3];
    }
    assert!(invalid.validate().is_err());
}
