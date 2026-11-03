//! Candidate fitting is separate from selecting a calibration or starting capture.
use eframe::egui;
use scancraft_geometry::work::WorkControl;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::Arc,
    thread::{self, JoinHandle},
};

struct Completed {
    directory: PathBuf,
    manifest: Value,
}
struct Job {
    work: Arc<WorkControl>,
    thread: JoinHandle<Result<Completed, String>>,
}
#[derive(Default)]
pub(super) struct CandidateUi {
    request: String,
    output: String,
    job: Option<Job>,
    completed: Option<Completed>,
    message: String,
}
impl CandidateUi {
    pub(super) fn is_busy(&self) -> bool {
        self.job.is_some()
    }
    pub(super) fn cancel(&self) {
        if let Some(job) = &self.job {
            job.work.cancel();
        }
    }
    pub(super) fn start(&mut self, request: PathBuf, output: PathBuf) {
        if self.is_busy() {
            return;
        }
        self.request = request.display().to_string();
        self.output = output.display().to_string();
        self.message = "Fitting stereo observations…".into();
        self.spawn(move |work| {
            let manifest =
                scancraft_workflow::calibration_candidate::run(&request, &output, work, &|| false)
                    .map_err(|e| e.to_string())?;
            Ok(Completed {
                directory: output,
                manifest,
            })
        });
    }
    fn spawn(
        &mut self,
        run: impl FnOnce(&WorkControl) -> Result<Completed, String> + Send + 'static,
    ) {
        if self.is_busy() {
            return;
        }
        let work = Arc::new(WorkControl::default());
        let worker = work.clone();
        match thread::Builder::new()
            .name("scancraft-calibration-fit".into())
            .spawn(move || run(&worker))
        {
            Ok(thread) => self.job = Some(Job { work, thread }),
            Err(e) => self.message = format!("Could not start calibration fit: {e}"),
        }
    }
    pub(super) fn tick(&mut self) {
        if !self.job.as_ref().is_some_and(|j| j.thread.is_finished()) {
            return;
        }
        let retained = if self.completed.is_some() {
            "Previous candidate retained."
        } else {
            "No previous candidate is available."
        };
        match self.job.take().unwrap().thread.join() {
            Ok(Ok(result)) => {
                self.message = format!(
                    "Saved candidate to {}. It has not been selected for scanning.",
                    result.directory.display()
                );
                self.completed = Some(result);
            }
            Ok(Err(e)) => self.message = format!("Calibration fit failed: {e}. {retained}"),
            Err(_) => {
                self.message = format!(
                    "Calibration worker panicked; check the output and partial folders for publication status. {retained}"
                )
            }
        }
    }
    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        can_start: bool,
        can_use: bool,
    ) -> Option<PathBuf> {
        ui.heading("Fit calibration candidate");
        ui.label("Fit stereo observations and save a calibration for reconstruction. Supply confirmed coordinate units and the intended device identity in the request.");
        ui.hyperlink_to("Candidate request format", "https://github.com/chevydan6/ScanCraft/blob/dev/first-genuine-scan-integration/docs/CALIBRATION_CANDIDATE.md");
        let busy = self.is_busy();
        ui.add_enabled_ui(can_start && !busy, |ui| {
            egui::Grid::new("calibration-candidate-request")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Fit request (JSON)");
                    ui.add(egui::TextEdit::singleline(&mut self.request).desired_width(650.));
                    ui.end_row();
                    ui.label("New candidate folder");
                    ui.add(egui::TextEdit::singleline(&mut self.output).desired_width(650.));
                    ui.end_row();
                });
            if ui
                .add_enabled(
                    !self.request.trim().is_empty() && !self.output.trim().is_empty(),
                    egui::Button::new("Fit and save candidate"),
                )
                .clicked()
            {
                self.start(self.request.trim().into(), self.output.trim().into());
            }
        });
        if let Some(job) = &self.job {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(job.work.progress().0);
                if ui.button("Cancel calibration fit").clicked() {
                    job.work.cancel();
                }
            });
        }
        ui.label(&self.message);
        ui.small("Declared units and device identity are not independently verified. Accuracy still requires measurements of a separate object.");
        if let Some(result) = &self.completed {
            ui.separator();
            ui.label(format!("Saved candidate: {}", result.directory.display()));
            ui.label(format!(
                "Record: {} · Source: {}",
                result.manifest["record_id"].as_str().unwrap_or("unknown"),
                result.manifest["provenance"].as_str().unwrap_or("unknown")
            ));
            ui.small("Use sets the calibration file for image, recording and live reconstruction. It does not start acquisition or replace your current model.");
            if ui
                .add_enabled(
                    can_use && !self.is_busy(),
                    egui::Button::new("Use for reconstruction"),
                )
                .clicked()
            {
                return Some(result.directory.join("calibration.rapcal"));
            }
        }
        None
    }
}
impl Drop for CandidateUi {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            job.work.cancel();
            let _ = job.thread.join();
        }
    }
}

#[cfg(test)]
mod tests;
