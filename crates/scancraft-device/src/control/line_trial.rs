//! Scoped fixed SDK line-laser boolean trial. No mode/energy/calibration/firmware writes.
use super::{ReadChannel, illumination::RangeRaw};
use crate::{SessionLock, invalid};
use serde::Serialize;
use std::io;
#[derive(Debug, Serialize)]
pub struct State {
    pub original_raw: i32,
    pub requested_raw: i32,
    pub restored: bool,
}
pub struct ScopedLine<'a> {
    _lock: &'a SessionLock,
    channel: ReadChannel,
    state: State,
}
fn get_packet(id: u16) -> [u8; 64] {
    let mut b = [0; 64];
    for (i, v) in [0x4d47u16, 2, 1, id].iter().enumerate() {
        b[i * 2..i * 2 + 2].copy_from_slice(&v.to_le_bytes());
    }
    b[8..12].copy_from_slice(&151u32.to_le_bytes());
    b
}
fn set_packet(enabled: bool, id: u16) -> [u8; 64] {
    let mut b = get_packet(id);
    b[2..4].copy_from_slice(&4u16.to_le_bytes());
    b[4..6].copy_from_slice(&2u16.to_le_bytes());
    b[12..16].copy_from_slice(&u32::from(enabled).to_le_bytes());
    b
}
fn boolean(range: &RangeRaw) -> io::Result<i32> {
    if !range.consistent_integer() || range.minimum != 0 || range.maximum != 1 || range.step != 1 {
        return Err(invalid(
            "line-laser property is not a consistent supported boolean",
        ));
    }
    Ok(range.current)
}
impl<'a> ScopedLine<'a> {
    pub fn enable(lock: &'a SessionLock) -> io::Result<Self> {
        let channel = ReadChannel::open()?;
        let original = boolean(&RangeRaw::parse(&channel.request(
            get_packet(4000),
            1,
            4000,
        )?)?)?;
        let mut lease = Self {
            _lock: lock,
            channel,
            state: State {
                original_raw: original,
                requested_raw: 1,
                restored: false,
            },
        };
        if let Err(e) = lease.set(true, 4001) {
            let restoration = lease.restore();
            return Err(invalid(format!(
                "line-laser enable failed: {e}; restoration: {restoration:?}"
            )));
        }
        Ok(lease)
    }
    fn set(&self, enabled: bool, id: u16) -> io::Result<()> {
        let response = self.channel.request(set_packet(enabled, id), 2, id)?;
        if !response.is_empty() {
            return Err(invalid("unexpected line-laser setter response payload"));
        }
        let observed = boolean(&RangeRaw::parse(&self.channel.request(
            get_packet(id + 1),
            1,
            id + 1,
        )?)?)?;
        if observed != i32::from(enabled) {
            return Err(invalid("line-laser readback mismatch"));
        }
        Ok(())
    }
    pub fn verify_enabled(&self) -> io::Result<()> {
        if boolean(&RangeRaw::parse(&self.channel.request(
            get_packet(4010),
            1,
            4010,
        )?)?)?
            != 1
        {
            return Err(invalid("line-laser state changed during capture"));
        }
        Ok(())
    }
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn restore(&mut self) -> io::Result<()> {
        if !self.state.restored {
            self.set(self.state.original_raw == 1, 4020).map_err(|e| {
                invalid(format!(
                    "line-laser restore to {} failed: {e}",
                    self.state.original_raw
                ))
            })?;
            self.state.restored = true;
        }
        Ok(())
    }
}
impl Drop for ScopedLine<'_> {
    fn drop(&mut self) {
        if let Err(e) = self.restore() {
            eprintln!("{e}");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_line_boolean_can_be_encoded_and_inconsistent_ranges_fail() {
        for enabled in [false, true] {
            let b = set_packet(enabled, 88);
            assert_eq!(&b[..12], &[0x47, 0x4d, 4, 0, 2, 0, 88, 0, 151, 0, 0, 0]);
            assert_eq!(&b[12..16], &u32::from(enabled).to_le_bytes());
            assert!(b[16..].iter().all(|v| *v == 0));
        }
        assert_eq!(
            &get_packet(99)[..12],
            &[0x47, 0x4d, 2, 0, 1, 0, 99, 0, 151, 0, 0, 0]
        );
        for current in [0, 1] {
            assert_eq!(
                boolean(&RangeRaw {
                    current,
                    maximum: 1,
                    minimum: 0,
                    default: 0,
                    step: 1
                })
                .unwrap(),
                current
            );
        }
        for current in [-1, 2, 393216] {
            assert!(
                boolean(&RangeRaw {
                    current,
                    maximum: 1,
                    minimum: 0,
                    default: 0,
                    step: 1
                })
                .is_err()
            );
        }
        assert!(
            boolean(&RangeRaw {
                current: 0,
                maximum: 8,
                minimum: 0,
                default: 0,
                step: 1
            })
            .is_err()
        );
    }
}
