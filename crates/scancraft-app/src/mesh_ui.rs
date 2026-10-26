//! Saved-version mesh editing with cancellable preparation and atomic publication.
use eframe::egui;
use scancraft_geometry::{
    mesh::triangle_normal, selection::OrthographicProjection, work::WorkControl,
};
use scancraft_workflow::mesh_document::LoadedMesh;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread::{self, JoinHandle},
};
struct Facet {
    vertices: [[f64; 3]; 3],
    normal: [f64; 3],
}
struct Model {
    loaded: LoadedMesh,
    facets: Vec<Facet>,
    bounds: Option<([f64; 3], [f64; 3])>,
    source_path: PathBuf,
    snapshot_source: bool,
}
impl Model {
    fn build(loaded: LoadedMesh, work: &WorkControl) -> Result<Self, String> {
        let mut bounds = None;
        for p in &loaded.mesh.vertices {
            work.check()?;
            let (min, max) = bounds.get_or_insert((*p, *p));
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
        let stride = loaded.mesh.triangles.len().div_ceil(50_000).max(1);
        let mut facets = Vec::new();
        for t in loaded.mesh.triangles.iter().step_by(stride) {
            work.check()?;
            let vertices = t.map(|i| loaded.mesh.vertices[i as usize]);
            facets.push(Facet {
                normal: triangle_normal(vertices[0], vertices[1], vertices[2])?,
                vertices,
            });
        }
        work.check()?;
        Ok(Self {
            loaded,
            facets,
            bounds,
            source_path: PathBuf::new(),
            snapshot_source: false,
        })
    }
}
struct Proposal {
    prepared: Arc<scancraft_workflow::mesh_trim::PreparedTrim>,
    model: Model,
}
enum Operation {
    Import {
        input: PathBuf,
        target: PathBuf,
        config: scancraft_geometry::mesh_import::Config,
    },
    SmoothPreview {
        source: Arc<Model>,
        config: scancraft_geometry::mesh_smooth::Config,
    },
    SimplifyPreview {
        source: Arc<Model>,
        config: scancraft_geometry::mesh_simplify::Config,
    },
    SaveSection {
        source: Arc<Model>,
        origin: [f64; 3],
        normal: [f64; 3],
        tolerance: f64,
        target: PathBuf,
    },
    Section {
        source: Arc<Model>,
        origin: [f64; 3],
        normal: [f64; 3],
        tolerance: f64,
    },
    Open(PathBuf),
    AnalyzeComponents(Arc<Model>),
    AnalyzeBoundaries(Arc<Model>),
    SplitFansPreview {
        source: Arc<Model>,
        ids: Vec<u32>,
    },
    OrientationPreview {
        source: Arc<Model>,
        reverse: Vec<usize>,
    },
    FillPreview {
        source: Arc<Model>,
        id: usize,
        planarity_mm: f64,
    },
    ComponentPreview {
        source: Arc<Model>,
        ids: Vec<usize>,
        keep: bool,
    },
    SaveHistory {
        current: Arc<Model>,
        undo: Vec<Arc<Model>>,
        redo: Vec<Arc<Model>>,
        limit: usize,
        target: PathBuf,
    },
    CleanupPreview {
        source: Arc<Model>,
        options: scancraft_geometry::mesh_edit::CleanupOptions,
    },
    Preview {
        source: Arc<Model>,
        normal: [f64; 3],
        offset: f64,
    },
    Apply {
        proposal: Proposal,
        target: PathBuf,
    },
}
enum Outcome {
    Imported {
        loaded: Box<Outcome>,
        path: PathBuf,
    },
    LoadedSection {
        normal: [f64; 3],
        model: Model,
        section: Arc<scancraft_geometry::mesh_section::Section>,
    },
    SavedSection(PathBuf),
    SectionAnalysis(Arc<scancraft_geometry::mesh_section::Section>),
    Loaded(Model),
    ComponentAnalysis(Arc<scancraft_geometry::mesh_components::Components>),
    BoundaryAnalysis(Arc<scancraft_geometry::mesh_boundaries::Boundaries>),
    LoadedHistory {
        current: Arc<Model>,
        undo: VecDeque<Arc<Model>>,
        redo: Vec<Arc<Model>>,
        limit: usize,
    },
    SavedHistory(PathBuf),
    Preview(Proposal),
    Applied(Model),
}
fn perform(operation: Operation, work: &WorkControl) -> Result<Outcome, String> {
    match operation {
        Operation::Import {
            input,
            target,
            config,
        } => {
            scancraft_workflow::mesh_trim::import_mesh(&input, config, &target, work)
                .map_err(|e| e.to_string())?;
            let path = target.join("project.meshproj");
            let loaded = Box::new(perform(Operation::Open(path.clone()), work)?);
            Ok(Outcome::Imported { loaded, path })
        }
        Operation::SmoothPreview { source, config } => {
            let prepared = if source.snapshot_source {
                scancraft_workflow::mesh_trim::prepare_smooth_snapshot(
                    &source.loaded.mesh,
                    config,
                    work,
                )
            } else {
                scancraft_workflow::mesh_trim::prepare_smooth(&source.source_path, config, work)
            }
            .map_err(|e| e.to_string())?;
            if prepared.original_mesh() != &source.loaded.mesh {
                return Err("source changed since loading; reopen before smoothing".into());
            }
            let loaded = LoadedMesh {
                mesh: prepared.mesh().clone(),
                topology: prepared.mesh().topology(work)?,
            };
            Ok(Outcome::Preview(Proposal {
                prepared: Arc::new(prepared),
                model: Model::build(loaded, work)?,
            }))
        }

        Operation::SimplifyPreview { source, config } => {
            let prepared = if source.snapshot_source {
                scancraft_workflow::mesh_trim::prepare_simplify_snapshot(
                    &source.loaded.mesh,
                    config,
                    work,
                )
            } else {
                scancraft_workflow::mesh_trim::prepare_simplify(&source.source_path, config, work)
            }
            .map_err(|e| e.to_string())?;
            if prepared.original_mesh() != &source.loaded.mesh {
                return Err("source changed since loading; reopen before simplifying".into());
            }
            let loaded = LoadedMesh {
                mesh: prepared.mesh().clone(),
                topology: prepared.mesh().topology(work)?,
            };
            Ok(Outcome::Preview(Proposal {
                prepared: Arc::new(prepared),
                model: Model::build(loaded, work)?,
            }))
        }

        Operation::SaveSection {
            source,
            origin,
            normal,
            tolerance,
            target,
        } => {
            scancraft_workflow::section_document::save(
                &source.loaded.mesh,
                origin,
                normal,
                tolerance,
                &target,
                work,
            )
            .map_err(|e| e.to_string())?;
            Ok(Outcome::SavedSection(target))
        }

        Operation::Section {
            source,
            origin,
            normal,
            tolerance,
        } => Ok(Outcome::SectionAnalysis(Arc::new(
            scancraft_geometry::mesh_section::intersect(
                &source.loaded.mesh,
                origin,
                normal,
                tolerance,
                work,
            )?,
        ))),
        Operation::SplitFansPreview { source, ids } => {
            let prepared = if source.snapshot_source {
                scancraft_workflow::mesh_trim::prepare_split_fans_snapshot(
                    &source.loaded.mesh,
                    &ids,
                    work,
                )
            } else {
                scancraft_workflow::mesh_trim::prepare_split_fans(&source.source_path, &ids, work)
            }
            .map_err(|e| e.to_string())?;
            if prepared.original_mesh() != &source.loaded.mesh {
                return Err(
                    "source changed since loading; reopen before vertex-fan splitting".into(),
                );
            }
            let mesh = prepared.mesh().clone();
            let topology = mesh.topology(work)?;
            Ok(Outcome::Preview(Proposal {
                prepared: Arc::new(prepared),
                model: Model::build(LoadedMesh { mesh, topology }, work)?,
            }))
        }
        Operation::OrientationPreview { source, reverse } => {
            let prepared = if source.snapshot_source {
                scancraft_workflow::mesh_trim::prepare_orientation_snapshot(
                    &source.loaded.mesh,
                    &reverse,
                    work,
                )
            } else {
                scancraft_workflow::mesh_trim::prepare_orientation(
                    &source.source_path,
                    &reverse,
                    work,
                )
            }
            .map_err(|e| e.to_string())?;
            if prepared.original_mesh() != &source.loaded.mesh {
                return Err(
                    "source changed since loading; reopen before orientation repair".into(),
                );
            }
            let mesh = prepared.mesh().clone();
            let topology = mesh.topology(work)?;
            Ok(Outcome::Preview(Proposal {
                prepared: Arc::new(prepared),
                model: Model::build(LoadedMesh { mesh, topology }, work)?,
            }))
        }
        Operation::FillPreview {
            source,
            id,
            planarity_mm,
        } => {
            let prepared = if source.snapshot_source {
                scancraft_workflow::mesh_trim::prepare_fill_snapshot(
                    &source.loaded.mesh,
                    id,
                    planarity_mm,
                    work,
                )
            } else {
                scancraft_workflow::mesh_trim::prepare_fill(
                    &source.source_path,
                    id,
                    planarity_mm,
                    work,
                )
            }
            .map_err(|e| e.to_string())?;
            if prepared.original_mesh() != &source.loaded.mesh {
                return Err("source file changed since loading; reopen before filling".into());
            }
            let mesh = prepared.mesh().clone();
            let topology = mesh.topology(work)?;
            Ok(Outcome::Preview(Proposal {
                prepared: Arc::new(prepared),
                model: Model::build(LoadedMesh { mesh, topology }, work)?,
            }))
        }
        Operation::Open(path) => {
            if path.extension().is_some_and(|s| s == "secproj") {
                let loaded = scancraft_workflow::section_document::load(&path, work)
                    .map_err(|e| e.to_string())?;
                let topology = loaded.mesh.topology(work)?;
                let mut model = Model::build(
                    LoadedMesh {
                        mesh: loaded.mesh,
                        topology,
                    },
                    work,
                )?;
                model.source_path = path;
                model.snapshot_source = true;
                return Ok(Outcome::LoadedSection {
                    normal: loaded.normal,
                    model,
                    section: Arc::new(loaded.section),
                });
            }

            let (editor, is_project) = scancraft_workflow::mesh_history::load_input(&path, work)
                .map_err(|e| e.to_string())?;
            let (current, undo, redo, limit) = editor.into_history();
            let build = |mesh: scancraft_geometry::mesh::Mesh| -> Result<Arc<Model>, String> {
                let topology = mesh.topology(work)?;
                let mut model = Model::build(LoadedMesh { mesh, topology }, work)?;
                model.source_path = path.clone();
                model.snapshot_source = is_project;
                Ok(Arc::new(model))
            };
            let current = build(current)?;
            if !is_project {
                return Ok(Outcome::Loaded(Arc::try_unwrap(current).ok().unwrap()));
            }
            let undo: VecDeque<_> = undo.into_iter().map(&build).collect::<Result<_, _>>()?;
            let redo: Vec<_> = redo.into_iter().map(&build).collect::<Result<_, _>>()?;
            let bytes = |m: &Model| {
                m.loaded.mesh.vertices.len() * 24
                    + m.loaded.mesh.triangles.len() * 12
                    + m.facets.len() * std::mem::size_of::<Facet>()
            };
            if bytes(&current) + undo.iter().chain(&redo).map(|m| bytes(m)).sum::<usize>()
                > 128 * 1024 * 1024
            {
                return Err("project exceeds mesh viewport history payload budget".into());
            }
            Ok(Outcome::LoadedHistory {
                current,
                undo,
                redo,
                limit,
            })
        }
        Operation::SaveHistory {
            current,
            undo,
            redo,
            limit,
            target,
        } => {
            let undo = undo.into_iter().map(|m| m.loaded.mesh.clone()).collect();
            let redo = redo.into_iter().map(|m| m.loaded.mesh.clone()).collect();
            let editor = scancraft_geometry::mesh_edit::MeshEditor::from_history(
                current.loaded.mesh.clone(),
                undo,
                redo,
                limit,
                work,
            )?;
            scancraft_workflow::mesh_history::save_snapshot(&editor, &target, work)
                .map_err(|e| e.to_string())?;
            Ok(Outcome::SavedHistory(target))
        }
        Operation::AnalyzeBoundaries(source) => Ok(Outcome::BoundaryAnalysis(Arc::new(
            scancraft_geometry::mesh_boundaries::analyze(&source.loaded.mesh, work)?,
        ))),
        Operation::AnalyzeComponents(source) => Ok(Outcome::ComponentAnalysis(Arc::new(
            scancraft_geometry::mesh_components::analyze(&source.loaded.mesh, work)?,
        ))),
        Operation::ComponentPreview { source, ids, keep } => {
            let prepared = if source.snapshot_source {
                scancraft_workflow::mesh_trim::prepare_components_snapshot(
                    &source.loaded.mesh,
                    &ids,
                    keep,
                    work,
                )
            } else {
                scancraft_workflow::mesh_trim::prepare_components(
                    &source.source_path,
                    &ids,
                    keep,
                    work,
                )
            }
            .map_err(|e| e.to_string())?;
            if prepared.original_mesh() != &source.loaded.mesh {
                return Err(
                    "source file changed since loading; reopen it before component editing".into(),
                );
            }
            let loaded = LoadedMesh {
                mesh: prepared.mesh().clone(),
                topology: prepared.mesh().topology(work)?,
            };
            let model = Model::build(loaded, work)?;
            Ok(Outcome::Preview(Proposal {
                prepared: Arc::new(prepared),
                model,
            }))
        }
        Operation::CleanupPreview { source, options } => {
            let prepared = if source.snapshot_source {
                scancraft_workflow::mesh_trim::prepare_cleanup_snapshot(
                    &source.loaded.mesh,
                    options,
                    work,
                )
            } else {
                scancraft_workflow::mesh_trim::prepare_cleanup(&source.source_path, options, work)
            }
            .map_err(|e| e.to_string())?;
            if prepared.original_mesh() != &source.loaded.mesh {
                return Err("source file changed since loading; reopen it before cleanup".into());
            }
            let loaded = LoadedMesh {
                mesh: prepared.mesh().clone(),
                topology: prepared.mesh().topology(work)?,
            };
            let model = Model::build(loaded, work)?;
            Ok(Outcome::Preview(Proposal {
                prepared: Arc::new(prepared),
                model,
            }))
        }
        Operation::Preview {
            source,
            normal,
            offset,
        } => {
            let prepared = if source.snapshot_source {
                scancraft_workflow::mesh_trim::prepare_snapshot(
                    &source.loaded.mesh,
                    normal,
                    offset,
                    work,
                )
            } else {
                scancraft_workflow::mesh_trim::prepare(&source.source_path, normal, offset, work)
            }
            .map_err(|e| e.to_string())?;
            if prepared.original_mesh() != &source.loaded.mesh {
                return Err("source file changed since loading; reopen it before trimming".into());
            }
            let loaded = LoadedMesh {
                mesh: prepared.mesh().clone(),
                topology: prepared.mesh().topology(work)?,
            };
            let model = Model::build(loaded, work)?;
            Ok(Outcome::Preview(Proposal {
                prepared: Arc::new(prepared),
                model,
            }))
        }
        Operation::Apply {
            mut proposal,
            target,
        } => {
            scancraft_workflow::mesh_trim::publish_prepared(&proposal.prepared, &target, work)
                .map_err(|e| e.to_string())?;
            // Render state was prepared before publication. No fallible work follows.
            proposal.model.source_path = target.join("mesh.json");
            Ok(Outcome::Applied(proposal.model))
        }
    }
}
struct Job {
    control: Arc<WorkControl>,
    rx: Receiver<Result<Outcome, String>>,
    handle: Option<JoinHandle<()>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.control.cancel();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
pub struct MeshUi {
    import_panel_open: bool,
    import_path: String,
    import_folder: String,
    import_units: Option<scancraft_geometry::mesh_import::Units>,
    import_format: scancraft_geometry::mesh_import::Format,
    import_weld: bool,
    smoothing: scancraft_geometry::mesh_smooth::Config,
    simplification: scancraft_geometry::mesh_simplify::Config,
    smooth_after_open: bool,
    smooth_panel_open: bool,
    simplify_after_open: bool,
    simplify_panel_open: bool,
    section: Option<Arc<scancraft_geometry::mesh_section::Section>>,
    section_folder: String,
    section_origin: [f64; 3],
    section_normal: [f64; 3],
    section_tolerance: f64,
    section_after_open: Option<([f64; 3], [f64; 3], f64)>,
    show_section: bool,
    selected_section: usize,
    path: String,
    model: Option<Arc<Model>>,
    pending: Option<Proposal>,
    preview_after_open: Option<([f64; 3], f64)>,
    boundaries_after_open: bool,
    undo: VecDeque<Arc<Model>>,
    redo: Vec<Arc<Model>>,
    plane_normal: [f64; 3],
    plane_offset: f64,
    output_folder: String,
    project_folder: String,
    history_limit: usize,
    cleanup_duplicates: bool,
    cleanup_unused: bool,
    components: Option<Arc<scancraft_geometry::mesh_components::Components>>,
    boundaries: Option<Arc<scancraft_geometry::mesh_boundaries::Boundaries>>,
    selected_boundary: usize,
    show_boundary: bool,
    fill_planarity_mm: f64,
    reverse_components: String,
    split_vertex_ids: String,
    show_normals: bool,
    component_ids: String,
    component_keep: bool,
    job: Option<Job>,
    message: String,
    yaw: f64,
    pitch: f64,
    zoom: f64,
    pan: egui::Vec2,
    shaded: bool,
    wireframe: bool,
}
impl Default for MeshUi {
    fn default() -> Self {
        Self {import_panel_open:false,import_path:String::new(),import_folder:String::new(),import_units:None,import_format:scancraft_geometry::mesh_import::Format::Stl,import_weld:false,smooth_after_open:false,smooth_panel_open:false,smoothing:Default::default(),simplification:scancraft_geometry::mesh_simplify::Config {target_triangles:1000,maximum_displacement_mm:0.05,maximum_normal_change_degrees:10.,preserve_crease_degrees:30.,maximum_collapses:10000},simplify_after_open:false,simplify_panel_open:false,section_folder:String::new(),section:None,section_origin:[0.;3],section_normal:[0.,0.,1.],section_tolerance:0.,section_after_open:None,show_section:true,selected_section:0,path:String::new(),model:None,pending:None,preview_after_open:None,boundaries_after_open:false,undo:VecDeque::new(),redo:Vec::new(),plane_normal:[1.,0.,0.],plane_offset:0.,output_folder:String::new(),project_folder:String::new(),history_limit:32,cleanup_duplicates:false,cleanup_unused:false,components:None,boundaries:None,selected_boundary:0,show_boundary:true,fill_planarity_mm:0.01,reverse_components:String::new(),split_vertex_ids:String::new(),show_normals:false,component_ids:String::new(),component_keep:true,job:None,message:"Open an exported mesh.json to inspect its surface. Physical accuracy remains unqualified.".into(),yaw:0.35,pitch:0.3,zoom:1.,pan:egui::Vec2::ZERO,shaded:true,wireframe:true}
    }
}
impl MeshUi {
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    pub fn open_external_import(
        &mut self,
        input: PathBuf,
        target: PathBuf,
        config: scancraft_geometry::mesh_import::Config,
    ) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        self.import_path = input.display().to_string();
        self.import_folder = target.display().to_string();
        self.import_format = config.format;
        self.import_units = Some(config.units);
        self.import_weld = config.weld_exact;
        self.import_panel_open = true;
        self.import_mesh();
    }
    fn import_mesh(&mut self) {
        if self.job.is_some()
            || self.pending.is_some()
            || self.import_path.trim().is_empty()
            || self.import_folder.trim().is_empty()
        {
            return;
        }
        let Some(units) = self.import_units else {
            self.message = "Choose the source mesh units before importing.".into();
            return;
        };
        self.start(Operation::Import {
            input: self.import_path.clone().into(),
            target: self.import_folder.clone().into(),
            config: scancraft_geometry::mesh_import::Config {
                format: self.import_format,
                units,
                weld_exact: self.import_weld,
            },
        });
    }
    pub fn open_smooth_preview(
        &mut self,
        path: PathBuf,
        config: scancraft_geometry::mesh_smooth::Config,
    ) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        self.smoothing = config;
        self.smooth_after_open = true;
        self.smooth_panel_open = true;
        self.open(path);
    }
    pub fn open_simplify_preview(
        &mut self,
        path: PathBuf,
        config: scancraft_geometry::mesh_simplify::Config,
    ) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        self.simplification = config;
        self.simplify_after_open = true;
        self.simplify_panel_open = true;
        self.open(path);
    }
    fn preview_smooth(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        if let Some(source) = self.model.clone() {
            self.start(Operation::SmoothPreview {
                source,
                config: self.smoothing.clone(),
            });
        }
    }

    fn preview_simplify(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        if let Some(source) = self.model.clone() {
            self.start(Operation::SimplifyPreview {
                source,
                config: self.simplification.clone(),
            });
        }
    }

    pub fn open_with_section(
        &mut self,
        path: PathBuf,
        origin: [f64; 3],
        normal: [f64; 3],
        tolerance: f64,
    ) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        self.section_after_open = Some((origin, normal, tolerance));
        self.open(path);
    }
    fn analyze_section(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        if let Some(source) = self.model.clone() {
            self.section = None;
            self.start(Operation::Section {
                source,
                origin: self.section_origin,
                normal: self.section_normal,
                tolerance: self.section_tolerance,
            });
        }
    }

    pub fn open_with_normals(&mut self, path: PathBuf) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        self.show_normals = true;
        self.open(path);
    }
    pub fn open_with_boundary_analysis(&mut self, path: PathBuf) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        self.boundaries_after_open = true;
        self.open(path);
    }
    pub fn open_plane_preview(&mut self, path: PathBuf, normal: [f64; 3], offset: f64) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        self.preview_after_open = Some((normal, offset));
        self.open(path);
    }
    pub fn open(&mut self, path: PathBuf) {
        if self.pending.is_some() {
            self.message = "Discard the mesh edit preview before opening another mesh.".into();
            return;
        }
        self.path = path.display().to_string();
        self.start(Operation::Open(path));
    }
    fn start(&mut self, operation: Operation) {
        if self.job.is_some() {
            return;
        }
        let control = Arc::new(WorkControl::default());
        let worker = control.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let handle = thread::Builder::new()
            .name("scancraft-mesh".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    perform(operation, &worker)
                }))
                .unwrap_or_else(|_| Err("mesh worker panicked; previous model retained".into()));
                let _ = tx.send(result);
            });
        match handle {
            Ok(handle) => {
                self.job = Some(Job {
                    control,
                    rx,
                    handle: Some(handle),
                });
                self.message = "Processing mesh…".into();
            }
            Err(e) => self.message = format!("Could not start mesh worker: {e}"),
        }
    }
    fn analyze_components(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        if let Some(model) = self.model.clone() {
            self.start(Operation::AnalyzeComponents(model));
        }
    }
    fn analyze_boundaries(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        if let Some(model) = self.model.clone() {
            self.start(Operation::AnalyzeBoundaries(model));
        }
    }
    fn preview_orientation(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        let ids = if self.reverse_components.trim().is_empty() {
            Ok(Vec::new())
        } else {
            self.reverse_components
                .split(',')
                .map(|s| s.trim().parse::<usize>())
                .collect::<Result<Vec<_>, _>>()
        };
        match ids {
            Ok(reverse)=>if let Some(source)=self.model.clone() {self.show_normals=true;self.start(Operation::OrientationPreview {source,reverse});},
            Err(_)=>self.message="Enter comma-separated component IDs to reverse, or leave empty to preserve each component's first face.".into(),
        }
    }
    fn preview_split_fans(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        match self
            .split_vertex_ids
            .split(',')
            .map(|s| s.trim().parse::<u32>())
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(ids) => {
                if let Some(source) = self.model.clone() {
                    self.start(Operation::SplitFansPreview { source, ids });
                }
            }
            Err(_) => {
                self.message =
                    "Enter explicit comma-separated vertex IDs from topology diagnostics.".into()
            }
        }
    }
    fn preview_fill(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        if let Some(analysis) = &self.boundaries
            && let Some(contour) = analysis.loops.get(self.selected_boundary)
            && let Some(source) = self.model.clone()
        {
            self.start(Operation::FillPreview {
                source,
                id: contour.id,
                planarity_mm: self.fill_planarity_mm,
            });
        }
    }
    fn preview_components(&mut self) {
        if self.job.is_some() || self.pending.is_some() || self.components.is_none() {
            return;
        }
        let ids = self
            .component_ids
            .split(',')
            .map(|s| s.trim().parse::<usize>())
            .collect::<Result<Vec<_>, _>>();
        match ids {
            Ok(ids) => {
                if let Some(source) = self.model.clone() {
                    self.start(Operation::ComponentPreview {
                        source,
                        ids,
                        keep: self.component_keep,
                    });
                }
            }
            Err(_) => {
                self.message =
                    "Enter explicit comma-separated component IDs from the analysis.".into()
            }
        }
    }
    fn preview_cleanup(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        if let Some(source) = self.model.clone() {
            self.start(Operation::CleanupPreview {
                source,
                options: scancraft_geometry::mesh_edit::CleanupOptions {
                    duplicate_faces: self.cleanup_duplicates,
                    unused_vertices: self.cleanup_unused,
                },
            });
        }
    }
    fn preview_trim(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        if let Some(source) = self.model.clone() {
            self.start(Operation::Preview {
                source,
                normal: self.plane_normal,
                offset: self.plane_offset,
            });
        }
    }
    fn apply_trim(&mut self) {
        if self.job.is_some() || self.output_folder.trim().is_empty() {
            return;
        }
        if let Some(proposal) = self.pending.take() {
            self.start(Operation::Apply {
                proposal,
                target: self.output_folder.clone().into(),
            });
        }
    }
    fn undo(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        if let Some(previous) = self.undo.pop_back() {
            self.components = None;
            self.boundaries = None;
            self.section = None;
            self.selected_boundary = 0;
            self.reverse_components.clear();
            self.split_vertex_ids.clear();
            if let Some(current) = self.model.replace(previous) {
                self.redo.push(current);
            }
            self.path = self
                .model
                .as_ref()
                .unwrap()
                .source_path
                .display()
                .to_string();
            self.message = "Restored previous saved mesh version. Files are preserved.".into();
        }
    }
    fn redo(&mut self) {
        if self.job.is_some() || self.pending.is_some() {
            return;
        }
        if let Some(next) = self.redo.pop() {
            self.components = None;
            self.boundaries = None;
            self.section = None;
            self.selected_boundary = 0;
            self.reverse_components.clear();
            self.split_vertex_ids.clear();
            if let Some(current) = self.model.replace(next) {
                self.undo.push_back(current);
            }
            self.path = self
                .model
                .as_ref()
                .unwrap()
                .source_path
                .display()
                .to_string();
            self.message = "Restored next saved mesh version. Files are preserved.".into();
        }
    }
    fn save_history(&mut self) {
        if self.job.is_some() || self.pending.is_some() || self.project_folder.trim().is_empty() {
            return;
        }
        if let Some(current) = self.model.clone() {
            self.start(Operation::SaveHistory {
                current,
                undo: self.undo.iter().cloned().collect(),
                redo: self.redo.clone(),
                limit: self.history_limit,
                target: self.project_folder.clone().into(),
            });
        }
    }
    fn accept(&mut self, outcome: Outcome) {
        if matches!(
            &outcome,
            Outcome::Loaded(_) | Outcome::LoadedHistory { .. } | Outcome::Applied(_)
        ) {
            self.components = None;
            self.boundaries = None;
            self.section = None;
            self.selected_boundary = 0;
            self.reverse_components.clear();
            self.split_vertex_ids.clear();
        }
        match outcome {
            Outcome::Imported { loaded, path } => {
                self.accept(*loaded);
                self.path = path.display().to_string();
                self.message="Mesh imported and original file preserved in the new bundle. Units converted to mm; face normals recomputed. Physical accuracy remains unqualified.".into();
            }

            Outcome::LoadedSection {
                model,
                section,
                normal,
            } => {
                let requested = self.section_after_open.take();
                self.accept(Outcome::Loaded(model));
                self.section_origin = section.plane_origin_mm;
                self.section_normal = normal;
                self.section_tolerance = section.tolerance_mm;
                self.accept(Outcome::SectionAnalysis(section));
                self.message="Section snapshot reopened: exact source mesh and plane restored; graph recomputed and totals checked. Physical accuracy unqualified.".into();
                if let Some((origin, normal, tolerance)) = requested {
                    self.section_origin = origin;
                    self.section_normal = normal;
                    self.section_tolerance = tolerance;
                    self.analyze_section();
                }
            }
            Outcome::SavedSection(path) => {
                self.message = format!(
                    "Section snapshot saved at {}. Reopen section.secproj with its source.meshproj sibling. Current mesh/history unchanged.",
                    path.display()
                );
            }
            Outcome::SectionAnalysis(section) => {
                self.message = format!(
                    "Section: {} segments, {} ordered contours, {} ambiguous graphs; total segment length {:.6} mm. Transient analysis; mesh unchanged. Physical accuracy unqualified.",
                    section.segments.len(),
                    section.contours.len(),
                    section.ambiguous_components.len(),
                    section.total_segment_length_mm
                );
                self.section = Some(section);
                self.selected_section = 0;
            }
            Outcome::BoundaryAnalysis(boundaries) => {
                self.message = format!(
                    "{} simple boundary contours; {} ambiguous boundary graphs. Openings are preserved.",
                    boundaries.loops.len(),
                    boundaries.ambiguous_components.len()
                );
                self.selected_boundary = 0;
                self.boundaries = Some(boundaries);
            }
            Outcome::ComponentAnalysis(components) => {
                self.message = format!(
                    "{} edge-connected components. IDs follow first-face order; choose parts explicitly.",
                    components.components.len()
                );
                self.components = Some(components);
            }
            Outcome::SavedHistory(path) => {
                self.message = format!("Full mesh undo/redo history saved in {}", path.display())
            }
            Outcome::LoadedHistory {
                current,
                undo,
                redo,
                limit,
            } => {
                self.model = Some(current);
                self.undo = undo;
                self.redo = redo;
                self.history_limit = limit;
                self.pending = None;
                self.zoom = 1.;
                self.pan = egui::Vec2::ZERO;
                self.message = format!(
                    "Mesh history reopened: {} undo / {} redo steps. Snapshots are self-contained.",
                    self.undo.len(),
                    self.redo.len()
                );
                if let Some((normal, offset)) = self.preview_after_open.take() {
                    self.plane_normal = normal;
                    self.plane_offset = offset;
                    self.preview_trim();
                }
                if let Some((origin, normal, tolerance)) = self.section_after_open.take() {
                    self.section_origin = origin;
                    self.section_normal = normal;
                    self.section_tolerance = tolerance;
                    self.analyze_section();
                }
                if std::mem::take(&mut self.smooth_after_open) {
                    self.preview_smooth();
                }
                if std::mem::take(&mut self.simplify_after_open) {
                    self.preview_simplify();
                }
                if std::mem::take(&mut self.boundaries_after_open) {
                    self.analyze_boundaries();
                }
            }
            Outcome::Loaded(model) => {
                self.history_limit = 32;
                self.model = Some(Arc::new(model));
                self.undo.clear();
                self.redo.clear();
                self.pending = None;
                self.message =
                    "Mesh loaded. Physical accuracy and watertightness remain unqualified.".into();
                self.zoom = 1.;
                self.pan = egui::Vec2::ZERO;
                if let Some((normal, offset)) = self.preview_after_open.take() {
                    self.plane_normal = normal;
                    self.plane_offset = offset;
                    self.preview_trim();
                }
                if let Some((origin, normal, tolerance)) = self.section_after_open.take() {
                    self.section_origin = origin;
                    self.section_normal = normal;
                    self.section_tolerance = tolerance;
                    self.analyze_section();
                }
                if std::mem::take(&mut self.smooth_after_open) {
                    self.preview_smooth();
                }
                if std::mem::take(&mut self.simplify_after_open) {
                    self.preview_simplify();
                }
                if std::mem::take(&mut self.boundaries_after_open) {
                    self.analyze_boundaries();
                }
            }
            Outcome::Preview(proposal) => {
                let report = proposal.prepared.report();
                self.message = if report["operation"] == "simplify" {
                    let d = &report["parameters"]["decimation"];
                    let status = match d["stopped_reason"].as_str() {
                        Some("target_met") => "requested count reached",
                        Some("constraints") => "constraints prevented the target",
                        Some("collapse_budget") => "collapse budget reached",
                        Some("candidate_budget") => "candidate budget reached",
                        Some("queue_budget") => "queue budget reached",
                        _ => "stopped",
                    };
                    format!(
                        "Unapplied simplification: {} -> {} triangles; {} collapses; maximum source-vertex movement {} mm; {}. Review before Apply and save edit, or discard. Global self-intersections and physical accuracy remain unqualified.",
                        d["original_triangles"],
                        d["result_triangles"],
                        d["collapsed_edges"],
                        d["maximum_source_vertex_displacement_mm"],
                        status
                    )
                } else if report["operation"] == "smooth" {
                    let d = &report["parameters"]["smoothing"];
                    let status = match d["stopped_reason"].as_str() {
                        Some("iterations_completed") => "requested iterations completed",
                        Some("stationary") => "no further vertex movement",
                        Some("constraints") => "geometric limits prevented further smoothing",
                        _ => "stopped",
                    };
                    format!(
                        "Unapplied smoothing: {} iterations; {} backtracking steps; maximum source-vertex movement {} mm; stop: {}. Review before Apply and save edit, or discard. Volume, self-intersections and physical accuracy remain unqualified.",
                        d["completed_iterations"],
                        d["backtracking_steps"],
                        d["maximum_source_vertex_displacement_mm"],
                        status
                    )
                } else if report["operation"] == "split_vertex_fans" {
                    format!(
                        "Unapplied vertex-fan split: {} coordinate copies added. Face positions stay identical; no physical gap is created. Review, then apply/save or discard.",
                        report["parameters"]["vertices_added"]
                    )
                } else if report["operation"] == "orient_faces" {
                    format!(
                        "Unapplied orientation preview: {} face windings changed. Outward direction is not inferred. Review normals, then apply/save or discard.",
                        report["parameters"]["faces_flipped"]
                    )
                } else if report["operation"] == "boundary_fill" {
                    format!(
                        "Unapplied fill preview: {} triangles added to boundary {}. Vertices stay fixed. Review before applying and saving.",
                        report["parameters"]["added_faces"], report["parameters"]["boundary_id"]
                    )
                } else if report["operation"] == "component_selection" {
                    format!(
                        "Unapplied component preview: {} triangles retained; {} faces removed. Review before applying the saved edit.",
                        proposal.model.loaded.mesh.triangles.len(),
                        report["removed_faces"]
                    )
                } else if report["operation"] == "cleanup" {
                    format!(
                        "Unapplied cleanup preview: {} triangles; removed {} duplicate faces and {} unused vertices. Review then apply/save or discard.",
                        proposal.model.loaded.mesh.triangles.len(),
                        report["removed_faces"],
                        report["removed_vertices"]
                    )
                } else {
                    format!(
                        "Unapplied trim preview: {} triangles, {} cut edges, {} rejected slivers. Apply saves a new bundle; discard retains the current version.",
                        proposal.model.loaded.mesh.triangles.len(),
                        report["cut_edges"],
                        report["rejected_slivers"]
                    )
                };
                self.pending = Some(proposal);
            }
            Outcome::Applied(model) => {
                if let Some(previous) = self.model.replace(Arc::new(model)) {
                    self.undo.push_back(previous);
                }
                self.redo.clear();
                let bytes = |m: &Model| {
                    m.loaded.mesh.vertices.len() * 24
                        + m.loaded.mesh.triangles.len() * 12
                        + m.facets.len() * std::mem::size_of::<Facet>()
                };
                let mut retained = self.model.as_ref().map_or(0, |m| bytes(m))
                    + self.undo.iter().map(|m| bytes(m)).sum::<usize>();
                let mut evicted = 0;
                while self.undo.len() > self.history_limit || retained > 128 * 1024 * 1024 {
                    let Some(old) = self.undo.pop_front() else {
                        break;
                    };
                    retained -= bytes(&old);
                    evicted += 1;
                }
                self.path = self
                    .model
                    .as_ref()
                    .unwrap()
                    .source_path
                    .display()
                    .to_string();
                self.message = if evicted == 0 {
                    "Mesh edit applied and saved. Source and prior versions retained. Undo is available."
                        .into()
                } else {
                    format!(
                        "Mesh edit applied and saved. Source and prior versions retained. {evicted} old undo steps evicted by history limits."
                    )
                };
            }
        }
    }
    pub fn tick(&mut self) {
        let Some(job) = &self.job else {
            return;
        };
        let result = match job.rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("mesh worker stopped unexpectedly".into()),
        };
        self.job = None;
        match result {
            Ok(outcome) => self.accept(outcome),
            Err(e) => {
                self.preview_after_open = None;
                self.smooth_after_open = false;
                self.simplify_after_open = false;
                self.boundaries_after_open = false;
                self.section_after_open = None;
                self.message =
                    format!("Mesh operation failed or cancelled: {e}; previous mesh retained")
            }
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.tick();
        ui.heading("Mesh workbench");
        ui.label("Mesh edits save new versions before changing the current mesh. Save a history bundle to reopen undo/redo. Physical qualification remains unfinished.");
        ui.horizontal(|ui| {
            ui.label("Mesh JSON/project:");
            ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(650.));
            if ui
                .add_enabled(
                    self.job.is_none() && self.pending.is_none() && !self.path.trim().is_empty(),
                    egui::Button::new("Open mesh"),
                )
                .clicked()
            {
                self.open(self.path.clone().into());
            }
        });
        egui::CollapsingHeader::new("Import STL / OBJ").default_open(self.import_panel_open).show(ui,|ui| {
            ui.small("Triangle OBJ and ASCII/binary STL. Choose source units explicitly. Exact welding can join coincident parts; separate preserves indexed identities. Unsupported attributes are rejected; normals are recomputed. Original bytes are retained.");
            ui.add_enabled_ui(self.job.is_none()&&self.pending.is_none(),|ui| {
                ui.horizontal(|ui| {ui.label("Source file:");ui.text_edit_singleline(&mut self.import_path);});
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.import_format,scancraft_geometry::mesh_import::Format::Stl,"STL");ui.selectable_value(&mut self.import_format,scancraft_geometry::mesh_import::Format::Obj,"OBJ");
                    ui.label("Source units:");
                    for (units,label) in [(scancraft_geometry::mesh_import::Units::Millimeters,"mm"),(scancraft_geometry::mesh_import::Units::Inches,"in"),(scancraft_geometry::mesh_import::Units::Meters,"m")] {ui.selectable_value(&mut self.import_units,Some(units),label);}
                    ui.checkbox(&mut self.import_weld,"Weld exactly equal coordinates");
                });
                ui.horizontal(|ui| {ui.label("New import bundle:");ui.text_edit_singleline(&mut self.import_folder);if ui.add_enabled(self.import_units.is_some()&&!self.import_path.trim().is_empty()&&!self.import_folder.trim().is_empty(),egui::Button::new("Import and open")).clicked() {self.import_mesh();}});
            });
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.shaded, "Shaded");
            ui.checkbox(&mut self.wireframe, "Wireframe");
            ui.checkbox(
                &mut self.show_normals,
                "Face normals (sampled, through surfaces)",
            ).on_hover_text("At most 2,000 face normals from the sampled preview; arrows do not modify geometry.");
            if ui.button("Fit view").clicked() {
                self.zoom = 1.;
                self.pan = egui::Vec2::ZERO;
            }
        });
        egui::CollapsingHeader::new("Mesh cross section").id_salt(("mesh-section",self.section.is_some())).default_open(self.section.is_some()).show(ui,|ui| {
            ui.label("Read-only plane section. Origin/tolerance in mm. Coplanar faces are omitted; branching graphs stay ambiguous. Save a snapshot bundle to retain the source and settings.");
            let mut changed=false;
            ui.add_enabled_ui(self.job.is_none(),|ui| {ui.horizontal_wrapped(|ui| {
                ui.label("Origin:");for value in &mut self.section_origin {changed|=ui.add(egui::DragValue::new(value).speed(0.1)).changed();}
                ui.label("Normal:");for value in &mut self.section_normal {changed|=ui.add(egui::DragValue::new(value).speed(0.01)).changed();}
                ui.label("Tolerance (mm):");changed|=ui.add(egui::DragValue::new(&mut self.section_tolerance).range(0. ..=100000.).speed(0.001)).changed();
            });});
            if changed {self.section=None;}
            ui.horizontal(|ui| {
                if ui.add_enabled(self.model.is_some()&&self.job.is_none()&&self.pending.is_none(),egui::Button::new("Compute section")).clicked() {self.analyze_section();}
                ui.checkbox(&mut self.show_section,"Show section graph (through surfaces)");
            });
            if let Some(section)=self.section.clone() {
                if self.pending.is_some() {ui.small("Section belongs to the saved mesh and is hidden during the edit preview.");}
                let closed=section.contours.iter().filter(|c|c.closed).count();
                ui.label(format!("{} segments · {} closed / {} open contours · {} ambiguous graphs · total length {:.6} mm",section.segments.len(),closed,section.contours.len()-closed,section.ambiguous_components.len(),section.total_segment_length_mm));
                ui.small(format!("Coplanar triangles omitted: {} · tangent contacts: {} · duplicate segments: {}. At most 10,000 sampled segments drawn; lengths use the full section.",section.coplanar_triangles,section.tangent_triangles,section.duplicate_segments));
                ui.horizontal(|ui| {ui.label("New section snapshot folder:");ui.text_edit_singleline(&mut self.section_folder);
                    if ui.add_enabled(self.job.is_none()&&self.pending.is_none()&&!self.section_folder.trim().is_empty(),egui::Button::new("Save section snapshot")).clicked() {
                        let source=self.model.as_ref().unwrap().clone();let target=PathBuf::from(&self.section_folder);self.start(Operation::SaveSection {source,origin:self.section_origin,normal:self.section_normal,tolerance:self.section_tolerance,target});
                    }
                });
                if !section.contours.is_empty() {ui.horizontal(|ui| {ui.label("Contour index:");ui.add(egui::DragValue::new(&mut self.selected_section).range(0..=section.contours.len()-1));let contour=&section.contours[self.selected_section.min(section.contours.len()-1)];ui.label(format!("{} · {} nodes · length {:.6} mm",if contour.closed {"Closed"} else {"Open"},contour.nodes.len(),contour.length_mm));});}
            }
        });
        egui::CollapsingHeader::new("Mesh smoothing").default_open(self.smooth_panel_open).show(ui,|ui| {
            ui.small("Uniform Laplacian smoothing can shrink surfaces. Boundary and source crease vertices stay fixed. Movement and normal limits apply against the source. No volume or global intersection guarantee.");
            ui.add_enabled_ui(self.job.is_none()&&self.pending.is_none(),|ui| {ui.horizontal_wrapped(|ui| {
                ui.label("Iterations:");ui.add(egui::DragValue::new(&mut self.smoothing.iterations).clamp_existing_to_range(false).range(1..=100));
                ui.label("Relaxation:");ui.add(egui::DragValue::new(&mut self.smoothing.relaxation).clamp_existing_to_range(false).range(0.001..=1.).speed(0.01));
                ui.label("Max source movement (mm):");ui.add(egui::DragValue::new(&mut self.smoothing.maximum_displacement_mm).clamp_existing_to_range(false).range(0. ..=100000.).speed(0.01));
                ui.label("Max normal change (deg):");ui.add(egui::DragValue::new(&mut self.smoothing.maximum_normal_change_degrees).clamp_existing_to_range(false).range(0. ..=89.));
                ui.label("Protect creases over (deg):");ui.add(egui::DragValue::new(&mut self.smoothing.preserve_crease_degrees).clamp_existing_to_range(false).range(0. ..=180.));
                if ui.add_enabled(self.model.is_some(),egui::Button::new("Preview smoothing")).clicked() {self.preview_smooth();}
            });});
        });
        egui::CollapsingHeader::new("Mesh simplification").default_open(self.simplify_panel_open).show(ui,|ui| {
            ui.small("Boundary and source crease vertices stay fixed. Movement is bounded against every original cluster vertex; normal change is bounded against surviving source faces. Targets can be unreachable. Current limit: 100,000 vertices / 200,000 triangles.");
            ui.add_enabled_ui(self.job.is_none()&&self.pending.is_none(),|ui| {ui.horizontal_wrapped(|ui| {
                ui.label("Target triangles:");ui.add(egui::DragValue::new(&mut self.simplification.target_triangles).clamp_existing_to_range(false).range(1..=200000));
                ui.label("Max source movement (mm):");ui.add(egui::DragValue::new(&mut self.simplification.maximum_displacement_mm).clamp_existing_to_range(false).range(0. ..=100000.).speed(0.01));
                ui.label("Max normal change (deg):");ui.add(egui::DragValue::new(&mut self.simplification.maximum_normal_change_degrees).clamp_existing_to_range(false).range(0. ..=89.));
                ui.label("Protect creases over (deg):");ui.add(egui::DragValue::new(&mut self.simplification.preserve_crease_degrees).clamp_existing_to_range(false).range(0. ..=180.));
                ui.label("Collapse budget:");ui.add(egui::DragValue::new(&mut self.simplification.maximum_collapses).clamp_existing_to_range(false).range(1..=100000));
                if ui.add_enabled(self.model.is_some(),egui::Button::new("Preview simplification")).clicked() {self.preview_simplify();}
            });});ui.small("Inspect the preview, then Apply and save edit or Discard preview in Mesh edits below. No global self-intersection or physical qualification.");
        });
        egui::CollapsingHeader::new("Mesh edits")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(
                    "Keep normal · position + offset ≤ 0. Cuts remain open; no caps are added.",
                );
                ui.add_enabled_ui(self.job.is_none() && self.pending.is_none(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Normal:");
                        for value in &mut self.plane_normal {
                            ui.add(egui::DragValue::new(value).speed(0.01));
                        }
                        ui.label("Offset:");
                        ui.add(egui::DragValue::new(&mut self.plane_offset).speed(0.1));
                        if ui
                            .add_enabled(self.model.is_some(), egui::Button::new("Preview trim"))
                            .clicked()
                        {
                            self.preview_trim();
                        }
                    });
                });
                ui.add_enabled_ui(self.job.is_none()&&self.pending.is_none(),|ui|{
                    ui.horizontal(|ui|{ui.checkbox(&mut self.cleanup_duplicates,"Remove duplicate faces");ui.checkbox(&mut self.cleanup_unused,"Remove unused vertices");if ui.add_enabled(self.model.is_some()&&(self.cleanup_duplicates||self.cleanup_unused),egui::Button::new("Preview cleanup")).clicked(){self.preview_cleanup();}});
                    ui.horizontal(|ui|{ui.label("Reverse component IDs (optional):");ui.text_edit_singleline(&mut self.reverse_components);if ui.add_enabled(self.model.is_some(),egui::Button::new("Preview consistent winding")).clicked(){self.preview_orientation();}});
                    ui.small("Default keeps each component's first-face sign. Component IDs come from component analysis; outward direction requires your choice.");
                    ui.horizontal(|ui|{ui.label("Vertex IDs to split:");ui.text_edit_singleline(&mut self.split_vertex_ids);if ui.add_enabled(self.model.is_some()&&!self.split_vertex_ids.trim().is_empty(),egui::Button::new("Preview disconnected-fan split")).clicked(){self.preview_split_fans();}});
                    ui.small("Explicit disconnected-fan vertices only. Coordinate copies create separate indices, not a geometric gap. Other defects remain.");
                    ui.small("Cleanup uses indexed face identity, ignores winding, and keeps the first face. It does not weld vertices or fill openings.");
                });
                ui.horizontal(|ui| {
                    ui.label("New result folder:");
                    ui.add(egui::TextEdit::singleline(&mut self.output_folder).desired_width(550.));
                });
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.job.is_none()
                                && self.pending.is_some()
                                && !self.output_folder.trim().is_empty(),
                            egui::Button::new("Apply and save edit"),
                        )
                        .clicked()
                    {
                        self.apply_trim();
                    }
                    if ui
                        .add_enabled(
                            self.job.is_none() && self.pending.is_some(),
                            egui::Button::new("Discard preview"),
                        )
                        .clicked()
                    {
                        self.pending = None;
                        self.message = "Mesh edit preview discarded; current mesh unchanged.".into();
                    }
                    if ui
                        .add_enabled(
                            self.job.is_none() && self.pending.is_none() && !self.undo.is_empty(),
                            egui::Button::new("Undo mesh edit"),
                        )
                        .clicked()
                    {
                        self.undo();
                    }
                    if ui
                        .add_enabled(
                            self.job.is_none() && self.pending.is_none() && !self.redo.is_empty(),
                            egui::Button::new("Redo mesh edit"),
                        )
                        .clicked()
                    {
                        self.redo();
                    }
                });
            });
        egui::CollapsingHeader::new("Disconnected mesh components").show(ui,|ui|{
            ui.small("Connectivity uses shared indexed edges. Vertex-only contact and coincident unshared vertices stay separate.");
            ui.add_enabled_ui(self.job.is_none()&&self.pending.is_none(),|ui|{
                if ui.add_enabled(self.model.is_some(),egui::Button::new("Analyze components")).clicked(){self.analyze_components();}
                if let Some(analysis)=&self.components {
                    ui.label(format!("{} components. No size-based removal is automatic.",analysis.components.len()));
                    egui::ScrollArea::vertical().id_salt("mesh-components").max_height(110.).show_rows(ui,ui.text_style_height(&egui::TextStyle::Body),analysis.components.len(),|ui,range|{for i in range{let c=&analysis.components[i];let text=format!("ID {} · {} triangles · min {:?} mm · max {:?} mm",c.id,c.triangles,c.minimum_mm,c.maximum_mm);ui.add(egui::Label::new(&text).truncate()).on_hover_text(text);}});
                }
                ui.horizontal(|ui|{ui.label("IDs:");ui.text_edit_singleline(&mut self.component_ids);ui.selectable_value(&mut self.component_keep,true,"Keep IDs");ui.selectable_value(&mut self.component_keep,false,"Remove IDs");if ui.add_enabled(self.components.is_some()&&!self.component_ids.trim().is_empty(),egui::Button::new("Preview component edit")).clicked(){self.preview_components();}});
            });
            ui.small("Review the preview, then use Apply and save edit in Mesh edits above. Undo restores exact originals.");
        });
        egui::CollapsingHeader::new("Boundary contours").default_open(true).show(ui,|ui|{
            ui.add_enabled_ui(self.job.is_none()&&self.pending.is_none(),|ui|{
                if ui.add_enabled(self.model.is_some(),egui::Button::new("Analyze boundaries")).clicked(){self.analyze_boundaries();}
                if let Some(analysis)=self.boundaries.clone() {
                    ui.label(format!("{} contours · {} ambiguous graphs · {} boundary edges",analysis.loops.len(),analysis.ambiguous_components.len(),analysis.boundary_edges));
                    egui::ScrollArea::vertical().id_salt("mesh-boundaries").max_height(90.).show_rows(ui,ui.text_style_height(&egui::TextStyle::Body),analysis.loops.len(),|ui,range|{for i in range {
                        let contour=&analysis.loops[i];
                        if ui.selectable_label(self.selected_boundary==i,format!("ID {} · {} edges · {:.3} mm perimeter",contour.id,contour.vertices.len(),contour.perimeter_mm)).on_hover_text(format!("Bounds {:?} to {:?} mm",contour.minimum_mm,contour.maximum_mm)).clicked(){self.selected_boundary=i;}
                    }});
                    ui.checkbox(&mut self.show_boundary,"Highlight selected contour through surfaces");
                    ui.horizontal(|ui|{ui.label("Planarity tolerance (mm):");ui.add(egui::DragValue::new(&mut self.fill_planarity_mm).speed(0.001).range(0.0..=1.0));if ui.add_enabled(!analysis.loops.is_empty(),egui::Button::new("Preview fill of selected contour")).clicked(){self.preview_fill();}});
                }
            });
            ui.small("Contours can be intentional openings. Fill only an explicitly selected planar contour; review then Apply and save edit above. Highlight shows the saved version through surfaces, sampling at most 10,000 edges.");
        });
        ui.horizontal(|ui| {
            ui.label("New history bundle:");
            ui.add(egui::TextEdit::singleline(&mut self.project_folder).desired_width(480.));
            if ui
                .add_enabled(
                    self.job.is_none()
                        && self.pending.is_none()
                        && self.model.is_some()
                        && !self.project_folder.trim().is_empty(),
                    egui::Button::new("Save full mesh history"),
                )
                .clicked()
            {
                self.save_history();
            }
        });
        ui.small(format!(
            "Mesh history: {} undo / {} redo steps. Opening a project restores these snapshots.",
            self.undo.len(),
            self.redo.len()
        ));
        if let Some(job) = &self.job {
            let (stage, fraction) = job.control.progress();
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(stage);
                ui.add(egui::ProgressBar::new(fraction).desired_width(160.));
                if ui.button("Cancel operation").clicked() {
                    job.control.cancel();
                }
            });
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(30));
        }
        if self.pending.is_some() {
            ui.colored_label(
                egui::Color32::YELLOW,
                "UNAPPLIED MESH EDIT PREVIEW · current saved version is unchanged",
            );
        }
        ui.label(&self.message);
        if let Some(model) = self
            .pending
            .as_ref()
            .map(|p| &p.model)
            .or(self.model.as_deref())
        {
            let t = &model.loaded.topology;
            ui.label(format!(
                "{} vertices · {} triangles · {} preview triangles",
                model.loaded.mesh.vertices.len(),
                model.loaded.mesh.triangles.len(),
                model.facets.len()
            ));
            ui.label(format!("Boundary edges {} · nonmanifold edges {} · winding conflicts {} · duplicate faces {} · isolated vertices {}",t.boundary_edges,t.nonmanifold_edges,t.inconsistent_winding_edges,t.duplicate_faces,t.isolated_vertices));
            ui.label(format!(
                "Nonmanifold vertices {} · disconnected-fan vertices {} · maximum fans {}",
                t.nonmanifold_vertices, t.vertices_with_disconnected_fans, t.maximum_vertex_fans
            ))
            .on_hover_text(format!(
                "Problem vertex IDs (first 32): {:?}",
                t.nonmanifold_vertex_ids.iter().take(32).collect::<Vec<_>>()
            ));
            if t.nonmanifold_vertices > 0 {
                ui.colored_label(egui::Color32::from_rgb(255,80,80),"Red markers show nonmanifold vertices through surfaces (up to 10,000 sampled). No automatic splitting or welding.");
            }
        }
        ui.small("Large meshes preview at most 50,000 sampled triangles; inspect exported geometry before judging small holes.");
        ui.small("Drag to orbit · Shift-drag to pan · Scroll to zoom. Orthographic, depth-sorted preview; intersecting surfaces can render incorrectly. View controls do not modify geometry.");
        let (response, painter) = ui.allocate_painter(
            ui.available_size().max(egui::vec2(100., 100.)),
            egui::Sense::click_and_drag(),
        );
        let painter = painter.with_clip_rect(response.rect);
        painter.rect_filled(response.rect, 4., egui::Color32::from_rgb(13, 20, 30));
        if response.dragged() {
            if ui.input(|i| i.modifiers.shift) {
                self.pan += response.drag_delta();
            } else {
                self.yaw += f64::from(response.drag_delta().x) * 0.01;
                self.pitch =
                    (self.pitch + f64::from(response.drag_delta().y) * 0.01).clamp(-1.55, 1.55);
            }
        }
        if response.hovered() {
            self.zoom = (self.zoom
                * (f64::from(ui.input(|i| i.smooth_scroll_delta.y)) * 0.003).exp())
            .clamp(0.01, 1000.);
        }
        let Some(model) = self
            .pending
            .as_ref()
            .map(|p| &p.model)
            .or(self.model.as_deref())
        else {
            return;
        };
        let Some(bounds) = model.bounds else {
            return;
        };
        let rect = response.rect;
        let Ok(projection) = OrthographicProjection::new(
            bounds,
            [self.yaw, self.pitch],
            self.zoom,
            [
                f64::from(rect.min.x),
                f64::from(rect.min.y),
                f64::from(rect.width()),
                f64::from(rect.height()),
            ],
            [f64::from(self.pan.x), f64::from(self.pan.y)],
        ) else {
            return;
        };
        let mut faces = Vec::with_capacity(model.facets.len());
        for (index, facet) in model.facets.iter().enumerate() {
            let points: Option<Vec<_>> = facet
                .vertices
                .iter()
                .map(|p| projection.project(*p))
                .collect();
            let Some(points) = points else {
                continue;
            };
            if points
                .iter()
                .any(|p| p.xy.iter().any(|v| !v.is_finite() || v.abs() > 1e9))
            {
                continue;
            }
            let depth = points.iter().map(|p| p.depth / 3.).sum::<f64>();
            let n = facet.normal;
            let z1 = -n[0] * self.yaw.sin() + n[2] * self.yaw.cos();
            let nz = n[1] * self.pitch.sin() + z1 * self.pitch.cos();
            let brightness = (0.25 + 0.75 * nz.abs()).clamp(0., 1.);
            faces.push((
                depth,
                index,
                points
                    .iter()
                    .map(|p| egui::pos2(p.xy[0] as f32, p.xy[1] as f32))
                    .collect::<Vec<_>>(),
                brightness,
            ));
        }
        faces.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for (_, _, points, brightness) in faces {
            let fill = if self.shaded {
                egui::Color32::from_rgb(
                    (70. * brightness) as u8,
                    (190. * brightness) as u8,
                    (235. * brightness) as u8,
                )
            } else {
                egui::Color32::TRANSPARENT
            };
            let stroke = if self.wireframe {
                egui::Stroke::new(0.7_f32, egui::Color32::from_rgb(130, 220, 245))
            } else {
                egui::Stroke::NONE
            };
            painter.add(egui::Shape::convex_polygon(points, fill, stroke));
        }
        if self.show_normals {
            let length = (0..3)
                .map(|i| bounds.1[i] / 25. - bounds.0[i] / 25.)
                .fold(0f64, f64::max);
            if length.is_finite() && length > 0. {
                for facet in model
                    .facets
                    .iter()
                    .step_by(model.facets.len().div_ceil(2000).max(1))
                {
                    let center = std::array::from_fn(|i| {
                        facet.vertices.iter().map(|p| p[i] / 3.).sum::<f64>()
                    });
                    let tip = std::array::from_fn(|i| center[i] + length * facet.normal[i]);
                    if let (Some(a), Some(b)) =
                        (projection.project(center), projection.project(tip))
                        && a.xy
                            .iter()
                            .chain(&b.xy)
                            .all(|v| v.is_finite() && v.abs() < 1e9)
                    {
                        let a = egui::pos2(a.xy[0] as f32, a.xy[1] as f32);
                        let b = egui::pos2(b.xy[0] as f32, b.xy[1] as f32);
                        painter.arrow(
                            a,
                            b - a,
                            egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(255, 170, 80)),
                        );
                    }
                }
            }
        }
        if self.show_boundary
            && self.pending.is_none()
            && let Some(contour) = self
                .boundaries
                .as_ref()
                .and_then(|a| a.loops.get(self.selected_boundary))
        {
            for (a, b) in contour
                .vertices
                .iter()
                .zip(contour.vertices.iter().cycle().skip(1))
                .step_by(contour.vertices.len().div_ceil(10_000).max(1))
            {
                let endpoints =
                    [*a, *b].map(|i| projection.project(model.loaded.mesh.vertices[i as usize]));
                if let [Some(a), Some(b)] = endpoints
                    && a.xy
                        .iter()
                        .chain(&b.xy)
                        .all(|v| v.is_finite() && v.abs() < 1e9)
                {
                    painter.line_segment(
                        [
                            egui::pos2(a.xy[0] as f32, a.xy[1] as f32),
                            egui::pos2(b.xy[0] as f32, b.xy[1] as f32),
                        ],
                        egui::Stroke::new(2.5_f32, egui::Color32::YELLOW),
                    );
                }
            }
        }
        if self.show_section
            && self.pending.is_none()
            && let Some(section) = &self.section
        {
            for &[a, b] in section
                .segments
                .iter()
                .step_by(section.segments.len().div_ceil(10_000).max(1))
            {
                if let (Some(a), Some(b)) = (
                    projection.project(section.nodes[a as usize].position_mm),
                    projection.project(section.nodes[b as usize].position_mm),
                ) && a
                    .xy
                    .iter()
                    .chain(&b.xy)
                    .all(|v| v.is_finite() && v.abs() < 1e9)
                {
                    painter.line_segment(
                        [
                            egui::pos2(a.xy[0] as f32, a.xy[1] as f32),
                            egui::pos2(b.xy[0] as f32, b.xy[1] as f32),
                        ],
                        egui::Stroke::new(2.5_f32, egui::Color32::from_rgb(240, 140, 255)),
                    );
                }
            }
        }
        let bad = &model.loaded.topology.nonmanifold_vertex_ids;
        for &id in bad.iter().step_by(bad.len().div_ceil(10_000).max(1)) {
            if let Some(point) = projection.project(model.loaded.mesh.vertices[id as usize])
                && point.xy.iter().all(|v| v.is_finite() && v.abs() < 1e9)
            {
                painter.circle_filled(
                    egui::pos2(point.xy[0] as f32, point.xy[1] as f32),
                    4_f32,
                    egui::Color32::from_rgb(255, 80, 80),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_import_requires_units_preserves_source_and_opens_editable_project() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-gui-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.obj");
        let bytes = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        std::fs::write(&input, bytes).unwrap();
        let target = root.join("imported");
        let mut ui = MeshUi {
            import_path: input.display().to_string(),
            import_folder: target.display().to_string(),
            import_format: scancraft_geometry::mesh_import::Format::Obj,
            ..Default::default()
        };
        ui.import_mesh();
        assert!(ui.job.is_none());
        assert!(!target.exists());
        assert!(ui.message.contains("Choose"));
        ui.open_external_import(
            input.clone(),
            target.clone(),
            scancraft_geometry::mesh_import::Config {
                format: scancraft_geometry::mesh_import::Format::Obj,
                units: scancraft_geometry::mesh_import::Units::Inches,
                weld_exact: false,
            },
        );
        wait(&mut ui);
        assert!(ui.message.contains("Mesh imported"));
        assert_eq!(
            ui.model.as_ref().unwrap().loaded.mesh.vertices[1],
            [25.4, 0., 0.]
        );
        assert_eq!(
            ui.path,
            target.join("project.meshproj").display().to_string()
        );
        assert_eq!(std::fs::read(&input).unwrap(), bytes);
        assert_eq!(std::fs::read(target.join("original.obj")).unwrap(), bytes);
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        ui.plane_normal = [1., 0., 0.];
        ui.plane_offset = -12.7;
        ui.preview_trim();
        wait(&mut ui);
        assert!(ui.pending.is_some());
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        ui.output_folder = root.join("edited").display().to_string();
        ui.apply_trim();
        wait(&mut ui);
        assert_eq!(ui.undo.len(), 1);
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn smoothing_preview_apply_saved_undo_and_redo_preserve_original() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-smooth-gui-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.json");
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../evidence/mesh-smooth-bump-input.json"),
            &input,
        )
        .unwrap();
        let original_bytes = std::fs::read(&input).unwrap();
        let config = scancraft_geometry::mesh_smooth::Config {
            iterations: 5,
            relaxation: 0.5,
            maximum_displacement_mm: 0.1,
            maximum_normal_change_degrees: 30.,
            preserve_crease_degrees: 180.,
        };
        let mut ui = MeshUi::default();
        ui.open_smooth_preview(input.clone(), config);
        wait(&mut ui);
        assert!(ui.message.contains("Unapplied smoothing"));
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh.triangles.len(), 32);
        assert!(
            ui.pending
                .as_ref()
                .unwrap()
                .model
                .loaded
                .mesh
                .triangles
                .len()
                == 32
        );
        assert!(ui.pending.as_ref().unwrap().model.loaded.mesh.vertices[12][2] < 0.3);
        assert!(ui.undo.is_empty());
        assert_eq!(std::fs::read(&input).unwrap(), original_bytes);
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        let result = ui.pending.as_ref().unwrap().model.loaded.mesh.clone();
        ui.output_folder = root.join("result").display().to_string();
        ui.apply_trim();
        wait(&mut ui);
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, result);
        assert_eq!(ui.undo.len(), 1);
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        ui.redo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, result);
        let mut reopened = MeshUi::default();
        reopened.open(root.join("result/project.meshproj"));
        wait(&mut reopened);
        assert_eq!(reopened.model.as_ref().unwrap().loaded.mesh, result);
        reopened.undo();
        assert_eq!(reopened.model.as_ref().unwrap().loaded.mesh, original);
        assert_eq!(std::fs::read(&input).unwrap(), original_bytes);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn simplification_preview_apply_saved_undo_and_redo_preserve_original() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-simplify-gui-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.json");
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../evidence/mesh-simplify-grid-input.json"),
            &input,
        )
        .unwrap();
        let original_bytes = std::fs::read(&input).unwrap();
        let config = scancraft_geometry::mesh_simplify::Config {
            target_triangles: 70,
            maximum_displacement_mm: 1.5,
            maximum_normal_change_degrees: 30.,
            preserve_crease_degrees: 30.,
            maximum_collapses: 100,
        };
        let mut ui = MeshUi::default();
        ui.open_simplify_preview(input.clone(), config);
        wait(&mut ui);
        assert!(ui.message.contains("Unapplied simplification"));
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh.triangles.len(), 98);
        assert!(
            ui.pending
                .as_ref()
                .unwrap()
                .model
                .loaded
                .mesh
                .triangles
                .len()
                <= 70
        );
        assert!(ui.undo.is_empty());
        assert_eq!(std::fs::read(&input).unwrap(), original_bytes);
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        let result = ui.pending.as_ref().unwrap().model.loaded.mesh.clone();
        ui.output_folder = root.join("result").display().to_string();
        ui.apply_trim();
        wait(&mut ui);
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, result);
        assert_eq!(ui.undo.len(), 1);
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        ui.redo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, result);
        let mut reopened = MeshUi::default();
        reopened.open(root.join("result/project.meshproj"));
        wait(&mut reopened);
        assert_eq!(reopened.model.as_ref().unwrap().loaded.mesh, result);
        reopened.undo();
        assert_eq!(reopened.model.as_ref().unwrap().loaded.mesh, original);
        assert_eq!(std::fs::read(&input).unwrap(), original_bytes);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn gui_section_snapshot_save_reopen_and_corrupt_open_preserve_source() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-section-gui-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let mut ui = MeshUi {
            model: Some(Arc::new(model())),
            section_origin: [5., 0., 0.],
            section_normal: [1., 0., 0.],
            ..Default::default()
        };
        ui.analyze_section();
        wait(&mut ui);
        let source = ui.model.as_ref().unwrap().clone();
        let target = root.join("snapshot");
        ui.start(Operation::SaveSection {
            source: source.clone(),
            origin: ui.section_origin,
            normal: ui.section_normal,
            tolerance: ui.section_tolerance,
            target: target.clone(),
        });
        wait(&mut ui);
        assert!(ui.message.contains("snapshot saved"));
        assert!(Arc::ptr_eq(ui.model.as_ref().unwrap(), &source));
        assert!(ui.undo.is_empty());
        let descriptor = target.join("section.secproj");
        ui.open(descriptor.clone());
        wait(&mut ui);
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, source.loaded.mesh);
        assert!(ui.model.as_ref().unwrap().snapshot_source);
        assert_eq!(ui.section.as_ref().unwrap().total_segment_length_mm, 10.);
        assert!(ui.message.contains("snapshot reopened"));
        let current = ui.model.as_ref().unwrap().clone();
        let retained = ui.section.as_ref().unwrap().clone();
        let mut bytes = std::fs::read(&descriptor).unwrap();
        bytes[16] ^= 1;
        std::fs::write(&descriptor, &bytes).unwrap();
        ui.open(descriptor);
        wait(&mut ui);
        assert!(ui.message.contains("previous mesh retained"));
        assert!(Arc::ptr_eq(ui.model.as_ref().unwrap(), &current));
        assert!(Arc::ptr_eq(ui.section.as_ref().unwrap(), &retained));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn section_worker_is_read_only_and_history_changes_invalidate_analysis() {
        let original = Arc::new(model());
        let mut ui = MeshUi {
            model: Some(original.clone()),
            section_origin: [5., 0., 0.],
            section_normal: [1., 0., 0.],
            ..Default::default()
        };
        ui.analyze_section();
        wait(&mut ui);
        assert!(Arc::ptr_eq(ui.model.as_ref().unwrap(), &original));
        let section = ui.section.as_ref().unwrap();
        assert_eq!(section.contours.len(), 1);
        assert!(!section.contours[0].closed);
        assert_eq!(section.total_segment_length_mm, 10.);
        assert!(ui.undo.is_empty());
        assert!(ui.redo.is_empty());
        assert!(ui.pending.is_none());
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            perform(
                Operation::Section {
                    source: original.clone(),
                    origin: [5., 0., 0.],
                    normal: [1., 0., 0.],
                    tolerance: 0.
                },
                &cancelled
            )
            .is_err()
        );
        ui.section_normal = [0.; 3];
        ui.analyze_section();
        wait(&mut ui);
        assert!(ui.section.is_none());
        assert!(Arc::ptr_eq(ui.model.as_ref().unwrap(), &original));
        assert!(ui.message.contains("previous mesh retained"));
        ui.section_normal = [1., 0., 0.];
        ui.analyze_section();
        wait(&mut ui);
        assert!(ui.section.is_some());
        ui.undo.push_back(Arc::new(model()));
        ui.undo();
        assert!(ui.section.is_none());
        ui.analyze_section();
        wait(&mut ui);
        assert!(ui.section.is_some());
        ui.redo();
        assert!(ui.section.is_none());
        ui.analyze_section();
        wait(&mut ui);
        assert!(ui.section.is_some());
        ui.accept(Outcome::Loaded(model()));
        assert!(ui.section.is_none());
    }
    #[test]
    fn section_startup_loads_cube_and_computes_full_closed_contour() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/mesh-section-cube-input.json");
        let mut ui = MeshUi::default();
        ui.open_with_section(path, [0., 0., 0.5], [0., 0., 1.], 0.);
        wait(&mut ui);
        let section = ui.section.as_ref().unwrap();
        assert_eq!(section.segments.len(), 8);
        assert_eq!(section.contours.len(), 1);
        assert!(section.contours[0].closed);
        assert_eq!(section.total_segment_length_mm, 4.);
        assert!(ui.pending.is_none());
        assert!(ui.undo.is_empty());
    }
    fn model() -> Model {
        let mesh = scancraft_geometry::mesh::Mesh {
            vertices: vec![[0., 0., 0.], [10., 0., 0.], [0., 10., 0.], [10., 10., 0.]],
            triangles: vec![[0, 2, 1], [1, 2, 3]],
        };
        let work = WorkControl::default();
        let topology = mesh.topology(&work).unwrap();
        Model::build(LoadedMesh { mesh, topology }, &work).unwrap()
    }
    #[test]
    fn fan_split_preview_apply_and_history_are_reversible_with_unchanged_face_positions() {
        let root =
            std::env::temp_dir().join(format!("scancraft-ui-fan-split-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("pinch.json");
        let bytes = include_bytes!("../../../evidence/mesh-vertex-pinch-input.json");
        std::fs::write(&path, bytes).unwrap();
        let mut ui = MeshUi::default();
        ui.open(path.clone());
        wait(&mut ui);
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        ui.split_vertex_ids = "0".into();
        ui.preview_split_fans();
        wait(&mut ui);
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        let preview = &ui.pending.as_ref().unwrap().model;
        assert_eq!(preview.loaded.mesh.vertices.len(), 8);
        assert_eq!(preview.loaded.topology.nonmanifold_vertices, 0);
        for (before, after) in original
            .triangles
            .iter()
            .zip(&preview.loaded.mesh.triangles)
        {
            for i in 0..3 {
                assert_eq!(
                    original.vertices[before[i] as usize].map(f64::to_bits),
                    preview.loaded.mesh.vertices[after[i] as usize].map(f64::to_bits)
                );
            }
        }
        ui.output_folder = root.join("split").display().to_string();
        ui.apply_trim();
        wait(&mut ui);
        assert_eq!(
            ui.model
                .as_ref()
                .unwrap()
                .loaded
                .topology
                .nonmanifold_vertices,
            0
        );
        assert!(ui.split_vertex_ids.is_empty());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        ui.redo();
        ui.open(root.join("split/project.meshproj"));
        wait(&mut ui);
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        drop(ui);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn pinched_vertex_diagnostic_emits_marker_without_changing_geometry() {
        let work = WorkControl::default();
        let loaded = scancraft_workflow::mesh_document::decode(
            include_bytes!("../../../evidence/mesh-vertex-pinch-input.json"),
            &work,
        )
        .unwrap();
        let mut ui = MeshUi {
            model: Some(Arc::new(Model::build(loaded, &work).unwrap())),
            ..Default::default()
        };
        let source = ui.model.as_ref().unwrap().loaded.mesh.clone();
        assert!(ui.model.as_ref().unwrap().loaded.topology.edge_closed);
        assert!(
            !ui.model
                .as_ref()
                .unwrap()
                .loaded
                .topology
                .closed_manifold_candidate
        );
        let context = egui::Context::default();
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200., 1000.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |egui| ui.ui(egui));
            },
        );
        assert_eq!(output.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Circle(c) if c.fill==egui::Color32::from_rgb(255,80,80))).count(),1);
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, source);
    }
    #[test]
    fn orientation_preview_normals_apply_and_undo_preserve_exact_source() {
        let root = std::env::temp_dir().join(format!("scancraft-ui-orient-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let bytes = include_bytes!("../../../evidence/mesh-orientation-input.json").to_vec();
        let path = root.join("source.json");
        std::fs::write(&path, &bytes).unwrap();
        let mut ui = MeshUi::default();
        ui.open(path.clone());
        wait(&mut ui);
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        ui.preview_orientation();
        wait(&mut ui);
        assert!(ui.show_normals);
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        assert_eq!(
            ui.pending.as_ref().unwrap().prepared.report()["parameters"]["faces_flipped"],
            1
        );
        let context = egui::Context::default();
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200., 1000.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |egui| ui.ui(egui));
            },
        );
        assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::LineSegment {stroke,..} if stroke.color==egui::Color32::from_rgb(255,170,80)&&stroke.width==1.5)));
        ui.output_folder = root.join("oriented").display().to_string();
        ui.apply_trim();
        wait(&mut ui);
        assert_eq!(
            ui.model
                .as_ref()
                .unwrap()
                .loaded
                .topology
                .inconsistent_winding_edges,
            0
        );
        assert_eq!(
            ui.model.as_ref().unwrap().loaded.mesh.vertices,
            original.vertices
        );
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        ui.redo();
        ui.open(root.join("oriented/project.meshproj"));
        wait(&mut ui);
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        drop(ui);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn selected_fill_preview_apply_and_history_restore_exact_source() {
        let root = std::env::temp_dir().join(format!("scancraft-ui-fill-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("ring.json");
        let bytes = include_bytes!("../../../evidence/mesh-boundary-ring-input.json");
        std::fs::write(&input, bytes).unwrap();
        let mut ui = MeshUi::default();
        ui.open(input.clone());
        wait(&mut ui);
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        ui.analyze_boundaries();
        wait(&mut ui);
        ui.selected_boundary = 1;
        ui.preview_fill();
        wait(&mut ui);
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        assert_eq!(
            ui.pending
                .as_ref()
                .unwrap()
                .model
                .loaded
                .mesh
                .triangles
                .len(),
            10
        );
        assert_eq!(
            ui.pending.as_ref().unwrap().model.loaded.mesh.vertices,
            original.vertices
        );
        ui.output_folder = root.join("filled").display().to_string();
        ui.apply_trim();
        wait(&mut ui);
        let filled = ui.model.as_ref().unwrap().loaded.mesh.clone();
        assert_eq!(filled.triangles.len(), 10);
        assert!(ui.boundaries.is_none());
        assert_eq!(std::fs::read(input).unwrap(), bytes);
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        ui.redo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, filled);
        ui.open(root.join("filled/project.meshproj"));
        wait(&mut ui);
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        drop(ui);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn boundary_worker_highlight_and_version_invalidation_preserve_geometry() {
        let mut ui = MeshUi {
            model: Some(Arc::new(model())),
            ..Default::default()
        };
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        ui.analyze_boundaries();
        wait(&mut ui);
        assert_eq!(ui.boundaries.as_ref().unwrap().loops.len(), 1);
        let context = egui::Context::default();
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200., 1000.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |egui| ui.ui(egui));
            },
        );
        let yellow_edges=output.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::LineSegment {stroke,..} if stroke.color==egui::Color32::YELLOW&&stroke.width==2.5)).count();
        assert_eq!(yellow_edges, 4);
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        assert!(ui.pending.is_none());
        assert!(ui.undo.is_empty());
        ui.undo.push_back(ui.model.as_ref().unwrap().clone());
        ui.undo();
        assert!(ui.boundaries.is_none());
        ui.analyze_boundaries();
        wait(&mut ui);
        ui.redo();
        assert!(ui.boundaries.is_none());
        ui.analyze_boundaries();
        wait(&mut ui);
        ui.accept(Outcome::Loaded(model()));
        assert!(ui.boundaries.is_none());
    }
    #[test]
    fn mesh_view_emits_shaded_triangle_shapes_without_changing_geometry() {
        let mut ui = MeshUi {
            model: Some(Arc::new(model())),
            ..Default::default()
        };
        let before = ui.model.as_ref().unwrap().loaded.mesh.clone();
        let context = egui::Context::default();
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000., 800.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |egui| ui.ui(egui));
            },
        );
        let faces=output.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Path(p) if p.closed&&p.points.len()==3&&p.fill!=egui::Color32::TRANSPARENT&&p.fill.b()>p.fill.r())).count();
        assert_eq!(faces, 2);
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, before);
        ui.shaded = false;
        ui.wireframe = true;
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000., 800.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |egui| ui.ui(egui));
            },
        );
        assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Path(p) if p.closed&&p.points.len()==3&&p.fill==egui::Color32::TRANSPARENT)));
    }
    #[test]
    fn failed_background_load_preserves_previous_mesh_and_cancellation_rejects_preview() {
        let mut ui = MeshUi {
            model: Some(Arc::new(model())),
            ..Default::default()
        };
        let before = ui.model.as_ref().unwrap().loaded.mesh.clone();
        ui.open(PathBuf::from("/nonexistent-scancraft-mesh-fixture.json"));
        let start = std::time::Instant::now();
        while ui.job.is_some() {
            assert!(start.elapsed() < std::time::Duration::from_secs(3));
            ui.tick();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, before);
        assert!(ui.message.contains("previous mesh retained"));
        let work = WorkControl::default();
        work.cancel();
        let model = model();
        assert!(Model::build(model.loaded, &work).is_err());
    }
    fn wait(ui: &mut MeshUi) {
        let start = std::time::Instant::now();
        while ui.job.is_some() {
            assert!(start.elapsed() < std::time::Duration::from_secs(5));
            ui.tick();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn gui_trim_preview_apply_undo_redo_and_failed_apply_preserve_exact_versions() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-mesh-gui-trim-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.json");
        let bytes=br#"{"schema_version":1,"units":"millimeters","mesh":{"vertices":[[0,0,0],[1,0,0],[0,1,0],[1,1,0]],"triangles":[[0,2,1],[1,2,3]]}}"#;
        std::fs::write(&input, bytes).unwrap();
        let mut ui = MeshUi::default();
        ui.open(input.clone());
        wait(&mut ui);
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        ui.plane_offset = -0.5;
        ui.preview_trim();
        wait(&mut ui);
        assert!(ui.pending.is_some());
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        assert_eq!(
            ui.pending
                .as_ref()
                .unwrap()
                .model
                .loaded
                .mesh
                .triangles
                .len(),
            3
        );
        assert_eq!(std::fs::read(&input).unwrap(), bytes);
        assert!(ui.undo.is_empty());
        ui.output_folder = root.join("result").display().to_string();
        ui.apply_trim();
        wait(&mut ui);
        assert!(ui.pending.is_none());
        assert_eq!(ui.undo.len(), 1);
        let edited = ui.model.as_ref().unwrap().loaded.mesh.clone();
        assert_eq!(edited.triangles.len(), 3);
        assert_eq!(
            std::fs::read(root.join("result/original.mesh.json")).unwrap(),
            bytes
        );
        let saved = scancraft_workflow::mesh_document::load(
            &root.join("result/mesh.json"),
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(saved.mesh, edited);
        for _ in 0..100 {
            ui.undo();
            assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
            ui.redo();
            assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, edited);
        }
        ui.plane_offset = -0.25;
        ui.preview_trim();
        wait(&mut ui);
        assert!(ui.pending.is_some());
        ui.apply_trim();
        wait(&mut ui);
        assert!(ui.message.contains("already exists"));
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, edited);
        assert_eq!(ui.undo.len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn saved_version_history_is_bounded_and_source_change_rejects_preview() {
        let mut ui = MeshUi::default();
        ui.accept(Outcome::Loaded(model()));
        for _ in 0..40 {
            ui.accept(Outcome::Applied(model()));
        }
        assert_eq!(ui.undo.len(), 32);
        assert!(ui.redo.is_empty());
        assert!(ui.message.contains("1 old undo steps"));
        let work = WorkControl::default();
        let root = std::env::temp_dir().join(format!(
            "scancraft-mesh-source-change-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.json");
        let original=br#"{"schema_version":1,"units":"millimeters","mesh":{"vertices":[[0,0,0],[1,0,0],[0,1,0]],"triangles":[[0,1,2]]}}"#;
        std::fs::write(&input, original).unwrap();
        let Outcome::Loaded(source) = perform(Operation::Open(input.clone()), &work).unwrap()
        else {
            panic!("open result")
        };
        std::fs::write(&input,br#"{"schema_version":1,"units":"millimeters","mesh":{"vertices":[[0,0,1],[1,0,1],[0,1,1]],"triangles":[[0,1,2]]}}"#).unwrap();
        let result = perform(
            Operation::Preview {
                source: Arc::new(source),
                normal: [1., 0., 0.],
                offset: -0.5,
            },
            &work,
        );
        assert!(matches!(result,Err(message) if message.contains("source file changed")));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn full_gui_history_survives_save_reopen_missing_sources_and_further_trim() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-gui-history-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.json");
        std::fs::write(&input,br#"{"schema_version":1,"units":"millimeters","mesh":{"vertices":[[0,0,0],[1,0,0],[0,1,0],[1,1,0]],"triangles":[[0,2,1],[1,2,3]]}}"#).unwrap();
        let mut ui = MeshUi::default();
        ui.open(input.clone());
        wait(&mut ui);
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        for (offset, name) in [(-0.5, "first"), (-0.25, "second")] {
            ui.plane_offset = offset;
            ui.preview_trim();
            wait(&mut ui);
            assert!(ui.pending.is_some());
            ui.output_folder = root.join(name).display().to_string();
            ui.apply_trim();
            wait(&mut ui);
        }
        let second = ui.model.as_ref().unwrap().loaded.mesh.clone();
        ui.undo();
        let first = ui.model.as_ref().unwrap().loaded.mesh.clone();
        assert_eq!((ui.undo.len(), ui.redo.len()), (1, 1));
        let target = root.join("saved-history");
        ui.project_folder = target.display().to_string();
        ui.save_history();
        wait(&mut ui);
        assert!(target.join("project.meshproj").exists());
        drop(ui);
        std::fs::remove_file(&input).unwrap();
        std::fs::remove_dir_all(root.join("first")).unwrap();
        std::fs::remove_dir_all(root.join("second")).unwrap();
        let mut restored = MeshUi::default();
        restored.open(target.join("project.meshproj"));
        wait(&mut restored);
        assert_eq!((restored.undo.len(), restored.redo.len()), (1, 1));
        assert_eq!(restored.model.as_ref().unwrap().loaded.mesh, first);
        restored.undo();
        assert_eq!(restored.model.as_ref().unwrap().loaded.mesh, original);
        restored.redo();
        restored.redo();
        assert_eq!(restored.model.as_ref().unwrap().loaded.mesh, second);
        restored.plane_offset = -0.125;
        restored.preview_trim();
        wait(&mut restored);
        assert!(restored.pending.is_some());
        assert_eq!(restored.model.as_ref().unwrap().loaded.mesh, second);
        restored.pending = None;
        restored.project_folder = target.display().to_string();
        restored.save_history();
        wait(&mut restored);
        assert!(restored.message.contains("already exists"));
        assert_eq!(restored.model.as_ref().unwrap().loaded.mesh, second);
        let mut corrupt = std::fs::read(target.join("project.meshproj")).unwrap();
        corrupt[20] ^= 1;
        std::fs::write(root.join("corrupt.meshproj"), corrupt).unwrap();
        let depth = (restored.undo.len(), restored.redo.len());
        restored.open(root.join("corrupt.meshproj"));
        wait(&mut restored);
        assert_eq!((restored.undo.len(), restored.redo.len()), depth);
        assert_eq!(restored.model.as_ref().unwrap().loaded.mesh, second);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn cleanup_preview_apply_and_undo_preserve_original_geometry_and_history() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-gui-cleanup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.json");
        let bytes=br#"{"schema_version":1,"units":"millimeters","mesh":{"vertices":[[0,0,0],[1,0,0],[0,1,0],[99,99,99]],"triangles":[[0,2,1],[2,1,0],[1,2,0]]}}"#;
        std::fs::write(&input, bytes).unwrap();
        let mut ui = MeshUi::default();
        ui.open(input.clone());
        wait(&mut ui);
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        assert_eq!(
            ui.model.as_ref().unwrap().loaded.topology.duplicate_faces,
            2
        );
        ui.cleanup_duplicates = true;
        ui.cleanup_unused = true;
        ui.preview_cleanup();
        wait(&mut ui);
        assert!(ui.pending.is_some());
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        let preview = &ui.pending.as_ref().unwrap().model;
        assert_eq!(preview.loaded.mesh.vertices.len(), 3);
        assert_eq!(preview.loaded.mesh.triangles, vec![[0, 2, 1]]);
        assert_eq!(preview.loaded.topology.duplicate_faces, 0);
        assert_eq!(preview.loaded.topology.isolated_vertices, 0);
        ui.output_folder = root.join("cleaned").display().to_string();
        ui.apply_trim();
        wait(&mut ui);
        assert_eq!(ui.undo.len(), 1);
        let cleaned = ui.model.as_ref().unwrap().loaded.mesh.clone();
        assert_eq!(
            std::fs::read(root.join("cleaned/original.mesh.json")).unwrap(),
            bytes
        );
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        ui.redo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, cleaned);
        let data = std::fs::read(root.join("cleaned/project.meshproj")).unwrap();
        let work = WorkControl::default();
        let mut project = scancraft_geometry::mesh_project::load(data.as_slice(), &work).unwrap();
        project.undo(&work).unwrap();
        assert_eq!(project.mesh(), &original);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn component_analysis_explicit_small_part_preview_apply_and_history_are_reversible() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-gui-components-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.json");
        let bytes=br#"{"schema_version":1,"units":"millimeters","mesh":{"vertices":[[0,0,0],[1,0,0],[0,1,0],[1,1,0],[0,0,1],[0,1,1],[10,0,0],[11,0,0],[10,1,0]],"triangles":[[0,2,1],[1,2,3],[0,4,5],[6,7,8]]}}"#;
        std::fs::write(&input, bytes).unwrap();
        let mut ui = MeshUi::default();
        ui.open(input);
        wait(&mut ui);
        let original = ui.model.as_ref().unwrap().loaded.mesh.clone();
        ui.analyze_components();
        wait(&mut ui);
        assert_eq!(ui.components.as_ref().unwrap().components.len(), 3);
        ui.component_ids = "2".into();
        ui.component_keep = true;
        ui.preview_components();
        wait(&mut ui);
        assert!(ui.pending.is_some());
        assert_eq!(
            ui.pending
                .as_ref()
                .unwrap()
                .model
                .loaded
                .mesh
                .triangles
                .len(),
            1
        );
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        ui.output_folder = root.join("kept-small").display().to_string();
        ui.apply_trim();
        wait(&mut ui);
        assert_eq!(
            ui.model.as_ref().unwrap().loaded.mesh.vertices,
            original.vertices[6..9]
        );
        assert!(ui.components.is_none());
        assert_eq!(
            std::fs::read(root.join("kept-small/original.mesh.json")).unwrap(),
            bytes
        );
        ui.undo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh, original);
        assert!(ui.components.is_none());
        ui.redo();
        assert_eq!(ui.model.as_ref().unwrap().loaded.mesh.triangles.len(), 1);
        let data = std::fs::read(root.join("kept-small/project.meshproj")).unwrap();
        let work = WorkControl::default();
        let mut project = scancraft_geometry::mesh_project::load(data.as_slice(), &work).unwrap();
        project.undo(&work).unwrap();
        assert_eq!(project.mesh(), &original);
        std::fs::remove_dir_all(root).unwrap();
    }
}
