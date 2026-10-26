//! Fixed read-only illumination-related SDK properties. Numeric states do not
//! prove physical emission, scan-mode binding, or valid illumination.
use super::ReadChannel;
use crate::{SessionLock, invalid};
use serde::Serialize;
use std::io;
const PROPERTIES: [(u32, &str); 17] = [
    (3, "OB_PROP_LASER_BOOL"),
    (6, "OB_PROP_FLOOD_BOOL"),
    (7, "OB_PROP_FLOOD_LEVEL_INT"),
    (79, "OB_PROP_LASER_MODE_INT"),
    (99, "OB_PROP_LASER_ENERGY_LEVEL_INT"),
    (119, "OB_PROP_LASER_HW_ENERGY_LEVEL_INT"),
    (148, "OB_PROP_LASER_OVERCURRENT_PROTECTION_STATUS_BOOL"),
    (149, "OB_PROP_LASER_PULSE_WIDTH_PROTECTION_STATUS_BOOL"),
    (151, "OB_PROP_LASER_LINE_BOOL"),
    (152, "OB_PROP_LASER_2_BOOL"),
    (153, "OB_PROP_LASER_3_BOOL"),
    (192, "OB_PROP_LINE_LASER_MODE_SWITCH_INT"),
    (197, "OB_PROP_IR_LEFT_LINE_LASER_LIGHT_TYPE_INT"),
    (198, "OB_PROP_IR_RIGHT_LINE_LASER_LIGHT_TYPE_INT"),
    (219, "OB_PROP_LASER_4_BOOL"),
    (239, "OB_PROP_LASER_5_BOOL"),
    (83, "OB_PROP_INDICATOR_LIGHT_BOOL"),
];
#[derive(Debug, Serialize, PartialEq)]
pub struct RangeRaw {
    pub current: i32,
    pub maximum: i32,
    pub minimum: i32,
    pub default: i32,
    pub step: i32,
}
impl RangeRaw {
    pub(super) fn parse(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != 20 {
            return Err(invalid(
                "property response must contain exactly five raw words",
            ));
        }
        let at = |i| i32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        Ok(Self {
            current: at(0),
            maximum: at(4),
            minimum: at(8),
            default: at(12),
            step: at(16),
        })
    }
    pub fn consistent_integer(&self) -> bool {
        self.minimum <= self.maximum
            && self.step > 0
            && [self.current, self.default].iter().all(|v| {
                *v >= self.minimum
                    && *v <= self.maximum
                    && (i64::from(*v) - i64::from(self.minimum)) % i64::from(self.step) == 0
            })
    }
}
#[derive(Debug, Serialize)]
pub struct Reading {
    pub property_id: u32,
    pub sdk_symbol: &'static str,
    pub range_raw: Option<RangeRaw>,
    pub error: Option<String>,
    pub integer_range_consistent: Option<bool>,
}
fn packet(index: usize, id: u16) -> io::Result<[u8; 64]> {
    let (property, _) = PROPERTIES
        .get(index)
        .ok_or_else(|| invalid("property outside fixed snapshot"))?;
    let mut b = [0; 64];
    for (i, v) in [0x4d47u16, 2, 1, id].iter().enumerate() {
        b[i * 2..i * 2 + 2].copy_from_slice(&v.to_le_bytes());
    }
    b[8..12].copy_from_slice(&property.to_le_bytes());
    Ok(b)
}
/// The caller's session lock covers the entire fixed GET_PROPERTY sequence.
/// SET_CUR only carries a GET_PROPERTY envelope, never a property setter.
pub fn snapshot(_lock: &SessionLock) -> io::Result<Vec<Reading>> {
    let channel = ReadChannel::open()?;
    let mut readings = Vec::with_capacity(PROPERTIES.len());
    for (i, (property_id, sdk_symbol)) in PROPERTIES.iter().enumerate() {
        let id = 2000 + i as u16;
        let result = channel
            .request(packet(i, id)?, 1, id)
            .and_then(|b| RangeRaw::parse(&b));
        let (range_raw, error) = match result {
            Ok(r) => (Some(r), None),
            Err(e) => (None, Some(e.to_string())),
        };
        let integer_range_consistent = range_raw.as_ref().map(RangeRaw::consistent_integer);
        let error = if integer_range_consistent == Some(false) {
            Some("inconsistent raw integer range; unusable for setting".into())
        } else {
            error
        };
        readings.push(Reading {
            property_id: *property_id,
            sdk_symbol,
            range_raw,
            error,
            integer_range_consistent,
        });
    }
    Ok(readings)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_fixed_get_property_packets_are_constructible() {
        for (i, (property, _)) in PROPERTIES.iter().enumerate() {
            let b = packet(i, 77).unwrap();
            assert_eq!(&b[..8], &[0x47, 0x4d, 2, 0, 1, 0, 77, 0]);
            assert_eq!(u32::from_le_bytes(b[8..12].try_into().unwrap()), *property);
            assert!(b[12..].iter().all(|v| *v == 0));
        }
        assert!(packet(PROPERTIES.len(), 0).is_err());
        assert!(packet(usize::MAX, 0).is_err());
    }
    #[test]
    fn inconsistent_success_payload_is_not_a_usable_setting_range() {
        assert!(
            !RangeRaw {
                current: 393216,
                maximum: 277,
                minimum: 705,
                default: 33429,
                step: 0
            }
            .consistent_integer()
        );
        assert!(
            RangeRaw {
                current: 0,
                maximum: 1,
                minimum: 0,
                default: 0,
                step: 1
            }
            .consistent_integer()
        );
    }
    #[test]
    fn property_words_keep_exact_signed_order_and_reject_wrong_lengths() {
        let bytes: Vec<u8> = [-1i32, 100, -20, 30, 5]
            .into_iter()
            .flat_map(i32::to_le_bytes)
            .collect();
        assert_eq!(
            RangeRaw::parse(&bytes).unwrap(),
            RangeRaw {
                current: -1,
                maximum: 100,
                minimum: -20,
                default: 30,
                step: 5
            }
        );
        for n in 0..64 {
            if n != 20 {
                assert!(RangeRaw::parse(&vec![0; n]).is_err());
            }
        }
    }
}
