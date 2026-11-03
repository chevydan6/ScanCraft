//! Explicit, read-only retained-artifact identity checks. Never invoked on load.
use crate::Error;
use scancraft_geometry::{
    project_origin::Origin,
    work::{Stage, WorkControl},
};
use serde::Serialize;
use std::{
    fs::{self, Metadata, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
fn stamp(m: &Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
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
#[derive(Debug, Serialize)]
pub struct ArtifactResult {
    pub name: String,
    pub expected_bytes: u64,
    pub expected_crc32: u32,
    pub actual_bytes: Option<u64>,
    pub actual_crc32: Option<u32>,
    pub status: String,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub passed: bool,
    pub directory_display: String,
    pub directory_unchanged: bool,
    pub artifacts: Vec<ArtifactResult>,
    pub source_device_binding_verified: bool,
    pub physical_accuracy_qualified: bool,
    pub synchronization_verified: bool,
}
pub fn verify(origin: &Origin, directory: &Path, work: &WorkControl) -> Result<Report, Error> {
    verify_inner(origin, directory, work, &|| Ok(()))
}
// Boundary hook permits deterministic mutation/cancellation regression tests.
fn verify_inner(
    origin: &Origin,
    directory: &Path,
    work: &WorkControl,
    before_final: &dyn Fn() -> Result<(), Error>,
) -> Result<Report, Error> {
    work.check()?;
    origin.validate()?;
    verify_artifacts(&origin.artifacts, directory, work, before_final)
}
/// Explicit sequence artifact verification; references alone never verify disk.
pub fn verify_sequence(
    sequence: &scancraft_geometry::sequence_references::Sequence,
    directory: &Path,
    work: &WorkControl,
) -> Result<Report, Error> {
    work.check()?;
    sequence.validate()?;
    let expected: Vec<_> = sequence
        .artifacts
        .iter()
        .map(|a| scancraft_geometry::project_origin::Artifact {
            name: a.name.clone(),
            bytes: a.bytes,
            crc32: a.crc32,
        })
        .collect();
    verify_artifacts(&expected, directory, work, &|| Ok(()))
}
/// Explicit live-session verification against saved fingerprints; no automatic path following.
pub fn verify_live(
    live: &scancraft_geometry::live_references::LiveSession,
    directory: &Path,
    work: &WorkControl,
) -> Result<Report, Error> {
    work.check()?;
    live.validate()?;
    let expected: Vec<_> = live
        .artifacts
        .iter()
        .map(|a| scancraft_geometry::project_origin::Artifact {
            name: a.name.clone(),
            bytes: a.bytes,
            crc32: a.crc32,
        })
        .collect();
    verify_artifacts(&expected, directory, work, &|| Ok(()))
}
fn verify_artifacts(
    expected: &[scancraft_geometry::project_origin::Artifact],
    directory: &Path,
    work: &WorkControl,
    before_final: &dyn Fn() -> Result<(), Error>,
) -> Result<Report, Error> {
    work.check()?;
    let before = fs::symlink_metadata(directory)?;
    if !before.is_dir() {
        return Err("source bundle must be an explicit directory, not a symlink".into());
    }
    let mut parents = std::collections::BTreeMap::new();
    for artifact in expected {
        work.check()?;
        if let Some(parent) = Path::new(&artifact.name)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            let path = directory.join(parent);
            if parents.contains_key(&path) {
                continue;
            }
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                format!(
                    "Cannot inspect retained artifact directory {}: {error}",
                    path.display()
                )
            })?;
            if !metadata.is_dir() {
                return Err(format!(
                    "Retained artifact directory {} must be a real directory, not a symlink",
                    path.display()
                )
                .into());
            }
            parents.insert(path, stamp(&metadata));
        }
    }
    let mut artifacts = Vec::new();
    let mut verified_stamps = Vec::new();
    let total_artifacts = expected.len();
    for (index, expected) in expected.iter().enumerate() {
        work.update(Stage::Reading, index, total_artifacts)?;
        let mut result = ArtifactResult {
            name: expected.name.clone(),
            expected_bytes: expected.bytes,
            expected_crc32: expected.crc32,
            actual_bytes: None,
            actual_crc32: None,
            status: "unread".into(),
        };
        let path = directory.join(&expected.name);
        let mut verified_stamp = None;
        let checked = (|| -> Result<(), Error> {
            if !fs::symlink_metadata(&path)?.is_file() {
                result.status = "not_regular_file".into();
                return Ok(());
            }
            let mut file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)?;
            let initial = file.metadata()?;
            if !initial.is_file() {
                result.status = "not_regular_file".into();
                return Ok(());
            }
            result.actual_bytes = Some(initial.len());
            if initial.len() != expected.bytes {
                result.status = "size_mismatch".into();
                return Ok(());
            }
            let mut hash = crc32fast::Hasher::new();
            let mut bytes = 0u64;
            let mut buffer = [0u8; 65536];
            // One extra byte detects concurrent growth without unbounded reading.
            while bytes <= expected.bytes {
                work.check()?;
                let available = (expected.bytes + 1 - bytes).min(buffer.len() as u64) as usize;
                let count = file.read(&mut buffer[..available])?;
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
                bytes += count as u64;
            }
            work.check()?;
            let crc = hash.finalize();
            result.actual_bytes = Some(bytes);
            result.actual_crc32 = Some(crc);
            let current = fs::symlink_metadata(&path)?;
            result.status = if !current.is_file()
                || stamp(&initial) != stamp(&file.metadata()?)
                || stamp(&initial) != stamp(&current)
            {
                "changed_during_verification"
            } else if bytes != expected.bytes {
                "size_mismatch"
            } else if crc != expected.crc32 {
                "crc_mismatch"
            } else {
                "match"
            }
            .into();
            if result.status == "match" {
                verified_stamp = Some(stamp(&current));
            }
            Ok(())
        })();
        // Cancellation is always fatal, never reduced to an artifact mismatch.
        work.check()?;
        if let Err(error) = checked {
            result.status = format!("unreadable: {error}");
        }
        artifacts.push(result);
        verified_stamps.push(verified_stamp);
    }
    before_final()?;
    work.update(Stage::Finishing, expected.len(), expected.len())?;
    for (artifact, expected) in artifacts.iter_mut().zip(verified_stamps) {
        if let Some(expected) = expected
            && !fs::symlink_metadata(directory.join(&artifact.name))
                .is_ok_and(|m| m.is_file() && stamp(&m) == expected)
        {
            artifact.status = "changed_during_verification".into();
        }
    }
    work.check()?;
    let unchanged = fs::symlink_metadata(directory)
        .is_ok_and(|m| m.is_dir() && stamp(&m) == stamp(&before))
        && parents.iter().all(|(path, expected)| {
            fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && stamp(&m) == *expected)
        });
    Ok(Report {
        schema_version: 1,
        passed: unchanged && artifacts.iter().all(|r| r.status == "match"),
        directory_display: directory.display().to_string(),
        directory_unchanged: unchanged,
        artifacts,
        source_device_binding_verified: false,
        physical_accuracy_qualified: false,
        synchronization_verified: false,
    })
}

/// Load only the chosen project, then explicitly inspect a chosen bundle directory.
pub fn verify_project(
    project: &Path,
    directory: &Path,
    work: &WorkControl,
) -> Result<Report, Error> {
    work.check()?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(project)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 + 16 {
        return Err("verification requires a regular bounded project".into());
    }
    let editor = scancraft_geometry::formats::load_project(std::io::BufReader::new(file))?;
    if let Some(live) = editor.live_session() {
        return verify_live(live, directory, work);
    }
    if let Some(sequence) = editor.sequence() {
        return verify_sequence(sequence, directory, work);
    }
    verify(
        editor
            .origin()
            .ok_or("project has no reconstruction provenance")?,
        directory,
        work,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_geometry::project_origin::Artifact;
    use serde_json::json;
    use std::{
        io::Write,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "scancraft-source-check-{}-{}",
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
    fn fixture(t: &Temp) -> (Origin, PathBuf) {
        let directory = t.0.join("bundle");
        fs::create_dir(&directory).unwrap();
        let artifacts = [
            "calibration.rapcal",
            "request.json",
            "recording-sources.json",
            "left-frame.raptor",
            "right-frame.raptor",
        ]
        .into_iter()
        .map(|name| {
            let bytes = name.as_bytes();
            fs::write(directory.join(name), bytes).unwrap();
            Artifact {
                name: name.into(),
                bytes: bytes.len() as u64,
                crc32: crc32fast::hash(bytes),
            }
        })
        .collect();
        (
            Origin {
                schema_version: 1,
                kind: "initial_recorded_stereo_reconstruction".into(),
                bundle_directory_display: "/old/missing/bundle".into(),
                calibration: json!({}),
                request: json!({}),
                recording_sources: json!({"source_device_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false}),
                artifacts,
            },
            directory,
        )
    }
    #[test]
    fn relocated_directory_matches_without_changing_files_or_reference() {
        let t = Temp::new();
        let (origin, old) = fixture(&t);
        let directory = t.0.join("moved");
        fs::rename(old, &directory).unwrap();
        let before: Vec<_> = origin
            .artifacts
            .iter()
            .map(|a| fs::read(directory.join(&a.name)).unwrap())
            .collect();
        let report = verify(&origin, &directory, &WorkControl::default()).unwrap();
        assert!(report.passed);
        assert!(report.directory_unchanged);
        assert_eq!(report.artifacts.len(), 5);
        assert!(report.artifacts.iter().all(|a| a.status == "match"));
        assert!(!report.physical_accuracy_qualified);
        assert_eq!(origin.bundle_directory_display, "/old/missing/bundle");
        for (a, b) in origin.artifacts.iter().zip(before) {
            assert_eq!(fs::read(directory.join(&a.name)).unwrap(), b);
        }
    }
    #[test]
    fn reports_corruption_size_missing_symlink_and_fifo_without_blocking() {
        let t = Temp::new();
        let (origin, directory) = fixture(&t);
        let first = directory.join(&origin.artifacts[0].name);
        let mut bytes = fs::read(&first).unwrap();
        bytes[0] ^= 1;
        fs::write(first, bytes).unwrap();
        fs::write(directory.join(&origin.artifacts[1].name), b"short").unwrap();
        let symlink = directory.join(&origin.artifacts[2].name);
        fs::remove_file(&symlink).unwrap();
        std::os::unix::fs::symlink("request.json", symlink).unwrap();
        let fifo = directory.join(&origin.artifacts[3].name);
        fs::remove_file(&fifo).unwrap();
        let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: the owned NUL-terminated pathname lives through mkfifo.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        fs::remove_file(directory.join(&origin.artifacts[4].name)).unwrap();
        let report = verify(&origin, &directory, &WorkControl::default()).unwrap();
        assert!(!report.passed);
        assert_eq!(report.artifacts[0].status, "crc_mismatch");
        assert_eq!(report.artifacts[1].status, "size_mismatch");
        assert_eq!(report.artifacts[2].status, "not_regular_file");
        assert_eq!(report.artifacts[3].status, "not_regular_file");
        assert!(report.artifacts[4].status.starts_with("unreadable"));
    }
    #[test]
    fn final_check_detects_mutation_of_an_already_hashed_file() {
        let t = Temp::new();
        let (origin, directory) = fixture(&t);
        let report = verify_inner(&origin, &directory, &WorkControl::default(), &|| {
            OpenOptions::new()
                .append(true)
                .open(directory.join(&origin.artifacts[0].name))?
                .write_all(b"changed")?;
            Ok(())
        })
        .unwrap();
        assert!(!report.passed);
        assert!(report.directory_unchanged);
        assert_eq!(report.artifacts[0].status, "changed_during_verification");
        let moved = t.0.join("moved");
        let report = verify_inner(&origin, &directory, &WorkControl::default(), &|| {
            fs::rename(&directory, &moved)?;
            Ok(())
        })
        .unwrap();
        assert!(!report.passed);
        assert!(!report.directory_unchanged);
    }
    #[test]
    fn cancellation_before_or_after_reads_is_fatal_and_symlink_root_is_rejected() {
        let t = Temp::new();
        let (origin, directory) = fixture(&t);
        let work = WorkControl::default();
        work.cancel();
        assert!(
            verify(&origin, &directory, &work)
                .unwrap_err()
                .to_string()
                .contains("cancelled")
        );
        let work = WorkControl::default();
        assert!(
            verify_inner(&origin, &directory, &work, &|| {
                work.cancel();
                Ok(())
            })
            .unwrap_err()
            .to_string()
            .contains("cancelled")
        );
        let link = t.0.join("link");
        std::os::unix::fs::symlink(&directory, &link).unwrap();
        assert!(verify(&origin, &link, &WorkControl::default()).is_err());
    }
    #[test]
    fn project_entry_requires_valid_origin_but_does_not_rewrite_project() {
        let t = Temp::new();
        let (origin, directory) = fixture(&t);
        let mut editor =
            scancraft_geometry::Editor::new(scancraft_geometry::Cloud { points: vec![] }, 8)
                .unwrap();
        let project = t.0.join("project.orproj");
        scancraft_geometry::formats::save_project(&editor, fs::File::create(&project).unwrap())
            .unwrap();
        assert!(verify_project(&project, &directory, &WorkControl::default()).is_err());
        editor.attach_origin(origin).unwrap();
        scancraft_geometry::formats::save_project(&editor, fs::File::create(&project).unwrap())
            .unwrap();
        let original = fs::read(&project).unwrap();
        assert!(
            verify_project(&project, &directory, &WorkControl::default())
                .unwrap()
                .passed
        );
        assert_eq!(fs::read(&project).unwrap(), original);
        let mut corrupt = original;
        corrupt.pop();
        fs::write(&project, corrupt).unwrap();
        assert!(verify_project(&project, &directory, &WorkControl::default()).is_err());
    }
}
