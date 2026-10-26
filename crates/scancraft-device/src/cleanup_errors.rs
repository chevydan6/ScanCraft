//! Collect cleanup failures without short-circuiting later restoration attempts.
use std::io;

#[derive(Default)]
pub(super) struct CleanupErrors {
    first_kind: Option<io::ErrorKind>,
    failures: Vec<String>,
}
impl CleanupErrors {
    pub(super) fn record(&mut self, phase: &str, result: io::Result<()>) {
        if let Err(error) = result {
            self.first_kind.get_or_insert(error.kind());
            self.failures.push(format!("{phase}: {error}"));
        }
    }
    /// Preserve the first failure kind and all phase diagnostics in call order.
    /// Successful operations add no diagnostics; no device operation is retried.
    pub(super) fn finish(self) -> io::Result<()> {
        match self.first_kind {
            Some(kind) => Err(io::Error::new(kind, self.failures.join("; "))),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_every_failed_phase_and_first_kind_across_successes() {
        let mut errors = CleanupErrors::default();
        errors.record(
            "stop streaming",
            Err(io::Error::from_raw_os_error(libc::ENODEV)),
        );
        errors.record(
            "release buffers",
            Err(io::Error::from_raw_os_error(libc::EBUSY)),
        );
        errors.record("restore format", Ok(()));
        errors.record("restore frame rate", Err(io::Error::other("rate rejected")));
        let error = errors.finish().unwrap_err();
        assert_eq!(
            error.kind(),
            io::Error::from_raw_os_error(libc::ENODEV).kind()
        );
        let message = error.to_string();
        let stop = message.find("stop streaming:").unwrap();
        let release = message.find("release buffers:").unwrap();
        let rate = message.find("restore frame rate: rate rejected").unwrap();
        assert!(stop < release && release < rate);
        assert!(!message.contains("restore format"));
        assert!(message.contains(&io::Error::from_raw_os_error(libc::ENODEV).to_string()));
        assert!(message.contains(&io::Error::from_raw_os_error(libc::EBUSY).to_string()));
    }

    #[test]
    fn empty_and_successful_cleanup_have_no_failure() {
        CleanupErrors::default().finish().unwrap();
        let mut errors = CleanupErrors::default();
        for phase in [
            "stop streaming",
            "release buffers",
            "restore format",
            "restore frame rate",
        ] {
            errors.record(phase, Ok(()));
        }
        errors.finish().unwrap();
    }
}
