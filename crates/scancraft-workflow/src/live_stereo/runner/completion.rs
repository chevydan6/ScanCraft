//! Two source owners finish acquisition before either explicitly closes its source.
//! A missing worker (spawn/open failure or unwind) cancels and retires its slot.
//! Waiting does not preempt a peer's driver call or establish a cleanup deadline.
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};
struct Shared {
    finished: Mutex<u8>,
    changed: Condvar,
    stop: Arc<AtomicBool>,
}
pub(super) struct Participant {
    shared: Arc<Shared>,
    bit: u8,
    arrived: bool,
}
/// Unique, non-cloneable slots exist before spawning. Dropping an unstarted slot
/// releases its peer, including when a thread builder drops a failed spawn closure.
pub(super) fn participants(stop: Arc<AtomicBool>) -> [Participant; 2] {
    let shared = Arc::new(Shared {
        finished: Mutex::new(0),
        changed: Condvar::new(),
        stop,
    });
    [1, 2].map(|bit| Participant {
        shared: shared.clone(),
        bit,
        arrived: false,
    })
}
impl Participant {
    pub(super) fn acquisition_finished(&mut self) {
        let mut finished = self
            .shared
            .finished
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *finished |= self.bit;
        self.arrived = true;
        self.shared.changed.notify_all();
        // Stop requests, including graceful stop, do not mean an in-flight peer
        // read has returned. Keep source ownership until that peer retires too.
        while *finished != 3 {
            finished = self
                .shared
                .changed
                .wait(finished)
                .unwrap_or_else(|e| e.into_inner());
        }
    }
}
impl Drop for Participant {
    fn drop(&mut self) {
        if !self.arrived {
            self.shared.stop.store(true, Ordering::Relaxed);
            let mut finished = self
                .shared
                .finished
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            *finished |= self.bit;
            self.shared.changed.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, thread, time::Duration};

    #[test]
    fn first_source_waits_for_peer_even_after_stop_request() {
        for stop_requested in [false, true] {
            let stop = Arc::new(AtomicBool::new(stop_requested));
            let [mut first, mut second] = participants(stop.clone());
            let observe = first.shared.clone();
            let (closed, closure) = mpsc::channel();
            let owner = thread::spawn(move || {
                first.acquisition_finished();
                closed.send(()).unwrap();
            });
            let lock = observe.finished.lock().unwrap();
            let (lock, timeout) = observe
                .changed
                .wait_timeout_while(lock, Duration::from_secs(5), |mask| *mask & 1 == 0)
                .unwrap();
            let arrived = *lock & 1 != 0;
            drop(lock);
            let early = closure.try_recv();
            second.acquisition_finished();
            owner.join().unwrap();
            assert!(arrived && !timeout.timed_out());
            assert!(matches!(early, Err(mpsc::TryRecvError::Empty)));
            closure.recv().unwrap();
            assert_eq!(stop.load(Ordering::Relaxed), stop_requested);
        }
    }
    #[test]
    fn missing_worker_slot_cancels_and_releases_the_started_peer() {
        let stop = Arc::new(AtomicBool::new(false));
        let [mut started, unstarted] = participants(stop.clone());
        let owner = thread::spawn(move || started.acquisition_finished());
        drop(unstarted);
        owner.join().unwrap();
        assert!(stop.load(Ordering::Relaxed));
    }
    #[test]
    fn worker_unwind_retires_its_slot_without_detaching_a_waiting_peer() {
        let stop = Arc::new(AtomicBool::new(false));
        let [mut started, failing] = participants(stop.clone());
        let owner = thread::spawn(move || started.acquisition_finished());
        let failed = thread::spawn(move || {
            let _slot = failing;
            panic!("test worker open panic");
        });
        assert!(failed.join().is_err());
        owner.join().unwrap();
        assert!(stop.load(Ordering::Relaxed));
    }
}

#[cfg(test)]
mod worker_tests;

#[cfg(test)]
mod cancellation_tests;
