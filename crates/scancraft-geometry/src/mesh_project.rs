//! Checksummed versioned mesh snapshots with exact bounded undo/redo history.
use crate::{mesh_edit::MeshEditor, work::WorkControl};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
pub const MAGIC: &[u8; 8] = b"MSHPRJ01";
pub const MAX_PAYLOAD: usize = 128 * 1024 * 1024;
struct Checked<'a, W> {
    output: W,
    bytes: usize,
    hash: crc32fast::Hasher,
    work: &'a WorkControl,
}
impl<W: Write> Write for Checked<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.work.check().map_err(io::Error::other)?;
        if self
            .bytes
            .checked_add(bytes.len())
            .is_none_or(|n| n > MAX_PAYLOAD)
        {
            return Err(io::Error::other(
                "mesh project exceeds 128 MiB; reduce history or split the mesh",
            ));
        }
        let n = self.output.write(bytes)?;
        self.bytes += n;
        self.hash.update(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.work.check().map_err(io::Error::other)?;
        self.output.flush()
    }
}
pub fn save(editor: &MeshEditor, mut output: impl Write, work: &WorkControl) -> io::Result<()> {
    editor.validate_history(work).map_err(io::Error::other)?;
    #[derive(Serialize)]
    struct View<'a> {
        schema_version: u32,
        units: &'static str,
        editor: &'a MeshEditor,
    }
    let view = View {
        schema_version: 1,
        units: "millimeters",
        editor,
    };
    let mut count = Checked {
        output: io::sink(),
        bytes: 0,
        hash: crc32fast::Hasher::new(),
        work,
    };
    serde_json::to_writer(&mut count, &view).map_err(io::Error::other)?;
    work.check().map_err(io::Error::other)?;
    let length = count.bytes;
    let crc = count.hash.finalize();
    output.write_all(MAGIC)?;
    output.write_all(&(length as u32).to_le_bytes())?;
    output.write_all(&crc.to_le_bytes())?;
    let mut checked = Checked {
        output: &mut output,
        bytes: 0,
        hash: crc32fast::Hasher::new(),
        work,
    };
    serde_json::to_writer(&mut checked, &view).map_err(io::Error::other)?;
    checked.flush()?;
    if checked.bytes != length || checked.hash.finalize() != crc {
        return Err(io::Error::other(
            "mesh project serialization changed between passes",
        ));
    }
    Ok(())
}
pub fn load(mut input: impl Read, work: &WorkControl) -> io::Result<MeshEditor> {
    work.check().map_err(io::Error::other)?;
    let mut header = [0u8; 16];
    input.read_exact(&mut header)?;
    if &header[..8] != MAGIC {
        return Err(io::Error::other("unsupported mesh project magic"));
    }
    let length = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
    let expected = u32::from_le_bytes(header[12..16].try_into().unwrap());
    if length > MAX_PAYLOAD {
        return Err(io::Error::other("mesh project length exceeds 128 MiB"));
    }
    let mut bytes = vec![0; length];
    for chunk in bytes.chunks_mut(65536) {
        work.check().map_err(io::Error::other)?;
        input.read_exact(chunk)?;
    }
    let mut extra = [0u8; 1];
    if input.read(&mut extra)? != 0 {
        return Err(io::Error::other("trailing mesh project bytes"));
    }
    if crc32fast::hash(&bytes) != expected {
        return Err(io::Error::other("mesh project CRC mismatch"));
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Stored {
        schema_version: u32,
        units: String,
        editor: MeshEditor,
    }
    let stored: Stored = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if stored.schema_version != 1 || stored.units != "millimeters" {
        return Err(io::Error::other("unsupported mesh project schema or units"));
    }
    stored
        .editor
        .validate_history(work)
        .map_err(io::Error::other)?;
    Ok(stored.editor)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::Mesh;
    fn original() -> Mesh {
        Mesh {
            vertices: vec![
                [0., 0., 0.],
                [f64::from_bits(0.1f64.to_bits() + 1), 0., 0.],
                [0., 1., 0.],
            ],
            triangles: vec![[0, 1, 2]],
        }
    }
    fn editor() -> MeshEditor {
        let w = WorkControl::default();
        let mut e = MeshEditor::new(original(), 8, &w).unwrap();
        for z in [1., 2.] {
            let mut mesh = original();
            for p in &mut mesh.vertices {
                p[2] = z;
            }
            e.replace(mesh, &w).unwrap();
        }
        e.undo(&w).unwrap();
        e
    }
    #[test]
    fn exact_roundtrip_restores_current_undo_and_redo_and_f64_bits() {
        let work = WorkControl::default();
        let e = editor();
        let mut bytes = Vec::new();
        save(&e, &mut bytes, &work).unwrap();
        let mut restored = load(bytes.as_slice(), &work).unwrap();
        assert_eq!(restored.mesh(), e.mesh());
        assert_eq!(restored.history_depth(), (1, 1));
        restored.undo(&work).unwrap();
        assert_eq!(restored.mesh(), &original());
        restored.redo(&work).unwrap();
        assert_eq!(restored.mesh(), e.mesh());
        restored.redo(&work).unwrap();
        assert!(restored.mesh().vertices.iter().all(|p| p[2] == 2.));
    }
    #[test]
    fn corruption_all_truncations_trailing_bytes_and_oversize_are_controlled() {
        let w = WorkControl::default();
        let mut bytes = Vec::new();
        save(&editor(), &mut bytes, &w).unwrap();
        for n in 0..bytes.len() {
            assert!(load(&bytes[..n], &w).is_err());
        }
        let mut corrupt = bytes.clone();
        corrupt[20] ^= 1;
        assert!(load(corrupt.as_slice(), &w).is_err());
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(load(trailing.as_slice(), &w).is_err());
        bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(load(bytes.as_slice(), &w).is_err());
    }
    #[test]
    fn crc_valid_bad_history_schema_units_and_cancellation_are_rejected() {
        let w = WorkControl::default();
        let mut bytes = Vec::new();
        save(&editor(), &mut bytes, &w).unwrap();
        let original: serde_json::Value = serde_json::from_slice(&bytes[16..]).unwrap();
        for kind in 0..4 {
            let mut json = original.clone();
            match kind {
                0 => json["editor"]["limit"] = serde_json::json!(0),
                1 => json["units"] = serde_json::json!("unknown"),
                2 => json["editor"]["undo"][0]["triangles"][0][0] = serde_json::json!(99),
                _ => json["schema_version"] = serde_json::json!(2),
            };
            let payload = serde_json::to_vec(&json).unwrap();
            let mut data = Vec::from(*MAGIC);
            data.extend((payload.len() as u32).to_le_bytes());
            data.extend(crc32fast::hash(&payload).to_le_bytes());
            data.extend(payload);
            assert!(load(data.as_slice(), &w).is_err());
        }
        let cancelled = WorkControl::default();
        cancelled.cancel();
        let mut out = Vec::new();
        assert!(save(&editor(), &mut out, &cancelled).is_err());
        assert!(out.is_empty());
        assert!(load(bytes.as_slice(), &cancelled).is_err());
    }
}
