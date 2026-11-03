use super::{
    Request,
    io::{CappedFile, pair_record},
};
use crate::{
    Digest,
    laser_stream::pair_journal::{self, Limits},
};
use scancraft_geometry::work::WorkControl;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::Path,
};
pub(super) const OVERHEAD: u64 = 8 * 1024 * 1024;
pub(super) fn limits(r: &Request) -> io::Result<Option<Limits>> {
    let extras = (
        r.pair_log_segment_bytes,
        r.maximum_pair_record_bytes,
        r.maximum_bundle_bytes,
    );
    if r.schema_version == 1 {
        if extras != (None, None, None) || !(4096..=1 << 30).contains(&r.maximum_pair_log_bytes) {
            return Err(io::Error::other("invalid legacy pair-log policy"));
        }
        return Ok(None);
    }
    if r.schema_version != 2 {
        return Err(io::Error::other("unknown laser capture schema"));
    }
    let (Some(segment_bytes), Some(maximum_record_bytes), Some(bundle)) = extras else {
        return Err(io::Error::other(
            "schema2 requires explicit segment, record and bundle quotas",
        ));
    };
    let limits = Limits {
        segment_bytes,
        maximum_record_bytes,
        maximum_total_bytes: r.maximum_pair_log_bytes,
    };
    limits.validate()?;
    let minimum = r
        .maximum_recording_bytes_per_camera
        .checked_mul(2)
        .and_then(|n| n.checked_add(r.maximum_pair_log_bytes))
        .and_then(|n| n.checked_add(OVERHEAD))
        .ok_or_else(|| io::Error::other("bundle quota overflow"))?;
    if bundle < minimum || bundle > 3 * (1u64 << 40) {
        return Err(io::Error::other(
            "bundle quota cannot admit declared raw/pair/metadata quotas",
        ));
    }
    Ok(Some(limits))
}
pub(super) fn admit(r: &Request, parent: &Path) -> io::Result<()> {
    if limits(r)?.is_none() {
        return Ok(());
    }
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent)?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: fd is live; stats is correctly sized writable statvfs storage.
    let result = unsafe { libc::fstatvfs(directory.as_raw_fd(), stats.as_mut_ptr()) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful fstatvfs initialized the complete structure.
    let stats = unsafe { stats.assume_init() };
    let available = u128::from(stats.f_bavail) * u128::from(stats.f_frsize);
    let requested = u128::from(r.maximum_bundle_bytes.unwrap()) + 64 * 1024 * 1024;
    if available < requested {
        return Err(io::Error::other(
            "insufficient available filesystem bytes for bundle quota plus headroom",
        ));
    }
    Ok(())
}
pub(super) enum Log {
    Single(CappedFile),
    Segments(pair_journal::Journal),
}
pub(super) struct Closed {
    pub artifacts: Vec<Digest>,
    pub details: Option<Value>,
}
impl Log {
    pub(super) fn open(root: &Path, r: &Request) -> io::Result<Self> {
        Ok(match limits(r)? {
            None => Self::Single(CappedFile::new(
                root,
                "pairs.ndjson",
                r.maximum_pair_log_bytes,
            )?),
            Some(limits) => Self::Segments(pair_journal::Journal::new(root, limits)?),
        })
    }
    pub(super) fn push(&mut self, record: &impl Serialize, work: &WorkControl) -> io::Result<()> {
        match self {
            Self::Single(out) => pair_record(out, record),
            Self::Segments(out) => out.push(record, work),
        }
    }
    pub(super) fn finish(self) -> io::Result<Closed> {
        match self {
            Self::Single(out) => Ok(Closed {
                artifacts: vec![out.finish()?],
                details: None,
            }),
            Self::Segments(out) => {
                let done = out.finish()?;
                Ok(Closed {
                    details: Some(
                        json!({"format":"segmented_ndjson_v1","index":"pairs.index.ndjson","segments":done.segments,"pairs":done.pairs,"bytes":done.bytes}),
                    ),
                    artifacts: done.artifacts,
                })
            }
        }
    }
}
