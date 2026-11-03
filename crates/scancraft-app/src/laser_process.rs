//! Laser helper supervision. Device and restoration ownership stays in the child.
mod helper;
mod replay;
use crate::{
    capture_ipc::{Decoder, Message},
    capture_process::{OwnedChild, nonblocking, private_log},
    laser_display::{Origin, Snapshot},
};
use scancraft_geometry::work::WorkControl;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub schema_version: u32,
    pub origin: Origin,
    pub recorded: [u64; 2],
    pub drops: [u64; 2],
    pub processed: u64,
    pub failed: u64,
    pub pending: [usize; 2],
    pub helper_omissions: u64,
}
impl Completion {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.failed > self.processed
            || self.recorded.iter().zip(self.drops).any(|(&n, d)| d > n)
            || self.processed > self.recorded[0].min(self.recorded[1])
            || self
                .pending
                .iter()
                .zip(self.recorded)
                .any(|(&n, raw)| n > 2 || n as u64 > raw)
        {
            Err("invalid laser completion counters".into())
        } else {
            Ok(())
        }
    }
    fn from_report(report: &Value, origin: Origin, omissions: u64) -> Result<Self, String> {
        if report["kind"] != "laser_candidate_stream_diagnostics"
            || report["raw_replay_integrity"] != "PASS"
            || report["reported_source_restoration"] != "PASS"
        {
            return Err("laser helper returned an incomplete capture report".into());
        }
        let s = &report["summary"];
        let mut recorded = [0; 2];
        let mut drops = [0; 2];
        let mut seen = [false; 2];
        for source in s["sources"].as_array().ok_or("missing source counters")? {
            let i = source["camera_index"]
                .as_u64()
                .filter(|&n| n < 2)
                .ok_or("invalid camera index")? as usize;
            if seen[i] {
                return Err("duplicate source counters".into());
            }
            seen[i] = true;
            recorded[i] = source["journaled_frames"]
                .as_u64()
                .ok_or("missing raw count")?;
            drops[i] = source["processing_queue_drops"]
                .as_u64()
                .ok_or("missing drop count")?;
        }
        if !seen.iter().all(|v| *v) {
            return Err("missing source counters".into());
        }
        let value = Self {
            schema_version: 1,
            origin,
            recorded,
            drops,
            processed: s["processed_pairs"].as_u64().ok_or("missing pair count")?,
            failed: s["failed_processing_pairs"]
                .as_u64()
                .ok_or("missing failure count")?,
            pending: serde_json::from_value(s["unmatched_pending"].clone())
                .map_err(|e| e.to_string())?,
            helper_omissions: omissions,
        };
        value.validate()?;
        Ok(value)
    }
}
#[derive(Default)]
struct Mailbox {
    latest: Option<Snapshot>,
    last: Option<[u64; 2]>,
}
pub struct Control {
    pub stop: AtomicBool,
    pub teardown: AtomicBool,
    pub work: Arc<WorkControl>,
    origin: Origin,
    mailbox: Mutex<Mailbox>,
    pub parent_omissions: AtomicU64,
    status: Mutex<String>,
}
impl Control {
    pub fn new(origin: Origin) -> Self {
        Self {
            stop: AtomicBool::new(false),
            teardown: AtomicBool::new(false),
            work: Arc::new(WorkControl::default()),
            origin,
            mailbox: Mutex::new(Mailbox::default()),
            parent_omissions: AtomicU64::new(0),
            status: Mutex::new("Starting helper…".into()),
        }
    }
    pub fn take(&self) -> Option<Snapshot> {
        self.mailbox.lock().ok()?.latest.take()
    }
    pub fn status(&self) -> String {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| "Helper status unavailable".into())
    }
    fn set_status(&self, message: String) {
        if let Ok(mut status) = self.status.lock() {
            *status = message;
        }
    }
    pub(crate) fn offer(&self, snapshot: Snapshot) -> Result<(), String> {
        snapshot.validate()?;
        if snapshot.origin != self.origin {
            return Err("unexpected laser preview source kind".into());
        }
        let mut m = self
            .mailbox
            .lock()
            .map_err(|_| "laser preview mailbox poisoned")?;
        if m.last
            .is_some_and(|old| snapshot.indices[0] <= old[0] || snapshot.indices[1] <= old[1])
        {
            return Err("laser preview indices regressed".into());
        }
        m.last = Some(snapshot.indices);
        if m.latest.replace(snapshot).is_some() {
            self.parent_omissions.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
}
pub fn run(
    paths: &[PathBuf],
    origin: Origin,
    control: &Arc<Control>,
) -> Result<Completion, String> {
    control.work.check()?;
    let (log_path, log) = private_log().map_err(|e| e.to_string())?;
    let observation = log.try_clone().map_err(|e| e.to_string())?;
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command
        .arg(if origin == Origin::Native {
            "--laser-helper-native"
        } else {
            "--laser-helper-replay"
        })
        .args(paths)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log));
    let mut child = OwnedChild::spawn(&mut command).map_err(|e| e.to_string())?;
    let pid = child.child().id();
    control.set_status(format!(
        "Helper PID {pid} owns acquisition; diagnostics: {}",
        log_path.display()
    ));
    monitor(&mut child, control, Duration::from_secs(2)).map_err(|e| {
        crate::helper_diagnostics::failure(
            child.child(),
            "laser helper",
            &log_path,
            &observation,
            &e,
        )
    })
}
fn monitor(
    child: &mut OwnedChild,
    control: &Control,
    grace: Duration,
) -> Result<Completion, String> {
    let result = supervise(child, control, grace);
    // Failed IPC cannot release the GUI guard while the helper may own settings.
    // Teardown alone may abandon waiting; OwnedChild retains/reaps the process.
    if result.is_err() {
        child.child().stdin.take();
        while !control.teardown.load(Ordering::Relaxed) {
            if matches!(child.child().try_wait(), Ok(Some(_))) {
                break;
            }
            control.set_status(format!(
                "Cleanup pending in helper PID {}; scanner ownership retained.",
                child.child().id()
            ));
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    result
}
fn supervise(child: &mut OwnedChild, c: &Control, grace: Duration) -> Result<Completion, String> {
    let mut output = child
        .child()
        .stdout
        .take()
        .ok_or("laser helper stdout missing")?;
    nonblocking(&output).map_err(|e| e.to_string())?;
    nonblocking(
        child
            .child()
            .stdin
            .as_ref()
            .ok_or("laser helper stdin missing")?,
    )
    .map_err(|e| e.to_string())?;
    let mut decoder = Decoder::default();
    let mut complete = None;
    let mut eof = false;
    let mut sent_stop = false;
    let mut abort = None;
    let mut bytes = [0; 65536];
    loop {
        if (c.work.check().is_err() || c.teardown.load(Ordering::Relaxed)) && abort.is_none() {
            child.child().stdin.take();
            abort = Some(Instant::now());
        }
        if abort.is_none() && !sent_stop && c.stop.load(Ordering::Relaxed) {
            match child
                .child()
                .stdin
                .as_mut()
                .ok_or("laser control pipe closed")?
                .write(&[1])
            {
                Ok(1) => sent_stop = true,
                Ok(_) => return Err("laser stop pipe made no progress".into()),
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        for _ in 0..64 {
            match output.read(&mut bytes) {
                Ok(0) => {
                    eof = true;
                    break;
                }
                Ok(n) => decoder.push(&bytes[..n]).map_err(|e| e.to_string())?,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.to_string()),
            }
            while let Some(message) = decoder.next().map_err(|e| e.to_string())? {
                match message {
                    Message::LaserProgress(s) => c.offer(*s)?,
                    Message::LaserComplete(r) => complete = Some(r),
                    _ => return Err("unexpected packet in laser helper stream".into()),
                }
            }
        }
        if let Some(status) = child.child().try_wait().map_err(|e| e.to_string())?
            && eof
        {
            decoder
                .finish()
                .map_err(|e| format!("Laser helper exited {status}; IPC: {e}"))?;
            if !status.success() {
                return Err(format!(
                    "Laser helper exited {status}; result: {complete:?}"
                ));
            }
            let result = complete.ok_or("laser helper omitted completion")?;
            if let Ok(v) = &result
                && v.origin != c.origin
            {
                return Err("unexpected laser completion source".into());
            }
            return result;
        }
        if abort.is_some_and(|s| s.elapsed() >= grace) {
            c.set_status(
                "Helper cleanup is still pending; scanner ownership is retained until helper exit."
                    .into(),
            );
            if c.teardown.load(Ordering::Relaxed) {
                return Err("Laser helper is still running; cleanup/restoration is NOT confirmed and its locks remain owned until it exits".into());
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
pub(crate) fn helper_entry(args: &[std::ffi::OsString]) -> Option<i32> {
    helper::entry(args)
}

#[cfg(test)]
mod tests;
