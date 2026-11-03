use super::*;
use scancraft_core::Frame;
use scancraft_recording::{CaptureContext, Reader};
use std::{
    io,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

fn profile() -> StreamProfile {
    StreamProfile {
        interface: 0,
        width: 2,
        height: 2,
        fourcc: *b"GREY",
        fps: 30,
    }
}
fn frame(index: usize) -> Frame {
    let mut metadata = vec![0; 16];
    metadata[10] = 6;
    metadata[11] = 4;
    metadata[12..16].copy_from_slice(&(index as u32 * 33333).to_le_bytes());
    Frame {
        sequence: index as u32,
        host_timestamp_us: index as u64 * 33333,
        flags: 0,
        metadata,
        data: vec![index as u8; 4],
    }
}
#[derive(Clone, Copy)]
enum Fault {
    None,
    Overflow,
    ReadPanic,
    ReadAndCloseAndWrite,
    Sync,
}
struct SourceGate {
    index: usize,
    armed: Arc<AtomicBool>,
    writing: mpsc::Receiver<()>,
    closed: mpsc::Sender<(usize, bool)>,
    stop: Arc<AtomicBool>,
    fault: Fault,
}
impl Source for SourceGate {
    fn context(&self) -> Result<CaptureContext> {
        Ok(CaptureContext::default())
    }
    fn next(&mut self) -> Result<Option<Frame>> {
        if self.index == 0 {
            self.armed.store(true, Ordering::SeqCst);
        }
        if self.index == 1 {
            self.writing
                .recv_timeout(Duration::from_secs(10))
                .map_err(|e| e.to_string())?;
            match self.fault {
                Fault::ReadPanic => panic!("test source read panic"),
                Fault::ReadAndCloseAndWrite => return Err("test read failure".into()),
                _ => (),
            }
        }
        let value = frame(self.index);
        self.index += 1;
        Ok(Some(value))
    }
    fn statistics(&self) -> Value {
        serde_json::json!({"produced":self.index})
    }
    fn close(self) -> Result<()> {
        self.closed
            .send((self.index, self.stop.load(Ordering::Relaxed)))
            .unwrap();
        if matches!(self.fault, Fault::ReadAndCloseAndWrite) {
            Err("test close failure".into())
        } else {
            Ok(())
        }
    }
}
struct SinkGate {
    armed: Arc<AtomicBool>,
    writing: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    bytes: Arc<Mutex<Vec<u8>>>,
    fault: Fault,
}
impl Write for SinkGate {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.writing.send(()).map_err(io::Error::other)?;
            self.release
                .recv_timeout(Duration::from_secs(20))
                .map_err(io::Error::other)?;
            if matches!(self.fault, Fault::ReadAndCloseAndWrite) {
                return Err(io::Error::other("test write failure"));
            }
        }
        self.bytes.lock().unwrap().extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct Outcome {
    closed: (usize, bool),
    stop: bool,
    result: Result<SourceReport>,
    bytes: Vec<u8>,
}
fn exercise(fault: Fault) -> Outcome {
    let count = if matches!(fault, Fault::Overflow) {
        QUEUE_CAPACITY + 2
    } else {
        4
    };
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let armed = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    let (writing, observing) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let (closed, closing) = mpsc::channel();
    let (finished, completion) = mpsc::channel();
    let source = SourceGate {
        index: 0,
        armed: armed.clone(),
        writing: observing,
        closed,
        stop: stop.clone(),
        fault,
    };
    let sink = SinkGate {
        armed,
        writing,
        release: blocked,
        bytes: bytes.clone(),
        fault,
    };
    let flag = stop.clone();
    let worker = thread::spawn(move || {
        // A full processing queue is optional preview loss, independent of required storage.
        let (sender, _receiver) = mpsc::sync_channel(1);
        let result = record_with_sink(
            profile(),
            count,
            (&flag, &Control::default()),
            sender,
            Side::Left,
            || Ok(source),
            (
                || Ok(sink),
                |_| {
                    if matches!(fault, Fault::Sync) {
                        Err("test sync failure".into())
                    } else {
                        Ok(())
                    }
                },
            ),
        );
        finished.send(result).unwrap();
    });
    let observed_close = closing.recv_timeout(Duration::from_secs(10));
    let premature_completion = completion.try_recv();
    // Always release/join before assertions, even if readiness failed.
    let _ = release.send(());
    worker.join().unwrap();
    assert!(
        matches!(premature_completion, Err(mpsc::TryRecvError::Empty)),
        "recording cannot finish while the sink is blocked"
    );
    let result = completion.recv().unwrap();
    let value = bytes.lock().unwrap().clone();
    Outcome {
        closed: observed_close.expect("source must close before storage drain"),
        stop: stop.load(Ordering::Relaxed),
        result,
        bytes: value,
    }
}
fn assert_recorded(bytes: &[u8], count: usize) {
    let mut direct =
        Writer::with_context(Vec::new(), profile(), CaptureContext::default()).unwrap();
    for i in 0..count {
        direct.push(&frame(i)).unwrap();
    }
    assert_eq!(bytes, direct.finish().unwrap());
    let mut reader = Reader::new(bytes).unwrap();
    for i in 0..count {
        assert_eq!(reader.read_frame().unwrap().unwrap(), frame(i));
    }
    assert!(reader.read_frame().unwrap().is_none());
}
#[test]
fn storage_stall_does_not_block_acquisition_or_source_close_and_bytes_are_unchanged() {
    let outcome = exercise(Fault::None);
    assert_eq!(outcome.closed, (4, false));
    assert!(!outcome.stop);
    let report = outcome.result.unwrap();
    assert_eq!(report.recorded_frames, 4);
    assert_eq!(report.queued_frames, 1);
    assert_eq!(report.processing_queue_drops, 3);
    assert_recorded(&outcome.bytes, 4);
}
#[test]
fn recording_overflow_cancels_before_close_and_retains_only_the_admitted_prefix() {
    let outcome = exercise(Fault::Overflow);
    assert_eq!(outcome.closed, (QUEUE_CAPACITY + 2, true));
    assert!(outcome.stop);
    assert!(
        outcome
            .result
            .err()
            .unwrap()
            .contains("recording queue full")
    );
    assert_recorded(&outcome.bytes, QUEUE_CAPACITY + 1);
}
#[test]
fn source_panic_closes_before_joining_the_stalled_recording_worker() {
    let outcome = exercise(Fault::ReadPanic);
    assert_eq!(outcome.closed, (1, true));
    assert!(outcome.stop);
    assert!(
        outcome
            .result
            .err()
            .unwrap()
            .contains("capture worker panicked")
    );
    assert_recorded(&outcome.bytes, 1);
}
#[test]
fn acquisition_cleanup_and_background_write_errors_are_all_retained() {
    let outcome = exercise(Fault::ReadAndCloseAndWrite);
    assert_eq!(outcome.closed, (1, true));
    assert!(outcome.stop);
    let error = outcome.result.err().unwrap();
    for message in [
        "test read failure",
        "test close failure",
        "test write failure",
    ] {
        assert!(error.contains(message), "{error}");
    }
}
#[test]
fn durability_failure_cannot_report_success_after_source_cleanup() {
    let outcome = exercise(Fault::Sync);
    assert_eq!(outcome.closed, (4, false));
    assert!(outcome.stop);
    assert!(outcome.result.err().unwrap().contains("test sync failure"));
    assert_recorded(&outcome.bytes, 4);
}
