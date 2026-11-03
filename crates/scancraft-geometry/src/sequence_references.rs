//! Initial sequence lineage and external artifact fingerprints, not current
//! per-point correspondence. Validation never opens external files or attests
//! device identity, clock synchronization or physical accuracy.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub id: u64,
    pub left_index: usize,
    pub right_index: usize,
    pub integrated: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub name: String,
    pub bytes: u64,
    pub crc32: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sequence {
    pub schema_version: u32,
    pub directory_display: String,
    pub session_id: String,
    pub source_kind: String,
    pub calibration_record_id: String,
    pub frames: Vec<Frame>,
    pub artifacts: Vec<Artifact>,
    pub source_device_binding_verified: bool,
    pub synchronization_verified: bool,
    pub physical_accuracy_qualified: bool,
}
impl Sequence {
    /// Bounded metadata and fixed safe artifact names only; CRCs describe bytes
    /// captured by the producer, not cryptographic authenticity or current disk.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.directory_display.is_empty()
            || self.directory_display.len() > 4096
            || self.session_id.is_empty()
            || self.session_id.len() > 128
            || !self
                .session_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_ .".contains(&b))
            || !["synthetic_fixture", "declared_observations"].contains(&self.source_kind.as_str())
            || self.calibration_record_id.is_empty()
            || self.calibration_record_id.len() > 128
            || !(1..=1000).contains(&self.frames.len())
            || self.frames.windows(2).any(|p| {
                p[0].id >= p[1].id
                    || p[0].left_index >= p[1].left_index
                    || p[0].right_index >= p[1].right_index
            })
            || self
                .frames
                .iter()
                .any(|f| f.left_index >= 1_000_000 || f.right_index >= 1_000_000)
            || self.source_device_binding_verified
            || self.synchronization_verified
            || self.physical_accuracy_qualified
        {
            return Err("invalid or qualified recorded-sequence references".into());
        }
        let mut expected: std::collections::HashSet<String> =
            ["sequence-request.json", "report.json", "points.ply"]
                .into_iter()
                .map(str::to_owned)
                .collect();
        for index in 0..self.frames.len() {
            for name in [
                "report.json",
                "left-frame.raptor",
                "right-frame.raptor",
                "calibration.rapcal",
                "request.json",
                "recording-sources.json",
                "points.ply",
                "manifest.json",
            ] {
                expected.insert(format!("frame-{index:04}/{name}"));
            }
        }
        if self.artifacts.len() != expected.len() {
            return Err("incomplete sequence artifact references".into());
        }
        for artifact in &self.artifacts {
            if artifact.bytes == 0
                || artifact.bytes > 1_u64 << 40
                || !expected.remove(&artifact.name)
            {
                return Err("invalid, unsafe or duplicate sequence artifact reference".into());
            }
        }
        // All strings/counts above are bounded. Serialization adds a separate
        // explicit format budget without reading any referenced artifact.
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 8 * 1024 * 1024 {
            return Err("sequence references exceed 8 MiB".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Sequence {
        let mut artifacts: Vec<_> = ["sequence-request.json", "report.json", "points.ply"]
            .into_iter()
            .map(|n| Artifact {
                name: n.into(),
                bytes: 10,
                crc32: 0,
            })
            .collect();
        for name in [
            "report.json",
            "left-frame.raptor",
            "right-frame.raptor",
            "calibration.rapcal",
            "request.json",
            "recording-sources.json",
            "points.ply",
            "manifest.json",
        ] {
            artifacts.push(Artifact {
                name: format!("frame-0000/{name}"),
                bytes: 10,
                crc32: 0,
            });
        }
        Sequence {
            schema_version: 1,
            directory_display: "missing-external-folder".into(),
            session_id: "synthetic".into(),
            source_kind: "synthetic_fixture".into(),
            calibration_record_id: "synthetic-calibration".into(),
            frames: vec![Frame {
                id: 1,
                left_index: 0,
                right_index: 0,
                integrated: true,
            }],
            artifacts,
            source_device_binding_verified: false,
            synchronization_verified: false,
            physical_accuracy_qualified: false,
        }
    }
    #[test]
    fn metadata_rejects_unsafe_incomplete_duplicate_and_qualified_references() {
        let good = fixture();
        good.validate().unwrap();
        for name in [
            "../escape",
            "/absolute",
            "frame-0000/../report.json",
            "frame-0001/report.json",
        ] {
            let mut bad = good.clone();
            bad.artifacts[0].name = name.into();
            assert!(bad.validate().is_err());
        }
        let mut bad = good.clone();
        bad.artifacts.pop();
        assert!(bad.validate().is_err());
        let mut bad = good.clone();
        bad.artifacts[1] = bad.artifacts[0].clone();
        assert!(bad.validate().is_err());
        let mut bad = good.clone();
        bad.physical_accuracy_qualified = true;
        assert!(bad.validate().is_err());
        let mut bad = good.clone();
        bad.frames[0].left_index = 1_000_000;
        assert!(bad.validate().is_err());
    }
    fn encode(mut value: serde_json::Value, magic: &[u8; 8], version: u32) -> Vec<u8> {
        value["version"] = version.into();
        let payload = serde_json::to_vec(&value).unwrap();
        let mut out = magic.to_vec();
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(&crc32fast::hash(&payload).to_le_bytes());
        out.extend_from_slice(&payload);
        out
    }
    #[test]
    fn version7_retains_references_through_history_without_external_io_and_rejects_downgrades() {
        let mut editor = crate::Editor::new(
            crate::Cloud {
                points: vec![crate::Point {
                    position: [1., 2., 3.],
                    color: None,
                    normal: None,
                }],
            },
            8,
        )
        .unwrap();
        let sequence = fixture();
        editor.attach_sequence(sequence.clone()).unwrap();
        assert!(editor.attach_sequence(sequence.clone()).is_err());
        editor.trim(&[0], false).unwrap();
        assert!(editor.undo());
        assert!(editor.redo());
        let mut bytes = Vec::new();
        crate::formats::save_project(&editor, &mut bytes).unwrap();
        assert_eq!(&bytes[..8], b"ORPROJ07");
        let mut loaded = crate::formats::load_project(bytes.as_slice()).unwrap();
        assert_eq!(loaded.sequence(), Some(&sequence));
        assert!(loaded.undo());
        assert_eq!(loaded.cloud.points.len(), 1);
        assert_eq!(loaded.sequence(), Some(&sequence));
        let value: serde_json::Value = serde_json::from_slice(&bytes[16..]).unwrap();
        assert!(
            crate::formats::load_project(encode(value.clone(), b"ORPROJ02", 2).as_slice()).is_err()
        );
        assert!(
            crate::formats::load_project(encode(value.clone(), b"ORPROJ06", 6).as_slice()).is_err()
        );
        let mut missing = value.clone();
        missing["editor"]
            .as_object_mut()
            .unwrap()
            .remove("sequence");
        assert!(crate::formats::load_project(encode(missing, b"ORPROJ07", 7).as_slice()).is_err());
        let mut qualified = value;
        qualified["editor"]["sequence"]["synchronization_verified"] = true.into();
        assert!(
            crate::formats::load_project(encode(qualified, b"ORPROJ07", 7).as_slice()).is_err()
        );
        let plain = crate::Editor::new(crate::Cloud { points: vec![] }, 8).unwrap();
        let mut legacy = Vec::new();
        crate::formats::save_project(&plain, &mut legacy).unwrap();
        assert_eq!(&legacy[..8], b"ORPROJ02");
        assert!(
            crate::formats::load_project(legacy.as_slice())
                .unwrap()
                .sequence()
                .is_none()
        );
    }
}
