//! One original-preserving container for an edited project and full supplied recordings.
//! Association proves selected-frame membership, not historical identity of every frame.
use crate::{
    Error, artifact,
    output::{publish, staging},
    project_bundle, recording_archive,
    recording_playback::Playback,
    recording_reconstruction::lineage,
};
use scancraft_geometry::{Editor, formats::load_project, work::WorkControl};
use scancraft_recording::Writer;
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

#[derive(Default)]
struct Fingerprint {
    bytes: u64,
    hash: crc32fast::Hasher,
}
impl Write for Fingerprint {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes += bytes.len() as u64;
        self.hash.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn without_path(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.remove("source_path_display");
        object.remove("source_path_display_is_lossy");
    }
    value
}
pub(crate) fn associate(
    editor: &Editor,
    recordings: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    let origin = editor
        .origin()
        .ok_or("scan project requires recorded reconstruction provenance")?;
    origin.validate()?;
    let mut checks = Vec::new();
    for (role, name, retained) in [
        ("left", "left.raptor", "left-frame.raptor"),
        ("right", "right.raptor", "right-frame.raptor"),
    ] {
        work.check()?;
        let expected = &origin.recording_sources[role];
        let index = expected["selected_frame_index"]
            .as_u64()
            .ok_or("missing selected frame index")?;
        let index = usize::try_from(index)?;
        let path = recordings.join(name);
        let mut source = Playback::open(&path, work)?;
        let selected = source.select_raw(index, work)?;
        let observed = lineage(&path, source.version(), &selected)?;
        if without_path(observed) != without_path(expected.clone()) {
            return Err(format!(
                "{role} full recording disagrees with project selected-frame lineage"
            )
            .into());
        }
        let mut fingerprint = Fingerprint::default();
        let mut writer = Writer::with_context(
            &mut fingerprint,
            selected.profile,
            selected.context.unwrap_or_default(),
        )?;
        writer.push(&selected.frame)?;
        writer.finish()?;
        let reference = origin
            .artifacts
            .iter()
            .find(|a| a.name == retained)
            .ok_or("missing retained frame reference")?;
        if fingerprint.bytes != reference.bytes || fingerprint.hash.finalize() != reference.crc32 {
            return Err(
                format!("{role} selected frame differs from retained project artifact").into(),
            );
        }
        source.check_source()?;
        checks.push(json!({"role":role,"recording":format!("recordings/{name}"),"retained_frame":format!("project/{retained}"),"selected_frame_index":index,"frames":source.len(),"selected_frame_lineage":"PASS","canonical_retained_frame_fingerprint":"PASS"}));
    }
    Ok(json!(checks))
}
pub(crate) fn references(
    editor: &Editor,
    archive: &Value,
    directory: &Path,
) -> Result<scancraft_geometry::recording_references::Recordings, Error> {
    use scancraft_geometry::recording_references::{File, Recordings};
    let origin = editor
        .origin()
        .ok_or("recording references require reconstruction provenance")?;
    let artifacts = archive["artifacts"]
        .as_array()
        .ok_or("missing recording archive inventory")?;
    let one = |role: &str, name: &str| -> Result<File, Error> {
        let artifact = artifacts
            .iter()
            .find(|a| a["role"] == name)
            .ok_or("missing complete recording fingerprint")?;
        let source = &origin.recording_sources[role];
        Ok(File {
            name: name.into(),
            bytes: artifact["bytes"].as_u64().ok_or("invalid recording size")?,
            crc32: u32::try_from(
                artifact["crc32"]
                    .as_u64()
                    .ok_or("invalid recording checksum")?,
            )?,
            frames: source["source_total_frames"]
                .as_u64()
                .ok_or("invalid source frame count")?,
            version: u8::try_from(
                source["source_version"]
                    .as_u64()
                    .ok_or("invalid source version")?,
            )?,
            selected_frame_index: source["selected_frame_index"]
                .as_u64()
                .ok_or("invalid selected frame index")?,
        })
    };
    let references = Recordings {
        schema_version: 1,
        directory_display: std::path::absolute(directory)?.display().to_string(),
        files: [one("left", "left.raptor")?, one("right", "right.raptor")?],
    };
    references.validate(origin)?;
    Ok(references)
}
pub fn export(
    editor: &Editor,
    sources: &Path,
    left: &Path,
    right: &Path,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    export_inner(editor, sources, left, right, target, work, &|| Ok(()))
}
fn export_inner(
    editor: &Editor,
    sources: &Path,
    left: &Path,
    right: &Path,
    target: &Path,
    work: &WorkControl,
    before_publish: &dyn Fn() -> Result<(), Error>,
) -> Result<Value, Error> {
    work.check()?;
    editor.validate_history()?;
    editor
        .origin()
        .ok_or("scan project requires recorded reconstruction provenance")?
        .validate()?;
    if fs::symlink_metadata(target).is_ok() {
        return Err("scan project output already exists".into());
    }
    let partial = staging(target)?;
    let result = (|| -> Result<Value, Error> {
        let recordings = recording_archive::archive(left, right, &partial.join("recordings"), work)?;
        let association = associate(editor, &partial.join("recordings"), work)?;
        let candidate=editor.copied_with_recordings(references(editor,&recordings,&target.join("recordings"))?)?;
        let project = project_bundle::export_nested(&candidate, sources, &partial.join("project"), &target.join("project"), work)?;
        let report = json!({"schema_version":1,"kind":"recorded_stereo_scan_project","project":"project/project.orproj","project_sources":"project","recordings_directory":"recordings","association":association,"project_bundle":project,"recording_archive":recordings,"scope":"edited project plus complete supplied stereo recordings; selected-frame membership checked","historical_full_recording_identity_verified":false,"source_device_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,"original_capture_completeness_verified":false,"recovery_provenance":"recovery reports are not embedded; retain them separately when inputs were recovered"});
        artifact(&partial,"manifest.json",work,|out| {serde_json::to_writer_pretty(out,&report)?; Ok(())})?;
        File::open(&partial)?.sync_all()?;
        before_publish()?;
        work.check()?;
        publish(&partial,target)?;
        Ok(report)
    })().map_err(|e| format!("{e}; incomplete scan project retained at {}",partial.display()))?;
    File::open(
        target
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?
    .sync_all()
    .map_err(|e| {
        format!(
            "scan project published at {}; parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(result)
}
pub fn export_file(
    project: &Path,
    sources: &Path,
    left: &Path,
    right: &Path,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    let bytes = crate::bounded(project, 64 * 1024 * 1024 + 16, work)?;
    let editor = load_project(bytes.as_slice())?;
    export(&editor, sources, left, right, target, work)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_geometry::formats::save_project;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "scancraft-scan-project-{}-{}",
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
    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence")
    }
    fn fixture() -> (Editor, PathBuf, PathBuf, PathBuf) {
        let sources = root().join("project-origin-gui-render/result");
        let editor = load_project(File::open(sources.join("project.orproj")).unwrap()).unwrap();
        (
            editor,
            sources,
            root().join("recorded-stereo-dataset/left.raptor"),
            root().join("recorded-stereo-dataset/right.raptor"),
        )
    }
    fn encoded(editor: &Editor) -> Vec<u8> {
        let mut out = Vec::new();
        save_project(editor, &mut out).unwrap();
        out
    }
    #[test]
    fn full_recordings_and_edited_project_survive_relocation_without_originals() {
        let t = Temp::new();
        let (mut editor, sources, left, right) = fixture();
        let original = editor.cloud.clone();
        editor.trim(&[0], true).unwrap();
        let before = encoded(&editor);
        let target = t.0.join("scan");
        let report = export(
            &editor,
            &sources,
            &left,
            &right,
            &target,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(report["association"][0]["frames"], 3);
        assert_eq!(report["historical_full_recording_identity_verified"], false);
        assert_eq!(
            fs::read(target.join("recordings/left.raptor")).unwrap(),
            fs::read(&left).unwrap()
        );
        assert_eq!(
            fs::read(target.join("recordings/right.raptor")).unwrap(),
            fs::read(&right).unwrap()
        );
        assert_eq!(encoded(&editor), before);
        let moved = t.0.join("moved");
        fs::rename(&target, &moved).unwrap();
        let mut loaded =
            load_project(File::open(moved.join("project/project.orproj")).unwrap()).unwrap();
        assert_eq!(
            loaded.origin().unwrap().bundle_directory_display,
            std::path::absolute(target.join("project"))
                .unwrap()
                .display()
                .to_string()
        );
        assert!(
            crate::source_verification::verify(
                loaded.origin().unwrap(),
                &moved.join("project"),
                &WorkControl::default()
            )
            .unwrap()
            .passed
        );
        associate(&loaded, &moved.join("recordings"), &WorkControl::default()).unwrap();
        assert!(loaded.undo());
        assert_eq!(loaded.cloud, original);
    }
    #[test]
    fn wrong_recording_lineage_or_retained_artifact_cannot_publish() {
        let t = Temp::new();
        let (editor, sources, left, right) = fixture();
        let target = t.0.join("bad");
        assert!(
            export(
                &editor,
                &sources,
                &right,
                &left,
                &target,
                &WorkControl::default()
            )
            .is_err()
        );
        assert!(!target.exists());
        let mut value = serde_json::to_value(editor.origin().unwrap()).unwrap();
        value["recording_sources"]["left"]["selected_frame_index"] = json!(0);
        let origin = serde_json::from_value(value).unwrap();
        let mut altered = Editor::new(editor.cloud.clone(), 8).unwrap();
        altered.attach_origin(origin).unwrap();
        assert!(
            export(
                &altered,
                &sources,
                &left,
                &right,
                &target,
                &WorkControl::default()
            )
            .is_err()
        );
        assert!(!target.exists());
    }
    #[test]
    fn changed_retained_reference_missing_origin_and_truncated_source_are_rejected() {
        let t = Temp::new();
        let (editor, sources, left, right) = fixture();
        let mut origin = editor.origin().unwrap().clone();
        origin
            .artifacts
            .iter_mut()
            .find(|a| a.name == "left-frame.raptor")
            .unwrap()
            .crc32 ^= 1;
        let mut altered = Editor::new(editor.cloud.clone(), 8).unwrap();
        altered.attach_origin(origin).unwrap();
        let target = t.0.join("changed");
        assert!(
            export(
                &altered,
                &sources,
                &left,
                &right,
                &target,
                &WorkControl::default()
            )
            .unwrap_err()
            .to_string()
            .contains("differs from retained")
        );
        assert!(!target.exists());
        let empty = Editor::new(editor.cloud.clone(), 8).unwrap();
        assert!(
            export(
                &empty,
                &sources,
                &left,
                &right,
                &target,
                &WorkControl::default()
            )
            .is_err()
        );
        let bad = t.0.join("truncated.raptor");
        let mut bytes = fs::read(&left).unwrap();
        bytes.pop();
        fs::write(&bad, bytes).unwrap();
        assert!(
            export(
                &editor,
                &sources,
                &bad,
                &right,
                &target,
                &WorkControl::default()
            )
            .is_err()
        );
        assert!(!target.exists());
    }
    #[test]
    fn cancellation_and_publication_race_preserve_existing_output() {
        let t = Temp::new();
        let (editor, sources, left, right) = fixture();
        let target = t.0.join("cancelled");
        let work = WorkControl::default();
        let result = export_inner(&editor, &sources, &left, &right, &target, &work, &|| {
            work.cancel();
            Ok(())
        });
        assert!(result.unwrap_err().to_string().contains("cancelled"));
        assert!(!target.exists());
        let target = t.0.join("race");
        assert!(
            export_inner(
                &editor,
                &sources,
                &left,
                &right,
                &target,
                &WorkControl::default(),
                &|| {
                    fs::create_dir(&target)?;
                    fs::write(target.join("keep"), b"sentinel")?;
                    Ok(())
                }
            )
            .is_err()
        );
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"sentinel");
        assert!(!target.join("manifest.json").exists());
    }
}
