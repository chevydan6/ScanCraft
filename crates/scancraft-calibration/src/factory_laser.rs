//! Raw fields of the observed 1680-byte laser records. This does not create a
//! usable calibration: sensor assignment, mode binding, units and the laser
//! coefficient formula still need independent verification.
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct LaserLensFields {
    /// fx, fy, cx, cy in serializer order, without image-mode binding.
    pub intrinsic_values: [f32; 4],
    pub width: u16,
    pub height: u16,
    /// Preserved verbatim. No distortion convention is selected here.
    pub coefficient_values: [f32; 8],
}
#[derive(Debug, Serialize, PartialEq)]
pub struct LaserTransformFields {
    /// Nine consecutive floats; matrix orientation/direction not assigned.
    pub matrix_values: [f32; 9],
    /// Raw factory units, deliberately not millimeters.
    pub translation_values: [f32; 3],
}
#[derive(Debug, Serialize, PartialEq)]
pub struct LaserRecordFields {
    pub label: String,
    pub record_offset: usize,
    pub image_size_fields: [[u16; 2]; 2],
    pub lenses: [LaserLensFields; 3],
    pub transforms: [LaserTransformFields; 2],
    /// Serializer copies 1280 bytes here. Ten-float grouping is suggested by
    /// the setter ABI, but the numerical formula and line identities are unknown.
    pub laser_coefficient_values: [[f32; 10]; 32],
    /// Row-major E and F as serialized by the preprocessing routine. These
    /// establish numerical conventions, not physical sensor/mode association.
    pub stored_essential: [[f32; 3]; 3],
    pub stored_fundamental: [[f32; 3]; 3],
}
#[derive(Debug, Serialize)]
pub struct EpipolarFieldComparison {
    pub computed_raw_fields: crate::epipolar::EpipolarMatrices,
    pub essential_max_abs_difference: f64,
    pub fundamental_max_abs_difference: f64,
}
impl LaserRecordFields {
    /// Compare redundant serialized E/F fields with their numerical formula.
    /// A small residual is internal consistency, not calibration qualification.
    pub fn compare_epipolar_fields(&self) -> Result<EpipolarFieldComparison, String> {
        let computed_raw_fields = crate::epipolar::raw_field_matrices(
            std::array::from_fn(|i| self.lenses[i].intrinsic_values.map(f64::from)),
            std::array::from_fn(|i| {
                std::array::from_fn(|j| f64::from(self.transforms[0].matrix_values[i * 3 + j]))
            }),
            self.transforms[0].translation_values.map(f64::from),
        )?;
        let difference = |computed: &[[f64; 3]; 3], stored: &[[f32; 3]; 3]| {
            computed
                .iter()
                .flatten()
                .zip(stored.iter().flatten())
                .map(|(a, b)| (a - f64::from(*b)).abs())
                .fold(0f64, f64::max)
        };
        Ok(EpipolarFieldComparison {
            essential_max_abs_difference: difference(
                &computed_raw_fields.essential,
                &self.stored_essential,
            ),
            fundamental_max_abs_difference: difference(
                &computed_raw_fields.fundamental,
                &self.stored_fundamental,
            ),
            computed_raw_fields,
        })
    }
}
fn size(bytes: &[u8], offset: usize) -> Result<[u16; 2], String> {
    let values = std::array::from_fn(|i| {
        u16::from_le_bytes(
            bytes[offset + 2 * i..offset + 2 * i + 2]
                .try_into()
                .unwrap(),
        )
    });
    if values.iter().any(|v| *v == 0 || *v > 8192) {
        return Err("invalid laser record dimensions".into());
    }
    Ok(values)
}
fn floats<const N: usize>(bytes: &[u8], offset: usize) -> Result<[f32; N], String> {
    let values = std::array::from_fn(|i| {
        f32::from_le_bytes(
            bytes[offset + 4 * i..offset + 4 * i + 4]
                .try_into()
                .unwrap(),
        )
    });
    if !values.iter().all(|v| v.is_finite()) {
        return Err("nonfinite laser record field".into());
    }
    Ok(values)
}
/// CRC verification and bounded record indexing precede every field access.
/// Unknown labels, nonzero reserved fields, and future layouts fail closed.
/// Historical opaque bytes at 0..16 and the tail at 0x670..0x690 are not
/// interpreted or dereferenced. No automatic CameraModel conversion exists.
pub fn inspect(bytes: &[u8]) -> Result<Vec<LaserRecordFields>, String> {
    crate::factory::inspect(bytes)?
        .records
        .into_iter()
        .map(|record| {
            if !matches!(record.label.as_str(), "LAS_L_SCAN" | "CROSS_LAS_L_SCAN") {
                return Err("unsupported laser record label".into());
            }
            let b = &bytes[record.offset..record.offset + record.bytes];
            if b[0x28..0x2c] != [0; 4] {
                return Err("unsupported laser record reserved field".into());
            }
            let lens = |offset| -> Result<LaserLensFields, String> {
                let intrinsic_values = floats(b, offset)?;
                if intrinsic_values[0] <= 0. || intrinsic_values[1] <= 0. {
                    return Err("invalid laser record focal lengths".into());
                }
                let [width, height] = size(b, offset + 16)?;
                Ok(LaserLensFields {
                    intrinsic_values,
                    width,
                    height,
                    coefficient_values: floats(b, offset + 20)?,
                })
            };
            let transform = |offset| -> Result<LaserTransformFields, String> {
                Ok(LaserTransformFields {
                    matrix_values: floats(b, offset)?,
                    translation_values: floats(b, offset + 36)?,
                })
            };
            let mut laser_coefficient_values = [[0.; 10]; 32];
            for (i, values) in laser_coefficient_values.iter_mut().enumerate() {
                *values = floats(b, 0x128 + i * 40)?;
            }
            Ok(LaserRecordFields {
                label: record.label,
                record_offset: record.offset,
                image_size_fields: [size(b, 0x20)?, size(b, 0x24)?],
                lenses: [lens(0x2c)?, lens(0x60)?, lens(0x94)?],
                transforms: [transform(0xc8)?, transform(0xf8)?],
                laser_coefficient_values,
                stored_essential: [floats(b, 0x628)?, floats(b, 0x634)?, floats(b, 0x640)?],
                stored_fundamental: [floats(b, 0x64c)?, floats(b, 0x658)?, floats(b, 0x664)?],
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seal(b: &mut [u8]) {
        b[4..8].fill(0);
        let crc = crc32fast::hash(b);
        b[4..8].copy_from_slice(&crc.to_le_bytes());
    }
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; 60 + 1680];
        for (offset, value) in [
            (0, 0x46447311u32),
            (8, 1740),
            (48, 0x4644610e),
            (52, 1),
            (56, 1680),
        ] {
            b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        b[76..86].copy_from_slice(b"LAS_L_SCAN");
        // Copy ranges independently verified by the synthetic reference probe.
        for (destination, source, len) in [
            (0x2c, 0, 52),
            (0x60, 0x34, 52),
            (0x94, 0x98, 52),
            (0xc8, 0x68, 48),
            (0xf8, 0xcc, 48),
            (0x128, 0xfc, 1280),
        ] {
            for i in 0..len / 4 {
                let value = (1000 + source / 4 + i) as f32;
                let at = 60 + destination + i * 4;
                b[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        for offset in [0x20, 0x24, 0x3c, 0x70, 0xa4] {
            b[60 + offset..60 + offset + 4].copy_from_slice(&[64, 0, 32, 0]);
        }
        for i in 0..18 {
            let at = 60 + 0x628 + i * 4;
            b[at..at + 4].copy_from_slice(&(2000. + i as f32).to_le_bytes());
        }
        seal(&mut b);
        b
    }
    #[test]
    fn synthetic_reference_copy_layout_preserves_all_decoded_fields() {
        let b = fixture();
        let r = inspect(&b).unwrap().remove(0);
        assert_eq!(r.image_size_fields, [[64, 32]; 2]);
        for (lens, source) in r.lenses.iter().zip([0, 0x34, 0x98]) {
            assert_eq!((lens.width, lens.height), (64, 32));
            for i in 0..4 {
                assert_eq!(lens.intrinsic_values[i], (1000 + source / 4 + i) as f32);
            }
            for i in 0..8 {
                assert_eq!(
                    lens.coefficient_values[i],
                    (1000 + source / 4 + 5 + i) as f32
                );
            }
        }
        for (transform, source) in r.transforms.iter().zip([0x68, 0xcc]) {
            for i in 0..9 {
                assert_eq!(transform.matrix_values[i], (1000 + source / 4 + i) as f32);
            }
            for i in 0..3 {
                assert_eq!(
                    transform.translation_values[i],
                    (1000 + source / 4 + 9 + i) as f32
                );
            }
        }
        for i in 0..32 {
            for j in 0..10 {
                assert_eq!(
                    r.laser_coefficient_values[i][j],
                    (1000 + 0xfc / 4 + i * 10 + j) as f32
                );
            }
        }
        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(r.stored_essential[i][j], 2000. + (i * 3 + j) as f32);
                assert_eq!(r.stored_fundamental[i][j], 2009. + (i * 3 + j) as f32);
            }
        }
        assert!(
            r.compare_epipolar_fields()
                .unwrap()
                .essential_max_abs_difference
                .is_finite()
        );
    }
    #[test]
    fn rejects_truncation_corruption_and_semantically_invalid_fields() {
        let b = fixture();
        for end in 0..b.len() {
            assert!(inspect(&b[..end]).is_err());
        }
        for i in 0..b.len() {
            let mut bad = b.clone();
            bad[i] ^= 1;
            assert!(inspect(&bad).is_err());
        }
        for (offset, value) in [
            (0x20, 0u32),
            (0x28, 1),
            (0x2c, (-1f32).to_bits()),
            (0x44, f32::NAN.to_bits()),
            (0xc8, f32::INFINITY.to_bits()),
            (0x624, f32::NEG_INFINITY.to_bits()),
            (0x66c, f32::NAN.to_bits()),
        ] {
            let mut bad = b.clone();
            bad[60 + offset..64 + offset].copy_from_slice(&value.to_le_bytes());
            seal(&mut bad);
            assert!(inspect(&bad).is_err(), "{offset:x}");
        }
    }
}
