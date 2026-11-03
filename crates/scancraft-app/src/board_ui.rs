//! Offline board preparation. Published observations and optional display are separate outcomes.
use eframe::egui;
use scancraft_geometry::work::WorkControl;
use scancraft_workflow::{calibration_observations, candidate_bundle::FrameView};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread::{self, JoinHandle},
};
mod candidate;

struct Processed {
    path: PathBuf,
    measurements: Value,
    frames: Result<[FrameView; 2], String>,
}
fn process(input: &Path, output: &Path, work: &WorkControl) -> Result<Processed, String> {
    process_with(input, output, work, |completed, work| {
        let [left, right] = &completed.frames;
        Ok([
            FrameView::from_frame(&left.profile, &left.frame, work).map_err(|e| e.to_string())?,
            FrameView::from_frame(&right.profile, &right.frame, work).map_err(|e| e.to_string())?,
        ])
    })
}
fn process_with(
    input: &Path,
    output: &Path,
    work: &WorkControl,
    preview: impl FnOnce(
        &calibration_observations::Completed,
        &WorkControl,
    ) -> Result<[FrameView; 2], String>,
) -> Result<Processed, String> {
    let completed = calibration_observations::run_with_frames(input, output, work, &|| false)
        .map_err(|e| e.to_string())?;
    let frames = preview(&completed, work);
    Ok(Processed {
        path: output.into(),
        measurements: completed.measurements,
        frames,
    })
}
struct Job {
    work: Arc<WorkControl>,
    thread: JoinHandle<Result<Processed, String>>,
}
#[derive(Default)]
pub struct BoardUi {
    candidate: candidate::CandidateUi,
    candidate_active: bool,
    request: String,
    output: String,
    job: Option<Job>,
    message: String,
    displayed_path: Option<PathBuf>,
    measurements: Option<Value>,
    frames: Option<[FrameView; 2]>,
    textures: [Option<egui::TextureHandle>; 2],
    stretch: bool,
    markers: bool,
}
impl BoardUi {
    pub fn is_busy(&self) -> bool {
        self.job.is_some() || self.candidate.is_busy()
    }
    pub fn cancel(&self) {
        self.candidate.cancel();
        if let Some(job) = &self.job {
            job.work.cancel();
        }
    }
    pub fn start(&mut self, request: PathBuf, output: PathBuf) {
        if self.is_busy() {
            return;
        }
        self.candidate_active = false;
        self.request = request.display().to_string();
        self.output = output.display().to_string();
        self.message = "Reading recordings and measuring target marks…".into();
        self.spawn(move |work| process(&request, &output, work));
    }
    pub fn start_candidate(&mut self, request: PathBuf, output: PathBuf) {
        if !self.is_busy() {
            self.candidate_active = true;
            self.candidate.start(request, output);
        }
    }
    fn spawn(
        &mut self,
        run: impl FnOnce(&WorkControl) -> Result<Processed, String> + Send + 'static,
    ) {
        let work = Arc::new(WorkControl::default());
        let worker = work.clone();
        match thread::Builder::new()
            .name("scancraft-board-observations".into())
            .spawn(move || run(&worker))
        {
            Ok(thread) => self.job = Some(Job { work, thread }),
            Err(e) => self.message = format!("Could not start board processing: {e}"),
        }
    }
    pub fn tick(&mut self) {
        self.candidate.tick();
        if !self.job.as_ref().is_some_and(|j| j.thread.is_finished()) {
            return;
        }
        let job = self.job.take().unwrap();
        match job.thread.join() {
            Ok(Ok(result))=>self.apply(result),
            Ok(Err(error))=>self.message=format!("Board processing failed: {error}. Previous view retained."),
            Err(_)=>self.message="Board worker panicked; publication status unknown. Check the requested output and partial folders. Previous view retained.".into(),
        }
    }
    fn apply(&mut self, result: Processed) {
        self.message = format!("Saved observations to {}.", result.path.display());
        match result.frames {
            Ok(frames)=>{
                self.frames=Some(frames);self.textures=Default::default();
                self.measurements=Some(result.measurements);self.displayed_path=Some(result.path);
                self.markers=true;
            }
            Err(error)=>self.message.push_str(&format!(" Preview unavailable: {error}. Previous view retained; saved files remain available.")),
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, can_start: bool, can_use: bool) -> Option<PathBuf> {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.candidate_active, false, "Recorded board");
            ui.selectable_value(&mut self.candidate_active, true, "Fit calibration");
        });
        if self.candidate_active {
            self.candidate
                .ui(ui, can_start && self.job.is_none(), can_use)
        } else {
            self.observation_ui(ui, can_start);
            None
        }
    }
    fn observation_ui(&mut self, ui: &mut egui::Ui, can_start: bool) {
        ui.heading("Calibration preparation");
        ui.label("Measure markers in a recorded board pair. Supply the target layout, exact target coordinates and explicit frame selections in the request file.");
        ui.hyperlink_to("Request format and calibration steps", "https://github.com/chevydan6/ScanCraft/blob/dev/first-genuine-scan-integration/docs/RECORDED_TARGET_OBSERVATIONS.md");
        ui.add_enabled_ui(can_start && !self.is_busy(), |ui| {
            egui::Grid::new("board-request")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Observation request (JSON)");
                    ui.add(egui::TextEdit::singleline(&mut self.request).desired_width(650.));
                    ui.end_row();
                    ui.label("New result folder");
                    ui.add(egui::TextEdit::singleline(&mut self.output).desired_width(650.));
                    ui.end_row();
                });
            if ui
                .add_enabled(
                    !self.request.trim().is_empty() && !self.output.trim().is_empty(),
                    egui::Button::new("Measure recorded board"),
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
                if ui.button("Cancel board processing").clicked() {
                    job.work.cancel();
                }
            });
        }
        ui.label(&self.message);
        ui.small("This step saves image observations. Physical units, exposure synchronization and dimensional accuracy still need independent verification.");
        if let (Some(path), Some(report)) = (&self.displayed_path, &self.measurements) {
            ui.separator();
            ui.label(format!("Displayed result: {}", path.display()));
            ui.label(format!(
                "Target units: {} · Shape-retained marks: left {}, right {} · Stereo pairs {}",
                report["target_units"].as_str().unwrap_or("unknown"),
                report["views"][0].as_array().map_or(0, Vec::len),
                report["views"][1].as_array().map_or(0, Vec::len),
                report["stereo_observations"].as_array().map_or(0, Vec::len)
            ));
            ui.small(format!(
                "Sensor PTS {} · Host separation {} µs",
                report["sensor_pts"], report["host_delta_us"]
            ));
        }
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut self.stretch, "Stretch display contrast")
                .changed()
            {
                self.textures = Default::default();
            }
            ui.checkbox(&mut self.markers, "Show marker decisions");
        });
        ui.small("Green: retained ordinary mark · Yellow: other detected mark / anchor · Red: shape rejected. Labels are target indices. Image pixels and fit inputs are unchanged by display controls.");
        if let (Some(frames), Some(report)) = (&self.frames, &self.measurements) {
            ui.columns(2, |columns| {
                for (side, ui) in columns.iter_mut().enumerate() {
                    let frame = &frames[side];
                    ui.label(if side == 0 {
                        "Left · interface 0"
                    } else {
                        "Right · interface 2"
                    });
                    if self.textures[side].is_none() {
                        let lut = frame.statistics.contrast_lut();
                        let pixels = if self.stretch {
                            frame.pixels.iter().map(|v| lut[usize::from(*v)]).collect()
                        } else {
                            frame.pixels.clone()
                        };
                        self.textures[side] = Some(ui.ctx().load_texture(
                            format!("board-camera-{side}"),
                            egui::ColorImage::from_gray(frame.size, &pixels),
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    // In a justified column Response::rect can be larger than the
                    // painted image. Share an explicit aspect-correct rectangle
                    // between the texture and source-pixel marker coordinates.
                    let native = egui::vec2(frame.size[0] as f32, frame.size[1] as f32);
                    let scale = (ui.available_width() / native.x).min(390. / native.y);
                    let (image_rect, _) =
                        ui.allocate_exact_size(native * scale, egui::Sense::hover());
                    egui::Image::new(self.textures[side].as_ref().unwrap())
                        .paint_at(ui, image_rect);
                    if self.markers {
                        draw_marks(ui, image_rect, frame, report, side);
                    }
                }
            });
        }
    }
}
fn draw_marks(ui: &egui::Ui, rect: egui::Rect, frame: &FrameView, report: &Value, side: usize) {
    let stride = frame.source_size[0]
        .div_ceil(320)
        .max(frame.source_size[1].div_ceil(240))
        .max(1) as f32;
    let camera = &report["cameras"][side];
    let Some(observations) = camera["raw_observations"].as_array() else {
        return;
    };
    for (index, o) in observations.iter().enumerate() {
        let target = camera["association"]["matches"]
            .as_array()
            .and_then(|matches| {
                matches
                    .iter()
                    .find(|m| m["observation_index"].as_u64() == Some(index as u64))
            })
            .and_then(|m| m["target_index"].as_u64());
        let retained = target.is_some_and(|t| {
            report["views"][side]
                .as_array()
                .is_some_and(|v| v.iter().any(|m| m["target_index"].as_u64() == Some(t)))
        });
        let color = if retained {
            egui::Color32::GREEN
        } else if o["shape"]["rms_gate_passed"] == false {
            egui::Color32::RED
        } else {
            egui::Color32::YELLOW
        };
        if let (Some(x), Some(y)) = (o["pixel"][0].as_f64(), o["pixel"][1].as_f64()) {
            let position = rect.min
                + egui::vec2(
                    (x as f32 / stride + 0.5) / frame.size[0] as f32 * rect.width(),
                    (y as f32 / stride + 0.5) / frame.size[1] as f32 * rect.height(),
                );
            ui.painter()
                .circle_stroke(position, 4., egui::Stroke::new(1_f32, color));
            if let Some(t) = target {
                ui.painter().text(
                    position + egui::vec2(5., -4.),
                    egui::Align2::LEFT_BOTTOM,
                    t.to_string(),
                    egui::FontId::monospace(10.),
                    color,
                );
            }
        }
    }
}
impl Drop for BoardUi {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            job.work.cancel();
            let _ = job.thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_or_cancelled_preview_keeps_saved_status_and_previous_view() {
        let mut ui = BoardUi::default();
        ui.displayed_path.replace("previous".into());
        ui.apply(Processed {
            path: "saved-new".into(),
            measurements: Value::Null,
            frames: Err("cancelled after publication".into()),
        });
        assert!(ui.message.contains("Saved observations to saved-new"));
        assert!(ui.message.contains("saved files remain available"));
        assert_eq!(ui.displayed_path.as_deref(), Some(Path::new("previous")));
        ui.spawn(|_| Err("invalid recording".into()));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while ui.is_busy() {
            assert!(std::time::Instant::now() < deadline);
            ui.tick();
            std::thread::yield_now();
        }
        assert!(ui.message.contains("invalid recording"));
        assert_eq!(ui.displayed_path.as_deref(), Some(Path::new("previous")));
    }
}
