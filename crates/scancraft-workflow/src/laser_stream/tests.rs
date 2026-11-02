use super::*;
use crate::live_stereo::runner::Source;
use scancraft_calibration::{Brown5, CameraModel, stereo::RigidPose};
use scancraft_calibration::{
    laser_surface::QuadraticSheet, laser_trace, stereo::TriangulationLimits,
};
use scancraft_core::Frame;
use scancraft_depth::laser_pair::{Continuity, Detection, Settings};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::{CaptureContext, Reader, Writer};
use serde_json::{Value, json};
use std::sync::{
    Arc, Barrier, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};

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
fn frame(template: &Frame, index: u64, pts: u32, host: u64) -> Frame {
    let mut f = template.clone();
    f.sequence = index as u32;
    f.host_timestamp_us = host;
    f.metadata[12..16].copy_from_slice(&pts.to_le_bytes());
    f
}
#[test]
fn constant_size_association_survives_long_indices_and_counter_wrap() {
    let (_, _, profiles, frames) = fixture();
    let mut a = Association::new(profiles, 5000, 1000).unwrap();
    for i in 0..4000u64 {
        let pts = (i as u32).wrapping_mul(1 << 30);
        for camera in [Camera::Second, Camera::First] {
            let result = a
                .push(
                    camera,
                    Sample {
                        index: i,
                        frame: frame(&frames[camera.index()], i, pts, 1000 + i * 2),
                    },
                )
                .unwrap();
            if camera.index() == 0 {
                let pair = result.unwrap();
                assert_eq!(pair.raw_pts, pts);
                assert_eq!(pair.samples.each_ref().map(|p| p.index), [i, i]);
            } else {
                assert!(result.is_none());
            }
        }
        assert_eq!(a.remaining(), [0, 0]);
    }
    assert_eq!(a.counts.delivered, [4000, 4000]);
    assert_eq!(a.counts.paired, 4000);
}
#[test]
fn association_rejects_ambiguous_order_and_keeps_pending_host_and_epoch_failures_explicit() {
    let (_, _, profiles, frames) = fixture();
    let make = || Association::new(profiles.clone(), 10000, 10).unwrap();
    let mut a = make();
    for i in 0..5 {
        assert!(
            a.push(
                Camera::First,
                Sample {
                    index: i,
                    frame: frame(&frames[0], i, i as u32, 100 + i)
                }
            )
            .unwrap()
            .is_none()
        );
    }
    assert_eq!(a.remaining(), [2, 0]);
    assert_eq!(a.counts.evicted_pending, [3, 0]);
    assert!(
        a.push(
            Camera::Second,
            Sample {
                index: 4,
                frame: frame(&frames[1], 4, 4, 1000)
            }
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(a.counts.rejected_host_window_pairs, 1);
    for (index, pts, host) in [
        (4, 5, 1100),
        (5, 4, 1100),
        (5, 3, 1100),
        (5, 4 + (1 << 31), 1100),
        (5, 5, 100),
    ] {
        let mut a = make();
        a.push(
            Camera::First,
            Sample {
                index: 4,
                frame: frame(&frames[0], 4, 4, 100),
            },
        )
        .unwrap();
        assert!(
            a.push(
                Camera::First,
                Sample {
                    index,
                    frame: frame(&frames[0], index, pts, host)
                }
            )
            .is_err()
        );
    }
    let mut a = make();
    a.push(
        Camera::First,
        Sample {
            index: 0,
            frame: frame(&frames[0], 0, 0, 100),
        },
    )
    .unwrap();
    assert!(
        a.push(
            Camera::Second,
            Sample {
                index: 0,
                frame: frame(&frames[1], 0, 1 << 31, 101)
            }
        )
        .is_err()
    );
    // Established old peer's identical raw value must not match a later cycle,
    // even under an adversarial host window that would otherwise admit it.
    let mut a = make();
    a.push(
        Camera::First,
        Sample {
            index: 0,
            frame: frame(&frames[0], 0, 1, 100),
        },
    )
    .unwrap();
    a.push(
        Camera::Second,
        Sample {
            index: 0,
            frame: frame(&frames[1], 0, 0, 100),
        },
    )
    .unwrap();
    for (i, pts) in [
        (1, 1 + (1 << 30)),
        (2, 1 + (1 << 31)),
        (3, 1 + (3 << 30)),
        (4, 0),
    ] {
        assert!(
            a.push(
                Camera::First,
                Sample {
                    index: i,
                    frame: frame(&frames[0], i, pts, 100 + i)
                }
            )
            .unwrap()
            .is_none()
        );
    }
    assert_eq!(a.counts.paired, 0);
}
struct Fake {
    template: Frame,
    next: u64,
    count: u64,
    gate: Option<Arc<Barrier>>,
    gate_once: bool,
    token: StopToken,
    closed: Arc<AtomicUsize>,
    fail_close: bool,
    fail_next: bool,
}
impl Source for Fake {
    fn context(&self) -> Result<CaptureContext> {
        Ok(CaptureContext::default())
    }
    fn next(&mut self) -> Result<Option<Frame>> {
        if self.fail_next {
            return Err("injected source failure".into());
        }
        if self.next == self.count || self.token.stop_requested() {
            return Ok(None);
        }
        if self.next > 0
            && (!self.gate_once || self.next == 1)
            && let Some(gate) = &self.gate
        {
            gate.wait();
        }
        let i = self.next;
        self.next += 1;
        Ok(Some(frame(
            &self.template,
            i,
            (u32::MAX - 1500).wrapping_add(i as u32 * 1000),
            1000 + i * 2,
        )))
    }
    fn statistics(&self) -> Value {
        json!({"returned":self.next})
    }
    fn close(self) -> Result<()> {
        self.closed.fetch_add(1, Ordering::Relaxed);
        if self.fail_close {
            Err("injected cleanup failure".into())
        } else {
            Ok(())
        }
    }
}
struct MemoryJournal {
    writer: Writer<Vec<u8>>,
    output: Arc<Mutex<Vec<Vec<u8>>>>,
    pushed: Arc<AtomicU64>,
    fail_push: bool,
    fail_finish: bool,
}
impl Journal for MemoryJournal {
    fn push(&mut self, f: &Frame) -> Result<()> {
        if self.fail_push {
            return Err("injected journal write failure".into());
        }
        self.writer.push(f).map_err(|e| e.to_string())?;
        self.pushed.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    fn finish(self) -> Result<()> {
        if self.fail_finish {
            return Err("injected journal finish failure".into());
        }
        self.output
            .lock()
            .unwrap()
            .push(self.writer.finish().map_err(|e| e.to_string())?);
        Ok(())
    }
}
#[test]
fn runner_preserves_raw_journals_and_full_pair_results_beyond_one_thousand_frames() {
    let (geometry, settings, _, frames) = fixture();
    let pipeline = Pipeline::new(&geometry, &settings, 30).unwrap();
    let work = Arc::new(WorkControl::default());
    let stop = Arc::new(AtomicBool::new(false));
    let gate = Arc::new(Barrier::new(3));
    let closed = Arc::new(AtomicUsize::new(0));
    let output = Arc::new(Mutex::new(Vec::new()));
    let pushed = Arc::new(AtomicU64::new(0));
    let n = 1200;
    let templates = frames.clone();
    let source_gate = gate.clone();
    let source_closed = closed.clone();
    let journal_out = output.clone();
    let journal_push = pushed.clone();
    let mut delivered = 0;
    let report = pipeline
        .run(
            n,
            &work,
            &stop,
            move |camera, _, token| {
                Ok(Fake {
                    template: templates[camera.index()].clone(),
                    next: 0,
                    count: n,
                    gate: Some(source_gate.clone()),
                    gate_once: false,
                    token,
                    closed: source_closed.clone(),
                    fail_close: false,
                    fail_next: false,
                })
            },
            move |_, profile, context| {
                Ok(MemoryJournal {
                    writer: Writer::with_context(Vec::new(), profile, context)
                        .map_err(|e| e.to_string())?,
                    output: journal_out.clone(),
                    pushed: journal_push.clone(),
                    fail_push: false,
                    fail_finish: false,
                })
            },
            |pair, result| {
                let r = result?;
                assert!(pushed.load(Ordering::Relaxed) >= 2 * (delivered + 1));
                let direct = pipeline
                    .processor
                    .process(
                        [
                            laser_pair::FrameInput {
                                profile: &pipeline.profiles[0],
                                frame: &pair.samples[0].frame,
                            },
                            laser_pair::FrameInput {
                                profile: &pipeline.profiles[1],
                                frame: &pair.samples[1].frame,
                            },
                        ],
                        &work,
                        &mut (),
                    )
                    .unwrap();
                assert_eq!(
                    serde_json::to_value(&r).unwrap(),
                    serde_json::to_value(direct).unwrap()
                );
                assert!(!r.physical_accuracy_qualified);
                delivered += 1;
                if delivered < n {
                    gate.wait();
                }
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(delivered, n);
    assert_eq!(closed.load(Ordering::Relaxed), 2);
    assert_eq!(report.processed_pairs, n);
    assert_eq!(report.failed_processing_pairs, 0);
    assert_eq!(report.unmatched_pending, [0, 0]);
    assert_eq!(report.undelivered_queued_frames, [Some(0), Some(0)]);
    for source in report.sources {
        assert_eq!(source.journaled_frames, n);
        assert_eq!(source.processing_queue_drops, 0);
        assert!(source.cleanup_confirmed);
        assert!(source.journal_finalized);
    }
    for bytes in output.lock().unwrap().iter() {
        let mut r = Reader::new(bytes.as_slice()).unwrap();
        let mut count = 0;
        while let Some(f) = r.read_frame().unwrap() {
            assert_eq!(f.host_timestamp_us, 1000 + count * 2);
            count += 1;
        }
        assert_eq!(count, n);
    }
}
#[test]
fn queue_pressure_omits_processing_only_after_required_raw_journal() {
    let (_, settings, profiles, frames) = fixture();
    let geometry = fixture().0;
    let pipeline = Pipeline::new(&geometry, &settings, 30).unwrap();
    let output = Arc::new(Mutex::new(Vec::new()));
    let pushed = Arc::new(AtomicU64::new(0));
    let closed = Arc::new(AtomicUsize::new(0));
    // Hold the consumer during a first pair while producers run to completion.
    // Once each source has been closed, all journals are final and queue pressure
    // is deterministic. A pair rendezvous prevents one camera racing all frames.
    let start = Arc::new(Barrier::new(3));
    let release = start.clone();
    let mut first = true;
    let n = 200;
    let done = closed.clone();
    let out = output.clone();
    let push = pushed.clone();
    let close = closed.clone();
    let source_frames = frames.clone();
    let work = Arc::new(WorkControl::default());
    let stop = Arc::new(AtomicBool::new(false));
    let report = pipeline
        .run(
            n,
            &work,
            &stop,
            move |c, _, token| {
                Ok(Fake {
                    template: source_frames[c.index()].clone(),
                    next: 0,
                    count: n,
                    gate: Some(start.clone()),
                    gate_once: true,
                    token,
                    closed: close.clone(),
                    fail_close: false,
                    fail_next: false,
                })
            },
            move |_, p, ctx| {
                Ok(MemoryJournal {
                    writer: Writer::with_context(Vec::new(), p, ctx).map_err(|e| e.to_string())?,
                    output: out.clone(),
                    pushed: push.clone(),
                    fail_push: false,
                    fail_finish: false,
                })
            },
            |_, r| {
                r?;
                if first {
                    first = false;
                    release.wait();
                }
                while done.load(Ordering::Relaxed) < 2 {
                    std::thread::yield_now();
                }
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(closed.load(Ordering::Relaxed), 2);
    assert!(
        report
            .sources
            .iter()
            .map(|s| s.processing_queue_drops)
            .sum::<u64>()
            > 0
    );
    for source in report.sources {
        assert_eq!(source.journaled_frames, n);
        assert_eq!(source.queued_frames + source.processing_queue_drops, n);
        assert!(source.cleanup_confirmed);
        assert!(source.journal_finalized);
    }
    assert_eq!(pushed.load(Ordering::Relaxed), 2 * n);
    assert_eq!(output.lock().unwrap().len(), 2);
    assert_eq!(pipeline.profiles()[0], profiles[0]);
}

#[test]
fn processing_admission_errors_are_delivered_while_raw_recording_continues() {
    let (geometry, mut settings, _, frames) = fixture();
    settings.maximum_total_work = 1;
    let pipeline = Pipeline::new(&geometry, &settings, 30).unwrap();
    let gate = Arc::new(Barrier::new(3));
    let source_gate = gate.clone();
    let closed = Arc::new(AtomicUsize::new(0));
    let close = closed.clone();
    let output = Arc::new(Mutex::new(Vec::new()));
    let out = output.clone();
    let pushed = Arc::new(AtomicU64::new(0));
    let mut calls = 0;
    let result = pipeline
        .run(
            3,
            &Arc::new(WorkControl::default()),
            &Arc::new(AtomicBool::new(false)),
            move |c, _, token| {
                Ok(Fake {
                    template: frames[c.index()].clone(),
                    next: 0,
                    count: 3,
                    gate: Some(source_gate.clone()),
                    gate_once: false,
                    token,
                    closed: close.clone(),
                    fail_close: false,
                    fail_next: false,
                })
            },
            move |_, p, ctx| {
                Ok(MemoryJournal {
                    writer: Writer::with_context(Vec::new(), p, ctx).unwrap(),
                    output: out.clone(),
                    pushed: pushed.clone(),
                    fail_push: false,
                    fail_finish: false,
                })
            },
            |_, r| {
                assert!(r.unwrap_err().contains("work budget"));
                calls += 1;
                if calls < 3 {
                    gate.wait();
                }
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(calls, 3);
    assert_eq!(result.failed_processing_pairs, 3);
    assert_eq!(
        result
            .sources
            .iter()
            .map(|s| s.journaled_frames)
            .collect::<Vec<_>>(),
        vec![3, 3]
    );
    assert_eq!(closed.load(Ordering::Relaxed), 2);
    assert_eq!(output.lock().unwrap().len(), 2);
}

#[test]
fn consumer_error_and_panic_stop_sources_with_retained_counts_and_cleanup() {
    for panic in [false, true] {
        let (geometry, settings, _, frames) = fixture();
        let pipeline = Pipeline::new(&geometry, &settings, 30).unwrap();
        let gate = Arc::new(Barrier::new(3));
        let source_gate = gate.clone();
        let closed = Arc::new(AtomicUsize::new(0));
        let close = closed.clone();
        let output = Arc::new(Mutex::new(Vec::new()));
        let pushed = Arc::new(AtomicU64::new(0));
        let failure = pipeline
            .run(
                100,
                &Arc::new(WorkControl::default()),
                &Arc::new(AtomicBool::new(false)),
                move |c, _, token| {
                    Ok(Fake {
                        template: frames[c.index()].clone(),
                        next: 0,
                        count: 100,
                        gate: Some(source_gate.clone()),
                        gate_once: true,
                        token,
                        closed: close.clone(),
                        fail_close: false,
                        fail_next: false,
                    })
                },
                move |_, p, ctx| {
                    Ok(MemoryJournal {
                        writer: Writer::with_context(Vec::new(), p, ctx).unwrap(),
                        output: output.clone(),
                        pushed: pushed.clone(),
                        fail_push: false,
                        fail_finish: false,
                    })
                },
                |_, r| {
                    r?;
                    gate.wait();
                    if panic {
                        panic!("injected consumer panic");
                    }
                    Err("injected consumer failure".into())
                },
            )
            .unwrap_err();
        assert!(failure.reason.contains(if panic {
            "panicked"
        } else {
            "consumer failure"
        }));
        let partial = failure.partial.unwrap();
        assert_eq!(partial.processed_pairs, 1);
        assert_eq!(closed.load(Ordering::Relaxed), 2);
        assert!(
            partial
                .sources
                .iter()
                .all(|s| s.opened && s.cleanup_confirmed && s.journaled_frames >= 1)
        );
    }
}

#[test]
fn journal_source_and_cleanup_errors_remain_failures_and_close_every_opened_source() {
    for mode in ["write", "finish", "source", "close", "factory", "metadata"] {
        let (geometry, settings, _, frames) = fixture();
        let pipeline = Pipeline::new(&geometry, &settings, 30).unwrap();
        let closed = Arc::new(AtomicUsize::new(0));
        let close = closed.clone();
        let opened = Arc::new(AtomicUsize::new(0));
        let open = opened.clone();
        let output = Arc::new(Mutex::new(Vec::new()));
        let pushed = Arc::new(AtomicU64::new(0));
        let failure = pipeline
            .run(
                2,
                &Arc::new(WorkControl::default()),
                &Arc::new(AtomicBool::new(false)),
                move |c, _, token| {
                    open.fetch_add(1, Ordering::Relaxed);
                    Ok(Fake {
                        template: {
                            let mut f = frames[c.index()].clone();
                            if mode == "metadata" {
                                f.metadata.resize(4097, 0);
                            }
                            f
                        },
                        next: 0,
                        count: 2,
                        gate: None,
                        gate_once: false,
                        token,
                        closed: close.clone(),
                        fail_close: mode == "close",
                        fail_next: mode == "source",
                    })
                },
                move |_, p, ctx| {
                    if mode == "factory" {
                        return Err("injected journal factory failure".into());
                    }
                    Ok(MemoryJournal {
                        writer: Writer::with_context(Vec::new(), p, ctx).unwrap(),
                        output: output.clone(),
                        pushed: pushed.clone(),
                        fail_push: mode == "write",
                        fail_finish: mode == "finish",
                    })
                },
                |_, _| Ok(()),
            )
            .unwrap_err();
        assert!(failure.reason.contains(if mode == "metadata" {
            "metadata exceeds"
        } else {
            "injected"
        }));
        assert_eq!(
            closed.load(Ordering::Relaxed),
            opened.load(Ordering::Relaxed)
        );
        let partial = failure.partial.unwrap();
        assert!(partial.sources.iter().any(|s| s.outcome.is_err()));
        for s in partial.sources.iter().filter(|s| s.opened) {
            assert_eq!(s.cleanup_confirmed, mode != "close");
            if mode == "finish" || mode == "close" {
                assert!(s.journaled_frames <= 2);
            } else {
                assert_eq!(s.journaled_frames, 0);
            }
        }
    }
}

#[test]
fn cancellation_and_graceful_stop_are_observed_without_opening_or_abandoning_sources() {
    let (geometry, settings, _, frames) = fixture();
    let pipeline = Pipeline::new(&geometry, &settings, 30).unwrap();
    let work = Arc::new(WorkControl::default());
    work.cancel();
    let stop = Arc::new(AtomicBool::new(false));
    let never_open = |_, _, _: StopToken| -> Result<Fake> { panic!("must not open") };
    let never_journal = |_, _, _| -> Result<MemoryJournal> { panic!("must not journal") };
    assert!(
        pipeline
            .run(2, &work, &stop, never_open, never_journal, |_, _| Ok(()))
            .unwrap_err()
            .partial
            .is_none()
    );
    let ready = Arc::new(WorkControl::default());
    assert!(
        pipeline
            .run(0, &ready, &stop, never_open, never_journal, |_, _| Ok(()))
            .unwrap_err()
            .partial
            .is_none()
    );
    stop.store(true, Ordering::Relaxed);
    let s = pipeline
        .run(2, &ready, &stop, never_open, never_journal, |_, _| Ok(()))
        .unwrap();
    assert!(s.graceful_stop_requested);
    assert!(s.sources.iter().all(|s| !s.opened));
    for cancel in [false, true] {
        let work = Arc::new(WorkControl::default());
        let stop = Arc::new(AtomicBool::new(false));
        let gate = Arc::new(Barrier::new(3));
        let source_gate = gate.clone();
        let closed = Arc::new(AtomicUsize::new(0));
        let close = closed.clone();
        let templates = frames.clone();
        let output = Arc::new(Mutex::new(Vec::new()));
        let pushed = Arc::new(AtomicU64::new(0));
        let mut first = true;
        let result = pipeline.run(
            100,
            &work,
            &stop,
            move |c, _, token| {
                Ok(Fake {
                    template: templates[c.index()].clone(),
                    next: 0,
                    count: 100,
                    gate: Some(source_gate.clone()),
                    gate_once: true,
                    token,
                    closed: close.clone(),
                    fail_close: false,
                    fail_next: false,
                })
            },
            move |_, p, ctx| {
                Ok(MemoryJournal {
                    writer: Writer::with_context(Vec::new(), p, ctx).unwrap(),
                    output: output.clone(),
                    pushed: pushed.clone(),
                    fail_push: false,
                    fail_finish: false,
                })
            },
            |_, r| {
                r?;
                if first {
                    first = false;
                    gate.wait();
                    if cancel {
                        work.cancel();
                    } else {
                        stop.store(true, Ordering::Relaxed);
                    }
                }
                Ok(())
            },
        );
        assert_eq!(closed.load(Ordering::Relaxed), 2);
        if cancel {
            assert!(result.unwrap_err().reason.contains("cancelled"));
        } else {
            assert!(result.unwrap().graceful_stop_requested);
        }
    }
}
