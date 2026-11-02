//! Read-only diagnostic viewport. Uses the shared worker and renderer, not Editor.
use crate::{
    cloud_jobs::{Document, Job, Operation, Preview},
    cloud_ui::{self, View},
};
use eframe::egui;
use scancraft_workflow::candidate_bundle::FrameView;
use std::path::PathBuf;

pub struct CandidateUi {
    path: String,
    loaded_path: String,
    job: Option<Job>,
    preview: Preview,
    view: View,
    message: String,
    frames: Option<[FrameView; 2]>,
    cameras: [Option<egui::TextureHandle>; 2],
    stretch_display: bool,
}

impl Default for CandidateUi {
    fn default() -> Self {
        Self {
            path: String::new(),
            loaded_path: String::new(),
            job: None,
            preview: Preview::default(),
            view: View {
                yaw: 0.6,
                pitch: -0.3,
                zoom: 1.,
                pan: egui::Vec2::ZERO,
            },
            message: "Open a recorded laser-matching result folder.".into(),
            frames: None,
            cameras: Default::default(),
            stretch_display: false,
        }
    }
}
impl CandidateUi {
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn cancel(&self) {
        if let Some(job) = &self.job {
            job.control.cancel();
        }
    }
    pub fn open(&mut self, path: PathBuf) {
        if self.is_busy() {
            return;
        }
        self.path = path.display().to_string();
        self.message = "Checking candidate artifacts and retained frames…".into();
        self.job = Some(Job::start(
            Document::default(),
            Operation::ViewCandidates(path),
        ));
    }
    pub fn tick(&mut self) {
        let Some(output) = self.job.as_mut().and_then(Job::poll) else {
            return;
        };
        self.job = None;
        match output.result {
            Ok(report) => {
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
                self.loaded_path = self.path.clone();
            }
            Err(error) => self.message = format!("Could not open candidates: {error}"),
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Laser candidates");
        ui.label("Diagnostic coordinates · model units and physical accuracy unverified");
        ui.horizontal(|ui| {
            ui.label("Result folder:");
            ui.add_enabled(
                !self.is_busy(),
                egui::TextEdit::singleline(&mut self.path).desired_width(520.),
            );
            if ui
                .add_enabled(!self.is_busy(), egui::Button::new("Open candidates"))
                .clicked()
            {
                self.open(self.path.clone().into());
            }
            if ui
                .add_enabled(self.is_busy(), egui::Button::new("Cancel loading"))
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
        if self.is_busy() {
            ui.spinner();
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
            loaded_path: "prior bundle".into(),
            preview: Preview {
                total: 1,
                bounds: Some(([1., 2., 3.], [1., 2., 3.])),
                ..Preview::default()
            },
            frames: Some([frame(0), frame(2)]),
            ..CandidateUi::default()
        };
        ui.open(std::env::temp_dir().join(format!(
            "scancraft-missing-candidate-{}",
            std::process::id()
        )));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while ui.is_busy() && std::time::Instant::now() < deadline {
            ui.tick();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!ui.is_busy());
        assert!(ui.message.contains("Could not open candidates"));
        assert_eq!(ui.preview.total, 1);
        assert_eq!(ui.loaded_path, "prior bundle");
        assert_eq!(ui.frames.as_ref().unwrap()[1].interface, 2);
        assert_eq!(ui.frames.as_ref().unwrap()[0].pixels, [0, 30, 60, 255]);
    }
    #[test]
    fn diagnostic_panel_renders_without_an_editor_or_metric_actions() {
        let mut view = CandidateUi::default();
        let context = egui::Context::default();
        let output = context.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| view.ui(ui));
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
                    egui::CentralPanel::default().show(ctx, |ui| view.ui(ui));
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
}
