//! Shared exact-request, unqualified calibration-fit publication.
use crate::{
    Error, artifact, digest,
    output::{publish, staging},
};
use scancraft_geometry::work::WorkControl;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};
/// A supplied declaration, never hardware/accuracy attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationProvenance {
    SyntheticFixture,
    UserSuppliedUnverified,
}
/// Coordinate numerical and workflow cancellation before irreversible publication.
pub(crate) fn check(work: &WorkControl, cancel: &dyn Fn() -> bool) -> Result<(), Error> {
    work.check()?;
    if cancel() {
        return Err("offline calibration cancelled".into());
    }
    Ok(())
}
/// Reject every existing destination, including dangling symlinks.
pub(crate) fn ensure_new(target: &Path) -> Result<(), Error> {
    match fs::symlink_metadata(target) {
        Ok(_) => Err("calibration output already exists; choose a new directory".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
/// Preserve exact request bytes and publish a fixed-name diagnostic bundle atomically.
pub(crate) fn publish_fit(
    bytes: &[u8],
    report: &Value,
    target: &Path,
    work: &WorkControl,
    cancel: &dyn Fn() -> bool,
    scope: &'static str,
) -> Result<Value, Error> {
    let partial = staging(target)?;
    let result=(||->Result<Value,Error>{
        let request_digest=artifact(&partial,"observations.json",work,|out|{out.write_all(bytes)?;Ok(())})?;
        check(work,cancel)?;
        let fit_digest=artifact(&partial,"fit.json",work,|out|{serde_json::to_writer_pretty(out,report)?;Ok(())})?;
        let manifest=json!({"schema_version":1,"scope":scope,"input":digest("observations.json",bytes),"artifacts":[request_digest,fit_digest],"device_binding_verified":false,"physical_accuracy_qualified":false});
        artifact(&partial,"manifest.json",work,|out|{serde_json::to_writer_pretty(out,&manifest)?;Ok(())})?;
        File::open(&partial)?.sync_all()?;
        check(work,cancel)?;
        publish(&partial,target)?;
        Ok(manifest)
    })().map_err(|e|format!("{e}; incomplete calibration bundle retained at {}",partial.display()))?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "calibration bundle published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(result)
}
