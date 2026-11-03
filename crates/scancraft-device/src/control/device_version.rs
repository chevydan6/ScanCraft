//! Fixed GET_STRUCTURE_DATA(1000) query; never updates firmware or device settings.
//! Protocol/layout reference: OrbbecSDK_v2 85e94c4ee4032817df2e33228dbd7f51517d3d4e,
//! Protocol.cpp::initGetStructureDataReq, InternalTypes.hpp::OBVersionInfo.
//! Independently implemented Rust; observed 164-byte Raptor reply only.
use super::{ReadChannel, parse_response, query};
use crate::{SessionLock, invalid};
use serde::Serialize;
use std::io;

const OPCODE: u16 = 3;
const PROPERTY: u32 = 1000;
const REQUEST_ID: u16 = 4600;
const PAYLOAD_BYTES: usize = 164;

/// Device-reported strings, not authenticated identity or calibration binding.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DeviceVersion {
    pub firmware: String,
    pub hardware: String,
    pub minimum_sdk: String,
    pub depth_chip: String,
    pub system_chip: String,
    pub serial: String,
    pub device_type_raw: i32,
    pub device_name: String,
    pub subsystem: String,
}

fn text(bytes: &[u8]) -> io::Result<String> {
    let end = bytes
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| invalid("unterminated device-version field"))?;
    if !bytes[..end].iter().all(|b| (0x20..=0x7e).contains(b)) {
        return Err(invalid("device-version field is not printable ASCII"));
    }
    // Bytes after the C-string terminator are padding, not part of its value.
    String::from_utf8(bytes[..end].to_vec()).map_err(|e| invalid(e.to_string()))
}
fn parse(bytes: &[u8]) -> io::Result<DeviceVersion> {
    if bytes.len() != PAYLOAD_BYTES {
        return Err(invalid("unsupported device-version payload length"));
    }
    let value = DeviceVersion {
        firmware: text(&bytes[0..16])?,
        hardware: text(&bytes[16..32])?,
        minimum_sdk: text(&bytes[32..48])?,
        depth_chip: text(&bytes[48..64])?,
        system_chip: text(&bytes[64..80])?,
        serial: text(&bytes[80..96])?,
        device_type_raw: i32::from_le_bytes(bytes[96..100].try_into().unwrap()),
        device_name: text(&bytes[100..116])?,
        subsystem: text(&bytes[116..132])?,
    };
    // Reserved bytes132..164 are deliberately uninterpreted. A serial may be
    // empty or a placeholder; none of these strings authenticate a device.
    if value.firmware.is_empty() {
        return Err(invalid("empty device-reported firmware version"));
    }
    Ok(value)
}
fn packet() -> [u8; 64] {
    let mut bytes = [0; 64];
    for (i, word) in [0x4d47u16, 2, OPCODE, REQUEST_ID].iter().enumerate() {
        bytes[i * 2..i * 2 + 2].copy_from_slice(&word.to_le_bytes());
    }
    bytes[8..12].copy_from_slice(&PROPERTY.to_le_bytes());
    bytes
}

/// Requires the process-wide session lock. The fd lock also protects the host
/// request/reply pair. SET_CUR transports a GET envelope; no setter is exposed.
/// This is an explicit command, not an additional query in acquisition threads.
pub fn read(_lock: &SessionLock) -> io::Result<DeviceVersion> {
    let channel = ReadChannel::open()?;
    let mut length = [0; 2];
    query(&channel.fd, channel.unit, 1, 0x85, &mut length)?;
    if u16::from_le_bytes(length) != 512 {
        return Err(invalid("unexpected device-version response channel length"));
    }
    query(&channel.fd, channel.unit, 2, 1, &mut packet())?;
    let mut response = [0; 512];
    query(&channel.fd, channel.unit, 1, 0x81, &mut response)?;
    parse(&parse_response(&response, OPCODE, REQUEST_ID)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> [u8; PAYLOAD_BYTES] {
        let mut bytes = [0; PAYLOAD_BYTES];
        for (offset, value) in [
            (0, "2.3.4"),
            (16, "test-board"),
            (32, "1.0.0"),
            (48, "depth-test"),
            (64, "system-test"),
            (80, "synthetic-id"),
            (100, "fixture-device"),
            (116, "sub-test"),
        ] {
            bytes[offset..offset + value.len()].copy_from_slice(value.as_bytes());
        }
        bytes[96..100].copy_from_slice(&2_i32.to_le_bytes());
        bytes
    }
    #[test]
    fn decodes_fixed_layout_without_interpreting_padding_or_reserved_bytes() {
        let mut bytes = fixture();
        bytes[15] = 0xff;
        bytes[132..].fill(0xff);
        let v = parse(&bytes).unwrap();
        assert_eq!(v.firmware, "2.3.4");
        assert_eq!(v.hardware, "test-board");
        assert_eq!(v.minimum_sdk, "1.0.0");
        assert_eq!(v.depth_chip, "depth-test");
        assert_eq!(v.system_chip, "system-test");
        assert_eq!(v.serial, "synthetic-id");
        assert_eq!(v.device_type_raw, 2);
        assert_eq!(v.device_name, "fixture-device");
        assert_eq!(v.subsystem, "sub-test");
        bytes[80..96].fill(0);
        assert!(parse(&bytes).unwrap().serial.is_empty());
    }
    #[test]
    fn rejects_unknown_layout_and_malformed_text() {
        let fixture = fixture();
        for n in 0..fixture.len() {
            assert!(parse(&fixture[..n]).is_err());
        }
        let mut extended = fixture.to_vec();
        extended.push(0);
        assert!(parse(&extended).is_err());
        for offset in [0, 16, 32, 48, 64, 80, 100, 116] {
            let mut bad = fixture;
            bad[offset..offset + 16].fill(b'x');
            assert!(parse(&bad).is_err());
            bad = fixture;
            bad[offset] = 0xff;
            assert!(parse(&bad).is_err());
            bad[offset] = b'\n';
            assert!(parse(&bad).is_err());
        }
        let mut empty = fixture;
        empty[0] = 0;
        assert!(parse(&empty).is_err());
    }
    #[test]
    fn only_fixed_version_get_and_matching_reply_are_accepted() {
        let request = packet();
        assert_eq!(
            &request[..12],
            &[0x47, 0x4d, 2, 0, 3, 0, 0xf8, 0x11, 0xe8, 3, 0, 0]
        );
        assert!(request[12..].iter().all(|&b| b == 0));
        let mut response = [0; 512];
        for (i, word) in [0x4252_u16, 83, OPCODE, REQUEST_ID, 0].iter().enumerate() {
            response[i * 2..i * 2 + 2].copy_from_slice(&word.to_le_bytes());
        }
        response[10..174].copy_from_slice(&fixture());
        assert_eq!(
            parse(&parse_response(&response, OPCODE, REQUEST_ID).unwrap())
                .unwrap()
                .firmware,
            "2.3.4"
        );
        for offset in [0, 4, 6, 8] {
            let mut bad = response;
            bad[offset] ^= 1;
            assert!(parse_response(&bad, OPCODE, REQUEST_ID).is_err());
        }
        response[2..4].copy_from_slice(&82_u16.to_le_bytes());
        assert!(parse(&parse_response(&response, OPCODE, REQUEST_ID).unwrap()).is_err());
    }
}
