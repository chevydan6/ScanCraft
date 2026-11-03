//! Exact-sequence metadata matching and dependent work before video-slot reuse.
//! No clocks are synthesized; missing/invalid metadata remains a capture failure.
use super::Packet;
use std::io;

pub(super) enum Metadata {
    Matched(Packet),
    Missing,
    Stopped,
}

/// Retains the existing 32-packet search bound and wrapping-sequence policy.
pub(super) fn matching_metadata(
    sequence: u32,
    pending: &mut Option<Packet>,
    mut read: impl FnMut() -> io::Result<Option<Packet>>,
) -> io::Result<Metadata> {
    for _ in 0..32 {
        let packet = if let Some(packet) = pending.take() {
            packet
        } else {
            let Some(packet) = read()? else {
                return Ok(Metadata::Stopped);
            };
            packet
        };
        if packet.sequence == sequence {
            return Ok(Metadata::Matched(packet));
        }
        if packet.sequence.wrapping_sub(sequence) < 0x8000_0000 {
            *pending = Some(packet);
            return Ok(Metadata::Missing);
        }
    }
    Ok(Metadata::Missing)
}

/// Always attempts requeue after ordinary copy/dependent-operation failures.
/// On panic the owning Stream still handles STREAMOFF/restoration during unwind.
/// Neither dependent work nor driver cleanup has a hard wall-clock bound.
pub(super) fn complete_then_requeue<T, U>(
    packet: io::Result<T>,
    complete: impl FnOnce(T) -> io::Result<U>,
    requeue: impl FnOnce() -> io::Result<()>,
) -> io::Result<U> {
    let result = packet.and_then(complete);
    match (result, requeue()) {
        (result, Ok(())) => result,
        // A failed requeue must not be mistaken for a cooperative cancelled wait,
        // even when the original error kind is Interrupted or TimedOut.
        (Ok(_), Err(error)) => Err(io::Error::other(format!(
            "requeue buffer failed ({:?}): {error}",
            error.kind()
        ))),
        (Err(primary), Err(cleanup)) => Err(io::Error::other(format!(
            "{primary}; requeue buffer failed ({:?}): {cleanup}",
            cleanup.kind()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn packet(sequence: u32) -> Packet {
        Packet {
            sequence,
            timestamp: 1,
            flags: 0,
            data: vec![sequence as u8],
        }
    }

    #[test]
    fn matching_metadata_is_returned_before_video_slot_reuse() {
        let events = RefCell::new(Vec::new());
        let result = complete_then_requeue(
            Ok(packet(17)),
            |video| {
                let metadata = matching_metadata(video.sequence, &mut None, || {
                    events.borrow_mut().push("metadata dequeued and requeued");
                    Ok(Some(packet(17)))
                })?;
                Ok((video, metadata))
            },
            || {
                events.borrow_mut().push("video requeued");
                Ok(())
            },
        )
        .unwrap();
        assert!(matches!(result.1, Metadata::Matched(p) if p.sequence == result.0.sequence));
        assert_eq!(
            *events.borrow(),
            ["metadata dequeued and requeued", "video requeued"]
        );
    }

    #[test]
    fn older_metadata_is_skipped_and_future_metadata_retained_across_wrap() {
        let mut pending = Some(packet(u32::MAX));
        let mut reads = 0;
        assert!(matches!(
            matching_metadata(0, &mut pending, || {
                reads += 1;
                Ok(Some(packet(1)))
            })
            .unwrap(),
            Metadata::Missing
        ));
        assert_eq!(reads, 1);
        assert!(
            matches!(matching_metadata(1, &mut pending, || panic!("pending packet lost")).unwrap(), Metadata::Matched(p) if p.sequence == 1 && p.data == [1])
        );
        assert!(pending.is_none());
    }

    #[test]
    fn metadata_search_remains_bounded_and_stop_is_distinct_from_missing() {
        let mut reads = 0;
        assert!(matches!(
            matching_metadata(100, &mut None, || {
                reads += 1;
                Ok(Some(packet(0)))
            })
            .unwrap(),
            Metadata::Missing
        ));
        assert_eq!(reads, 32);
        assert!(matches!(
            matching_metadata(100, &mut None, || Ok(None)).unwrap(),
            Metadata::Stopped
        ));
        assert_eq!(
            matching_metadata(100, &mut None, || Err(io::ErrorKind::NotConnected.into()))
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::NotConnected
        );
    }

    #[test]
    fn copy_failure_skips_dependent_work_but_still_returns_slot() {
        let returned = Cell::new(false);
        let error = complete_then_requeue::<(), ()>(
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "bad copied length",
            )),
            |_| panic!("dependent operation after failed copy"),
            || {
                returned.set(true);
                Ok(())
            },
        )
        .unwrap_err();
        assert!(returned.get());
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn dependent_failure_and_requeue_failure_are_both_retained() {
        let error = complete_then_requeue::<(), ()>(
            Ok(()),
            |_| {
                Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "metadata disconnected",
                ))
            },
            || {
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "video requeue interrupted",
                ))
            },
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(error.to_string().contains("metadata disconnected"));
        assert!(error.to_string().contains("video requeue interrupted"));
        let error =
            complete_then_requeue(Ok(()), |_| Ok(()), || Err(io::ErrorKind::TimedOut.into()))
                .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
    }
}
