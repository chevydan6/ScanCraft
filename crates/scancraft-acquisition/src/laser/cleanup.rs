//! Keep dependent controls after the last possible stream-format mutation.
use super::super::{Result, combine};
use scancraft_device::{ir_gain::ScopedIrGain, stream_group::SettingsLease};

pub(super) trait StreamRestore {
    fn restore(&mut self) -> Result<()>;
    /// Consumes ownership; no deferred stream mutations after returning.
    fn finish(self) -> Result<()>;
}
pub(super) trait GainRestore {
    /// Must not trust a cached success from before a stream restoration attempt.
    fn restore_after_settings(&mut self) -> Result<()>;
}
impl StreamRestore for SettingsLease {
    fn restore(&mut self) -> Result<()> {
        self.restore().map_err(|e| e.to_string())
    }
    fn finish(self) -> Result<()> {
        self.finish().map_err(|e| e.to_string())
    }
}
impl GainRestore for ScopedIrGain<'_> {
    fn restore_after_settings(&mut self) -> Result<()> {
        self.restore_after_settings().map_err(|e| e.to_string())
    }
}
pub(super) struct Settings<S: StreamRestore, G: GainRestore> {
    stream: Option<S>,
    pub(super) gain: Option<G>,
    restored: bool,
}
impl<S: StreamRestore, G: GainRestore> Settings<S, G> {
    pub(super) fn new(stream: S, gain: Option<G>) -> Self {
        Self {
            stream: Some(stream),
            gain,
            restored: false,
        }
    }
    pub(super) fn restore(&mut self) -> Result<()> {
        let stream = self.stream.as_mut().map(StreamRestore::restore).transpose();
        let gain = self
            .gain
            .as_mut()
            .map(GainRestore::restore_after_settings)
            .transpose();
        let result = combine(stream.map(|_| ()), gain.map(|_| ()));
        self.restored = result.is_ok();
        result
    }
    fn finalize(&mut self) -> Result<()> {
        if self.restored {
            // Both native leases cache success. Drop performs no more writes:
            // do not introduce another control operation after publication.
            self.stream.take();
            self.gain.take();
            return Ok(());
        }
        let stream = self.stream.take().map(StreamRestore::finish).transpose();
        // Even a failed final stream attempt may have changed either gain.
        let gain = self
            .gain
            .as_mut()
            .map(GainRestore::restore_after_settings)
            .transpose();
        // A failed gain's own Drop may retry, but no stream owner remains.
        self.gain.take();
        combine(stream.map(|_| ()), gain.map(|_| ()))
    }
    pub(super) fn finish(mut self) -> Result<()> {
        self.finalize()
    }
}
impl<S: StreamRestore, G: GainRestore> Drop for Settings<S, G> {
    fn drop(&mut self) {
        if let Err(e) = self.finalize() {
            scancraft_core::diagnostics::note(format_args!("{e}"));
        }
    }
}

#[cfg(test)]
mod tests;
