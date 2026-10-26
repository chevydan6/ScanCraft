//! One owned document per bounded background job. Failed/cancelled proposals never commit.
use scancraft_geometry::{
    Cloud, Editor,
    filtering::{radius_filter_with, remove_duplicates_with},
    formats::{load_project, read_ply, save_project},
    work::{Stage, WorkControl},
    write_ply,
};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread::{self, JoinHandle},
};
#[derive(Default)]
pub struct Document {
    pub editor: Option<Editor>,
    pub selection: Vec<usize>,
}
#[derive(Clone)]
pub struct DisplayPoint {
    pub normal: Option<[f64; 3]>,
    pub position: [f64; 3],
    pub color: [u8; 3],
    pub selected: bool,
    pub index: usize,
}
#[derive(Clone, Default)]
pub struct Preview {
    pub normals: usize,
    pub points: Vec<DisplayPoint>,
    pub bounds: Option<([f64; 3], [f64; 3])>,
    pub total: usize,
    pub selected: usize,
}
#[derive(Clone)]
pub enum Operation {
    StatisticalPreview {
        neighbors: usize,
        sigma: f64,
    },
    VerifyRecordings {
        directory: PathBuf,
    },
    ExportProjectBundle {
        sources: PathBuf,
        target: PathBuf,
    },
    VerifySources {
        directory: PathBuf,
    },
    DefineFrame {
        origin: usize,
        axes: Option<(usize, usize)>,
    },
    LoadFrame(PathBuf),
    SaveFrame {
        label: String,
        frame: scancraft_geometry::coordinate_frame::Frame,
        target: PathBuf,
    },
    ApplyFrame {
        label: String,
        frame: scancraft_geometry::coordinate_frame::Frame,
    },
    AlignFrame {
        origin: usize,
        axes: Option<(usize, usize)>,
    },
    RenameMeasurement {
        index: usize,
        label: String,
    },
    RemoveMeasurement {
        index: usize,
    },
    UndoMeasurement,
    RedoMeasurement,
    RecordSphere {
        label: String,
    },
    RecordCylinder {
        label: String,
        maximum_rms_mm: f64,
    },
    RecordDistance {
        label: String,
    },
    RecordAngle {
        label: String,
        vertex: usize,
    },
    RecordCircle {
        label: String,
        maximum_plane_rms_mm: f64,
    },
    FitCircle {
        maximum_plane_rms_mm: f64,
    },
    FitSphere,
    FitCylinder {
        maximum_rms_mm: f64,
    },
    PointPlane {
        probe: usize,
        maximum_plane_rms_mm: f64,
        label: Option<String>,
    },
    MeasureDistance,
    MeasureAngle {
        vertex: usize,
    },
    FlipNormals,
    FuseSequence {
        settings: PathBuf,
        output: PathBuf,
    },
    ReconstructRecordings {
        inputs: scancraft_workflow::recording_reconstruction::Inputs,
        output: PathBuf,
    },
    Reconstruct {
        inputs: scancraft_workflow::FileInputs,
        output: PathBuf,
    },
    Load {
        path: PathBuf,
        unit_scale: Option<f64>,
    },
    SelectBox {
        min: [f64; 3],
        max: [f64; 3],
    },
    Invert,
    ClearSelection,
    SelectScreen {
        projection: scancraft_geometry::selection::OrthographicProjection,
        shape: scancraft_geometry::selection::ScreenShape,
        visibility: scancraft_geometry::selection::Visibility,
        mode: scancraft_geometry::selection::SelectionMode,
    },
    Pick {
        index: usize,
        additive: bool,
    },
    Trim {
        keep: bool,
    },
    Downsample(f64),
    Radius {
        radius: f64,
        neighbors: usize,
    },
    Duplicates,
    FitPlane,
    Undo,
    Redo,
    Save {
        path: PathBuf,
        project: bool,
    },
}
pub struct Report {
    pub coordinate_frame: Option<(String, scancraft_geometry::coordinate_frame::Frame)>,
    pub message: String,
    pub preview: Option<Preview>,
    pub reset_view: bool,
    pub edited: bool,
    pub project_saved: bool,
    pub opened_path: Option<PathBuf>,
}
pub struct Output {
    pub document: Document,
    pub result: Result<Report, String>,
}
pub struct Job {
    pub control: Arc<WorkControl>,
    rx: Receiver<Result<Report, String>>,
    handle: Option<JoinHandle<()>>,
    document: Option<Arc<Mutex<Document>>>,
}
impl Job {
    pub fn start(document: Document, operation: Operation) -> Self {
        let control = Arc::new(WorkControl::default());
        let c = control.clone();
        let document = Arc::new(Mutex::new(document));
        let worker_document = document.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let worker_tx = tx.clone();
        let handle = match thread::Builder::new()
            .name("scancraft-geometry".into())
            .spawn(move || {
                let mut document = worker_document.lock().unwrap_or_else(|p| p.into_inner());
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    execute(&mut document, operation, &c)
                }))
                .unwrap_or_else(|_| {
                    Err("Geometry worker panicked; original document ownership retained".into())
                });
                let _ = worker_tx.send(result);
            }) {
            Ok(handle) => Some(handle),
            Err(e) => {
                let _ = tx.send(Err(format!("Could not start geometry worker: {e}")));
                None
            }
        };
        Self {
            control,
            rx,
            handle,
            document: Some(document),
        }
    }
    pub fn poll(&mut self) -> Option<Output> {
        self.document.as_ref()?;
        let mut result = match self.rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                Err("Geometry worker stopped unexpectedly; document ownership retained".into())
            }
        };
        if let Some(handle) = self.handle.take()
            && handle.join().is_err()
        {
            result = Err("Geometry worker panicked; document ownership retained".into());
        }
        let shared = self.document.take()?;
        let mutex = Arc::try_unwrap(shared)
            .ok()
            .expect("joined worker released the document");
        let document = mutex.into_inner().unwrap_or_else(|p| p.into_inner());
        Some(Output { document, result })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.control.cancel();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn build_preview(
    cloud: &Cloud,
    selection: &[usize],
    work: &WorkControl,
) -> Result<Preview, String> {
    cloud.validate_with(work)?;
    let mut selected = selection.to_vec();
    selected.sort_unstable();
    if selected.last().is_some_and(|i| *i >= cloud.points.len())
        || selected.windows(2).any(|p| p[0] == p[1])
    {
        return Err("invalid selection indices".into());
    }
    let mut result = Preview {
        normals: 0,
        points: Vec::with_capacity(cloud.points.len().min(50000)),
        bounds: None,
        total: cloud.points.len(),
        selected: selected.len(),
    };
    let step = cloud.points.len().div_ceil(50000).max(1);
    work.update(Stage::Preview, 0, cloud.points.len())?;
    for (i, p) in cloud.points.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Preview, i, cloud.points.len())?;
        }
        if let Some((min, max)) = &mut result.bounds {
            for a in 0..3 {
                min[a] = min[a].min(p.position[a]);
                max[a] = max[a].max(p.position[a]);
            }
        } else {
            result.bounds = Some((p.position, p.position));
        }
        let normal = p.normal.and_then(|n| {
            let length = n[0].hypot(n[1]).hypot(n[2]);
            (length.is_finite() && length > 0.).then(|| n.map(|v| v / length))
        });
        if normal.is_some() {
            result.normals += 1;
        }
        if i % step == 0 {
            result.points.push(DisplayPoint {
                normal,
                position: p.position,
                color: p.color.unwrap_or([101, 190, 226]),
                selected: selected.binary_search(&i).is_ok(),
                index: i,
            });
        }
    }
    work.check()?;
    Ok(result)
}
fn report(message: impl Into<String>, preview: Option<Preview>) -> Report {
    Report {
        coordinate_frame: None,
        message: message.into(),
        preview,
        reset_view: false,
        edited: false,
        project_saved: false,
        opened_path: None,
    }
}
fn editor(document: &Document) -> Result<&Editor, String> {
    document
        .editor
        .as_ref()
        .ok_or_else(|| "Import a point cloud first".into())
}
fn apply_coordinate_frame(
    document: &mut Document,
    label: String,
    frame: scancraft_geometry::coordinate_frame::Frame,
    work: &WorkControl,
) -> Result<Report, String> {
    let next = frame.apply(&editor(document)?.cloud, work)?;
    let preview = build_preview(&next, &document.selection, work)?;
    work.check()?;
    document.editor.as_mut().unwrap().replace(next)?;
    let mut result = report(
        format!(
            "Applied frame {label:?}: origin {:?} mm; axes {:?}. Positions and normals transformed; scale unchanged. Measurements keep historical coordinates. Undo is available.",
            frame.origin_mm, frame.axes
        ),
        Some(preview),
    );
    result.edited = true;
    result.reset_view = true;
    result.coordinate_frame = Some((label, frame));
    Ok(result)
}
fn execute(
    document: &mut Document,
    operation: Operation,
    work: &WorkControl,
) -> Result<Report, String> {
    if let Operation::ExportProjectBundle { sources, target } = &operation {
        let editor = document
            .editor
            .as_ref()
            .ok_or("no editable project is open")?;
        let summary = scancraft_workflow::project_bundle::export(editor, sources, target, work)
            .map_err(|e| e.to_string())?;
        return Ok(report(
            format!(
                "Exported project bundle with {} retained source artifacts to {}. Editable state replay-verified; active document and references unchanged. Selected raw frames only; physical accuracy remains unqualified.",
                summary["copied_source_artifacts"],
                target.display()
            ),
            None,
        ));
    }
    if let Operation::VerifyRecordings { directory } = &operation {
        let editor = document
            .editor
            .as_ref()
            .ok_or("no editable project is open")?;
        scancraft_workflow::scan_project_load::verify_recordings(editor, directory, work)
            .map_err(|e| e.to_string())?;
        return Ok(report(
            "Complete recording verification passed: saved fingerprints, full replay and initial selected-frame membership match. Project references unchanged. Device binding, synchronization and physical accuracy remain unqualified.",
            None,
        ));
    }
    if let Operation::VerifySources { directory } = &operation {
        let origin = document
            .editor
            .as_ref()
            .and_then(|e| e.origin())
            .ok_or("project has no recorded reconstruction provenance")?;
        let checked = scancraft_workflow::source_verification::verify(origin, directory, work)
            .map_err(|e| e.to_string())?;
        let count = checked
            .artifacts
            .iter()
            .filter(|r| r.status == "match")
            .count();
        let issues: Vec<_> = checked
            .artifacts
            .iter()
            .filter(|r| r.status != "match")
            .map(|r| format!("{}: {}", r.name, r.status))
            .collect();
        return Ok(report(
            format!(
                "Source verification {}: {count}/{} artifacts match CRC32 and length. {}{} Device binding, synchronization and physical accuracy remain unqualified. Project references unchanged.",
                if checked.passed { "passed" } else { "failed" },
                checked.artifacts.len(),
                issues.join("; "),
                if checked.directory_unchanged {
                    ""
                } else {
                    " Bundle directory changed during verification."
                }
            ),
            None,
        ));
    }

    work.check()?;
    if let Operation::LoadFrame(path) = operation {
        let loaded =
            scancraft_workflow::frame_document::load(&path, work).map_err(|e| e.to_string())?;
        let mut r = report(
            format!(
                "Loaded frame {:?}; cloud unchanged. Apply explicitly to map the current world-mm cloud into this local frame.",
                loaded.label
            ),
            None,
        );
        r.coordinate_frame = Some((loaded.label, loaded.frame));
        return Ok(r);
    }
    if let Operation::SaveFrame {
        label,
        frame,
        target,
    } = operation
    {
        let d = scancraft_workflow::frame_document::Document::new(label, frame)
            .map_err(|e| e.to_string())?;
        scancraft_workflow::frame_document::save(&d, &target, work).map_err(|e| e.to_string())?;
        return Ok(report(
            format!(
                "Reusable frame saved at {}. Cloud and project state unchanged.",
                target.join("frame.rapframe").display()
            ),
            None,
        ));
    }
    if let Operation::ApplyFrame { label, frame } = operation {
        return apply_coordinate_frame(document, label, frame, work);
    }
    if let Operation::FuseSequence { settings, output } = operation {
        if std::fs::symlink_metadata(&output).is_ok() {
            return Err("Result folder already exists; choose a new folder".into());
        }
        let prepared =
            scancraft_workflow::sequence::prepare(&settings, work).map_err(|e| e.to_string())?;
        let editor = Editor::new(prepared.cloud().clone(), 32)?;
        let preview = build_preview(&editor.cloud, &[], work)?;
        let summary = scancraft_workflow::sequence::publish_prepared(&prepared, &output, work)
            .map_err(|e| e.to_string())?;
        document.editor = Some(editor);
        document.selection.clear();
        let mut result = report(
            format!(
                "Fused {} points from {} integrated frames; {} tracking rejections. Saved in {}. Physical accuracy remains unqualified.",
                summary["points"],
                summary["integrated_frames"],
                summary["rejected_frames"],
                output.display()
            ),
            Some(preview),
        );
        result.reset_view = true;
        result.project_saved = true;
        result.opened_path = Some(output.join("project.orproj"));
        return Ok(result);
    }
    if matches!(
        &operation,
        Operation::Reconstruct { .. } | Operation::ReconstructRecordings { .. }
    ) {
        let output = match &operation {
            Operation::Reconstruct { output, .. }
            | Operation::ReconstructRecordings { output, .. } => output.clone(),
            _ => unreachable!(),
        };
        if std::fs::symlink_metadata(&output).is_ok() {
            return Err("Result folder already exists; choose a new folder".into());
        }
        let prepared = match &operation {
            Operation::Reconstruct { inputs, .. } => scancraft_workflow::prepare(inputs, work),
            Operation::ReconstructRecordings { inputs, .. } => {
                scancraft_workflow::recording_reconstruction::prepare(inputs, work)
            }
            _ => unreachable!(),
        }
        .map_err(|e| e.to_string())?;
        // Build all fallible in-memory state before durable publication. A late
        // cancellation after publication is a completed operation, not rollback.
        let proposal = if prepared.cloud().is_some_and(|c| !c.points.is_empty()) {
            let editor = prepared.create_editor(&output).map_err(|e| e.to_string())?;
            let preview = build_preview(&editor.cloud, &[], work)?;
            Some((editor, preview))
        } else {
            None
        };
        let summary = scancraft_workflow::publish_prepared(&prepared, &output, work)
            .map_err(|e| e.to_string())?;
        if let Some((editor, preview)) = proposal {
            document.editor = Some(editor);
            document.selection.clear();
            let mut r = report(
                format!(
                    "Reconstructed {} points{}; rejected correspondences: {}, invalid mappings: {}, rejected 3D points: {}. Saved in {}. Exposure timing and physical accuracy remain unqualified.",
                    summary.points,
                    if summary.mesh_triangles > 0 {
                        format!(", exported {} mesh triangles", summary.mesh_triangles)
                    } else {
                        String::new()
                    },
                    summary.rejected_seeds,
                    summary.mapping_failures,
                    summary.rejected_geometry,
                    output.display()
                ),
                Some(preview),
            );
            r.reset_view = true;
            r.project_saved = true;
            r.opened_path = Some(output.join("project.orproj"));
            return Ok(r);
        }
        return Ok(report(
            format!(
                "No acceptable geometry. Diagnostics saved in {}; current document retained.",
                output.display()
            ),
            None,
        ));
    }
    if let Operation::Load { path, unit_scale } = operation {
        if path.is_dir() {
            let loaded = scancraft_workflow::scan_project_load::load(&path, work)
                .map_err(|e| e.to_string())?;
            let preview = build_preview(&loaded.editor.cloud, &[], work)?;
            work.check()?;
            document.editor = Some(loaded.editor);
            document.selection.clear();
            let mut result = report(
                "Opened verified scan folder. All packaged artifact fingerprints and complete recordings checked; selected-frame association passed. Source directory relocated in memory. Physical accuracy and synchronization remain unqualified.",
                Some(preview),
            );
            result.reset_view = true;
            result.opened_path = Some(path.join("project/project.orproj"));
            return Ok(result);
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() > 2 * 1024 * 1024 * 1024 {
            return Err("Import requires a regular file no larger than 2 GiB".into());
        }
        let mut input = ProgressReader {
            input: file,
            work,
            done: 0,
            total: metadata.len() as usize,
        };
        // CLI and workbench historically used different project extensions.
        // Detect the versioned content, preserving the prefix for either parser.
        let mut prefix = [0u8; 8];
        input.read_exact(&mut prefix).map_err(|e| e.to_string())?;
        let project = prefix.starts_with(b"ORPROJ");
        let input = std::io::Cursor::new(prefix).chain(BufReader::new(input));
        let loaded = if project {
            load_project(input).map_err(|e| e.to_string())?
        } else {
            Editor::new(read_ply(input, unit_scale).map_err(|e| e.to_string())?, 8)?
        };
        let preview = build_preview(&loaded.cloud, &[], work)?;
        work.check()?;
        document.editor = Some(loaded);
        document.selection.clear();
        let mut r = report(
            "Imported. All positions use millimeters; file units override the fallback choice.",
            Some(preview),
        );
        r.reset_view = true;
        r.opened_path = Some(path);
        return Ok(r);
    }
    let current = editor(document)?;
    if matches!(
        operation,
        Operation::RenameMeasurement { .. }
            | Operation::RemoveMeasurement { .. }
            | Operation::UndoMeasurement
            | Operation::RedoMeasurement
    ) {
        work.check()?;
        let editor = document.editor.as_mut().unwrap();
        match operation {
            Operation::RenameMeasurement { index, label } => {
                editor.rename_measurement(index, label)?
            }
            Operation::RemoveMeasurement { index } => editor.remove_measurement(index)?,
            Operation::UndoMeasurement => {
                if !editor.undo_measurement()? {
                    return Err("no measurement change to undo".into());
                }
            }
            Operation::RedoMeasurement => {
                if !editor.redo_measurement()? {
                    return Err("no measurement change to redo".into());
                }
            }
            _ => unreachable!(),
        }
        let mut result = report(
            "Measurement list updated. Snapshot undo/redo is separate from geometry history.",
            None,
        );
        result.edited = true;
        return Ok(result);
    }
    if let Operation::PointPlane {
        probe,
        maximum_plane_rms_mm,
        label,
    } = operation
    {
        if !(4..=100_001).contains(&document.selection.len()) {
            return Err("select one probe and 3..100000 plane points".into());
        }
        let mut indices = document.selection.clone();
        indices.sort_unstable();
        indices.dedup();
        if indices.len() != document.selection.len() {
            return Err("point-plane selection must have distinct source indices".into());
        }
        let local = indices
            .iter()
            .position(|&i| i == probe)
            .ok_or("probe source index must be selected")?;
        let mut points = Vec::with_capacity(indices.len());
        for (i, &index) in indices.iter().enumerate() {
            if i % 1024 == 0 {
                work.check()?;
            }
            points.push(
                current
                    .cloud
                    .points
                    .get(index)
                    .ok_or("invalid point-plane selection")?
                    .position,
            );
        }
        let snapshot = scancraft_geometry::measurement_snapshot::Snapshot::point_plane(
            label.clone().unwrap_or_else(|| "Point-plane result".into()),
            points,
            local,
            maximum_plane_rms_mm,
            work,
        )?;
        let scancraft_geometry::measurement_snapshot::ResultValue::PointPlane { fit, .. } =
            &snapshot.result
        else {
            unreachable!()
        };
        let message = format!(
            "Point-plane distance {:.6} mm; signed {:.6} mm; foot {:?} mm; normal {:?}; reference plane RMS {:.6} mm; {} plane points; probe source index {probe}. Sign follows positive dominant normal component; physical accuracy unqualified.",
            fit.distance_mm,
            fit.signed_distance_mm,
            fit.foot_mm,
            fit.normal,
            fit.plane_rms_mm,
            fit.plane_points
        );
        work.check()?;
        let mut r = report(message, None);
        if label.is_some() {
            document
                .editor
                .as_mut()
                .unwrap()
                .add_measurement(snapshot)?;
            r.edited = true;
        }
        return Ok(r);
    }
    if matches!(
        operation,
        Operation::RecordSphere { .. }
            | Operation::RecordCylinder { .. }
            | Operation::RecordCircle { .. }
            | Operation::RecordDistance { .. }
            | Operation::RecordAngle { .. }
    ) {
        if matches!(operation, Operation::RecordCylinder { .. })
            && !(6..=20_000).contains(&document.selection.len())
        {
            return Err("cylinder snapshot requires 6..20000 selected points".into());
        }
        let mut points = Vec::with_capacity(document.selection.len());
        for (i, index) in document.selection.iter().enumerate() {
            if i % 1024 == 0 {
                work.check()?;
            }
            points.push(
                current
                    .cloud
                    .points
                    .get(*index)
                    .ok_or("measurement selection out of bounds")?
                    .position,
            );
        }
        let snapshot = match operation {
            Operation::RecordCylinder {
                label,
                maximum_rms_mm,
            } => scancraft_geometry::measurement_snapshot::Snapshot::cylinder(
                label,
                points,
                maximum_rms_mm,
                work,
            )?,
            Operation::RecordDistance { label } => {
                if document.selection.len() != 2 || document.selection[0] == document.selection[1] {
                    return Err("distance requires two distinct source indices".into());
                }
                scancraft_geometry::measurement_snapshot::Snapshot::distance(label, points, work)?
            }
            Operation::RecordAngle { label, vertex } => {
                let mut unique = document.selection.clone();
                unique.sort_unstable();
                unique.dedup();
                if unique.len() != 3 || document.selection.len() != 3 {
                    return Err("angle requires three distinct source indices".into());
                }
                let local = document
                    .selection
                    .iter()
                    .position(|&i| i == vertex)
                    .ok_or("angle vertex must be selected")?;
                scancraft_geometry::measurement_snapshot::Snapshot::angle(
                    label, points, local, work,
                )?
            }
            Operation::RecordSphere { label } => {
                scancraft_geometry::measurement_snapshot::Snapshot::sphere(label, points, work)?
            }
            Operation::RecordCircle {
                label,
                maximum_plane_rms_mm,
            } => scancraft_geometry::measurement_snapshot::Snapshot::circle(
                label,
                points,
                maximum_plane_rms_mm,
                work,
            )?,
            _ => unreachable!(),
        };
        work.check()?;
        document
            .editor
            .as_mut()
            .unwrap()
            .add_measurement(snapshot)?;
        let mut result = report(
            "Measurement snapshot added. Save the project to persist it.",
            None,
        );
        result.edited = true;
        return Ok(result);
    }
    let define_only = matches!(operation, Operation::DefineFrame { .. });
    if let Operation::AlignFrame { origin, axes } | Operation::DefineFrame { origin, axes } =
        operation
    {
        let get = |index: usize| -> Result<[f64; 3], String> {
            if !document.selection.contains(&index) {
                return Err("frame reference source indices must be selected".into());
            }
            Ok(current
                .cloud
                .points
                .get(index)
                .ok_or("invalid frame reference index")?
                .position)
        };
        let origin_position = get(origin)?;
        let frame = if let Some((x, xy)) = axes {
            if origin == x || origin == xy || x == xy {
                return Err("frame requires three distinct source indices".into());
            }
            scancraft_geometry::coordinate_frame::Frame::from_points(
                origin_position,
                get(x)?,
                get(xy)?,
            )?
        } else {
            scancraft_geometry::coordinate_frame::Frame::at_origin(origin_position)?
        };
        if define_only {
            work.check()?;
            let mut r = report(
                "Coordinate frame defined from selected references; cloud unchanged. Save or apply it explicitly.",
                None,
            );
            r.coordinate_frame = Some(("Manual frame".into(), frame));
            return Ok(r);
        }
        return apply_coordinate_frame(document, "Manual frame".into(), frame, work);
    }
    if matches!(operation, Operation::FlipNormals) {
        let next = scancraft_geometry::normal_edit::flip_selected(
            &current.cloud,
            &document.selection,
            work,
        )?;
        let preview = build_preview(&next, &document.selection, work)?;
        work.check()?;
        document.editor.as_mut().unwrap().replace(next)?;
        let mut result = report(
            "Selected normal directions flipped. Undo is available.",
            Some(preview),
        );
        result.edited = true;
        return Ok(result);
    }
    if let Operation::StatisticalPreview { neighbors, sigma } = operation {
        let statistics = scancraft_geometry::statistical_filter::analyze(
            &current.cloud,
            neighbors,
            sigma,
            work,
        )?;
        let preview = build_preview(&current.cloud, &statistics.outlier_indices, work)?;
        work.check()?;
        let message = format!(
            "Selected {} statistical candidates (k={}, threshold {:.6} mm, mean {:.6} mm, sample deviation {:.6} mm). Geometry unchanged. Review sparse legitimate details before Delete/Keep selected.",
            statistics.outlier_indices.len(),
            neighbors,
            statistics.threshold_mm,
            statistics.global_mean_mm,
            statistics.sample_standard_deviation_mm
        );
        document.selection = statistics.outlier_indices;
        return Ok(report(message, Some(preview)));
    }
    let proposal = match operation {
        Operation::Trim { keep } => Some(current.cloud.trimmed_with(
            &document.selection,
            keep,
            work,
        )?),
        Operation::Downsample(size) => Some(current.cloud.voxel_downsample_with(size, work)?),
        Operation::Radius { radius, neighbors } => {
            Some(radius_filter_with(&current.cloud, radius, neighbors, work)?)
        }
        Operation::Duplicates => Some(remove_duplicates_with(&current.cloud, work)?),
        _ => None,
    };
    if let Some(next) = proposal {
        let preview = build_preview(&next, &[], work)?;
        work.check()?;
        document.editor.as_mut().unwrap().replace(next)?;
        document.selection.clear();
        let mut r = report(
            "Edit applied. Undo is available; voxel downsampling clears normals.",
            Some(preview),
        );
        r.edited = true;
        return Ok(r);
    }
    match operation {
        Operation::ClearSelection => {
            let preview = build_preview(&current.cloud, &[], work)?;
            work.check()?;
            document.selection.clear();
            Ok(report("Selection cleared", Some(preview)))
        }
        Operation::SelectScreen {
            projection,
            shape,
            visibility,
            mode,
        } => {
            let incoming = scancraft_geometry::selection::select_screen(
                &current.cloud,
                &projection,
                &shape,
                visibility,
                0.5,
                work,
            )?;
            let selection = scancraft_geometry::selection::combine_selection(
                current.cloud.points.len(),
                &document.selection,
                &incoming,
                mode,
                work,
            )?;
            let preview = build_preview(&current.cloud, &selection, work)?;
            work.check()?;
            document.selection = selection;
            Ok(report(
                "Screen selection updated using the full cloud (0.5-pixel boundary tolerance)",
                Some(preview),
            ))
        }
        Operation::SelectBox { min, max } => {
            let selection = current.cloud.select_box_with(min, max, work)?;
            let preview = build_preview(&current.cloud, &selection, work)?;
            work.check()?;
            document.selection = selection;
            Ok(report("Box selection updated", Some(preview)))
        }
        Operation::Pick { index, additive } => {
            if index >= current.cloud.points.len() {
                return Err("picked point index out of range".into());
            }
            let mut selection = if additive {
                document.selection.clone()
            } else {
                Vec::new()
            };
            if additive && selection.contains(&index) {
                selection.retain(|i| *i != index);
            } else {
                selection.push(index);
            }
            selection.sort_unstable();
            let preview = build_preview(&current.cloud, &selection, work)?;
            work.check()?;
            document.selection = selection;
            Ok(report("Point selection updated", Some(preview)))
        }
        Operation::Invert => {
            let mut mask = vec![false; current.cloud.points.len()];
            for (i, index) in document.selection.iter().enumerate() {
                if i % 1024 == 0 {
                    work.check()?;
                }
                *mask.get_mut(*index).ok_or("invalid selection")? = true;
            }
            let mut selection = Vec::new();
            for (i, marked) in mask.into_iter().enumerate() {
                if i % 1024 == 0 {
                    work.update(Stage::Processing, i, current.cloud.points.len())?;
                }
                if !marked {
                    selection.push(i);
                }
            }
            let preview = build_preview(&current.cloud, &selection, work)?;
            work.check()?;
            document.selection = selection;
            Ok(report("Selection inverted", Some(preview)))
        }
        Operation::Undo | Operation::Redo => {
            let undo = matches!(operation, Operation::Undo);
            let target = if undo {
                current.undo_cloud()
            } else {
                current.redo_cloud()
            };
            let Some(target) = target else {
                return Ok(report(
                    if undo {
                        "Nothing to undo"
                    } else {
                        "Nothing to redo"
                    },
                    None,
                ));
            };
            let preview = build_preview(target, &[], work)?;
            work.check()?;
            let current = document.editor.as_mut().unwrap();
            if undo {
                current.undo();
            } else {
                current.redo();
            }
            document.selection.clear();
            let mut r = report(
                if undo {
                    "Undo complete"
                } else {
                    "Redo complete"
                },
                Some(preview),
            );
            r.edited = true;
            Ok(r)
        }
        Operation::MeasureDistance | Operation::MeasureAngle { .. } => {
            let expected = if matches!(operation, Operation::MeasureDistance) {
                2
            } else {
                3
            };
            if document.selection.len() != expected {
                return Err(format!(
                    "measurement requires exactly {expected} selected points"
                ));
            }
            let mut indices = document.selection.clone();
            indices.sort_unstable();
            indices.dedup();
            if indices.len() != expected {
                return Err("measurement requires distinct source indices".into());
            }
            let points: Vec<_> = indices
                .iter()
                .map(|&i| {
                    current
                        .cloud
                        .points
                        .get(i)
                        .map(|p| p.position)
                        .ok_or("invalid selection")
                })
                .collect::<Result<_, _>>()?;
            work.check()?;
            let message = match operation {
                Operation::MeasureDistance => {
                    let distance = scancraft_geometry::distance_mm(points[0], points[1])?;
                    format!(
                        "Point distance {distance:.6} mm ({:.6} in); source indices {:?} (declared units; physical accuracy unqualified)",
                        distance / 25.4,
                        indices
                    )
                }
                Operation::MeasureAngle { vertex } => {
                    let middle = indices
                        .iter()
                        .position(|&i| i == vertex)
                        .ok_or("angle vertex index must be one of the three selected points")?;
                    let arms: Vec<_> = (0..3).filter(|&i| i != middle).collect();
                    let angle = scancraft_geometry::measurement::angle_degrees(
                        points[arms[0]],
                        points[middle],
                        points[arms[1]],
                    )?;
                    format!(
                        "Angle {angle:.6} degrees at source index {vertex}; arm indices {} and {} (physical accuracy unqualified)",
                        indices[arms[0]], indices[arms[1]]
                    )
                }
                _ => unreachable!(),
            };
            work.check()?;
            Ok(report(message, None))
        }
        Operation::FitPlane
        | Operation::FitSphere
        | Operation::FitCircle { .. }
        | Operation::FitCylinder { .. } => {
            if matches!(operation, Operation::FitCylinder { .. })
                && !(6..=20_000).contains(&document.selection.len())
            {
                return Err("cylinder fit requires 6..20000 selected surface points".into());
            }
            let mut points = Vec::with_capacity(document.selection.len());
            for (i, index) in document.selection.iter().enumerate() {
                if i % 1024 == 0 {
                    work.update(Stage::Processing, i, document.selection.len())?;
                }
                points.push(
                    current
                        .cloud
                        .points
                        .get(*index)
                        .ok_or("invalid selection")?
                        .position,
                );
            }
            if let Operation::FitCylinder { maximum_rms_mm } = operation {
                let cylinder = scancraft_geometry::cylinder::fit(&points, maximum_rms_mm, work)?;
                return Ok(report(
                    format!(
                        "Cylinder axis point {:?} mm; unit direction {:?}; diameter {:.6} mm; geometric RMS {:.6} mm; maximum residual {:.6} mm; sampled axial range {:?} mm; Jacobian condition {:.3}; {} converged starts; {} iterations. Local fit; physical accuracy unqualified.",
                        cylinder.axis_point_mm,
                        cylinder.axis_direction,
                        cylinder.diameter_mm,
                        cylinder.geometric_rms_mm,
                        cylinder.maximum_absolute_residual_mm,
                        cylinder.axial_range_mm,
                        cylinder.jacobian_condition_number,
                        cylinder.starts_converged,
                        cylinder.iterations
                    ),
                    None,
                ));
            }
            if let Operation::FitCircle {
                maximum_plane_rms_mm,
            } = operation
            {
                let circle = scancraft_geometry::measurement::fit_circle(
                    &points,
                    maximum_plane_rms_mm,
                    work,
                )?;
                return Ok(report(
                    format!(
                        "Circle center {:?} mm; diameter {:.6} mm; plane RMS {:.6} mm; geometric RMS {:.6} mm (imported units)",
                        circle.center_mm,
                        circle.diameter_mm,
                        circle.plane_rms_mm,
                        circle.geometric_rms_mm
                    ),
                    None,
                ));
            }
            if matches!(operation, Operation::FitSphere) {
                let sphere = scancraft_geometry::measurement::fit_sphere(&points, work)?;
                return Ok(report(
                    format!(
                        "Sphere center {:?} mm; radius {:.6} mm; diameter {:.6} mm; geometric RMS {:.6} mm; maximum residual {:.6} mm (imported units)",
                        sphere.center_mm,
                        sphere.radius_mm,
                        2. * sphere.radius_mm,
                        sphere.geometric_rms_mm,
                        sphere.maximum_absolute_residual_mm
                    ),
                    None,
                ));
            }
            let plane = scancraft_geometry::registration::fit_plane(&points)?;
            work.check()?;
            Ok(report(
                format!(
                    "Plane normal {:?}; offset {:.6} mm; RMS {:.6} mm (imported units)",
                    plane.normal, plane.offset_mm, plane.rms_mm
                ),
                None,
            ))
        }
        Operation::Save { path, project } => {
            publish(current, &path, project, work)?;
            let mut r = report(format!("Saved {}", path.display()), None);
            r.project_saved = project;
            Ok(r)
        }
        _ => Err("unsupported geometry operation".into()),
    }
}
struct ProgressReader<'a, R> {
    input: R,
    work: &'a WorkControl,
    done: usize,
    total: usize,
}
impl<R: Read> Read for ProgressReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.work
            .update(Stage::Reading, self.done, self.total)
            .map_err(io::Error::other)?;
        let n = self.input.read(buf)?;
        self.done += n;
        Ok(n)
    }
}
struct ProgressWriter<'a, W> {
    output: W,
    work: &'a WorkControl,
    done: usize,
}
impl<W: Write> Write for ProgressWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.work
            .update(Stage::Writing, self.done, 0)
            .map_err(io::Error::other)?;
        let n = self.output.write(buf)?;
        self.done += n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.work.check().map_err(io::Error::other)?;
        self.output.flush()
    }
}
struct PartialFile {
    path: PathBuf,
}
impl Drop for PartialFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
fn publish(
    editor: &Editor,
    path: &std::path::Path,
    project: bool,
    work: &WorkControl,
) -> Result<(), String> {
    work.check()?;
    let parent = path.parent().ok_or("save path has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let partial = path.with_extension(format!(
        "{}.partial",
        path.extension().unwrap_or_default().to_string_lossy()
    ));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)
        .map_err(|e| e.to_string())?;
    let guard = PartialFile { path: partial };
    let mut writer = ProgressWriter {
        output: BufWriter::new(file),
        work,
        done: 0,
    };
    if project {
        save_project(editor, &mut writer).map_err(|e| e.to_string())?;
    } else {
        write_ply(&editor.cloud, &mut writer).map_err(|e| e.to_string())?;
    }
    writer.flush().map_err(|e| e.to_string())?;
    writer
        .output
        .get_ref()
        .sync_all()
        .map_err(|e| e.to_string())?;
    drop(writer);
    work.check()?;
    // Linearization point: cancellation after publication does not undo a completed save.
    std::fs::hard_link(&guard.path, path).map_err(|e| e.to_string())?;
    if let Err(e) = File::open(parent).and_then(|f| f.sync_all()) {
        eprintln!("saved project, but directory sync failed: {e}");
    }
    drop(guard);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_geometry::Point;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    fn document(n: usize) -> Document {
        Document {
            editor: Some(
                Editor::new(
                    Cloud {
                        points: (0..n)
                            .map(|i| Point {
                                position: [i as f64, (i % 7) as f64, 0.],
                                color: Some([1, 2, 3]),
                                normal: Some([0., 0., 1.]),
                            })
                            .collect(),
                    },
                    8,
                )
                .unwrap(),
            ),
            selection: vec![0],
        }
    }
    fn snapshot(d: &Document) -> Vec<u8> {
        let mut b = Vec::new();
        save_project(d.editor.as_ref().unwrap(), &mut b).unwrap();
        b
    }
    #[test]
    fn cylinder_record_worker_preserves_geometry_selection_and_persists_metadata_history() {
        let points: Vec<_> = [-6., 0., 6.]
            .into_iter()
            .flat_map(|z| {
                (0..24).map(move |i| {
                    let t = i as f64 * std::f64::consts::TAU / 24.;
                    Point {
                        position: [3. * t.cos(), 3. * t.sin(), z],
                        color: Some([1, 2, 3]),
                        normal: None,
                    }
                })
            })
            .collect();
        let count = points.len();
        let original = Cloud { points };
        let mut d = Document {
            editor: Some(Editor::new(original.clone(), 8).unwrap()),
            selection: (0..count).collect(),
        };
        let selected = d.selection.clone();
        let work = WorkControl::default();
        let before = snapshot(&d);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            execute(
                &mut d,
                Operation::RecordCylinder {
                    label: "Cancelled".into(),
                    maximum_rms_mm: 1e-6
                },
                &cancelled
            )
            .is_err()
        );
        assert_eq!(snapshot(&d), before);
        let r = execute(
            &mut d,
            Operation::RecordCylinder {
                label: "Reference".into(),
                maximum_rms_mm: 1e-6,
            },
            &work,
        )
        .unwrap();
        assert!(r.edited);
        assert!(!r.reset_view);
        assert!(r.preview.is_none());
        assert_eq!(d.editor.as_ref().unwrap().cloud, original);
        assert_eq!(d.selection, selected);
        let before = snapshot(&d);
        assert!(
            execute(
                &mut d,
                Operation::RecordCylinder {
                    label: "Reference".into(),
                    maximum_rms_mm: 1e-6
                },
                &work
            )
            .is_err()
        );
        assert_eq!(snapshot(&d), before);
        let mut bytes = Vec::new();
        save_project(d.editor.as_ref().unwrap(), &mut bytes).unwrap();
        let mut reopened = load_project(bytes.as_slice()).unwrap();
        assert!(
            matches!(&reopened.measurements()[0].result,scancraft_geometry::measurement_snapshot::ResultValue::Cylinder {fit,..} if (fit.diameter_mm-6.).abs()<1e-6)
        );
        assert!(reopened.undo_measurement().unwrap());
        assert!(reopened.measurements().is_empty());
        assert!(reopened.redo_measurement().unwrap());
        assert_eq!(reopened.measurements()[0].source_points_mm.len(), count);
    }
    #[test]
    fn cylinder_fit_is_read_only_and_preserves_existing_redo() {
        let points: Vec<_> = [-6., 0., 6.]
            .into_iter()
            .flat_map(|z| {
                (0..24).map(move |i| {
                    let t = i as f64 * std::f64::consts::TAU / 24.;
                    Point {
                        position: [3. * t.cos(), 3. * t.sin(), z],
                        color: Some([1, 2, 3]),
                        normal: None,
                    }
                })
            })
            .collect();
        let count = points.len();
        let mut editor = Editor::new(Cloud { points }, 8).unwrap();
        editor.trim(&[0], false).unwrap();
        editor.undo();
        let mut d = Document {
            editor: Some(editor),
            selection: (0..count).collect(),
        };
        let before = snapshot(&d);
        let selected = d.selection.clone();
        let work = WorkControl::default();
        let r = execute(
            &mut d,
            Operation::FitCylinder {
                maximum_rms_mm: 1e-6,
            },
            &work,
        )
        .unwrap();
        assert!(r.message.contains("diameter 6.000000 mm"));
        assert!(!r.edited && !r.reset_view && !r.project_saved);
        assert!(r.preview.is_none());
        assert_eq!(snapshot(&d), before);
        assert_eq!(d.selection, selected);
        d.selection = vec![0, 1, 2];
        assert!(
            execute(
                &mut d,
                Operation::FitCylinder {
                    maximum_rms_mm: 1e-6
                },
                &work
            )
            .is_err()
        );
        assert_eq!(snapshot(&d), before);
        d.selection = selected;
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            execute(
                &mut d,
                Operation::FitCylinder {
                    maximum_rms_mm: 1e-6
                },
                &cancelled
            )
            .is_err()
        );
        assert_eq!(snapshot(&d), before);
        assert!(d.editor.as_mut().unwrap().redo());
    }
    #[test]
    fn reusable_frame_definition_save_load_and_cross_cloud_apply_are_separate() {
        let work = WorkControl::default();
        let mut source = document(3);
        let positions = [[10., 20., 30.], [10., 22., 30.], [9., 21., 30.]];
        for (p, position) in source
            .editor
            .as_mut()
            .unwrap()
            .cloud
            .points
            .iter_mut()
            .zip(positions)
        {
            p.position = position;
        }
        source.selection = vec![0, 1, 2];
        let before = snapshot(&source);
        let r = execute(
            &mut source,
            Operation::DefineFrame {
                origin: 0,
                axes: Some((1, 2)),
            },
            &work,
        )
        .unwrap();
        assert!(!r.edited && !r.project_saved && !r.reset_view);
        assert_eq!(snapshot(&source), before);
        let (_, frame) = r.coordinate_frame.unwrap();
        let temp = Temp::new();
        let target = temp.path.join("frame");
        let r = execute(
            &mut source,
            Operation::SaveFrame {
                label: "Cross-cloud".into(),
                frame: frame.clone(),
                target: target.clone(),
            },
            &work,
        )
        .unwrap();
        assert!(!r.edited && !r.project_saved && !r.reset_view);
        assert_eq!(snapshot(&source), before);
        let mut other = document(1);
        other.editor.as_mut().unwrap().cloud.points[0].position = [12., 23., 34.];
        let original = other.editor.as_ref().unwrap().cloud.clone();
        let before = snapshot(&other);
        let r = execute(
            &mut other,
            Operation::LoadFrame(target.join("frame.rapframe")),
            &work,
        )
        .unwrap();
        assert!(!r.edited && !r.project_saved && !r.reset_view);
        assert_eq!(snapshot(&other), before);
        let (label, frame) = r.coordinate_frame.unwrap();
        assert_eq!(label, "Cross-cloud");
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            execute(
                &mut other,
                Operation::ApplyFrame {
                    label: label.clone(),
                    frame: frame.clone()
                },
                &cancelled
            )
            .is_err()
        );
        assert_eq!(snapshot(&other), before);
        let r = execute(&mut other, Operation::ApplyFrame { label, frame }, &work).unwrap();
        assert!(r.edited && r.reset_view);
        assert_eq!(
            other.editor.as_ref().unwrap().cloud.points[0].position,
            [3., -2., 4.]
        );
        let mut bytes = Vec::new();
        save_project(other.editor.as_ref().unwrap(), &mut bytes).unwrap();
        let mut reopened = load_project(bytes.as_slice()).unwrap();
        assert!(reopened.undo());
        assert_eq!(reopened.cloud, original);
        let before = snapshot(&other);
        let path = target.join("frame.rapframe");
        let mut data = std::fs::read(&path).unwrap();
        data[16] ^= 1;
        std::fs::write(&path, data).unwrap();
        assert!(execute(&mut other, Operation::LoadFrame(path), &work).is_err());
        assert_eq!(snapshot(&other), before);
        let mut empty = Document::default();
        let r = execute(
            &mut empty,
            Operation::SaveFrame {
                label: "Empty cloud allowed".into(),
                frame: scancraft_geometry::coordinate_frame::Frame::at_origin([0.; 3]).unwrap(),
                target: temp.path.join("empty"),
            },
            &work,
        )
        .unwrap();
        assert!(!r.edited);
        assert!(empty.editor.is_none());
    }
    #[test]
    fn coordinate_alignment_is_atomic_and_project_history_restores_exact_source() {
        let mut d = document(4);
        let positions = [
            [10., 20., 30.],
            [10., 22., 30.],
            [9., 21., 30.],
            [12., 23., 34.],
        ];
        for (p, position) in d
            .editor
            .as_mut()
            .unwrap()
            .cloud
            .points
            .iter_mut()
            .zip(positions)
        {
            p.position = position;
            p.normal = Some([2., 0., 0.]);
        }
        let original = d.editor.as_ref().unwrap().cloud.clone();
        let measurement = scancraft_geometry::measurement_snapshot::Snapshot::distance(
            "Historical".into(),
            vec![positions[0], positions[1]],
            &WorkControl::default(),
        )
        .unwrap();
        d.editor
            .as_mut()
            .unwrap()
            .add_measurement(measurement)
            .unwrap();
        let historical = serde_json::to_vec(d.editor.as_ref().unwrap().measurements()).unwrap();
        d.selection = vec![2, 0, 1];
        let before = snapshot(&d);
        let work = WorkControl::default();
        assert!(
            execute(
                &mut d,
                Operation::AlignFrame {
                    origin: 0,
                    axes: Some((1, 99))
                },
                &work
            )
            .is_err()
        );
        assert_eq!(snapshot(&d), before);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            execute(
                &mut d,
                Operation::AlignFrame {
                    origin: 0,
                    axes: Some((1, 2))
                },
                &cancelled
            )
            .is_err()
        );
        assert_eq!(snapshot(&d), before);
        let r = execute(
            &mut d,
            Operation::AlignFrame {
                origin: 0,
                axes: Some((1, 2)),
            },
            &work,
        )
        .unwrap();
        assert!(r.edited && r.reset_view);
        assert_eq!(d.selection, vec![2, 0, 1]);
        assert_eq!(
            d.editor.as_ref().unwrap().cloud.points[3].position,
            [3., -2., 4.]
        );
        assert_eq!(
            serde_json::to_vec(d.editor.as_ref().unwrap().measurements()).unwrap(),
            historical
        );
        let mut bytes = Vec::new();
        save_project(d.editor.as_ref().unwrap(), &mut bytes).unwrap();
        let mut reopened = load_project(bytes.as_slice()).unwrap();
        assert!(reopened.undo());
        assert_eq!(reopened.cloud, original);
        assert_eq!(
            serde_json::to_vec(reopened.measurements()).unwrap(),
            historical
        );
        assert!(reopened.redo());
        assert_eq!(reopened.cloud.points[3].position, [3., -2., 4.]);
        d.selection = vec![0];
        assert!(
            execute(
                &mut d,
                Operation::AlignFrame {
                    origin: 0,
                    axes: None
                },
                &work
            )
            .unwrap()
            .edited
        );
    }
    #[test]
    fn point_plane_worker_excludes_probe_and_saves_local_index_without_geometry_edits() {
        let mut d = document(5);
        let points = [
            [-1., -1., 0.],
            [-1., 1., 0.],
            [1., -1., 0.],
            [1., 1., 0.],
            [0., 0., 5.],
        ];
        for (point, position) in d
            .editor
            .as_mut()
            .unwrap()
            .cloud
            .points
            .iter_mut()
            .zip(points)
        {
            point.position = position;
        }
        d.selection = vec![4, 2, 0, 3, 1];
        let before = snapshot(&d);
        let work = WorkControl::default();
        let result = execute(
            &mut d,
            Operation::PointPlane {
                probe: 4,
                maximum_plane_rms_mm: 0.,
                label: None,
            },
            &work,
        )
        .unwrap();
        assert!(result.message.contains("distance 5.000000 mm"));
        assert!(!result.edited);
        assert_eq!(snapshot(&d), before);
        assert!(
            execute(
                &mut d,
                Operation::PointPlane {
                    probe: 99,
                    maximum_plane_rms_mm: 0.,
                    label: None
                },
                &work
            )
            .is_err()
        );
        assert_eq!(snapshot(&d), before);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            execute(
                &mut d,
                Operation::PointPlane {
                    probe: 4,
                    maximum_plane_rms_mm: 0.,
                    label: Some("Height".into())
                },
                &cancelled
            )
            .is_err()
        );
        assert_eq!(snapshot(&d), before);
        assert!(
            execute(
                &mut d,
                Operation::PointPlane {
                    probe: 4,
                    maximum_plane_rms_mm: 0.,
                    label: Some("Height".into())
                },
                &work
            )
            .unwrap()
            .edited
        );
        assert_eq!(d.selection, vec![4, 2, 0, 3, 1]);
        assert_eq!(
            d.editor
                .as_ref()
                .unwrap()
                .cloud
                .points
                .iter()
                .map(|p| p.position)
                .collect::<Vec<_>>(),
            points
        );
        let mut bytes = Vec::new();
        save_project(d.editor.as_ref().unwrap(), &mut bytes).unwrap();
        let mut reopened = load_project(bytes.as_slice()).unwrap();
        assert!(matches!(
            reopened.measurements()[0].result,
            scancraft_geometry::measurement_snapshot::ResultValue::PointPlane { probe: 4, .. }
        ));
        reopened.undo_measurement().unwrap();
        assert!(reopened.measurements().is_empty());
    }
    #[test]
    fn direct_snapshot_worker_retains_vertex_and_commits_only_valid_records() {
        let mut d = document(3);
        d.editor.as_mut().unwrap().cloud.points[0].position = [0., 0., 0.];
        d.editor.as_mut().unwrap().cloud.points[1].position = [3., 0., 0.];
        d.editor.as_mut().unwrap().cloud.points[2].position = [0., 4., 0.];
        d.selection = vec![2, 0, 1];
        let work = WorkControl::default();
        let result = execute(
            &mut d,
            Operation::RecordAngle {
                label: "Corner".into(),
                vertex: 0,
            },
            &work,
        )
        .unwrap();
        assert!(result.edited);
        assert_eq!(d.selection, vec![2, 0, 1]);
        assert!(matches!(
            d.editor.as_ref().unwrap().measurements()[0].result,
            scancraft_geometry::measurement_snapshot::ResultValue::Angle {
                degrees: 90.,
                vertex: 1
            }
        ));
        let before = snapshot(&d);
        assert!(
            execute(
                &mut d,
                Operation::RecordAngle {
                    label: "Bad".into(),
                    vertex: 9
                },
                &work
            )
            .is_err()
        );
        assert_eq!(snapshot(&d), before);
        d.selection = vec![1, 2];
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            execute(
                &mut d,
                Operation::RecordDistance {
                    label: "Length".into()
                },
                &cancelled
            )
            .is_err()
        );
        assert_eq!(snapshot(&d), before);
        execute(
            &mut d,
            Operation::RecordDistance {
                label: "Length".into(),
            },
            &work,
        )
        .unwrap();
        let mut bytes = Vec::new();
        save_project(d.editor.as_ref().unwrap(), &mut bytes).unwrap();
        let mut reopened = load_project(bytes.as_slice()).unwrap();
        assert_eq!(reopened.measurements().len(), 2);
        reopened.undo_measurement().unwrap();
        assert_eq!(reopened.measurements().len(), 1);
        reopened.redo_measurement().unwrap();
        assert!(matches!(
            reopened.measurements()[1].result,
            scancraft_geometry::measurement_snapshot::ResultValue::Distance { distance_mm: 5. }
        ));
    }
    #[test]
    fn direct_measurements_are_explicit_read_only_and_reject_invalid_selection() {
        let mut d = document(3);
        d.editor.as_mut().unwrap().cloud.points[0].position = [0., 0., 0.];
        d.editor.as_mut().unwrap().cloud.points[1].position = [3., 0., 0.];
        d.editor.as_mut().unwrap().cloud.points[2].position = [0., 4., 0.];
        d.editor.as_mut().unwrap().trim(&[0], false).unwrap();
        d.editor.as_mut().unwrap().undo();
        let before = snapshot(&d);
        d.selection = vec![2, 1];
        let work = WorkControl::default();
        let result = execute(&mut d, Operation::MeasureDistance, &work).unwrap();
        assert!(result.message.contains("5.000000 mm"));
        assert!(!result.edited);
        assert!(result.preview.is_none());
        assert_eq!(d.selection, vec![2, 1]);
        d.selection = vec![2, 0, 1];
        let result = execute(&mut d, Operation::MeasureAngle { vertex: 0 }, &work).unwrap();
        assert!(
            result
                .message
                .contains("90.000000 degrees at source index 0")
        );
        assert!(!result.edited);
        assert_eq!(snapshot(&d), before);
        assert!(execute(&mut d, Operation::MeasureAngle { vertex: 9 }, &work).is_err());
        assert!(execute(&mut d, Operation::MeasureDistance, &work).is_err());
        d.selection = vec![0, 0];
        assert!(execute(&mut d, Operation::MeasureDistance, &work).is_err());
        d.selection = vec![0, 99];
        assert!(execute(&mut d, Operation::MeasureDistance, &work).is_err());
        let cancelled = WorkControl::default();
        cancelled.cancel();
        d.selection = vec![1, 2];
        assert!(execute(&mut d, Operation::MeasureDistance, &cancelled).is_err());
        assert_eq!(snapshot(&d), before);
        assert!(d.editor.as_mut().unwrap().redo());
    }
    struct Temp {
        path: PathBuf,
    }
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "scancraft-worker-test-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&path).unwrap();
            Self { path }
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
    #[test]
    fn statistical_preview_changes_only_selection_and_delete_undo_restores_exact_attributes() {
        let cloud = Cloud {
            points: vec![[0., 0., 0.], [1., 0., 0.], [10., 0., 0.]]
                .into_iter()
                .map(|position| scancraft_geometry::Point {
                    position,
                    color: Some([80, 180, 240]),
                    normal: Some([0., 0., 1.]),
                })
                .collect(),
        };
        let mut document = Document {
            editor: Some(Editor::new(cloud.clone(), 8).unwrap()),
            selection: vec![0],
        };
        let work = WorkControl::default();
        let initial = snapshot(&document);
        let report = execute(
            &mut document,
            Operation::StatisticalPreview {
                neighbors: 1,
                sigma: 1.,
            },
            &work,
        )
        .unwrap();
        assert!(!report.edited);
        assert_eq!(snapshot(&document), initial);
        assert_eq!(document.selection, vec![2]);
        execute(&mut document, Operation::Trim { keep: false }, &work).unwrap();
        assert_eq!(
            document.editor.as_ref().unwrap().cloud.points,
            cloud.points[..2]
        );
        execute(&mut document, Operation::Undo, &work).unwrap();
        assert_eq!(document.editor.as_ref().unwrap().cloud, cloud);
        document.selection = vec![0];
        let before = snapshot(&document);
        assert!(
            execute(
                &mut document,
                Operation::StatisticalPreview {
                    neighbors: 3,
                    sigma: 1.
                },
                &work
            )
            .is_err()
        );
        assert_eq!(snapshot(&document), before);
        assert_eq!(document.selection, vec![0]);
    }
    #[test]
    fn scan_folder_load_relocates_references_and_failure_preserves_document_and_selection() {
        let temp = Temp::new();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence");
        let sources = root.join("project-origin-gui-render/result");
        let target = temp.path.join("scan");
        let work = WorkControl::default();
        scancraft_workflow::scan_project::export_file(
            &sources.join("project.orproj"),
            &sources,
            &root.join("recorded-stereo-dataset/left.raptor"),
            &root.join("recorded-stereo-dataset/right.raptor"),
            &target,
            &work,
        )
        .unwrap();
        let mut document = Document::default();
        let loaded = execute(
            &mut document,
            Operation::Load {
                path: target.clone(),
                unit_scale: None,
            },
            &work,
        )
        .unwrap();
        assert!(loaded.message.contains("Opened verified scan folder"));
        assert_eq!(
            document
                .editor
                .as_ref()
                .unwrap()
                .origin()
                .unwrap()
                .bundle_directory_display,
            target.join("project").display().to_string()
        );
        let saved = temp.path.join("standalone.orproj");
        execute(
            &mut document,
            Operation::Save {
                path: saved.clone(),
                project: true,
            },
            &work,
        )
        .unwrap();
        let bytes = std::fs::read(&saved).unwrap();
        assert_eq!(&bytes[..8], b"ORPROJ06");
        execute(
            &mut document,
            Operation::Load {
                path: saved,
                unit_scale: None,
            },
            &work,
        )
        .unwrap();
        let checked = execute(
            &mut document,
            Operation::VerifyRecordings {
                directory: target.join("recordings"),
            },
            &work,
        )
        .unwrap();
        assert!(checked.message.contains("verification passed"));
        assert!(!checked.edited);
        document.selection = vec![0];
        let before = snapshot(&document);
        std::fs::write(target.join("recordings/right.raptor"), b"corrupt").unwrap();
        assert!(
            execute(
                &mut document,
                Operation::Load {
                    path: target,
                    unit_scale: None
                },
                &work
            )
            .is_err()
        );
        assert_eq!(snapshot(&document), before);
        assert_eq!(document.selection, vec![0]);
    }
    #[test]
    fn recorded_origin_survives_gui_worker_edit_save_load_and_missing_bundle() {
        let temp = Temp::new();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/recorded-stereo-dataset");
        let target = temp.path.join("result");
        let mut document = Document::default();
        let work = WorkControl::default();
        execute(
            &mut document,
            Operation::ReconstructRecordings {
                inputs: scancraft_workflow::recording_reconstruction::Inputs {
                    calibration: root.join("calibration.rapcal"),
                    left: root.join("left.raptor"),
                    left_index: 1,
                    right: root.join("right.raptor"),
                    right_index: 2,
                    settings: root.join("request.json"),
                },
                output: target.clone(),
            },
            &work,
        )
        .unwrap();
        let origin = document.editor.as_ref().unwrap().origin().unwrap().clone();
        assert_eq!(document.editor.as_ref().unwrap().cloud.points.len(), 3);
        document.selection = vec![0];
        execute(&mut document, Operation::Trim { keep: true }, &work).unwrap();
        let snapshot_before = snapshot(&document);
        let verified = execute(
            &mut document,
            Operation::VerifySources {
                directory: target.clone(),
            },
            &work,
        )
        .unwrap();
        assert!(verified.message.contains("verification passed"));
        assert!(!verified.edited);
        assert!(!verified.project_saved);
        assert_eq!(snapshot(&document), snapshot_before);
        std::fs::rename(&target, temp.path.join("moved-bundle")).unwrap();
        let moved = temp.path.join("moved-bundle");
        let verified = execute(
            &mut document,
            Operation::VerifySources {
                directory: moved.clone(),
            },
            &work,
        )
        .unwrap();
        assert!(verified.message.contains("5/5"));
        assert_eq!(snapshot(&document), snapshot_before);
        let archive = temp.path.join("archive");
        let packaged = execute(
            &mut document,
            Operation::ExportProjectBundle {
                sources: moved.clone(),
                target: archive.clone(),
            },
            &work,
        )
        .unwrap();
        assert!(!packaged.edited);
        assert!(!packaged.project_saved);
        assert_eq!(snapshot(&document), snapshot_before);
        let mut archived = scancraft_geometry::formats::load_project(
            std::fs::File::open(archive.join("project.orproj")).unwrap(),
        )
        .unwrap();
        assert_eq!(archived.cloud.points.len(), 1);
        assert!(archived.undo());
        assert_eq!(archived.cloud.points.len(), 3);
        std::fs::write(moved.join("request.json"), b"damaged").unwrap();
        let failed = execute(
            &mut document,
            Operation::VerifySources { directory: moved },
            &work,
        )
        .unwrap();
        assert!(failed.message.contains("verification failed"));
        assert!(!failed.edited);
        assert_eq!(snapshot(&document), snapshot_before);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            execute(
                &mut document,
                Operation::VerifySources {
                    directory: target.clone()
                },
                &cancelled
            )
            .is_err()
        );
        assert_eq!(snapshot(&document), snapshot_before);
        let project = temp.path.join("edited.orproj");
        execute(
            &mut document,
            Operation::Save {
                path: project.clone(),
                project: true,
            },
            &work,
        )
        .unwrap();
        let mut reopened = Document::default();
        execute(
            &mut reopened,
            Operation::Load {
                path: project,
                unit_scale: None,
            },
            &work,
        )
        .unwrap();
        assert_eq!(reopened.editor.as_ref().unwrap().origin(), Some(&origin));
        assert_eq!(reopened.editor.as_ref().unwrap().cloud.points.len(), 1);
        execute(&mut reopened, Operation::Undo, &work).unwrap();
        assert_eq!(reopened.editor.as_ref().unwrap().cloud.points.len(), 3);
        execute(&mut reopened, Operation::Redo, &work).unwrap();
        assert_eq!(reopened.editor.as_ref().unwrap().origin(), Some(&origin));
    }
    fn reconstruction_inputs() -> scancraft_workflow::FileInputs {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
        scancraft_workflow::FileInputs {
            calibration: root.join("calibration.rapcal"),
            left: root.join("left.png"),
            right: root.join("right.png"),
            settings: root.join("request.json"),
        }
    }
    #[test]
    fn reconstruction_job_publishes_before_replacing_document_and_keeps_saved_coordinates() {
        let temp = Temp::new();
        let output = temp.path.join("reconstruction");
        let mut doc = document(10);
        let result = execute(
            &mut doc,
            Operation::Reconstruct {
                inputs: reconstruction_inputs(),
                output: output.clone(),
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert!(result.reset_view && result.project_saved);
        assert_eq!(result.opened_path, Some(output.join("project.orproj")));
        assert!(doc.selection.is_empty());
        assert_eq!(doc.editor.as_ref().unwrap().cloud.points.len(), 3);
        let saved = load_project(File::open(output.join("project.orproj")).unwrap()).unwrap();
        assert_eq!(saved.cloud, doc.editor.as_ref().unwrap().cloud);
        let before = snapshot(&doc);
        assert!(
            execute(
                &mut doc,
                Operation::Reconstruct {
                    inputs: reconstruction_inputs(),
                    output
                },
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(snapshot(&doc), before);
    }
    #[test]
    fn failed_or_cancelled_reconstruction_keeps_original_history_and_selection() {
        let temp = Temp::new();
        let mut doc = document(10);
        execute(
            &mut doc,
            Operation::Trim { keep: false },
            &WorkControl::default(),
        )
        .unwrap();
        execute(&mut doc, Operation::Undo, &WorkControl::default()).unwrap();
        doc.selection = vec![2];
        let before = snapshot(&doc);
        let output = temp.path.join("result");
        let work = WorkControl::default();
        work.cancel();
        assert!(
            execute(
                &mut doc,
                Operation::Reconstruct {
                    inputs: reconstruction_inputs(),
                    output: output.clone()
                },
                &work
            )
            .is_err()
        );
        let mut missing = reconstruction_inputs();
        missing.calibration = temp.path.join("missing");
        assert!(
            execute(
                &mut doc,
                Operation::Reconstruct {
                    inputs: missing,
                    output: output.clone()
                },
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(snapshot(&doc), before);
        assert_eq!(doc.selection, [2]);
        assert!(!output.exists());
    }
    #[test]
    fn rejected_reconstruction_saves_diagnostics_without_replacing_current_cloud() {
        let temp = Temp::new();
        let mut inputs = reconstruction_inputs();
        inputs.left = temp.path.join("left.png");
        inputs.right = temp.path.join("right.png");
        for path in [&inputs.left, &inputs.right] {
            image::save_buffer(path, &[20; 2048], 64, 32, image::ColorType::L8).unwrap();
        }
        let mut doc = document(10);
        let before = snapshot(&doc);
        let output = temp.path.join("empty");
        let result = execute(
            &mut doc,
            Operation::Reconstruct {
                inputs,
                output: output.clone(),
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert!(!result.reset_view && !result.project_saved);
        assert_eq!(snapshot(&doc), before);
        assert_eq!(doc.selection, [0]);
        assert!(output.join("report.json").exists());
        assert!(!output.join("project.orproj").exists());
        assert!(!output.join("points.ply").exists());
    }
    #[test]
    fn cli_reconstructed_project_loads_by_content_and_can_be_edited_and_undone() {
        let fixture = include_bytes!("../../../evidence/offline-cli-verified/project.orproj");
        let temp = Temp::new();
        for name in [
            "reconstructed.orproj",
            "reconstructed.orproject",
            "reconstructed.bin",
        ] {
            let path = temp.path.join(name);
            std::fs::write(&path, fixture).unwrap();
            let mut doc = document(10);
            let result = execute(
                &mut doc,
                Operation::Load {
                    path,
                    unit_scale: Some(25.4),
                },
                &WorkControl::default(),
            )
            .unwrap();
            assert!(result.reset_view);
            assert!(doc.selection.is_empty());
            let cloud = doc.editor.as_ref().unwrap().cloud.clone();
            assert_eq!(cloud.points.len(), 3);
            for point in &cloud.points {
                assert!((point.position[2] - 400.).abs() < 1e-7);
            }
            doc.selection = vec![1];
            execute(
                &mut doc,
                Operation::Trim { keep: false },
                &WorkControl::default(),
            )
            .unwrap();
            assert_eq!(doc.editor.as_ref().unwrap().cloud.points.len(), 2);
            execute(&mut doc, Operation::Undo, &WorkControl::default()).unwrap();
            assert_eq!(doc.editor.as_ref().unwrap().cloud, cloud);
        }
    }
    #[test]
    fn startup_import_refuses_unspecified_ply_units_until_conversion_is_chosen() {
        let temp = Temp::new();
        let path = temp.path.join("unknown-units.ply");
        let cloud = Cloud {
            points: vec![Point {
                position: [1., 2., 3.],
                color: None,
                normal: None,
            }],
        };
        let mut bytes = Vec::new();
        write_ply(&cloud, &mut bytes).unwrap();
        let text = String::from_utf8(bytes)
            .unwrap()
            .replace("comment units millimeters\n", "");
        std::fs::write(&path, text).unwrap();
        let mut doc = document(10);
        let before = snapshot(&doc);
        assert!(
            execute(
                &mut doc,
                Operation::Load {
                    path: path.clone(),
                    unit_scale: None
                },
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(snapshot(&doc), before);
        execute(
            &mut doc,
            Operation::Load {
                path,
                unit_scale: Some(25.4),
            },
            &WorkControl::default(),
        )
        .unwrap();
        let point = doc.editor.as_ref().unwrap().cloud.points[0].position;
        for (actual, expected) in point.into_iter().zip([25.4, 50.8, 76.2]) {
            assert!((actual - expected).abs() < 1e-12);
        }
    }
    #[test]
    fn renamed_project_preserves_history_and_corrupt_import_preserves_current_document() {
        let temp = Temp::new();
        let path = temp.path.join("project.ply");
        let mut original = document(20);
        execute(
            &mut original,
            Operation::Trim { keep: false },
            &WorkControl::default(),
        )
        .unwrap();
        let bytes = snapshot(&original);
        std::fs::write(&path, &bytes).unwrap();
        let mut doc = document(10);
        execute(
            &mut doc,
            Operation::Load {
                path: path.clone(),
                unit_scale: Some(1.),
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(snapshot(&doc), bytes);
        execute(&mut doc, Operation::Undo, &WorkControl::default()).unwrap();
        assert_eq!(doc.editor.as_ref().unwrap().cloud.points.len(), 20);
        let before = snapshot(&doc);
        doc.selection = vec![2];
        let mut corrupt = bytes;
        let n = corrupt.len();
        corrupt[n - 1] ^= 1;
        std::fs::write(&path, corrupt).unwrap();
        assert!(
            execute(
                &mut doc,
                Operation::Load {
                    path,
                    unit_scale: Some(1.)
                },
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(snapshot(&doc), before);
        assert_eq!(doc.selection, [2]);
    }
    #[test]
    fn unexpected_worker_termination_recovers_document_and_does_not_stay_busy() {
        let d = document(100);
        let before = snapshot(&d);
        let shared = Arc::new(Mutex::new(d));
        let worker = shared.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || {
            let _document = worker.lock().unwrap();
            drop(tx);
            panic!("injected worker termination before edit commit");
        });
        let mut job = Job {
            control: Arc::new(WorkControl::default()),
            rx,
            handle: Some(handle),
            document: Some(shared),
        };
        let start = Instant::now();
        let output = loop {
            if let Some(o) = job.poll() {
                break o;
            }
            assert!(start.elapsed() < Duration::from_secs(3));
            thread::yield_now();
        };
        assert!(output.result.is_err());
        assert_eq!(snapshot(&output.document), before);
        assert!(job.poll().is_none());
    }
    #[test]
    fn failed_and_cancelled_proposals_preserve_geometry_and_redo() {
        let mut d = document(100);
        let e = d.editor.as_mut().unwrap();
        e.trim(&[1, 2], false).unwrap();
        e.undo();
        let before = snapshot(&d);
        let selection = d.selection.clone();
        assert!(execute(&mut d, Operation::Downsample(-1.), &WorkControl::default()).is_err());
        let work = WorkControl::default();
        work.cancel();
        assert!(execute(&mut d, Operation::Trim { keep: true }, &work).is_err());
        assert_eq!(snapshot(&d), before);
        assert_eq!(d.selection, selection);
        assert!(d.editor.as_mut().unwrap().redo());
        assert_eq!(d.editor.as_ref().unwrap().cloud.points.len(), 98);
    }
    #[test]
    fn selected_normal_flip_preserves_selection_and_survives_history_save_reopen() {
        let cloud = Cloud {
            points: (0..3)
                .map(|i| scancraft_geometry::Point {
                    position: [i as f64, 2., 3.],
                    color: Some([1, 2, 3]),
                    normal: Some([0., 0., 2.]),
                })
                .collect(),
        };
        let mut document = Document {
            editor: Some(Editor::new(cloud.clone(), 8).unwrap()),
            selection: vec![1],
        };
        let work = WorkControl::default();
        let result = execute(&mut document, Operation::FlipNormals, &work).unwrap();
        assert!(result.edited);
        assert_eq!(document.selection, [1]);
        let flipped = document.editor.as_ref().unwrap().cloud.clone();
        assert_eq!(flipped.points[1].normal, Some([-0., -0., -2.]));
        assert_eq!(flipped.points[0], cloud.points[0]);
        assert_eq!(flipped.points[2], cloud.points[2]);
        execute(&mut document, Operation::Undo, &work).unwrap();
        assert_eq!(document.editor.as_ref().unwrap().cloud, cloud);
        execute(&mut document, Operation::Redo, &work).unwrap();
        assert_eq!(document.editor.as_ref().unwrap().cloud, flipped);
        document.selection = vec![99];
        let before = snapshot(&document);
        assert!(execute(&mut document, Operation::FlipNormals, &work).is_err());
        assert_eq!(snapshot(&document), before);
        document.selection = vec![1];
        let before = snapshot(&document);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(execute(&mut document, Operation::FlipNormals, &cancelled).is_err());
        assert_eq!(snapshot(&document), before);
        let temp = Temp::new();
        let path = temp.path.join("normal-flip.orproj");
        execute(
            &mut document,
            Operation::Save {
                path: path.clone(),
                project: true,
            },
            &work,
        )
        .unwrap();
        let mut reopened = Document::default();
        execute(
            &mut reopened,
            Operation::Load {
                path,
                unit_scale: None,
            },
            &work,
        )
        .unwrap();
        assert_eq!(reopened.editor.as_ref().unwrap().cloud, flipped);
        execute(&mut reopened, Operation::Undo, &work).unwrap();
        assert_eq!(reopened.editor.as_ref().unwrap().cloud, cloud);
    }
    #[test]
    fn sphere_measurement_preserves_document_selection_and_redo_history() {
        let mut points = Vec::new();
        for axis in 0..3 {
            for sign in [-1., 1.] {
                let mut position = [1., 2., 3.];
                position[axis] += 5. * sign;
                points.push(scancraft_geometry::Point {
                    position,
                    color: Some([1, 2, 3]),
                    normal: None,
                });
            }
        }
        let mut editor = Editor::new(Cloud { points }, 8).unwrap();
        editor.trim(&[0], false).unwrap();
        editor.undo();
        let mut document = Document {
            editor: Some(editor),
            selection: (0..6).collect(),
        };
        let before = snapshot(&document);
        let selection = document.selection.clone();
        let work = WorkControl::default();
        let result = execute(&mut document, Operation::FitSphere, &work).unwrap();
        assert!(result.message.contains("diameter 10.000000 mm"));
        assert!(result.message.contains("geometric RMS"));
        assert!(!result.edited);
        assert_eq!(snapshot(&document), before);
        assert_eq!(document.selection, selection);
        document.selection = vec![0, 1, 2];
        let before = snapshot(&document);
        assert!(execute(&mut document, Operation::FitSphere, &work).is_err());
        assert_eq!(snapshot(&document), before);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(execute(&mut document, Operation::FitSphere, &cancelled).is_err());
        assert_eq!(snapshot(&document), before);
        assert!(document.editor.as_mut().unwrap().redo());
    }
    #[test]
    fn circle_measurement_reports_diameter_without_editing_or_consuming_history() {
        let points = (0..8)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 8.;
                scancraft_geometry::Point {
                    position: [2. + 5. * a.cos(), 3. + 5. * a.sin(), 4.],
                    color: None,
                    normal: None,
                }
            })
            .collect();
        let mut editor = Editor::new(Cloud { points }, 8).unwrap();
        editor.trim(&[0], false).unwrap();
        editor.undo();
        let mut document = Document {
            editor: Some(editor),
            selection: (0..8).collect(),
        };
        let before = snapshot(&document);
        let selection = document.selection.clone();
        let result = execute(
            &mut document,
            Operation::FitCircle {
                maximum_plane_rms_mm: 0.01,
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert!(result.message.contains("diameter 10.000000 mm"));
        assert!(!result.edited);
        assert_eq!(snapshot(&document), before);
        assert_eq!(document.selection, selection);
        assert!(
            execute(
                &mut document,
                Operation::FitCircle {
                    maximum_plane_rms_mm: -1.
                },
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(snapshot(&document), before);
        assert!(document.editor.as_mut().unwrap().redo());
    }
    #[test]
    fn saved_measurement_worker_marks_dirty_and_project_reopen_retains_source_evidence() {
        let positions = vec![
            [5., 0., 0.],
            [-5., 0., 0.],
            [0., 5., 0.],
            [0., -5., 0.],
            [0., 0., 5.],
            [0., 0., -5.],
        ];
        let cloud = Cloud {
            points: positions
                .iter()
                .map(|p| scancraft_geometry::Point {
                    position: *p,
                    color: None,
                    normal: None,
                })
                .collect(),
        };
        let mut document = Document {
            editor: Some(Editor::new(cloud.clone(), 8).unwrap()),
            selection: (0..6).collect(),
        };
        let work = WorkControl::default();
        let result = execute(
            &mut document,
            Operation::RecordSphere {
                label: "Ball".into(),
            },
            &work,
        )
        .unwrap();
        assert!(result.edited);
        assert_eq!(document.editor.as_ref().unwrap().cloud, cloud);
        assert_eq!(document.selection, (0..6).collect::<Vec<_>>());
        let before = snapshot(&document);
        assert!(
            execute(
                &mut document,
                Operation::RecordSphere {
                    label: "Ball".into()
                },
                &work
            )
            .is_err()
        );
        assert_eq!(snapshot(&document), before);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            execute(
                &mut document,
                Operation::RecordSphere {
                    label: "Other".into()
                },
                &cancelled
            )
            .is_err()
        );
        assert_eq!(snapshot(&document), before);
        let temp = Temp::new();
        let path = temp.path.join("measured.orproj");
        execute(
            &mut document,
            Operation::Save {
                path: path.clone(),
                project: true,
            },
            &work,
        )
        .unwrap();
        let mut loaded = Document::default();
        execute(
            &mut loaded,
            Operation::Load {
                path,
                unit_scale: None,
            },
            &work,
        )
        .unwrap();
        let record = &loaded.editor.as_ref().unwrap().measurements()[0];
        assert_eq!(record.label, "Ball");
        assert_eq!(record.source_points_mm, positions);
        assert!(!record.physical_accuracy_qualified);
    }
    #[test]
    fn measurement_management_worker_is_reversible_and_leaves_geometry_selection_intact() {
        let mut document = document(6);
        let positions = vec![
            [5., 0., 0.],
            [-5., 0., 0.],
            [0., 5., 0.],
            [0., -5., 0.],
            [0., 0., 5.],
            [0., 0., -5.],
        ];
        document
            .editor
            .as_mut()
            .unwrap()
            .add_measurement(
                scancraft_geometry::measurement_snapshot::Snapshot::sphere(
                    "Ball".into(),
                    positions,
                    &WorkControl::default(),
                )
                .unwrap(),
            )
            .unwrap();
        let geometry = document.editor.as_ref().unwrap().cloud.clone();
        let selection = document.selection.clone();
        let work = WorkControl::default();
        assert!(
            execute(
                &mut document,
                Operation::RenameMeasurement {
                    index: 0,
                    label: "Reference".into()
                },
                &work
            )
            .unwrap()
            .edited
        );
        execute(
            &mut document,
            Operation::RemoveMeasurement { index: 0 },
            &work,
        )
        .unwrap();
        assert!(document.editor.as_ref().unwrap().measurements().is_empty());
        execute(&mut document, Operation::UndoMeasurement, &work).unwrap();
        assert_eq!(
            document.editor.as_ref().unwrap().measurements()[0].label,
            "Reference"
        );
        execute(&mut document, Operation::UndoMeasurement, &work).unwrap();
        assert_eq!(
            document.editor.as_ref().unwrap().measurements()[0].label,
            "Ball"
        );
        execute(&mut document, Operation::RedoMeasurement, &work).unwrap();
        let before = snapshot(&document);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            execute(
                &mut document,
                Operation::RemoveMeasurement { index: 0 },
                &cancelled
            )
            .is_err()
        );
        assert_eq!(snapshot(&document), before);
        assert!(
            execute(
                &mut document,
                Operation::RenameMeasurement {
                    index: 99,
                    label: "Bad".into()
                },
                &work
            )
            .is_err()
        );
        assert_eq!(snapshot(&document), before);
        assert_eq!(document.editor.as_ref().unwrap().cloud, geometry);
        assert_eq!(document.selection, selection);
    }
    #[test]
    fn edits_and_undo_redo_return_correct_full_resolution_previews() {
        let mut d = document(100);
        let before = snapshot(&d);
        execute(
            &mut d,
            Operation::SelectBox {
                min: [2., 0., 0.],
                max: [4., 7., 0.],
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(d.selection, [2, 3, 4]);
        let result = execute(
            &mut d,
            Operation::Trim { keep: false },
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(result.preview.unwrap().total, 97);
        assert!(result.edited);
        execute(&mut d, Operation::Undo, &WorkControl::default()).unwrap();
        assert_eq!(d.editor.as_ref().unwrap().cloud.points.len(), 100);
        // Undo intentionally leaves a redo record, while geometry and all point attributes are exact.
        let loaded = load_project(before.as_slice()).unwrap();
        assert_eq!(d.editor.as_ref().unwrap().cloud, loaded.cloud);
        execute(&mut d, Operation::Redo, &WorkControl::default()).unwrap();
        assert_eq!(d.editor.as_ref().unwrap().cloud.points.len(), 97);
    }
    #[test]
    fn cancellation_during_filtering_returns_owned_original_document() {
        let d = document(20000);
        let before = snapshot(&d);
        let mut job = Job::start(
            d,
            Operation::Radius {
                radius: 1e9,
                neighbors: 19999,
            },
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while job.control.progress().0 != "Processing" && Instant::now() < deadline {
            thread::yield_now();
        }
        assert_eq!(job.control.progress().0, "Processing");
        let start = Instant::now();
        job.control.cancel();
        let output = loop {
            if let Some(out) = job.poll() {
                break out;
            }
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "cancellation failed to return"
            );
            thread::sleep(Duration::from_millis(1));
        };
        let cancellation_ms = start.elapsed().as_secs_f64() * 1000.;
        assert!(output.result.is_err());
        assert_eq!(snapshot(&output.document), before);
        assert_eq!(output.document.selection, [0]);
        println!("filter cancellation observed in {:.3} ms", cancellation_ms);
    }
    #[test]
    fn save_roundtrip_is_atomic_and_never_overwrites_existing_data() {
        let temp = Temp::new();
        let path = temp.path.join("test.orproject");
        let mut d = document(100);
        let saved = execute(
            &mut d,
            Operation::Save {
                path: path.clone(),
                project: true,
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert!(saved.project_saved);
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            load_project(bytes.as_slice()).unwrap().cloud,
            d.editor.as_ref().unwrap().cloud
        );
        assert!(
            execute(
                &mut d,
                Operation::Save {
                    path: path.clone(),
                    project: true
                },
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!temp.path.join("test.orproject.partial").exists());
        let work = WorkControl::default();
        work.cancel();
        let cancelled = temp.path.join("cancelled.orproject");
        assert!(
            execute(
                &mut d,
                Operation::Save {
                    path: cancelled.clone(),
                    project: true
                },
                &work
            )
            .is_err()
        );
        assert!(!cancelled.exists());
    }
    #[test]
    fn background_load_restores_editable_history_and_failed_import_keeps_document() {
        let temp = Temp::new();
        let path = temp.path.join("editable.orproject");
        let mut d = document(100);
        d.editor.as_mut().unwrap().trim(&[1, 2], false).unwrap();
        execute(
            &mut d,
            Operation::Save {
                path: path.clone(),
                project: true,
            },
            &WorkControl::default(),
        )
        .unwrap();
        let mut loaded = Document::default();
        let result = execute(
            &mut loaded,
            Operation::Load {
                path,
                unit_scale: Some(1000.),
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert!(result.reset_view);
        assert_eq!(snapshot(&loaded), snapshot(&d));
        assert!(loaded.selection.is_empty());
        loaded.editor.as_mut().unwrap().undo();
        assert_eq!(loaded.editor.as_ref().unwrap().cloud.points.len(), 100);
        let before = snapshot(&loaded);
        assert!(
            execute(
                &mut loaded,
                Operation::Load {
                    path: temp.path.join("missing.ply"),
                    unit_scale: Some(1.)
                },
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(snapshot(&loaded), before);
    }
    #[test]
    fn failed_save_retains_a_preexisting_partial_file() {
        let temp = Temp::new();
        let partial = temp.path.join("test.ply.partial");
        std::fs::write(&partial, b"previous recovery data").unwrap();
        let mut d = document(10);
        assert!(
            execute(
                &mut d,
                Operation::Save {
                    path: temp.path.join("test.ply"),
                    project: false
                },
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(std::fs::read(partial).unwrap(), b"previous recovery data");
    }
    #[test]
    fn preview_limits_rendering_but_keeps_unsampled_bounds_and_indices() {
        let mut d = document(100001);
        d.editor.as_mut().unwrap().cloud.points[100000].position[2] = 999.;
        let selection: Vec<_> = (0..100001).step_by(2).collect();
        let p = build_preview(
            &d.editor.as_ref().unwrap().cloud,
            &selection,
            &WorkControl::default(),
        )
        .unwrap();
        assert!(p.points.len() <= 50000);
        assert_eq!(p.total, 100001);
        assert_eq!(p.selected, 50001);
        assert_eq!(p.bounds.unwrap().1[2], 999.);
        for point in p.points {
            assert_eq!(point.selected, point.index % 2 == 0);
            assert_eq!(
                point.position,
                d.editor.as_ref().unwrap().cloud.points[point.index].position
            );
        }
    }
    #[test]
    fn screen_selection_edits_unsampled_points_and_undo_restores_original_attributes() {
        use scancraft_geometry::selection::{
            OrthographicProjection, ScreenShape, SelectionMode, Visibility,
        };
        let mut d = document(100001);
        let last = d.editor.as_ref().unwrap().cloud.points[100000].clone();
        let projection = OrthographicProjection::new(
            ([0., 0., 0.], [100000., 7., 1.]),
            [0.; 2],
            1.,
            [0., 0., 1000., 1000.],
            [0.; 2],
        )
        .unwrap();
        let q = projection.project(last.position).unwrap();
        let result = execute(
            &mut d,
            Operation::SelectScreen {
                projection,
                shape: ScreenShape::Rectangle {
                    min: [q.xy[0] - 0.1, q.xy[1] - 0.1],
                    max: [q.xy[0] + 0.1, q.xy[1] + 0.1],
                },
                visibility: Visibility::Through,
                mode: SelectionMode::Replace,
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert!(d.selection.contains(&100000));
        assert!(
            result
                .preview
                .as_ref()
                .unwrap()
                .points
                .iter()
                .all(|p| p.index != 100000)
        );
        let count = d.selection.len();
        execute(
            &mut d,
            Operation::Trim { keep: true },
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(d.editor.as_ref().unwrap().cloud.points.len(), count);
        assert_eq!(d.editor.as_ref().unwrap().cloud.points.last(), Some(&last));
        execute(&mut d, Operation::Undo, &WorkControl::default()).unwrap();
        assert_eq!(d.editor.as_ref().unwrap().cloud.points.len(), 100001);
        assert_eq!(d.editor.as_ref().unwrap().cloud.points[100000], last);
    }
    #[test]
    fn picking_preserves_full_resolution_indices_and_toggles() {
        let mut d = document(100);
        execute(
            &mut d,
            Operation::Pick {
                index: 20,
                additive: false,
            },
            &WorkControl::default(),
        )
        .unwrap();
        execute(
            &mut d,
            Operation::Pick {
                index: 40,
                additive: true,
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(d.selection, [20, 40]);
        execute(
            &mut d,
            Operation::Pick {
                index: 20,
                additive: true,
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(d.selection, [40]);
    }
    #[test]
    fn sequence_job_prepares_preview_before_publication_and_preserves_document_on_failure() {
        let temp = Temp::new();
        let cloud = Cloud {
            points: [[0.25, 0.25, 0.25], [10.25, 0.25, 0.25], [0.25, 20.25, 0.25]]
                .into_iter()
                .map(|position| Point {
                    position,
                    color: Some([10, 20, 30]),
                    normal: None,
                })
                .collect(),
        };
        let mut bytes = Vec::new();
        write_ply(&cloud, &mut bytes).unwrap();
        std::fs::write(temp.path.join("frame.ply"), bytes).unwrap();
        let settings = temp.path.join("sequence.json");
        let request = r#"{"schema_version":1,"session_id":"gui-test","source":"synthetic_fixture","voxel_mm":1.0,"maximum_voxels":10,
            "tracking":{"max_correspondence_mm":2.0,"max_rms_mm":0.01,"minimum_overlap":0.8,"max_step_translation_mm":1.0,"max_step_rotation_rad":0.1,"iterations":20,"tolerance_mm":1e-8},
            "frames":[{"id":0,"path":"frame.ply","tracking_indices":[0,1,2]}]}"#;
        std::fs::write(&settings, request.as_bytes()).unwrap();
        let mut doc = document(20);
        let before = snapshot(&doc);
        let selection = doc.selection.clone();
        let cancelled = WorkControl::default();
        cancelled.cancel();
        for (input, work) in [
            (settings.clone(), &cancelled),
            (temp.path.join("missing.json"), &WorkControl::default()),
        ] {
            assert!(
                execute(
                    &mut doc,
                    Operation::FuseSequence {
                        settings: input,
                        output: temp.path.join("failed")
                    },
                    work
                )
                .is_err()
            );
            assert_eq!(snapshot(&doc), before);
            assert_eq!(doc.selection, selection);
            assert!(!temp.path.join("failed").exists());
        }
        let output = temp.path.join("result");
        let report = execute(
            &mut doc,
            Operation::FuseSequence {
                settings: settings.clone(),
                output: output.clone(),
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert!(report.reset_view && report.project_saved);
        assert_eq!(report.preview.unwrap().total, 3);
        assert!(doc.selection.is_empty());
        let saved = load_project(File::open(output.join("project.orproj")).unwrap()).unwrap();
        assert_eq!(saved.cloud, doc.editor.as_ref().unwrap().cloud);
        assert_eq!(saved.cloud.points.len(), cloud.points.len());
        let before = snapshot(&doc);
        assert!(
            execute(
                &mut doc,
                Operation::FuseSequence { settings, output },
                &WorkControl::default()
            )
            .is_err()
        );
        assert_eq!(snapshot(&doc), before);
    }
}
