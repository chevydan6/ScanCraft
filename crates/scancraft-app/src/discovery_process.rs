//! Read-only device probing stays in a child, including potentially blocking QUERYCAP.
use crate::{
    capture_ipc,
    capture_process::{self, OwnedChild},
    capture_worker::Worker,
};
use std::{
    ffi::OsString,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub(crate) struct Discovery {
    cancel: Arc<AtomicBool>,
    abort: Arc<AtomicBool>,
    worker: Worker,
}
impl Discovery {
    pub(crate) fn start() -> Result<Self, String> {
        let cancel = Arc::new(AtomicBool::new(false));
        let control = cancel.clone();
        let abort = Arc::new(AtomicBool::new(false));
        let teardown = abort.clone();
        let worker = Worker::spawn(move || run(&control, &teardown)).map_err(|e| e.to_string())?;
        Ok(Self {
            cancel,
            abort,
            worker,
        })
    }
    pub(crate) fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
    pub(crate) fn poll(&mut self) -> Option<Result<String, String>> {
        self.worker.poll()
    }
}
impl Drop for Discovery {
    fn drop(&mut self) {
        self.abort.store(true, Ordering::Relaxed);
        self.cancel();
        // This joins IPC supervision only. A stalled probe retains its descriptors
        // in the helper; no kill/reset or successful-cleanup claim is made.
        let _ = self.worker.join();
    }
}
fn run(cancel: &AtomicBool, abort: &AtomicBool) -> Result<String, String> {
    if cancel.load(Ordering::Relaxed) {
        return Ok("Discovery cancelled before launch".into());
    }
    let (path, log) = capture_process::private_log().map_err(|e| e.to_string())?;
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command
        .arg("--discovery-helper")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log));
    let mut helper = OwnedChild::spawn(&mut command).map_err(|e| e.to_string())?;
    let pid = helper.child().id();
    let mut unexpected_frame = false;
    let result = capture_process::supervise(
        &mut helper,
        cancel,
        abort,
        |_| {
            unexpected_frame = true;
            false
        },
        Duration::from_secs(2),
    );
    retain_until_exit(&mut helper, abort);
    if unexpected_frame {
        return Err("Unexpected frame in discovery protocol".into());
    }
    result.map_err(|e| {
        format!(
            "Discovery: {e}; helper PID {pid}; diagnostics: {}",
            path.display()
        )
    })
}
// Failed IPC must not release the GUI's pending guard while a probe is alive.
// Only application teardown may abandon waiting; ordinary Stop cannot do so.
fn retain_until_exit(helper: &mut OwnedChild, abort: &AtomicBool) {
    helper.child().stdin.take();
    while !abort.load(Ordering::Relaxed) {
        if matches!(helper.child().try_wait(), Ok(Some(_))) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
pub(crate) fn helper_entry(args: &[OsString]) -> Option<i32> {
    if args.first().is_none_or(|a| a != "--discovery-helper") {
        return None;
    }
    if args.len() != 1 {
        eprintln!("discovery helper accepts no arguments");
        return Some(1);
    }
    eprintln!("discovery helper PID {} starting", std::process::id());
    let result = scancraft_device::discover()
        .map(|nodes| {
            format!(
                "{} native video/metadata nodes\n{}",
                nodes.len(),
                nodes
                    .iter()
                    .map(|n| format!(
                        "{} · interface {} · {}",
                        n.path.display(),
                        n.interface,
                        if n.metadata { "metadata" } else { "image" }
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
        .map_err(|e| e.to_string());
    eprintln!("discovery result: {result:?}");
    let success = result.is_ok();
    let written = capture_ipc::write_complete(&mut std::io::stdout().lock(), &result);
    Some(if success && written.is_ok() { 0 } else { 1 })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn polling_and_cancelling_do_not_wait_for_probe_completion() {
        let (release, wait) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker = Worker::spawn(move || {
            wait.recv().unwrap();
            Ok("probe finished".into())
        })
        .unwrap();
        let mut job = Discovery {
            cancel: cancel.clone(),
            abort: Arc::new(AtomicBool::new(false)),
            worker,
        };
        // Release before unwinding/drop on assertion failure as well.
        struct Release(Option<std::sync::mpsc::Sender<()>>);
        impl Drop for Release {
            fn drop(&mut self) {
                if let Some(tx) = self.0.take() {
                    let _ = tx.send(());
                }
            }
        }
        let mut release = Release(Some(release));
        assert!(job.poll().is_none());
        job.cancel();
        assert!(cancel.load(Ordering::Relaxed));
        assert!(job.poll().is_none());
        release.0.take().unwrap().send(()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let result = loop {
            if let Some(result) = job.poll() {
                break result;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        };
        assert_eq!(result, Ok("probe finished".into()));
    }
    #[test]
    fn failed_probe_keeps_pending_guard_until_actual_child_exit() {
        let mut command = Command::new("sh");
        command
            .args(["-c", "exec sleep 0.1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut helper = OwnedChild::spawn(&mut command).unwrap();
        let start = std::time::Instant::now();
        retain_until_exit(&mut helper, &AtomicBool::new(false));
        assert!(helper.child().try_wait().unwrap().is_some());
        assert!(start.elapsed() >= Duration::from_millis(100));
    }
    #[test]
    fn teardown_may_stop_waiting_without_claiming_child_exit() {
        let mut command = Command::new("sh");
        command
            .args(["-c", "exec sleep 1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut helper = OwnedChild::spawn(&mut command).unwrap();
        retain_until_exit(&mut helper, &AtomicBool::new(true));
        assert!(helper.child().try_wait().unwrap().is_none());
    }
    #[test]
    fn invalid_request_never_probes_devices() {
        assert_eq!(
            helper_entry(&["--discovery-helper".into(), "extra".into()]),
            Some(1)
        );
        assert_eq!(helper_entry(&[]), None);
    }
    #[test]
    fn cancelled_discovery_does_not_spawn() {
        assert_eq!(
            run(&AtomicBool::new(true), &AtomicBool::new(false)),
            Ok("Discovery cancelled before launch".into())
        );
    }
}
