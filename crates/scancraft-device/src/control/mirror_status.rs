//! Fixed mirror-state reads. Reported flags do not establish physical image axes.
use super::{ReadChannel, illumination::RangeRaw};
use crate::{SessionLock, invalid};
use serde::Serialize;
use std::io;

const PROPERTIES: [(u32, &str); 3] = [
    (18, "OB_PROP_IR_MIRROR_BOOL"),
    (112, "OB_PROP_IR_RIGHT_MIRROR_BOOL"),
    (81, "OB_PROP_COLOR_MIRROR_BOOL"),
];

#[derive(Debug, Serialize)]
pub struct Reading {
    pub property_id: u32,
    pub sdk_symbol: &'static str,
    pub range_raw: Option<RangeRaw>,
    pub reported_enabled: Option<bool>,
    pub error: Option<String>,
}

fn packet(index: usize, id: u16) -> io::Result<[u8; 64]> {
    let (property, _) = PROPERTIES
        .get(index)
        .ok_or_else(|| invalid("property outside fixed mirror snapshot"))?;
    let mut bytes = [0; 64];
    for (i, word) in [0x4d47u16, 2, 1, id].iter().enumerate() {
        bytes[2 * i..2 * i + 2].copy_from_slice(&word.to_le_bytes());
    }
    bytes[8..12].copy_from_slice(&property.to_le_bytes());
    Ok(bytes)
}

fn enabled(range: &RangeRaw) -> io::Result<bool> {
    if !range.consistent_integer() || range.minimum != 0 || range.maximum != 1 || range.step != 1 {
        return Err(invalid("mirror property is not a consistent boolean range"));
    }
    Ok(range.current == 1)
}

/// Session lock covers the entire fixed GET_PROPERTY sequence. No setter exists.
pub fn snapshot(_lock: &SessionLock) -> io::Result<Vec<Reading>> {
    let channel = ReadChannel::open()?;
    PROPERTIES
        .iter()
        .enumerate()
        .map(|(i, &(property_id, sdk_symbol))| {
            let id = 5000 + i as u16;
            let result = channel
                .request(packet(i, id)?, 1, id)
                .and_then(|bytes| RangeRaw::parse(&bytes));
            let (range_raw, reported_enabled, error) = match result {
                Ok(range) => match enabled(&range) {
                    Ok(value) => (Some(range), Some(value), None),
                    Err(e) => (Some(range), None, Some(e.to_string())),
                },
                Err(e) => (None, None, Some(e.to_string())),
            };
            Ok(Reading {
                property_id,
                sdk_symbol,
                range_raw,
                reported_enabled,
                error,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_snapshot_has_only_get_envelopes() {
        for (i, property) in [18u32, 112, 81].iter().enumerate() {
            let bytes = packet(i, 5000).unwrap();
            assert_eq!(&bytes[..8], &[0x47, 0x4d, 2, 0, 1, 0, 0x88, 0x13]);
            assert_eq!(&bytes[8..12], &property.to_le_bytes());
            assert!(bytes[12..].iter().all(|b| *b == 0));
        }
        assert!(packet(3, 5000).is_err());
        assert!(packet(usize::MAX, 5000).is_err());
    }
    #[test]
    fn malformed_boolean_ranges_never_become_mirror_claims() {
        let mut range = RangeRaw {
            current: 1,
            maximum: 1,
            minimum: 0,
            default: 0,
            step: 1,
        };
        assert!(enabled(&range).unwrap());
        range.current = 0;
        assert!(!enabled(&range).unwrap());
        range.current = 2;
        assert!(enabled(&range).is_err());
        range.maximum = 2;
        assert!(enabled(&range).is_err());
        range.maximum = 1;
        range.current = 0;
        range.step = 0;
        assert!(enabled(&range).is_err());
    }
}
