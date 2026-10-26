//! Checked reusable world-mm to local-mm coordinate frames.
use super::{
    Error, artifact, bounded,
    output::{publish, staging},
};
use scancraft_geometry::{coordinate_frame::Frame, work::WorkControl};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};
const MAGIC: &[u8; 8] = b"RAPFRM01";
const MAX_PAYLOAD: usize = 16 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub schema_version: u32,
    pub units: String,
    pub mapping: String,
    pub label: String,
    pub frame: Frame,
    pub physical_accuracy_qualified: bool,
}
impl Document {
    pub fn new(label: String, frame: Frame) -> Result<Self, Error> {
        let d = Self {
            schema_version: 1,
            units: "millimeters".into(),
            mapping: "world_to_local".into(),
            label,
            frame,
            physical_accuracy_qualified: false,
        };
        d.validate()?;
        Ok(d)
    }
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema_version != 1
            || self.units != "millimeters"
            || self.mapping != "world_to_local"
            || self.physical_accuracy_qualified
            || self.label.trim().is_empty()
            || self.label.len() > 128
            || self.label.chars().any(char::is_control)
        {
            return Err("unsupported or invalid coordinate-frame document".into());
        }
        self.frame.validate()?;
        Ok(())
    }
}
pub fn save(document: &Document, target: &Path, work: &WorkControl) -> Result<(), Error> {
    work.check()?;
    document.validate()?;
    if fs::symlink_metadata(target).is_ok() {
        return Err("coordinate-frame output already exists".into());
    }
    let payload = serde_json::to_vec(document)?;
    if payload.len() > MAX_PAYLOAD {
        return Err("coordinate-frame payload exceeds byte budget".into());
    }
    let partial = staging(target)?;
    let result = (|| -> Result<(), Error> {
        artifact(&partial, "frame.rapframe", work, |out| {
            out.write_all(MAGIC)?;
            out.write_all(&(payload.len() as u32).to_le_bytes())?;
            out.write_all(&crc32fast::hash(&payload).to_le_bytes())?;
            out.write_all(&payload)?;
            Ok(())
        })?;
        File::open(&partial)?.sync_all()?;
        work.check()?;
        publish(&partial, target)?;
        Ok(())
    })();
    result.map_err(|e| {
        format!(
            "{e}; incomplete coordinate-frame staging retained at {}",
            partial.display()
        )
    })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "coordinate frame published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(())
}
pub fn load(path: &Path, work: &WorkControl) -> Result<Document, Error> {
    let bytes = bounded(path, MAX_PAYLOAD + 16, work)?;
    if bytes.len() < 16 || &bytes[..8] != MAGIC {
        return Err("invalid coordinate-frame header".into());
    }
    let length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if length > MAX_PAYLOAD
        || bytes.len() != length + 16
        || crc32fast::hash(&bytes[16..]) != u32::from_le_bytes(bytes[12..16].try_into().unwrap())
    {
        return Err("coordinate-frame length/CRC mismatch".into());
    }
    let document: Document = serde_json::from_slice(&bytes[16..])?;
    document.validate()?;
    work.check()?;
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    static NEXT_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    struct Temp(std::path::PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "scancraft-frame-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn document() -> Document {
        Document::new(
            "Fixture frame".into(),
            Frame::from_points([10., 20., 30.], [10., 22., 30.], [9., 21., 30.]).unwrap(),
        )
        .unwrap()
    }
    fn saved(temp: &Temp) -> std::path::PathBuf {
        let target = temp.0.join("bundle");
        save(&document(), &target, &WorkControl::default()).unwrap();
        target.join("frame.rapframe")
    }
    #[test]
    fn frame_roundtrip_preserves_exact_parameters_and_no_overwrite() {
        let temp = Temp::new();
        let path = saved(&temp);
        let before = fs::read(&path).unwrap();
        let loaded = load(&path, &WorkControl::default()).unwrap();
        assert_eq!(loaded.label, "Fixture frame");
        assert_eq!(loaded.frame.origin_mm, document().frame.origin_mm);
        assert_eq!(loaded.frame.axes, document().frame.axes);
        assert!(save(&document(), path.parent().unwrap(), &WorkControl::default()).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(load(&path, &cancelled).is_err());
        let target = temp.0.join("cancelled");
        assert!(save(&document(), &target, &cancelled).is_err());
        assert!(!target.exists());
    }
    #[test]
    fn crc_header_length_and_trailing_corruptions_fail() {
        let temp = Temp::new();
        let path = saved(&temp);
        let original = fs::read(&path).unwrap();
        for kind in 0..4 {
            let mut bad = original.clone();
            match kind {
                0 => bad[16] ^= 1,
                1 => bad.push(0),
                2 => bad[0] ^= 1,
                _ => bad.truncate(10),
            };
            fs::write(&path, &bad).unwrap();
            assert!(load(&path, &WorkControl::default()).is_err());
        }
    }
    #[test]
    fn crc_valid_invalid_units_mapping_rotation_and_labels_are_rejected() {
        let temp = Temp::new();
        let path = saved(&temp);
        let original = fs::read(&path).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&original[16..]).unwrap();
        for kind in 0..8 {
            let mut changed = value.clone();
            match kind {
                0 => changed["units"] = serde_json::json!("inches"),
                1 => changed["mapping"] = serde_json::json!("local_to_world"),
                2 => changed["physical_accuracy_qualified"] = serde_json::json!(true),
                3 => changed["label"] = serde_json::json!("\n"),
                4 => {
                    changed["frame"]["axes"] =
                        serde_json::json!([[-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]])
                }
                5 => changed["frame"]["unknown"] = serde_json::json!(1),
                6 => changed["schema_version"] = serde_json::json!(2),
                _ => {
                    changed["frame"]["axes"] = serde_json::json!([
                        [f64::MAX, f64::MAX, f64::MAX],
                        [f64::MAX, -f64::MAX, f64::MAX],
                        [f64::MAX, f64::MAX, -f64::MAX]
                    ])
                }
            };
            let payload = serde_json::to_vec(&changed).unwrap();
            let mut bytes = MAGIC.to_vec();
            bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&crc32fast::hash(&payload).to_le_bytes());
            bytes.extend(payload);
            fs::write(&path, &bytes).unwrap();
            assert!(load(&path, &WorkControl::default()).is_err(), "case {kind}");
        }
    }
}
