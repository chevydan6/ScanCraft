use super::*;
use crate::Reader;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;

fn profile() -> StreamProfile {
    StreamProfile {
        interface: 0,
        width: 2,
        height: 2,
        fourcc: *b"GREY",
        fps: 30,
    }
}
fn frame(sequence: u32) -> Frame {
    let mut metadata = vec![0; 22];
    metadata[10] = 12;
    metadata[11] = 0x0c;
    metadata[12..16].copy_from_slice(&(sequence * 10_000).to_le_bytes());
    Frame {
        sequence,
        host_timestamp_us: u64::from(sequence) * 10_000,
        flags: 0,
        metadata,
        data: vec![1, 2, 3, 4],
    }
}
fn read_frames(bytes: &[u8]) -> Vec<Frame> {
    let mut reader = Reader::new(bytes).unwrap();
    let mut frames = Vec::new();
    while let Some(frame) = reader.read_frame().unwrap() {
        frames.push(frame);
    }
    frames
}

#[test]
fn queued_recording_is_byte_identical_to_existing_writer() {
    let mut direct = Writer::new(Vec::new(), profile()).unwrap();
    let mut queued = QueuedWriter::new(Writer::new(Vec::new(), profile()).unwrap(), 8).unwrap();
    let frames: Vec<_> = (1..=8).map(frame).collect();
    for frame in &frames {
        direct.push(frame).unwrap();
        queued.push(frame.clone()).unwrap();
    }
    let bytes = queued.finish().unwrap();
    assert_eq!(bytes, direct.finish().unwrap());
    assert_eq!(read_frames(&bytes), frames);
}

struct GateSink {
    bytes: Arc<Mutex<Vec<u8>>>,
    armed: Arc<AtomicBool>,
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}
impl Write for GateSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.entered.send(()).map_err(io::Error::other)?;
            self.release
                .recv_timeout(Duration::from_secs(5))
                .map_err(io::Error::other)?;
        }
        self.bytes.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
type Gated = (
    QueuedWriter<GateSink>,
    mpsc::Receiver<()>,
    mpsc::Sender<()>,
    Arc<Mutex<Vec<u8>>>,
);
fn gated(capacity: usize) -> Gated {
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let armed = Arc::new(AtomicBool::new(false));
    let (entered, ready) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let writer = Writer::new(
        GateSink {
            bytes: bytes.clone(),
            armed: armed.clone(),
            entered,
            release: released,
        },
        profile(),
    )
    .unwrap();
    armed.store(true, Ordering::SeqCst);
    (
        QueuedWriter::new(writer, capacity).unwrap(),
        ready,
        release,
        bytes,
    )
}

#[test]
fn stalled_storage_has_bounded_admission_and_overflow_cannot_finish_successfully() {
    let (mut queued, ready, release, bytes) = gated(2);
    queued.push(frame(1)).unwrap();
    let started = ready.recv_timeout(Duration::from_secs(2));
    let second = queued.push(frame(2));
    let third = queued.push(frame(3));
    let overflow = queued.push(frame(4));
    let repeated = queued.push(frame(5));
    // Always unblock the worker before evaluating assertions or joining it.
    let _ = release.send(());
    let finished = queued.finish();
    assert!(started.is_ok());
    assert!(second.is_ok() && third.is_ok());
    assert_eq!(overflow.unwrap_err().kind(), io::ErrorKind::WouldBlock);
    assert_eq!(repeated.unwrap_err().kind(), io::ErrorKind::WouldBlock);
    assert_eq!(finished.err().unwrap().kind(), io::ErrorKind::WouldBlock);
    assert_eq!(
        read_frames(&bytes.lock().unwrap()),
        (1..=3).map(frame).collect::<Vec<_>>()
    );
}

#[test]
fn drop_joins_a_stalled_worker_and_preserves_the_admitted_prefix() {
    let (mut queued, ready, release, bytes) = gated(2);
    queued.push(frame(1)).unwrap();
    let started = ready.recv_timeout(Duration::from_secs(2));
    let admitted = queued.push(frame(2));
    let (dropping, observed) = mpsc::channel();
    let (done, finished) = mpsc::channel();
    let owner = thread::spawn(move || {
        dropping.send(()).unwrap();
        drop(queued);
        done.send(()).unwrap();
    });
    let drop_started = observed.recv_timeout(Duration::from_secs(2));
    let early = finished.recv_timeout(Duration::from_millis(100));
    let _ = release.send(());
    owner.join().unwrap();
    assert!(started.is_ok() && admitted.is_ok() && drop_started.is_ok());
    assert!(matches!(early, Err(mpsc::RecvTimeoutError::Timeout)));
    assert!(finished.recv().is_ok());
    assert_eq!(
        read_frames(&bytes.lock().unwrap()),
        vec![frame(1), frame(2)]
    );
}

struct FaultSink(Arc<AtomicUsize>);
impl Write for FaultSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.0.load(Ordering::SeqCst) {
            1 => Err(io::Error::new(io::ErrorKind::StorageFull, "test disk full")),
            2 => panic!("test writer panic"),
            _ => Ok(bytes.len()),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.0.load(Ordering::SeqCst) == 3 {
            Err(io::Error::other("test flush failure"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn writer_io_flush_and_panic_failures_cannot_report_success() {
    for mode in 1..=3 {
        let fault = Arc::new(AtomicUsize::new(0));
        let writer = Writer::new(FaultSink(fault.clone()), profile()).unwrap();
        let mut queued = QueuedWriter::new(writer, 2).unwrap();
        fault.store(mode, Ordering::SeqCst);
        queued.push(frame(1)).unwrap();
        let error = queued.finish().err().unwrap();
        match mode {
            1 => {
                assert_eq!(error.kind(), io::ErrorKind::StorageFull);
                assert!(error.to_string().contains("test disk full"));
            }
            2 => assert!(error.to_string().contains("worker panicked")),
            _ => assert!(error.to_string().contains("test flush failure")),
        }
    }
}

struct FailingDropSink {
    armed: Arc<AtomicBool>,
    dropping: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}
impl Write for FailingDropSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.armed.load(Ordering::SeqCst) {
            Err(io::Error::other(
                "test write failure before slow destructor",
            ))
        } else {
            Ok(bytes.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for FailingDropSink {
    fn drop(&mut self) {
        if self.armed.load(Ordering::SeqCst) {
            let _ = self.dropping.send(());
            let _ = self.release.recv_timeout(Duration::from_secs(5));
        }
    }
}

#[test]
fn disconnected_admission_does_not_join_a_sink_still_cleaning_up() {
    let armed = Arc::new(AtomicBool::new(false));
    let (dropping, ready) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let writer = Writer::new(
        FailingDropSink {
            armed: armed.clone(),
            dropping,
            release: released,
        },
        profile(),
    )
    .unwrap();
    let mut queued = QueuedWriter::new(writer, 2).unwrap();
    armed.store(true, Ordering::SeqCst);
    queued.push(frame(1)).unwrap();
    let started = ready.recv_timeout(Duration::from_secs(2));
    let admission = queued.push(frame(2));
    let _ = release.send(());
    let completion = queued.finish();
    assert!(started.is_ok());
    assert_eq!(admission.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    assert!(
        completion
            .err()
            .unwrap()
            .to_string()
            .contains("test write failure before slow destructor")
    );
}

#[test]
fn invalid_frame_admission_poison_and_worker_ordering_validation_are_preserved() {
    for bad in [
        Frame {
            // Valid repeated UVC records, rejected specifically by the byte bound.
            metadata: frame(1).metadata.repeat(187),
            ..frame(1)
        },
        Frame {
            flags: 0x40,
            ..frame(1)
        },
    ] {
        let mut queued = QueuedWriter::new(Writer::new(Vec::new(), profile()).unwrap(), 2).unwrap();
        assert!(queued.push(bad).is_err());
        assert!(queued.push(frame(1)).is_err());
        assert!(queued.finish().is_err());
    }
    let mut queued = QueuedWriter::new(Writer::new(Vec::new(), profile()).unwrap(), 2).unwrap();
    queued.push(frame(1)).unwrap();
    queued.push(frame(1)).unwrap();
    assert!(queued.finish().is_err());
    for capacity in [0, 33, usize::MAX] {
        assert!(QueuedWriter::new(Writer::new(Vec::new(), profile()).unwrap(), capacity).is_err());
    }
}

#[test]
fn borrowed_admission_preserves_input_and_poisoning_matches_owned_admission() {
    let input = frame(1);
    let before = input.clone();
    let mut copied = QueuedWriter::new(Writer::new(Vec::new(), profile()).unwrap(), 2).unwrap();
    let mut owned = QueuedWriter::new(Writer::new(Vec::new(), profile()).unwrap(), 2).unwrap();
    copied.push_copy(&input).unwrap();
    owned.push(input.clone()).unwrap();
    assert_eq!(input, before);
    assert_eq!(copied.finish().unwrap(), owned.finish().unwrap());

    for invalid in [
        Frame {
            metadata: vec![0; 4097],
            ..input.clone()
        },
        Frame {
            data: vec![0; 5],
            ..input.clone()
        },
    ] {
        let mut copied = QueuedWriter::new(Writer::new(Vec::new(), profile()).unwrap(), 2).unwrap();
        let mut owned = QueuedWriter::new(Writer::new(Vec::new(), profile()).unwrap(), 2).unwrap();
        let borrowed_error = copied.push_copy(&invalid).unwrap_err();
        let owned_error = owned.push(invalid.clone()).unwrap_err();
        assert_eq!(borrowed_error.kind(), owned_error.kind());
        assert_eq!(borrowed_error.to_string(), owned_error.to_string());
        assert_eq!(
            copied.push_copy(&input).unwrap_err().to_string(),
            borrowed_error.to_string()
        );
        assert_eq!(
            copied.finish().unwrap_err().to_string(),
            borrowed_error.to_string()
        );
    }
}
