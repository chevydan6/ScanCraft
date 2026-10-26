//! Original-preserving mesh edit bundles: trims, cleanup, components and explicit fills.
use super::{
    Error, artifact, bounded, digest, mesh_document,
    output::{publish, staging},
};
use scancraft_geometry::{
    mesh_edit::{CleanupOptions, MeshEditor, cleanup, trim_plane},
    work::WorkControl,
};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};
pub struct PreparedTrim {
    bytes: Vec<u8>,
    editor: MeshEditor,
    report: Value,
    original_name: &'static str,
}
impl PreparedTrim {
    pub fn mesh(&self) -> &scancraft_geometry::mesh::Mesh {
        self.editor.mesh()
    }
    pub fn original_mesh(&self) -> &scancraft_geometry::mesh::Mesh {
        self.editor
            .previous_mesh()
            .expect("prepared trim retains original")
    }
    pub fn report(&self) -> &Value {
        &self.report
    }
}
/// Import external triangle geometry with explicit units and welding, preserving raw bytes.
/// Imported projects start without an edit history; original_mesh() is not exposed here.
pub fn import_mesh(
    input: &Path,
    config: scancraft_geometry::mesh_import::Config,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("mesh import output already exists".into());
    }
    let bytes = bounded(input, 128 * 1024 * 1024, work)?;
    let (mesh, details) = scancraft_geometry::mesh_import::decode(&bytes, &config, work)?;
    let topology = mesh.topology(work)?;
    let report = json!({"schema_version":1,"operation":"import_mesh","units":"millimeters","input":digest("external_mesh",&bytes),"parameters":{"configuration":config,"import":details},"topology":topology,"cut_edges":0,"removed_faces":0,"removed_vertices":0,"physical_accuracy_qualified":false,"watertightness_qualified":false,"self_intersections_checked":false});
    let original_name = match config.format {
        scancraft_geometry::mesh_import::Format::Obj => "original.obj",
        scancraft_geometry::mesh_import::Format::Stl => "original.stl",
    };
    let prepared = PreparedTrim {
        bytes,
        editor: MeshEditor::new(mesh, 32, work)?,
        report,
        original_name,
    };
    publish_prepared(&prepared, target, work)
}
pub fn run(
    input: &Path,
    normal: [f64; 3],
    offset_mm: f64,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("trim output already exists".into());
    }
    let prepared = prepare(input, normal, offset_mm, work)?;
    publish_prepared(&prepared, target, work)
}
pub fn prepare(
    input: &Path,
    normal: [f64; 3],
    offset_mm: f64,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    let bytes = bounded(input, 128 * 1024 * 1024 + 16, work)?;
    prepare_bytes(bytes, Edit::Plane(normal, offset_mm), work)
}
pub fn prepare_snapshot(
    mesh: &scancraft_geometry::mesh::Mesh,
    normal: [f64; 3],
    offset_mm: f64,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    mesh.validate(work)?;
    let bytes =
        serde_json::to_vec_pretty(&json!({"schema_version":1,"units":"millimeters","mesh":mesh}))?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("mesh snapshot exceeds byte budget".into());
    }
    prepare_bytes(bytes, Edit::Plane(normal, offset_mm), work)
}
enum Edit {
    Plane([f64; 3], f64),
    Cleanup(CleanupOptions),
    Components { ids: Vec<usize>, keep: bool },
    Fill { id: usize, planarity_mm: f64 },
    Orient(Vec<usize>),
    SplitFans(Vec<u32>),
    Smooth(scancraft_geometry::mesh_smooth::Config),
    Simplify(scancraft_geometry::mesh_simplify::Config),
}
pub fn prepare_smooth(
    input: &Path,
    config: scancraft_geometry::mesh_smooth::Config,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    prepare_bytes(
        bounded(input, 128 * 1024 * 1024 + 16, work)?,
        Edit::Smooth(config),
        work,
    )
}
pub fn prepare_smooth_snapshot(
    mesh: &scancraft_geometry::mesh::Mesh,
    config: scancraft_geometry::mesh_smooth::Config,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    mesh.validate(work)?;
    let bytes =
        serde_json::to_vec_pretty(&json!({"schema_version":1,"units":"millimeters","mesh":mesh}))?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("mesh snapshot exceeds byte budget".into());
    }
    prepare_bytes(bytes, Edit::Smooth(config), work)
}
pub fn prepare_simplify(
    input: &Path,
    config: scancraft_geometry::mesh_simplify::Config,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    prepare_bytes(
        bounded(input, 128 * 1024 * 1024 + 16, work)?,
        Edit::Simplify(config),
        work,
    )
}
pub fn prepare_simplify_snapshot(
    mesh: &scancraft_geometry::mesh::Mesh,
    config: scancraft_geometry::mesh_simplify::Config,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    mesh.validate(work)?;
    let bytes =
        serde_json::to_vec_pretty(&json!({"schema_version":1,"units":"millimeters","mesh":mesh}))?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("mesh snapshot exceeds byte budget".into());
    }
    prepare_bytes(bytes, Edit::Simplify(config), work)
}
pub fn prepare_split_fans(
    input: &Path,
    ids: &[u32],
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    prepare_bytes(
        bounded(input, 128 * 1024 * 1024 + 16, work)?,
        Edit::SplitFans(ids.to_vec()),
        work,
    )
}
pub fn prepare_split_fans_snapshot(
    mesh: &scancraft_geometry::mesh::Mesh,
    ids: &[u32],
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    mesh.validate(work)?;
    let bytes =
        serde_json::to_vec_pretty(&json!({"schema_version":1,"units":"millimeters","mesh":mesh}))?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("mesh snapshot exceeds byte budget".into());
    }
    prepare_bytes(bytes, Edit::SplitFans(ids.to_vec()), work)
}
pub fn prepare_orientation(
    input: &Path,
    reverse: &[usize],
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    prepare_bytes(
        bounded(input, 128 * 1024 * 1024 + 16, work)?,
        Edit::Orient(reverse.to_vec()),
        work,
    )
}
pub fn prepare_orientation_snapshot(
    mesh: &scancraft_geometry::mesh::Mesh,
    reverse: &[usize],
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    mesh.validate(work)?;
    let bytes =
        serde_json::to_vec_pretty(&json!({"schema_version":1,"units":"millimeters","mesh":mesh}))?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("mesh snapshot exceeds byte budget".into());
    }
    prepare_bytes(bytes, Edit::Orient(reverse.to_vec()), work)
}
pub fn prepare_fill(
    input: &Path,
    id: usize,
    planarity_mm: f64,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    prepare_bytes(
        bounded(input, 128 * 1024 * 1024 + 16, work)?,
        Edit::Fill { id, planarity_mm },
        work,
    )
}
pub fn prepare_fill_snapshot(
    mesh: &scancraft_geometry::mesh::Mesh,
    id: usize,
    planarity_mm: f64,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    mesh.validate(work)?;
    let bytes =
        serde_json::to_vec_pretty(&json!({"schema_version":1,"units":"millimeters","mesh":mesh}))?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("mesh snapshot exceeds byte budget".into());
    }
    prepare_bytes(bytes, Edit::Fill { id, planarity_mm }, work)
}
pub fn prepare_cleanup(
    input: &Path,
    options: CleanupOptions,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    prepare_bytes(
        bounded(input, 128 * 1024 * 1024 + 16, work)?,
        Edit::Cleanup(options),
        work,
    )
}
pub fn prepare_cleanup_snapshot(
    mesh: &scancraft_geometry::mesh::Mesh,
    options: CleanupOptions,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    mesh.validate(work)?;
    let bytes =
        serde_json::to_vec_pretty(&json!({"schema_version":1,"units":"millimeters","mesh":mesh}))?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("mesh snapshot exceeds byte budget".into());
    }
    prepare_bytes(bytes, Edit::Cleanup(options), work)
}
pub fn prepare_components(
    input: &Path,
    ids: &[usize],
    keep: bool,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    prepare_bytes(
        bounded(input, 128 * 1024 * 1024 + 16, work)?,
        Edit::Components {
            ids: ids.to_vec(),
            keep,
        },
        work,
    )
}
pub fn prepare_components_snapshot(
    mesh: &scancraft_geometry::mesh::Mesh,
    ids: &[usize],
    keep: bool,
    work: &WorkControl,
) -> Result<PreparedTrim, Error> {
    mesh.validate(work)?;
    let bytes =
        serde_json::to_vec_pretty(&json!({"schema_version":1,"units":"millimeters","mesh":mesh}))?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("mesh snapshot exceeds byte budget".into());
    }
    prepare_bytes(
        bytes,
        Edit::Components {
            ids: ids.to_vec(),
            keep,
        },
        work,
    )
}
fn prepare_bytes(bytes: Vec<u8>, edit: Edit, work: &WorkControl) -> Result<PreparedTrim, Error> {
    let original_name = if bytes.starts_with(scancraft_geometry::mesh_project::MAGIC) {
        "original.meshproj"
    } else {
        "original.mesh.json"
    };
    let mut editor = if bytes.starts_with(scancraft_geometry::mesh_project::MAGIC) {
        scancraft_geometry::mesh_project::load(bytes.as_slice(), work)?
    } else {
        MeshEditor::new(mesh_document::decode(&bytes, work)?.mesh, 32, work)?
    };
    let original_topology = editor.mesh().topology(work)?;
    let (mesh, details) = match edit {
        Edit::Smooth(config) => {
            let (mesh, report) =
                scancraft_geometry::mesh_smooth::smooth(editor.mesh(), &config, work)?;
            (
                mesh,
                json!({"operation":"smooth","configuration":config,"smoothing":report,"plane":null,"cut_edges":0,"rejected_slivers":0,"removed_faces":0,"removed_vertices":0}),
            )
        }
        Edit::Simplify(config) => {
            let p = scancraft_geometry::mesh_simplify::simplify(editor.mesh(), &config, work)?;
            let removed_faces = editor.mesh().triangles.len() - p.mesh.triangles.len();
            let removed_vertices = editor.mesh().vertices.len() - p.mesh.vertices.len();
            (
                p.mesh,
                json!({"operation":"simplify","configuration":config,"decimation":p.report,"source_to_output_vertices":p.source_to_output,"plane":null,"cut_edges":0,"rejected_slivers":0,"removed_faces":removed_faces,"removed_vertices":removed_vertices}),
            )
        }

        Edit::SplitFans(ids) => {
            let p = scancraft_geometry::mesh_split_fans::split(editor.mesh(), &ids, work)?;
            (
                p.mesh,
                json!({"operation":"split_vertex_fans","selected_vertex_ids":ids,"vertices_added":p.vertices_added,"splits":p.splits,"face_coordinates_unchanged":true,"geometric_gap_created":false,"plane":null,"cut_edges":0,"rejected_slivers":0,"removed_faces":0,"removed_vertices":0}),
            )
        }
        Edit::Orient(reverse) => {
            let p = scancraft_geometry::mesh_orientation::orient(editor.mesh(), &reverse, work)?;
            (
                p.mesh,
                json!({"operation":"orient_faces","reverse_component_ids":reverse,"seed_policy":"first triangle per indexed edge component; explicit reversals applied afterward","outward_direction_verified":false,"faces_flipped":p.faces_flipped,"components":p.components,"vertices_moved":false,"plane":null,"cut_edges":0,"rejected_slivers":0,"removed_faces":0,"removed_vertices":0}),
            )
        }
        Edit::Fill { id, planarity_mm } => {
            let p = scancraft_geometry::mesh_fill::fill(editor.mesh(), id, planarity_mm, work)?;
            (
                p.mesh,
                json!({"operation":"boundary_fill","boundary_id":id,"maximum_planarity_error_mm":p.maximum_planarity_error_mm,"planarity_tolerance_mm":planarity_mm,"added_faces":p.added_triangles,"vertices_moved":false,"plane":null,"cut_edges":0,"rejected_slivers":0,"removed_faces":0,"removed_vertices":0}),
            )
        }
        Edit::Plane(normal, offset_mm) => {
            let p = trim_plane(editor.mesh(), normal, offset_mm, work)?;
            (
                p.mesh,
                json!({"operation":"plane_trim","plane":{"normal":normal,"offset_mm":offset_mm,"keep":"normal dot position + offset <= 0"},"cut_edges":p.cut_edges,"rejected_slivers":p.rejected_slivers,"removed_faces":0,"removed_vertices":0}),
            )
        }
        Edit::Components { ids, keep } => {
            let p = scancraft_geometry::mesh_components::select(editor.mesh(), &ids, keep, work)?;
            let removed_vertices = editor.mesh().vertices.len() - p.mesh.vertices.len();
            (
                p.mesh,
                json!({"operation":"component_selection","selected_ids":ids,"keep_selected":keep,"component_count":p.component_count,"selected_components":p.selected,"plane":null,"cut_edges":0,"rejected_slivers":0,"removed_faces":p.removed_triangles,"removed_vertices":removed_vertices}),
            )
        }
        Edit::Cleanup(options) => {
            let p = cleanup(editor.mesh(), options, work)?;
            (
                p.mesh,
                json!({"operation":"cleanup","duplicate_faces":options.duplicate_faces,"unused_vertices":options.unused_vertices,"plane":null,"cut_edges":0,"rejected_slivers":0,"removed_faces":p.removed_faces,"removed_vertices":p.removed_vertices}),
            )
        }
    };
    let commit = editor.replace(mesh, work)?;
    let topology = editor.mesh().topology(work)?;
    let report = json!({"schema_version":1,"input":digest(original_name,&bytes),
        "units":"millimeters","operation":details["operation"],"parameters":details,"plane":details["plane"],
        "original_topology":original_topology,"result_topology":topology,
        "cut_edges":details["cut_edges"],"rejected_slivers":details["rejected_slivers"],"removed_faces":details["removed_faces"],"removed_vertices":details["removed_vertices"],
        "vertices":editor.mesh().vertices.len(),"triangles":editor.mesh().triangles.len(),
        "cut_is_capped":if details["operation"]=="boundary_fill" {Value::Null}else{json!(false)},"original_source_preserved":true,"evicted_in_memory_undo_steps":commit.evicted_undo_steps,
        "physical_accuracy_qualified":false,"watertightness_qualified":false});
    work.check()?;
    Ok(PreparedTrim {
        bytes,
        editor,
        report,
        original_name,
    })
}
pub fn publish_prepared(
    prepared: &PreparedTrim,
    target: &Path,
    work: &WorkControl,
) -> Result<Value, Error> {
    if fs::symlink_metadata(target).is_ok() {
        return Err("trim output already exists".into());
    }
    work.check()?;
    let bytes = &prepared.bytes;
    let editor = &prepared.editor;
    let report = &prepared.report;
    let partial = staging(target)?;
    let result = (|| -> Result<Value, Error> {
        let mut artifacts = vec![artifact(&partial, "report.json", work, |out| {
            serde_json::to_writer_pretty(out, &report)?;
            Ok(())
        })?];
        artifacts.push(artifact(&partial, prepared.original_name, work, |out| {
            out.write_all(bytes)?;
            Ok(())
        })?);
        artifacts.push(artifact(&partial, "mesh.json", work, |out| {
            serde_json::to_writer_pretty(
                out,
                &json!({"schema_version":1,"units":"millimeters","mesh":editor.mesh()}),
            )?;
            Ok(())
        })?);
        artifacts.push(artifact(&partial, "project.meshproj", work, |out| {
            scancraft_geometry::mesh_project::save(editor, out, work)?;
            Ok(())
        })?);
        if !editor.mesh().triangles.is_empty() {
            artifacts.push(artifact(&partial, "mesh.stl", work, |out| {
                editor.mesh().write_stl(out, work)?;
                Ok(())
            })?);
            artifacts.push(artifact(&partial, "mesh.obj", work, |out| {
                editor.mesh().write_obj(out, work)?;
                Ok(())
            })?);
        }
        let summary = json!({"schema_version":1,"vertices":editor.mesh().vertices.len(),"triangles":editor.mesh().triangles.len(),"cut_edges":report["cut_edges"],"operation":report["operation"],"removed_faces":report["removed_faces"],"removed_vertices":report["removed_vertices"],"original_source_preserved":true,"physical_accuracy_qualified":false,"artifacts":artifacts});
        artifact(&partial, "manifest.json", work, |out| {
            serde_json::to_writer_pretty(out, &summary)?;
            Ok(())
        })?;
        File::open(&partial)?.sync_all()?;
        work.check()?;
        publish(&partial, target)?;
        Ok(summary)
    })();
    let summary = result.map_err(|e| {
        format!(
            "{e}; incomplete mesh trim retained at {}",
            partial.display()
        )
    })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "mesh trim published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(summary)
}
