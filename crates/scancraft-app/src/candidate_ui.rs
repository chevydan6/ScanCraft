//! Read-only diagnostic viewport. Uses the shared worker and renderer, not Editor.
mod capture;
mod processing;
use crate::{
    cloud_jobs::{Document, Job, Operation, Preview},
    cloud_ui::{self, View},
};
use eframe::egui;
use scancraft_workflow::candidate_bundle::FrameView;
use std::path::PathBuf;

enum Action {
    Open,
    Fused,
    Process,
}

pub struct CandidateUi {
    path: String,
    pair_number: u64,
    loaded_path: String,
    job: Option<Job>,
    preview: Preview,
    view: View,
    message: String,
    frames: Option<[FrameView; 2]>,
    session: Option<scancraft_workflow::tracked_candidates::bundle::SessionView>,
    cameras: [Option<egui::TextureHandle>; 2],
    stretch_display: bool,
    processing: processing::Processing,
    action: Action,
    capture: capture::Capture,
}

impl Default for CandidateUi {
    fn default() -> Self {
        Self {
            path: String::new(),
            pair_number: 1,
            loaded_path: String::new(),
            job: None,
            preview: Preview::default(),
            view: View {
                yaw: 0.6,
                pitch: -0.3,
                zoom: 1.,
                pan: egui::Vec2::ZERO,
            },
            message: "Open a saved capture, recorded-pair result or fused scan folder.".into(),
            frames: None,
            session: None,
            cameras: Default::default(),
            stretch_display: false,
            processing: Default::default(),
            action: Action::Open,
            capture: Default::default(),
        }
    }
}
impl CandidateUi {
    #[cfg(test)]
    pub(crate) fn test_capture(
        &mut self,
        run: impl FnOnce(
            std::sync::Arc<crate::laser_process::Control>,
        ) -> Result<crate::laser_process::Completion, String>
        + Send
        + 'static,
    ) -> std::sync::Arc<crate::laser_process::Control> {
        self.capture.test_job(run)
    }
    pub fn is_busy(&self) -> bool {
        self.job.is_some() || self.capture.is_busy()
    }
    pub fn capture_busy(&self) -> bool {
        self.capture.is_busy()
    }
    pub fn stop_capture(&self) {
        self.capture.request_stop();
    }
    pub fn abort_capture(&self) {
        self.capture.abort();
    }
    pub fn start_capture(&mut self, paths: Vec<PathBuf>, origin: crate::laser_display::Origin) {
        if self.is_busy() {
            return;
        }
        match self.capture.start(paths, origin) {
            Ok(()) => {
                self.message =
                    "Starting laser capture; prior view retained until a new pair arrives.".into()
            }
            Err(e) => self.message = format!("Could not start laser capture: {e}"),
        }
    }
    fn tick_capture(&mut self) -> bool {
        let Some(update) = self.capture.tick() else {
            return false;
        };
        if let Some(snapshot) = update.snapshot {
            let mut preview = Preview {
                total: snapshot.positions.len(),
                ..Preview::default()
            };
            for (index, position) in snapshot.positions.into_iter().enumerate() {
                if let Some((min, max)) = &mut preview.bounds {
                    for axis in 0..3 {
                        min[axis] = min[axis].min(position[axis]);
                        max[axis] = max[axis].max(position[axis]);
                    }
                } else {
                    preview.bounds = Some((position, position));
                }
                preview.points.push(crate::cloud_jobs::DisplayPoint {
                    normal: None,
                    position,
                    color: [101, 190, 226],
                    selected: false,
                    index,
                });
            }
            self.session = None;
            self.preview = preview;
            self.frames = Some(snapshot.frames);
            self.cameras = Default::default();
            if update.first {
                self.view = View {
                    yaw: 0.6,
                    pitch: -0.3,
                    zoom: 1.,
                    pan: egui::Vec2::ZERO,
                };
            }
            let origin = if snapshot.origin == crate::laser_display::Origin::Native {
                "Native capture"
            } else {
                "Recording replay"
            };
            self.loaded_path = format!(
                "{origin}: {} · source frames {} / {}",
                update.target.display(),
                snapshot.indices[0],
                snapshot.indices[1]
            );
            self.message = if let Some(error) = snapshot.processing_error {
                format!("Pair processing failed: {error}. Raw journaling continues.")
            } else {
                format!(
                    "{} candidates · {} queries · {} numerical failures · {} without unique match",
                    self.preview.total,
                    snapshot.queries,
                    snapshot.query_failures,
                    snapshot.without_unique
                )
            };
            self.message.push_str(&format!(" · raw accepted {:?} · processing queue drops {:?} · helper/GUI preview omissions {}/{}. Capture awaits finalization.",snapshot.acquisition.journaled_frames,snapshot.acquisition.processing_queue_drops,snapshot.helper_omissions,update.parent_omissions));
        }
        if let Some(result) = update.completion {
            match result {
                Ok(c) => {
                    self.path = update.target.display().to_string();
                    self.pair_number = c.processed.max(1);
                    self.message = format!(
                        "Saved capture at {}. Raw frames {:?}; {} pairs processed, {} pair failures, processing drops {:?}, unmatched {:?}. Preview omissions helper/GUI {}/{}. Source cleanup and recording verification passed. View is a single diagnostic pair; tracking and fusion remain unavailable.",
                        update.target.display(),
                        c.recorded,
                        c.processed,
                        c.failed,
                        c.drops,
                        c.pending,
                        c.helper_omissions,
                        update.parent_omissions
                    )
                }
                Err(e) => {
                    self.message = format!(
                        "Laser capture failed: {e}. Inspect {} and any retained staging folder; prior/latest view retained.",
                        update.target.display()
                    );
                    return true;
                }
            }
        }
        false
    }
    pub fn cancel(&self) {
        self.capture.abort();
        if let Some(job) = &self.job {
            job.control.cancel();
        }
    }
    pub fn open(&mut self, path: PathBuf) {
        self.open_pair(path, 0);
    }
    pub fn open_pair(&mut self, path: PathBuf, index: u64) {
        if self.is_busy() {
            return;
        }
        self.path = path.display().to_string();
        self.pair_number = index.saturating_add(1);
        self.action = Action::Open;
        self.message = "Checking candidate artifacts and retained frames…".into();
        self.job = Some(Job::start(
            Document::default(),
            Operation::ViewCandidates { path, index },
        ));
    }
    pub fn open_fused(&mut self, path: PathBuf, index: usize) {
        if self.is_busy() {
            return;
        }
        self.path = path.display().to_string();
        self.pair_number = (index as u64).saturating_add(1);
        self.action = Action::Fused;
        self.message = "Checking fused artifacts and original source pairs…".into();
        self.job = Some(Job::start(
            Document::default(),
            Operation::ViewFusedCandidates { path, index },
        ));
    }
    pub fn process(
        &mut self,
        inputs: scancraft_workflow::recorded_traces::Inputs,
        target: PathBuf,
    ) {
        if self.is_busy() {
            return;
        }
        self.processing.set(&inputs, &target);
        self.path = target.display().to_string();
        self.action = Action::Process;
        self.message =
            "Validating recordings, detecting laser traces and matching the selected pair…".into();
        self.job = Some(Job::start(
            Document::default(),
            Operation::ProcessLaserCandidates { inputs, target },
        ));
    }
    pub fn tick(&mut self) -> bool {
        let capture_failed = self.tick_capture();
        let Some(output) = self.job.as_mut().and_then(Job::poll) else {
            return capture_failed;
        };
        self.job = None;
        match output.result {
            Ok(report) => {
                self.session = report.candidate_session;
                self.frames = report.candidate_frames;
                self.cameras = Default::default();
                if let Some(preview) = report.preview {
                    self.preview = preview;
                }
                self.view = View {
                    yaw: 0.6,
                    pitch: -0.3,
                    zoom: 1.,
                    pan: egui::Vec2::ZERO,
                };
                self.message = report.message;
                self.loaded_path = match self.action {
                    Action::Fused => format!(
                        "{} · fused scan · camera frame {}",
                        self.path, self.pair_number
                    ),
                    Action::Open => format!("{} · pair {}", self.path, self.pair_number),
                    Action::Process => self.path.clone(),
                };
            }
            Err(error) => {
                self.message = format!(
                    "{}: {error}",
                    match self.action {
                        Action::Open => "Could not open candidates",
                        Action::Fused => "Could not open fused scan",
                        Action::Process => "Recorded-pair processing failed",
                    }
                )
            }
        }
        capture_failed
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, can_capture: bool) {
        ui.heading("Laser candidates");
        ui.label("Diagnostic coordinates · model units and physical accuracy unverified");
        ui.horizontal_wrapped(|ui| {
            ui.label("Result folder:");
            ui.add_enabled(
                !self.is_busy(),
                egui::TextEdit::singleline(&mut self.path).desired_width(520.),
            );
            ui.label("Frame / pair:");
            let enabled = !self.is_busy();
            ui.add_enabled(
                enabled,
                egui::DragValue::new(&mut self.pair_number).range(1..=1_000_000),
            );
            if ui
                .add_enabled(!self.is_busy(), egui::Button::new("Open candidates"))
                .clicked()
            {
                self.open_pair(self.path.clone().into(), self.pair_number.saturating_sub(1));
            }
            if ui
                .add_enabled(!self.is_busy(), egui::Button::new("Open fused scan"))
                .clicked()
            {
                self.open_fused(
                    self.path.clone().into(),
                    self.pair_number.saturating_sub(1) as usize,
                );
            }
            if ui
                .add_enabled(self.job.is_some(), egui::Button::new("Cancel operation"))
                .clicked()
            {
                self.cancel();
            }
        });
        ui.label(&self.message);
        if !self.loaded_path.is_empty() {
            ui.label(format!("Showing: {}", self.loaded_path));
        }
        ui.small("Drag to orbit · Shift-drag to pan · Scroll to zoom. Coordinates depend on an unverified camera and laser model.");
        if let Some(job) = &self.job {
            let (stage, fraction) = job.control.progress();
            ui.add(egui::ProgressBar::new(fraction).text(format!("Current stage: {stage}")));
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(30));
        }
        let can_process = !self.is_busy();
        if let Some(paths) = self.capture.show(ui, can_capture && can_process) {
            self.start_capture(paths, crate::laser_display::Origin::Native);
        }
        if let Some((inputs, target)) = self.processing.show(ui, can_process) {
            self.process(inputs, target);
        }
        ui.horizontal(|ui| {
            if ui.button("Reset view").clicked() {
                self.view = View {
                    yaw: 0.6,
                    pitch: -0.3,
                    zoom: 1.,
                    pan: egui::Vec2::ZERO,
                };
            }
            ui.label(format!("{} candidate points", self.preview.total));
        });
        self.session_panel(ui);
        self.frame_panel(ui);
        let size = egui::vec2(
            ui.available_width().max(1.),
            ui.available_height().max(100.),
        );
        let (response, painter) = ui.allocate_painter(size, egui::Sense::drag());
        painter.rect_filled(response.rect, 0., egui::Color32::from_rgb(18, 22, 27));
        if response.dragged() {
            let delta = ui.input(|i| i.pointer.delta());
            if ui.input(|i| i.modifiers.shift) {
                self.view.pan += delta;
            } else {
                self.view.yaw += delta.x * 0.01;
                self.view.pitch = (self.view.pitch + delta.y * 0.01).clamp(-1.55, 1.55);
            }
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            self.view.zoom = (self.view.zoom * (scroll * 0.002).exp()).clamp(0.05, 100.);
        }
        cloud_ui::draw(
            &self.preview,
            &painter,
            response.rect,
            View {
                yaw: self.view.yaw,
                pitch: self.view.pitch,
                zoom: self.view.zoom,
                pan: self.view.pan,
            },
            None,
            None,
        );
    }
    fn session_panel(&self, ui: &mut egui::Ui) {
        let Some(session) = &self.session else {
            return;
        };
        ui.label(format!(
            "{} integrated · {} tracking only · {} rejected · {} unavailable",
            session.integrated, session.tracking_only, session.rejected, session.unavailable
        ));
        egui::CollapsingHeader::new("Tracking and fusion outcomes").show(ui, |ui| {
            ui.small("RMS uses unverified model units. The first accepted pose is an identity anchor, not measured accuracy. No marker points enter the surface.");
            egui::ScrollArea::vertical().id_salt("fused-frame-outcomes").max_height(110.).show(ui, |ui| {
                for (index, frame) in session.frames.iter().enumerate() {
                    ui.label(format!("{}Frame {} (id {}) · {} · {} surface / {} marker candidates · RMS {}",
                        if index == session.selected_frame { "► " } else { "" }, index+1, frame.frame_id,
                        frame.status.label(), frame.surface_points, frame.tracking_points,
                        frame.rms_model_units.map_or_else(|| "unavailable".into(), |v| format!("{v:.6}"))));
                    if let Some(reason) = &frame.reason { ui.small(reason); }
                }
            });
        });
    }
    fn frame_panel(&mut self, ui: &mut egui::Ui) {
        let Some(frames) = &self.frames else {
            return;
        };
        egui::CollapsingHeader::new("Retained camera frames")
            .default_open(true)
            .show(ui, |ui| {
                if ui.checkbox(&mut self.stretch_display, "Stretch display contrast only (p01–p99)").changed() {
                    self.cameras = Default::default();
                }
                ui.small("Original image orientation · sampled preview · statistics use every raw pixel. Image brightness alone does not establish laser visibility, focus or scan accuracy.");
                ui.columns(2, |columns| {
                    for (index, (frame, ui)) in frames.iter().zip(columns).enumerate() {
                        let texture = self.cameras[index].get_or_insert_with(|| {
                            let pixels = if self.stretch_display {
                                let lut = frame.statistics.contrast_lut();
                                frame.pixels.iter().map(|&n| lut[usize::from(n)]).collect()
                            } else { frame.pixels.clone() };
                            ui.ctx().load_texture(format!("candidate-frame-{index}"),egui::ColorImage::from_gray(frame.size,&pixels),egui::TextureOptions::NEAREST)
                        });
                        ui.label(format!("Camera {} · interface {} · {} × {}",index+1,frame.interface,frame.source_size[0],frame.source_size[1]));
                        let s = &frame.statistics;
                        ui.small(format!("Raw min {} · median {} · p99 {} · max {}",s.minimum,s.median,s.p99,s.maximum));
                        ui.small(format!("Raw 0: {} ({:.3}%) · raw 255: {} ({:.3}%)",s.black_clipped,100.*s.black_clipped as f64/s.pixels as f64,s.white_clipped,100.*s.white_clipped as f64/s.pixels as f64))
                            .on_hover_text("Exact endpoint counts, not the detector's configurable saturation threshold or a count of rejected laser runs.");
                        let scale = (ui.available_width()/frame.size[0] as f32).min(140./frame.size[1] as f32).min(1.);
                        ui.image((texture.id(),egui::vec2(frame.size[0] as f32*scale,frame.size[1] as f32*scale)));
                    }
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(interface: u8) -> FrameView {
        let pixels = vec![0, 30, 60, 255];
        FrameView {
            interface,
            source_size: [2, 2],
            size: [2, 2],
            statistics: scancraft_image::diagnostics::statistics(
                scancraft_image::GrayImage::new(2, 2, &pixels).unwrap(),
            ),
            pixels,
        }
    }
    #[test]
    fn failed_open_preserves_the_prior_view_and_its_source_label() {
        let mut ui = CandidateUi {
            loaded_path: "prior bundle · pair 3".into(),
            preview: Preview {
                total: 1,
                bounds: Some(([1., 2., 3.], [1., 2., 3.])),
                ..Preview::default()
            },
            frames: Some([frame(0), frame(2)]),
            ..CandidateUi::default()
        };
        ui.open_pair(
            std::env::temp_dir().join(format!(
                "scancraft-missing-candidate-{}",
                std::process::id()
            )),
            7,
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while ui.is_busy() && std::time::Instant::now() < deadline {
            ui.tick();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!ui.is_busy());
        assert_eq!(ui.pair_number, 8);
        assert!(ui.message.contains("Could not open candidates"));
        assert_eq!(ui.preview.total, 1);
        assert_eq!(ui.loaded_path, "prior bundle · pair 3");
        assert_eq!(ui.frames.as_ref().unwrap()[1].interface, 2);
        assert_eq!(ui.frames.as_ref().unwrap()[0].pixels, [0, 30, 60, 255]);
    }
    #[test]
    fn diagnostic_panel_renders_without_an_editor_or_metric_actions() {
        let mut view = CandidateUi::default();
        let context = egui::Context::default();
        let output = context.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| view.ui(ui, true));
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("model units and physical accuracy unverified"));
        assert!(text.contains("Open candidates"));
        for metric_action in ["Save project", "Export PLY", "Measure distance"] {
            assert!(!text.contains(metric_action));
        }
        assert!(view.preview.points.is_empty());
        assert!(!view.is_busy());
    }
    #[test]
    fn failed_laser_start_retains_prior_view_and_reports_failure_to_root_close() {
        let mut view = CandidateUi {
            loaded_path: "prior bundle".into(),
            frames: Some([frame(0), frame(2)]),
            preview: Preview {
                total: 7,
                ..Preview::default()
            },
            ..CandidateUi::default()
        };
        view.test_capture(|_| Err("preflight failed before scanner open".into()));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut failed = false;
        while view.is_busy() {
            failed |= view.tick();
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(failed);
        assert_eq!(view.loaded_path, "prior bundle");
        assert_eq!(view.preview.total, 7);
        assert_eq!(view.frames.as_ref().unwrap()[0].pixels, [0, 30, 60, 255]);
        assert!(view.message.contains("preflight failed"));
    }
    #[test]
    fn failed_processing_keeps_prior_frames_points_and_source_label() {
        let mut ui = CandidateUi {
            loaded_path: "prior bundle".into(),
            frames: Some([frame(0), frame(2)]),
            preview: Preview {
                total: 7,
                ..Preview::default()
            },
            ..CandidateUi::default()
        };
        let absent = std::env::temp_dir().join(format!(
            "scancraft-absent-laser-input-{}",
            std::process::id()
        ));
        ui.processing.set(
            &scancraft_workflow::recorded_traces::Inputs {
                geometry: absent.join("geometry.json"),
                first_recording: absent.join("first.raptor"),
                first_index: 0,
                second_recording: absent.join("second.raptor"),
                second_index: 0,
                request: absent.join("request.json"),
            },
            &absent.join("result"),
        );
        // Click the actual form header and button through egui, rather than
        // calling process directly; verify dispatch as well as failure retention.
        let context = egui::Context::default();
        let draw = |ui: &mut CandidateUi, time: f64, click: Option<egui::Pos2>| {
            let mut input = egui::RawInput {
                time: Some(time),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280., 900.),
                )),
                ..Default::default()
            };
            if let Some(pos) = click {
                input.events = vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    },
                ];
            }
            context.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |panel| ui.ui(panel, true));
            })
        };
        let text_position = |output: &egui::FullOutput, label: &str| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(t) if t.galley.job.text == label => {
                        Some(t.pos + t.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .unwrap()
        };
        let output = draw(&mut ui, 0., None);
        let header = text_position(&output, "Process recorded laser pair");
        let _ = draw(&mut ui, 1., Some(header));
        let output = draw(&mut ui, 2., None);
        let button = text_position(&output, "Process pair");
        let _ = draw(&mut ui, 3., Some(button));
        assert!(ui.is_busy());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while ui.is_busy() && std::time::Instant::now() < deadline {
            ui.tick();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!ui.is_busy());
        assert!(ui.message.contains("Recorded-pair processing failed"));
        assert_eq!(ui.loaded_path, "prior bundle");
        assert_eq!(ui.preview.total, 7);
        assert_eq!(ui.frames.as_ref().unwrap()[0].pixels, [0, 30, 60, 255]);
        assert!(!absent.join("result").exists());
    }
    #[test]
    fn camera_panel_labels_raw_statistics_and_keeps_display_stretch_out_of_frame_data() {
        let mut view = CandidateUi {
            frames: Some([frame(0), frame(2)]),
            ..CandidateUi::default()
        };
        let context = egui::Context::default();
        for stretch in [false, true] {
            view.stretch_display = stretch;
            view.cameras = Default::default();
            let output = context.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280., 900.),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| view.ui(ui, true));
                },
            );
            let text = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::epaint::Shape::Text(t) => Some(t.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(text.contains("interface 0"));
            assert!(text.contains("interface 2"));
            assert!(text.contains("Raw min 0 · median 30 · p99 255 · max 255"));
            assert!(text.contains("raw 255: 1 (25.000%)"));
            assert!(text.contains("Stretch display contrast only"));
            assert!(view.cameras.iter().all(Option::is_some));
            assert_eq!(view.frames.as_ref().unwrap()[0].pixels, [0, 30, 60, 255]);
        }
    }
    #[test]
    fn completed_capture_presets_saved_folder_and_last_display_pair() {
        let mut view = CandidateUi::default();
        view.test_capture(|_| {
            Ok(crate::laser_process::Completion {
                schema_version: 1,
                origin: crate::laser_display::Origin::RecordingReplay,
                recorded: [10; 2],
                drops: [0; 2],
                processed: 7,
                failed: 0,
                pending: [0; 2],
                helper_omissions: 0,
            })
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while view.is_busy() {
            view.tick();
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(view.path, "synthetic replay");
        assert_eq!(view.pair_number, 7);
        assert!(view.message.contains("Saved capture"));
    }
    #[test]
    fn fused_open_button_dispatches_and_keeps_the_prior_view_after_failure() {
        let mut view = CandidateUi {
            path: std::env::temp_dir()
                .join(format!("missing-fused-{}", std::process::id()))
                .display()
                .to_string(),
            loaded_path: "prior checked result".into(),
            preview: Preview {
                total: 7,
                ..Default::default()
            },
            ..Default::default()
        };
        let context = egui::Context::default();
        let draw = |view: &mut CandidateUi, time: f64, click: Option<egui::Pos2>| {
            let mut input = egui::RawInput {
                time: Some(time),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500., 1000.),
                )),
                ..Default::default()
            };
            if let Some(pos) = click {
                input.events = vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    },
                ];
            }
            context.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| view.ui(ui, true));
            })
        };
        let output = draw(&mut view, 0., None);
        let button = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) if t.galley.job.text == "Open fused scan" => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap();
        let _ = draw(&mut view, 1., Some(button));
        assert!(view.is_busy());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while view.is_busy() {
            view.tick();
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(view.message.contains("Could not open fused scan"));
        assert_eq!(view.loaded_path, "prior checked result");
        assert_eq!(view.preview.total, 7);
    }
}
