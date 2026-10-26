//! Strict bounded mesh-envelope loading; imported data remains unqualified.
use super::{Error, bounded};
use scancraft_geometry::{
    mesh::{Mesh, Topology},
    work::WorkControl,
};
use serde::Deserialize;
use std::path::Path;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema_version: u32,
    units: String,
    mesh: Mesh,
}
pub struct LoadedMesh {
    pub mesh: Mesh,
    pub topology: Topology,
}
pub fn load(path: &Path, work: &WorkControl) -> Result<LoadedMesh, Error> {
    let bytes = bounded(path, 128 * 1024 * 1024, work)?;
    decode(&bytes, work)
}
pub fn decode(bytes: &[u8], work: &WorkControl) -> Result<LoadedMesh, Error> {
    work.check()?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("mesh input exceeds byte budget".into());
    }
    let envelope: Envelope = serde_json::from_slice(bytes)?;
    if envelope.schema_version != 1 || envelope.units != "millimeters" {
        return Err("unsupported mesh schema or units".into());
    }
    let topology = envelope.mesh.topology(work)?;
    work.check()?;
    Ok(LoadedMesh {
        mesh: envelope.mesh,
        topology,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    #[test]
    fn bounded_loader_requires_schema_units_valid_indices_and_respects_cancellation() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "scancraft-mesh-load-{}-{}.json",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let work = WorkControl::default();
        let valid = serde_json::json!({"schema_version":1,"units":"millimeters","mesh":{"vertices":[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]],"triangles":[[0,1,2]]}});
        std::fs::write(&path, serde_json::to_vec(&valid).unwrap()).unwrap();
        let loaded = load(&path, &work).unwrap();
        assert_eq!(loaded.topology.boundary_edges, 3);
        for kind in 0..3 {
            let mut bad = valid.clone();
            match kind {
                0 => bad["units"] = serde_json::json!("unknown"),
                1 => bad["mesh"]["triangles"][0][0] = serde_json::json!(99),
                _ => bad["schema_version"] = serde_json::json!(2),
            };
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(load(&path, &work).is_err());
        }
        std::fs::write(&path, b"truncated").unwrap();
        assert!(load(&path, &work).is_err());
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(load(&path, &cancelled).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
