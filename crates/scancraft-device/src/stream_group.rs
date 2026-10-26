//! Save group settings before any worker starts; restore after all workers stop.
//! Restoration uses only the saved format/rate, never arbitrary control/property writes.
use crate::{VIDEO, discover, ioctl, open, stream_state};
use std::{fs::File, io};
use v4l2_sys_mit as v;
struct Entry {
    fd: File,
    format: v::v4l2_format,
    rate: v::v4l2_streamparm,
    expected: stream_state::State,
}
pub struct SettingsLease {
    entries: Vec<Entry>,
    restored: bool,
}
fn restore_plan(
    count: usize,
    format: impl Fn(usize) -> io::Result<()>,
    rate: impl Fn(usize) -> io::Result<()>,
    verify: impl Fn(usize) -> io::Result<()>,
) -> io::Result<()> {
    let mut errors = Vec::new();
    // Finish every format restoration before applying any saved rate.
    for (phase, action) in [
        ("format", &format as &dyn Fn(usize) -> io::Result<()>),
        ("rate", &rate),
        ("readback", &verify),
    ] {
        for i in 0..count {
            if let Err(e) = action(i) {
                errors.push(format!("{phase} entry {i}: {e}"));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "stream group restoration failed: {}",
            errors.join("; ")
        )))
    }
}
impl SettingsLease {
    /// Caller holds SessionLock. All entries are captured before streaming setup.
    pub fn capture(interfaces: &[u8]) -> io::Result<Self> {
        if interfaces.is_empty()
            || interfaces.len() > 3
            || interfaces.iter().any(|i| ![0, 2, 4].contains(i))
            || interfaces
                .iter()
                .enumerate()
                .any(|(n, i)| interfaces[..n].contains(i))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid stream settings group",
            ));
        }
        let nodes = discover()?;
        let mut entries = Vec::new();
        for interface in interfaces {
            let matching: Vec<_> = nodes
                .iter()
                .filter(|n| !n.metadata && n.interface == *interface)
                .collect();
            if matching.len() != 1 {
                return Err(io::Error::other(
                    "requires exactly one video node per settings group interface",
                ));
            }
            let fd = open(&matching[0].path)?;
            // SAFETY: zero-valid UAPI POD; VIDEO selects pix/capture members for GETs.
            let mut format: v::v4l2_format = unsafe { std::mem::zeroed() };
            format.type_ = VIDEO;
            ioctl(&fd, 4, 3, &mut format)?;
            // SAFETY: zero-valid UAPI POD initialized with capture type before GET_PARM.
            let mut rate: v::v4l2_streamparm = unsafe { std::mem::zeroed() };
            rate.type_ = VIDEO;
            ioctl(&fd, 21, 3, &mut rate)?;
            let expected = stream_state::from_fd(&fd, *interface)?;
            entries.push(Entry {
                fd,
                format,
                rate,
                expected,
            });
        }
        Ok(Self {
            entries,
            restored: false,
        })
    }
    pub fn restore(&mut self) -> io::Result<()> {
        if self.restored {
            return Ok(());
        }
        let result = restore_plan(
            self.entries.len(),
            |i| {
                let e = &self.entries[i];
                let mut format = e.format;
                ioctl(&e.fd, 5, 3, &mut format)
            },
            |i| {
                let e = &self.entries[i];
                let mut rate = e.rate;
                ioctl(&e.fd, 22, 3, &mut rate)
            },
            |i| {
                let e = &self.entries[i];
                if stream_state::from_fd(&e.fd, e.expected.interface)? != e.expected {
                    return Err(io::Error::other(
                        "reported format/rate differs from pre-group snapshot",
                    ));
                }
                Ok(())
            },
        );
        self.restored = result.is_ok();
        result
    }
}
impl Drop for SettingsLease {
    fn drop(&mut self) {
        if let Err(e) = self.restore() {
            eprintln!("{e}");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[test]
    fn group_restores_all_formats_before_any_rate_then_verifies_every_entry() {
        let events = RefCell::new(Vec::new());
        restore_plan(
            2,
            |i| {
                events.borrow_mut().push(("format", i));
                Ok(())
            },
            |i| {
                events.borrow_mut().push(("rate", i));
                Ok(())
            },
            |i| {
                events.borrow_mut().push(("verify", i));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            *events.borrow(),
            [
                ("format", 0),
                ("format", 1),
                ("rate", 0),
                ("rate", 1),
                ("verify", 0),
                ("verify", 1)
            ]
        );
    }
    #[test]
    fn every_entry_is_attempted_and_readback_failures_are_not_hidden() {
        let calls = RefCell::new(Vec::new());
        let result = restore_plan(
            2,
            |i| {
                calls.borrow_mut().push((0, i));
                if i == 0 {
                    Err(io::Error::other("format rejected"))
                } else {
                    Ok(())
                }
            },
            |i| {
                calls.borrow_mut().push((1, i));
                Ok(())
            },
            |i| {
                calls.borrow_mut().push((2, i));
                Err(io::Error::other("readback mismatch"))
            },
        );
        assert!(result.unwrap_err().to_string().contains("readback"));
        assert_eq!(calls.borrow().len(), 6);
    }
    #[test]
    fn invalid_groups_fail_before_discovery_or_device_changes() {
        for group in [&[][..], &[0, 0], &[1], &[0, 2, 4, 0]] {
            assert!(SettingsLease::capture(group).is_err());
        }
    }
}
