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
    worker: Worker,
}
impl Discovery {
    pub(crate) fn start() -> Result<Self, String> {
        let cancel = Arc::new(AtomicBool::new(false));
        let control = cancel.clone();
        let worker = Worker::spawn(move || run(&control)).map_err(|e| e.to_string())?;
        Ok(Self { cancel, worker })
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
        self.cancel();
        // This joins IPC supervision only. A stalled probe retains its descriptors
        // in the helper; no kill/reset or successful-cleanup claim is made.
        let _ = self.worker.join();
    }
}
fn run(cancel: &AtomicBool) -> Result<String, String> {
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
        cancel,
        |_| {
            unexpected_frame = true;
            false
        },
        Duration::from_secs(2),
    );
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
            run(&AtomicBool::new(true)),
            Ok("Discovery cancelled before launch".into())
        );
    }
}
