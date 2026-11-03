use super::*;
use crate::{Paced, Temp, context, fixture, frame, open, settings};
use scancraft_recording::{Reader, queued::QueuedWriter};
use scancraft_workflow::live_stereo::{Side, control::Control, runner::Source};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

struct HeldSink {
    out: BufWriter<fs::File>,
    armed: Arc<std::sync::atomic::AtomicBool>,
    entered: mpsc::Sender<()>,
    release: Option<mpsc::Receiver<()>>,
}
impl Write for HeldSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.armed.load(Ordering::Acquire)
            && let Some(release) = self.release.take()
        {
            self.entered.send(()).map_err(io::Error::other)?;
            release
                .recv_timeout(Duration::from_secs(10))
                .map_err(io::Error::other)?;
        }
        self.out.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}
struct ReleaseOnDrop(Vec<mpsc::Sender<()>>);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        for release in &self.0 {
            let _ = release.send(());
        }
    }
}
fn until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "controlled fixture progress timed out"
        );
        thread::yield_now();
    }
}
fn pixels() -> [Vec<u8>; 2] {
    ["left.png", "right.png"].map(|n| {
        image::open(fixture().join(n))
            .unwrap()
            .to_luma8()
            .into_raw()
    })
}

#[test]
fn both_delayed_recording_consumers_bound_the_synthetic_source() {
    let temp = Temp::new();
    let processor = open(&temp.0, &settings(32)).unwrap();
    let target = temp.0.join("held");
    let partial = scancraft_workflow::output::staging(&target).unwrap();
    let pixels = pixels();
    let prototypes = [
        frame(Side::Left, 0, &pixels[0]),
        frame(Side::Right, 0, &pixels[1]),
    ];
    let progress = Arc::new(RecordingProgress::for_target(
        &target,
        processor.profiles().clone(),
        context(),
        &prototypes,
    ));
    let closed = Arc::new(AtomicUsize::new(0));
    let emitted: [Arc<AtomicUsize>; 2] = std::array::from_fn(|_| Arc::new(AtomicUsize::new(0)));
    thread::scope(|scope| {
        // Declared inside the scope: assertion unwinding releases gates before
        // scope joins. Each held write also has its own finite failure deadline.
        let mut releases = ReleaseOnDrop(Vec::new());
        let mut entries = Vec::new();
        let mut workers = Vec::new();
        for (index, side) in [Side::Left, Side::Right].into_iter().enumerate() {
            let (entered, entry) = mpsc::channel();
            let (release, released) = mpsc::channel();
            releases.0.push(release);
            entries.push(entry);
            let armed = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let sink = HeldSink {
                out: BufWriter::new(
                    fs::File::create(partial.join(if index == 0 {
                        "left.raptor"
                    } else {
                        "right.raptor"
                    }))
                    .unwrap(),
                ),
                armed: armed.clone(),
                entered,
                release: Some(released),
            };
            let direct =
                Writer::with_context(sink, processor.profiles()[index].clone(), context()).unwrap();
            let mut writer = QueuedWriter::new(direct, 8).unwrap();
            armed.store(true, Ordering::Release);
            let mut source = Paced {
                side,
                next: 0,
                maximum: 32,
                pixels: pixels[index].clone(),
                // Processing is deliberately ready immediately, isolating the
                // recording gate. The 1,001-pair test exercises real previews.
                acknowledged: Arc::new(AtomicUsize::new(usize::MAX)),
                recording: progress.clone(),
                closed: closed.clone(),
                control: Arc::new(Control::new(false)),
            };
            let count = emitted[index].clone();
            workers.push(scope.spawn(move || {
                while let Some(frame) = source.next().unwrap() {
                    writer.push(frame).unwrap();
                    count.fetch_add(1, Ordering::Release);
                }
                source.close().unwrap();
                writer.finish().unwrap();
            }));
        }
        for entry in entries {
            entry.recv_timeout(Duration::from_secs(10)).unwrap();
        }
        until(|| {
            progress
                .blocked
                .iter()
                .all(|x| x.load(Ordering::Acquire) == progress.window)
        });
        for count in &emitted {
            assert_eq!(count.load(Ordering::Acquire), progress.window);
        }
        // Releasing only left must not admit more frames on either source.
        releases.0[0].send(()).unwrap();
        until(|| {
            fs::metadata(partial.join("left.raptor")).unwrap().len()
                >= progress.layouts[0].header + progress.layouts[0].frame
        });
        assert!(!progress.ready(progress.window, 0).unwrap());
        assert!(!progress.ready(progress.window, 1).unwrap());
        for count in &emitted {
            assert_eq!(count.load(Ordering::Acquire), progress.window);
        }
        releases.0[1].send(()).unwrap();
        for worker in workers {
            worker.join().unwrap();
        }
    });
    assert_eq!(closed.load(Ordering::Acquire), 2);
    for (side, name) in ["left.raptor", "right.raptor"].into_iter().enumerate() {
        assert_eq!(emitted[side].load(Ordering::Acquire), 32);
        let mut reader = Reader::new(std::io::BufReader::new(
            fs::File::open(partial.join(name)).unwrap(),
        ))
        .unwrap();
        for index in 0..32 {
            assert_eq!(reader.read_frame().unwrap().unwrap().sequence, index);
        }
        assert!(reader.read_frame().unwrap().is_none());
    }
}

#[test]
fn recording_wait_preserves_cancellation_and_deadline() {
    let temp = Temp::new();
    let processor = open(&temp.0, &settings(32)).unwrap();
    let pixels = pixels();
    let prototypes = [
        frame(Side::Left, 0, &pixels[0]),
        frame(Side::Right, 0, &pixels[1]),
    ];
    let progress = Arc::new(RecordingProgress::for_target(
        &temp.0.join("absent"),
        processor.profiles().clone(),
        context(),
        &prototypes,
    ));
    let source = Paced {
        side: Side::Left,
        next: progress.window,
        maximum: 32,
        pixels: pixels[0].clone(),
        acknowledged: Arc::new(AtomicUsize::new(usize::MAX)),
        recording: progress,
        closed: Arc::new(AtomicUsize::new(0)),
        control: Arc::new(Control::new(false)),
    };
    assert!(
        !source
            .wait_for_progress(&|| true, Instant::now() + Duration::from_secs(10))
            .unwrap()
    );
    assert!(
        source
            .wait_for_progress(&|| false, Instant::now())
            .unwrap_err()
            .contains("synthetic pacing timed out")
    );
}
