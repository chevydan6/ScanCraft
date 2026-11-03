//! Explicit inputs for the existing bounded, recording-backed laser workflow.
use eframe::egui;
use scancraft_workflow::recorded_traces::Inputs;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct Processing {
    geometry: String,
    first: String,
    second: String,
    request: String,
    target: String,
    first_index: usize,
    second_index: usize,
}
impl Processing {
    pub(super) fn set(&mut self, inputs: &Inputs, target: &Path) {
        self.geometry = inputs.geometry.display().to_string();
        self.first = inputs.first_recording.display().to_string();
        self.second = inputs.second_recording.display().to_string();
        self.request = inputs.request.display().to_string();
        self.target = target.display().to_string();
        self.first_index = inputs.first_index;
        self.second_index = inputs.second_index;
    }
    pub(super) fn show(&mut self, ui: &mut egui::Ui, enabled: bool) -> Option<(Inputs, PathBuf)> {
        let mut start = false;
        egui::CollapsingHeader::new("Process recorded laser pair").show(ui,|ui| {
            ui.small("Select two saved recordings and explicit model/settings files. Produces diagnostic geometry with unverified units; does not activate calibration or open the scanner.");
            ui.add_enabled_ui(enabled,|ui| {
                egui::Grid::new("laser-pair-files").num_columns(2).show(ui,|ui| {
                    ui.label("Camera model (JSON)");
                    ui.add(egui::TextEdit::singleline(&mut self.geometry).desired_width(520.));ui.end_row();
                    ui.label("Camera 1 recording");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.first).desired_width(400.));
                        ui.label("Frame");ui.add(egui::DragValue::new(&mut self.first_index));
                    });ui.end_row();
                    ui.label("Camera 2 recording");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.second).desired_width(400.));
                        ui.label("Frame");ui.add(egui::DragValue::new(&mut self.second_index));
                    });ui.end_row();
                    ui.label("Laser matching settings (JSON)");
                    ui.add(egui::TextEdit::singleline(&mut self.request).desired_width(520.));ui.end_row();
                    ui.label("New result folder");
                    ui.add(egui::TextEdit::singleline(&mut self.target).desired_width(520.));ui.end_row();
                });
                ui.small("Frames are zero-based. Existing result folders are never overwritten. Errors retain any incomplete staging folder for diagnosis.");
                let ready=[&self.geometry,&self.first,&self.second,&self.request,&self.target].iter().all(|s|!s.trim().is_empty());
                start=ui.add_enabled(ready,egui::Button::new("Process pair")).clicked();
            });
        });
        start.then(|| {
            (
                Inputs {
                    geometry: self.geometry.trim().into(),
                    first_recording: self.first.trim().into(),
                    first_index: self.first_index,
                    second_recording: self.second.trim().into(),
                    second_index: self.second_index,
                    request: self.request.trim().into(),
                },
                self.target.trim().into(),
            )
        })
    }
}
