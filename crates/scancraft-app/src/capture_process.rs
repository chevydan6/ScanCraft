//! Preview helper supervision. Only the child owns scanner state/restoration.
use crate::capture_ipc::{self, Decoder, Message};
use scancraft_core::Frame;
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const GRACE: Duration = Duration::from_secs(2);

// Allocate the waiter before launching a helper. On early return it can reap a
// surviving child without any GUI join or scanner restoration duties of its own.
pub(crate) struct OwnedChild {
    child: Option<Child>,
    reap: mpsc::Sender<Child>,
}
impl OwnedChild {
    pub(crate) fn spawn(command: &mut Command) -> io::Result<Self> {
        let (reap, receiver) = mpsc::channel::<Child>();
        std::thread::Builder::new()
            .name("scancraft-helper-reaper".into())
            .spawn(move || {
                if let Ok(mut child) = receiver.recv() {
                    let _ = child.wait();
                }
            })?;
        Ok(Self {
            child: Some(command.spawn()?),
            reap,
        })
    }
    pub(crate) fn child(&mut self) -> &mut Child {
        self.child.as_mut().unwrap()
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // EOF requests graceful stop; never kill a device-owning helper.
            child.stdin.take();
            let _ = self.reap.send(child);
        }
    }
}
pub(crate) fn private_log() -> io::Result<(PathBuf, File)> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "scancraft-capture-{}-{stamp}-{}.log",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(&path)?;
    Ok((path, file))
}
/// Run from a GUI worker, which owns only IPC and process supervision. On a
/// cancellation timeout the helper retains locks and cleanup responsibilities;
/// its private stderr log remains available after the GUI stops waiting.
pub(crate) fn run(
    interface: u8,
    record: bool,
    folder: &Path,
    cancel: &AtomicBool,
    abort: &AtomicBool,
    deliver: impl FnMut(Frame) -> bool,
) -> Result<String, String> {
    if cancel.load(Ordering::Relaxed) {
        return Ok("Cancelled before launching capture helper".into());
    }
    let (log_path, log) = private_log().map_err(|e| e.to_string())?;
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command
        .arg("--capture-helper")
        .arg(interface.to_string())
        .arg(if record { "record" } else { "preview" })
        .arg(folder)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log));
    let mut helper = OwnedChild::spawn(&mut command).map_err(|e| {
        format!(
            "Could not start capture helper: {e}; diagnostics: {}",
            log_path.display()
        )
    })?;
    let pid = helper.child().id();
    supervise(&mut helper, cancel, abort, deliver, GRACE).map_err(|e| {
        format!(
            "{e}; helper PID {pid}; cleanup diagnostics: {}",
            log_path.display()
        )
    })
}
pub(crate) fn nonblocking(file: &impl AsRawFd) -> io::Result<()> {
    // SAFETY: F_GETFL/F_SETFL operate on a borrowed, live pipe descriptor. Only
    // this monitor uses it; no descriptor ownership is transferred.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
pub(crate) fn supervise(
    helper: &mut OwnedChild,
    cancel: &AtomicBool,
    abort: &AtomicBool,
    mut deliver: impl FnMut(Frame) -> bool,
    grace: Duration,
) -> Result<String, String> {
    let mut output = helper
        .child()
        .stdout
        .take()
        .ok_or("helper stdout missing")?;
    nonblocking(&output).map_err(|e| e.to_string())?;
    let mut decoder = Decoder::default();
    let mut completion = None;
    let mut stopping = None;
    let mut consumer_closed = false;
    let mut eof = false;
    let mut bytes = [0; 65536];
    loop {
        if cancel.load(Ordering::Relaxed) || abort.load(Ordering::Relaxed) || consumer_closed {
            helper.child().stdin.take();
        }
        // Ordinary Stop permits arbitrarily slow recording finalization. Only
        // explicit abort or a lost consumer starts the supervision grace.
        if stopping.is_none() && (abort.load(Ordering::Relaxed) || consumer_closed) {
            stopping = Some(Instant::now());
        }
        // Limit each batch so continuous input cannot starve cancellation.
        for _ in 0..64 {
            match output.read(&mut bytes) {
                Ok(0) => {
                    eof = true;
                    break;
                }
                Ok(n) => decoder.push(&bytes[..n]).map_err(|e| e.to_string())?,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(format!("Capture IPC read failed: {e}")),
            }
            while let Some(message) = decoder.next().map_err(|e| e.to_string())? {
                match message {
                    Message::Frame(frame) => {
                        if !consumer_closed {
                            consumer_closed = !deliver(frame);
                        }
                    }
                    Message::Complete(result) => completion = Some(result),
                    Message::LiveProgress(_) | Message::LiveComplete(_) => {
                        return Err("Live-scan packet in preview helper stream".into());
                    }
                }
            }
        }
        if let Some(status) = helper.child().try_wait().map_err(|e| e.to_string())?
            && eof
        {
            decoder.finish().map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!(
                    "Capture helper exited {status}; result: {completion:?}"
                ));
            }
            return completion.ok_or("Capture helper omitted its result")?;
        }
        if stopping.is_some_and(|start| start.elapsed() >= grace) {
            return Err("Capture helper is still running; device cleanup/restoration is NOT confirmed. Any acquired device handles and locks remain owned by that helper until cleanup finishes".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
/// Internal entry point runs before eframe initialization. Invalid requests never
/// open a scanner. Stdin EOF or any control byte requests graceful cancellation.
pub(crate) fn helper_entry(args: &[OsString]) -> Option<i32> {
    if args.first().is_none_or(|arg| arg != "--capture-helper") {
        return None;
    }
    let result = (|| -> Result<(), String> {
        if args.len() != 4 {
            return Err("capture helper requires INTERFACE record|preview FOLDER".into());
        }
        let interface: u8 = args[1]
            .to_str()
            .ok_or("invalid interface")?
            .parse()
            .map_err(|_| "invalid interface")?;
        if ![0, 2, 4].contains(&interface) {
            return Err("unsupported capture interface".into());
        }
        let record = match args[2].to_str() {
            Some("record") => true,
            Some("preview") => false,
            _ => return Err("invalid capture mode".into()),
        };
        eprintln!(
            "{}",
            serde_json::json!({"helper_pid":std::process::id(),"status":"starting","kind":"camera_preview_or_recording"})
        );
        let cancel = Arc::new(AtomicBool::new(false));
        let c = cancel.clone();
        std::thread::Builder::new()
            .name("scancraft-helper-control".into())
            .spawn(move || {
                let mut byte = [0];
                loop {
                    match io::stdin().read(&mut byte) {
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        _ => {
                            c.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        let mut output = io::stdout().lock();
        let mut transport_error = None;
        let outcome = crate::capture_session::run(
            interface,
            record,
            Path::new(&args[3]),
            || cancel.load(Ordering::Relaxed),
            |frame| match capture_ipc::write_frame(&mut output, &frame) {
                Ok(()) => true,
                Err(error) => {
                    transport_error = Some(error.to_string());
                    false
                }
            },
        );
        // stderr is a private file supplied by the supervisor, independent of
        // stdout transport. Keep the final cleanup outcome even after IPC loss.
        eprintln!(
            "{}",
            serde_json::json!({"capture_result": outcome, "ipc_error": transport_error})
        );
        let outcome = if let Some(error) = transport_error {
            Err(format!(
                "Capture display transport failed: {error}; capture outcome: {outcome:?}"
            ))
        } else {
            outcome
        };
        capture_ipc::write_complete(&mut output, &outcome).map_err(|e| e.to_string())?;
        output.flush().map_err(|e| e.to_string())?;
        outcome.map(|_| ())
    })();
    if let Err(error) = &result {
        eprintln!("Capture helper failed: {error}");
    }
    Some(if result.is_ok() { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pre_cancelled_request_does_not_launch_a_helper() {
        assert_eq!(
            run(
                255,
                true,
                Path::new("/unused"),
                &AtomicBool::new(true),
                &AtomicBool::new(true),
                |_| panic!("cancelled request delivered a frame")
            ),
            Ok("Cancelled before launching capture helper".into())
        );
    }

    #[test]
    fn invalid_helper_request_is_rejected_before_device_access() {
        assert_eq!(
            helper_entry(&[
                "--capture-helper".into(),
                "255".into(),
                "preview".into(),
                "/unused".into()
            ]),
            Some(1)
        );
        assert_eq!(helper_entry(&[]), None);
    }
    #[test]
    fn cancelled_stalled_process_does_not_block_supervisor_until_child_exit() {
        let mut command = Command::new("sh");
        command
            .args(["-c", "exec sleep 1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut helper = OwnedChild::spawn(&mut command).unwrap();
        let start = Instant::now();
        let error = supervise(
            &mut helper,
            &AtomicBool::new(true),
            &AtomicBool::new(true),
            |_| true,
            Duration::from_millis(20),
        )
        .unwrap_err();
        assert!(error.contains("NOT confirmed"));
        assert!(helper.child().try_wait().unwrap().is_none());
        assert!(start.elapsed() < Duration::from_millis(800));
        // Existing reaper receives the live child. It owns waiting only, never
        // device restoration; no kill, reset or abandoned device-owning thread.
        drop(helper);
    }
    #[test]
    fn timed_out_worker_retains_child_lock_until_cleanup_actually_finishes() {
        use std::os::unix::fs::OpenOptionsExt;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "scancraft-helper-lock-test-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir(&dir).unwrap();
        struct Release(PathBuf);
        impl Drop for Release {
            fn drop(&mut self) {
                let _ = std::fs::write(self.0.join("release"), []);
            }
        }
        let release = Release(dir.clone());
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(dir.join("lock"))
            .unwrap();
        // SAFETY: flock acts only on a live, privately owned test file descriptor.
        assert_eq!(
            unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        let probe = OpenOptions::new()
            .read(true)
            .write(true)
            .open(dir.join("lock"))
            .unwrap();
        let mut command = Command::new("sh");
        command
            .args(["-c", "while [ ! -e \"$1\" ]; do sleep 0.01; done", "sh"])
            .arg(dir.join("release"))
            .stdin(Stdio::from(lock))
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut helper = OwnedChild::spawn(&mut command).unwrap();
        // Command owns its configured Stdio too; drop the parent's duplicate.
        drop(command);
        let cancel = Arc::new(AtomicBool::new(false));
        let c = cancel.clone();
        let mut worker = crate::capture_worker::Worker::spawn(move || {
            supervise(&mut helper, &c, &c, |_| true, Duration::from_millis(20))
        })
        .unwrap();
        cancel.store(true, Ordering::Relaxed);
        let deadline = Instant::now() + Duration::from_secs(5);
        let error = loop {
            if let Some(result) = worker.poll() {
                break result.unwrap_err();
            }
            assert!(
                Instant::now() < deadline,
                "IPC worker did not finish while child cleanup stalled"
            );
            std::thread::sleep(Duration::from_millis(2));
        };
        assert!(error.contains("NOT confirmed"));
        // SAFETY: probe is a distinct open description for the private test file.
        assert_ne!(
            unsafe { libc::flock(probe.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        drop(release);
        loop {
            // SAFETY: the owned probe descriptor remains open throughout this loop.
            if unsafe { libc::flock(probe.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "child did not release its lock after cleanup"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        drop(probe);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn graceful_stop_waits_for_finalization_beyond_abort_grace() {
        let mut wire = Vec::new();
        capture_ipc::write_complete(&mut wire, &Ok("recording finalized".into())).unwrap();
        let escaped = wire
            .iter()
            .map(|b| format!("\\{b:03o}"))
            .collect::<String>();
        let mut command = Command::new("sh");
        // Wait for stdin EOF, then simulate slow sync/publication without a device.
        command
            .arg("-c")
            .arg(format!("cat >/dev/null; sleep 0.1; printf '{escaped}'"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut helper = OwnedChild::spawn(&mut command).unwrap();
        let start = Instant::now();
        let result = supervise(
            &mut helper,
            &AtomicBool::new(true),
            &AtomicBool::new(false),
            |_| true,
            Duration::from_millis(10),
        );
        assert_eq!(result, Ok("recording finalized".into()));
        assert!(start.elapsed() >= Duration::from_millis(100));
    }

    #[test]
    fn process_frame_and_completion_require_clean_exit() {
        let frame = Frame {
            sequence: 7,
            host_timestamp_us: 123,
            flags: 0,
            metadata: vec![4, 5],
            data: vec![6, 7, 8],
        };
        let mut wire = Vec::new();
        capture_ipc::write_frame(&mut wire, &frame).unwrap();
        capture_ipc::write_complete(&mut wire, &Ok("cleanup finished".into())).unwrap();
        // Only internally generated octal byte escapes enter this test shell.
        let escaped = wire
            .iter()
            .map(|b| format!("\\{b:03o}"))
            .collect::<String>();
        for exit in [0, 1] {
            let mut command = Command::new("sh");
            command
                .arg("-c")
                .arg(format!("printf '{escaped}'; exit {exit}"))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            let mut helper = OwnedChild::spawn(&mut command).unwrap();
            let mut frames = Vec::new();
            let result = supervise(
                &mut helper,
                &AtomicBool::new(false),
                &AtomicBool::new(false),
                |frame| {
                    frames.push(frame);
                    true
                },
                Duration::from_secs(1),
            );
            assert_eq!(frames.as_slice(), std::slice::from_ref(&frame));
            if exit == 0 {
                assert_eq!(result, Ok("cleanup finished".into()));
            } else {
                assert!(result.unwrap_err().contains("exited"));
            }
        }
    }

    #[test]
    fn helper_diagnostic_is_private_and_independent_of_ipc() {
        use std::os::unix::fs::PermissionsExt;
        let (path, mut file) = private_log().unwrap();
        assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        file.write_all(b"cleanup failed after parent transport closed")
            .unwrap();
        drop(file);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"cleanup failed after parent transport closed"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn helper_exit_without_completion_is_not_success() {
        let mut command = Command::new("sh");
        command
            .args(["-c", "exit 0"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut helper = OwnedChild::spawn(&mut command).unwrap();
        let error = supervise(
            &mut helper,
            &AtomicBool::new(false),
            &AtomicBool::new(false),
            |_| true,
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(error.contains("complete final result"));
    }
}
