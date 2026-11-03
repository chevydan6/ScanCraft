//! Capture cleanup precedes recording drain, with peer cancellation on failure.
use std::sync::atomic::{AtomicBool, Ordering};

/// Eight owned frames per camera absorb finite storage stalls; overflow fails capture.
pub(super) const QUEUE_CAPACITY: usize = 8;

pub(super) fn complete<T>(
    acquired: Result<T, String>,
    cancel: &AtomicBool,
    close_camera: impl FnOnce() -> Result<(), String>,
    finish_recording: impl FnOnce() -> Result<(), String>,
) -> Result<T, String> {
    if acquired.is_err() {
        cancel.store(true, Ordering::Relaxed);
    }
    let closed = close_camera();
    if closed.is_err() {
        cancel.store(true, Ordering::Relaxed);
    }
    let recorded = finish_recording();
    if recorded.is_err() {
        cancel.store(true, Ordering::Relaxed);
    }
    match (acquired, closed, recorded) {
        (Ok(report), Ok(()), Ok(())) => Ok(report),
        (acquired, closed, recorded) => Err([acquired.err(), closed.err(), recorded.err()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("; ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn failures_cancel_peer_before_recording_drain_and_retain_all_errors() {
        for acquisition_fails in [false, true] {
            let cancel = AtomicBool::new(false);
            let order = RefCell::new(Vec::new());
            let result = complete(
                if acquisition_fails {
                    Err("capture error".into())
                } else {
                    Ok(7)
                },
                &cancel,
                || {
                    assert_eq!(cancel.load(Ordering::Relaxed), acquisition_fails);
                    order.borrow_mut().push("camera");
                    Err("close error".into())
                },
                || {
                    assert!(cancel.load(Ordering::Relaxed));
                    order.borrow_mut().push("recording");
                    Err("disk error".into())
                },
            );
            let error = result.unwrap_err();
            assert_eq!(error.contains("capture error"), acquisition_fails);
            assert!(error.contains("close error") && error.contains("disk error"));
            assert_eq!(*order.borrow(), ["camera", "recording"]);
        }
    }

    #[test]
    fn successful_capture_requires_successful_recording_completion() {
        let cancel = AtomicBool::new(false);
        assert_eq!(complete(Ok(7), &cancel, || Ok(()), || Ok(())).unwrap(), 7);
        assert!(!cancel.load(Ordering::Relaxed));
        assert_eq!(
            complete(Ok(7), &cancel, || Ok(()), || Err("disk error".into())),
            Err("disk error".into())
        );
        assert!(cancel.load(Ordering::Relaxed));
    }
}
