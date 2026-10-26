use scancraft_geometry::{measurement::fit_circle, work::WorkControl};
#[test]
fn tilted_circles_support_small_scale_and_distant_origins() {
    let h = std::f64::consts::FRAC_1_SQRT_2;
    for (center, radius, tolerance) in [
        ([10., 20., 30.], 5., 1e-9),
        ([1e9, -2e9, 3e9], 5., 1e-4),
        ([0.; 3], 1e-100, 1e-109),
    ] {
        let points: Vec<_> = (0..16)
            .map(|i| {
                let angle = i as f64 * std::f64::consts::TAU / 16.;
                [
                    center[0] + radius * angle.cos(),
                    center[1] + radius * angle.sin() * h,
                    center[2] - radius * angle.sin() * h,
                ]
            })
            .collect();
        let original = points.clone();
        let fit = fit_circle(&points, tolerance, &WorkControl::default()).unwrap();
        assert!((fit.radius_mm - radius).abs() < tolerance);
        assert!(fit.geometric_rms_mm < tolerance);
        for (a, b) in fit.center_mm.into_iter().zip(center) {
            assert!((a - b).abs() < tolerance);
        }
        assert!(((fit.normal[1] + fit.normal[2]) * h).abs() > 0.999999);
        assert_eq!(points, original);
    }
}
#[test]
fn explicit_plane_gate_and_three_dimensional_curve_residual_are_enforced() {
    let points: Vec<_> = (0..16)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 16.;
            [
                5. * a.cos(),
                5. * a.sin(),
                if i % 2 == 0 { 0.2 } else { -0.2 },
            ]
        })
        .collect();
    assert!(fit_circle(&points, 0.1, &WorkControl::default()).is_err());
    let fit = fit_circle(&points, 0.3, &WorkControl::default()).unwrap();
    assert!((fit.diameter_mm - 10.).abs() < 1e-9);
    assert!((fit.plane_rms_mm - 0.2).abs() < 1e-9);
    assert!((fit.geometric_rms_mm - 0.2).abs() < 1e-9);
    assert!((fit.maximum_absolute_residual_mm - 0.2).abs() < 1e-9);
}
#[test]
fn degenerate_invalid_or_cancelled_inputs_return_no_circle() {
    for points in [
        vec![[0.; 3]; 3],
        vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.]],
        vec![[0.; 3]; 2],
        vec![[f64::NAN, 0., 0.]; 4],
    ] {
        assert!(fit_circle(&points, 0.1, &WorkControl::default()).is_err());
    }
    let points = vec![[1., 0., 0.], [0., 1., 0.], [-1., 0., 0.], [0., -1., 0.]];
    assert!(fit_circle(&points, f64::NAN, &WorkControl::default()).is_err());
    let work = WorkControl::default();
    work.cancel();
    assert!(fit_circle(&points, 0.1, &work).is_err());
}
