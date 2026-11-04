use super::*;
use scancraft_image::diagnostics::GrayStatistics;

#[test]
fn domain_preview_roundtrip_retains_exclusions_and_rejects_unbounded_or_failed_counts() {
    let mut original = snapshot(Origin::RecordingReplay, 0);
    let legacy = serde_json::to_value(&original).unwrap();
    assert!(legacy.get("domain_rejections").is_none());
    original.domain_rejections = Some(DomainSummary {
        rejected_trace_vertices: 1,
        rejected_trace_segments: 2,
        rejected_sheet_projections: 3,
    });
    original.validate().unwrap();
    let value = serde_json::to_value(&original).unwrap();
    let copy: Snapshot = serde_json::from_value(value).unwrap();
    assert_eq!(copy, original);
    for kind in 0..4 {
        let mut candidate: Snapshot =
            serde_json::from_value(serde_json::to_value(&original).unwrap()).unwrap();
        let summary = candidate.domain_rejections.as_mut().unwrap();
        match kind {
            0 => summary.rejected_trace_vertices = 65537,
            1 => summary.rejected_trace_segments = 65537,
            2 => summary.rejected_sheet_projections = 257,
            _ => {
                candidate.processing_error = Some("failed".into());
                candidate.cameras = None;
            }
        }
        assert!(candidate.validate().is_err());
    }
}
pub(crate) fn snapshot(origin: Origin, index: u64) -> Snapshot {
    Snapshot {
        schema_version: 1,
        origin,
        indices: [index; 2],
        host_timestamp_us: [1000 + index; 2],
        raw_pts: index as u32,
        frames: [0, 2].map(|interface| FrameView {
            interface,
            source_size: [2, 2],
            size: [2, 2],
            pixels: vec![0, 30, 60, 255],
            statistics: GrayStatistics {
                pixels: 4,
                minimum: 0,
                maximum: 255,
                mean: 86.25,
                p01: 0,
                median: 30,
                p99: 255,
                black_clipped: 1,
                white_clipped: 1,
            },
        }),
        positions: vec![[1., 2., 3.]],
        queries: 1,
        query_failures: 0,
        without_unique: 0,
        cameras: Some([0, 2].map(|interface| CameraSummary {
            interface,
            observations: 4,
            supported_traces: 1,
            supported_observations: 4,
            rejected_border_runs: 0,
            rejected_width_runs: 0,
            rejected_peak_runs: 0,
            rejected_saturation_runs: 0,
            continuity_ambiguous_observations: 0,
            continuity_comparisons: 3,
        })),
        processing_error: None,
        domain_rejections: None,
        acquisition: ProgressSnapshot {
            journaled_frames: [index + 1; 2],
            processing_queue_drops: [0; 2],
        },
        helper_omissions: 0,
    }
}
#[test]
fn display_validation_rejects_nonfinite_inconsistent_and_excessive_data() {
    for change in [
        |s: &mut Snapshot| s.positions[0][0] = f64::NAN,
        |s: &mut Snapshot| s.positions = vec![[0.; 3]; MAX_QUERIES + 1],
        |s: &mut Snapshot| s.queries = usize::MAX,
        |s: &mut Snapshot| s.without_unique = 1,
        |s: &mut Snapshot| s.frames[0].statistics.mean = f64::INFINITY,
        |s: &mut Snapshot| s.frames[0].statistics.p99 = 29,
        |s: &mut Snapshot| s.frames[0].source_size = [u32::MAX; 2],
        |s: &mut Snapshot| s.frames[0].size = [usize::MAX; 2],
        |s: &mut Snapshot| s.acquisition.journaled_frames = [0; 2],
        |s: &mut Snapshot| s.acquisition.processing_queue_drops = [2; 2],
        |s: &mut Snapshot| s.cameras.as_mut().unwrap()[0].supported_observations = 5,
        |s: &mut Snapshot| s.frames[1].interface = 0,
        |s: &mut Snapshot| s.processing_error = Some("bad".into()),
    ] {
        let mut s = snapshot(Origin::Native, 0);
        s.validate().unwrap();
        change(&mut s);
        assert!(s.validate().is_err());
    }
}
#[test]
fn decode_caps_precede_geometry_allocation_and_unknown_fields_are_rejected() {
    let mut v = serde_json::to_value(snapshot(Origin::RecordingReplay, 3)).unwrap();
    v["positions"] = serde_json::json!(vec![[0.; 3]; MAX_QUERIES + 1]);
    assert!(
        serde_json::from_value::<Snapshot>(v.clone())
            .unwrap_err()
            .to_string()
            .contains("point cap")
    );
    v["positions"] = serde_json::json!([[1., 2., 3.]]);
    v["frames"][0]["pixels"] = serde_json::json!(vec![0u8; 76801]);
    assert!(
        serde_json::from_value::<Snapshot>(v)
            .unwrap_err()
            .to_string()
            .contains("pixel budget")
    );
    let mut v = serde_json::to_value(snapshot(Origin::Native, 0)).unwrap();
    v["unexpected"] = true.into();
    assert!(serde_json::from_value::<Snapshot>(v).is_err());
}

pub(crate) fn raw_pair(
    index: u64,
) -> (
    scancraft_workflow::laser_stream::Pair,
    [scancraft_core::StreamProfile; 2],
    scancraft_workflow::laser_stream::bundle::PairOutcome,
    ProgressSnapshot,
) {
    use scancraft_workflow::laser_stream::{Pair, Sample};
    let mut metadata = vec![0; 16];
    metadata[10] = 6;
    metadata[11] = 4;
    metadata[12..16].copy_from_slice(&(index as u32).to_le_bytes());
    (
        Pair {
            samples: std::array::from_fn(|_| Sample {
                index,
                frame: scancraft_core::Frame {
                    sequence: index as u32,
                    host_timestamp_us: 1000 + index,
                    flags: 0,
                    metadata: metadata.clone(),
                    data: vec![0, 30, 60, 255],
                },
            }),
            raw_pts: index as u32,
        },
        [0, 2].map(|interface| scancraft_core::StreamProfile {
            interface,
            width: 2,
            height: 2,
            fourcc: *b"GREY",
            fps: 30,
        }),
        Err("diagnostic processing error".into()),
        ProgressSnapshot {
            journaled_frames: [index + 1; 2],
            processing_queue_drops: [0; 2],
        },
    )
}
#[test]
fn owned_preview_keeps_exact_original_pixels_statistics_and_result_after_source_reuse() {
    use scancraft_geometry::work::WorkControl;
    use scancraft_workflow::laser_stream::bundle::PreviewInput;
    let (mut pair, profiles, outcome, progress) = raw_pair(3);
    let work = WorkControl::default();
    let pending = Pending::capture(
        PreviewInput {
            pair: &pair,
            profiles: &profiles,
            outcome: &outcome,
            progress,
        },
        Origin::RecordingReplay,
        &work,
    )
    .unwrap();
    for sample in &mut pair.samples {
        sample.frame.data.fill(77);
        sample.frame.metadata.clear();
    }
    let received = pending.finish(&work).unwrap();
    let mut expected = snapshot(Origin::RecordingReplay, 3);
    expected.positions.clear();
    expected.queries = 0;
    expected.cameras = None;
    expected.processing_error = Some("diagnostic processing error".into());
    assert_eq!(received, expected);
    assert!(pair.samples.iter().all(|s| s.frame.data == [77; 4]));
}
#[test]
fn pending_input_bounds_validation_and_cancellation_remain_separate_from_wire_validation() {
    use scancraft_geometry::work::WorkControl;
    use scancraft_workflow::laser_stream::bundle::PreviewInput;
    for invalid in 0..4 {
        let (mut pair, mut profiles, outcome, progress) = raw_pair(0);
        match invalid {
            0 => pair.samples[0].frame.metadata = vec![0; 4097],
            1 => pair.samples[0].frame.data.truncate(3),
            2 => profiles[0].width = 4_194_305,
            _ => pair.samples[0].frame.flags = 0x40,
        };
        let work = WorkControl::default();
        assert!(
            Pending::capture(
                PreviewInput {
                    pair: &pair,
                    profiles: &profiles,
                    outcome: &outcome,
                    progress
                },
                Origin::Native,
                &work
            )
            .is_err()
        );
        assert!(work.check().is_ok());
    }
    let (pair, profiles, _, progress) = raw_pair(0);
    let error = "界".repeat(2000);
    let outcome = Err(error);
    let work = WorkControl::default();
    let pending = Pending::capture(
        PreviewInput {
            pair: &pair,
            profiles: &profiles,
            outcome: &outcome,
            progress,
        },
        Origin::Native,
        &work,
    )
    .unwrap();
    let snapshot = pending.finish(&work).unwrap();
    assert_eq!(snapshot.processing_error.unwrap().chars().count(), 1024);
    let pending = Pending::capture(
        PreviewInput {
            pair: &pair,
            profiles: &profiles,
            outcome: &outcome,
            progress,
        },
        Origin::Native,
        &work,
    )
    .unwrap();
    work.cancel();
    assert!(pending.finish(&work).is_err());
    assert!(
        Pending::capture(
            PreviewInput {
                pair: &pair,
                profiles: &profiles,
                outcome: &outcome,
                progress
            },
            Origin::Native,
            &work
        )
        .is_err()
    );
}

#[test]
fn maximum_raw_payload_and_metadata_are_preserved_without_sampling() {
    use scancraft_geometry::work::WorkControl;
    use scancraft_workflow::laser_stream::bundle::PreviewInput;
    let (mut pair, mut profiles, outcome, progress) = raw_pair(0);
    for (profile, sample) in profiles.iter_mut().zip(&mut pair.samples) {
        profile.width = 2048;
        profile.height = 2048;
        sample.frame.data = vec![17; scancraft_image::MAX_PIXELS];
        sample.frame.metadata = sample.frame.metadata.repeat(256);
        assert_eq!(sample.frame.metadata.len(), 4096);
    }
    let work = WorkControl::default();
    let pending = Pending::capture(
        PreviewInput {
            pair: &pair,
            profiles: &profiles,
            outcome: &outcome,
            progress,
        },
        Origin::Native,
        &work,
    )
    .unwrap();
    let snapshot = pending.finish(&work).unwrap();
    for frame in &snapshot.frames {
        assert_eq!(frame.source_size, [2048, 2048]);
        assert_eq!(frame.statistics.pixels, scancraft_image::MAX_PIXELS);
        assert_eq!(frame.statistics.minimum, 17);
        assert_eq!(frame.statistics.maximum, 17);
        assert_eq!(frame.statistics.mean, 17.);
        assert!(frame.pixels.iter().all(|&v| v == 17));
    }
}

#[test]
fn dense_preview_accepts_exact_shared_capacity_without_promoting_geometry() {
    let mut s = snapshot(Origin::RecordingReplay, 0);
    s.positions = vec![[1., 2., 3.]; MAX_QUERIES];
    s.queries = MAX_QUERIES;
    s.validate().unwrap();
    let decoded: Snapshot = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    assert_eq!(decoded, s);
    s.queries += 1;
    assert!(s.validate().is_err());
}
