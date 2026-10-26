//! Scoped CLI termination: signal handlers only set a lock-free atomic request.
use std::{
    io,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, AtomicI32, Ordering},
};
static ACTIVE: AtomicBool = AtomicBool::new(false);
static REQUESTED: AtomicI32 = AtomicI32::new(0);
extern "C" fn handle(signal: libc::c_int) {
    REQUESTED.store(signal, Ordering::Relaxed);
}
pub fn check() -> Result<(), String> {
    match REQUESTED.load(Ordering::Relaxed) {
        0 => Ok(()),
        libc::SIGINT => Err("native acquisition interrupted by SIGINT; cleanup requested".into()),
        libc::SIGTERM => Err("native acquisition interrupted by SIGTERM; cleanup requested".into()),
        _ => Err("native acquisition interrupted; cleanup requested".into()),
    }
}
pub struct Guard {
    previous: Vec<(libc::c_int, libc::sigaction)>,
}
impl Guard {
    pub fn install() -> io::Result<Self> {
        ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| io::Error::other("termination scope already active"))?;
        REQUESTED.store(0, Ordering::Relaxed);
        let mut guard = Self {
            previous: Vec::with_capacity(2),
        };
        // SAFETY: Linux sigaction is C POD; zero initializes mask/flags/restorer.
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = handle as *const () as usize;
        // Do not restart blocked operations: EINTR must unwind capture promptly.
        action.sa_flags = 0;
        // SAFETY: the mask is valid writable storage and both signal numbers are valid.
        if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0 {
            return Err(io::Error::last_os_error());
        }
        for signal in [libc::SIGINT, libc::SIGTERM] {
            // SAFETY: valid mask pointer and a catchable Linux signal.
            if unsafe { libc::sigaddset(&mut action.sa_mask, signal) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        for signal in [libc::SIGINT, libc::SIGTERM] {
            let mut old = MaybeUninit::uninit();
            // SAFETY: action and output live through the call; old is read only after success.
            if unsafe { libc::sigaction(signal, &action, old.as_mut_ptr()) } != 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: successful sigaction initialized the previous action.
            guard.previous.push((signal, unsafe { old.assume_init() }));
        }
        Ok(guard)
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        for (signal, action) in self.previous.iter().rev() {
            // SAFETY: restore exactly the initialized action saved for this signal.
            if unsafe { libc::sigaction(*signal, action, std::ptr::null_mut()) } != 0 {
                eprintln!(
                    "could not restore signal handler {signal}: {}",
                    io::Error::last_os_error()
                );
            }
        }
        REQUESTED.store(0, Ordering::Relaxed);
        ACTIVE.store(false, Ordering::Release);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::{Command, Stdio},
        sync::atomic::AtomicUsize,
        time::{Duration, Instant},
    };
    static PREVIOUS_CALLS: AtomicUsize = AtomicUsize::new(0);
    extern "C" fn previous(_: libc::c_int) {
        PREVIOUS_CALLS.fetch_add(1, Ordering::Relaxed);
    }
    #[test]
    fn isolated_signal_child() {
        if std::env::var_os("SCANCRAFT_TERMINATION_CHILD").is_none() {
            return;
        }
        // SAFETY: this helper runs alone in a separate test process; no other tests' handlers change.
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = previous as *const () as usize;
        let mut original = Vec::new();
        for signal in [libc::SIGINT, libc::SIGTERM] {
            let mut old = MaybeUninit::uninit();
            // SAFETY: valid POD action/output pointers and catchable signals in the isolated child.
            assert_eq!(
                unsafe { libc::sigaction(signal, &action, old.as_mut_ptr()) },
                0
            );
            // SAFETY: successful call initialized old.
            original.push((signal, unsafe { old.assume_init() }));
        }
        for signal in [libc::SIGINT, libc::SIGTERM] {
            let guard = Guard::install().unwrap();
            assert!(check().is_ok());
            // SAFETY: signal is handled by our atomic-only scoped callback.
            assert_eq!(unsafe { libc::raise(signal) }, 0);
            assert!(check().unwrap_err().contains(if signal == libc::SIGINT {
                "SIGINT"
            } else {
                "SIGTERM"
            }));
            assert!(Guard::install().is_err());
            assert!(check().is_err());
            drop(guard);
            assert!(check().is_ok());
            // SAFETY: restored test callback only increments an atomic in this child process.
            assert_eq!(unsafe { libc::raise(signal) }, 0);
        }
        assert_eq!(PREVIOUS_CALLS.load(Ordering::Relaxed), 2);
        for (signal, old) in original {
            // SAFETY: restore initialized previous action before leaving the isolated test.
            assert_eq!(
                unsafe { libc::sigaction(signal, &old, std::ptr::null_mut()) },
                0
            );
        }
    }
    #[test]
    fn both_signals_cancel_nested_scope_is_rejected_and_previous_handlers_are_restored() {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "termination::tests::isolated_signal_child",
                "--nocapture",
            ])
            .env("SCANCRAFT_TERMINATION_CHILD", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let start = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if start.elapsed() > Duration::from_secs(5) {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("isolated signal test timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
