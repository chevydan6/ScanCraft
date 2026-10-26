//! Offline organized-sample meshing with no-overwrite artifact publication.
use super::{
    Error, artifact, bounded, digest,
    output::{publish, staging},
};
use scancraft_geometry::{mesh::from_grid, work::WorkControl};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    path::Path,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    units: String,
    source: String,
    width: u32,
    height: u32,
    samples: Vec<Option<[f64; 3]>>,
    maximum_edge_mm: f64,
}
pub fn run(input: &Path, target: &Path, work: &WorkControl) -> Result<Value, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("mesh output already exists".into());
    }
    let bytes = bounded(input, 64 * 1024 * 1024, work)?;
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.schema_version != 1
        || request.units != "millimeters"
        || !["synthetic_fixture", "unverified_imports"].contains(&request.source.as_str())
    {
        return Err("unsupported grid schema, units or source declaration".into());
    }
    let result = from_grid(
        request.width,
        request.height,
        &request.samples,
        request.maximum_edge_mm,
        work,
    )?;
    let topology = result.mesh.topology(work)?;
    let report = json!({"schema_version":1,"source":request.source,"units":"millimeters",
        "input":digest("organized_grid",&bytes),"width":request.width,"height":request.height,
        "maximum_edge_mm":request.maximum_edge_mm,"vertices":result.mesh.vertices.len(),"triangles":result.mesh.triangles.len(),
        "incomplete_cells":result.incomplete_cells,"rejected_triangles":result.rejected_triangles,"topology":topology,
        "self_intersections_checked":false,"vertex_manifoldness_checked":false,"watertightness_qualified":false,
        "physical_accuracy_qualified":false});
    let partial = staging(target)?;
    let outcome = (|| -> Result<Value, Error> {
        let mut artifacts = vec![artifact(&partial, "report.json", work, |out| {
            serde_json::to_writer_pretty(out, &report)?;
            Ok(())
        })?];
        if !result.mesh.triangles.is_empty() {
            artifacts.push(artifact(&partial, "mesh.json", work, |out| {
                serde_json::to_writer_pretty(
                    out,
                    &json!({"schema_version":1,"units":"millimeters","mesh":result.mesh}),
                )?;
                Ok(())
            })?);
            artifacts.push(artifact(&partial, "mesh.stl", work, |out| {
                result.mesh.write_stl(out, work)?;
                Ok(())
            })?);
            artifacts.push(artifact(&partial, "mesh.obj", work, |out| {
                result.mesh.write_obj(out, work)?;
                Ok(())
            })?);
        }
        let summary = json!({"schema_version":1,"vertices":result.mesh.vertices.len(),"triangles":result.mesh.triangles.len(),"status":if result.mesh.triangles.is_empty(){"no_surface"}else{"surface_produced"},"physical_accuracy_qualified":false,"artifacts":artifacts});
        artifact(&partial, "manifest.json", work, |out| {
            serde_json::to_writer_pretty(out, &summary)?;
            Ok(())
        })?;
        File::open(&partial)?.sync_all()?;
        work.check()?;
        publish(&partial, target)?;
        Ok(summary)
    })();
    let summary = outcome.map_err(|e| {
        format!(
            "{e}; incomplete mesh bundle retained at {}",
            partial.display()
        )
    })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "mesh bundle published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(summary)
}
