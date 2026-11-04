use super::*;
use crate::{Point, tracking::TrackingConfig};

fn points(shift: f64) -> Vec<[f64; 3]> {
    vec![
        [0.25 + shift, 0.25, 0.25],
        [10.25 + shift, 0.25, 0.25],
        [0.25 + shift, 20.25, 0.25],
    ]
}
fn cloud(shift: f64) -> Cloud {
    Cloud {
        points: points(shift)
            .into_iter()
            .map(|position| Point {
                position,
                color: None,
                normal: None,
            })
            .collect(),
    }
}
fn session(voxels: usize, frames: usize) -> ScanSession {
    let mut s = ScanSession::new(
        "recovery".into(),
        TrackingConfig {
            max_correspondence_mm: 2.,
            max_rms_mm: 0.01,
            minimum_overlap: 0.8,
            max_step_translation_mm: 1.,
            max_step_rotation_rad: 0.1,
            iterations: 20,
            tolerance_mm: 1e-8,
        },
        1.,
        voxels,
        frames,
    )
    .unwrap();
    s.start().unwrap();
    s
}
fn moved() -> ScanSession {
    let mut s = session(3, 100);
    for id in 0..16 {
        let shift = id as f64 * 0.2;
        s.process_with_reference_recovery(
            id,
            &cloud(shift),
            |_, _| Ok(points(shift)),
            &WorkControl::default(),
        )
        .unwrap();
    }
    assert_eq!(s.retained_reference_ids(), vec![0, 7, 15]);
    s
}

#[test]
fn recovery_preserves_world_and_rolls_back_fusion_cancellation_and_late_selector_failure() {
    let mut s = moved();
    let before = s.cloud(&WorkControl::default()).unwrap();
    let mut too_large = cloud(0.);
    too_large.points.push(Point {
        position: [100.; 3],
        color: None,
        normal: None,
    });
    assert!(
        s.process_with_reference_recovery(
            16,
            &too_large,
            |_, _| Ok(points(0.)),
            &WorkControl::default()
        )
        .unwrap_err()
        .contains("capacity")
    );
    assert_eq!(s.last_accepted(), Some(15));
    assert_eq!(s.last_integrated(), Some(15));
    assert_eq!(s.history().len(), 16);
    assert_eq!(s.retained_reference_ids(), vec![0, 7, 15]);
    assert_eq!(s.cloud(&WorkControl::default()).unwrap(), before);
    let work = WorkControl::default();
    let mut calls = 0;
    assert!(
        s.process_with_reference_recovery(
            16,
            &cloud(0.),
            |_, w| {
                calls += 1;
                if calls == 2 {
                    w.cancel();
                }
                Ok(points(0.))
            },
            &work
        )
        .is_err()
    );
    assert_eq!(calls, 2);
    let mut calls = 0;
    assert!(
        s.process_with_reference_recovery(
            16,
            &cloud(0.),
            |_, _| {
                calls += 1;
                if calls == 3 {
                    Err("late selector failure".into())
                } else {
                    Ok(points(0.))
                }
            },
            &WorkControl::default()
        )
        .unwrap_err()
        .contains("late selector")
    );
    assert_eq!(calls, 3);
    assert_eq!(s.last_accepted(), Some(15));
    assert_eq!(s.history().len(), 16);
    assert_eq!(s.cloud(&WorkControl::default()).unwrap(), before);
    let out = s
        .process_with_reference_recovery(
            16,
            &cloud(0.),
            |_, _| Ok(points(0.)),
            &WorkControl::default(),
        )
        .unwrap();
    assert_eq!(out.recovered_from, Some(0));
    assert_eq!(out.attempts.len(), 4);
    assert!(!out.conflicting_references);
    let pose = out.frame.unwrap().tracking.world_from_frame.unwrap();
    assert!(pose.translation.norm() < 1e-10);
    assert_eq!(s.last_accepted(), Some(16));
    assert_eq!(s.last_integrated(), Some(16));
    assert_eq!(s.history().len(), 17);
    assert!(
        s.process_with_reference_recovery(
            16,
            &cloud(0.),
            |_, _| Ok(points(0.)),
            &WorkControl::default()
        )
        .is_err()
    );
}

#[test]
fn conflicting_reference_specific_correspondences_never_publish_a_pose_or_surface() {
    let mut s = moved();
    let before = s.cloud(&WorkControl::default()).unwrap();
    let mut calls = 0;
    let out = s
        .process_with_reference_recovery(
            16,
            &cloud(0.),
            |reference, _| {
                calls += 1;
                if calls == 1 {
                    Ok(Vec::new())
                } else {
                    Ok(reference.unwrap().positions.to_vec())
                }
            },
            &WorkControl::default(),
        )
        .unwrap();
    assert!(out.conflicting_references);
    assert!(out.maximum_pose_disagreement > 2.9);
    assert!(out.recovered_from.is_none());
    assert!(out.frame.is_none());
    assert_eq!(
        out.attempts
            .iter()
            .filter(|a| a.tracking.as_ref().is_ok_and(|u| u.accepted))
            .count(),
        3
    );
    assert_eq!(s.last_accepted(), Some(15));
    assert_eq!(s.last_integrated(), Some(15));
    assert_eq!(s.history().len(), 16);
    assert_eq!(s.cloud(&WorkControl::default()).unwrap(), before);
    // Unavailable/ambiguous attempts consume an ID, without fabricating history.
    assert!(
        s.process_with_reference_recovery(
            16,
            &cloud(0.),
            |_, _| Ok(points(0.)),
            &WorkControl::default()
        )
        .is_err()
    );
    let out = s
        .process_with_reference_recovery(
            17,
            &cloud(0.),
            |_, _| Ok(points(0.)),
            &WorkControl::default(),
        )
        .unwrap();
    assert_eq!(out.recovered_from, Some(0));
}

#[test]
fn tracking_only_cadence_evicts_oldest_nonanchor_and_enforces_capacity_and_mode() {
    let mut s = session(3, 140);
    let empty = Cloud { points: vec![] };
    for id in 0..140 {
        let out = s
            .process_with_reference_recovery(
                id,
                &empty,
                |_, _| Ok(points(0.)),
                &WorkControl::default(),
            )
            .unwrap();
        assert!(out.frame.unwrap().fusion.is_none());
    }
    let expected = std::iter::once(0)
        .chain((23..=135).step_by(8))
        .collect::<Vec<_>>();
    assert_eq!(s.retained_reference_ids(), expected);
    assert_eq!(expected.len(), MAX_REFERENCES);
    assert_eq!(s.last_integrated(), None);
    assert_eq!(s.last_accepted(), Some(139));
    assert!(
        s.process_with_reference_recovery(
            140,
            &empty,
            |_, _| Ok(points(0.)),
            &WorkControl::default()
        )
        .is_err()
    );
    assert!(
        s.process_with_tracking_cloud(140, &empty, &cloud(0.), &WorkControl::default())
            .is_err()
    );
}

#[test]
fn reference_insertion_and_eviction_commit_only_after_fusion_succeeds() {
    let mut s = session(3, 130);
    for id in 0..127 {
        s.process_with_reference_recovery(
            id,
            &cloud(0.),
            |_, _| Ok(points(0.)),
            &WorkControl::default(),
        )
        .unwrap();
    }
    let references = s.retained_reference_ids();
    assert_eq!(references.len(), MAX_REFERENCES);
    assert_eq!(references[1], 7);
    let before = s.cloud(&WorkControl::default()).unwrap();
    let mut large = cloud(0.);
    large.points.push(Point {
        position: [100.; 3],
        color: None,
        normal: None,
    });
    assert!(
        s.process_with_reference_recovery(
            127,
            &large,
            |_, _| Ok(points(0.)),
            &WorkControl::default()
        )
        .unwrap_err()
        .contains("capacity")
    );
    assert_eq!(s.retained_reference_ids(), references);
    assert_eq!(s.last_accepted(), Some(126));
    assert_eq!(s.history().len(), 127);
    assert_eq!(s.cloud(&WorkControl::default()).unwrap(), before);
    s.process_with_reference_recovery(
        127,
        &cloud(0.),
        |_, _| Ok(points(0.)),
        &WorkControl::default(),
    )
    .unwrap();
    let updated = s.retained_reference_ids();
    assert_eq!(updated.len(), MAX_REFERENCES);
    assert_eq!(updated[0], 0);
    assert_eq!(updated[1], 15);
    assert_eq!(updated.last(), Some(&127));
}

#[test]
fn insufficient_and_invalid_observations_have_distinct_transaction_semantics() {
    let mut s = session(3, 3);
    let empty = Cloud { points: vec![] };
    let out = s
        .process_with_reference_recovery(0, &empty, |_, _| Ok(vec![]), &WorkControl::default())
        .unwrap();
    assert!(out.frame.is_none());
    assert!(s.history().is_empty());
    assert!(s.retained_reference_ids().is_empty());
    assert!(
        s.process_with_reference_recovery(
            0,
            &empty,
            |_, _| Ok(points(0.)),
            &WorkControl::default()
        )
        .is_err()
    );
    for invalid in [vec![[f64::NAN, 0., 0.]], vec![[0.; 3]; 513]] {
        assert!(
            s.process_with_reference_recovery(
                1,
                &empty,
                |_, _| Ok(invalid.clone()),
                &WorkControl::default()
            )
            .is_err()
        );
    }
    let out = s
        .process_with_reference_recovery(1, &empty, |_, _| Ok(points(0.)), &WorkControl::default())
        .unwrap();
    assert!(out.frame.unwrap().tracking.accepted);
    assert_eq!(s.retained_reference_ids(), vec![1]);
    let out = s
        .process_with_reference_recovery(
            2,
            &empty,
            |_, _| Ok(points(1000.)),
            &WorkControl::default(),
        )
        .unwrap();
    assert!(!out.frame.unwrap().tracking.accepted);
    assert!(out.recovered_from.is_none());
    assert_eq!(out.attempts.len(), 2);
    assert_eq!(s.last_accepted(), Some(1));
    assert!(
        s.process_with_reference_recovery(
            3,
            &empty,
            |_, _| Ok(points(0.)),
            &WorkControl::default()
        )
        .is_err()
    );
    let mut legacy = session(3, 3);
    legacy
        .process_with_tracking_cloud(0, &empty, &cloud(0.), &WorkControl::default())
        .unwrap();
    assert!(
        legacy
            .process_with_reference_recovery(
                1,
                &empty,
                |_, _| Ok(points(0.)),
                &WorkControl::default()
            )
            .is_err()
    );
}
