use nalgebra::{Rotation3, Vector3};
use scancraft_geometry::{
    point_plane::{self, Config},
    registration::Rigid,
    work::WorkControl,
};
type Points = Vec<[f64; 3]>;
fn fixture() -> (Points, Points, Points, Rigid) {
    let pose = Rigid {
        rotation: Rotation3::from_scaled_axis(Vector3::new(0.02, -0.015, 0.025)).into_inner(),
        translation: Vector3::new(0.3, -0.2, 0.4),
    };
    let mut target = Vec::new();
    let mut normals = Vec::new();
    for a in [-6., -2., 2., 6.] {
        for b in [-5., 0., 5.] {
            target.extend([[30., a, b], [a, 40., b], [a, b, 50.]]);
            normals.extend([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        }
    }
    let source = target
        .iter()
        .map(|p| (pose.rotation.transpose() * (Vector3::from(*p) - pose.translation)).into())
        .collect();
    (source, target, normals, pose)
}
fn config() -> Config {
    Config {
        maximum_distance_mm: 4.,
        maximum_iterations: 30,
        tolerance_mm: 1e-10,
        maximum_rotation_step_rad: 0.2,
    }
}
#[test]
fn aligns_three_planes_with_known_rotation_and_translation() {
    let (source, target, normals, expected) = fixture();
    let original = source.clone();
    let result = point_plane::icp(
        &source,
        &target,
        &normals,
        Rigid::identity(),
        config(),
        &WorkControl::default(),
    )
    .unwrap();
    assert!(result.converged);
    assert!(result.point_to_plane_rms_mm < 1e-8);
    assert!((result.transform.rotation - expected.rotation).norm() < 1e-8);
    assert!((result.transform.translation - expected.translation).norm() < 1e-8);
    assert_eq!(result.correspondences, source.len());
    assert_eq!(source, original);
}
#[test]
fn normal_sign_and_magnitude_do_not_change_alignment() {
    let (source, target, normals, expected) = fixture();
    let normals: Vec<_> = normals.iter().map(|n| n.map(|v| v * -7.)).collect();
    let result = point_plane::icp(
        &source,
        &target,
        &normals,
        Rigid::identity(),
        config(),
        &WorkControl::default(),
    )
    .unwrap();
    assert!((result.transform.rotation - expected.rotation).norm() < 1e-8);
    assert!((result.transform.translation - expected.translation).norm() < 1e-8);
}
#[test]
fn a_single_plane_is_not_a_six_axis_pose_constraint() {
    let target: Vec<_> = (0..20)
        .map(|i| [(i % 5) as f64, (i / 5) as f64, 0.])
        .collect();
    let source: Vec<_> = target.iter().map(|p| [p[0], p[1], p[2] + 0.1]).collect();
    let normals = vec![[0., 0., 1.]; target.len()];
    let error = point_plane::icp(
        &source,
        &target,
        &normals,
        Rigid::identity(),
        config(),
        &WorkControl::default(),
    )
    .err()
    .unwrap();
    assert!(error.contains("underconstrained"));
}
#[test]
fn invalid_normals_overlap_rotation_limits_and_cancellation_fail_without_a_pose() {
    let (source, target, mut normals, _) = fixture();
    normals[0] = [0.; 3];
    assert!(
        point_plane::icp(
            &source,
            &target,
            &normals,
            Rigid::identity(),
            config(),
            &WorkControl::default()
        )
        .is_err()
    );
    normals[0] = [f64::NAN, 0., 1.];
    assert!(
        point_plane::icp(
            &source,
            &target,
            &normals,
            Rigid::identity(),
            config(),
            &WorkControl::default()
        )
        .is_err()
    );
    let (_, _, normals, _) = fixture();
    let mut small = config();
    small.maximum_distance_mm = 1e-9;
    assert!(
        point_plane::icp(
            &source,
            &target,
            &normals,
            Rigid::identity(),
            small,
            &WorkControl::default()
        )
        .is_err()
    );
    assert!(point_plane::step(&source, &target, &normals, 1e-6, &WorkControl::default()).is_err());
    let work = WorkControl::default();
    work.cancel();
    assert!(
        point_plane::icp(
            &source,
            &target,
            &normals,
            Rigid::identity(),
            config(),
            &work
        )
        .is_err()
    );
}

#[test]
fn iteration_exhaustion_is_not_reported_as_convergence() {
    let (source, target, normals, _) = fixture();
    let mut limits = config();
    limits.maximum_iterations = 1;
    let result = point_plane::icp(
        &source,
        &target,
        &normals,
        Rigid::identity(),
        limits,
        &WorkControl::default(),
    )
    .unwrap();
    assert!(!result.converged);
    assert_eq!(result.iterations, 1);
}

#[test]
fn centered_solve_handles_a_distant_coordinate_origin() {
    let (source, target, normals, expected) = fixture();
    let offset = Vector3::new(1e6, -2e6, 3e6);
    let source: Vec<_> = source
        .iter()
        .map(|p| (Vector3::from(*p) + offset).into())
        .collect();
    let target: Vec<_> = target
        .iter()
        .map(|p| (Vector3::from(*p) + offset).into())
        .collect();
    let mut limits = config();
    limits.tolerance_mm = 1e-7;
    let result = point_plane::icp(
        &source,
        &target,
        &normals,
        Rigid::identity(),
        limits,
        &WorkControl::default(),
    )
    .unwrap();
    let translation = expected.translation + offset - expected.rotation * offset;
    assert!(result.converged);
    assert!(result.point_to_plane_rms_mm < 1e-7);
    assert!((result.transform.rotation - expected.rotation).norm() < 1e-8);
    assert!((result.transform.translation - translation).norm() < 0.01);
    for (a, b) in source.iter().zip(target) {
        assert!((Vector3::from(result.transform.apply(*a)) - Vector3::from(b)).norm() < 1e-7);
    }
}

#[test]
fn huber_fixed_correspondences_reduce_outlier_bias_without_modifying_inputs() {
    use point_plane::Loss;
    let (source, target, normals, expected) = fixture();
    let mut contaminated = target.clone();
    for i in (0..target.len()).step_by(9) {
        for j in 0..3 {
            contaminated[i][j] += 6. * normals[i][j];
        }
    }
    let run = |loss| {
        let mut pose = Rigid::identity();
        for _ in 0..35 {
            let transformed: Vec<_> = source.iter().map(|p| pose.apply(*p)).collect();
            let (delta, _) = point_plane::step_with_loss(
                &transformed,
                &contaminated,
                &normals,
                0.5,
                loss,
                &WorkControl::default(),
            )
            .unwrap();
            pose = delta.compose(&pose);
        }
        pose
    };
    let quadratic = run(Loss::Quadratic);
    let robust = run(Loss::Huber { delta_mm: 0.02 });
    let error = |pose: &Rigid| {
        source
            .iter()
            .zip(&target)
            .map(|(p, q)| (Vector3::from(pose.apply(*p)) - Vector3::from(*q)).norm_squared())
            .sum::<f64>()
            .sqrt()
            / (source.len() as f64).sqrt()
    };
    eprintln!("quadratic={} robust={}", error(&quadratic), error(&robust));
    // Independent nonlinear SciPy Huber optimum for this deliberately biased set.
    assert!((error(&robust) - 0.0643517142378787).abs() < 2e-6);
    assert!(error(&quadratic) > 10. * error(&robust));
    assert!((robust.rotation - expected.rotation).norm() < 0.01);
    let (original, _, _, _) = fixture();
    assert_eq!(source, original);
}
#[test]
fn huber_matches_quadratic_below_scale_and_retains_rank_and_parameter_guards() {
    use point_plane::Loss;
    let (source, target, normals, _) = fixture();
    let a = point_plane::icp(
        &source,
        &target,
        &normals,
        Rigid::identity(),
        config(),
        &WorkControl::default(),
    )
    .unwrap();
    let b = point_plane::icp_with_loss(
        &source,
        &target,
        &normals,
        Rigid::identity(),
        config(),
        Loss::Huber { delta_mm: 100. },
        &WorkControl::default(),
    )
    .unwrap();
    assert!((a.transform.translation - b.transform.translation).norm() < 1e-10);
    assert_eq!(b.downweighted_correspondences, 0);
    assert!(b.converged);
    for delta_mm in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(
            point_plane::icp_with_loss(
                &source,
                &target,
                &normals,
                Rigid::identity(),
                config(),
                Loss::Huber { delta_mm },
                &WorkControl::default()
            )
            .is_err()
        );
    }
    let single = vec![[0., 0., 1.]; target.len()];
    assert!(
        point_plane::step_with_loss(
            &source,
            &target,
            &single,
            0.5,
            Loss::Huber { delta_mm: 0.1 },
            &WorkControl::default()
        )
        .err()
        .unwrap()
        .contains("underconstrained")
    );
    let work = WorkControl::default();
    work.cancel();
    assert!(
        point_plane::icp_with_loss(
            &source,
            &target,
            &normals,
            Rigid::identity(),
            config(),
            Loss::Huber { delta_mm: 0.1 },
            &work
        )
        .is_err()
    );
}

#[test]
fn huber_icp_downweights_large_residuals_but_reports_the_full_plane_rms() {
    use point_plane::Loss;
    let (clean, target, normals, _) = fixture();
    let mut source = clean.clone();
    for i in (0..source.len()).step_by(9) {
        source[i][0] += 1.3;
    }
    let run = |loss| {
        let mut c = config();
        c.maximum_iterations = 90;
        c.tolerance_mm = 1e-8;
        point_plane::icp_with_loss(
            &source,
            &target,
            &normals,
            Rigid::identity(),
            c,
            loss,
            &WorkControl::default(),
        )
        .unwrap()
    };
    let a = run(Loss::Quadratic);
    let b = run(Loss::Huber { delta_mm: 0.02 });
    let clean_error = |pose: &Rigid| {
        clean
            .iter()
            .zip(&target)
            .map(|(p, q)| (Vector3::from(pose.apply(*p)) - Vector3::from(*q)).norm_squared())
            .sum::<f64>()
            .sqrt()
            / (clean.len() as f64).sqrt()
    };
    assert!(b.converged);
    assert!(b.downweighted_correspondences >= 4);
    assert!(clean_error(&b.transform) < 0.25 * clean_error(&a.transform));
    assert!(b.point_to_plane_rms_mm > 0.1);
    assert!(b.point_to_plane_rms_mm > (2. * b.objective_mean_mm2).sqrt());
}
