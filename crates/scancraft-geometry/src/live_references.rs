//! Initial live-session lineage, not per-point correspondence or hardware attestation.
//! Metadata validation never follows external paths. CRC detects corruption, not authenticity.
use crate::sequence_references::{Artifact, Frame};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveSession {
    pub schema_version: u32,
    pub directory_display: String,
    pub session_id: String,
    pub synthetic: bool,
    pub recorded_frames: [usize; 2],
    pub pairs: Vec<Frame>,
    pub artifacts: Vec<Artifact>,
    pub source_device_binding_verified: bool,
    pub synchronization_verified: bool,
    pub physical_accuracy_qualified: bool,
}
impl LiveSession {
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
            || self.recorded_frames.iter().any(|n| !(1..=1000).contains(n))
            || !(1..=1000).contains(&self.pairs.len())
            || !self.pairs.iter().any(|p| p.integrated)
            || self.source_device_binding_verified
            || self.synchronization_verified
            || self.physical_accuracy_qualified
        {
            return Err("invalid or qualified live-session references".into());
        }
        for (index, pair) in self.pairs.iter().enumerate() {
            if pair.id != index as u64
                || pair.left_index >= self.recorded_frames[0]
                || pair.right_index >= self.recorded_frames[1]
                || (index > 0
                    && (pair.left_index <= self.pairs[index - 1].left_index
                        || pair.right_index <= self.pairs[index - 1].right_index))
            {
                return Err("invalid live-session pair lineage".into());
            }
        }
        // No self-reference: project and outer manifest are deliberately excluded.
        let mut expected: std::collections::HashSet<String> = [
            "calibration.rapcal",
            "request.json",
            "scan-request.json",
            "left.raptor",
            "right.raptor",
            "report.json",
            "points.ply",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        for index in 0..self.pairs.len() {
            expected.insert(format!("pair-{index:04}.json"));
        }
        if self.artifacts.len() != expected.len() {
            return Err("incomplete live-session artifact references".into());
        }
        for artifact in &self.artifacts {
            if artifact.bytes == 0
                || artifact.bytes > 1_u64 << 40
                || !expected.remove(&artifact.name)
            {
                return Err("invalid, unsafe or duplicate live-session artifact".into());
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 1024 * 1024 {
            return Err("live-session references exceed 1 MiB".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> LiveSession {
        let mut artifacts: Vec<_> = [
            "calibration.rapcal",
            "request.json",
            "scan-request.json",
            "left.raptor",
            "right.raptor",
            "report.json",
            "points.ply",
            "pair-0000.json",
        ]
        .into_iter()
        .map(|name| Artifact {
            name: name.into(),
            bytes: 10,
            crc32: 0,
        })
        .collect();
        artifacts.sort_by(|a, b| a.name.cmp(&b.name));
        LiveSession {
            schema_version: 1,
            directory_display: "missing-live-bundle".into(),
            session_id: "synthetic-live".into(),
            synthetic: true,
            recorded_frames: [3, 4],
            pairs: vec![Frame {
                id: 0,
                left_index: 1,
                right_index: 2,
                integrated: true,
            }],
            artifacts,
            source_device_binding_verified: false,
            synchronization_verified: false,
            physical_accuracy_qualified: false,
        }
    }
    #[test]
    fn rejects_incomplete_unsafe_duplicate_and_unfounded_qualified_lineage() {
        let good = fixture();
        good.validate().unwrap();
        for name in [
            "../escape",
            "project.orproj",
            "manifest.json",
            "pair-0001.json",
            "/absolute",
        ] {
            let mut bad = good.clone();
            bad.artifacts[0].name = name.into();
            assert!(bad.validate().is_err());
        }
        let mut bad = good.clone();
        bad.artifacts.pop();
        assert!(bad.validate().is_err());
        let mut bad = good.clone();
        bad.artifacts[0] = bad.artifacts[1].clone();
        assert!(bad.validate().is_err());
        let mut bad = good.clone();
        bad.physical_accuracy_qualified = true;
        assert!(bad.validate().is_err());
        let mut bad = good.clone();
        bad.pairs[0].left_index = 3;
        assert!(bad.validate().is_err());
        let mut bad = good.clone();
        bad.pairs[0].id = 1;
        assert!(bad.validate().is_err());
        let mut bad = good.clone();
        bad.pairs[0].integrated = false;
        assert!(bad.validate().is_err());
        let mut bad = good;
        bad.recorded_frames[1] = 1001;
        assert!(bad.validate().is_err());
    }
    #[test]
    fn version8_retains_lineage_through_edits_and_reopen_without_opening_sources() {
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
        let lineage = fixture();
        editor.attach_live_session(lineage.clone()).unwrap();
        assert!(editor.attach_live_session(lineage.clone()).is_err());
        editor.trim(&[0], false).unwrap();
        assert!(editor.undo());
        assert!(editor.redo());
        let mut encoded = Vec::new();
        crate::formats::save_project(&editor, &mut encoded).unwrap();
        assert_eq!(&encoded[..8], b"ORPROJ08");
        let mut loaded = crate::formats::load_project(encoded.as_slice()).unwrap();
        assert_eq!(loaded.live_session(), Some(&lineage));
        assert!(loaded.undo());
        assert_eq!(loaded.cloud.points.len(), 1);
        assert_eq!(loaded.live_session(), Some(&lineage));
        let value: serde_json::Value = serde_json::from_slice(&encoded[16..]).unwrap();
        for version in [2, 7, 8] {
            let mut corrupt = value.clone();
            corrupt["version"] = version.into();
            if version == 8 {
                corrupt["editor"]
                    .as_object_mut()
                    .unwrap()
                    .remove("live_session");
            }
            let payload = serde_json::to_vec(&corrupt).unwrap();
            let mut bytes = format!("ORPROJ{version:02}").into_bytes();
            bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&crc32fast::hash(&payload).to_le_bytes());
            bytes.extend(payload);
            assert!(crate::formats::load_project(bytes.as_slice()).is_err());
        }
        let plain = crate::Editor::new(crate::Cloud { points: vec![] }, 8).unwrap();
        let mut bytes = Vec::new();
        crate::formats::save_project(&plain, &mut bytes).unwrap();
        assert_eq!(&bytes[..8], b"ORPROJ02");
    }
}
