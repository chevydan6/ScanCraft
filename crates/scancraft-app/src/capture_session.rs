//! Owns one preview/recording session independently of GUI channels.
//! Device I/O is still synchronous; process isolation is a separate lifecycle step.
use scancraft_core::{Frame, StreamProfile};
use scancraft_device::{Capture, SessionLock};
use scancraft_recording::Writer;
use std::{
    fs::OpenOptions,
    io::BufWriter,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

/// Run one session. Delivery returns false when its consumer has disconnected;
/// display backpressure is handled by the caller and never skips recording.
pub(crate) fn run(
    interface: u8,
    record: bool,
    folder: &Path,
    cancelled: impl Fn() -> bool,
    mut deliver: impl FnMut(Frame) -> bool,
) -> Result<String, String> {
    if cancelled() {
        return Ok("Cancelled before opening the scanner".into());
    }
    let _locks = SessionLock::acquire().map_err(|e| e.to_string())?;
    if cancelled() {
        return Ok("Cancelled before opening the scanner".into());
    }
    let profile = StreamProfile {
        interface,
        width: if interface == 4 { 1280 } else { 1920 },
        height: if interface == 4 { 800 } else { 1200 },
        fourcc: if interface == 4 { *b"MJPG" } else { *b"GREY" },
        fps: if interface == 4 { 10 } else { 50 },
    };
    let capture = if record {
        Capture::open_recording(profile.clone())
    } else {
        Capture::open(profile.clone())
    }
    .map_err(|e| e.to_string())?;
    let (output, partial, recording, rejected, missing, gaps) = with_close(
        capture,
        |capture| {
            let output = if record {
                let folder = folder.to_path_buf();
                std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
                let stamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_nanos();
                Some(folder.join(format!("capture-{stamp}.raptor")))
            } else {
                None
            };
            let partial = output.as_ref().map(|p| p.with_extension("raptor.partial"));
            let mut recording = if let Some(path) = &partial {
                Some(
                    Writer::with_context(
                        BufWriter::new(
                            OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .open(path)
                                .map_err(|e| e.to_string())?,
                        ),
                        profile,
                        {
                            let mut context = scancraft_recording::CaptureContext::default();
                            if let Some(observed) = capture.recording_observations() {
                                context
                                    .add_capture_observations(observed)
                                    .map_err(|e| e.to_string())?;
                            }
                            context
                        },
                    )
                    .map_err(|e| e.to_string())?,
                )
            } else {
                None
            };
            while !cancelled() {
                let frame = capture.next_frame().map_err(|e| e.to_string())?;
                if let Some(w) = &mut recording {
                    w.push(&frame).map_err(|e| e.to_string())?;
                }
                if !deliver(frame) {
                    break;
                }
            }
            Ok((
                output,
                partial,
                recording,
                capture.rejected,
                capture.metadata_missing,
                capture.sequence_drops,
            ))
        },
        |capture| capture.close().map_err(|e| e.to_string()),
    )?;
    if let Some(w) = recording {
        let file = w.finish().map_err(|e| e.to_string())?;
        file.get_ref().sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::hard_link(partial.as_ref().unwrap(), output.as_ref().unwrap())
            .map_err(|e| e.to_string())?;
        std::fs::remove_file(partial.as_ref().unwrap()).map_err(|e| e.to_string())?;
    }
    Ok(format!(
        "Stopped cleanly. Rejected buffers={rejected}, missing metadata={missing}, sequence gaps={gaps}. Recording: {}",
        output.map_or_else(|| "off".into(), |p| p.display().to_string())
    ))
}

/// Preserve the operation failure and explicit device-restoration evidence.
/// Successful recording publication is permitted only after successful close.
fn with_close<S, T>(
    mut source: S,
    operation: impl FnOnce(&mut S) -> Result<T, String>,
    close: impl FnOnce(S) -> Result<(), String>,
) -> Result<T, String> {
    let operation = operation(&mut source);
    // This is explicit even on operation failure. Panics retain source Drop's
    // best-effort cleanup and are handled by the existing worker lifecycle.
    let close = close(source);
    match (operation, close) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(format!("Capture cleanup failed: {error}")),
        (Err(error), Err(cleanup)) => {
            Err(format!("{error}; capture cleanup also failed: {cleanup}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquisition_or_write_error_still_closes_and_retains_restoration_failure() {
        let closed = std::cell::Cell::new(false);
        let result = with_close(
            (),
            |_| Err::<(), _>("recording write failed".into()),
            |_| {
                closed.set(true);
                Err("stream restoration failed".into())
            },
        );
        assert!(closed.get());
        let error = result.unwrap_err();
        assert!(error.contains("recording write failed"));
        assert!(error.contains("stream restoration failed"));
    }

    #[test]
    fn cleanup_failure_never_returns_successful_publication_state() {
        let events = std::cell::RefCell::new(Vec::new());
        let result = with_close(
            (),
            |_| {
                events.borrow_mut().push("operation");
                Ok(42)
            },
            |_| {
                events.borrow_mut().push("close");
                Err("streamoff failed".into())
            },
        );
        assert_eq!(*events.borrow(), ["operation", "close"]);
        assert!(result.unwrap_err().contains("streamoff failed"));
        assert_eq!(with_close((), |_| Ok(42), |_| Ok(())), Ok(42));
        assert_eq!(
            with_close((), |_| Err::<(), _>("capture failed".into()), |_| Ok(())),
            Err("capture failed".into())
        );
    }
}
