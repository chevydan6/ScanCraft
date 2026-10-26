use scancraft_geometry::{
    tracking::{RegistrationMethod, Tracker, TrackingConfig, TrackingState},
    work::WorkControl,
};
#[test]
fn normal_validation_errors_do_not_advance_tracker_or_reference() {
    let config = TrackingConfig {
        max_correspondence_mm: 2.,
        max_rms_mm: 0.01,
        minimum_overlap: 0.8,
        max_step_translation_mm: 1.,
        max_step_rotation_rad: 0.1,
        iterations: 20,
        tolerance_mm: 1e-8,
    };
    let mut tracker = Tracker::new_with_method(config, RegistrationMethod::PointToPlane).unwrap();
    let mut points = Vec::new();
    let mut normals = Vec::new();
    for a in [-3., 0., 3.] {
        for b in [-2., 0., 2.] {
            points.extend([[30., a, b], [a, 40., b], [a, b, 50.]]);
            normals.extend([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        }
    }
    let work = WorkControl::default();
    assert!(tracker.track(0, &points, &work).is_err());
    assert_eq!(tracker.state(), TrackingState::Empty);
    assert_eq!(tracker.last_accepted(), None);
    assert!(
        tracker
            .track_with_normals(0, &points, &normals, &work)
            .unwrap()
            .accepted
    );
    let shifted: Vec<_> = points.iter().map(|p| [p[0] - 0.2, p[1], p[2]]).collect();
    let mut invalid = normals.clone();
    invalid[0] = [0.; 3];
    assert!(
        tracker
            .track_with_normals(1, &shifted, &invalid, &work)
            .is_err()
    );
    assert_eq!(tracker.state(), TrackingState::Tracking);
    assert_eq!(tracker.last_accepted(), Some(0));
    let update = tracker
        .track_with_normals(1, &shifted, &normals, &work)
        .unwrap();
    assert!(update.accepted);
    assert!((update.world_from_frame.unwrap().translation[0] - 0.2).abs() < 1e-8);
    tracker.reset();
    assert_eq!(tracker.state(), TrackingState::Empty);
    assert!(tracker.track(0, &points, &work).is_err());
    assert!(
        tracker
            .track_with_normals(0, &points, &normals, &work)
            .unwrap()
            .accepted
    );
}

#[test]
fn successive_rotated_frames_retain_normals_in_their_local_reference_basis() {
    use nalgebra::{Rotation3, Vector3};
    let config = TrackingConfig {
        max_correspondence_mm: 3.,
        max_rms_mm: 0.01,
        minimum_overlap: 0.8,
        max_step_translation_mm: 1.,
        max_step_rotation_rad: 0.1,
        iterations: 30,
        tolerance_mm: 1e-8,
    };
    let mut tracker = Tracker::new_with_method(config, RegistrationMethod::PointToPlane).unwrap();
    let mut world = Vec::new();
    let mut normals = Vec::new();
    for a in [-3., 0., 3.] {
        for b in [-2., 0., 2.] {
            world.extend([[30., a, b], [a, 40., b], [a, b, 50.]]);
            normals.extend([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        }
    }
    for id in 0..3u64 {
        let rotation =
            Rotation3::from_scaled_axis(Vector3::new(0.01, -0.008, 0.012) * id as f64).into_inner();
        let translation = Vector3::new(0.2, -0.1, 0.15) * id as f64;
        let local: Vec<_> = world
            .iter()
            .map(|p| (rotation.transpose() * (Vector3::from(*p) - translation)).into())
            .collect();
        let local_normals: Vec<_> = normals
            .iter()
            .map(|n| (rotation.transpose() * Vector3::from(*n)).into())
            .collect();
        let update = tracker
            .track_with_normals(id, &local, &local_normals, &WorkControl::default())
            .unwrap();
        assert!(update.accepted, "{}", update.reason);
        let pose = update.world_from_frame.unwrap();
        assert!((pose.rotation - rotation).norm() < 1e-8);
        assert!((pose.translation - translation).norm() < 1e-7);
    }
}

#[test]
fn huber_tracker_retains_euclidean_gate_reference_and_normal_requirements() {
    let config = TrackingConfig {
        max_correspondence_mm: 3.,
        max_rms_mm: 0.01,
        minimum_overlap: 0.8,
        max_step_translation_mm: 1.,
        max_step_rotation_rad: 0.1,
        iterations: 90,
        tolerance_mm: 1e-8,
    };
    assert!(
        Tracker::new_with_method(
            config,
            RegistrationMethod::PointToPlaneHuber { delta_mm: 0. }
        )
        .is_err()
    );
    let mut tracker = Tracker::new_with_method(
        config,
        RegistrationMethod::PointToPlaneHuber { delta_mm: 0.02 },
    )
    .unwrap();
    let mut points = Vec::new();
    let mut normals = Vec::new();
    for a in [-3., 0., 3.] {
        for b in [-2., 0., 2.] {
            points.extend([[30., a, b], [a, 40., b], [a, b, 50.]]);
            normals.extend([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        }
    }
    let work = WorkControl::default();
    assert!(tracker.track(0, &points, &work).is_err());
    assert_eq!(tracker.state(), TrackingState::Empty);
    assert!(
        tracker
            .track_with_normals(0, &points, &normals, &work)
            .unwrap()
            .accepted
    );
    let mut outlier = points.clone();
    outlier[0][0] += 1.;
    let rejected = tracker
        .track_with_normals(1, &outlier, &normals, &work)
        .unwrap();
    assert!(!rejected.accepted);
    assert_eq!(rejected.reason, "alignment residual exceeds gate");
    assert!(rejected.rms_mm.unwrap() > config.max_rms_mm);
    assert_eq!(tracker.last_accepted(), Some(0));
    assert!(
        tracker
            .track_with_normals(2, &points, &normals, &work)
            .unwrap()
            .accepted
    );
}
