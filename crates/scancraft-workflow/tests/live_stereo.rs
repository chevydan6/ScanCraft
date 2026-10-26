use scancraft_core::{Frame, StreamProfile};
use scancraft_geometry::work::WorkControl;
use scancraft_workflow::live_stereo::{Association, Pair, Processor, Sample, Side};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(
            scancraft_workflow::output::staging(&std::env::temp_dir().join("live-stereo-test"))
                .unwrap(),
        )
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn profile(interface: u8) -> StreamProfile {
    StreamProfile {
        interface,
        width: 64,
        height: 32,
        fourcc: *b"GREY",
        fps: 30,
    }
}
fn sample(index: usize, pts: u32, host: u64, data: Vec<u8>) -> Sample {
    let mut metadata = vec![0; 16];
    metadata[10] = 6;
    metadata[11] = 4;
    metadata[12..16].copy_from_slice(&pts.to_le_bytes());
    Sample {
        index,
        frame: Frame {
            sequence: index as u32,
            host_timestamp_us: host,
            flags: 0,
            metadata,
            data,
        },
    }
}
fn source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset")
}
fn settings(path: &Path) {
    fs::write(path,serde_json::to_vec(&json!({"schema_version":1,"session_id":"synthetic-live","fps":30,"maximum_frames_per_camera":8,
 "maximum_host_delta_us":1000,"tracking_samples":512,"voxel_mm":0.1,"maximum_voxels":1000,
 "tracking":{"max_correspondence_mm":10.0,"max_rms_mm":0.01,"minimum_overlap":0.8,"max_step_translation_mm":1.0,
 "max_step_rotation_rad":0.1,"iterations":40,"tolerance_mm":1e-8}})).unwrap()).unwrap();
}
fn processor(t: &Temp, work: &WorkControl) -> Processor {
    settings(&t.0.join("scan.json"));
    let s = source();
    Processor::open(
        &s.join("calibration.rapcal"),
        &s.join("request.json"),
        &t.0.join("scan.json"),
        scancraft_workflow::live_stereo::InputMode::SyntheticFixture,
        work,
    )
    .unwrap()
}
fn images() -> [Vec<u8>; 2] {
    ["left.png", "right.png"].map(|n| image::open(source().join(n)).unwrap().to_luma8().into_raw())
}
fn pair(index: usize, images: &[Vec<u8>; 2]) -> Pair {
    let pts = 1000 + index as u32 * 33333;
    Pair {
        left: sample(index, pts, u64::from(pts), images[0].clone()),
        right: sample(index, pts, u64::from(pts) + 10, images[1].clone()),
        pts,
    }
}
#[test]
fn association_pairs_interleaved_wrap_and_accounts_capacity_and_host_rejection() {
    let mut a = Association::new([profile(0), profile(2)], 8, 20).unwrap();
    let bytes = vec![0; 2048];
    for (i, pts) in [(0, u32::MAX - 100), (1, 100), (2, 200)] {
        assert!(
            a.push(
                Side::Left,
                sample(i, pts, 1000 + i as u64 * 100, bytes.clone())
            )
            .unwrap()
            .is_none()
        );
    }
    assert_eq!(a.counts.evicted_pending, [1, 0]);
    let p = a
        .push(Side::Right, sample(1, 100, 1110, bytes.clone()))
        .unwrap()
        .unwrap();
    assert_eq!(p.left.index, 1);
    assert_eq!(p.right.index, 1);
    assert!(
        a.push(Side::Right, sample(2, 200, 1300, bytes))
            .unwrap()
            .is_none()
    );
    assert_eq!(a.counts.paired, 1);
    assert_eq!(a.counts.rejected_host_window_pairs, 1);
    assert_eq!(a.remaining(), [0, 0]);
    assert_eq!(a.counts.delivered, [3, 2]);
}
#[test]
fn association_rejects_repeated_reordered_oversize_and_corrupt_sources() {
    let bytes = vec![0; 2048];
    let mut a = Association::new([profile(0), profile(2)], 8, 20).unwrap();
    a.push(Side::Left, sample(0, 100, 1000, bytes.clone()))
        .unwrap();
    assert!(
        a.push(Side::Left, sample(1, 100, 1100, bytes.clone()))
            .is_err()
    );
    assert!(
        a.push(Side::Left, sample(0, 200, 1100, bytes.clone()))
            .is_err()
    );
    assert!(
        a.push(Side::Right, sample(8, 200, 1100, bytes.clone()))
            .is_err()
    );
    let mut invalid = sample(0, 100, 1000, bytes);
    invalid.frame.flags = 0x40;
    assert!(a.push(Side::Right, invalid).is_err());
    let mut zero = profile(0);
    zero.width = 0;
    assert!(Association::new([zero, profile(2)], 8, 20).is_err());
    assert!(Association::new([profile(0), profile(2)], 1001, 20).is_err());
}
#[test]
fn incremental_reconstruction_retains_geometry_and_rejects_reused_pairs() {
    let t = Temp::new();
    let work = WorkControl::default();
    let mut p = processor(&t, &work);
    let images = images();
    let first = pair(0, &images);
    let a = p.process(&first, &work).unwrap();
    assert_eq!(a["tracking"]["integrated"], true);
    assert_eq!(a["synchronization_verified"], false);
    assert_eq!(a["physical_accuracy_qualified"], false);
    assert!(p.process(&first, &work).is_err());
    let b = p.process(&pair(1, &images), &work).unwrap();
    assert_eq!(b["tracking"]["integrated"], true);
    let cloud = p.finish(&work).unwrap();
    assert!(!cloud.points.is_empty());
    assert!(
        cloud
            .points
            .iter()
            .all(|v| v.position.iter().all(|v| v.is_finite()))
    );
}
#[test]
fn no_geometry_does_not_invent_pose_or_poison_later_tracking() {
    let t = Temp::new();
    let work = WorkControl::default();
    let mut p = processor(&t, &work);
    let images = images();
    p.process(&pair(0, &images), &work).unwrap();
    let blank = [vec![0; 2048], vec![0; 2048]];
    let lost = p.process(&pair(1, &blank), &work).unwrap();
    assert_eq!(lost["tracking"]["state"], "no_geometry");
    assert_eq!(lost["tracking"]["integrated"], false);
    assert!(lost["tracking"]["world_from_frame"].is_null());
    let recovery = p.process(&pair(2, &images), &work).unwrap();
    assert_eq!(recovery["tracking"]["integrated"], true);
}
#[test]
fn physical_mode_rejects_synthetic_before_any_acquisition_and_cancel_does_not_consume_pair() {
    let t = Temp::new();
    let work = WorkControl::default();
    let mut p = processor(&t, &work);
    let s = source();
    assert!(
        Processor::open(
            &s.join("calibration.rapcal"),
            &s.join("request.json"),
            &t.0.join("scan.json"),
            scancraft_workflow::live_stereo::InputMode::PhysicalCapture,
            &work
        )
        .is_err()
    );
    let cancelled = WorkControl::default();
    cancelled.cancel();
    let candidate = pair(0, &images());
    assert!(p.process(&candidate, &cancelled).is_err());
    assert_eq!(
        p.process(&candidate, &work).unwrap()["tracking"]["integrated"],
        true
    );
}

#[test]
fn wrong_depth_loses_tracking_without_fusion_then_recovers() {
    let t = Temp::new();
    let work = WorkControl::default();
    let mut p = processor(&t, &work);
    let original = images();
    assert_eq!(
        p.process(&pair(0, &original), &work).unwrap()["tracking"]["integrated"],
        true
    );
    let mut wrong = original.clone();
    wrong[1].fill(0);
    for y in 0..32 {
        for x in 0..56 {
            wrong[1][y * 64 + x] = 2 * original[0][y * 64 + x + 8] + 20;
        }
    }
    // This existing synthetic rig's right sensor is rotated 180 degrees.
    wrong[1].reverse();
    let lost = p.process(&pair(1, &wrong), &work).unwrap();
    assert_eq!(lost["tracking"]["state"], "Lost");
    assert_eq!(lost["tracking"]["integrated"], false);
    assert!(lost["tracking"]["world_from_frame"].is_null());
    let recovered = p.process(&pair(2, &original), &work).unwrap();
    assert_eq!(recovered["tracking"]["integrated"], true);
}

#[test]
fn pause_retains_raw_pair_lineage_without_geometry_then_resumes_in_same_world() {
    let t = Temp::new();
    let work = WorkControl::default();
    let mut p = processor(&t, &work);
    let pixels = images();
    p.set_paused(true).unwrap();
    let paused = p.process(&pair(0, &pixels), &work).unwrap();
    assert_eq!(paused["tracking"]["state"], "paused");
    assert!(paused["reconstruction"].is_null());
    assert_eq!(p.preview_cloud(&work).unwrap().0, 0);
    p.set_paused(false).unwrap();
    assert_eq!(
        p.process(&pair(1, &pixels), &work).unwrap()["tracking"]["integrated"],
        true
    );
    let before = p.preview_cloud(&work).unwrap();
    p.set_paused(true).unwrap();
    p.process(&pair(2, &pixels), &work).unwrap();
    assert_eq!(p.preview_cloud(&work).unwrap(), before);
    p.set_paused(false).unwrap();
    assert_eq!(
        p.process(&pair(3, &pixels), &work).unwrap()["tracking"]["integrated"],
        true
    );
    p.finish(&work).unwrap();
    assert!(p.set_paused(false).is_err());
}
