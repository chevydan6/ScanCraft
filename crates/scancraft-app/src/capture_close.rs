//! Ordinary window-close intent, separate from capture-thread ownership.
//! Waiting has no forced deadline: a driver call may be uninterruptible.
use std::time::{Duration, Instant};
#[derive(Default)]
pub(crate) struct CaptureClose {
    requested_at: Option<Instant>,
}
#[derive(Debug, PartialEq)]
pub(crate) enum Action {
    None,
    CancelViewportClose,
    RetryViewportClose,
}
impl CaptureClose {
    pub(crate) fn advance(&mut self, requested: bool, active: bool, now: Instant) -> Action {
        if requested && active {
            self.requested_at.get_or_insert(now);
            return Action::CancelViewportClose;
        }
        if self.is_waiting() && !active {
            self.requested_at = None;
            return Action::RetryViewportClose;
        }
        Action::None
    }
    pub(crate) fn is_waiting(&self) -> bool {
        self.requested_at.is_some()
    }
    pub(crate) fn slow(&self, now: Instant) -> bool {
        self.requested_at
            .and_then(|start| now.checked_duration_since(start))
            .is_some_and(|elapsed| elapsed >= Duration::from_secs(10))
    }
    pub(crate) fn keep_open(&mut self) {
        self.requested_at = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn close_waits_for_termination_then_retries_exactly_once() {
        let mut close = CaptureClose::default();
        let now = Instant::now();
        assert_eq!(close.advance(true, true, now), Action::CancelViewportClose);
        assert!(close.is_waiting());
        assert_eq!(close.advance(false, true, now), Action::None);
        assert_eq!(close.advance(false, false, now), Action::RetryViewportClose);
        assert!(!close.is_waiting());
        assert_eq!(close.advance(false, false, now), Action::None);
    }
    #[test]
    fn prolonged_or_repeated_close_never_forces_release_or_resets_elapsed() {
        let mut close = CaptureClose::default();
        let now = Instant::now();
        close.advance(true, true, now);
        assert!(!close.slow(now + Duration::from_secs(9)));
        assert_eq!(
            close.advance(true, true, now + Duration::from_secs(10)),
            Action::CancelViewportClose
        );
        assert!(close.slow(now + Duration::from_secs(10)));
        assert_eq!(
            close.advance(false, true, now + Duration::from_secs(3600)),
            Action::None
        );
        assert!(close.is_waiting());
    }
    #[test]
    fn keeping_open_cancels_only_close_intent_and_idle_close_is_untouched() {
        let mut close = CaptureClose::default();
        let now = Instant::now();
        assert_eq!(close.advance(true, false, now), Action::None);
        close.advance(true, true, now);
        close.keep_open();
        assert!(!close.is_waiting());
        assert_eq!(close.advance(false, false, now), Action::None);
        assert!(!close.slow(now + Duration::from_secs(20)));
    }
}
