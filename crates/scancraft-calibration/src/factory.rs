//! Observed factory container integrity and indexing, not an optical calibration decoder.
//! No camera assignments, units, distortion model or extrinsics are inferred here.
use serde::Serialize;

const LIMIT: usize = 8192;
const HEADER: usize = 60;
const STRIDE: usize = 1680;

#[derive(Debug, Serialize, PartialEq)]
pub struct FactoryIndex {
    pub declared_bytes: usize,
    pub checksum_crc32: u32,
    pub records: Vec<FactoryRecord>,
}
#[derive(Debug, Serialize, PartialEq)]
pub struct FactoryRecord {
    pub offset: usize,
    pub bytes: usize,
    pub label: String,
}
/// Integrity of an observed factory envelope, without interpreting its payload.
#[derive(Debug, Serialize, PartialEq)]
pub struct FactoryEnvelope {
    pub signature: u32,
    pub body_signature: u32,
    pub declared_bytes: usize,
    pub checksum_crc32: u32,
}
/// Validate either observed envelope signature pair. Bytes beyond the declared
/// length are flash padding and deliberately not authenticated. A valid CRC is
/// not proof of camera mapping, optical semantics, or physical accuracy.
pub fn verify(bytes: &[u8]) -> Result<FactoryEnvelope, String> {
    if !(HEADER..=LIMIT).contains(&bytes.len()) {
        return Err("factory input must contain 60..8192 bytes".into());
    }
    let word = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    let signature = word(0);
    let body_signature = word(48);
    if !matches!(
        (signature, body_signature),
        (0x46447311, 0x4644610e) | (0x46447312, 0x4644610f)
    ) {
        return Err("unsupported factory envelope signature pair".into());
    }
    let declared = word(8) as usize;
    if !(HEADER..=bytes.len()).contains(&declared) {
        return Err("factory declared length outside input".into());
    }
    let mut crc = crc32fast::Hasher::new();
    crc.update(&bytes[..4]);
    crc.update(&[0; 4]);
    crc.update(&bytes[8..declared]);
    let checksum = word(4);
    if crc.finalize() != checksum {
        return Err("factory checksum mismatch".into());
    }
    Ok(FactoryEnvelope {
        signature,
        body_signature,
        declared_bytes: declared,
        checksum_crc32: checksum,
    })
}
/// Index only the observed original-Raptor laser records. Other verified
/// envelopes remain opaque and cannot be used as a calibrated camera model.
pub fn inspect(bytes: &[u8]) -> Result<FactoryIndex, String> {
    let envelope = verify(bytes)?;
    if envelope.signature != 0x46447311 {
        return Err("factory payload schema unsupported for record indexing".into());
    }
    let word = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    let declared = envelope.declared_bytes;
    let checksum = envelope.checksum_crc32;
    let count = word(52) as usize;
    if word(56) as usize != STRIDE
        || count == 0
        || count > (LIMIT - HEADER) / STRIDE
        || HEADER + count * STRIDE != declared
    {
        return Err("unsupported factory record count/stride/length".into());
    }
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let offset = HEADER + i * STRIDE;
        let raw = &bytes[offset + 16..offset + 32];
        let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
        if end == 0 || !raw[..end].iter().all(|b| b.is_ascii_graphic()) {
            return Err("invalid factory record label".into());
        }
        let label = std::str::from_utf8(&raw[..end]).unwrap().to_owned();
        records.push(FactoryRecord {
            offset,
            bytes: STRIDE,
            label,
        });
    }
    Ok(FactoryIndex {
        declared_bytes: declared,
        checksum_crc32: checksum,
        records,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seal(b: &mut [u8]) {
        b[4..8].fill(0);
        let end = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
        let crc = crc32fast::hash(&b[..end]);
        b[4..8].copy_from_slice(&crc.to_le_bytes());
    }
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; HEADER + STRIDE];
        for (offset, value) in [
            (0, 0x46447311u32),
            (8, b.len() as u32),
            (48, 0x4644610e),
            (52, 1),
            (56, STRIDE as u32),
        ] {
            b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        b[HEADER + 16..HEADER + 20].copy_from_slice(b"TEST");
        seal(&mut b);
        b
    }
    #[test]
    fn second_envelope_is_verified_but_not_misindexed_as_laser() {
        let mut b = fixture();
        b[..4].copy_from_slice(&0x46447312u32.to_le_bytes());
        b[48..52].copy_from_slice(&0x4644610fu32.to_le_bytes());
        seal(&mut b);
        let v = verify(&b).unwrap();
        assert_eq!(v.signature, 0x46447312);
        assert!(inspect(&b).is_err());
        for i in 0..b.len() {
            let mut corrupt = b.clone();
            corrupt[i] ^= 1;
            assert!(verify(&corrupt).is_err());
        }
        b[48..52].copy_from_slice(&0x4644610eu32.to_le_bytes());
        seal(&mut b);
        assert!(verify(&b).is_err());
    }
    #[test]
    fn authenticates_declared_bytes_and_indexes_without_optical_inference() {
        let mut b = fixture();
        let index = inspect(&b).unwrap();
        assert_eq!(
            index.records,
            vec![FactoryRecord {
                offset: HEADER,
                bytes: STRIDE,
                label: "TEST".into()
            }]
        );
        b.resize(LIMIT, 255);
        assert_eq!(inspect(&b).unwrap(), index);
        b[LIMIT - 1] = 3; // padding is explicitly outside the checksum.
        assert_eq!(inspect(&b).unwrap(), index);
    }
    #[test]
    fn rejects_every_truncation_and_single_byte_corruption() {
        let b = fixture();
        for end in 0..b.len() {
            assert!(inspect(&b[..end]).is_err());
        }
        for i in 0..b.len() {
            let mut corrupt = b.clone();
            corrupt[i] ^= 1;
            assert!(inspect(&corrupt).is_err(), "byte {i}");
        }
        assert!(inspect(&vec![0; LIMIT + 1]).is_err());
    }
    #[test]
    fn valid_crc_does_not_bypass_structural_bounds() {
        for (offset, value) in [(52, 0), (52, u32::MAX), (56, 0), (56, u32::MAX), (48, 0)] {
            let mut b = fixture();
            b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            seal(&mut b);
            assert!(inspect(&b).is_err());
        }
        let mut b = fixture();
        b[HEADER + 16] = 0xff;
        seal(&mut b);
        assert!(inspect(&b).is_err());
        for size in [0u32, 59, u32::MAX] {
            b[8..12].copy_from_slice(&size.to_le_bytes());
            assert!(inspect(&b).is_err());
        }
    }
}
