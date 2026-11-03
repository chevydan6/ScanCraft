use super::*;
use crate::live_stereo::{
    Side,
    control::Control,
    runner::{Source, recording::record_controlled},
};
use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::{CaptureContext, Reader};
use serde_json::Value;
use std::{fs, sync::mpsc, thread, time::Duration};

struct HeldSource {
    left: bool,
    mode: usize,
    reading: Arc<AtomicBool>,
    ready: Option<mpsc::Sender<()>>,
    wait: mpsc::Receiver<()>,
    closed: mpsc::Sender<bool>,
    control: Arc<Control>,
}
impl Source for HeldSource {
    fn context(&self) -> Result<CaptureContext, String> {
        Ok(CaptureContext::default())
    }
    fn next(&mut self) -> Result<Option<Frame>, String> {
        if !self.left {
            self.reading.store(true, Ordering::SeqCst);
            self.ready.take().unwrap().send(()).unwrap();
        }
        self.wait
            .recv_timeout(Duration::from_secs(10))
            .map_err(|e| e.to_string())?;
        if self.left {
            match self.mode {
                1 => return Err("test acquisition failure".into()),
                2 => panic!("test acquisition panic"),
                3 => {
                    self.control.request_stop();
                }
                4 => {
                    let mut metadata = vec![0; 16];
                    metadata[10] = 6;
                    metadata[11] = 4;
                    return Ok(Some(Frame {
                        sequence: 0,
                        host_timestamp_us: 0,
                        flags: 0,
                        metadata,
                        data: vec![0; 4],
                    }));
                }
                _ => (),
            }
        } else {
            self.reading.store(false, Ordering::SeqCst);
        }
        Ok(None)
    }
    fn statistics(&self) -> Value {
        Value::Null
    }
    fn close(self) -> Result<(), String> {
        let still_reading = self.reading.load(Ordering::SeqCst);
        self.closed.send(self.left).unwrap();
        if still_reading {
            Err("source closed while peer was acquiring".into())
        } else {
            Ok(())
        }
    }
}
fn profile(left: bool) -> StreamProfile {
    StreamProfile {
        interface: if left { 0 } else { 2 },
        width: 2,
        height: 2,
        fourcc: *b"GREY",
        fps: 30,
    }
}
#[test]
fn actual_worker_coordinates_normal_graceful_error_and_panic_before_close() {
    for mode in 0..5 {
        let root =
            crate::output::staging(&std::env::temp_dir().join("coordinated-live-worker")).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let control = Arc::new(Control::default());
        let [mut first, mut second] = participants(stop.clone());
        let observe = first.shared.clone();
        let reading = Arc::new(AtomicBool::new(false));
        let (ready, started) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let (closed, closes) = mpsc::channel();
        let left = HeldSource {
            left: true,
            mode,
            reading: reading.clone(),
            ready: None,
            wait: started,
            closed: closed.clone(),
            control: control.clone(),
        };
        let right = HeldSource {
            left: false,
            mode,
            reading,
            ready: Some(ready),
            wait: held,
            closed,
            control: control.clone(),
        };
        let path = root.join("left.raptor");
        let flag = stop.clone();
        let c = control.clone();
        let first_owner = thread::spawn(move || {
            let (tx, _rx) = mpsc::sync_channel(1);
            record_controlled(
                profile(true),
                &path,
                1,
                (&flag, &c, Some(&mut first)),
                tx,
                Side::Left,
                || Ok(left),
            )
        });
        let path = root.join("right.raptor");
        let flag = stop.clone();
        let c = control.clone();
        let second_owner = thread::spawn(move || {
            let (tx, _rx) = mpsc::sync_channel(1);
            record_controlled(
                profile(false),
                &path,
                1,
                (&flag, &c, Some(&mut second)),
                tx,
                Side::Right,
                || Ok(right),
            )
        });
        let lock = observe.finished.lock().unwrap();
        let (lock, timeout) = observe
            .changed
            .wait_timeout_while(lock, Duration::from_secs(5), |mask| *mask & 1 == 0)
            .unwrap();
        let arrived = *lock & 1 != 0;
        drop(lock);
        let early = closes.try_recv();
        let cancelled = stop.load(Ordering::Relaxed);
        let _ = release.send(());
        let left_result = first_owner.join().unwrap();
        let right_result = second_owner.join().unwrap();
        assert!(arrived && !timeout.timed_out());
        assert!(matches!(early, Err(mpsc::TryRecvError::Empty)));
        assert_eq!(cancelled, mode == 1 || mode == 2);
        assert_eq!(control.stop_requested(), mode == 3);
        assert!(right_result.is_ok());
        match mode {
            1 => assert!(
                left_result
                    .err()
                    .unwrap()
                    .contains("test acquisition failure")
            ),
            2 => assert!(
                left_result
                    .err()
                    .unwrap()
                    .contains("capture worker panicked")
            ),
            _ => assert!(left_result.is_ok()),
        }
        assert_eq!(closes.into_iter().count(), 2);
        for side in ["left", "right"] {
            let mut reader =
                Reader::new(fs::File::open(root.join(format!("{side}.raptor"))).unwrap()).unwrap();
            if mode == 4 && side == "left" {
                assert!(reader.read_frame().unwrap().is_some());
            }
            assert!(reader.read_frame().unwrap().is_none());
        }
        fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn source_open_failure_or_panic_retires_the_slot_before_peer_cleanup() {
    for panic in [false, true] {
        let stop = Arc::new(AtomicBool::new(false));
        let [mut waiting, mut opening] = participants(stop.clone());
        let observe = waiting.shared.clone();
        let owner = thread::spawn(move || waiting.acquisition_finished());
        let lock = observe.finished.lock().unwrap();
        let (lock, timeout) = observe
            .changed
            .wait_timeout_while(lock, Duration::from_secs(5), |mask| *mask & 1 == 0)
            .unwrap();
        let arrived = *lock & 1 != 0;
        drop(lock);
        let flag = stop.clone();
        let failure = thread::spawn(move || {
            let (tx, _rx) = mpsc::sync_channel(1);
            record_controlled::<HeldSource>(
                profile(false),
                std::path::Path::new("unused-open-failure"),
                1,
                (&flag, &Control::default(), Some(&mut opening)),
                tx,
                Side::Right,
                || {
                    if panic {
                        panic!("test source open panic")
                    } else {
                        Err("test source open failure".into())
                    }
                },
            )
        })
        .join();
        owner.join().unwrap();
        assert!(arrived && !timeout.timed_out());
        assert!(stop.load(Ordering::Relaxed));
        if panic {
            assert!(failure.is_err());
        } else {
            assert!(
                failure
                    .unwrap()
                    .err()
                    .unwrap()
                    .contains("test source open failure")
            );
        }
    }
}
