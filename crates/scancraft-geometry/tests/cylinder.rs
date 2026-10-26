use scancraft_geometry::{cylinder::fit, work::WorkControl};
fn samples(radius: f64, height: f64) -> Vec<[f64; 3]> {
    [-height / 2., 0., height / 2.]
        .into_iter()
        .flat_map(|z| {
            (0..32).map(move |i| {
                let t = i as f64 * std::f64::consts::TAU / 32.;
                [radius * t.cos(), radius * t.sin(), z]
            })
        })
        .collect()
}
#[test]
fn known_long_and_squat_cylinders_have_geometric_diameter_and_axis() {
    for height in [1., 12., 60.] {
        let points = samples(3., height);
        let result = fit(&points, 1e-7, &WorkControl::default()).unwrap();
        assert!((result.diameter_mm - 6.).abs() < 1e-7);
        assert!(result.axis_direction[2].abs() > 1. - 1e-8);
        assert!(result.geometric_rms_mm < 1e-7);
        assert!(result.starts_converged > 0);
        assert!(result.jacobian_condition_number < 1e8);
        assert!((result.axial_range_mm[1] - result.axial_range_mm[0] - height).abs() < 1e-7);
    }
}
#[test]
fn arbitrary_rigid_pose_and_distant_origin_are_supported() {
    let rotation = nalgebra::Rotation3::from_euler_angles(0.31, -0.52, 0.73);
    let translation = nalgebra::Vector3::new(100., -80., 40.);
    let points: Vec<_> = samples(3., 24.)
        .iter()
        .map(|p| (rotation * nalgebra::Vector3::from(*p) + translation).into())
        .collect();
    let result = fit(&points, 1e-7, &WorkControl::default()).unwrap();
    let expected = rotation * nalgebra::Vector3::new(0., 0., 1.);
    assert!(
        nalgebra::Vector3::from(result.axis_direction)
            .dot(&expected)
            .abs()
            > 1. - 1e-8
    );
    assert!((nalgebra::Vector3::from(result.axis_point_mm) - translation).norm() < 1e-6);
    assert!((result.radius_mm - 3.).abs() < 1e-7);
    let points: Vec<_> = samples(3., 12.)
        .iter()
        .map(|p| [p[0] + 1e9, p[1] - 2e9, p[2] + 3e9])
        .collect();
    let result = fit(&points, 1e-5, &WorkControl::default()).unwrap();
    assert!((result.radius_mm - 3.).abs() < 1e-5);
}
#[test]
fn noisy_radial_residuals_and_rms_gate_are_honest() {
    let mut points = samples(10., 24.);
    for p in &mut points {
        let t = p[1].atan2(p[0]);
        let r = 10. + 0.02 * (5. * t).cos() + 0.01 * (3. * t).sin();
        p[0] = r * t.cos();
        p[1] = r * t.sin();
    }
    let result = fit(&points, 0.02, &WorkControl::default()).unwrap();
    assert!((result.radius_mm - 10.).abs() < 1e-6);
    assert!((result.geometric_rms_mm - (0.00025_f64).sqrt()).abs() < 1e-6);
    assert!(result.maximum_absolute_residual_mm >= result.geometric_rms_mm);
    assert!(fit(&points, 0.001, &WorkControl::default()).is_err());
}
#[test]
fn scaling_invalid_data_degeneracy_and_cancellation_are_explicit() {
    for scale in [1e-80, 1e80] {
        let points: Vec<_> = samples(3., 12.)
            .iter()
            .map(|p| p.map(|v| v * scale))
            .collect();
        let result = fit(&points, scale * 1e-7, &WorkControl::default()).unwrap();
        assert!((result.radius_mm / scale - 3.).abs() < 1e-6);
    }
    let planar: Vec<_> = (0..16)
        .map(|i| {
            let t = i as f64 * std::f64::consts::TAU / 16.;
            [t.cos(), t.sin(), 0.]
        })
        .collect();
    assert!(fit(&planar, 1., &WorkControl::default()).is_err());
    assert!(fit(&[[0.; 3]; 6], 1., &WorkControl::default()).is_err());
    for limit in [-1., f64::NAN, f64::INFINITY] {
        assert!(fit(&samples(3., 12.), limit, &WorkControl::default()).is_err());
    }
    let mut invalid = samples(3., 12.);
    invalid[0][0] = f64::INFINITY;
    assert!(fit(&invalid, 1., &WorkControl::default()).is_err());
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(fit(&samples(3., 12.), 1., &cancelled).is_err());
}
#[test]
fn asymmetric_partial_wall_refines_axis_without_assuming_a_principal_direction() {
    let rotation = nalgebra::Rotation3::from_euler_angles(-0.4, 0.6, 0.2);
    let center = nalgebra::Vector3::new(-4., 5., 6.);
    let expected = rotation * nalgebra::Vector3::new(0., 0., 1.);
    let mut points = Vec::new();
    for base in [-4., -1., 2., 6.] {
        for i in 0..20 {
            let t = 0.3 + 1.7 * i as f64 / 19.;
            let z = base + 0.2 * (2. * t).cos() + 0.15 * t.sin();
            points.push(
                (rotation * nalgebra::Vector3::new(2. * t.cos(), 2. * t.sin(), z) + center).into(),
            );
        }
    }
    let result = fit(&points, 1e-6, &WorkControl::default()).unwrap();
    assert!((result.radius_mm - 2.).abs() < 1e-6);
    assert!(
        nalgebra::Vector3::from(result.axis_direction)
            .dot(&expected)
            .abs()
            > 1. - 1e-8
    );
    let d = nalgebra::Vector3::from(result.axis_point_mm) - center;
    assert!((d - expected * d.dot(&expected)).norm() < 1e-6);
}
