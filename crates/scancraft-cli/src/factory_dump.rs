//! Read existing bounded factory bytes; never decode/apply or modify calibration.
use crate::output::{publish, staging};
use scancraft_device::{
    SessionLock,
    control::{self, FactoryHeader, FactoryRegion},
};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};
pub fn run(
    region: FactoryRegion,
    target: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("factory output already exists".into());
    }
    let _lock = SessionLock::acquire()?;
    let bytes = control::factory_region(region)?;
    let manifest = serde_json::json!({"schema_version":1,"read_only_opcode":25,"region_start":region.base(),"bytes":bytes.len(),"crc32":crc32fast::hash(&bytes),"header":FactoryHeader::parse(&bytes[..32])?,"file":"factory-region.bin","optical_calibration_decoded":false,"calibration_performed":false,"firmware_modified":false,"interpretation":"Opaque existing factory data, not an application calibration record"});
    let partial = staging(target)?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(partial.join("factory-region.bin"))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(partial.join("manifest.json"))?;
        serde_json::to_writer_pretty(&mut file, &manifest)?;
        file.flush()?;
        file.sync_all()?;
        File::open(&partial)?.sync_all()?;
        publish(&partial, target)?;
        Ok(())
    })();
    result.map_err(|e| {
        format!(
            "{e}; incomplete factory output retained at {}",
            partial.display()
        )
    })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent)?.sync_all()?;
    Ok(manifest)
}
