//! Preserve complete supplied recordings byte-for-byte after strict full replay.
use crate::{
    Error, artifact,
    output::{publish, staging},
};
use scancraft_geometry::work::{Stage, WorkControl};
use scancraft_recording::Reader;
use serde_json::{Value, json};
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::{self, BufReader, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
pub const MAX_FRAMES: u64 = 1_000_000;
const MAX_BYTES: u64 = 1 << 40;
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
struct Source<'a> {
    path: &'a Path,
    file: File,
    initial: Metadata,
}
impl<'a> Source<'a> {
    fn open(path: &'a Path) -> Result<Self, Error> {
        let before = fs::symlink_metadata(path)?;
        if !before.is_file() || before.len() > MAX_BYTES.min((usize::MAX / 2) as u64) {
            return Err(
                "archive requires a bounded regular source file, without final symlinks".into(),
            );
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(path)?;
        let initial = file.metadata()?;
        if stamp(&initial) != stamp(&before) {
            return Err("recording changed while opening archive source".into());
        }
        if !initial.is_file() || initial.len() > MAX_BYTES.min((usize::MAX / 2) as u64) {
            return Err("archive source must be a regular file no larger than 1 TiB".into());
        }
        Ok(Self {
            path,
            file,
            initial,
        })
    }
    fn unchanged(&self) -> Result<(), Error> {
        let current = fs::symlink_metadata(self.path)?;
        if !current.is_file()
            || stamp(&current) != stamp(&self.initial)
            || stamp(&self.file.metadata()?) != stamp(&self.initial)
        {
            return Err("recording changed during archive; source was not written".into());
        }
        Ok(())
    }
}
struct CopyReader<'a, R> {
    input: R,
    output: &'a mut dyn Write,
    work: &'a WorkControl,
    bytes: u64,
    total: u64,
    hash: crc32fast::Hasher,
}
impl<R: Read> Read for CopyReader<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.work
            .update(Stage::Reading, self.bytes as usize, self.total as usize)
            .map_err(io::Error::other)?;
        let capacity = out.len().min(65536);
        let n = self.input.read(&mut out[..capacity])?;
        self.output.write_all(&out[..n])?;
        self.hash.update(&out[..n]);
        self.bytes += n as u64;
        Ok(n)
    }
}
fn inspect(
    input: impl Read,
    interface: u8,
    limit: u64,
    work: &WorkControl,
) -> Result<Value, Error> {
    let mut reader = Reader::new(input)?;
    if reader.profile.interface != interface {
        return Err(format!("expected recording interface {interface}").into());
    }
    let mut frames = 0u64;
    let mut first = None;
    let mut last = None;
    let mut previous = None;
    let mut gaps = 0u64;
    loop {
        work.check()?;
        let Some(frame) = reader.read_frame()? else {
            break;
        };
        if frames == limit {
            return Err("recording exceeds archive frame budget".into());
        }
        let pts = scancraft_core::uvc_pts(&frame.metadata)?;
        let info = json!({"sequence":frame.sequence,"host_timestamp_us":frame.host_timestamp_us,"sensor_pts":pts});
        if first.is_none() {
            first = Some(info.clone());
        }
        last = Some(info);
        if let Some(p) = previous {
            gaps += u64::from(frame.sequence.wrapping_sub(p) - 1);
        }
        previous = Some(frame.sequence);
        frames += 1;
        work.update(Stage::Validating, frames as usize, 0)?;
    }
    if frames == 0 {
        return Err("archive recording contains no frames".into());
    }
    Ok(
        json!({"version":reader.version,"header_crc_verified":reader.version>=2,"profile":reader.profile,"context":reader.context,"frames":frames,"first_frame":first,"last_frame":last,"accepted_sequence_gaps":gaps,"frame_order_crc_footer_eof":"PASS","images_decoded":false}),
    )
}
pub fn archive(
    left: &Path,
    right: &Path,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    archive_inner(left, right, target, MAX_FRAMES, work, &|| Ok(()))
}
fn archive_inner(
    left: &Path,
    right: &Path,
    target: &Path,
    limit: u64,
    work: &WorkControl,
    before_publish: &dyn Fn() -> Result<(), Error>,
) -> Result<Value, Error> {
    work.check()?;
    if limit == 0 || limit > MAX_FRAMES {
        return Err("archive frame budget must be 1..1000000".into());
    }
    if fs::symlink_metadata(target).is_ok() {
        return Err("recording archive output already exists".into());
    }
    let sources = [Source::open(left)?, Source::open(right)?];
    let partial = staging(target)?;
    let summary=(||->Result<Value,Error>{
        let mut files=Vec::new();let mut artifacts=Vec::new();
        for (source,(name,interface)) in sources.iter().zip([("left.raptor",0),("right.raptor",2)]) {
            let mut checked=None;let mut copied_bytes=0;let mut copied_crc=0;
            let digest=artifact(&partial,name,work,|out|{
                let mut tee=CopyReader{input:BufReader::new((&source.file).take(source.initial.len())),output:out,work,bytes:0,total:source.initial.len(),hash:crc32fast::Hasher::new()};
                checked=Some(inspect(&mut tee,interface,limit,work)?);copied_bytes=tee.bytes;copied_crc=tee.hash.finalize();Ok(())
            })?;
            source.unchanged()?;
            if copied_bytes!=source.initial.len() || copied_bytes!=digest.bytes || copied_crc!=digest.crc32{return Err("archived recording byte copy mismatch".into());}
            let replay=inspect(BufReader::new(File::open(partial.join(name))?),interface,limit,work)?;
            if checked.as_ref()!=Some(&replay){return Err("archived recording full replay mismatch".into());}
            files.push(json!({"name":name,"source_path_display":source.path.display().to_string(),"bytes":digest.bytes,"crc32":digest.crc32,"recording":replay,"byte_exact_copy_verified":true}));artifacts.push(digest);
        }
        let report=json!({"schema_version":1,"scope":"complete supplied left/right recording files; no automatic frame pairing","files":files,"source_device_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,"original_capture_completeness_verified":false,"recovery_provenance":"keep recovery reports with recovered inputs; valid footers do not prove original scan completeness","original_source_bytes_preserved":true,"artifacts":artifacts});
        artifact(&partial,"manifest.json",work,|out|{serde_json::to_writer_pretty(out,&report)?;Ok(())})?;
        File::open(&partial)?.sync_all()?;before_publish()?;work.check()?;for source in &sources {source.unchanged()?;}work.check()?;publish(&partial,target)?;Ok(report)
    })().map_err(|e|format!("{e}; incomplete archive retained at {}",partial.display()))?;
    File::open(
        target
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )
    .and_then(|f| f.sync_all())
    .map_err(|e| {
        format!(
            "recording archive was published at {}; parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(summary)
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
                "scancraft-full-archive-{}-{}",
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
    fn fixture(t: &Temp) -> [PathBuf; 2] {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/recorded-stereo-dataset");
        ["left.raptor", "right.raptor"].map(|name| {
            let p = t.0.join(name);
            fs::copy(root.join(name), &p).unwrap();
            p
        })
    }
    #[test]
    fn complete_files_metadata_and_all_frames_are_preserved_without_source_changes() {
        let t = Temp::new();
        let [left, right] = fixture(&t);
        let before = [fs::read(&left).unwrap(), fs::read(&right).unwrap()];
        let target = t.0.join("archive");
        let report = archive(&left, &right, &target, &WorkControl::default()).unwrap();
        for (i, name) in ["left.raptor", "right.raptor"].into_iter().enumerate() {
            assert_eq!(fs::read(target.join(name)).unwrap(), before[i]);
            assert_eq!(report["files"][i]["recording"]["frames"], 3);
            assert_eq!(
                report["files"][i]["recording"]["context"]["controls"]["synthetic"],
                true
            );
            assert_eq!(report["files"][i]["recording"]["accepted_sequence_gaps"], 0);
        }
        assert_eq!(fs::read(left).unwrap(), before[0]);
        assert_eq!(fs::read(right).unwrap(), before[1]);
        assert_eq!(report["synchronization_verified"], false);
        assert_eq!(report["original_capture_completeness_verified"], false);
    }
    #[test]
    fn legacy_header_is_not_rewritten_or_retroactively_crc_qualified() {
        let t = Temp::new();
        let [left, right] = fixture(&t);
        let b = fs::read(&left).unwrap();
        let n = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
        let h: Value = serde_json::from_slice(&b[16..16 + n]).unwrap();
        let p = serde_json::to_vec(&h["profile"]).unwrap();
        let mut old = b"RAPTOR01".to_vec();
        old.extend_from_slice(&(p.len() as u32).to_le_bytes());
        old.extend_from_slice(&p);
        old.extend_from_slice(&b[16 + n..]);
        fs::write(&left, &old).unwrap();
        let target = t.0.join("archive");
        let report = archive(&left, &right, &target, &WorkControl::default()).unwrap();
        assert_eq!(fs::read(target.join("left.raptor")).unwrap(), old);
        assert_eq!(report["files"][0]["recording"]["version"], 1);
        assert_eq!(
            report["files"][0]["recording"]["header_crc_verified"],
            false
        );
        assert!(report["files"][0]["recording"]["context"].is_null());
    }
    #[test]
    fn invalid_suffix_budget_wrong_role_and_empty_recordings_never_publish() {
        let t = Temp::new();
        let [left, right] = fixture(&t);
        let original = fs::read(&left).unwrap();
        let mut bad = original.clone();
        bad.pop();
        fs::write(&left, bad).unwrap();
        let target = t.0.join("bad");
        assert!(archive(&left, &right, &target, &WorkControl::default()).is_err());
        assert!(!target.exists());
        let mut corrupt = original.clone();
        let at = corrupt.len() - 13;
        corrupt[at] ^= 1;
        fs::write(&left, corrupt).unwrap();
        assert!(archive(&left, &right, &target, &WorkControl::default()).is_err());
        assert!(!target.exists());
        fs::write(&left, &original).unwrap();
        assert!(
            archive_inner(&left, &right, &target, 2, &WorkControl::default(), &|| Ok(
                ()
            ))
            .is_err()
        );
        assert!(!target.exists());
        assert!(archive(&right, &left, &target, &WorkControl::default()).is_err());
        assert!(!target.exists());
        let reader = Reader::new(original.as_slice()).unwrap();
        let bytes = scancraft_recording::Writer::new(Vec::new(), reader.profile)
            .unwrap()
            .finish()
            .unwrap();
        fs::write(&left, bytes).unwrap();
        assert!(archive(&left, &right, &target, &WorkControl::default()).is_err());
        assert!(!target.exists());
    }
    #[test]
    fn cancellation_after_full_copy_retains_staging_and_preserves_originals() {
        let t = Temp::new();
        let [left, right] = fixture(&t);
        let original = fs::read(&left).unwrap();
        let target = t.0.join("cancelled");
        let work = WorkControl::default();
        assert!(
            archive_inner(&left, &right, &target, MAX_FRAMES, &work, &|| {
                work.cancel();
                Ok(())
            })
            .unwrap_err()
            .to_string()
            .contains("cancelled")
        );
        assert!(!target.exists());
        assert_eq!(fs::read(&left).unwrap(), original);
        assert!(fs::read_dir(&t.0).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("cancelled.partial-")
        }));
        assert!(archive(&left, &right, &target, &work).is_err());
    }
    #[test]
    fn source_mutation_and_destination_race_cannot_clobber_or_publish() {
        let t = Temp::new();
        let [left, right] = fixture(&t);
        let original = fs::read(&left).unwrap();
        let target = t.0.join("changed");
        assert!(
            archive_inner(
                &left,
                &right,
                &target,
                MAX_FRAMES,
                &WorkControl::default(),
                &|| {
                    fs::write(&left, b"changed")?;
                    Ok(())
                }
            )
            .is_err()
        );
        assert!(!target.exists());
        fs::write(&left, original).unwrap();
        let target = t.0.join("race");
        assert!(
            archive_inner(
                &left,
                &right,
                &target,
                MAX_FRAMES,
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
        assert!(!target.join("left.raptor").exists());
        assert!(
            archive(&left, &right, &target, &WorkControl::default())
                .unwrap_err()
                .to_string()
                .contains("already exists")
        );
    }
    #[test]
    fn source_symlinks_and_fifos_are_rejected_without_blocking() {
        let t = Temp::new();
        let [left, right] = fixture(&t);
        let link = t.0.join("link");
        std::os::unix::fs::symlink(&left, &link).unwrap();
        assert!(archive(&link, &right, &t.0.join("output"), &WorkControl::default()).is_err());
        let fifo = t.0.join("fifo");
        let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: the owned NUL-terminated path lives through mkfifo.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(archive(&fifo, &right, &t.0.join("output"), &WorkControl::default()).is_err());
        assert!(!t.0.join("output").exists());
    }
}
