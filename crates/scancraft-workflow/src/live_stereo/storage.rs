//! Opt-in long-session resource admission. Filesystem space is a snapshot, not a reservation.
use super::Config;
use crate::laser_stream::pair_journal::Limits;
use scancraft_core::StreamProfile;
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::Path,
};

pub const MAXIMUM_FRAMES: usize = 100_000;
pub const OVERHEAD_BYTES: u64 = 512 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Storage {
    pub maximum_recording_bytes_per_camera: u64,
    pub pair_journal: Limits,
    pub maximum_bundle_bytes: u64,
}
impl Config {
    pub(super) fn validate_capacity(&self) -> Result<(), String> {
        match (self.schema_version, &self.storage) {
            (1, None) if (1..=1000).contains(&self.maximum_frames_per_camera) => Ok(()),
            (2, Some(storage)) if (1..=MAXIMUM_FRAMES).contains(&self.maximum_frames_per_camera) => {
                storage.pair_journal.validate().map_err(|e| e.to_string())?;
                let minimum = storage.maximum_recording_bytes_per_camera.checked_mul(2)
                    .and_then(|n| n.checked_add(storage.pair_journal.maximum_total_bytes))
                    .and_then(|n| n.checked_add(OVERHEAD_BYTES)).ok_or("live bundle quota overflow")?;
                if storage.maximum_recording_bytes_per_camera > 1 << 40
                    || storage.maximum_bundle_bytes < minimum || storage.maximum_bundle_bytes > 3 * (1u64 << 40) {
                    return Err("live bundle quota cannot admit raw, journal and metadata quotas".into());
                }
                Ok(())
            },
            _ => Err("live capacity requires schema1 <=1000 without storage, or schema2 <=100000 with explicit storage quotas".into()),
        }
    }
    pub(super) fn validate_raw_budget(&self, profiles: &[StreamProfile; 2]) -> Result<(), String> {
        let Some(storage) = &self.storage else {
            return Ok(());
        };
        for profile in profiles {
            let required = (u64::from(profile.width) * u64::from(profile.height) + 4096 + 32)
                .checked_mul(self.maximum_frames_per_camera as u64)
                .and_then(|n| n.checked_add(1024 * 1024 + 32))
                .ok_or("live raw quota overflow")?;
            if required > storage.maximum_recording_bytes_per_camera {
                return Err(
                    "live raw quota cannot admit frame count and maximum metadata/header/footer"
                        .into(),
                );
            }
        }
        Ok(())
    }
}
pub(super) fn admit(storage: Option<&Storage>, target: &Path) -> Result<(), String> {
    let Some(storage) = storage else {
        return Ok(());
    };
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent)
        .map_err(|e| e.to_string())?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: the directory fd is live and stats is correctly sized writable storage.
    if unsafe { libc::fstatvfs(directory.as_raw_fd(), stats.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // SAFETY: successful fstatvfs initialized stats.
    let stats = unsafe { stats.assume_init() };
    if u128::from(stats.f_bavail) * u128::from(stats.f_frsize)
        < u128::from(storage.maximum_bundle_bytes) + 64 * 1024 * 1024
    {
        return Err(
            "insufficient available filesystem bytes for live bundle quota plus headroom".into(),
        );
    }
    Ok(())
}
