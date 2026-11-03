//! Bounded recording worker. Admission never waits for storage or silently drops a frame.
//!
//! `finish` and Drop close admission, drain admitted frames and join the worker.
//! These can wait for filesystem operations; this is not a hard cleanup deadline.
//! A finished recording prefix is not proof that its enclosing capture succeeded.
use crate::{MAX_FRAME, Writer, invalid};
use scancraft_core::{Frame, StreamProfile, validate_frame};
use std::{
    borrow::Cow,
    io::{self, Write},
    sync::mpsc::{self, SyncSender, TrySendError},
    thread::{self, JoinHandle},
};

pub struct QueuedWriter<W: Write + Send + 'static> {
    sender: Option<SyncSender<Frame>>,
    worker: Option<JoinHandle<io::Result<W>>>,
    profile: StreamProfile,
    failure: Option<(io::ErrorKind, String)>,
}
impl<W: Write + Send + 'static> QueuedWriter<W> {
    /// At most `capacity` queued frames plus one worker-owned frame are retained.
    /// Each admitted frame obeys the recording format's existing size limits.
    /// The worker also uses the existing Writer's temporary serialization buffer.
    pub fn new(mut writer: Writer<W>, capacity: usize) -> io::Result<Self> {
        if !(1..=32).contains(&capacity) {
            return Err(invalid("recording queue capacity must be 1..32 frames"));
        }
        let profile = writer.profile.clone();
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let worker = thread::Builder::new()
            .name(format!("scancraft-record-{}", profile.interface))
            .spawn(move || {
                for frame in receiver {
                    writer.push(&frame)?;
                }
                writer.finish()
            })?;
        Ok(Self {
            sender: Some(sender),
            worker: Some(worker),
            profile,
            failure: None,
        })
    }

    /// Transfers ownership on success. Any admission error poisons completion;
    /// callers must stop acquisition and must not publish a successful capture.
    /// Writer ordering, CRC and I/O errors are also checked before `finish` succeeds.
    pub fn push(&mut self, frame: Frame) -> io::Result<()> {
        self.admit(Cow::Owned(frame))
    }

    /// Validate the existing frame/size limits before allocating an owned copy.
    /// For callers that also retain pixels for processing. Admission has the same
    /// poisoning/overflow behavior as `push`; only admitted frames are required
    /// to appear in a successful recording. The caller's bytes remain unchanged.
    pub fn push_copy(&mut self, frame: &Frame) -> io::Result<()> {
        self.admit(Cow::Borrowed(frame))
    }

    fn admit(&mut self, frame: Cow<'_, Frame>) -> io::Result<()> {
        if let Some((kind, message)) = &self.failure {
            return Err(io::Error::new(*kind, message.clone()));
        }
        let result = (|| {
            if frame.metadata.len() > 4096
                || frame
                    .metadata
                    .len()
                    .checked_add(frame.data.len())
                    .and_then(|n| n.checked_add(24))
                    .is_none_or(|n| n > MAX_FRAME)
            {
                return Err(invalid("frame exceeds size limits"));
            }
            validate_frame(&self.profile, &frame).map_err(invalid)?;
            match self.sender.as_ref().unwrap().try_send(frame.into_owned()) {
                Ok(()) => Ok(()),
                Err(TrySendError::Full(_)) => Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "recording queue full; capture cannot continue without losing frames",
                )),
                // Joining here could wait for a failing sink's destructor/flush.
                // Keep admission nonblocking; finish retrieves the worker error.
                Err(TrySendError::Disconnected(_)) => Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "recording worker disconnected; finish is required to collect its failure",
                )),
            }
        })();
        if let Err(error) = &result {
            self.failure = Some((error.kind(), error.to_string()));
        }
        result
    }

    fn join(&mut self) -> io::Result<W> {
        self.sender.take();
        self.worker
            .take()
            .ok_or_else(|| io::Error::other("recording worker already joined"))?
            .join()
            .map_err(|_| io::Error::other("recording worker panicked"))?
    }

    /// Drains and joins before returning the existing Writer's finished output.
    /// A previous admission error can never become successful completion.
    pub fn finish(mut self) -> io::Result<W> {
        let joined = self.join();
        match (self.failure.take(), joined) {
            (None, result) => result,
            (Some((kind, message)), Ok(_)) => Err(io::Error::new(kind, message)),
            (Some((kind, message)), Err(error)) => Err(io::Error::new(
                kind,
                format!("{message}; recording completion: {error}"),
            )),
        }
    }
}
impl<W: Write + Send + 'static> Drop for QueuedWriter<W> {
    fn drop(&mut self) {
        if self.worker.is_some()
            && let Err(error) = self.join()
        {
            scancraft_core::diagnostics::note(format_args!(
                "recording worker cleanup failed: {error}"
            ));
        }
    }
}

#[cfg(test)]
mod tests;
