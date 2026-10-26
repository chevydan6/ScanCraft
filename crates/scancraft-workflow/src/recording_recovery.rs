//! Original-preserving, replay-verified publication of a recovered recording prefix.
use super::{
    Error, artifact,
    output::{publish, staging},
};
use scancraft_geometry::work::{Stage, WorkControl};
use scancraft_recording::{Reader, recovery};
use serde_json::{Value, json};
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::{self, BufReader, Read},
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
pub fn run(
    input: &Path,
    target: &Path,
    maximum_frames: u64,
    work: &WorkControl,
    checkpoint: &dyn Fn() -> Result<(), Error>,
) -> Result<Value, Error> {
    checkpoint()?;
    work.check()?;
    if !(1..=recovery::MAX_RECOVERY_FRAMES).contains(&maximum_frames) {
        return Err("recovery frame budget must be 1..1000000".into());
    }
    if fs::symlink_metadata(target).is_ok() {
        return Err("recovery output already exists".into());
    }
    let source = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
        .open(input)?;
    let initial = source.metadata()?;
    if !initial.is_file() || initial.len() > (1_u64 << 40).min((usize::MAX / 2) as u64) {
        return Err("recovery requires a regular source file no larger than 1 TiB".into());
    }
    let initial_stamp = stamp(&initial);
    let partial = staging(target)?;
    let outcome=(||->Result<Value,Error> {
        let mut report=None;
        let recording=artifact(&partial,"recovered.raptor",work,|out| {
            report=Some(recovery::recover_prefix(BufReader::new((&source).take(initial.len())),out,maximum_frames,&|offset| {
                checkpoint().map_err(io::Error::other)?;
                work.update(Stage::Reading,offset as usize,initial.len() as usize).map_err(io::Error::other)
            })?);Ok(())
        })?;
        let report=report.expect("successful recovery returns a report");
        let mut reader=Reader::new(BufReader::new(File::open(partial.join("recovered.raptor"))?))?;
        let mut count=0;
        loop {checkpoint()?;work.check()?;if reader.read_frame()?.is_none() {break;}count+=1;}
        if count!=report.recovered_frames || reader.profile!=report.profile {return Err("recovered recording replay mismatch".into());}
        let details=json!({"schema_version":1,"source_bytes":initial.len(),"maximum_frames":maximum_frames,"recovery":report,"output_recording_version":reader.version,"full_crc_footer_replay":"PASS","original_source_preserved":true,"recovered_prefix_only":true,"physical_accuracy_qualified":false});
        let report_artifact=artifact(&partial,"report.json",work,|out| {serde_json::to_writer_pretty(out,&details)?;Ok(())})?;
        let summary=json!({"schema_version":1,"recovered_frames":count,"recovered_prefix_only":true,"source_complete":report.source_complete,"source_version":report.source_version,"source_header_crc_verified":report.source_header_crc_verified,"stop_reason":report.stop_reason,"full_crc_footer_replay":"PASS","original_source_preserved":true,"physical_accuracy_qualified":false,"artifacts":[recording,report_artifact]});
        artifact(&partial,"manifest.json",work,|out| {serde_json::to_writer_pretty(out,&summary)?;Ok(())})?;
        File::open(&partial)?.sync_all()?;
        checkpoint()?;work.check()?;
        let current=fs::symlink_metadata(input)?;
        if !current.is_file() || stamp(&source.metadata()?)!=initial_stamp || stamp(&current)!=initial_stamp {return Err("recording source changed during recovery".into());}
        publish(&partial,target)?;
        Ok(summary)
    })().map_err(|e|format!("{e}; incomplete recovery retained at {}",partial.display()))?;
    File::open(
        target
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?
    .sync_all()?;
    Ok(outcome)
}
