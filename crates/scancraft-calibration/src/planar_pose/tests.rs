use super::*;
use crate::Brown5;
use nalgebra::Rotation3;
use std::cell::Cell;
fn fixture(skew: f64, noise: f64) -> (CameraModel, RigidPose, Vec<Observation>) {
    let camera = CameraModel {
        width: 1280,
        height: 800,
        fx: 900.,
        fy: 850.,
        cx: 629.,
        cy: 393.,
        skew,
        distortion: Brown5 {
            k1: -0.09,
            k2: 0.025,
            k3: -0.005,
            p1: 0.0008,
            p2: -0.0011,
        },
        valid_radius: 1.,
    };
    let r = Rotation3::from_euler_angles(0.25, -0.32, 0.09);
    let pose = RigidPose {
        rotation: std::array::from_fn(|i| std::array::from_fn(|j| r[(i, j)])),
        translation_mm: [14., -8., 420.],
    };
    let mut points = Vec::new();
    for y in 0..6 {
        for x in 0..8 {
            let xy = [(x as f64 - 3.5) * 20., (y as f64 - 2.5) * 20.];
            let mut pixel = camera.project(pose.apply([xy[0], xy[1], 0.])).unwrap();
            let phase = (y * 8 + x) as f64;
            pixel[0] += noise * (phase * 1.7).sin();
            pixel[1] += noise * (phase * 0.9).cos();
            points.push(Observation {
                target_xy: xy,
                pixel,
            });
        }
    }
    (camera, pose, points)
}
#[test]
fn exact_pose_and_unit_scaling_keep_fixed_lens_and_pixel_residuals() {
    for skew in [0., 6.] {
        let (camera, truth, points) = fixture(skew, 0.);
        for scale in [0.01, 1., 100.] {
            let mut scaled = points.clone();
            for p in &mut scaled {
                for v in &mut p.target_xy {
                    *v *= scale;
                }
            }
            let result = fit(&camera, &scaled, &Options::default(), &|| false).unwrap();
            assert_eq!(result.fixed_camera, camera);
            assert!(result.rms_px < 1e-8);
            for i in 0..3 {
                assert!(
                    (result.translation_in_supplied_target_units[i] / scale
                        - truth.translation_mm[i])
                        .abs()
                        < 1e-6
                );
                for j in 0..3 {
                    assert!((result.rotation[i][j] - truth.rotation[i][j]).abs() < 1e-8);
                }
            }
        }
    }
}
#[test]
fn noisy_pose_improves_and_reports_every_residual_without_changing_camera() {
    let (camera, _, points) = fixture(6., 0.12);
    let result = fit(&camera, &points, &Options::default(), &|| false).unwrap();
    assert_eq!(result.fixed_camera, camera);
    assert!(result.rms_px < result.initial_rms_px);
    assert!(result.rms_px > 0.05 && result.rms_px < 0.2);
    assert_eq!(result.residuals_px.len(), points.len());
    let pose = RigidPose {
        rotation: result.rotation,
        translation_mm: result.translation_in_supplied_target_units,
    };
    let mut squared = 0.;
    for (p, residual) in points.iter().zip(&result.residuals_px) {
        let predicted = camera
            .project(pose.apply([p.target_xy[0], p.target_xy[1], 0.]))
            .unwrap();
        for i in 0..2 {
            assert!((predicted[i] - p.pixel[i] - residual[i]).abs() < 1e-12);
            squared += residual[i] * residual[i];
        }
    }
    assert!(((squared / points.len() as f64).sqrt() - result.rms_px).abs() < 1e-12);
    assert!(
        fit(
            &camera,
            &points,
            &Options {
                max_reprojection_error_px: 0.01,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
}
#[test]
fn invalid_degenerate_domain_iteration_and_cancellation_fail_closed() {
    let (camera, _, points) = fixture(0., 0.12);
    assert!(fit(&camera, &points[..3], &Options::default(), &|| false).is_err());
    let mut bad = points.clone();
    bad[1].target_xy = bad[0].target_xy;
    assert!(fit(&camera, &bad, &Options::default(), &|| false).is_err());
    bad = points.clone();
    for (i, p) in bad.iter_mut().enumerate() {
        p.target_xy = [i as f64, 0.];
    }
    assert!(fit(&camera, &bad, &Options::default(), &|| false).is_err());
    bad = points.clone();
    bad[0].pixel[0] = f64::NAN;
    assert!(fit(&camera, &bad, &Options::default(), &|| false).is_err());
    let mut limited = camera.clone();
    limited.valid_radius = 0.01;
    assert!(fit(&limited, &points, &Options::default(), &|| false).is_err());
    assert!(
        fit(
            &camera,
            &points,
            &Options {
                max_iterations: 1,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    assert!(
        fit(
            &camera,
            &points,
            &Options {
                max_iterations: 0,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    for after in [0, 3, 8] {
        let calls = Cell::new(0);
        let error = fit(&camera, &points, &Options::default(), &|| {
            let n = calls.get();
            calls.set(n + 1);
            n >= after
        })
        .unwrap_err();
        assert!(error.contains("cancelled"), "{error}");
    }
}
#[test]
fn pose_normal_gradient_matches_finite_difference_of_objective() {
    let (camera, pose, points) = fixture(6., 0.12);
    let score = evaluate(&camera, &pose, &points, &|| false).unwrap();
    for i in 0..6 {
        let epsilon = if i < 3 { 1e-7 } else { 1e-5 };
        let mut step = SVector::zeros();
        step[i] = epsilon;
        let plus = evaluate(&camera, &step_pose(&pose, step).unwrap(), &points, &|| {
            false
        })
        .unwrap();
        let minus = evaluate(&camera, &step_pose(&pose, -step).unwrap(), &points, &|| {
            false
        })
        .unwrap();
        let numerical = (plus.squared - minus.squared) / (4. * epsilon);
        assert!(
            (numerical - score.normal.gradient[i]).abs() < 1e-5,
            "axis {i}: {numerical} vs {}",
            score.normal.gradient[i]
        );
    }
}
