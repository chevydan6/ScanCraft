use super::*;
use crate::live_stereo::runner::completion;
use std::sync::mpsc;

struct Held {
    first: bool,
    mode: usize,
    frame: Frame,
    reading: Arc<AtomicBool>,
    ready: mpsc::Sender<()>,
    wait: mpsc::Receiver<()>,
    closed: mpsc::Sender<bool>,
    signals: Arc<Signals>,
}
impl Source for Held {
    fn context(&self) -> Result<CaptureContext> {
        Ok(CaptureContext::default())
    }
    fn next(&mut self) -> Result<Option<Frame>> {
        if !self.first {
            self.reading.store(true, Ordering::SeqCst);
            self.ready.send(()).unwrap();
        }
        self.wait
            .recv_timeout(Duration::from_secs(10))
            .map_err(|e| e.to_string())?;
        if !self.first {
            self.reading.store(false, Ordering::SeqCst);
            return Ok(None);
        }
        match self.mode {
            1 => Err("test read failure".into()),
            2 => panic!("test read panic"),
            3 => {
                self.signals.requested.store(true, Ordering::Relaxed);
                Ok(None)
            }
            4 | 6 => Ok(Some(self.frame.clone())),
            5 => {
                self.signals.work.cancel();
                Ok(None)
            }
            _ => Ok(None),
        }
    }
    fn statistics(&self) -> Value {
        if self.first && self.mode == 8 {
            panic!("test statistics panic");
        }
        Value::Null
    }
    fn close(self) -> Result<()> {
        let reading = self.reading.load(Ordering::SeqCst);
        self.closed.send(self.first).unwrap();
        if reading {
            Err("closed while peer still acquiring".into())
        } else {
            Ok(())
        }
    }
}
struct Memory {
    mode: usize,
}
impl Journal for Memory {
    fn push(&mut self, _: &Frame) -> Result<()> {
        if self.mode == 6 {
            Err("test journal push failure".into())
        } else {
            Ok(())
        }
    }
    fn finish(self) -> Result<()> {
        if self.mode == 7 {
            Err("test journal finish failure".into())
        } else {
            Ok(())
        }
    }
}

#[test]
fn laser_worker_keeps_source_open_until_peer_read_returns_on_all_outcomes() {
    for mode in 0..9 {
        let signals = Arc::new(Signals {
            stop: Arc::new(AtomicBool::new(false)),
            requested: Arc::new(AtomicBool::new(false)),
            work: Arc::new(WorkControl::default()),
            progress: Progress::default(),
        });
        let [mut first, mut second] = completion::participants(signals.stop.clone());
        let observe = first.arrival_observer();
        let reading = Arc::new(AtomicBool::new(false));
        let (ready, started) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let (closed, closes) = mpsc::channel();
        let (_, _, profiles, frames) = super::super::tests::fixture();
        let left = Held {
            first: true,
            mode,
            frame: frames[0].clone(),
            reading: reading.clone(),
            ready: ready.clone(),
            wait: started,
            closed: closed.clone(),
            signals: signals.clone(),
        };
        let right = Held {
            first: false,
            mode,
            frame: frames[1].clone(),
            reading,
            ready,
            wait,
            closed,
            signals: signals.clone(),
        };
        let profile = profiles[0].clone();
        let flags = signals.clone();
        let left = thread::spawn(move || {
            let (sender, _receiver) = mpsc::sync_channel(1);
            let report = produce(
                Camera::First,
                profile,
                1,
                (&flags, &mut first),
                sender,
                || Ok(left),
                |_| Ok(Memory { mode }),
            );
            drop(first);
            report
        });
        let profile = profiles[1].clone();
        let flags = signals.clone();
        let right = thread::spawn(move || {
            let (sender, _receiver) = mpsc::sync_channel(1);
            let report = produce(
                Camera::Second,
                profile,
                1,
                (&flags, &mut second),
                sender,
                || Ok(right),
                |_| Ok(Memory { mode: 0 }),
            );
            drop(second);
            report
        });
        let arrived = observe();
        let early_close = closes.try_recv();
        let stopped = signals.stop.load(Ordering::Relaxed);
        let _ = release.send(());
        let left = left.join().unwrap();
        let right = right.join().unwrap();
        assert!(arrived, "mode {mode}");
        assert!(
            matches!(early_close, Err(mpsc::TryRecvError::Empty)),
            "premature close in mode {mode}"
        );
        assert!(left.cleanup_confirmed && right.cleanup_confirmed);
        assert!(right.outcome.is_ok());
        assert_eq!(left.outcome.is_err(), matches!(mode, 1 | 2 | 6 | 7 | 8));
        assert_eq!(stopped, matches!(mode, 1 | 2 | 6 | 7 | 8));
        assert_eq!(closes.into_iter().count(), 2);
    }
}
