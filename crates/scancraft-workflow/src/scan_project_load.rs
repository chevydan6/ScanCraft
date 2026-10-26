//! Explicit read-only loading of fixed-layout scan folders. CRC is integrity, not authenticity.
use crate::{Error, scan_project::associate, source_verification};
use scancraft_geometry::{
    Editor,
    formats::load_project,
    work::{Stage, WorkControl},
};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs::{self, File, Metadata, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
type Stamp = (u64, u64, u64, i64, i64, i64, i64);
fn stamp(m: &Metadata) -> Stamp {
    (
        m.dev(),
        m.ino(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    )
}
struct Entry {
    path: PathBuf,
    file: File,
    initial: Stamp,
    directory: bool,
}
#[derive(Default)]
struct Guard {
    entries: Vec<Entry>,
}
impl Guard {
    fn open(&mut self, path: &Path, directory: bool, max: u64) -> Result<&mut File, Error> {
        let before = fs::symlink_metadata(path)?;
        if (directory && !before.is_dir())
            || (!directory && (!before.is_file() || before.len() > max))
        {
            return Err(
                "scan folder requires bounded regular files and directories without symlinks"
                    .into(),
            );
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(path)?;
        if stamp(&before) != stamp(&file.metadata()?) {
            return Err("scan folder changed while opening".into());
        }
        self.entries.push(Entry {
            path: path.into(),
            file,
            initial: stamp(&before),
            directory,
        });
        Ok(&mut self.entries.last_mut().unwrap().file)
    }
    fn read(
        &mut self,
        path: &Path,
        max: u64,
        retain: bool,
        work: &WorkControl,
    ) -> Result<(Vec<u8>, u64, u32), Error> {
        let file = self.open(path, false, max)?;
        let mut bytes = Vec::new();
        let mut total = 0u64;
        let mut hash = crc32fast::Hasher::new();
        let mut buffer = [0u8; 65536];
        loop {
            work.update(Stage::Reading, total as usize, 0)?;
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > max {
                return Err("scan folder file exceeds byte budget".into());
            }
            hash.update(&buffer[..n]);
            if retain {
                bytes.extend_from_slice(&buffer[..n]);
            }
        }
        Ok((bytes, total, hash.finalize()))
    }
    fn check(&self) -> Result<(), Error> {
        for e in &self.entries {
            let current = fs::symlink_metadata(&e.path)?;
            if (e.directory && !current.is_dir())
                || (!e.directory && !current.is_file())
                || stamp(&current) != e.initial
                || stamp(&e.file.metadata()?) != e.initial
            {
                return Err("scan folder changed during verification; reopen it".into());
            }
        }
        Ok(())
    }
}
pub struct Loaded {
    pub editor: Editor,
    pub report: Value,
    pub left: PathBuf,
    pub right: PathBuf,
}
pub fn load(directory: &Path, work: &WorkControl) -> Result<Loaded, Error> {
    load_inner(directory, work, &|| Ok(()))
}
fn load_inner(
    directory: &Path,
    work: &WorkControl,
    before_final: &dyn Fn() -> Result<(), Error>,
) -> Result<Loaded, Error> {
    work.check()?;
    let mut guard = Guard::default();
    for path in [
        directory.to_path_buf(),
        directory.join("project"),
        directory.join("recordings"),
    ] {
        guard.open(&path, true, 0)?;
    }
    let (bytes, _, _) = guard.read(
        &directory.join("manifest.json"),
        32 * 1024 * 1024,
        true,
        work,
    )?;
    let root: Value = serde_json::from_slice(&bytes)?;
    if root["schema_version"] != 1
        || root["kind"] != "recorded_stereo_scan_project"
        || root["project"] != "project/project.orproj"
        || root["project_sources"] != "project"
        || root["recordings_directory"] != "recordings"
    {
        return Err("unsupported scan folder layout/schema".into());
    }
    for flag in [
        "historical_full_recording_identity_verified",
        "source_device_binding_verified",
        "synchronization_verified",
        "physical_accuracy_qualified",
        "original_capture_completeness_verified",
    ] {
        if root[flag] != false {
            return Err(
                "scan folder cannot assert unqualified physical or historical guarantees".into(),
            );
        }
    }
    let mut project_bytes = None;
    for (child, key, names, max) in [
        (
            "project",
            "project_bundle",
            &[
                "calibration.rapcal",
                "request.json",
                "recording-sources.json",
                "left-frame.raptor",
                "right-frame.raptor",
                "project.orproj",
                "bundle-report.json",
            ][..],
            128 * 1024 * 1024u64,
        ),
        (
            "recordings",
            "recording_archive",
            &["left.raptor", "right.raptor"][..],
            1u64 << 40,
        ),
    ] {
        let (bytes, _, _) = guard.read(
            &directory.join(child).join("manifest.json"),
            32 * 1024 * 1024,
            true,
            work,
        )?;
        let manifest: Value = serde_json::from_slice(&bytes)?;
        if manifest != root[key] {
            return Err("scan folder nested manifest disagrees with root".into());
        }
        let artifacts = manifest["artifacts"]
            .as_array()
            .ok_or("missing scan folder artifact inventory")?;
        let mut seen = HashSet::new();
        for a in artifacts {
            let name = a["role"]
                .as_str()
                .ok_or("invalid scan folder artifact name")?;
            if !names.contains(&name) || !seen.insert(name) {
                return Err("unexpected or duplicate scan folder artifact".into());
            }
            let size = a["bytes"]
                .as_u64()
                .filter(|n| *n > 0 && *n <= max)
                .ok_or("invalid scan folder artifact byte budget")?;
            let crc = a["crc32"]
                .as_u64()
                .filter(|n| *n <= u32::MAX as u64)
                .ok_or("invalid scan folder artifact checksum")?;
            let retain = name == "project.orproj";
            if retain && size > 64 * 1024 * 1024 + 16 {
                return Err("scan folder project exceeds byte budget".into());
            }
            let (bytes, actual, hash) =
                guard.read(&directory.join(child).join(name), size, retain, work)?;
            if actual != size || u64::from(hash) != crc {
                return Err(
                    format!("scan folder artifact integrity mismatch: {child}/{name}").into(),
                );
            }
            if retain {
                project_bytes = Some(bytes);
            }
        }
        if seen.len() != names.len() {
            return Err("incomplete scan folder artifact inventory".into());
        }
    }
    let editor = load_project(
        project_bytes
            .ok_or("missing scan folder project")?
            .as_slice(),
    )?;
    let origin = editor
        .origin()
        .ok_or("scan folder project lacks recorded provenance")?;
    let checked = source_verification::verify(origin, &directory.join("project"), work)?;
    if !checked.passed {
        return Err("scan folder retained inputs disagree with project".into());
    }
    let association = associate(&editor, &directory.join("recordings"), work)?;
    if association != root["association"] {
        return Err("scan folder association disagrees with manifest".into());
    }
    let references = crate::scan_project::references(
        &editor,
        &root["recording_archive"],
        &directory.join("recordings"),
    )?;
    if let Some(saved) = editor.recordings()
        && (saved.schema_version != references.schema_version || saved.files != references.files)
    {
        return Err("project recording references disagree with packaged recordings".into());
    }
    let editor = editor.copied_with_recordings(references)?;
    let relocated = editor.copied_with_source_directory(
        std::path::absolute(directory.join("project"))?
            .display()
            .to_string(),
    )?;
    before_final()?;
    work.check()?;
    guard.check()?;
    Ok(Loaded {
        editor: relocated,
        left: directory.join("recordings/left.raptor"),
        right: directory.join("recordings/right.raptor"),
        report: json!({"schema_version":1,"integrity":"PASS","scope":"fixed-layout scan folder manifests, artifact fingerprints, full recording replay and selected-frame membership","source_directory_relocated_in_memory":true,"source_files_written":false,"historical_full_recording_identity_verified":false,"source_device_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,"association":association}),
    })
}

/// Explicitly verify complete recordings against saved project references.
/// Standalone project parsing itself never invokes this operation.
pub fn verify_recordings(
    editor: &Editor,
    directory: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    work.check()?;
    editor.validate_history()?;
    let references = editor
        .recordings()
        .ok_or("project has no complete recording references")?;
    let mut guard = Guard::default();
    guard.open(directory, true, 0)?;
    for expected in &references.files {
        let (_, bytes, crc) =
            guard.read(&directory.join(&expected.name), expected.bytes, false, work)?;
        if bytes != expected.bytes || crc != expected.crc32 {
            return Err(
                format!("complete recording fingerprint mismatch: {}", expected.name).into(),
            );
        }
    }
    let association = associate(editor, directory, work)?;
    work.check()?;
    guard.check()?;
    Ok(
        json!({"schema_version":1,"integrity":"PASS","scope":"saved complete recording fingerprints and initial selected-frame membership","association":association,"project_references_changed":false,"source_files_written":false,"historical_full_recording_identity_verified":false,"source_device_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false}),
    )
}
pub fn verify_project_recordings(
    project: &Path,
    directory: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    let bytes = crate::bounded(project, 64 * 1024 * 1024 + 16, work)?;
    let editor = load_project(bytes.as_slice())?;
    verify_recordings(&editor, directory, work)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "scancraft-scan-load-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn fixture(t: &Temp) -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence");
        let source = root.join("project-origin-gui-render/result");
        let dest = t.0.join("scan");
        let mut editor = load_project(File::open(source.join("project.orproj")).unwrap()).unwrap();
        editor.trim(&[0], true).unwrap();
        crate::scan_project::export(
            &editor,
            &source,
            &root.join("recorded-stereo-dataset/left.raptor"),
            &root.join("recorded-stereo-dataset/right.raptor"),
            &dest,
            &WorkControl::default(),
        )
        .unwrap();
        dest
    }
    #[test]
    fn relocated_load_checks_every_artifact_and_updates_only_memory() {
        let t = Temp::new();
        let old = fixture(&t);
        let moved = t.0.join("moved");
        fs::rename(old, &moved).unwrap();
        let bytes = fs::read(moved.join("project/project.orproj")).unwrap();
        let mut loaded = load(&moved, &WorkControl::default()).unwrap();
        assert_eq!(loaded.report["integrity"], "PASS");
        assert_eq!(
            loaded.editor.origin().unwrap().bundle_directory_display,
            moved.join("project").display().to_string()
        );
        assert_eq!(
            fs::read(moved.join("project/project.orproj")).unwrap(),
            bytes
        );
        assert_eq!(loaded.left, moved.join("recordings/left.raptor"));
        assert_eq!(loaded.editor.cloud.points.len(), 1);
        assert!(loaded.editor.undo());
        assert_eq!(loaded.editor.cloud.points.len(), 3);
        assert!(loaded.editor.redo());
        assert_eq!(loaded.editor.cloud.points.len(), 1);
    }
    #[test]
    fn corruption_of_unselected_frames_and_nested_manifests_is_rejected() {
        let t = Temp::new();
        let p = fixture(&t);
        let path = p.join("recordings/left.raptor");
        let original = fs::read(&path).unwrap();
        let mut bad = original.clone();
        let n = bad.len();
        bad[n - 20] ^= 1;
        fs::write(&path, bad).unwrap();
        assert!(
            load(&p, &WorkControl::default())
                .err()
                .unwrap()
                .to_string()
                .contains("integrity mismatch")
        );
        fs::write(path, original).unwrap();
        fs::write(p.join("project/manifest.json"), b"{}").unwrap();
        assert!(load(&p, &WorkControl::default()).is_err());
    }
    #[test]
    fn saved_edited_projects_keep_complete_recording_identity_and_explicit_verification() {
        use scancraft_geometry::formats::save_project;
        let t = Temp::new();
        let folder = fixture(&t);
        let work = WorkControl::default();
        let mut loaded = load(&folder, &work).unwrap();
        let refs = loaded.editor.recordings().unwrap().clone();
        assert_eq!(
            refs.directory_display,
            folder.join("recordings").display().to_string()
        );
        loaded.editor.undo();
        let mut saved = Vec::new();
        save_project(&loaded.editor, &mut saved).unwrap();
        assert_eq!(&saved[..8], b"ORPROJ06");
        let reopened = load_project(saved.as_slice()).unwrap();
        assert_eq!(reopened.recordings(), Some(&refs));
        assert_eq!(
            verify_recordings(&reopened, &folder.join("recordings"), &work).unwrap()["integrity"],
            "PASS"
        );
        let mut after = Vec::new();
        save_project(&reopened, &mut after).unwrap();
        assert_eq!(saved, after);
        fs::write(folder.join("recordings/left.raptor"), b"changed").unwrap();
        assert!(verify_recordings(&reopened, &folder.join("recordings"), &work).is_err());
        let mut after = Vec::new();
        save_project(&reopened, &mut after).unwrap();
        assert_eq!(saved, after);
    }
    #[test]
    fn crc_valid_nested_project_cannot_disagree_with_container_recording_fingerprints() {
        use scancraft_geometry::formats::save_project;
        let t = Temp::new();
        let folder = fixture(&t);
        let loaded = load(&folder, &WorkControl::default()).unwrap();
        let mut refs = loaded.editor.recordings().unwrap().clone();
        refs.files[0].crc32 ^= 1;
        let editor = loaded.editor.copied_with_recordings(refs).unwrap();
        let mut bytes = Vec::new();
        save_project(&editor, &mut bytes).unwrap();
        fs::write(folder.join("project/project.orproj"), &bytes).unwrap();
        let root_path = folder.join("manifest.json");
        let mut root: Value = serde_json::from_slice(&fs::read(&root_path).unwrap()).unwrap();
        for artifact in root["project_bundle"]["artifacts"].as_array_mut().unwrap() {
            if artifact["role"] == "project.orproj" {
                artifact["bytes"] = json!(bytes.len());
                artifact["crc32"] = json!(crc32fast::hash(&bytes));
            }
        }
        fs::write(
            folder.join("project/manifest.json"),
            serde_json::to_vec(&root["project_bundle"]).unwrap(),
        )
        .unwrap();
        fs::write(root_path, serde_json::to_vec(&root).unwrap()).unwrap();
        assert!(
            load(&folder, &WorkControl::default())
                .err()
                .unwrap()
                .to_string()
                .contains("recording references disagree")
        );
    }
    #[test]
    fn unexpected_paths_symlinks_and_late_changes_or_cancel_are_rejected() {
        let t = Temp::new();
        let p = fixture(&t);
        let work = WorkControl::default();
        assert!(
            load_inner(&p, &work, &|| {
                work.cancel();
                Ok(())
            })
            .is_err()
        );
        assert!(
            load_inner(&p, &WorkControl::default(), &|| {
                fs::write(p.join("project/request.json"), b"changed")?;
                Ok(())
            })
            .is_err()
        );
        let t = Temp::new();
        let p = fixture(&t);
        let original = p.join("recordings/left.raptor");
        let copy = t.0.join("left");
        fs::rename(&original, &copy).unwrap();
        std::os::unix::fs::symlink(copy, original).unwrap();
        assert!(load(&p, &WorkControl::default()).is_err());
        let t = Temp::new();
        let p = fixture(&t);
        let path = p.join("manifest.json");
        let mut root: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        root["project"] = json!("../other");
        fs::write(path, serde_json::to_vec(&root).unwrap()).unwrap();
        assert!(load(&p, &WorkControl::default()).is_err());
    }
}
