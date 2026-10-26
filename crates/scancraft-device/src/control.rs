//! Host-protocol query transport; operational setters are isolated in scoped modules.
pub mod flood_trial;
pub mod illumination;
pub mod line_trial;
pub mod mirror_status;
use crate::{discover, invalid, open};
use std::{fs::File, io, mem::size_of, os::fd::AsRawFd};
#[repr(C)]
struct Query {
    unit: u8,
    selector: u8,
    query: u8,
    size: u16,
    data: *mut u8,
}
fn query(fd: &File, unit: u8, selector: u8, request: u8, data: &mut [u8]) -> io::Result<()> {
    if ![1, 2, 3].contains(&selector) || data.is_empty() || data.len() > 1024 {
        return Err(invalid("invalid XU query shape"));
    }
    let mut q = Query {
        unit,
        selector,
        query: request,
        size: data.len() as u16,
        data: data.as_mut_ptr(),
    };
    let code = ((3u32 << 30) | ((size_of::<Query>() as u32) << 16) | (u32::from(b'u') << 8) | 0x21)
        as libc::c_ulong;
    // SAFETY: repr(C) matches uvc_xu_control_query; data remains writable for the complete ioctl.
    if unsafe { libc::ioctl(fd.as_raw_fd(), code, &mut q as *mut Query) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn extension_unit(descriptors: &[u8]) -> io::Result<u8> {
    let guid = [
        0xa1, 0x51, 0x57, 0xa5, 0xc5, 0xf3, 0x5e, 0x4a, 0x8d, 0x5a, 0x68, 0x54, 0xb8, 0xfa, 0x27,
        0x16,
    ];
    let mut pos = 0;
    let mut control_interface = false;
    let mut found = None;
    while pos < descriptors.len() {
        let length = descriptors[pos] as usize;
        if length < 2 || pos + length > descriptors.len() {
            return Err(invalid("truncated USB descriptor"));
        }
        let d = &descriptors[pos..pos + length];
        if d[1] == 4 {
            control_interface = length >= 9 && d[2] == 0 && d[5] == 14 && d[6] == 1;
        }
        if control_interface && d[1] == 0x24 && length >= 24 && d[2] == 6 && d[4..20] == guid {
            if found.is_some() {
                return Err(invalid("ambiguous extension unit"));
            }
            found = Some(d[3]);
        }
        pos += length;
    }
    found.ok_or_else(|| invalid("Raptor XU GUID not present"))
}
pub fn parse_response(response: &[u8], opcode: u16, id: u16) -> io::Result<Vec<u8>> {
    if response.len() < 10 {
        return Err(invalid("truncated host response"));
    }
    let word = |i| u16::from_le_bytes([response[i], response[i + 1]]);
    if word(0) != 0x4252 || word(4) != opcode || word(6) != id {
        return Err(invalid("response magic/opcode/id mismatch"));
    }
    let end = 8 + usize::from(word(2)) * 2;
    if end < 10 || end > response.len() {
        return Err(invalid("invalid response payload length"));
    }
    if word(8) != 0 {
        return Err(invalid(format!("device response status {:#x}", word(8))));
    }
    Ok(response[10..end].to_vec())
}
/// Bounded factory regions observed in the original vendor read trace.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum FactoryRegion {
    Region200000,
    Region202000,
    Region20a000,
}
impl FactoryRegion {
    pub fn base(self) -> u32 {
        match self {
            Self::Region200000 => 0x200000,
            Self::Region202000 => 0x202000,
            Self::Region20a000 => 0x20a000,
        }
    }
    pub fn parse(value: &str) -> io::Result<Self> {
        match value {
            "200000" => Ok(Self::Region200000),
            "202000" => Ok(Self::Region202000),
            "20a000" => Ok(Self::Region20a000),
            _ => Err(invalid("factory region must be 200000, 202000 or 20a000")),
        }
    }
}
pub const FACTORY_REGION_BYTES: usize = 8192;
#[derive(Debug, Clone, serde::Serialize)]
pub struct FactoryHeader {
    pub raw_words: [u32; 8],
    pub recognized_signature: bool,
}
impl FactoryHeader {
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != 32 {
            return Err(invalid("factory header must contain exactly 32 bytes"));
        }
        let raw_words = std::array::from_fn(|i| {
            u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap())
        });
        Ok(Self {
            recognized_signature: matches!(raw_words[0], 0x46447311 | 0x46447312),
            raw_words,
        })
    }
}
fn factory_packet(region: FactoryRegion, offset: usize, id: u16) -> io::Result<[u8; 64]> {
    if offset >= FACTORY_REGION_BYTES || !offset.is_multiple_of(32) {
        return Err(invalid(
            "factory read offset outside fixed region/alignment",
        ));
    }
    let mut packet = [0; 64];
    for (i, word) in [0x4d47u16, 3, 25, id].iter().enumerate() {
        packet[i * 2..i * 2 + 2].copy_from_slice(&word.to_le_bytes());
    }
    packet[8..12].copy_from_slice(&(region.base() + offset as u32).to_le_bytes());
    packet[12..14].copy_from_slice(&16u16.to_le_bytes());
    Ok(packet)
}
struct ReadChannel {
    fd: File,
    unit: u8,
}
impl ReadChannel {
    fn open() -> io::Result<Self> {
        let nodes = discover()?;
        let videos: Vec<_> = nodes
            .iter()
            .filter(|n| !n.metadata && n.interface == 0)
            .collect();
        if videos.len() != 1 {
            return Err(invalid("requires exactly one Raptor"));
        }
        let node = videos[0];
        let unit = extension_unit(&std::fs::read(node.usb_path.join("descriptors"))?)?;
        let fd = open(&node.path)?;
        // SAFETY: valid owned fd; lock prevents cooperating command users interleaving.
        if unsafe { libc::flock(fd.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut length = [0; 2];
        query(&fd, unit, 2, 0x85, &mut length)?;
        if u16::from_le_bytes(length) != 64 {
            return Err(invalid("unexpected XU channel length"));
        }
        Ok(Self { fd, unit })
    }
    fn request(&self, mut packet: [u8; 64], opcode: u16, id: u16) -> io::Result<Vec<u8>> {
        // SET_CUR carries a read command envelope; it is not a flash write.
        query(&self.fd, self.unit, 2, 1, &mut packet)?;
        let mut response = [0; 64];
        query(&self.fd, self.unit, 2, 0x81, &mut response)?;
        parse_response(&response, opcode, id)
    }
    fn factory_chunk(&self, region: FactoryRegion, offset: usize, id: u16) -> io::Result<Vec<u8>> {
        let data = self.request(factory_packet(region, offset, id)?, 25, id)?;
        if data.len() != 32 {
            return Err(invalid("factory read response length mismatch"));
        }
        Ok(data)
    }
}
/// Read-only GET_PROTOCOL_VERSION. No generic vendor/firmware/calibration writer.
pub fn protocol_version() -> io::Result<Vec<u8>> {
    let channel = ReadChannel::open()?;
    let mut packet = [0; 64];
    for (i, word) in [0x4d47u16, 0, 0xfe, 1].iter().enumerate() {
        packet[i * 2..i * 2 + 2].copy_from_slice(&word.to_le_bytes());
    }
    channel.request(packet, 0xfe, 1)
}
/// Read 32 bytes from one previously observed region. Hold SessionLock in caller.
pub fn factory_header(region: FactoryRegion) -> io::Result<FactoryHeader> {
    FactoryHeader::parse(&ReadChannel::open()?.factory_chunk(region, 0, 1000)?)
}
/// Read a fixed 8 KiB region using only opcode 25 and 32-byte chunks. Unknown
/// signatures/implausible size words abort; no decoded optical model is implied.
pub fn factory_region(region: FactoryRegion) -> io::Result<Vec<u8>> {
    let channel = ReadChannel::open()?;
    let first = channel.factory_chunk(region, 0, 1000)?;
    let header = FactoryHeader::parse(&first)?;
    if !header.recognized_signature || !(32..=8192).contains(&header.raw_words[2]) {
        return Err(invalid(
            "factory header format/size word is not recognized; refusing region dump",
        ));
    }
    let mut data = first;
    for offset in (32..FACTORY_REGION_BYTES).step_by(32) {
        data.extend_from_slice(&channel.factory_chunk(
            region,
            offset,
            1000 + (offset / 32) as u16,
        )?);
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn factory_reads_are_fixed_opcode_aligned_and_region_bounded() {
        for region in [
            FactoryRegion::Region200000,
            FactoryRegion::Region202000,
            FactoryRegion::Region20a000,
        ] {
            for offset in (0..FACTORY_REGION_BYTES).step_by(32) {
                let packet = factory_packet(region, offset, 123).unwrap();
                assert_eq!(&packet[..8], &[0x47, 0x4d, 3, 0, 25, 0, 123, 0]);
                assert_eq!(
                    u32::from_le_bytes(packet[8..12].try_into().unwrap()),
                    region.base() + offset as u32
                );
                assert_eq!(u16::from_le_bytes(packet[12..14].try_into().unwrap()), 16);
                assert!(packet[14..].iter().all(|b| *b == 0));
            }
            for offset in [1, 31, 8192, usize::MAX] {
                assert!(factory_packet(region, offset, 0).is_err());
            }
        }
        assert!(FactoryRegion::parse("215000").is_err());
    }
    #[test]
    fn factory_header_preserves_opaque_fields_without_inventing_optics() {
        let mut bytes = [0; 32];
        bytes[..4].copy_from_slice(&0x46447311u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&6780u32.to_le_bytes());
        let header = FactoryHeader::parse(&bytes).unwrap();
        assert!(header.recognized_signature);
        assert_eq!(header.raw_words[2], 6780);
        assert!(
            !FactoryHeader::parse(&[255; 32])
                .unwrap()
                .recognized_signature
        );
        assert!(FactoryHeader::parse(&bytes[..31]).is_err());
    }
    #[test]
    fn rejects_bad_envelopes() {
        for n in 0..100 {
            assert!(parse_response(&vec![0; n], 0xfe, 1).is_err());
        }
    }
    #[test]
    fn valid_envelope_and_mismatches() {
        let mut r = Vec::new();
        for v in [0x4252u16, 3, 0xfe, 1, 0, 42, 43] {
            r.extend_from_slice(&v.to_le_bytes());
        }
        assert_eq!(parse_response(&r, 0xfe, 1).unwrap(), [42, 0, 43, 0]);
        assert!(parse_response(&r, 0xfe, 2).is_err());
        r[8] = 1;
        assert!(parse_response(&r, 0xfe, 1).is_err());
    }
    #[test]
    fn descriptor_bounds() {
        for n in 0..100 {
            assert!(extension_unit(&vec![255; n]).is_err());
        }
    }
}
