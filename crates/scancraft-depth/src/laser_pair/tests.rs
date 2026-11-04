use super::*;
use scancraft_calibration::{Brown5, CameraModel, stereo::RigidPose};

fn fixture() -> (StereoCalibration, Settings, [StreamProfile; 2], [Frame; 2]) {
    let camera = CameraModel {
        width: 10,
        height: 5,
        fx: 10.,
        fy: 10.,
        cx: 4.5,
        cy: 2.,
        skew: 0.,
        valid_radius: 1.,
        distortion: Brown5::ZERO,
    };
    let mut second = camera.clone();
    second.cy = 1.5;
    let geometry = StereoCalibration {
        left: camera,
        right: second,
        right_from_left: RigidPose {
            rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            translation_mm: [-1., 0., 0.],
        },
    };
    let settings = Settings {
        interfaces: [0, 2],
        reflect_x: [false, false],
        maximum_host_delta_us: 1000,
        detection: Detection {
            background: 10,
            minimum_peak: 30,
            maximum_width: 3,
            saturation: 255,
            maximum_observations: 100,
            opening_radius: None,
        },
        continuity: Continuity {
            maximum_row_gap: 1,
            maximum_horizontal_step: 1.,
            maximum_slope: 0.,
            minimum_span_rows: 2,
            minimum_observations: 2,
        },
        sheets: vec![QuadraticSheet {
            quadratic_y: 0.,
            linear_y: 0.,
            linear_z: 0.,
            offset: 1.5,
        }],
        triangulation: TriangulationLimits {
            min_depth_z_mm: 1.,
            max_depth_z_mm: 100.,
            min_ray_angle_rad: 0.0001,
            max_ray_gap_mm: 0.01,
            max_reprojection_error_px: 0.01,
        },
        matching: laser_trace::Config {
            intersection: scancraft_calibration::epipolar_trace::Config {
                maximum_segment_length_px: 3.,
                normalized_tolerance: 1e-10,
                minimum_plane_sine: 1e-8,
            },
            maximum_sheet_distance_px: 0.25,
            maximum_work: 1000,
        },
        maximum_queries: 100,
        maximum_total_work: 10000,
    };
    let profiles = [0, 2].map(|interface| StreamProfile {
        interface,
        width: 10,
        height: 5,
        fourcc: *b"GREY",
        fps: 30,
    });
    let frames = [6, 5].map(|x| {
        let mut data = vec![0; 50];
        for y in 0..5 {
            data[y * 10 + x] = 80;
        }
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&100u32.to_le_bytes());
        Frame {
            sequence: 0,
            host_timestamp_us: 1000,
            flags: 0,
            metadata,
            data,
        }
    });
    (geometry, settings, profiles, frames)
}
fn inputs<'a>(profiles: &'a [StreamProfile; 2], frames: &'a [Frame; 2]) -> [FrameInput<'a>; 2] {
    [
        FrameInput {
            profile: &profiles[0],
            frame: &frames[0],
        },
        FrameInput {
            profile: &profiles[1],
            frame: &frames[1],
        },
    ]
}
#[derive(Default)]
struct Journal {
    began: bool,
    queries: Vec<serde_json::Value>,
}
impl Sink for Journal {
    fn begin(&mut self, info: PairInfo<'_>) -> Result<(), String> {
        self.began = true;
        assert_eq!(info.first_query_observation_indices.len(), 5);
        assert_eq!(info.sides[0].detections.observations.len(), 5);
        Ok(())
    }
    fn query(&mut self, q: &Query) -> Result<(), String> {
        self.queries
            .push(serde_json::json!({"first_observation":q.first_observation,"result":q.result}));
        Ok(())
    }
}

#[test]
fn pure_and_detailed_sinks_produce_the_same_finite_unqualified_candidates() {
    let (geometry, settings, profiles, frames) = fixture();
    let processor = Processor::new(&geometry, &settings).unwrap();
    let original = frames.clone();
    let mut journal = Journal::default();
    let detailed = processor
        .process(
            inputs(&profiles, &frames),
            &WorkControl::default(),
            &mut journal,
        )
        .unwrap();
    let plain = processor
        .process(inputs(&profiles, &frames), &WorkControl::default(), &mut ())
        .unwrap();
    assert!(journal.began);
    assert_eq!(journal.queries.len(), 5);
    assert_eq!(
        serde_json::to_value(&plain).unwrap(),
        serde_json::to_value(&detailed).unwrap()
    );
    assert_eq!(plain.queries, 5);
    assert_eq!(plain.candidates.len(), 4);
    assert_eq!(plain.outcomes.len(), 5);
    assert_eq!(plain.query_failures, 0);
    for (index, camera) in plain.cameras.iter().enumerate() {
        assert_eq!(camera.interface, profiles[index].interface);
        assert_eq!(camera.observations, 5);
        assert_eq!(camera.supported_traces, 1);
        assert_eq!(camera.supported_observations, 5);
        assert_eq!(camera.rejected_saturation_runs, 0);
    }
    for point in &plain.candidates {
        assert!((point.position_in_supplied_geometry_units[0] - 1.5).abs() < 1e-8);
        assert!((point.position_in_supplied_geometry_units[2] - 10.).abs() < 1e-8);
        assert!(
            point
                .position_in_supplied_geometry_units
                .iter()
                .all(|v| v.is_finite())
        );
    }
    assert!(plain.raw_sensor_pts_equal);
    assert!(
        !plain.units_verified
            && !plain.physical_accuracy_qualified
            && !plain.synchronization_verified
            && !plain.geometry_binding_verified
            && !plain.projector_line_identity_verified
    );
    for (a, b) in frames.iter().zip(&original) {
        assert_eq!(a.data, b.data);
        assert_eq!(a.metadata, b.metadata);
    }
}

#[test]
fn invalid_raw_frames_profiles_and_pair_timing_fail_before_the_sink_begins() {
    let (geometry, settings, profiles, frames) = fixture();
    let processor = Processor::new(&geometry, &settings).unwrap();
    for kind in 0..7 {
        let mut profiles = profiles.clone();
        let mut frames = frames.clone();
        match kind {
            0 => profiles[1].interface = 0,
            1 => profiles[1].width = 11,
            2 => frames[1].metadata[12..16].copy_from_slice(&101u32.to_le_bytes()),
            3 => frames[1].host_timestamp_us = 2001,
            4 => frames[1].flags = 0x40,
            5 => {
                frames[1].data.pop();
            }
            _ => frames[1].metadata.clear(),
        }
        let mut journal = Journal::default();
        assert!(
            processor
                .process(
                    inputs(&profiles, &frames),
                    &WorkControl::default(),
                    &mut journal
                )
                .is_err()
        );
        assert!(!journal.began);
        assert!(journal.queries.is_empty());
    }
}

#[test]
fn empty_and_invalid_pairs_do_not_leave_stale_state_in_a_reused_processor() {
    let (geometry, settings, profiles, frames) = fixture();
    let processor = Processor::new(&geometry, &settings).unwrap();
    let first = processor
        .process(inputs(&profiles, &frames), &WorkControl::default(), &mut ())
        .unwrap();
    let mut empty = frames.clone();
    for frame in &mut empty {
        frame.data.fill(0);
    }
    let blank = processor
        .process(inputs(&profiles, &empty), &WorkControl::default(), &mut ())
        .unwrap();
    assert_eq!(blank.queries, 0);
    assert!(blank.candidates.is_empty() && blank.outcomes.is_empty());
    assert!(
        blank.cameras.iter().all(|c| c.observations == 0
            && c.supported_traces == 0
            && c.supported_observations == 0)
    );
    let mut bad = frames.clone();
    bad[0].flags = 0x40;
    assert!(
        processor
            .process(inputs(&profiles, &bad), &WorkControl::default(), &mut ())
            .is_err()
    );
    let again = processor
        .process(inputs(&profiles, &frames), &WorkControl::default(), &mut ())
        .unwrap();
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(again).unwrap()
    );
    let mut invalid = settings.clone();
    invalid.maximum_queries = 0;
    assert!(Processor::new(&geometry, &invalid).is_err());
}

#[test]
fn duplicate_hypotheses_and_per_query_budget_errors_remain_complete_outcomes() {
    let (geometry, mut settings, profiles, frames) = fixture();
    settings.sheets.push(settings.sheets[0]);
    let ambiguous = Processor::new(&geometry, &settings)
        .unwrap()
        .process(inputs(&profiles, &frames), &WorkControl::default(), &mut ())
        .unwrap();
    assert_eq!(ambiguous.queries, 5);
    assert!(ambiguous.candidates.is_empty());
    assert_eq!(ambiguous.queries_without_unique_match, 5);
    assert_eq!(
        ambiguous
            .outcomes
            .iter()
            .filter(|o| matches!(
                o.outcome,
                Outcome::NoUnique {
                    supported_combinations: 2,
                    ..
                }
            ))
            .count(),
        4
    );
    settings.sheets.pop();
    settings.matching.maximum_work = 5;
    let bounded = Processor::new(&geometry, &settings)
        .unwrap()
        .process(inputs(&profiles, &frames), &WorkControl::default(), &mut ())
        .unwrap();
    assert_eq!(bounded.query_failures, 4);
    assert_eq!(bounded.outcomes.len(), 5);
    assert!(bounded.candidates.is_empty());
    assert_eq!(
        bounded
            .outcomes
            .iter()
            .filter(|o| matches!(&o.outcome,Outcome::Error(e) if e.contains("work limit")))
            .count(),
        4
    );
}

#[test]
fn admission_bounds_hold_before_any_sink_side_effects() {
    let (geometry, mut settings, profiles, frames) = fixture();
    settings.maximum_total_work = 45;
    let exact = Processor::new(&geometry, &settings)
        .unwrap()
        .process(inputs(&profiles, &frames), &WorkControl::default(), &mut ())
        .unwrap();
    assert!(exact.admission.uses_projected_roots);
    assert_eq!(exact.admission.coarse_total_work_bound, 65);
    assert_eq!(exact.admission.conservative_total_work_bound, 45);
    assert_eq!(exact.candidates.len(), 4);
    settings.maximum_total_work = 44;
    let mut journal = Journal::default();
    assert!(
        Processor::new(&geometry, &settings)
            .unwrap()
            .process(
                inputs(&profiles, &frames),
                &WorkControl::default(),
                &mut journal
            )
            .is_err()
    );
    assert!(!journal.began);
}

#[test]
fn sink_failure_and_cancellation_return_no_complete_report_and_do_not_cancel_devices() {
    struct Stop<'a> {
        work: &'a WorkControl,
        cancel: bool,
        calls: usize,
    }
    impl Sink for Stop<'_> {
        fn query(&mut self, _q: &Query) -> Result<(), String> {
            self.calls += 1;
            if self.cancel {
                self.work.cancel();
                Ok(())
            } else {
                Err("journal rejected record".into())
            }
        }
    }
    let (geometry, settings, profiles, frames) = fixture();
    let processor = Processor::new(&geometry, &settings).unwrap();
    for cancel in [false, true] {
        let work = WorkControl::default();
        let mut sink = Stop {
            work: &work,
            cancel,
            calls: 0,
        };
        assert!(
            processor
                .process(inputs(&profiles, &frames), &work, &mut sink)
                .is_err()
        );
        assert_eq!(sink.calls, 1);
        assert_eq!(work.check().is_err(), cancel);
    }
    let work = WorkControl::default();
    work.cancel();
    let mut journal = Journal::default();
    assert!(
        processor
            .process(inputs(&profiles, &frames), &work, &mut journal)
            .is_err()
    );
    assert!(!journal.began);
}

#[test]
fn concurrent_large_pair_detection_preserves_complete_reports_and_raw_frames() {
    let (mut geometry, mut settings, mut profiles, mut frames) = fixture();
    for camera in [&mut geometry.left, &mut geometry.right] {
        camera.width = 640;
        camera.height = 480;
        camera.fx = 640.;
        camera.fy = 640.;
        camera.cx = 320.;
        camera.cy = 240.;
    }
    settings.detection.maximum_observations = 2000;
    settings.maximum_queries = 1000;
    settings.matching.maximum_work = 10000;
    settings.maximum_total_work = 1_000_000;
    for i in 0..2 {
        profiles[i].width = 640;
        profiles[i].height = 480;
        frames[i].data = vec![0; 640 * 480];
        for row in 0..480 {
            frames[i].data[row * 640 + if i == 0 { 416 } else { 352 }] = 80;
        }
    }
    let original = frames.clone();
    let work = WorkControl::default();
    let mut processor = Processor::new(&geometry, &settings).unwrap();
    processor.parallel_detection = false;
    let serial = processor
        .process(inputs(&profiles, &frames), &work, &mut ())
        .unwrap();
    processor.parallel_detection = true;
    let parallel = processor
        .process(inputs(&profiles, &frames), &work, &mut ())
        .unwrap();
    assert_eq!(serial.candidates.len(), 480);
    assert_eq!(
        serde_json::to_vec(&serial).unwrap(),
        serde_json::to_vec(&parallel).unwrap()
    );
    for i in 0..2 {
        assert_eq!(frames[i].data, original[i].data);
        assert_eq!(frames[i].metadata, original[i].metadata);
    }
}

#[test]
fn concurrent_detection_joins_and_retains_first_camera_error_precedence() {
    let (_, mut settings, _, _) = fixture();
    settings.detection.maximum_width = 600;
    settings.continuity.minimum_span_rows = 500;
    let narrow = vec![0; 512 * 512];
    let short = vec![0; 640 * 480];
    let images = [
        GrayImage::new(512, 512, &narrow).unwrap(),
        GrayImage::new(640, 480, &short).unwrap(),
    ];
    let work = WorkControl::default();
    let first_error = detect(images[0], &settings.detection, &settings.continuity, &work)
        .err()
        .unwrap();
    let second_error = detect(images[1], &settings.detection, &settings.continuity, &work)
        .err()
        .unwrap();
    assert_ne!(first_error, second_error);
    for images in [images, [images[1], images[0]]] {
        let serial = detection::pair(images, &settings, &work, false)
            .err()
            .unwrap();
        let parallel = detection::pair(images, &settings, &work, true)
            .err()
            .unwrap();
        assert_eq!(serial, parallel);
    }
}

#[test]
fn concurrent_detection_observes_cancellation_before_starting_borrowed_work() {
    let (_, settings, _, _) = fixture();
    let data = vec![0; 640 * 480];
    let images = [GrayImage::new(640, 480, &data).unwrap(); 2];
    let work = WorkControl::default();
    work.cancel();
    assert!(
        detection::pair(images, &settings, &work, true)
            .err()
            .unwrap()
            .contains("cancelled")
    );
}

#[test]
fn domain_opt_in_retains_usable_segments_and_accounts_for_every_exclusion() {
    let (mut geometry, settings, profiles, frames) = fixture();
    geometry.right.valid_radius = 0.16;
    let strict = Processor::new(&geometry, &settings)
        .unwrap()
        .process(inputs(&profiles, &frames), &WorkControl::default(), &mut ())
        .unwrap();
    assert_eq!(strict.query_failures, 5);
    assert!(strict.domain_rejections.is_none());
    #[derive(Default)]
    struct Exclusions {
        vertices: usize,
        segments: usize,
        roots: usize,
    }
    impl Sink for Exclusions {
        fn begin(&mut self, info: PairInfo<'_>) -> Result<(), String> {
            let rejected = info.domain_rejections.unwrap();
            self.vertices = rejected.vertices.len();
            self.segments = rejected.segments.len();
            assert_eq!(rejected.vertices[0].vertex_index, 4);
            Ok(())
        }
        fn query(&mut self, query: &Query) -> Result<(), String> {
            self.roots += query.rejected_projections.as_ref().unwrap().len();
            Ok(())
        }
    }
    let mut journal = Exclusions::default();
    let report = Processor::new(&geometry, &settings)
        .unwrap()
        .with_valid_domain_segments(true)
        .process(
            inputs(&profiles, &frames),
            &WorkControl::default(),
            &mut journal,
        )
        .unwrap();
    assert_eq!(report.candidates.len(), 3);
    assert_eq!(report.query_failures, 0);
    assert_eq!(report.queries_without_unique_match, 2);
    assert_eq!(
        (journal.vertices, journal.segments, journal.roots),
        (1, 1, 2)
    );
    let counts = report.domain_rejections.unwrap();
    assert_eq!(
        (
            counts.rejected_trace_vertices,
            counts.rejected_trace_segments,
            counts.rejected_sheet_projections
        ),
        (1, 1, 2)
    );
    assert!(!report.units_verified && !report.physical_accuracy_qualified);
}

#[test]
fn domain_admission_fallback_preserves_results_and_enforces_exact_work_bound() {
    let (geometry, mut settings, profiles, frames) = fixture();
    let plain = Processor::new(&geometry, &settings)
        .unwrap()
        .process(inputs(&profiles, &frames), &WorkControl::default(), &mut ())
        .unwrap();
    settings.maximum_total_work = 45;
    let bounded = Processor::new(&geometry, &settings)
        .unwrap()
        .with_valid_domain_segments(true)
        .process(inputs(&profiles, &frames), &WorkControl::default(), &mut ())
        .unwrap();
    assert_eq!(bounded.admission.conservative_total_work_bound, 45);
    assert!(bounded.admission.uses_projected_roots);
    assert_eq!(
        serde_json::to_value(&plain.candidates).unwrap(),
        serde_json::to_value(&bounded.candidates).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&plain.outcomes).unwrap(),
        serde_json::to_value(&bounded.outcomes).unwrap()
    );
    settings.maximum_total_work = 44;
    let mut journal = Journal::default();
    assert!(
        Processor::new(&geometry, &settings)
            .unwrap()
            .with_valid_domain_segments(true)
            .process(
                inputs(&profiles, &frames),
                &WorkControl::default(),
                &mut journal
            )
            .is_err()
    );
    assert!(!journal.began);
}

#[test]
fn missing_pts_on_both_cameras_is_not_equal_sensor_timing() {
    let (geometry, settings, profiles, original) = fixture();
    let processor = Processor::new(&geometry, &settings).unwrap();
    for empty in [true, false] {
        let mut frames = original.clone();
        for f in &mut frames {
            f.metadata = if empty {
                Vec::new()
            } else {
                let mut metadata = vec![0; 12];
                metadata[10] = 2;
                metadata
            };
        }
        let mut journal = Journal::default();
        assert!(
            processor
                .process(
                    inputs(&profiles, &frames),
                    &WorkControl::default(),
                    &mut journal
                )
                .is_err()
        );
        assert!(!journal.began);
    }
}
