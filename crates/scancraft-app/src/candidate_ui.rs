//! Read-only diagnostic viewport. Uses the shared worker and renderer, not Editor.
use crate::{
    cloud_jobs::{Document, Job, Operation, Preview},
    cloud_ui::{self, View},
};
use eframe::egui;
use std::path::PathBuf;

pub struct CandidateUi {
    path: String,
    loaded_path: String,
    job: Option<Job>,
    preview: Preview,
    view: View,
    message: String,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_open_preserves_the_prior_view_and_its_source_label() {
        let mut ui = CandidateUi {
            loaded_path: "prior bundle".into(),
            preview: Preview {
                total: 1,
                bounds: Some(([1., 2., 3.], [1., 2., 3.])),
                ..Preview::default()
            },
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
}
