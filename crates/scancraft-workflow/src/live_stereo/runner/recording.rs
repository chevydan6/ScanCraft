//! Camera ownership is independent of bounded recording writes. Close the source
//! before draining storage, including acquisition failure and unwinding paths.
use super::{Control, Event, Result, Sample, Side, Source, SourceReport, completion::Participant};
use crate::laser_stream::bundle::io::CappedFile;
use scancraft_core::StreamProfile;
use scancraft_recording::{Writer, queued::QueuedWriter};
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, TrySendError},
    },
};

/// Same bounded backlog as the finite native capture path. Full admission fails;
/// it never becomes an unreported loss or a wait for storage on the camera thread.
pub(super) const QUEUE_CAPACITY: usize = 8;

pub(super) struct RecordingLimits {
    frames: usize,
    bytes: Option<u64>,
}
impl From<usize> for RecordingLimits {
    fn from(frames: usize) -> Self {
        Self {
            frames,
            bytes: None,
        }
    }
}
impl From<(usize, Option<u64>)> for RecordingLimits {
    fn from((frames, bytes): (usize, Option<u64>)) -> Self {
        Self { frames, bytes }
    }
}
enum Sink {
    Legacy(BufWriter<std::fs::File>),
    Budgeted(CappedFile),
}
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Legacy(out) => out.write(bytes),
            Self::Budgeted(out) => out.write(bytes),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Legacy(out) => out.flush(),
            Self::Budgeted(out) => out.flush(),
        }
    }
}
impl Sink {
    fn finish(self) -> Result<()> {
        match self {
            Self::Legacy(out) => out.get_ref().sync_all().map_err(|e| e.to_string()),
            Self::Budgeted(out) => out.finish().map(|_| ()).map_err(|e| e.to_string()),
        }
    }
}

pub(super) fn record_controlled<S: Source>(
    profile: StreamProfile,
    path: &Path,
    maximum: impl Into<RecordingLimits>,
    signals: (&AtomicBool, &Control, Option<&mut Participant>),
    sender: SyncSender<Event>,
    side: Side,
    open: impl FnOnce() -> Result<S>,
) -> Result<SourceReport> {
    let maximum = maximum.into();
    record_with_sink(
        profile,
        maximum.frames,
        signals,
        sender,
        side,
        open,
        (
            || {
                if let Some(limit) = maximum.bytes {
                    let root = path.parent().ok_or("recording lacks parent")?;
                    let role = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .ok_or("invalid recording name")?;
                    return CappedFile::new(root, role, limit)
                        .map(Sink::Budgeted)
                        .map_err(|e| e.to_string());
                }
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .map(|f| Sink::Legacy(BufWriter::new(f)))
                    .map_err(|e| e.to_string())
            },
            Sink::finish,
        ),
    )
}

// A private sink seam permits deterministic storage stalls/failures while testing
// the actual camera worker. The public Source/bundle APIs remain unchanged.
fn record_with_sink<S: Source, W: Write + Send + 'static>(
    profile: StreamProfile,
    maximum: usize,
    signals: (&AtomicBool, &Control, Option<&mut Participant>),
    sender: SyncSender<Event>,
    side: Side,
    open: impl FnOnce() -> Result<S>,
    sink: (impl FnOnce() -> Result<W>, impl FnOnce(W) -> Result<()>),
) -> Result<SourceReport> {
    let (stop, control, completion) = signals;
    // Outside the unwind boundary: a source panic must not join a blocked writer
    // while the camera is still open. Declaration order also protects outer unwind.
    let mut writer = None;
    let mut source = open()?;
    let acquired = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let output = (sink.0)()?;
        let direct = Writer::with_context(output, profile.clone(), source.context()?)
            .map_err(|e| e.to_string())?;
        writer = Some(QueuedWriter::new(direct, QUEUE_CAPACITY).map_err(|e| e.to_string())?);
        let mut report = SourceReport {
            interface: profile.interface,
            recorded_frames: 0,
            queued_frames: 0,
            processing_queue_drops: 0,
            statistics: Value::Null,
        };
        for index in 0..maximum {
            if stop.load(Ordering::Relaxed) || control.stop_requested() {
                break;
            }
            let Some(frame) =
                source.next_until(&|| stop.load(Ordering::Relaxed) || control.stop_requested())?
            else {
                break;
            };
            // Processing and recording each own their bytes. The recording queue
            // cannot borrow a frame that is concurrently consumed by processing.
            writer
                .as_mut()
                .unwrap()
                .push_copy(&frame)
                .map_err(|e| e.to_string())?;
            report.recorded_frames += 1;
            control.recorded[side.index()].fetch_add(1, Ordering::Relaxed);
            match sender.try_send(Event {
                side,
                sample: Sample { index, frame },
            }) {
                Ok(()) => report.queued_frames += 1,
                Err(TrySendError::Full(_)) => {
                    report.processing_queue_drops += 1;
                    control.queue_drops[side.index()].fetch_add(1, Ordering::Relaxed);
                }
                Err(TrySendError::Disconnected(_)) => {
                    report.processing_queue_drops += 1;
                    control.queue_drops[side.index()].fetch_add(1, Ordering::Relaxed);
                    break;
                }
            }
        }
        report.statistics = source.statistics();
        Ok(report)
    }))
    .unwrap_or_else(|_| Err("capture worker panicked".into()));
    if acquired.is_err() {
        stop.store(true, Ordering::Relaxed);
    }
    if let Some(completion) = completion {
        completion.acquisition_finished();
    }
    let closed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| source.close()))
        .unwrap_or_else(|_| Err("source cleanup panicked".into()));
    if closed.is_err() {
        stop.store(true, Ordering::Relaxed);
    }
    let recorded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if let Some(writer) = writer.take() {
            let output = writer.finish().map_err(|e| e.to_string())?;
            (sink.1)(output)?;
        }
        Ok(())
    }))
    .unwrap_or_else(|_| Err("recording completion panicked".into()));
    if recorded.is_err() {
        stop.store(true, Ordering::Relaxed);
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
mod tests;
