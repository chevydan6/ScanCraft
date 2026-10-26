//! Capture-thread completion only; no scanner, UI or file ownership policy.
use std::{
    io,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread::{self, JoinHandle},
};

type Outcome = Result<String, String>;
pub(crate) struct Worker {
    result: Receiver<Outcome>,
    pending: Option<Outcome>,
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    pub(crate) fn spawn(run: impl FnOnce() -> Outcome + Send + 'static) -> io::Result<Self> {
        let (sender, result) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("scancraft-capture".into())
            .spawn(move || {
                let _ = sender.send(run());
            })?;
        Ok(Self {
            result,
            pending: None,
            thread: Some(thread),
        })
    }
    /// Never joins a live thread. A result can precede final thread cleanup.
    /// Disconnection alone does not permit a replacement capture while cleanup
    /// is still running. Panics override any previously sent success message.
    pub(crate) fn poll(&mut self) -> Option<Outcome> {
        self.receive_result();
        self.finish_if_ready()
    }
    fn receive_result(&mut self) {
        match self.result.try_recv() {
            Ok(result) => self.pending = Some(result),
            Err(TryRecvError::Disconnected) if self.pending.is_none() => {
                self.pending = Some(Err("Capture worker disconnected without a result".into()));
            }
            Err(_) => {}
        }
    }
    fn finish_if_ready(&mut self) -> Option<Outcome> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        if self.join().is_err() {
            return Some(Err("Capture worker panicked".into()));
        }
        // Publication may race with the first receive. After join, no later
        // sender can appear, so consume the final result before classifying it.
        self.receive_result();
        Some(
            self.pending
                .take()
                .unwrap_or_else(|| Err("Capture worker finished without a result".into())),
        )
    }
    /// Application shutdown retains ownership/cleanup duties. This synchronous
    /// join has no bounded-driver-shutdown guarantee (independent SCAN-REV-005).
    pub(crate) fn join(&mut self) -> thread::Result<()> {
        self.thread.take().map_or(Ok(()), |thread| thread.join())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn late_result_between_empty_receive_and_thread_finish_is_retained() {
        let (release, wait) = mpsc::channel();
        let mut worker = Worker::spawn(move || {
            wait.recv().unwrap();
            Ok("late completion".into())
        })
        .unwrap();
        // Force the first poll phase to observe Empty before publishing.
        worker.receive_result();
        assert!(worker.pending.is_none());
        release.send(()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !worker.thread.as_ref().unwrap().is_finished() {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        assert_eq!(worker.finish_if_ready(), Some(Ok("late completion".into())));
    }
    #[test]
    fn result_before_thread_cleanup_does_not_block_poll() {
        let (sender, result) = mpsc::channel();
        let (ready, started) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let thread = thread::spawn(move || {
            sender.send(Ok("completed".into())).unwrap();
            ready.send(()).unwrap();
            let _ = wait.recv();
        });
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        let mut worker = Worker {
            result,
            pending: None,
            thread: Some(thread),
        };
        let (returned, observed) = mpsc::channel();
        let poller = thread::spawn(move || {
            let result = worker.poll();
            returned.send((result, worker)).unwrap();
        });
        let outcome = observed.recv_timeout(Duration::from_secs(5));
        // Always release before asserting, so a regression cannot strand threads.
        release.send(()).unwrap();
        poller.join().unwrap();
        let (result, mut worker) = outcome.expect("poll blocked on a live worker");
        assert!(result.is_none());
        worker.join().unwrap();
        assert_eq!(worker.pending.take(), Some(Ok("completed".into())));
    }
    #[test]
    fn clean_exit_without_result_reports_disconnection() {
        let (sender, result) = mpsc::channel();
        let thread = thread::spawn(move || drop(sender));
        let mut worker = Worker {
            result,
            pending: None,
            thread: Some(thread),
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let result = loop {
            if let Some(result) = worker.poll() {
                break result;
            }
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        };
        assert!(result.unwrap_err().contains("disconnected"));
        assert!(worker.poll().is_none());
    }
}
