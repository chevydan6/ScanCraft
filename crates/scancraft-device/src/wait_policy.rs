//! Cooperative wait policy only. A timeout cannot preempt a blocked ioctl or read.
use std::{
    io,
    time::{Duration, Instant},
};
pub(crate) fn dequeue<T>(
    deadline: Instant,
    maximum: Duration,
    stopped: &impl Fn() -> bool,
    read: impl FnOnce(Duration) -> io::Result<T>,
) -> io::Result<Option<T>> {
    if stopped() {
        return Ok(None);
    }
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::TimedOut, "validated-frame deadline elapsed")
        })?;
    match read(maximum.min(remaining)) {
        Ok(value) => Ok(Some(value)),
        // Do not turn disconnects or other driver failures into a successful stop.
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
            ) && stopped() =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn stop_and_expired_deadline_prevent_another_read() {
        let future = Instant::now() + Duration::from_secs(10);
        assert_eq!(
            dequeue(
                future,
                Duration::from_secs(2),
                &|| true,
                |_| -> io::Result<u8> { panic!("read after stop") }
            )
            .unwrap(),
            None
        );
        let past = Instant::now() - Duration::from_secs(1);
        assert_eq!(
            dequeue(
                past,
                Duration::from_secs(2),
                &|| false,
                |_| -> io::Result<u8> { panic!("read after deadline") }
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::TimedOut
        );
    }
    #[test]
    fn remaining_deadline_caps_the_supplied_wait() {
        let maximum = Duration::from_secs(2);
        let deadline = Instant::now() + Duration::from_secs(1);
        assert_eq!(
            dequeue(deadline, maximum, &|| false, |timeout| {
                assert!(timeout <= Duration::from_secs(1));
                assert!(!timeout.is_zero());
                Ok(7)
            })
            .unwrap(),
            Some(7)
        );
    }
    #[test]
    fn stopping_during_timeout_preserves_real_errors_and_successful_data() {
        let stop = Cell::new(false);
        let deadline = Instant::now() + Duration::from_secs(10);
        let stopped = || stop.get();
        let result = dequeue(
            deadline,
            Duration::from_secs(2),
            &stopped,
            |_| -> io::Result<u8> {
                stop.set(true);
                Err(io::ErrorKind::TimedOut.into())
            },
        );
        assert_eq!(result.unwrap(), None);
        stop.set(false);
        let result = dequeue(
            deadline,
            Duration::from_secs(2),
            &stopped,
            |_| -> io::Result<u8> {
                stop.set(true);
                Err(io::ErrorKind::NotConnected.into())
            },
        );
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotConnected);
        stop.set(false);
        let result = dequeue(deadline, Duration::from_secs(2), &stopped, |_| {
            stop.set(true);
            Ok(9)
        });
        assert_eq!(result.unwrap(), Some(9));
    }
}
