use super::*;
use crate::Point;

fn cloud(points: &[[f64; 3]]) -> Cloud {
    Cloud {
        points: points
            .iter()
            .map(|p| Point {
                position: *p,
                color: None,
                normal: None,
            })
            .collect(),
    }
}
fn markers(shift: f64) -> Cloud {
    cloud(&[[shift, 0., 0.], [10. + shift, 0., 0.], [shift, 20., 0.]])
}
fn config() -> TrackingConfig {
    TrackingConfig {
        max_correspondence_mm: 2.,
        max_rms_mm: 0.01,
        minimum_overlap: 0.8,
        max_step_translation_mm: 1.,
        max_step_rotation_rad: 0.1,
        iterations: 20,
        tolerance_mm: 1e-8,
    }
}
fn session(capacity: usize) -> ScanSession {
    let mut s = ScanSession::new("separate".into(), config(), 1., capacity, 10).unwrap();
    s.start().unwrap();
    s
}

#[test]
fn marker_pose_transforms_surface_without_fusing_marker_geometry() {
    let work = WorkControl::default();
    let mut s = session(10);
    s.process_with_tracking_cloud(0, &cloud(&[[100.25, 5.25, 7.25]]), &markers(0.), &work)
        .unwrap();
    let update = s
        .process_with_tracking_cloud(
            1,
            &cloud(&[[100.45, 5.25, 7.25], [110.45, 5.25, 7.25]]),
            &markers(0.2),
            &work,
        )
        .unwrap();
    assert!(update.tracking.accepted);
    assert_eq!(update.fusion.as_ref().unwrap().input_points, 2);
    assert!(
        (update
            .tracking
            .world_from_frame
            .as_ref()
            .unwrap()
            .translation[0]
            + 0.2)
            .abs()
            < 1e-10
    );
    let fused = s.cloud(&work).unwrap();
    assert_eq!(fused.points.len(), 2);
    for (point, expected) in fused.points.iter().zip([100.25, 110.25]) {
        assert!((point.position[0] - expected).abs() < 1e-10);
        assert!((point.position[1] - 5.25).abs() < 1e-10);
        assert!((point.position[2] - 7.25).abs() < 1e-10);
    }
}

#[test]
fn separate_observations_preserve_atomic_failure_and_retry() {
    let work = WorkControl::default();
    let mut s = session(1);
    s.process_with_tracking_cloud(0, &cloud(&[[100.25, 5.25, 7.25]]), &markers(0.), &work)
        .unwrap();
    let before = s.cloud(&work).unwrap();
    let error = s
        .process_with_tracking_cloud(
            1,
            &cloud(&[[100.45, 5.25, 7.25], [110.45, 5.25, 7.25]]),
            &markers(0.2),
            &work,
        )
        .unwrap_err();
    assert!(error.contains("capacity"));
    assert_eq!(s.last_accepted(), Some(0));
    assert_eq!(s.last_integrated(), Some(0));
    assert_eq!(s.history().len(), 1);
    assert_eq!(s.cloud(&work).unwrap(), before);
    let mut invalid = markers(0.2);
    invalid.points[0].position[0] = f64::NAN;
    assert!(
        s.process_with_tracking_cloud(1, &before, &invalid, &work)
            .is_err()
    );
    for count in [2, 513] {
        let invalid = cloud(&vec![[0.; 3]; count]);
        assert!(
            s.process_with_tracking_cloud(1, &before, &invalid, &work)
                .is_err()
        );
    }
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(
        s.process_with_tracking_cloud(1, &before, &markers(0.2), &cancelled)
            .is_err()
    );
    assert_eq!(s.history().len(), 1);
    assert_eq!(s.cloud(&work).unwrap(), before);
    let retry = s
        .process_with_tracking_cloud(1, &cloud(&[[100.45, 5.25, 7.25]]), &markers(0.2), &work)
        .unwrap();
    assert!(retry.tracking.accepted);
    assert_eq!(s.last_accepted(), Some(1));
    assert_eq!(s.last_integrated(), Some(1));
}

#[test]
fn rejected_marker_alignment_does_not_fuse_surface_and_can_recover() {
    let work = WorkControl::default();
    let mut s = session(10);
    let surface = cloud(&[[100.25, 5.25, 7.25]]);
    s.process_with_tracking_cloud(0, &surface, &markers(0.), &work)
        .unwrap();
    let rejected = s
        .process_with_tracking_cloud(1, &surface, &markers(1000.), &work)
        .unwrap();
    assert!(!rejected.tracking.accepted);
    assert!(rejected.tracking.world_from_frame.is_none());
    assert!(rejected.fusion.is_none());
    assert_eq!(s.last_integrated(), Some(0));
    assert!(
        s.process_with_tracking_cloud(2, &surface, &markers(0.), &work)
            .unwrap()
            .tracking
            .accepted
    );
    assert_eq!(s.history().len(), 3);
    let fused = s.cloud(&work).unwrap();
    assert_eq!(fused.points.len(), 1);
    for (actual, expected) in fused.points[0]
        .position
        .iter()
        .zip(surface.points[0].position)
    {
        assert!((actual - expected).abs() < 1e-10);
    }
}

#[test]
fn separate_tracking_normals_are_required_only_on_observations() {
    let work = WorkControl::default();
    let mut s = ScanSession::new_with_method(
        "normals".into(),
        config(),
        1.,
        10,
        10,
        RegistrationMethod::PointToPlane,
    )
    .unwrap();
    s.start().unwrap();
    let surface = cloud(&[[100.25, 5.25, 7.25]]);
    let mut observed = cloud(&[
        [0., 0., 0.],
        [10., 0., 0.],
        [0., 20., 0.],
        [0., 0., 30.],
        [4., 5., 8.],
        [-7., 2., 4.],
    ]);
    assert!(
        s.process_with_tracking_cloud(0, &surface, &observed, &work)
            .unwrap_err()
            .contains("missing a normal")
    );
    assert!(s.history().is_empty());
    for point in &mut observed.points {
        point.normal = Some([0., 0., 1.]);
    }
    assert!(
        s.process_with_tracking_cloud(0, &surface, &observed, &work)
            .unwrap()
            .tracking
            .accepted
    );
    assert_eq!(s.cloud(&work).unwrap(), surface);
}

#[test]
fn empty_surface_advances_only_tracking_until_real_surface_arrives() {
    let work = WorkControl::default();
    let mut s = session(10);
    let empty = cloud(&[]);
    for (id, shift) in [(0, 0.), (1, 0.2)] {
        let update = s
            .process_with_tracking_cloud(id, &empty, &markers(shift), &work)
            .unwrap();
        assert!(update.tracking.accepted);
        assert!(update.fusion.is_none());
        assert_eq!(s.last_accepted(), Some(id));
        assert_eq!(s.last_integrated(), None);
        assert!(s.cloud(&work).unwrap().points.is_empty());
    }
    let update = s
        .process_with_tracking_cloud(2, &cloud(&[[100.65, 5.25, 7.25]]), &markers(0.4), &work)
        .unwrap();
    assert!(update.tracking.accepted);
    assert_eq!(update.fusion.as_ref().unwrap().input_points, 1);
    assert_eq!(s.last_integrated(), Some(2));
    assert!((s.cloud(&work).unwrap().points[0].position[0] - 100.25).abs() < 1e-10);
    assert_eq!(s.history().len(), 3);
}
