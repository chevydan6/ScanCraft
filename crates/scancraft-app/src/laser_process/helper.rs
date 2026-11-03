use super::{Completion, replay};
use crate::{
    capture_ipc,
    laser_display::{Origin, Pending, Snapshot},
};
use scancraft_geometry::work::WorkControl;
use scancraft_workflow::laser_stream::bundle::{PreviewInput, PreviewStatus};
use std::{
    io::{self, Read, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
fn controls(mut input: impl Read, work: &WorkControl, stop: &AtomicBool) {
    let mut bytes = [0; 32];
    loop {
        match input.read(&mut bytes) {
            Ok(0) => break,
            Ok(n) => {
                if bytes[..n].iter().any(|&v| v != 1) {
                    break;
                }
                stop.store(true, Ordering::Relaxed);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    work.cancel();
    stop.store(true, Ordering::Relaxed);
}
// Optional display preparation failures are omissions, not acquisition failures.
// Only an actual pipe write/flush failure asks the native owner to clean up.
fn deliver(
    mut snapshot: Snapshot,
    out: &mut impl Write,
    work: &WorkControl,
    stop: &AtomicBool,
    omitted: &AtomicU64,
) -> Result<(), String> {
    snapshot.helper_omissions = omitted.load(Ordering::Relaxed);
    let packet = match capture_ipc::prepare_laser_progress(&snapshot) {
        Ok(packet) => packet,
        Err(e) => {
            omitted.fetch_add(1, Ordering::Relaxed);
            crate::helper_diagnostics::note(format_args!(
                "Laser preview omitted; raw recording continues: {e}"
            ));
            return Ok(());
        }
    };
    if let Err(e) = out.write_all(&packet).and_then(|_| out.flush()) {
        work.cancel();
        stop.store(true, Ordering::Relaxed);
        return Err(e.to_string());
    }
    Ok(())
}
fn offer(
    input: PreviewInput<'_>,
    origin: Origin,
    work: &WorkControl,
    mailbox: &Mutex<Option<Pending>>,
    omitted: &AtomicU64,
) -> PreviewStatus {
    match Pending::capture(input, origin, work) {
        Ok(pending) => match mailbox.lock() {
            Ok(mut slot) => {
                if slot.replace(pending).is_some() {
                    omitted.fetch_add(1, Ordering::Relaxed);
                }
                PreviewStatus::Accepted
            }
            Err(_) => {
                omitted.fetch_add(1, Ordering::Relaxed);
                PreviewStatus::Omitted
            }
        },
        Err(e) => {
            omitted.fetch_add(1, Ordering::Relaxed);
            crate::helper_diagnostics::note(format_args!(
                "Laser preview handoff omitted; raw recording continues: {e}"
            ));
            PreviewStatus::Omitted
        }
    }
}
fn publish(
    mailbox: &Mutex<Option<Pending>>,
    done: &AtomicBool,
    omitted: &AtomicU64,
    work: &WorkControl,
    mut send: impl FnMut(Snapshot) -> Result<(), String>,
) -> Result<(), String> {
    loop {
        let finished = done.load(Ordering::Acquire);
        let pending = mailbox
            .lock()
            .map_err(|_| "laser helper mailbox poisoned")?
            .take();
        // Expensive work and output I/O never hold the mailbox mutex.
        if let Some(pending) = pending {
            match pending.finish(work) {
                Ok(snapshot) => send(snapshot)?,
                Err(e) => {
                    omitted.fetch_add(1, Ordering::Relaxed);
                    crate::helper_diagnostics::note(format_args!(
                        "Laser preview omitted during worker preparation; raw recording continues: {e}"
                    ));
                }
            }
        }
        // Acquire done before taking the slot: every completed producer offer
        // precedes it, so final work cannot be missed when stopping the publisher.
        if finished {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}
fn session(paths: &[PathBuf], origin: Origin) -> Result<Completion, String> {
    let work = Arc::new(WorkControl::default());
    let stop = Arc::new(AtomicBool::new(false));
    let w = work.clone();
    let s = stop.clone();
    std::thread::Builder::new()
        .name("scancraft-laser-control".into())
        .spawn(move || controls(io::stdin(), &w, &s))
        .map_err(|e| e.to_string())?;
    let mailbox: Arc<Mutex<Option<Pending>>> = Arc::new(Mutex::new(None));
    let omitted = Arc::new(AtomicU64::new(0));
    let done = Arc::new(AtomicBool::new(false));
    let m = mailbox.clone();
    let o = omitted.clone();
    let d = done.clone();
    let w = work.clone();
    let s = stop.clone();
    let publisher = std::thread::Builder::new()
        .name("scancraft-laser-display".into())
        .spawn(move || -> Result<(), String> {
            publish(&m, &d, &o, &w, |snapshot| {
                deliver(snapshot, &mut io::stdout().lock(), &w, &s, &o)
            })
        })
        .map_err(|e| e.to_string())?;
    let preview = |input: PreviewInput<'_>| offer(input, origin, &work, &mailbox, &omitted);
    let outcome = if origin == Origin::Native {
        scancraft_acquisition::laser::scan(&paths[0], &paths[1], &paths[2], &work, &stop, preview)
    } else {
        replay::run(paths, &work, &stop, preview)
    };
    done.store(true, Ordering::Release);
    let transport = publisher
        .join()
        .map_err(|_| "laser display publisher panicked")?;
    match (outcome, transport) {
        (Ok(report), Ok(())) => {
            Completion::from_report(&report, origin, omitted.load(Ordering::Relaxed))
        }
        (Err(e), _) => Err(e),
        (Ok(_), Err(e)) => Err(format!(
            "Capture was saved, but preview transport failed: {e}; inspect target report.json"
        )),
    }
}
pub(super) fn entry(args: &[std::ffi::OsString]) -> Option<i32> {
    let origin = match args.first().and_then(|v| v.to_str()) {
        Some("--laser-helper-native") => Origin::Native,
        Some("--laser-helper-replay") => Origin::RecordingReplay,
        _ => return None,
    };
    let result = if args.len() != if origin == Origin::Native { 4 } else { 6 } {
        Err("invalid laser helper arguments".into())
    } else {
        crate::helper_diagnostics::note(format_args!(
            "laser helper PID {} started ({origin:?})",
            std::process::id()
        ));
        let paths = args[1..].iter().map(PathBuf::from).collect::<Vec<_>>();
        session(&paths, origin)
    };
    if let Err(e) = &result {
        crate::helper_diagnostics::note(format_args!("Laser helper failed: {e}"));
    }
    let sent = capture_ipc::write_laser_complete(&mut io::stdout().lock(), &result);
    if let Err(e) = &sent {
        crate::helper_diagnostics::note(format_args!("Laser completion transport failed: {e}"));
    }
    Some(if result.is_ok() && sent.is_ok() { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_omission_is_independent_of_raw_work_but_broken_pipe_requests_cleanup() {
        let work = WorkControl::default();
        let stop = AtomicBool::new(false);
        let omitted = AtomicU64::new(0);
        let mut bad = crate::laser_display::tests::snapshot(Origin::Native, 0);
        bad.frames[0].pixels.clear();
        let mut out = Vec::new();
        deliver(bad, &mut out, &work, &stop, &omitted).unwrap();
        assert!(out.is_empty());
        assert!(work.check().is_ok());
        assert!(!stop.load(Ordering::Relaxed));
        assert_eq!(omitted.load(Ordering::Relaxed), 1);
        let valid = crate::laser_display::tests::snapshot(Origin::Native, 1);
        deliver(valid, &mut out, &work, &stop, &omitted).unwrap();
        let mut decoder = capture_ipc::Decoder::default();
        decoder.push(&out).unwrap();
        assert!(
            matches!(decoder.next().unwrap(),Some(capture_ipc::Message::LaserProgress(s)) if s.helper_omissions==1)
        );
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        assert!(
            deliver(
                crate::laser_display::tests::snapshot(Origin::Native, 2),
                &mut Broken,
                &work,
                &stop,
                &omitted
            )
            .is_err()
        );
        assert!(work.check().is_err());
        assert!(stop.load(Ordering::Relaxed));
        assert_eq!(omitted.load(Ordering::Relaxed), 1);
    }
    #[test]
    fn stop_byte_is_graceful_until_eof_and_unknown_controls_abort() {
        let (read, mut write) = std::os::unix::net::UnixStream::pair().unwrap();
        let work = Arc::new(WorkControl::default());
        let stop = Arc::new(AtomicBool::new(false));
        let (w, s) = (work.clone(), stop.clone());
        let h = std::thread::spawn(move || controls(read, &w, &s));
        write.write_all(&[1]).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !stop.load(Ordering::Relaxed) {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(work.check().is_ok());
        drop(write);
        h.join().unwrap();
        assert!(work.check().is_err());
        let work = WorkControl::default();
        let stop = AtomicBool::new(false);
        controls(&[2u8][..], &work, &stop);
        assert!(work.check().is_err());
        assert!(stop.load(Ordering::Relaxed));
    }
}

#[cfg(test)]
mod worker_tests;
