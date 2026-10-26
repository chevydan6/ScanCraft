use crate::cloud_jobs::{Document, Job, Operation, Preview};
use crate::screen_tools::{ScreenTools, Tool};
use eframe::egui;
use scancraft_geometry::selection::OrthographicProjection;
use std::path::PathBuf;
pub struct CloudUi {
    tools: ScreenTools,
    show_normals: bool,
    show_measurements: bool,
    normal_length_pixels: f32,
    document: Option<Document>,
    job: Option<Job>,
    preview: Preview,
    path: String,
    min: [f64; 3],
    max: [f64; 3],
    voxel: f64,
    radius: f64,
    neighbors: usize,
    statistical_neighbors: usize,
    statistical_sigma: f64,
    statistical_after_open: Option<(usize, f64)>,
    circle_plane_rms_mm: f64,
    cylinder_rms_mm: f64,
    cylinder_startup: Option<u8>,
    cylinder_panel_open: bool,
    angle_vertex_index: usize,
    plane_probe_index: usize,
    frame_indices: [usize; 3],
    frame_panel_open: bool,
    reusable_frame: Option<scancraft_geometry::coordinate_frame::Frame>,
    frame_label: String,
    frame_file: String,
    frame_folder: String,
    plane_rms_limit_mm: f64,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    pan: egui::Vec2,
    message: String,
    unit_scale: f64,
    dirty: bool,
    pending_import: Option<Operation>,
    reconstruction_files: [String; 5],
    reconstruction_open: bool,
    recording_files: [String; 5],
    recording_indices: [usize; 2],
    recording_open: bool,
    source_directory: String,
    recording_reference_directory: String,
    bundle_target: String,
    package_after_open: Option<(PathBuf, PathBuf)>,
    source_open: bool,
    verify_after_open: Option<PathBuf>,
    recorded_sequence_files: [String; 6],
    recorded_sequence_open: bool,
    sequence_files: [String; 2],
    sequence_open: bool,
    import_after_save: bool,
    measurement_rename: Option<(usize, String)>,
}
impl Default for CloudUi {
    fn default() -> Self {
        Self {
            tools: ScreenTools::default(),
            show_normals: false,
            show_measurements: false,
            normal_length_pixels: 24.,
            document: Some(Document::default()),
            job: None,
            preview: Preview::default(),
            path: String::new(),
            min: [0.; 3],
            max: [0.; 3],
            voxel: 1.,
            radius: 2.,
            neighbors: 3,
            statistical_neighbors: 16,
            statistical_sigma: 1.,
            statistical_after_open: None,
            circle_plane_rms_mm: 0.05,
            cylinder_rms_mm: 0.05,
            cylinder_startup: None,
            cylinder_panel_open: false,
            angle_vertex_index: 0,
            plane_probe_index: 0,
            frame_indices: [0, 1, 2],
            frame_panel_open: false,
            reusable_frame: None,
            frame_label: "Manual frame".into(),
            frame_file: String::new(),
            frame_folder: String::new(),
            plane_rms_limit_mm: 0.05,
            yaw: 0.,
            pitch: 0.,
            zoom: 1.,
            pan: egui::Vec2::ZERO,
            message: "Import geometry or reconstruct a calibrated stereo image pair.".into(),
            unit_scale: 1.,
            dirty: false,
            pending_import: None,
            import_after_save: false,
            measurement_rename: None,
            reconstruction_files: Default::default(),
            reconstruction_open: false,
            recording_files: Default::default(),
            recording_indices: [0; 2],
            recording_open: false,
            source_directory: String::new(),
            recording_reference_directory: String::new(),
            bundle_target: String::new(),
            package_after_open: None,
            source_open: false,
            verify_after_open: None,
            recorded_sequence_files: Default::default(),
            recorded_sequence_open: false,
            sequence_files: Default::default(),
            sequence_open: false,
        }
    }
}
impl CloudUi {
    pub fn open_cylinder_startup(&mut self, path: PathBuf, maximum_rms_mm: f64) {
        self.cylinder_rms_mm = maximum_rms_mm;
        self.cylinder_startup = Some(0);
        self.cylinder_panel_open = true;
        self.open_startup(path);
    }
    pub fn open_saved_frame(&mut self, path: PathBuf) {
        self.frame_panel_open = true;
        self.frame_file = path.to_string_lossy().into_owned();
        self.start(Operation::LoadFrame(path));
    }
    pub fn open_frame_startup(&mut self, path: PathBuf) {
        self.frame_panel_open = true;
        self.open_startup(path);
    }
    pub fn open_measurements_startup(&mut self, path: PathBuf) {
        self.show_measurements = true;
        self.open_startup(path);
    }
    pub fn open_normals_startup(&mut self, path: PathBuf) {
        self.show_normals = true;
        self.open_startup(path);
    }
    pub fn export_bundle_startup(&mut self, project: PathBuf, sources: PathBuf, target: PathBuf) {
        self.source_open = true;
        self.source_directory = sources.display().to_string();
        self.bundle_target = target.display().to_string();
        self.verify_after_open = None;
        self.package_after_open = Some((sources, target));
        self.open_startup(project);
    }
    pub fn verify_sources_startup(&mut self, project: PathBuf, directory: PathBuf) {
        self.package_after_open = None;
        self.source_open = true;
        self.source_directory = directory.display().to_string();
        self.verify_after_open = Some(directory);
        self.open_startup(project);
    }
    pub fn statistical_startup(&mut self, path: PathBuf, neighbors: usize, sigma: f64) {
        self.statistical_neighbors = neighbors;
        self.statistical_sigma = sigma;
        self.statistical_after_open = Some((neighbors, sigma));
        self.open_startup(path);
    }
    /// Completed live scans use the same unsaved-document guard as other imports.
    pub fn request_open(&mut self, path: PathBuf) -> bool {
        if self.is_busy() {
            return false;
        }
        self.submit(Operation::Load {
            path,
            unit_scale: None,
        });
        true
    }
    pub fn open_startup(&mut self, path: PathBuf) {
        self.path = path.to_string_lossy().into_owned();
        self.message = "Opening geometry file or verifying scan folder…".into();
        // Startup has no explicit fallback-unit choice. Honor declared units or
        // ask the user to choose a conversion in the existing import controls.
        self.start(Operation::Load {
            path,
            unit_scale: None,
        });
    }
    pub fn recorded_sequence_startup(
        &mut self,
        inputs: scancraft_workflow::recording_reconstruction::Inputs,
        settings: PathBuf,
        output: PathBuf,
    ) {
        self.recorded_sequence_files = [
            inputs.calibration.display().to_string(),
            inputs.left.display().to_string(),
            inputs.right.display().to_string(),
            inputs.settings.display().to_string(),
            settings.display().to_string(),
            output.display().to_string(),
        ];
        self.recorded_sequence_open = true;
        self.start(Operation::ReconstructRecordedSequence {
            inputs,
            settings,
            output,
        });
    }
    pub fn sequence_startup(&mut self, settings: PathBuf, output: PathBuf) {
        self.sequence_files = [settings.display().to_string(), output.display().to_string()];
        self.sequence_open = true;
        self.start(Operation::FuseSequence { settings, output });
    }
    pub fn reconstruct_startup(&mut self, inputs: scancraft_workflow::FileInputs, output: PathBuf) {
        self.reconstruction_files = [
            inputs.calibration.display().to_string(),
            inputs.left.display().to_string(),
            inputs.right.display().to_string(),
            inputs.settings.display().to_string(),
            output.display().to_string(),
        ];
        self.reconstruction_open = true;
        self.start(Operation::Reconstruct { inputs, output });
    }
    pub fn recording_reconstruction_startup(
        &mut self,
        inputs: scancraft_workflow::recording_reconstruction::Inputs,
        output: PathBuf,
    ) {
        self.recording_files = [
            inputs.calibration.display().to_string(),
            inputs.left.display().to_string(),
            inputs.right.display().to_string(),
            inputs.settings.display().to_string(),
            output.display().to_string(),
        ];
        self.recording_indices = [inputs.left_index, inputs.right_index];
        self.recording_open = true;
        self.start(Operation::ReconstructRecordings { inputs, output });
    }
    fn reconstruction_operation(&self) -> Operation {
        Operation::Reconstruct {
            inputs: scancraft_workflow::FileInputs {
                calibration: self.reconstruction_files[0].clone().into(),
                left: self.reconstruction_files[1].clone().into(),
                right: self.reconstruction_files[2].clone().into(),
                settings: self.reconstruction_files[3].clone().into(),
            },
            output: self.reconstruction_files[4].clone().into(),
        }
    }
    pub fn tick(&mut self) {
        let Some(output) = self.job.as_mut().and_then(Job::poll) else {
            return;
        };
        self.job = None;
        self.document = Some(output.document);
        match output.result {
            Ok(report) => {
                let opened = report.opened_path.is_some();
                if let Some((label, frame)) = report.coordinate_frame {
                    self.frame_label = label;
                    self.reusable_frame = Some(frame);
                }
                if let Some(path) = report.opened_path {
                    self.path = path.to_string_lossy().into_owned();
                }
                self.message = report.message;
                if let Some(preview) = report.preview {
                    self.preview = preview;
                }
                if report.reset_view {
                    self.dirty = false;
                    if let Some((min, max)) = self.preview.bounds {
                        self.min = min;
                        self.max = max;
                    }
                    self.zoom = 1.;
                    self.pan = egui::Vec2::ZERO;
                }
                if report.edited {
                    self.dirty = true;
                }
                if report.project_saved {
                    self.dirty = false;
                    if self.import_after_save {
                        self.import_after_save = false;
                        if let Some(operation) = self.pending_import.take() {
                            self.start(operation);
                        }
                    }
                }
                if opened && let Some((neighbors, sigma)) = self.statistical_after_open.take() {
                    self.start(Operation::StatisticalPreview { neighbors, sigma });
                    return;
                }
                if opened && let Some(directory) = self.verify_after_open.take() {
                    self.start(Operation::VerifySources { directory });
                    return;
                }
                if opened && let Some((sources, target)) = self.package_after_open.take() {
                    self.start(Operation::ExportProjectBundle { sources, target });
                    return;
                }
                if self.cylinder_startup == Some(0) && opened {
                    self.cylinder_startup = Some(1);
                    self.start(Operation::Invert);
                } else if self.cylinder_startup == Some(1) {
                    self.cylinder_startup = None;
                    self.start(Operation::FitCylinder {
                        maximum_rms_mm: self.cylinder_rms_mm,
                    });
                }
            }
            Err(e) => {
                self.cylinder_startup = None;
                self.verify_after_open = None;
                self.statistical_after_open = None;
                self.package_after_open = None;
                self.import_after_save = false;
                self.message = format!("Operation failed or was cancelled: {e}");
            }
        }
    }
    fn submit(&mut self, operation: Operation) {
        if self.dirty
            && matches!(
                &operation,
                Operation::Load { .. }
                    | Operation::Reconstruct { .. }
                    | Operation::ReconstructRecordings { .. }
                    | Operation::FuseSequence { .. }
                    | Operation::ReconstructRecordedSequence { .. }
            )
        {
            self.pending_import = Some(operation);
        } else {
            self.start(operation);
        }
    }
    fn start(&mut self, operation: Operation) {
        self.measurement_rename = None;
        self.tools.cancel();
        if let Some(document) = self.document.take() {
            self.job = Some(Job::start(document, operation));
        }
    }
    pub fn save_project(&mut self, folder: &str) {
        if self.job.is_none() {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            self.start(Operation::Save {
                path: PathBuf::from(folder).join(format!("cloud-{stamp}.orproject")),
                project: true,
            });
        }
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn cancel(&self) {
        if let Some(job) = &self.job {
            job.control.cancel();
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, folder: &mut String) {
        self.tick();
        let controls_height = (ui.available_height() - 280.).max(100.);
        egui::ScrollArea::vertical().id_salt("cloud-controls").max_height(controls_height).auto_shrink([false,true]).show(ui,|ui| {
        let mut verify_directory = None;
        let mut verify_recordings = None;
        let mut export_bundle = None;
        if let Some(live) = self.document.as_ref().and_then(|d|d.editor.as_ref()).and_then(|e|e.live_session()) {
            egui::CollapsingHeader::new("Live session sources").default_open(self.source_open).show(ui, |ui| {
                ui.label(format!("Session {} · {} candidate pairs · raw frames {}/{}", live.session_id, live.pairs.len(), live.recorded_frames[0], live.recorded_frames[1]));
                ui.label(if live.synthetic { "Synthetic fixture session" } else { "Declared physical observations; binding unverified" });
                ui.small(format!("Recorded folder: {}",live.directory_display));
                ui.label("References survive edits. External bytes are not checked on load; synchronization and physical accuracy remain unqualified.");
                ui.add_enabled_ui(self.job.is_none(), |ui| {
                    ui.horizontal(|ui| {ui.label("Folder to verify:");ui.text_edit_singleline(&mut self.source_directory);if ui.button("Use recorded location").clicked(){self.source_directory=live.directory_display.clone();}});
                    if ui.add_enabled(!self.source_directory.trim().is_empty(), egui::Button::new("Verify live session sources on disk")).clicked(){verify_directory=Some(PathBuf::from(&self.source_directory));}
                    ui.horizontal(|ui| { ui.label("New bundle folder:");ui.text_edit_singleline(&mut self.bundle_target); });
                    if ui.add_enabled(!self.source_directory.trim().is_empty()&&!self.bundle_target.trim().is_empty(), egui::Button::new("Export live project and raw session")).clicked(){export_bundle=Some((PathBuf::from(&self.source_directory),PathBuf::from(&self.bundle_target)));}
                });
            });
        }
        if let Some(sequence) = self.document.as_ref().and_then(|d|d.editor.as_ref()).and_then(|e|e.sequence()) {
            egui::CollapsingHeader::new("Recorded sequence sources").default_open(self.source_open).show(ui, |ui| {
                ui.label(format!("Session {} · {} selected pairs · {} retained artifact references",sequence.session_id,sequence.frames.len(),sequence.artifacts.len()));
                ui.label(format!("Declared source: {} · Calibration: {}", sequence.source_kind, sequence.calibration_record_id));
                ui.small(format!("Recorded folder: {}", sequence.directory_display));
                ui.label("References survive edits and reopening. External bytes are not checked on load; binding, synchronization and physical accuracy remain unqualified.");
                ui.add_enabled_ui(self.job.is_none(), |ui| {
                    ui.horizontal(|ui| {ui.label("Folder to verify:");ui.text_edit_singleline(&mut self.source_directory);if ui.button("Use recorded location").clicked(){self.source_directory=sequence.directory_display.clone();}});
                    if ui.add_enabled(!self.source_directory.trim().is_empty(), egui::Button::new("Verify sequence sources on disk")).clicked(){verify_directory=Some(PathBuf::from(&self.source_directory));}
                });
            });
        }
        if let Some(origin) = self
            .document
            .as_ref()
            .and_then(|d| d.editor.as_ref())
            .and_then(|e| e.origin())
        {
            egui::CollapsingHeader::new("Initial reconstruction provenance").default_open(self.source_open).show(ui,|ui| {
                ui.small("Initial inputs only; editing may change point correspondence. External references are not opened or verified on project load. Device binding, synchronization and physical accuracy remain unqualified.");
                ui.label(format!("Source bundle: {}",origin.bundle_directory_display));
                ui.label(format!("Calibration: {}",origin.calibration.get("record_id").and_then(|v|v.as_str()).unwrap_or("unknown")));
                for side in ["left","right"] {ui.label(format!("{side}: source frame {}",origin.recording_sources[side]["selected_frame_index"]));}
                for artifact in &origin.artifacts {ui.small(format!("{} · {} bytes · CRC32 {:08x}",artifact.name,artifact.bytes,artifact.crc32));}
                ui.add_enabled_ui(self.job.is_none(),|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Bundle to check:");ui.text_edit_singleline(&mut self.source_directory);
                        if ui.button("Use recorded location").clicked(){self.source_directory=origin.bundle_directory_display.clone();}
                        if ui.button("Use project folder").clicked(){let parent=std::path::Path::new(&self.path).parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));self.source_directory=parent.display().to_string();}
                    });
                    if ui.add_enabled(!self.source_directory.trim().is_empty(),egui::Button::new("Verify source bundle")).clicked(){verify_directory=Some(PathBuf::from(&self.source_directory));}
                    egui::CollapsingHeader::new("Export project with retained sources").show(ui,|ui| {
                        ui.small("Copies the edited project and five retained artifacts into a new folder. Full original recording sessions remain external. Active document stays unchanged.");
                        ui.horizontal(|ui|{ui.label("New bundle folder:");ui.text_edit_singleline(&mut self.bundle_target);});
                        if ui.add_enabled(!self.source_directory.trim().is_empty()&&!self.bundle_target.trim().is_empty(),egui::Button::new("Export new project bundle")).clicked(){export_bundle=Some((PathBuf::from(&self.source_directory),PathBuf::from(&self.bundle_target)));}
                    });
                });
            });
        }
        if let Some(references)=self.document.as_ref().and_then(|d|d.editor.as_ref()).and_then(|e|e.recordings()) {
            egui::CollapsingHeader::new("Complete recording references").default_open(true).show(ui,|ui| {
                ui.small("Saved fingerprints describe the supplied full recordings. Files are checked only when explicitly requested. No synchronization or physical calibration is inferred.");
                ui.label(format!("Recorded folder: {}",references.directory_display));
                for file in &references.files {ui.label(format!("{} · {} frames · {} bytes · CRC32 {:08x}",file.name,file.frames,file.bytes,file.crc32));}
                ui.add_enabled_ui(self.job.is_none(),|ui| {
                    ui.horizontal(|ui| {ui.label("Folder to verify:");ui.text_edit_singleline(&mut self.recording_reference_directory);if ui.button("Use recorded folder").clicked(){self.recording_reference_directory=references.directory_display.clone();}});
                    if ui.add_enabled(!self.recording_reference_directory.trim().is_empty(),egui::Button::new("Verify complete recordings")).clicked(){verify_recordings=Some(PathBuf::from(&self.recording_reference_directory));}
                });
            });
        }
        if let Some(directory)=verify_recordings {self.start(Operation::VerifyRecordings{directory});}
        if let Some((sources, target)) = export_bundle {
            self.start(Operation::ExportProjectBundle { sources, target });
        }
        if let Some(directory) = verify_directory {
            self.start(Operation::VerifySources { directory });
        }
        if let Some(operation) = self.pending_import.clone().filter(|_| self.job.is_none()) {
            egui::Modal::new(egui::Id::new("import-unsaved")).show(ui.ctx(), |ui| {
                ui.heading("Open another document?");
                ui.label("The current geometry has unsaved changes.");
                if ui.button("Keep current document").clicked() {
                    self.pending_import = None;
                }
                if ui.button("Save project, then continue").clicked() {
                    self.import_after_save = true;
                    self.save_project(folder);
                }
                if ui.button("Discard edits and continue").clicked() {
                    self.pending_import = None;
                    self.start(operation);
                }
            });
        }
        ui.heading("Point-cloud workbench");
        ui.label(
            "Offline geometry workbench · Scanner mode and physical accuracy remain unqualified",
        );
        let idle = self.job.is_none() && self.pending_import.is_none();
        let loaded = self.document.as_ref().is_some_and(|d| d.editor.is_some());
        let mut operation = None;
        egui::CollapsingHeader::new("Track and fuse cloud sequence").default_open(self.sequence_open).show(ui,|ui| {
            ui.label("Offline cloud frames in millimeters. Tracks accepted alignments and records tracking loss; physical accuracy remains unqualified.");
            ui.add_enabled_ui(idle,|ui| {
                for (label,value) in ["Sequence settings","New result folder"].into_iter().zip(self.sequence_files.iter_mut()) {
                    ui.horizontal(|ui|{ui.label(label);ui.add(egui::TextEdit::singleline(value).desired_width(500.));});
                }
                if ui.add_enabled(self.sequence_files.iter().all(|s|!s.trim().is_empty()),egui::Button::new("Track, fuse and open")).clicked() {
                    operation=Some(Operation::FuseSequence {settings:self.sequence_files[0].clone().into(),output:self.sequence_files[1].clone().into()});
                }
            });
        });
        egui::CollapsingHeader::new("Reconstruct and fuse recorded stereo sequence").default_open(self.recorded_sequence_open).show(ui, |ui| {
            ui.label("Offline explicit frame pairs with supplied calibration. Exposure timing and physical accuracy remain unqualified. Keep the complete result folder for frame provenance.");
            ui.add_enabled_ui(idle, |ui| {
                for (label, value) in ["Calibration file", "Left recording", "Right recording", "Reconstruction settings", "Sequence settings", "New result folder"].into_iter().zip(self.recorded_sequence_files.iter_mut()) {
                    ui.horizontal(|ui| { ui.label(label); ui.add(egui::TextEdit::singleline(value).desired_width(450.)); });
                }
                if ui.add_enabled(self.recorded_sequence_files.iter().all(|s|!s.trim().is_empty()), egui::Button::new("Reconstruct, track, fuse and open")).clicked() {
                    operation = Some(Operation::ReconstructRecordedSequence {
                        inputs: scancraft_workflow::recording_reconstruction::Inputs {
                            calibration:self.recorded_sequence_files[0].clone().into(),left:self.recorded_sequence_files[1].clone().into(),right:self.recorded_sequence_files[2].clone().into(),settings:self.recorded_sequence_files[3].clone().into(),left_index:0,right_index:0,
                        }, settings:self.recorded_sequence_files[4].clone().into(),output:self.recorded_sequence_files[5].clone().into(),
                    });
                }
            });
        });
        egui::CollapsingHeader::new("Reconstruct recorded stereo frames").default_open(self.recording_open).show(ui,|ui| {
            ui.label("Explicit left/right frame selection with supplied calibration and settings. Exposure synchronization and physical calibration binding are not independently verified. Calibration remains deferred unless you supply a verified record.");
            ui.add_enabled_ui(idle,|ui| {
                for (label,value) in ["Calibration file","Left recording (interface 0)","Right recording (interface 2)","Reconstruction settings","New result folder"].into_iter().zip(self.recording_files.iter_mut()) {
                    ui.horizontal(|ui|{ui.label(label);ui.add(egui::TextEdit::singleline(value).desired_width(450.));});
                }
                ui.horizontal(|ui|{ui.label("Zero-based left frame:");ui.add(egui::DragValue::new(&mut self.recording_indices[0]));ui.label("Right frame:");ui.add(egui::DragValue::new(&mut self.recording_indices[1]));});
                if ui.add_enabled(self.recording_files.iter().all(|s|!s.trim().is_empty()),egui::Button::new("Reconstruct recorded frames and open")).clicked() {
                    operation=Some(Operation::ReconstructRecordings {inputs:scancraft_workflow::recording_reconstruction::Inputs {calibration:self.recording_files[0].clone().into(),left:self.recording_files[1].clone().into(),right:self.recording_files[2].clone().into(),settings:self.recording_files[3].clone().into(),left_index:self.recording_indices[0],right_index:self.recording_indices[1]},output:self.recording_files[4].clone().into()});
                }
            });
        });
        egui::CollapsingHeader::new("Reconstruct from stereo images").default_open(self.reconstruction_open).show(ui,|ui| {
            ui.label("Offline image pair with matching calibration and reconstruction settings. Capture timing is not verified by this operation.");
            ui.add_enabled_ui(idle,|ui| {
                for (label,value) in ["Calibration file","Left image","Right image","Reconstruction settings","New result folder"].into_iter().zip(self.reconstruction_files.iter_mut()) {
                    ui.horizontal(|ui|{ui.label(label);ui.add(egui::TextEdit::singleline(value).desired_width(500.));});
                }
                if ui.add_enabled(self.reconstruction_files.iter().all(|s|!s.trim().is_empty()),egui::Button::new("Reconstruct and open")).clicked() {
                    operation=Some(self.reconstruction_operation());
                }
            });
        });
        ui.add_enabled_ui(idle, |ui| {
            ui.horizontal(|ui| {
                ui.label("File or scan folder:");
                ui.text_edit_singleline(&mut self.path);
                if ui.button("Import").clicked() {
                    operation = Some(Operation::Load {
                        path: PathBuf::from(&self.path),
                        unit_scale: Some(self.unit_scale),
                    });
                }
            });
            ui.horizontal(|ui| {
                ui.label("If file omits units:");
                egui::ComboBox::from_id_salt("plyunits")
                    .selected_text(if self.unit_scale == 1. {
                        "millimeters"
                    } else if self.unit_scale == 1000. {
                        "meters"
                    } else {
                        "inches"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.unit_scale, 1., "millimeters");
                        ui.selectable_value(&mut self.unit_scale, 1000., "meters");
                        ui.selectable_value(&mut self.unit_scale, 25.4, "inches");
                    });
                ui.label("Save folder:");
                ui.text_edit_singleline(folder);
            });
        });
        if self.dirty {
            ui.colored_label(egui::Color32::YELLOW, "Unsaved geometry changes");
        }
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} points · {} selected",
                self.preview.total, self.preview.selected
            ));
            ui.add_enabled_ui(idle && loaded, |ui| {
                if ui.button("Undo").clicked() {
                    operation = Some(Operation::Undo);
                }
                if ui.button("Redo").clicked() {
                    operation = Some(Operation::Redo);
                }
            });
            if ui.button("Fit view").clicked() {
                self.zoom = 1.;
                self.pan = egui::Vec2::ZERO;
            }
        });
        ui.add_enabled_ui(idle && loaded, |ui| {
            ui.horizontal(|ui| {
                ui.label("Box min (mm):");
                for v in &mut self.min {
                    ui.add(egui::DragValue::new(v).speed(0.1));
                }
                ui.label("max:");
                for v in &mut self.max {
                    ui.add(egui::DragValue::new(v).speed(0.1));
                }
            });
            ui.horizontal(|ui| {
                for (label, op) in [
                    (
                        "Select box",
                        Operation::SelectBox {
                            min: self.min,
                            max: self.max,
                        },
                    ),
                    ("Invert selection", Operation::Invert),
                    ("Delete selected", Operation::Trim { keep: false }),
                    ("Keep selected", Operation::Trim { keep: true }),
                ] {
                    if ui.button(label).clicked() {
                        operation = Some(op);
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Voxel (mm):");
                ui.add(egui::DragValue::new(&mut self.voxel).range(0.0001..=100000.));
                if ui.button("Downsample").clicked() {
                    operation = Some(Operation::Downsample(self.voxel));
                }
                for (label, project) in [("Export PLY", false), ("Save project", true)] {
                    if ui.button(label).clicked() {
                        let stamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos();
                        operation = Some(Operation::Save {
                            path: PathBuf::from(&*folder).join(format!(
                                "cloud-{stamp}.{}",
                                if project { "orproject" } else { "ply" }
                            )),
                            project,
                        });
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("Remove exact duplicates").clicked() {
                    operation = Some(Operation::Duplicates);
                }
                ui.label("Neighbor radius (mm):");
                ui.add(egui::DragValue::new(&mut self.radius).range(0.0001..=100000.));
                ui.label("Minimum:");
                ui.add(egui::DragValue::new(&mut self.neighbors).range(1..=1000));
                if ui.button("Filter isolated points").clicked() {
                    operation = Some(Operation::Radius {
                        radius: self.radius,
                        neighbors: self.neighbors,
                    });
                }
                ui.label("Statistical k:");ui.add(egui::DragValue::new(&mut self.statistical_neighbors).range(1..=1024));
                ui.label("Sigma multiplier:");ui.add(egui::DragValue::new(&mut self.statistical_sigma).speed(0.1).range(0.0..=1000.));
                if ui.button("Preview statistical outliers").clicked(){operation=Some(Operation::StatisticalPreview{neighbors:self.statistical_neighbors,sigma:self.statistical_sigma});}
                let selection = self
                    .document
                    .as_ref()
                    .map(|d| d.selection.as_slice())
                    .unwrap_or_default();
                if ui
                    .add_enabled(
                        selection.len() == 2,
                        egui::Button::new("Measure point distance"),
                    )
                    .clicked()
                {
                    operation = Some(Operation::MeasureDistance);
                }
                ui.label("Angle vertex (source index):");
                ui.add(egui::DragValue::new(&mut self.angle_vertex_index));
                if ui
                    .add_enabled(
                        selection.len() == 3 && selection.contains(&self.angle_vertex_index),
                        egui::Button::new("Measure angle"),
                    )
                    .clicked()
                {
                    operation = Some(Operation::MeasureAngle {
                        vertex: self.angle_vertex_index,
                    });
                }
                if selection.len() <= 3 && !selection.is_empty() {
                    ui.small(format!("Selected source indices: {selection:?}"));
                }
                if ui.button("Fit plane to selected").clicked() {
                    operation = Some(Operation::FitPlane);
                }
                ui.label("Circle plane RMS limit (mm):");
                ui.add(
                    egui::DragValue::new(&mut self.circle_plane_rms_mm)
                        .range(0. ..=100000.)
                        .speed(0.01),
                );
                let circle_ready = self
                    .document
                    .as_ref()
                    .is_some_and(|d| d.selection.len() >= 3);
                if ui
                    .add_enabled(circle_ready, egui::Button::new("Fit circle to selected"))
                    .clicked()
                {
                    operation = Some(Operation::FitCircle {
                        maximum_plane_rms_mm: self.circle_plane_rms_mm,
                    });
                }
                let sphere_ready = self
                    .document
                    .as_ref()
                    .is_some_and(|d| d.selection.len() >= 4);
                if ui
                    .add_enabled(sphere_ready, egui::Button::new("Fit sphere to selected"))
                    .clicked()
                {
                    operation = Some(Operation::FitSphere);
                }
                let selected = self
                    .document
                    .as_ref()
                    .is_some_and(|d| !d.selection.is_empty());
                if ui
                    .add_enabled(selected, egui::Button::new("Flip selected normals"))
                    .clicked()
                {
                    operation = Some(Operation::FlipNormals);
                }
            });
        });
        egui::CollapsingHeader::new("Cylinder measurement").default_open(self.cylinder_panel_open).show(ui, |ui| {
            ui.label("Select cylindrical wall points across multiple heights and angles (6..20,000). Endcaps and outliers affect this local geometric least-squares fit. Save retained evidence in the snapshot panel; physical accuracy remains unqualified.");
            ui.horizontal(|ui| {ui.label("Maximum geometric RMS (mm):");ui.add(egui::DragValue::new(&mut self.cylinder_rms_mm).range(0. ..=100000.).speed(0.01));
                let ready=idle&&self.document.as_ref().is_some_and(|d|(6..=20_000).contains(&d.selection.len()));
                if ui.add_enabled(ready,egui::Button::new("Fit cylinder to selected")).clicked() {operation=Some(Operation::FitCylinder {maximum_rms_mm:self.cylinder_rms_mm});}
            });
        });
        egui::CollapsingHeader::new("Point-to-plane measurement").show(ui, |ui| {
            ui.label("Select a probe plus at least three reference plane points. The probe is excluded from the fit. Source indices refer to the current cloud.");
            ui.horizontal_wrapped(|ui| {
                ui.label("Probe source index:");ui.add(egui::DragValue::new(&mut self.plane_probe_index));
                ui.label("Maximum reference plane RMS (mm):");ui.add(egui::DragValue::new(&mut self.plane_rms_limit_mm).range(0. ..=100000.).speed(0.01));
                let editor=self.document.as_ref().and_then(|d|d.editor.as_ref());
                let selected=self.document.as_ref().map(|d|d.selection.as_slice()).unwrap_or_default();
                let ready=idle && (4..=100_001).contains(&selected.len()) && selected.contains(&self.plane_probe_index);
                if ui.add_enabled(ready,egui::Button::new("Measure point to plane")).clicked() {operation=Some(Operation::PointPlane {probe:self.plane_probe_index,maximum_plane_rms_mm:self.plane_rms_limit_mm,label:None});}
                if ui.add_enabled(ready && editor.is_some_and(|e|e.measurements().len()<16),egui::Button::new("Save point-plane measurement")).clicked() {
                    let label=(1..=17).map(|i|format!("Point-plane {i}")).find(|label|!editor.unwrap().measurements().iter().any(|s|s.label==*label)).unwrap();
                    operation=Some(Operation::PointPlane {probe:self.plane_probe_index,maximum_plane_rms_mm:self.plane_rms_limit_mm,label:Some(label)});
                }
                if !selected.is_empty() {ui.small(format!("First selected source indices: {:?}",&selected[..selected.len().min(16)]));}
            });
        });
        egui::CollapsingHeader::new("Coordinate frame").default_open(self.frame_panel_open).show(ui, |ui| {
            ui.label("Transform the entire cloud using selected source references. +X follows origin to X; the third point chooses the +XY side. Normals rotate; scale is unchanged. Historical measurement snapshots keep their old coordinates.");
            ui.add_enabled_ui(idle && loaded, |ui| {ui.horizontal_wrapped(|ui| {
                for (label,index) in ["Origin source index:","X source index:","XY-side source index:"].into_iter().zip(&mut self.frame_indices) {ui.label(label);ui.add(egui::DragValue::new(index));}
                let selection=self.document.as_ref().map(|d|d.selection.as_slice()).unwrap_or_default();
                let [origin,x,xy]=self.frame_indices;
                if ui.add_enabled(selection.contains(&origin),egui::Button::new("Move origin to reference")).clicked() {operation=Some(Operation::AlignFrame {origin,axes:None});}
                let ready=origin!=x&&origin!=xy&&x!=xy&&self.frame_indices.iter().all(|i|selection.contains(i));
                if ui.add_enabled(ready,egui::Button::new("Define reusable frame")).clicked() {operation=Some(Operation::DefineFrame {origin,axes:Some((x,xy))});}
                if ui.add_enabled(ready,egui::Button::new("Align coordinate frame")).clicked() {operation=Some(Operation::AlignFrame {origin,axes:Some((x,xy))});}
                if !selection.is_empty() {ui.small(format!("First selected source indices: {:?}",&selection[..selection.len().min(16)]));}
            });});
            ui.separator();ui.label("Reusable frame maps world millimeters into local millimeters. Loading/defining does not change the cloud; Apply transforms the current cloud, including one already transformed.");
            ui.add_enabled_ui(idle,|ui| {
                ui.horizontal(|ui| {ui.label("Frame file:");ui.text_edit_singleline(&mut self.frame_file);if ui.add_enabled(!self.frame_file.trim().is_empty(),egui::Button::new("Load frame")).clicked() {operation=Some(Operation::LoadFrame(self.frame_file.clone().into()));}});
                if let Some(frame)=&self.reusable_frame {
                    ui.small(format!("Origin {:?} mm; axes {:?}",frame.origin_mm,frame.axes));
                    ui.horizontal(|ui| {ui.label("Frame name:");ui.text_edit_singleline(&mut self.frame_label);if ui.add_enabled(loaded,egui::Button::new("Apply reusable frame")).clicked() {operation=Some(Operation::ApplyFrame {label:self.frame_label.clone(),frame:frame.clone()});}});
                    ui.horizontal(|ui| {ui.label("New frame folder:");ui.text_edit_singleline(&mut self.frame_folder);if ui.add_enabled(!self.frame_folder.trim().is_empty(),egui::Button::new("Save reusable frame")).clicked() {operation=Some(Operation::SaveFrame {label:self.frame_label.clone(),frame:frame.clone(),target:self.frame_folder.clone().into()});}});
                }
            });
        });
        egui::CollapsingHeader::new("Saved measurement snapshots")
            .id_salt((
                "saved-measurements",
                self.document
                    .as_ref()
                    .and_then(|d| d.editor.as_ref())
                    .map_or((0, false), |e| {
                        (e.measurements().len(), e.has_measurement_history())
                    }),
            ))
            .default_open(
                self.document
                    .as_ref()
                    .and_then(|d| d.editor.as_ref())
                    .is_some_and(|e| !e.measurements().is_empty() || e.has_measurement_history()),
            )
            .show(ui, |ui| {
                ui.label(
                    "Snapshots retain source coordinates; later cloud edits do not update them.",
                );
                if let Some(editor) = self.document.as_ref().and_then(|d| d.editor.as_ref()) {
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                idle && editor.can_undo_measurement(),
                                egui::Button::new("Undo snapshot change"),
                            )
                            .clicked()
                        {
                            operation = Some(Operation::UndoMeasurement);
                        }
                        if ui
                            .add_enabled(
                                idle && editor.can_redo_measurement(),
                                egui::Button::new("Redo snapshot change"),
                            )
                            .clicked()
                        {
                            operation = Some(Operation::RedoMeasurement);
                        }
                    });
                    for (index, snapshot) in editor.measurements().iter().enumerate() {
                        let summary = match &snapshot.result {
                            scancraft_geometry::measurement_snapshot::ResultValue::Cylinder {fit,maximum_rms_mm} => format!("cylinder diameter {:.6} mm; radial RMS {:.6} mm; limit {:.6} mm",fit.diameter_mm,fit.geometric_rms_mm,maximum_rms_mm),
                            scancraft_geometry::measurement_snapshot::ResultValue::PointPlane {
                                fit,
                                ..
                            } => format!(
                                "point-plane {:.6} mm; plane RMS {:.6} mm",
                                fit.distance_mm, fit.plane_rms_mm
                            ),
                            scancraft_geometry::measurement_snapshot::ResultValue::Sphere { fit } => {
                                format!("diameter {:.6} mm", fit.diameter_mm)
                            }
                            scancraft_geometry::measurement_snapshot::ResultValue::Circle {
                                fit,
                                ..
                            } => format!("diameter {:.6} mm", fit.diameter_mm),
                            scancraft_geometry::measurement_snapshot::ResultValue::Distance {
                                distance_mm,
                            } => format!(
                                "distance {distance_mm:.6} mm ({:.6} in)",
                                distance_mm / 25.4
                            ),
                            scancraft_geometry::measurement_snapshot::ResultValue::Angle {
                                degrees,
                                vertex,
                            } => format!("angle {degrees:.6} degrees; retained vertex {vertex}"),
                        };
                        ui.horizontal(|ui| {
                            let response=ui.label(format!("{} · {} · {} source points",snapshot.label,summary,snapshot.source_points_mm.len()));
                            if let scancraft_geometry::measurement_snapshot::ResultValue::Cylinder {fit,..}=&snapshot.result {
                                response.on_hover_text(format!("Historical axis point {:?} mm; direction {:?}; sampled axial range {:?} mm; maximum radial residual {:.6} mm; normalized Jacobian condition {:.3}; {} iterations; {} converged starts. No physical qualification.",fit.axis_point_mm,fit.axis_direction,fit.axial_range_mm,fit.maximum_absolute_residual_mm,fit.jacobian_condition_number,fit.iterations,fit.starts_converged));
                            }
                            if ui.add_enabled(idle, egui::Button::new("Rename")).clicked() {
                                self.measurement_rename = Some((index, snapshot.label.clone()));
                            }
                            if ui.add_enabled(idle, egui::Button::new("Remove")).clicked() {
                                operation = Some(Operation::RemoveMeasurement { index });
                                self.measurement_rename = None;
                            }
                        });
                    }
                    let mut close_rename = false;
                    if let Some((index, label)) = &mut self.measurement_rename {
                        ui.horizontal(|ui| {
                            ui.label("New snapshot label:");
                            ui.text_edit_singleline(label);
                            if ui
                                .add_enabled(idle, egui::Button::new("Apply rename"))
                                .clicked()
                            {
                                operation = Some(Operation::RenameMeasurement {
                                    index: *index,
                                    label: label.clone(),
                                });
                                close_rename = true;
                            }
                            if ui.button("Cancel rename").clicked() {
                                close_rename = true;
                            }
                        });
                    }
                    if close_rename {
                        self.measurement_rename = None;
                    }
                    let count = self.document.as_ref().map_or(0, |d| d.selection.len());
                    for (kind, ready) in [
                        ("Sphere", count >= 4),
                        ("Cylinder", (6..=20_000).contains(&count)),
                        ("Circle", count >= 3),
                        ("Distance", count == 2),
                        (
                            "Angle",
                            count == 3
                                && self.document.as_ref().is_some_and(|d| {
                                    d.selection.contains(&self.angle_vertex_index)
                                }),
                        ),
                    ] {
                        if ui
                            .add_enabled(
                                idle && ready && editor.measurements().len() < 16,
                                egui::Button::new(format!("Save selected {kind} measurement")),
                            )
                            .clicked()
                        {
                            let label = (1..=17)
                                .map(|i| format!("{kind} {i}"))
                                .find(|label| {
                                    !editor.measurements().iter().any(|s| s.label == *label)
                                })
                                .unwrap();
                            operation = Some(if kind == "Cylinder" {
                                Operation::RecordCylinder {
                                    label,
                                    maximum_rms_mm: self.cylinder_rms_mm,
                                }
                            } else if kind == "Distance" {
                                Operation::RecordDistance { label }
                            } else if kind == "Angle" {
                                Operation::RecordAngle {
                                    label,
                                    vertex: self.angle_vertex_index,
                                }
                            } else if kind == "Sphere" {
                                Operation::RecordSphere { label }
                            } else {
                                Operation::RecordCircle {
                                    label,
                                    maximum_plane_rms_mm: self.circle_plane_rms_mm,
                                }
                            });
                        }
                    }
                }
            });
        if let Some(document) = &self.document
            && document.selection.len() == 2
            && let Some(editor) = &document.editor
        {
            let a = editor.cloud.points[document.selection[0]].position;
            let b = editor.cloud.points[document.selection[1]].position;
            if let Ok(d) = scancraft_geometry::distance_mm(a, b) {
                ui.label(format!(
                    "Selected-point distance: {d:.6} mm (imported units)"
                ));
            }
        }
        if let Some(op) = operation {
            self.submit(op);
        }

        ui.horizontal(|ui| {
            ui.add_enabled(
                self.preview.normals > 0,
                egui::Checkbox::new(&mut self.show_normals, "Show point normals"),
            );
            ui.label("Arrow length (pixels):");
            ui.add(egui::DragValue::new(&mut self.normal_length_pixels).range(4. ..=80.));
            ui.small(format!(
                "{} supported normals; at most 2,000 sampled arrows",
                self.preview.normals
            ));
        });
        self.tools.toolbar(ui, idle && loaded);
        ui.checkbox(
            &mut self.show_measurements,
            "Show historical measurement outlines (purple)",
        );
        if ui
            .add_enabled(idle && loaded, egui::Button::new("Clear selection"))
            .clicked()
        {
            self.start(Operation::ClearSelection);
        }
        });
        let idle = self.job.is_none() && self.pending_import.is_none();
        let loaded = self.document.as_ref().is_some_and(|d| d.editor.is_some());
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
        egui::ScrollArea::vertical()
            .id_salt("cloud-status")
            .max_height(64.)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.label(&self.message);
            });
        ui.label("Click to select · Ctrl-click to toggle · Drag to orbit · Shift-drag to pan · Scroll to zoom. View samples at most 50,000 points; edits use all points.");
        let (response, painter) = ui.allocate_painter(
            ui.available_size().max(egui::vec2(100., 100.)),
            egui::Sense::click_and_drag(),
        );
        painter.rect_filled(response.rect, 5., egui::Color32::from_rgb(13, 20, 30));
        if !self.tools.active()
            && (response.dragged_by(egui::PointerButton::Secondary)
                || (self.tools.tool == Tool::Navigate
                    && response.dragged_by(egui::PointerButton::Primary)))
        {
            if ui.input(|i| i.modifiers.shift) {
                self.pan += response.drag_delta();
            } else {
                self.yaw += response.drag_delta().x * 0.01;
                self.pitch = (self.pitch + response.drag_delta().y * 0.01).clamp(-1.55, 1.55);
            }
        }
        if response.hovered() && !self.tools.active() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            self.zoom = (self.zoom * (scroll * 0.003).exp()).clamp(0.01, 1000.);
        }
        let pointer = if idle
            && self.tools.tool == Tool::Navigate
            && response.clicked_by(egui::PointerButton::Primary)
        {
            response.interact_pointer_pos()
        } else {
            None
        };
        let view = View {
            yaw: self.yaw,
            pitch: self.pitch,
            zoom: self.zoom,
            pan: self.pan,
        };
        let projection = projection(&self.preview, response.rect, &view);
        let picked = draw(
            &self.preview,
            &painter,
            response.rect,
            View {
                yaw: self.yaw,
                pitch: self.pitch,
                zoom: self.zoom,
                pan: self.pan,
            },
            pointer,
            self.show_normals.then_some(self.normal_length_pixels),
        );
        if self.show_measurements
            && let (Some(projection), Some(editor)) = (
                projection.as_ref(),
                self.document.as_ref().and_then(|d| d.editor.as_ref()),
            )
        {
            crate::measurement_view::draw(
                editor.measurements(),
                projection,
                &painter,
                response.rect,
            );
        }
        if let Some(request) =
            self.tools
                .interact(ui, &response, &painter, projection, idle && loaded)
        {
            self.start(Operation::SelectScreen {
                projection: request.projection,
                shape: request.shape,
                visibility: request.visibility,
                mode: request.mode,
            });
        }
        if let Some(index) = picked {
            self.start(Operation::Pick {
                index,
                additive: ui.input(|i| i.modifiers.ctrl || i.modifiers.command),
            });
        }
    }
}
pub(crate) struct View {
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) zoom: f32,
    pub(crate) pan: egui::Vec2,
}
fn projection(preview: &Preview, rect: egui::Rect, view: &View) -> Option<OrthographicProjection> {
    OrthographicProjection::new(
        preview.bounds?,
        [f64::from(view.yaw), f64::from(view.pitch)],
        f64::from(view.zoom),
        [
            f64::from(rect.min.x),
            f64::from(rect.min.y),
            f64::from(rect.width()),
            f64::from(rect.height()),
        ],
        [f64::from(view.pan.x), f64::from(view.pan.y)],
    )
    .ok()
}
pub(crate) fn draw(
    preview: &Preview,
    p: &egui::Painter,
    rect: egui::Rect,
    view: View,
    pointer: Option<egui::Pos2>,
    normal_length: Option<f32>,
) -> Option<usize> {
    let projection = projection(preview, rect, &view)?;
    let mut projected = Vec::with_capacity(preview.points.len());
    for point in &preview.points {
        let Some(q) = projection.project(point.position) else {
            continue;
        };
        let depth = q.depth;
        let pos = egui::pos2(q.xy[0] as f32, q.xy[1] as f32);
        if pos.x.is_finite() && pos.y.is_finite() && rect.contains(pos) {
            let color = if point.selected {
                egui::Color32::YELLOW
            } else {
                let [r, g, b] = point.color;
                egui::Color32::from_rgb(r, g, b)
            };
            projected.push((depth, pos, color, point.index));
        }
    }
    projected.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.3.cmp(&b.3)));
    let mut hit: Option<(f32, f64, usize)> = None;
    for (depth, pos, color, index) in projected {
        p.circle_filled(pos, 1.8, color);
        if let Some(pointer) = pointer {
            let d = pos.distance_sq(pointer);
            if d <= 36.
                && hit.is_none_or(|(old, z, old_index)| {
                    d < old - 1e-6
                        || ((d - old).abs() <= 1e-6
                            && (depth > z || (depth == z && index > old_index)))
                })
            {
                hit = Some((d, depth, index));
            }
        }
    }
    if let Some(length) = normal_length {
        let painter = p.with_clip_rect(rect);
        for (origin, direction) in normal_arrows(preview, rect, &view, length) {
            painter.arrow(
                origin,
                direction,
                egui::Stroke::new(1_f32, egui::Color32::from_rgb(255, 160, 70)),
            );
        }
    }
    hit.map(|(_, _, index)| index)
}

fn normal_arrows(
    preview: &Preview,
    rect: egui::Rect,
    view: &View,
    length: f32,
) -> Vec<(egui::Pos2, egui::Vec2)> {
    if !length.is_finite() || !(1. ..=128.).contains(&length) {
        return Vec::new();
    }
    let Some(projection) = projection(preview, rect, view) else {
        return Vec::new();
    };
    let count = preview.points.iter().filter(|p| p.normal.is_some()).count();
    let step = count.div_ceil(2000).max(1);
    let (sy, cy) = f64::from(view.yaw).sin_cos();
    let (sp, cp) = f64::from(view.pitch).sin_cos();
    preview
        .points
        .iter()
        .filter(|p| p.normal.is_some())
        .step_by(step)
        .take(2000)
        .filter_map(|p| {
            let n = p.normal?;
            let q = projection.project(p.position)?;
            let origin = egui::pos2(q.xy[0] as f32, q.xy[1] as f32);
            let x = n[0] * cy + n[2] * sy;
            let z = -n[0] * sy + n[2] * cy;
            let y = n[1] * cp - z * sp;
            let vector = egui::vec2(x as f32, -y as f32) * length;
            (rect.contains(origin) && vector.is_finite() && vector.length_sq() > 0.25)
                .then_some((origin, vector))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud_jobs::DisplayPoint;
    #[test]
    fn expanded_source_and_measurements_leave_viewport_inside_default_window() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/project-origin-gui-render/result");
        let mut ui = CloudUi::default();
        let mut editor = scancraft_geometry::formats::load_project(
            std::fs::File::open(root.join("project.orproj")).unwrap(),
        )
        .unwrap();
        let points = editor
            .cloud
            .points
            .iter()
            .take(2)
            .map(|p| p.position)
            .collect();
        editor
            .add_measurement(
                scancraft_geometry::measurement_snapshot::Snapshot::distance(
                    "layout fixture".into(),
                    points,
                    &scancraft_geometry::work::WorkControl::default(),
                )
                .unwrap(),
            )
            .unwrap();
        ui.document.as_mut().unwrap().editor = Some(editor);
        ui.source_open = true;
        let ctx = egui::Context::default();
        let mut folder = String::new();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1150., 850.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |egui_ui| ui.ui(egui_ui, &mut folder));
            },
        );
        let rectangles: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(rect)
                    if rect.fill == egui::Color32::from_rgb(13, 20, 30) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(rectangles.len(), 1);
        assert!(rectangles[0].height() >= 180.);
        assert!(rectangles[0].max.y <= 850.);
    }
    #[test]
    fn bundle_export_startup_loads_then_exports_without_replacing_active_origin() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/project-origin-gui-render/result");
        let target = std::env::temp_dir().join(format!(
            "scancraft-ui-bundle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut ui = CloudUi::default();
        ui.export_bundle_startup(root.join("project.orproj"), root.clone(), target.clone());
        let start = std::time::Instant::now();
        while ui.is_busy() {
            ui.tick();
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(ui.message.contains("Exported project bundle"));
        assert!(!ui.dirty);
        assert!(target.join("project.orproj").is_file());
        assert_ne!(
            ui.document
                .as_ref()
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .origin()
                .unwrap()
                .bundle_directory_display,
            target.display().to_string()
        );
        std::fs::remove_dir_all(target).unwrap();
    }
    #[test]
    fn source_verification_startup_waits_for_load_and_keeps_dirty_state_on_later_checks() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/project-origin-gui-render/result");
        let mut ui = CloudUi::default();
        ui.verify_sources_startup(root.join("project.orproj"), root.clone());
        let start = std::time::Instant::now();
        while ui.is_busy() {
            ui.tick();
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(ui.message.contains("verification passed"));
        assert!(!ui.dirty);
        let origin = ui
            .document
            .as_ref()
            .unwrap()
            .editor
            .as_ref()
            .unwrap()
            .origin()
            .unwrap()
            .clone();
        ui.dirty = true;
        ui.start(Operation::VerifySources { directory: root });
        let start = std::time::Instant::now();
        while ui.is_busy() {
            ui.tick();
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(ui.dirty);
        assert_eq!(
            ui.document
                .as_ref()
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .origin(),
            Some(&origin)
        );
    }
    #[test]
    fn normal_preview_normalizes_only_display_copies_and_skips_zero_vectors() {
        use scancraft_geometry::{Cloud, Point, work::WorkControl};
        let cloud = Cloud {
            points: vec![
                Point {
                    position: [0.; 3],
                    color: None,
                    normal: Some([2., 0., 0.]),
                },
                Point {
                    position: [1.; 3],
                    color: None,
                    normal: Some([0.; 3]),
                },
            ],
        };
        let original = cloud.clone();
        let preview =
            crate::cloud_jobs::build_preview(&cloud, &[], &WorkControl::default()).unwrap();
        assert_eq!(preview.normals, 1);
        assert_eq!(preview.points[0].normal, Some([1., 0., 0.]));
        assert_eq!(preview.points[1].normal, None);
        assert_eq!(cloud, original);
    }
    #[test]
    fn normal_arrows_are_bounded_projected_and_independent_of_large_origin() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.));
        let view = View {
            yaw: 0.,
            pitch: 0.,
            zoom: 1.,
            pan: egui::Vec2::ZERO,
        };
        let preview = Preview {
            points: (0..5001)
                .map(|index| DisplayPoint {
                    normal: Some([1., 0., 0.]),
                    position: [1e300; 3],
                    color: [1, 2, 3],
                    selected: false,
                    index,
                })
                .collect(),
            bounds: Some(([0.; 3], [2e300; 3])),
            total: 5001,
            selected: 0,
            normals: 5001,
        };
        let arrows = normal_arrows(&preview, rect, &view, 24.);
        assert!(!arrows.is_empty());
        assert!(arrows.len() <= 2000);
        assert!(
            arrows
                .iter()
                .all(|(_, v)| (*v - egui::vec2(24., 0.)).length() < 1e-5)
        );
        assert!(normal_arrows(&preview, rect, &view, f32::NAN).is_empty());
        let away = View {
            yaw: std::f32::consts::FRAC_PI_2,
            ..view
        };
        assert!(normal_arrows(&preview, rect, &away, 24.).is_empty());
    }
    fn run_draw(preview: &Preview, pointer: egui::Pos2) -> (Option<usize>, egui::FullOutput) {
        let context = egui::Context::default();
        let mut picked = None;
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.));
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Middle,
                    egui::Id::new("test"),
                ));
                picked = draw(
                    preview,
                    &painter,
                    rect,
                    View {
                        yaw: 0.,
                        pitch: 0.,
                        zoom: 1.,
                        pan: egui::Vec2::ZERO,
                    },
                    Some(pointer),
                    Some(24.),
                );
            },
        );
        (picked, output)
    }
    #[test]
    fn reconstruction_and_import_wait_for_unsaved_document_choice() {
        let mut ui = CloudUi::default();
        let cloud = scancraft_geometry::Cloud {
            points: vec![scancraft_geometry::Point {
                position: [1., 2., 3.],
                color: None,
                normal: None,
            }],
        };
        ui.document.as_mut().unwrap().editor =
            Some(scancraft_geometry::Editor::new(cloud.clone(), 8).unwrap());
        ui.document.as_mut().unwrap().selection = vec![0];
        ui.dirty = true;
        ui.path = "original.orproj".into();
        let missing = PathBuf::from("missing-calibration");
        ui.submit(Operation::Reconstruct {
            inputs: scancraft_workflow::FileInputs {
                calibration: missing.clone(),
                left: missing.clone(),
                right: missing.clone(),
                settings: missing.clone(),
            },
            output: missing,
        });
        assert!(!ui.is_busy());
        assert!(matches!(
            ui.pending_import,
            Some(Operation::Reconstruct { .. })
        ));
        assert_eq!(
            ui.document.as_ref().unwrap().editor.as_ref().unwrap().cloud,
            cloud
        );
        assert_eq!(ui.document.as_ref().unwrap().selection, [0]);
        assert!(ui.dirty);
        assert_eq!(ui.path, "original.orproj");
        ui.pending_import = None;
        ui.submit(Operation::ReconstructRecordings {
            inputs: scancraft_workflow::recording_reconstruction::Inputs {
                calibration: "missing".into(),
                left: "left".into(),
                left_index: 0,
                right: "right".into(),
                right_index: 0,
                settings: "settings".into(),
            },
            output: "out".into(),
        });
        assert!(!ui.is_busy());
        assert!(matches!(
            ui.pending_import,
            Some(Operation::ReconstructRecordings { .. })
        ));
        assert_eq!(
            ui.document.as_ref().unwrap().editor.as_ref().unwrap().cloud,
            cloud
        );
        ui.pending_import = None;
        ui.submit(Operation::ReconstructRecordedSequence {
            inputs: scancraft_workflow::recording_reconstruction::Inputs {
                calibration: "missing".into(),
                left: "left".into(),
                right: "right".into(),
                settings: "recon.json".into(),
                left_index: 0,
                right_index: 0,
            },
            settings: "sequence.json".into(),
            output: "result".into(),
        });
        assert!(!ui.is_busy());
        assert!(matches!(
            ui.pending_import,
            Some(Operation::ReconstructRecordedSequence { .. })
        ));
        assert_eq!(
            ui.document.as_ref().unwrap().editor.as_ref().unwrap().cloud,
            cloud
        );
        assert_eq!(ui.document.as_ref().unwrap().selection, [0]);
        assert!(ui.dirty);
        ui.pending_import = None;
        ui.submit(Operation::FuseSequence {
            settings: "sequence.json".into(),
            output: "result".into(),
        });
        assert!(!ui.is_busy());
        assert!(matches!(
            ui.pending_import,
            Some(Operation::FuseSequence { .. })
        ));
        assert_eq!(
            ui.document.as_ref().unwrap().editor.as_ref().unwrap().cloud,
            cloud
        );
        assert!(ui.dirty);
        ui.pending_import = None;
        ui.submit(Operation::Load {
            path: "another.orproj".into(),
            unit_scale: None,
        });
        assert!(!ui.is_busy());
        assert!(matches!(ui.pending_import, Some(Operation::Load { .. })));
        assert_eq!(
            ui.document.as_ref().unwrap().editor.as_ref().unwrap().cloud,
            cloud
        );
    }
    #[test]
    fn point_picking_returns_frontmost_original_index() {
        let preview = Preview {
            normals: 2,
            points: vec![
                DisplayPoint {
                    normal: Some([1., 0., 0.]),
                    position: [0., 0., -1.],
                    color: [1, 2, 3],
                    selected: false,
                    index: 100,
                },
                DisplayPoint {
                    normal: Some([1., 0., 0.]),
                    position: [0., 0., 1.],
                    color: [1, 2, 3],
                    selected: false,
                    index: 900,
                },
            ],
            bounds: Some(([0., 0., -1.], [0., 0., 1.])),
            total: 1000,
            selected: 0,
        };
        assert_eq!(run_draw(&preview, egui::pos2(400., 300.)).0, Some(900));
        assert_eq!(run_draw(&preview, egui::pos2(10., 10.)).0, None);
    }
    #[test]
    fn extreme_finite_coordinates_render_without_overflow() {
        let preview = Preview {
            normals: 0,
            points: vec![
                DisplayPoint {
                    normal: None,
                    position: [-1e308, 0., 0.],
                    color: [1, 2, 3],
                    selected: false,
                    index: 0,
                },
                DisplayPoint {
                    normal: None,
                    position: [1e308, 0., 0.],
                    color: [1, 2, 3],
                    selected: false,
                    index: 1,
                },
            ],
            bounds: Some(([-1e308, 0., 0.], [1e308, 0., 0.])),
            total: 2,
            selected: 0,
        };
        let (_, output) = run_draw(&preview, egui::pos2(400., 300.));
        let circles: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| {
                if let egui::Shape::Circle(c) = &s.shape {
                    Some(c)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(circles.len(), 2);
        assert!(
            circles
                .iter()
                .all(|c| c.center.x.is_finite() && c.center.y.is_finite())
        );
    }
    #[test]
    fn completed_live_scan_import_keeps_unsaved_document_until_explicit_decision() {
        let mut ui = CloudUi::default();
        let cloud = scancraft_geometry::Cloud {
            points: vec![scancraft_geometry::Point {
                position: [1., 2., 3.],
                color: None,
                normal: None,
            }],
        };
        ui.document.as_mut().unwrap().editor =
            Some(scancraft_geometry::Editor::new(cloud.clone(), 32).unwrap());
        ui.dirty = true;
        assert!(ui.request_open("completed-scan/project.orproj".into()));
        assert!(matches!(ui.pending_import, Some(Operation::Load { .. })));
        assert!(ui.job.is_none());
        assert!(ui.dirty);
        assert_eq!(
            ui.document.as_ref().unwrap().editor.as_ref().unwrap().cloud,
            cloud
        );
    }
}
