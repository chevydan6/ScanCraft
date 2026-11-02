use super::*;
use scancraft_image::diagnostics::GrayStatistics;
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
        |s: &mut Snapshot| s.positions = vec![[0.; 3]; 4097],
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
    v["positions"] = serde_json::json!(vec![[0.; 3]; 4097]);
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
