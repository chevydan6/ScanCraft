use super::{
    Association, AssociationCounts, Camera, PROCESSING_QUEUE_CAPACITY, Pair, Pipeline, Result,
    Sample,
};
use crate::live_stereo::runner::{Source, completion};
use scancraft_core::{Frame, StreamProfile};
use scancraft_depth::laser_pair::{FrameInput, Report};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::{CaptureContext, Writer};
use serde::Serialize;
use serde_json::Value;
use std::{
    fs::File,
    io::BufWriter,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread,
    time::Duration,
};

#[cfg(test)]
mod retirement_tests;

/// Required raw retention. push must complete before processing admission.
/// finish must flush/validate/finalize its output or return a truthful error.
/// Adapters own no-overwrite paths, provenance, recovery and publication policy.
pub trait Journal {
    fn push(&mut self, frame: &Frame) -> Result<()>;
    fn finish(self) -> Result<()>;
}
impl Journal for Writer<BufWriter<File>> {
    fn push(&mut self, frame: &Frame) -> Result<()> {
        Writer::push(self, frame).map_err(|e| e.to_string())
    }
    fn finish(self) -> Result<()> {
        let out = Writer::finish(self).map_err(|e| e.to_string())?;
        out.get_ref().sync_all().map_err(|e| e.to_string())
    }
}
#[derive(Debug, Serialize)]
pub struct SourceReport {
    pub camera_index: usize,
    pub interface: u8,
    pub opened: bool,
    pub cleanup_confirmed: bool,
    pub journal_finalized: bool,
    pub journaled_frames: u64,
    pub queued_frames: u64,
    pub processing_queue_drops: u64,
    pub statistics: Value,
    pub outcome: Result<()>,
}
#[derive(Debug, Serialize)]
pub struct Summary {
    pub sources: Vec<SourceReport>,
    pub association: AssociationCounts,
    pub unmatched_pending: [usize; 2],
    pub undelivered_queued_frames: [Option<u64>; 2],
    pub processed_pairs: u64,
    pub failed_processing_pairs: u64,
    pub graceful_stop_requested: bool,
    pub synchronization_verified: bool,
    pub physical_accuracy_qualified: bool,
}
#[derive(Serialize)]
pub struct Failure {
    pub reason: String,
    pub partial: Option<Box<Summary>>,
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Failure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}
impl std::error::Error for Failure {}
struct Event {
    camera: Camera,
    sample: Sample,
}
fn combine(result: Result<()>, cleanup: Result<()>) -> Result<()> {
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
        (Err(e), Err(c)) => Err(format!("{e}; cleanup: {c}")),
    }
}
fn caught<T>(call: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(call))
        .unwrap_or_else(|_| Err("laser adapter/consumer panicked".into()))
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressSnapshot {
    pub journaled_frames: [u64; 2],
    pub processing_queue_drops: [u64; 2],
}
#[derive(Default)]
struct Progress {
    recorded: [AtomicU64; 2],
    drops: [AtomicU64; 2],
}
impl Progress {
    fn snapshot(&self) -> ProgressSnapshot {
        // Read drops first: acquiring a published drop also observes its earlier
        // raw append. Counts are monotonic observations, not one atomic instant
        // or a claim of disk durability.
        let processing_queue_drops = self.drops.each_ref().map(|n| n.load(Ordering::Acquire));
        ProgressSnapshot {
            journaled_frames: self.recorded.each_ref().map(|n| n.load(Ordering::Acquire)),
            processing_queue_drops,
        }
    }
}
struct Signals {
    stop: Arc<AtomicBool>,
    requested: Arc<AtomicBool>,
    work: Arc<WorkControl>,
    progress: Progress,
}
impl Signals {
    fn stopping(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
            || self.requested.load(Ordering::Relaxed)
            || self.work.check().is_err()
    }
}
/// Cooperative source-side stop predicate, including pipeline failure, owner stop
/// and WorkControl cancellation. A source should poll this inside frame waits.
#[derive(Clone)]
pub struct StopToken(Arc<Signals>);
impl StopToken {
    pub fn stop_requested(&self) -> bool {
        self.0.stopping()
    }
}
fn produce<S: Source, J: Journal>(
    camera: Camera,
    profile: StreamProfile,
    maximum: u64,
    ownership: (&Signals, &mut completion::Participant),
    sender: SyncSender<Event>,
    open: impl FnOnce() -> Result<S>,
    journal: impl FnOnce(CaptureContext) -> Result<J>,
) -> SourceReport {
    let (signals, completion) = ownership;
    let mut report = SourceReport {
        camera_index: camera.index(),
        interface: profile.interface,
        opened: false,
        cleanup_confirmed: false,
        journal_finalized: false,
        journaled_frames: 0,
        queued_frames: 0,
        processing_queue_drops: 0,
        statistics: Value::Null,
        outcome: Ok(()),
    };
    if signals.stopping() {
        return report;
    }
    let mut source = match caught(open) {
        Ok(s) => s,
        Err(e) => {
            report.outcome = Err(e);
            return report;
        }
    };
    report.opened = true;
    let result = caught(|| {
        let mut journal = journal(source.context()?)?;
        for index in 0..maximum {
            if signals.stopping() {
                break;
            }
            let Some(frame) = source.next()? else {
                break;
            };
            super::validate_sample(&profile, &frame)?;
            journal.push(&frame)?;
            report.journaled_frames += 1;
            signals.progress.recorded[camera.index()].fetch_add(1, Ordering::Release);
            match sender.try_send(Event {
                camera,
                sample: Sample { index, frame },
            }) {
                Ok(()) => report.queued_frames += 1,
                Err(TrySendError::Full(_)) => {
                    report.processing_queue_drops += 1;
                    signals.progress.drops[camera.index()].fetch_add(1, Ordering::Release);
                }
                Err(TrySendError::Disconnected(_)) => {
                    report.processing_queue_drops += 1;
                    signals.progress.drops[camera.index()].fetch_add(1, Ordering::Release);
                    break;
                }
            }
        }
        journal.finish()?;
        report.journal_finalized = true;
        Ok(())
    });
    let stats = caught(|| Ok(source.statistics())).map(|s| {
        report.statistics = s;
    });
    if result.is_err() || stats.is_err() {
        signals.stop.store(true, Ordering::Relaxed);
    }
    // A finished source must stay open while its peer may still be acquiring.
    // Cancellation is not proof that a peer's in-flight driver call returned.
    completion.acquisition_finished();
    let cleanup = caught(|| source.close());
    report.cleanup_confirmed = cleanup.is_ok();
    report.outcome = combine(combine(result, stats), cleanup);
    report
}
struct Workers {
    signals: Arc<Signals>,
    handles: Vec<thread::JoinHandle<SourceReport>>,
}
impl Drop for Workers {
    fn drop(&mut self) {
        self.signals.stop.store(true, Ordering::Relaxed);
        for h in self.handles.drain(..) {
            if h.join().is_err() {
                scancraft_core::diagnostics::note(format_args!(
                    "laser source worker panicked during cleanup"
                ));
            }
        }
    }
}
impl Pipeline<'_> {
    /// Stream at most maximum raw frames per camera (u64 indices, no history cap).
    /// Raw journals precede a four-frame nonblocking processing queue; full queues
    /// omit processing only, with explicit per-source counts. Source/device calls,
    /// journals, callbacks and cleanup are synchronous and have no hard deadline.
    ///
    /// Pair-level processing errors are delivered and counted without stopping raw
    /// retention; per-query errors remain in Report. Required consumer errors stop
    /// the run. Optional preview consumers must handle omissions and return Ok.
    /// No fusion, metric Cloud, calibration activation or file publication occurs.
    pub fn run<S: Source + 'static, J: Journal, O, F, D>(
        &self,
        maximum: u64,
        work: &Arc<WorkControl>,
        stop: &Arc<AtomicBool>,
        open: O,
        journal: F,
        mut deliver: D,
    ) -> std::result::Result<Summary, Failure>
    where
        O: Fn(Camera, StreamProfile, StopToken) -> Result<S> + Send + Sync + 'static,
        F: Fn(Camera, StreamProfile, scancraft_recording::CaptureContext) -> Result<J>
            + Send
            + Sync
            + 'static,
        D: FnMut(&Pair, Result<Report>) -> Result<()>,
    {
        self.run_progress(maximum, work, stop, open, journal, |pair, result, _| {
            deliver(pair, result)
        })
    }
    /// Observed atomic source counters at each pair boundary; not a simultaneous
    /// transaction or durable-file attestation. No history is retained.
    pub fn run_progress<S: Source + 'static, J: Journal, O, F, D>(
        &self,
        maximum: u64,
        work: &Arc<WorkControl>,
        stop: &Arc<AtomicBool>,
        open: O,
        journal: F,
        mut deliver: D,
    ) -> std::result::Result<Summary, Failure>
    where
        O: Fn(Camera, StreamProfile, StopToken) -> Result<S> + Send + Sync + 'static,
        F: Fn(Camera, StreamProfile, CaptureContext) -> Result<J> + Send + Sync + 'static,
        D: FnMut(&Pair, Result<Report>, ProgressSnapshot) -> Result<()>,
    {
        let early = |reason| Failure {
            reason,
            partial: None,
        };
        work.check().map_err(early)?;
        let mut association =
            Association::new(self.profiles.clone(), maximum, self.maximum_host_delta_us)
                .map_err(early)?;
        let signals = Arc::new(Signals {
            stop: Arc::new(AtomicBool::new(false)),
            requested: stop.clone(),
            work: work.clone(),
            progress: Progress::default(),
        });
        let mut workers = Workers {
            signals: signals.clone(),
            handles: Vec::new(),
        };
        let (sender, receiver) = mpsc::sync_channel(PROCESSING_QUEUE_CAPACITY);
        let open = Arc::new(open);
        let journal = Arc::new(journal);
        let mut spawn_error = None;
        let slots = completion::participants(signals.stop.clone());
        for (camera, mut slot) in [Camera::First, Camera::Second].into_iter().zip(slots) {
            let signals = signals.clone();
            let sender = sender.clone();
            let open = open.clone();
            let journal = journal.clone();
            let profile = self.profiles[camera.index()].clone();
            match thread::Builder::new()
                .name(format!("scancraft-laser-{}", camera.index()))
                .spawn(move || {
                    let report = produce(
                        camera,
                        profile.clone(),
                        maximum,
                        (&signals, &mut slot),
                        sender,
                        || open(camera, profile.clone(), StopToken(signals.clone())),
                        |context| journal(camera, profile.clone(), context),
                    );
                    if report.outcome.is_err() {
                        signals.stop.store(true, Ordering::Relaxed);
                    }
                    report
                }) {
                Ok(handle) => workers.handles.push(handle),
                Err(e) => {
                    spawn_error = Some(e.to_string());
                    break;
                }
            }
        }
        drop(sender);
        let mut processed = 0;
        let mut failed = 0;
        let processing = caught(|| {
            if let Some(e) = spawn_error {
                return Err(e);
            }
            loop {
                work.check()?;
                if stop.load(Ordering::Relaxed) {
                    signals.stop.store(true, Ordering::Relaxed);
                }
                match receiver.recv_timeout(Duration::from_millis(20)) {
                    Ok(event) => {
                        if let Some(pair) = association.push(event.camera, event.sample)? {
                            let result = self.processor.process(
                                [
                                    FrameInput {
                                        profile: &self.profiles[0],
                                        frame: &pair.samples[0].frame,
                                    },
                                    FrameInput {
                                        profile: &self.profiles[1],
                                        frame: &pair.samples[1].frame,
                                    },
                                ],
                                work,
                                &mut (),
                            );
                            work.check()?;
                            processed += 1;
                            if result.is_err() {
                                failed += 1;
                            }
                            deliver(&pair, result, signals.progress.snapshot())?;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            Ok(())
        });
        signals.stop.store(true, Ordering::Relaxed);
        drop(receiver);
        let mut sources = Vec::new();
        let mut errors = Vec::new();
        if let Err(e) = processing {
            errors.push(e);
        }
        for handle in workers.handles.drain(..) {
            match handle.join() {
                Ok(r) => {
                    if let Err(e) = &r.outcome {
                        errors.push(format!("interface {}: {e}", r.interface));
                    }
                    sources.push(r);
                }
                Err(_) => {
                    errors.push("laser source thread join failed; counts/cleanup unknown".into())
                }
            }
        }
        let mut undelivered = [None; 2];
        for source in &sources {
            undelivered[source.camera_index] = source
                .queued_frames
                .checked_sub(association.counts.delivered[source.camera_index]);
            if undelivered[source.camera_index].is_none() {
                errors.push("laser delivery counters are inconsistent".into());
            }
        }
        let summary = Summary {
            sources,
            undelivered_queued_frames: undelivered,
            association: std::mem::take(&mut association.counts),
            unmatched_pending: association.remaining(),
            processed_pairs: processed,
            failed_processing_pairs: failed,
            graceful_stop_requested: stop.load(Ordering::Relaxed),
            synchronization_verified: false,
            physical_accuracy_qualified: false,
        };
        if errors.is_empty() {
            Ok(summary)
        } else {
            Err(Failure {
                reason: errors.join("; "),
                partial: Some(Box::new(summary)),
            })
        }
    }
}
