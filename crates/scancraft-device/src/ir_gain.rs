//! Temporary standard IR gain trials only; no exposure, projector or vendor writes.
use crate::{SessionLock, discover, invalid, ioctl, open};
use serde::Serialize;
use std::{fs::File, io, mem::zeroed};
use v4l2_sys_mit as v;
#[derive(Debug, Clone, Serialize)]
pub struct GainChange {
    pub interface: u8,
    pub original_raw: i32,
    pub requested_raw: i32,
    pub restored: bool,
}
pub struct ScopedIrGain<'lock> {
    _lock: &'lock SessionLock,
    fds: Vec<File>,
    changes: Vec<GainChange>,
}
fn restore_all(
    changes: &mut [GainChange],
    mut write: impl FnMut(usize, i32) -> io::Result<()>,
) -> io::Result<()> {
    let mut errors = Vec::new();
    for (i, c) in changes.iter_mut().enumerate().rev() {
        if !c.restored {
            match write(i, c.original_raw) {
                Ok(()) => c.restored = true,
                Err(e) => errors.push(format!(
                    "interface {} original gain {}: {e}",
                    c.interface, c.original_raw
                )),
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(invalid(format!(
            "gain restoration failed: {}",
            errors.join("; ")
        )))
    }
}
fn apply_all(
    changes: &mut [GainChange],
    mut write: impl FnMut(usize, i32) -> io::Result<()>,
) -> io::Result<()> {
    for i in 0..changes.len() {
        changes[i].restored = false;
        if let Err(e) = write(i, changes[i].requested_raw) {
            let restoration = restore_all(changes, &mut write);
            return Err(invalid(format!(
                "gain apply failed: {e}; restoration: {restoration:?}"
            )));
        }
    }
    Ok(())
}
fn gain_fd(interface: u8) -> io::Result<File> {
    if ![0, 2].contains(&interface) {
        return Err(invalid("IR gain supports only interfaces 0 and 2"));
    }
    let nodes = discover()?;
    let nodes: Vec<_> = nodes
        .iter()
        .filter(|n| !n.metadata && n.interface == interface)
        .collect();
    if nodes.len() != 1 {
        return Err(invalid(
            "gain trial requires exactly one Raptor IR node per interface",
        ));
    }
    open(&nodes[0].path)
}
fn get(fd: &File) -> io::Result<i32> {
    let mut c = v::v4l2_control {
        id: v::V4L2_CID_GAIN,
        value: 0,
    };
    ioctl(fd, 27, 3, &mut c)?;
    Ok(c.value)
}
fn set(fd: &File, value: i32) -> io::Result<()> {
    let mut c = v::v4l2_control {
        id: v::V4L2_CID_GAIN,
        value,
    };
    ioctl(fd, 28, 3, &mut c)?;
    if get(fd)? != value {
        return Err(invalid("IR gain readback differs from requested value"));
    }
    Ok(())
}
fn validate(q: &v::v4l2_queryctrl, value: i32) -> io::Result<()> {
    let forbidden = v::V4L2_CTRL_FLAG_DISABLED
        | v::V4L2_CTRL_FLAG_READ_ONLY
        | v::V4L2_CTRL_FLAG_INACTIVE
        | v::V4L2_CTRL_FLAG_GRABBED
        | v::V4L2_CTRL_FLAG_WRITE_ONLY;
    if q.type_ != v::v4l2_ctrl_type_V4L2_CTRL_TYPE_INTEGER
        || q.flags & forbidden != 0
        || q.step <= 0
        || q.minimum > q.maximum
        || !(1000..=15000).contains(&value)
        || value < q.minimum
        || value > q.maximum
        || (i64::from(value) - i64::from(q.minimum)) % i64::from(q.step) != 0
    {
        return Err(invalid(
            "gain is outside supported IR trial bounds or control is unavailable",
        ));
    }
    Ok(())
}
pub fn read(interface: u8) -> io::Result<i32> {
    get(&gain_fd(interface)?)
}
impl<'lock> ScopedIrGain<'lock> {
    /// Requires the session lock for the whole capture/restoration lifetime.
    pub fn apply(lock: &'lock SessionLock, raw: i32) -> io::Result<Self> {
        let mut lease = Self {
            _lock: lock,
            fds: Vec::new(),
            changes: Vec::new(),
        };
        // Validate/read both before any mutation.
        for interface in [0, 2] {
            let fd = gain_fd(interface)?;
            // SAFETY: integer/byte-only UAPI struct with zeroed reserved fields.
            let mut q: v::v4l2_queryctrl = unsafe { zeroed() };
            q.id = v::V4L2_CID_GAIN;
            ioctl(&fd, 36, 3, &mut q)?;
            validate(&q, raw)?;
            let original = get(&fd)?;
            validate(&q, original)?;
            lease.fds.push(fd);
            lease.changes.push(GainChange {
                interface,
                original_raw: original,
                requested_raw: raw,
                restored: true,
            });
        }
        apply_all(&mut lease.changes, |i, value| set(&lease.fds[i], value))?;
        Ok(lease)
    }
    pub fn changes(&self) -> Vec<GainChange> {
        self.changes.clone()
    }
    /// Explicit restore and readback. Call before publishing a completed trial.
    pub fn restore(&mut self) -> io::Result<()> {
        restore_all(&mut self.changes, |i, value| set(&self.fds[i], value))
    }
}
impl Drop for ScopedIrGain<'_> {
    fn drop(&mut self) {
        if let Err(e) = self.restore() {
            eprintln!("{e}");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn changes() -> Vec<GainChange> {
        [0, 2]
            .into_iter()
            .map(|interface| GainChange {
                interface,
                original_raw: 1000,
                requested_raw: 8000,
                restored: true,
            })
            .collect()
    }
    #[test]
    fn ambiguous_second_write_failure_restores_both_controls() {
        let mut changes = changes();
        let mut actual = [1000; 2];
        let mut calls = 0;
        let result = apply_all(&mut changes, |i, value| {
            actual[i] = value;
            calls += 1;
            if calls == 2 {
                Err(invalid("injected failure after mutation"))
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        assert_eq!(actual, [1000, 1000]);
        assert_eq!(calls, 4);
        assert!(changes.iter().all(|c| c.restored));
    }
    #[test]
    fn restore_failure_does_not_skip_other_camera_or_claim_success() {
        let mut changes = changes();
        for c in &mut changes {
            c.restored = false;
        }
        let mut actual = [8000; 2];
        assert!(
            restore_all(&mut changes, |i, value| {
                if i == 1 {
                    Err(invalid("injected disconnected control"))
                } else {
                    actual[i] = value;
                    Ok(())
                }
            })
            .is_err()
        );
        assert_eq!(actual, [1000, 8000]);
        assert!(changes[0].restored);
        assert!(!changes[1].restored);
    }
    #[test]
    fn gain_trial_honors_raw_range_steps_and_unavailable_flags() {
        // SAFETY: all fields are integers/bytes.
        let mut q: v::v4l2_queryctrl = unsafe { zeroed() };
        q.type_ = v::v4l2_ctrl_type_V4L2_CTRL_TYPE_INTEGER;
        q.minimum = 1000;
        q.maximum = 15000;
        q.step = 100;
        for value in [1000, 3000, 8000, 15000] {
            assert!(validate(&q, value).is_ok());
        }
        for value in [999, 1050, 15001, i32::MIN, i32::MAX] {
            assert!(validate(&q, value).is_err());
        }
        for flag in [
            v::V4L2_CTRL_FLAG_DISABLED,
            v::V4L2_CTRL_FLAG_READ_ONLY,
            v::V4L2_CTRL_FLAG_INACTIVE,
            v::V4L2_CTRL_FLAG_GRABBED,
            v::V4L2_CTRL_FLAG_WRITE_ONLY,
        ] {
            q.flags = flag;
            assert!(validate(&q, 3000).is_err());
        }
        q.flags = v::V4L2_CTRL_FLAG_VOLATILE;
        assert!(validate(&q, 3000).is_ok());
        q.step = 0;
        assert!(validate(&q, 3000).is_err());
    }
}
