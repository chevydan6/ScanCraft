//! Bounded integrity-checked calibration records. Provenance never implies physical qualification.
use crate::stereo::StereoCalibration;
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    SyntheticFixture,
    EstimatedFromObservations,
    DeviceReadout,
    Imported,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceBinding {
    pub serial: String,
    pub firmware: String,
    pub mode: String,
    pub left_size: [u32; 2],
    pub right_size: [u32; 2],
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CalibrationRecord {
    pub schema_version: u32,
    pub record_id: String,
    pub units: String,
    pub camera_axes: String,
    pub provenance: Provenance,
    pub binding: Option<DeviceBinding>,
    pub geometry: StereoCalibration,
    pub notes: String,
}
impl CalibrationRecord {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.units != "millimeters"
            || self.camera_axes != "x_right_y_down_z_forward"
        {
            return Err("unsupported calibration version, units or axis convention".into());
        }
        if self.record_id.is_empty() || self.record_id.len() > 128 || self.notes.len() > 4096 {
            return Err("invalid calibration record metadata".into());
        }
        self.geometry.validate()?;
        if let Some(b) = &self.binding
            && ([&b.serial, &b.firmware, &b.mode]
                .iter()
                .any(|v| v.is_empty() || v.len() > 256)
                || b.left_size != [self.geometry.left.width, self.geometry.left.height]
                || b.right_size != [self.geometry.right.width, self.geometry.right.height])
        {
            return Err("calibration binding disagrees with image geometry".into());
        }
        Ok(())
    }
    pub fn validate_for_capture(&self, actual: &DeviceBinding) -> Result<(), String> {
        self.validate()?;
        if self.provenance == Provenance::SyntheticFixture {
            return Err("synthetic calibration is not permitted for physical capture".into());
        }
        if self.binding.as_ref() != Some(actual) {
            return Err("calibration does not match serial, firmware, mode and image sizes".into());
        }
        Ok(())
    }
}
const MAGIC: &[u8; 8] = b"RAPCAL01";
const LIMIT: usize = 1024 * 1024;
fn invalid(s: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s.into())
}
pub fn write(record: &CalibrationRecord, mut output: impl Write) -> io::Result<()> {
    record.validate().map_err(invalid)?;
    let data = serde_json::to_vec(record)?;
    if data.len() > LIMIT {
        return Err(invalid("calibration record exceeds 1 MiB"));
    }
    output.write_all(MAGIC)?;
    output.write_all(&(data.len() as u32).to_le_bytes())?;
    output.write_all(&crc32fast::hash(&data).to_le_bytes())?;
    output.write_all(&data)?;
    output.flush()
}
pub fn read(mut input: impl Read) -> io::Result<CalibrationRecord> {
    let mut header = [0; 16];
    input.read_exact(&mut header)?;
    if &header[..8] != MAGIC {
        return Err(invalid("unknown calibration format"));
    }
    let n = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
    if n > LIMIT {
        return Err(invalid("calibration record exceeds 1 MiB"));
    }
    let crc = u32::from_le_bytes(header[12..16].try_into().unwrap());
    let mut data = vec![0; n];
    input.read_exact(&mut data)?;
    if crc32fast::hash(&data) != crc {
        return Err(invalid("calibration checksum mismatch"));
    }
    let mut extra = [0];
    if input.read(&mut extra)? != 0 {
        return Err(invalid("trailing calibration bytes"));
    }
    let record: CalibrationRecord = serde_json::from_slice(&data)?;
    record.validate().map_err(invalid)?;
    Ok(record)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn record() -> CalibrationRecord {
        CalibrationRecord {
            schema_version: 1,
            record_id: "synthetic-unit-fixture".into(),
            units: "millimeters".into(),
            camera_axes: "x_right_y_down_z_forward".into(),
            provenance: Provenance::SyntheticFixture,
            binding: None,
            geometry: crate::stereo::fixtures::rig(),
            notes: "Synthetic data only; not scanner calibration".into(),
        }
    }
    #[test]
    fn exact_roundtrip_and_every_truncation_or_byte_corruption_rejected() {
        let record = record();
        let mut b = Vec::new();
        write(&record, &mut b).unwrap();
        assert_eq!(read(b.as_slice()).unwrap(), record);
        for n in 0..b.len() {
            assert!(read(&b[..n]).is_err());
            let mut corrupt = b.clone();
            corrupt[n] ^= 1;
            assert!(read(corrupt.as_slice()).is_err());
        }
    }
    #[test]
    fn synthetic_and_mismatched_device_bindings_cannot_pass_capture_checks() {
        let mut r = record();
        let binding = DeviceBinding {
            serial: "test-device".into(),
            firmware: "test-firmware".into(),
            mode: "test-mode".into(),
            left_size: [1280, 800],
            right_size: [1280, 800],
        };
        r.binding = Some(binding.clone());
        assert!(r.validate_for_capture(&binding).is_err());
        r.provenance = Provenance::Imported;
        r.validate_for_capture(&binding).unwrap();
        let mut other = binding;
        other.firmware = "changed".into();
        assert!(r.validate_for_capture(&other).is_err());
    }
    #[test]
    fn unsupported_units_and_invalid_geometry_fail_before_writing() {
        let mut r = record();
        r.units = "inches".into();
        assert!(write(&r, Vec::new()).is_err());
        r.units = "millimeters".into();
        r.geometry.left.fx = f64::NAN;
        assert!(write(&r, Vec::new()).is_err());
    }
}
