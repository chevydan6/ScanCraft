use scancraft_geometry::{measurement::fit_sphere, work::WorkControl};
fn axial(center: [f64; 3], radii: [f64; 3]) -> Vec<[f64; 3]> {
    let mut points = Vec::new();
    for axis in 0..3 {
        for sign in [-1., 1.] {
            let mut point = center;
            point[axis] += sign * radii[axis];
            points.push(point);
        }
    }
    points
}
#[test]
fn known_spheres_small_scales_and_distant_origins_fit_without_source_changes() {
    for (center, radius, tolerance) in [
        ([10., 20., 30.], 5., 1e-10),
        ([1e12, -2e12, 3e12], 5., 1e-3),
        ([0.; 3], 1e-100, 1e-110),
    ] {
        let points = axial(center, [radius; 3]);
        let original = points.clone();
        let fit = fit_sphere(&points, &WorkControl::default()).unwrap();
        for (a, b) in fit.center_mm.into_iter().zip(center) {
            assert!((a - b).abs() < tolerance);
        }
        assert!((fit.radius_mm - radius).abs() < tolerance);
        assert!((fit.diameter_mm - 2. * radius).abs() < 2. * tolerance);
        assert!(fit.geometric_rms_mm < tolerance);
        assert!(fit.maximum_absolute_residual_mm < tolerance);
        assert_eq!(fit.points, 6);
        assert_eq!(points, original);
    }
}
#[test]
fn residual_report_is_geometric_even_when_algebraic_fit_is_not_exact() {
    let radii = [5.01, 4.99, 5.02];
    let points = axial([0.; 3], radii);
    let fit = fit_sphere(&points, &WorkControl::default()).unwrap();
    let expected = (radii.iter().map(|r| r * r).sum::<f64>() / 3.).sqrt();
    assert!((fit.radius_mm - expected).abs() < 1e-10);
    let rms = (radii.iter().map(|r| (r - expected).powi(2)).sum::<f64>() / 3.).sqrt();
    assert!((fit.geometric_rms_mm - rms).abs() < 1e-10);
    assert!(rms > 0.01);
}
#[test]
fn planar_collinear_repeated_nonfinite_and_cancelled_inputs_do_not_return_spheres() {
    for points in [
        vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
        vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [3., 0., 0.]],
        vec![[1.; 3]; 6],
        vec![[f64::NAN, 0., 0.]; 6],
        vec![[0.; 3]; 3],
    ] {
        assert!(fit_sphere(&points, &WorkControl::default()).is_err());
    }
    let work = WorkControl::default();
    work.cancel();
    assert!(fit_sphere(&axial([0.; 3], [5.; 3]), &work).is_err());
}
