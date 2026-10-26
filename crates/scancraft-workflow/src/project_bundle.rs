//! Original-preserving export of an edited project and its retained source artifacts.
use crate::{
    Error, artifact,
    output::{publish, staging},
    source_verification,
};
use scancraft_geometry::{
    Editor,
    formats::{load_project, save_project},
    work::WorkControl,
};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
pub fn export(
    editor: &Editor,
    sources: &Path,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    export_inner(editor, sources, target, work, &|| Ok(()))
}
// Explicit boundary for deterministic cancellation/publication-race regressions.
fn export_inner(
    editor: &Editor,
    sources: &Path,
    target: &Path,
    work: &WorkControl,
    before_publish: &dyn Fn() -> Result<(), Error>,
) -> Result<Value, Error> {
    export_inner_at(editor, sources, target, target, work, before_publish)
}
pub(crate) fn export_nested(
    editor: &Editor,
    sources: &Path,
    target: &Path,
    final_location: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    export_inner_at(editor, sources, target, final_location, work, &|| Ok(()))
}
fn export_inner_at(
    editor: &Editor,
    sources: &Path,
    target: &Path,
    final_location: &Path,
    work: &WorkControl,
    before_publish: &dyn Fn() -> Result<(), Error>,
) -> Result<Value, Error> {
    work.check()?;
    editor.validate_history()?;
    if fs::symlink_metadata(target).is_ok() {
        return Err("project bundle output already exists".into());
    }
    let origin = editor
        .origin()
        .ok_or("project has no reconstruction provenance")?;
    // Reject oversized documents before cloning history or allocating staging.
    let mut preflight = DigestSink::default();
    save_project(editor, &mut preflight)?;
    work.check()?;
    let checked = source_verification::verify(origin, sources, work)?;
    if !checked.passed {
        return Err("source artifacts do not match project references; bundle not created".into());
    }
    let destination = std::path::absolute(final_location)?;
    let partial = staging(target)?;
    let outcome=(||->Result<Value,Error> {
        let mut artifacts=Vec::new();
        for expected in &origin.artifacts {
            let digest=artifact(&partial,&expected.name,work,|out| {
                let mut input=OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW|libc::O_NONBLOCK).open(sources.join(&expected.name))?;
                let metadata=input.metadata()?;
                if !metadata.is_file() || metadata.len()!=expected.bytes{return Err("source artifact changed before copy".into());}
                let mut buffer=[0u8;65536];let mut total=0u64;
                while total<=expected.bytes {
                    work.check()?;
                    let count=input.read(&mut buffer[..(expected.bytes+1-total).min(65536) as usize])?;
                    if count==0 {break;}
                    total+=count as u64;
                    if total>expected.bytes{return Err("source artifact grew during copy".into());}
                    out.write_all(&buffer[..count])?;
                }
                if total!=expected.bytes{return Err("source artifact shortened during copy".into());}
                Ok(())
            })?;
            if digest.bytes!=expected.bytes || digest.crc32!=expected.crc32{return Err("copied source artifact checksum disagrees with project reference".into());}
            artifacts.push(digest);
        }
        let candidate=editor.copied_with_source_directory(destination.display().to_string())?;
        let project=artifact(&partial,"project.orproj",work,|out|{save_project(&candidate,out)?;Ok(())})?;
        drop(candidate);
        // Canonical full-state replay covers geometry, both histories and metadata.
        let reopened=load_project(BufReader::new(File::open(partial.join("project.orproj"))?))?;
        let mut replay=DigestSink::default();save_project(&reopened,&mut replay)?;
        if replay.bytes!=project.bytes || replay.hash.finalize()!=project.crc32{return Err("packaged project full-state replay mismatch".into());}
        drop(reopened);
        artifacts.push(project);
        let copied=source_verification::verify(origin,&partial,work)?;
        if !copied.passed{return Err("copied source bundle failed verification".into());}
        let details=json!({"schema_version":1,"scope":"edited project and five retained initial reconstruction artifacts; selected raw frames only","source_directory_display":sources.display().to_string(),"destination_directory_display":destination.display().to_string(),"source_check":checked,"copied_check":copied,"project_full_state_replay":"PASS","original_project_and_source_bytes_preserved":true,"source_device_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false});
        artifacts.push(artifact(&partial,"bundle-report.json",work,|out|{serde_json::to_writer_pretty(out,&details)?;Ok(())})?);
        let summary=json!({"schema_version":1,"scope":"selected-frame project bundle","points":editor.cloud.points.len(),"project_full_state_replay":"PASS","copied_source_artifacts":origin.artifacts.len(),"original_project_and_source_bytes_preserved":true,"source_device_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,"artifacts":artifacts});
        artifact(&partial,"manifest.json",work,|out|{serde_json::to_writer_pretty(out,&summary)?;Ok(())})?;
        File::open(&partial)?.sync_all()?;
        before_publish()?;work.check()?;
        if !source_verification::verify(origin,sources,work)?.passed{return Err("source artifacts changed before publication".into());}
        work.check()?;publish(&partial,target)?;Ok(summary)
    })().map_err(|e|format!("{e}; incomplete project bundle retained at {}",partial.display()))?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|e| {
            format!(
                "project bundle was published at {}; parent directory sync failed: {e}",
                target.display()
            )
        })?;
    Ok(outcome)
}
#[derive(Default)]
struct DigestSink {
    hash: crc32fast::Hasher,
    bytes: u64,
}
impl Write for DigestSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.hash.update(bytes);
        self.bytes += bytes.len() as u64;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub fn export_file(
    project: &Path,
    sources: &Path,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    work.check()?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(project)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 + 16 {
        return Err("bundle export requires a regular bounded project".into());
    }
    let editor = load_project(BufReader::new(file))?;
    export(&editor, sources, target, work)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "scancraft-project-bundle-{}-{}",
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
    fn fixture(temp: &Temp) -> (Editor, PathBuf) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/project-origin-gui-render/result");
        let sources = temp.0.join("sources");
        fs::create_dir(&sources).unwrap();
        for entry in fs::read_dir(&root).unwrap() {
            let path = entry.unwrap().path();
            fs::copy(&path, sources.join(path.file_name().unwrap())).unwrap();
        }
        (
            load_project(File::open(sources.join("project.orproj")).unwrap()).unwrap(),
            sources,
        )
    }
    fn encoded(editor: &Editor) -> Vec<u8> {
        let mut bytes = Vec::new();
        save_project(editor, &mut bytes).unwrap();
        bytes
    }
    #[test]
    fn export_preserves_edited_state_histories_sources_and_original_document() {
        let temp = Temp::new();
        let (mut editor, sources) = fixture(&temp);
        let initial = editor.cloud.clone();
        let points = editor
            .cloud
            .points
            .iter()
            .take(2)
            .map(|p| p.position)
            .collect();
        let snapshot = scancraft_geometry::measurement_snapshot::Snapshot::distance(
            "fixture distance".into(),
            points,
            &WorkControl::default(),
        )
        .unwrap();
        editor.add_measurement(snapshot).unwrap();
        editor.trim(&[0], true).unwrap();
        let original = encoded(&editor);
        let source_project = fs::read(sources.join("project.orproj")).unwrap();
        let target = temp.0.join("exported");
        let summary = export(&editor, &sources, &target, &WorkControl::default()).unwrap();
        assert_eq!(summary["copied_source_artifacts"], 5);
        assert_eq!(encoded(&editor), original);
        assert_eq!(
            fs::read(sources.join("project.orproj")).unwrap(),
            source_project
        );
        let mut copied = load_project(File::open(target.join("project.orproj")).unwrap()).unwrap();
        let expected = editor
            .copied_with_source_directory(
                std::path::absolute(&target).unwrap().display().to_string(),
            )
            .unwrap();
        assert_eq!(encoded(&copied), encoded(&expected));
        assert_eq!(copied.cloud.points.len(), 1);
        assert_eq!(copied.measurements().len(), 1);
        assert!(copied.undo());
        assert_eq!(copied.cloud, initial);
        assert!(copied.redo());
        assert!(copied.undo_measurement().unwrap());
        assert!(copied.redo_measurement().unwrap());
        for artifact in &editor.origin().unwrap().artifacts {
            assert_eq!(
                fs::read(sources.join(&artifact.name)).unwrap(),
                fs::read(target.join(&artifact.name)).unwrap()
            );
        }
        fs::rename(&sources, temp.0.join("old-sources-moved")).unwrap();
        assert!(
            source_verification::verify_project(
                &target.join("project.orproj"),
                &target,
                &WorkControl::default()
            )
            .unwrap()
            .passed
        );
    }
    #[test]
    fn existing_output_and_bad_sources_are_refused_before_staging() {
        let temp = Temp::new();
        let (editor, sources) = fixture(&temp);
        let target = temp.0.join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), b"sentinel").unwrap();
        assert!(export(&editor, &sources, &target, &WorkControl::default()).is_err());
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"sentinel");
        let count = fs::read_dir(&temp.0).unwrap().count();
        fs::write(sources.join("request.json"), b"broken").unwrap();
        assert!(
            export(
                &editor,
                &sources,
                &temp.0.join("bad"),
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), count);
    }
    #[test]
    fn cancellation_after_copy_retains_staging_without_publication_or_original_changes() {
        let temp = Temp::new();
        let (editor, sources) = fixture(&temp);
        let before = encoded(&editor);
        let target = temp.0.join("cancelled");
        let work = WorkControl::default();
        let error = export_inner(&editor, &sources, &target, &work, &|| {
            work.cancel();
            Ok(())
        })
        .unwrap_err();
        assert!(error.to_string().contains("cancelled"));
        assert!(error.to_string().contains("retained at"));
        assert!(!target.exists());
        assert_eq!(encoded(&editor), before);
        assert!(fs::read_dir(&temp.0).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("cancelled.partial-")
        }));
        assert!(
            source_verification::verify(
                editor.origin().unwrap(),
                &sources,
                &WorkControl::default()
            )
            .unwrap()
            .passed
        );
    }
    #[test]
    fn source_change_and_destination_race_cannot_publish_over_existing_data() {
        let temp = Temp::new();
        let (editor, sources) = fixture(&temp);
        let path = sources.join("request.json");
        let original = fs::read(&path).unwrap();
        let target = temp.0.join("changed");
        assert!(
            export_inner(&editor, &sources, &target, &WorkControl::default(), &|| {
                fs::write(&path, b"changed")?;
                Ok(())
            })
            .is_err()
        );
        assert!(!target.exists());
        fs::write(path, original).unwrap();
        let target = temp.0.join("race");
        assert!(
            export_inner(&editor, &sources, &target, &WorkControl::default(), &|| {
                fs::create_dir(&target)?;
                fs::write(target.join("keep"), b"sentinel")?;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"sentinel");
        assert!(!target.join("project.orproj").exists());
    }
    #[test]
    fn file_entry_rejects_bad_project_and_precancellation_without_creating_output() {
        let temp = Temp::new();
        let (_, sources) = fixture(&temp);
        let target = temp.0.join("output");
        let work = WorkControl::default();
        work.cancel();
        assert!(export_file(&sources.join("project.orproj"), &sources, &target, &work).is_err());
        assert!(!target.exists());
        let bad = temp.0.join("bad.orproj");
        fs::write(&bad, b"invalid").unwrap();
        assert!(export_file(&bad, &sources, &target, &WorkControl::default()).is_err());
        let link = temp.0.join("link");
        std::os::unix::fs::symlink(sources.join("project.orproj"), &link).unwrap();
        assert!(export_file(&link, &sources, &target, &WorkControl::default()).is_err());
        assert!(!target.exists());
    }
    #[test]
    fn oversized_document_is_rejected_before_staging_or_source_access() {
        let temp = Temp::new();
        let (mut editor, _) = fixture(&temp);
        let point = scancraft_geometry::Point {
            position: [1.2345678901234567, 2.345678901234567, 3.456789012345678],
            normal: Some([
                0.12345678901234567,
                0.23456789012345678,
                0.345_678_901_234_567_9,
            ]),
            color: Some([1, 2, 3]),
        };
        let encoded_size = serde_json::to_vec(&point).unwrap().len();
        editor.cloud = scancraft_geometry::Cloud {
            points: vec![point; 64 * 1024 * 1024 / encoded_size + 1],
        };
        let target = temp.0.join("oversized");
        let error = export(
            &editor,
            &temp.0.join("missing-sources"),
            &target,
            &WorkControl::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("64 MiB"), "{error}");
        assert!(!target.exists());
        assert!(!fs::read_dir(&temp.0).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("oversized.partial-")
        }));
    }
}
