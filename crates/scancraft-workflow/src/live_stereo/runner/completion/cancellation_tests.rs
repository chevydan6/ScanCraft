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

struct PollingSource {
    started: mpsc::Sender<()>,
    resume: mpsc::Receiver<()>,
}
impl Source for PollingSource {
    fn context(&self) -> Result<CaptureContext, String> {
        Ok(CaptureContext::default())
    }
    fn next(&mut self) -> Result<Option<Frame>, String> {
        Err("runner bypassed cooperative source read".into())
    }
    fn next_until(&mut self, cancelled: &dyn Fn() -> bool) -> Result<Option<Frame>, String> {
        assert!(!cancelled());
        self.started.send(()).unwrap();
        self.resume.recv_timeout(Duration::from_secs(5)).unwrap();
        if cancelled() {
            Ok(None)
        } else {
            Err("in-progress source did not receive cancellation".into())
        }
    }
    fn statistics(&self) -> Value {
        Value::Null
    }
    fn close(self) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn in_progress_read_observes_peer_failure_and_public_stop_independently() {
    for graceful in [false, true] {
        let root = crate::output::staging(&std::env::temp_dir().join("live-read-cancel")).unwrap();
        let path = root.join("capture.raptor");
        let stop = Arc::new(AtomicBool::new(false));
        let control = Arc::new(Control::default());
        let flag = stop.clone();
        let commands = control.clone();
        let (started, ready) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        let owner = thread::spawn(move || {
            let (sender, _receiver) = mpsc::sync_channel(1);
            record_controlled(
                StreamProfile {
                    interface: 0,
                    width: 2,
                    height: 2,
                    fourcc: *b"GREY",
                    fps: 30,
                },
                &path,
                1,
                (&flag, &commands, None),
                sender,
                Side::Left,
                || Ok(PollingSource { started, resume }),
            )
        });
        let ready = ready.recv_timeout(Duration::from_secs(5));
        if graceful {
            control.request_stop();
        } else {
            stop.store(true, Ordering::Relaxed);
        }
        let released = release.send(());
        let result = owner.join().unwrap();
        assert!(ready.is_ok() && released.is_ok());
        assert!(result.is_ok(), "{:?}", result.err());
        assert_eq!(stop.load(Ordering::Relaxed), !graceful);
        assert_eq!(control.stop_requested(), graceful);
        let mut reader = Reader::new(fs::File::open(root.join("capture.raptor")).unwrap()).unwrap();
        assert!(reader.read_frame().unwrap().is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
