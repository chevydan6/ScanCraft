//! Atomic self-contained section snapshots; source geometry plus checked request.
use super::{
    Error, artifact, bounded,
    output::{publish, staging},
};
use scancraft_geometry::{
    mesh::Mesh,
    mesh_edit::MeshEditor,
    mesh_project,
    mesh_section::{self, Section},
    work::WorkControl,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};
const MAGIC: &[u8; 8] = b"SECPLN01";
const MAX_DESCRIPTOR: usize = 64 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    schema_version: u32,
    units: String,
    algorithm: String,
    physical_accuracy_qualified: bool,
    origin_mm: [f64; 3],
    normal: [f64; 3],
    tolerance_mm: f64,
    source_bytes: u64,
    source_crc32: u32,
    nodes: usize,
    segments: usize,
    contours: usize,
    ambiguous_components: usize,
    total_length_mm: f64,
}
pub struct Loaded {
    pub normal: [f64; 3],
    pub mesh: Mesh,
    pub section: Section,
}
pub fn save(
    mesh: &Mesh,
    origin: [f64; 3],
    normal: [f64; 3],
    tolerance: f64,
    target: &Path,
    work: &WorkControl,
) -> Result<(), Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("section output already exists".into());
    }
    let section = mesh_section::intersect(mesh, origin, normal, tolerance, work)?;
    let editor = MeshEditor::new(mesh.clone(), 1, work)?;
    let partial = staging(target)?;
    let result = (|| -> Result<(), Error> {
        let source = artifact(&partial, "source.meshproj", work, |out| {
            mesh_project::save(&editor, out, work)?;
            Ok(())
        })?;
        let descriptor = Descriptor {
            schema_version: 1,
            units: "millimeters".into(),
            algorithm: "indexed_plane_section_v1".into(),
            physical_accuracy_qualified: false,
            origin_mm: origin,
            normal,
            tolerance_mm: tolerance,
            source_bytes: source.bytes,
            source_crc32: source.crc32,
            nodes: section.nodes.len(),
            segments: section.segments.len(),
            contours: section.contours.len(),
            ambiguous_components: section.ambiguous_components.len(),
            total_length_mm: section.total_segment_length_mm,
        };
        let payload = serde_json::to_vec(&descriptor)?;
        if payload.len() > MAX_DESCRIPTOR {
            return Err("section descriptor exceeds byte budget".into());
        }
        artifact(&partial, "section.secproj", work, |out| {
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
            "{e}; incomplete section staging retained at {}",
            partial.display()
        )
    })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "section published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(())
}
pub fn load(path: &Path, work: &WorkControl) -> Result<Loaded, Error> {
    let data = bounded(path, MAX_DESCRIPTOR + 16, work)?;
    if data.len() < 16 || &data[..8] != MAGIC {
        return Err("invalid section document magic/header".into());
    }
    let length = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    if length > MAX_DESCRIPTOR
        || data.len() != length + 16
        || crc32fast::hash(&data[16..]) != u32::from_le_bytes(data[12..16].try_into().unwrap())
    {
        return Err("section descriptor length/CRC mismatch".into());
    }
    let d: Descriptor = serde_json::from_slice(&data[16..])?;
    if d.schema_version != 1
        || d.units != "millimeters"
        || d.algorithm != "indexed_plane_section_v1"
        || d.physical_accuracy_qualified
        || !d.total_length_mm.is_finite()
        || d.total_length_mm < 0.
        || d.source_bytes > (mesh_project::MAX_PAYLOAD + 16) as u64
    {
        return Err("unsupported or invalid section descriptor".into());
    }
    let source = bounded(
        &path.with_file_name("source.meshproj"),
        mesh_project::MAX_PAYLOAD + 16,
        work,
    )?;
    if source.len() as u64 != d.source_bytes || crc32fast::hash(&source) != d.source_crc32 {
        return Err("section source size/CRC mismatch".into());
    }
    let editor = mesh_project::load(source.as_slice(), work)?;
    let (mesh, undo, redo, _) = editor.into_history();
    if !undo.is_empty() || !redo.is_empty() {
        return Err("section source must contain one mesh snapshot".into());
    }
    let section = mesh_section::intersect(&mesh, d.origin_mm, d.normal, d.tolerance_mm, work)?;
    if section.nodes.len() != d.nodes
        || section.segments.len() != d.segments
        || section.contours.len() != d.contours
        || section.ambiguous_components.len() != d.ambiguous_components
        || (section.total_segment_length_mm - d.total_length_mm).abs()
            > 1e-12 * section.total_segment_length_mm.max(f64::MIN_POSITIVE)
    {
        return Err("recomputed section disagrees with recorded totals".into());
    }
    work.check()?;
    Ok(Loaded {
        normal: d.normal,
        mesh,
        section,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Temp(std::path::PathBuf);
    impl Temp {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "scancraft-section-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn mesh() -> Mesh {
        Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        }
    }
    fn saved(temp: &Temp) -> std::path::PathBuf {
        let folder = temp.0.join("snapshot");
        save(
            &mesh(),
            [0.5, 0., 0.],
            [1., 0., 0.],
            0.,
            &folder,
            &WorkControl::default(),
        )
        .unwrap();
        folder.join("section.secproj")
    }
    #[test]
    fn snapshot_roundtrip_retains_exact_source_and_recomputes_graph() {
        let temp = Temp::new();
        let path = saved(&temp);
        let loaded = load(&path, &WorkControl::default()).unwrap();
        assert_eq!(loaded.mesh, mesh());
        assert_eq!(loaded.section.total_segment_length_mm, 1.);
        assert_eq!(loaded.section.contours.len(), 1);
        assert!(!loaded.section.contours[0].closed);
        let original = fs::read(&path).unwrap();
        assert!(
            save(
                &mesh(),
                [0.5, 0., 0.],
                [1., 0., 0.],
                0.,
                path.parent().unwrap(),
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        let target = temp.0.join("cancelled");
        assert!(
            save(
                &mesh(),
                [0.5, 0., 0.],
                [1., 0., 0.],
                0.,
                &target,
                &cancelled
            )
            .is_err()
        );
        assert!(!target.exists());
        assert!(load(&path, &cancelled).is_err());
    }
    #[test]
    fn corruption_missing_source_and_trailing_data_are_rejected() {
        let temp = Temp::new();
        let path = saved(&temp);
        let original = fs::read(&path).unwrap();
        let mut bad = original.clone();
        bad[16] ^= 1;
        fs::write(&path, &bad).unwrap();
        assert!(load(&path, &WorkControl::default()).is_err());
        bad = original.clone();
        bad.push(0);
        fs::write(&path, &bad).unwrap();
        assert!(load(&path, &WorkControl::default()).is_err());
        fs::write(&path, &original).unwrap();
        let source = path.with_file_name("source.meshproj");
        let mut bytes = fs::read(&source).unwrap();
        bytes[16] ^= 1;
        fs::write(&source, &bytes).unwrap();
        assert!(load(&path, &WorkControl::default()).is_err());
        fs::remove_file(source).unwrap();
        assert!(load(&path, &WorkControl::default()).is_err());
    }
    #[test]
    fn crc_valid_inconsistent_totals_and_unknown_metadata_are_rejected() {
        let temp = Temp::new();
        let path = saved(&temp);
        let original = fs::read(&path).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&original[16..]).unwrap();
        for (key, bad) in [
            ("segments", serde_json::json!(999)),
            ("units", serde_json::json!("inches")),
            ("physical_accuracy_qualified", serde_json::json!(true)),
            ("source_path", serde_json::json!("/tmp/other")),
        ] {
            let mut changed = value.clone();
            changed[key] = bad;
            let payload = serde_json::to_vec(&changed).unwrap();
            let mut bytes = MAGIC.to_vec();
            bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&crc32fast::hash(&payload).to_le_bytes());
            bytes.extend(payload);
            fs::write(&path, &bytes).unwrap();
            assert!(load(&path, &WorkControl::default()).is_err(), "{key}");
        }
    }
}

#[cfg(test)]
mod normal_roundtrip_tests {
    use super::*;
    #[test]
    fn original_input_normal_survives_resaving_without_normalization_drift() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-section-normal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let mesh = Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        };
        let normal = [1., 3., 7.];
        let work = WorkControl::default();
        let first = root.join("first");
        save(&mesh, [0.5, 0., 0.], normal, 0., &first, &work).unwrap();
        let loaded = load(&first.join("section.secproj"), &work).unwrap();
        assert_eq!(loaded.normal, normal);
        let second = root.join("second");
        save(
            &loaded.mesh,
            loaded.section.plane_origin_mm,
            loaded.normal,
            loaded.section.tolerance_mm,
            &second,
            &work,
        )
        .unwrap();
        assert_eq!(
            fs::read(first.join("section.secproj")).unwrap(),
            fs::read(second.join("section.secproj")).unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
