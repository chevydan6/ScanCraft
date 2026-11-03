//! Finite stereo acquisitions retain their own cameras until both are complete.
//! Peer failures/panics/spawn failures set the existing shared cancellation flag.
//! This does not bound driver calls or count unobserved exposures while waiting.
use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    thread,
    time::Duration,
};

#[derive(Default)]
pub(super) struct Completion {
    arrived: AtomicUsize,
}
impl Completion {
    pub(super) fn wait(
        &self,
        cancelled: &AtomicBool,
        check: impl Fn() -> Result<(), String>,
    ) -> Result<(), String> {
        if self.arrived.fetch_add(1, Ordering::AcqRel) >= 2 {
            return Err("stereo completion received more than two workers".into());
        }
        loop {
            check()?;
            if cancelled.load(Ordering::Relaxed) {
                return Err("stereo completion cancelled because another worker failed".into());
            }
            if self.arrived.load(Ordering::Acquire) == 2 {
                return Ok(());
            }
            thread::park_timeout(Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, mpsc};

    #[test]
    fn first_camera_cannot_close_until_peer_finishes_acquisition() {
        let completion = Arc::new(Completion::default());
        let cancel = Arc::new(AtomicBool::new(false));
        let (checked, ready) = mpsc::sync_channel(1);
        let (closed, observed) = mpsc::channel();
        let gate = completion.clone();
        let stop = cancel.clone();
        let first = thread::spawn(move || {
            let result = gate.wait(&stop, || {
                let _ = checked.try_send(());
                Ok(())
            });
            closed.send(result).unwrap();
        });
        let started = ready.recv_timeout(Duration::from_secs(2));
        let early = observed.try_recv();
        // Release the worker even if the test's readiness observation fails.
        let second = if started.is_ok() {
            completion.wait(&cancel, || Ok(()))
        } else {
            cancel.store(true, Ordering::Relaxed);
            Err("test worker did not start".into())
        };
        first.join().unwrap();
        assert!(started.is_ok());
        assert!(matches!(early, Err(mpsc::TryRecvError::Empty)));
        assert!(second.is_ok());
        assert!(observed.recv().unwrap().is_ok());
    }

    #[test]
    fn peer_failure_releases_a_waiting_camera_without_a_second_arrival() {
        let completion = Arc::new(Completion::default());
        let cancel = Arc::new(AtomicBool::new(false));
        let (checked, ready) = mpsc::sync_channel(1);
        let gate = completion.clone();
        let stop = cancel.clone();
        let first = thread::spawn(move || {
            gate.wait(&stop, || {
                let _ = checked.try_send(());
                Ok(())
            })
        });
        let started = ready.recv_timeout(Duration::from_secs(2));
        cancel.store(true, Ordering::Relaxed);
        let result = first.join().unwrap();
        assert!(started.is_ok());
        assert!(result.unwrap_err().contains("another worker failed"));
        assert_eq!(completion.arrived.load(Ordering::Acquire), 1);
    }

    #[test]
    fn termination_errors_and_extra_arrivals_never_appear_as_completion() {
        let gate = Completion::default();
        let cancel = AtomicBool::new(false);
        assert_eq!(
            gate.wait(&cancel, || Err("termination requested".into())),
            Err("termination requested".into())
        );
        cancel.store(true, Ordering::Relaxed);
        assert!(
            gate.wait(&cancel, || Ok(()))
                .unwrap_err()
                .contains("cancelled")
        );
        assert!(
            gate.wait(&cancel, || Ok(()))
                .unwrap_err()
                .contains("more than two")
        );
    }
}
