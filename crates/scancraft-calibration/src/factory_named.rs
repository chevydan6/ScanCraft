//! Lens blocks from the observed 3.0.13 named-camera payload. This is a raw
//! field decoder, not a stereo calibration: coefficient conventions, pose
//! association, translation units and image-mode binding remain unverified.
use serde::Serialize;

const DECLARED: usize = 6646;
const STRIDE: usize = 2178;
#[derive(Debug, Serialize, PartialEq)]
pub struct NamedLens {
    pub label: String,
    pub width: u32,
    pub height: u32,
    /// Four floats preserved in file order; not converted to CameraModel.
    pub intrinsic_values: [f32; 4],
    /// Eight Brown-model floats preserved in file order; not converted to Brown5.
    pub brown_values: [f32; 8],
    /// Values exported by the reference as Rotate.RX/RY/RZ. Angle convention
    /// and transform direction remain unverified; not a rotation matrix.
    pub rotation_fields: [f32; 3],
    /// Values exported as Translate.TX/TY/TZ. Units remain unverified.
    pub translation_fields: [f32; 3],
}
/// Supports exactly the observed version, count, stride and Brown lens schema.
/// Embedded historical process-pointer/padding bytes are never dereferenced.
pub fn inspect(bytes: &[u8]) -> Result<Vec<NamedLens>, String> {
    let envelope = crate::factory::verify(bytes)?;
    if envelope.signature != 0x46447312 || envelope.declared_bytes != DECLARED {
        return Err("unsupported named-camera envelope".into());
    }
    let p = &bytes[60..DECLARED];
    if bytes[52..56] != 1u32.to_le_bytes()
        || bytes[56..60] != 6586u32.to_le_bytes()
        || p[0] != 1
        || &p[1..11] != b"3.0.13\0\0\0\0"
        || p[48] != 3
    {
        return Err("unsupported named-camera version/count/layout".into());
    }
    let mut lenses = Vec::with_capacity(3);
    for (i, expected) in ["IR_L", "IR_R", "COLOR"].into_iter().enumerate() {
        let base = i * STRIDE;
        let label = &p[base + 50..base + 62];
        let end = label
            .iter()
            .position(|b| *b == 0)
            .ok_or("unterminated camera label")?;
        if &label[..end] != expected.as_bytes() || &p[base + 72..base + 78] != b"Brown\0" {
            return Err("unsupported named-camera label/lens model".into());
        }
        let u32_at =
            |offset| u32::from_le_bytes(p[base + offset..base + offset + 4].try_into().unwrap());
        let height = u32_at(64);
        let width = u32_at(68);
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err("invalid factory camera dimensions".into());
        }
        let intrinsic_values = std::array::from_fn(|j| f32::from_bits(u32_at(104 + j * 4)));
        let brown_values = std::array::from_fn(|j| f32::from_bits(u32_at(120 + j * 4)));
        let rotation_fields = std::array::from_fn(|j| f32::from_bits(u32_at(2203 + j * 4)));
        let translation_fields = std::array::from_fn(|j| f32::from_bits(u32_at(2215 + j * 4)));
        if !intrinsic_values
            .iter()
            .chain(&brown_values)
            .chain(&rotation_fields)
            .chain(&translation_fields)
            .all(|v| v.is_finite())
            || intrinsic_values[0] <= 0.
            || intrinsic_values[1] <= 0.
        {
            return Err("invalid factory lens values".into());
        }
        lenses.push(NamedLens {
            label: expected.into(),
            width,
            height,
            intrinsic_values,
            brown_values,
            rotation_fields,
            translation_fields,
        });
    }
    Ok(lenses)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seal(b: &mut [u8]) {
        b[4..8].fill(0);
        let crc = crc32fast::hash(&b[..DECLARED]);
        b[4..8].copy_from_slice(&crc.to_le_bytes());
    }
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; DECLARED];
        for (offset, value) in [
            (0, 0x46447312u32),
            (8, DECLARED as u32),
            (48, 0x4644610f),
            (52, 1),
            (56, 6586),
        ] {
            b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        b[60] = 1;
        b[61..71].copy_from_slice(b"3.0.13\0\0\0\0");
        b[108] = 3;
        for (i, label) in [b"IR_L".as_slice(), b"IR_R", b"COLOR"]
            .into_iter()
            .enumerate()
        {
            let base = 60 + i * STRIDE;
            b[base + 50..base + 50 + label.len()].copy_from_slice(label);
            b[base + 72..base + 78].copy_from_slice(b"Brown\0");
            for (off, value) in [
                (64, 32u32),
                (68, 64),
                (104, 100f32.to_bits()),
                (108, 101f32.to_bits()),
            ] {
                b[base + off..base + off + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        seal(&mut b);
        b
    }
    #[test]
    fn decodes_named_lens_values_without_creating_stereo_calibration() {
        let b = fixture();
        let lenses = inspect(&b).unwrap();
        assert_eq!(lenses.len(), 3);
        assert_eq!(lenses[1].label, "IR_R");
        assert_eq!((lenses[0].width, lenses[0].height), (64, 32));
        assert_eq!(lenses[0].intrinsic_values, [100., 101., 0., 0.]);
        assert_eq!(lenses[2].brown_values, [0.; 8]);
    }
    #[test]
    fn refuses_unknown_versions_models_labels_and_nonfinite_lenses_even_with_crc() {
        for (offset, value) in [(61, b'4'), (108, 4), (110, b'X'), (132, b'K')] {
            let mut b = fixture();
            b[offset] = value;
            seal(&mut b);
            assert!(inspect(&b).is_err());
        }
        for (offset, value) in [
            (124, 0u32),
            (128, 8193),
            (164, 0),
            (168, f32::INFINITY.to_bits()),
            (180, f32::NAN.to_bits()),
        ] {
            let mut b = fixture();
            b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            seal(&mut b);
            assert!(inspect(&b).is_err());
        }
        let mut b = fixture();
        b[110..122].fill(b'X');
        seal(&mut b);
        assert!(inspect(&b).is_err());
    }
    #[test]
    fn pose_fields_preserve_file_order_and_signed_zero_without_interpreting_angles() {
        let mut b = fixture();
        for i in 0..3 {
            let values = [-0.0f32, 0.2 + i as f32, -0.3, 100. + i as f32, -2., 3.];
            for (j, value) in values.iter().enumerate() {
                let off = 60 + i * STRIDE + 2203 + j * 4;
                b[off..off + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        seal(&mut b);
        let lenses = inspect(&b).unwrap();
        for (i, lens) in lenses.iter().enumerate() {
            assert_eq!(lens.rotation_fields[0].to_bits(), (-0.0f32).to_bits());
            assert_eq!(lens.rotation_fields[1], 0.2 + i as f32);
            assert_eq!(lens.rotation_fields[2], -0.3);
            assert_eq!(lens.translation_fields, [100. + i as f32, -2., 3.]);
        }
        for i in 0..3 {
            for j in 0..6 {
                let mut bad = b.clone();
                let off = 60 + i * STRIDE + 2203 + j * 4;
                bad[off..off + 4].copy_from_slice(&f32::NAN.to_le_bytes());
                seal(&mut bad);
                assert!(inspect(&bad).is_err());
            }
        }
    }
    #[test]
    fn rejects_all_truncations_and_ignores_opaque_pointer_bytes() {
        let mut b = fixture();
        for end in 0..b.len() {
            assert!(inspect(&b[..end]).is_err());
        }
        let expected = inspect(&b).unwrap();
        b[60 + 88..60 + 96].fill(255);
        seal(&mut b);
        assert_eq!(inspect(&b).unwrap(), expected);
    }
}
