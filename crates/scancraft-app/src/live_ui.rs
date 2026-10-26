//! Live acquisition owns a separate display; existing editable documents are untouched.
use crate::{
    capture_worker::Worker,
    cloud_jobs::{Preview, build_preview},
    cloud_ui::{View, draw},
};
use eframe::egui;
use scancraft_geometry::work::WorkControl;
use scancraft_workflow::live_stereo::control::{Control, Phase, Thumbnail};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};
struct Job {
    worker: Worker<Value>,
    work: Arc<WorkControl>,
    control: Arc<Control>,
    target: PathBuf,
}
pub struct LiveUi {
    files: [String; 4],
    job: Option<Job>,
    preview: Preview,
    cameras: [Option<egui::TextureHandle>; 2],
    message: String,
    tracking: String,
    complete_project: Option<PathBuf>,
    fused: usize,
    recorded: [usize; 2],
    drops: [usize; 2],
    skipped: usize,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    pan: egui::Vec2,
    synthetic: bool,
}
impl Default for LiveUi {
    fn default() -> Self {
        Self {
            files: Default::default(),
            job: None,
            preview: Default::default(),
            cameras: Default::default(),
            message: "Choose an explicit calibration and settings to start a native scan.".into(),
            tracking: "Waiting".into(),
            complete_project: None,
            fused: 0,
            recorded: [0; 2],
            drops: [0; 2],
            skipped: 0,
            yaw: 0.,
            pitch: 0.,
            zoom: 1.,
            pan: egui::Vec2::ZERO,
            synthetic: false,
        }
    }
}
fn texture(
    slot: &mut Option<egui::TextureHandle>,
    ctx: &egui::Context,
    index: usize,
    image: Thumbnail,
) {
    let color = egui::ColorImage::from_gray([image.width, image.height], &image.pixels);
    if let Some(texture) = slot {
        texture.set(color, egui::TextureOptions::LINEAR);
    } else {
        *slot = Some(ctx.load_texture(
            format!("live-ir-{index}"),
            color,
            egui::TextureOptions::LINEAR,
        ));
    }
}
impl LiveUi {
    #[cfg(test)]
    pub(crate) fn test_job(
        &mut self,
        run: impl FnOnce(Arc<WorkControl>, Arc<Control>) -> Result<Value, String> + Send + 'static,
    ) -> Arc<Control> {
        self.spawn(PathBuf::from("unused-test-target"), true, run);
        self.job.as_ref().unwrap().control.clone()
    }
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn request_stop(&self) {
        if let Some(job) = &self.job {
            job.control.request_stop();
        }
    }
    fn abort(&self) {
        if let Some(job) = &self.job {
            job.work.cancel();
            job.control.request_stop();
        }
    }
    fn spawn(
        &mut self,
        target: PathBuf,
        synthetic: bool,
        run: impl FnOnce(Arc<WorkControl>, Arc<Control>) -> Result<Value, String> + Send + 'static,
    ) {
        if self.is_busy() {
            return;
        }
        let work = Arc::new(WorkControl::default());
        let control = Arc::new(Control::new(true));
        let w = work.clone();
        let c = control.clone();
        match Worker::spawn(move || run(w, c)) {
            Ok(worker) => {
                self.job = Some(Job {
                    worker,
                    work,
                    control,
                    target,
                });
                self.complete_project = None;
                self.preview = Preview::default();
                self.cameras = Default::default();
                self.fused = 0;
                self.recorded = [0; 2];
                self.drops = [0; 2];
                self.skipped = 0;
                self.synthetic = synthetic;
                self.tracking = "Starting".into();
                self.message = "Preparing scan…".into();
            }
            Err(error) => self.message = format!("Could not start scan: {error}"),
        }
    }
    fn start_native(&mut self) {
        let paths = self.files.clone().map(PathBuf::from);
        let target = paths[3].clone();
        self.spawn(target, false, move |work, control| {
            scancraft_acquisition::scan(&paths[0], &paths[1], &paths[2], &paths[3], &work, &control)
        });
    }
    /// Explicit diagnostic startup only; the ordinary Start button is always physical.
    pub fn synthetic_startup(&mut self, target: PathBuf) {
        let out = target.clone();
        self.spawn(target, true, move |work, control| {
            synthetic_run(&out, &work, &control)
        });
    }
    /// Returns true on a newly finished failure, so root-close intent stays cancelled.
    pub fn tick(&mut self, ctx: &egui::Context) -> bool {
        self.tick_poll(ctx, Worker::poll)
    }
    // The completion boundary is injectable for synchronized, device-free lifecycle tests.
    fn tick_poll(
        &mut self,
        ctx: &egui::Context,
        poll: impl FnOnce(&mut Worker<Value>) -> Option<Result<Value, String>>,
    ) -> bool {
        let Some(job) = &mut self.job else {
            return false;
        };
        // A completed poll joins the owner: counters and the final mailbox are stable.
        let finished = poll(&mut job.worker);
        self.recorded = job.control.recorded_frames();
        self.drops = job.control.processing_drops();
        self.skipped = job.control.skipped_previews();
        if let Some(snapshot) = job.control.take_preview() {
            self.fused = snapshot.fused_points;
            self.tracking = snapshot.tracking["state"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string();
            // This is bounded display preparation (<=50000 points), not acquisition or reconstruction.
            match build_preview(&snapshot.cloud, &[], &WorkControl::default()) {
                Ok(preview) => self.preview = preview,
                Err(error) => self.message = format!("Preview unavailable: {error}"),
            }
            for (index, image) in snapshot.cameras.into_iter().enumerate() {
                texture(&mut self.cameras[index], ctx, index, image);
            }
        }
        if let Some(result) = finished {
            let failed = result.is_err();
            self.message = match result {
                Ok(report) => {
                    self.fused = report["fused_points"].as_u64().unwrap_or(0) as usize;
                    if report["geometry_exported"] == true {
                        self.complete_project = Some(job.target.join("project.orproj"));
                    }
                    format!(
                        "Scan saved to {} · {} fused points",
                        job.target.display(),
                        self.fused
                    )
                }
                Err(error) => format!("Scan failed: {error}"),
            };
            self.job = None;
            return failed;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
        false
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, can_start: bool, can_open: bool) -> Option<PathBuf> {
        let mut open = None;
        let controls_height = (ui.available_height() - 180.).max(120.);
        egui::ScrollArea::vertical().id_salt("live-scan-controls").max_height(controls_height).auto_shrink([false,true]).show(ui,|ui|{
        ui.heading("Live stereo scan");
        if self.synthetic {
            ui.colored_label(
                egui::Color32::YELLOW,
                "Synthetic pipeline test — no scanner is used.",
            );
        } else {
            ui.label("Experimental · camera assignment, synchronization and physical accuracy are not validated.");
        }
        ui.add_enabled_ui(!self.is_busy(), |ui| {
            for (label, value) in [
                "Calibration (.rapcal)",
                "Reconstruction settings (.json)",
                "Scan settings (.json)",
                "New scan folder",
            ]
            .into_iter()
            .zip(self.files.iter_mut())
            {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.add(
                        egui::TextEdit::singleline(value)
                            .desired_width(430.)
                            .char_limit(4096),
                    );
                });
            }
        });
        ui.horizontal(|ui| {
            let ready =
                can_start && !self.is_busy() && self.files.iter().all(|p| !p.trim().is_empty());
            if ui
                .add_enabled(ready, egui::Button::new("Start native scan"))
                .clicked()
            {
                self.start_native();
            }
            if let Some(job) = &self.job {
                let phase = job.control.phase();
                let finishing = matches!(
                    phase,
                    Phase::Stopping | Phase::Finishing | Phase::Complete | Phase::Failed
                );
                let paused = job.control.pause_requested();
                if ui
                    .add_enabled(
                        !finishing,
                        egui::Button::new(if paused {
                            "Resume reconstruction"
                        } else {
                            "Pause reconstruction"
                        }),
                    )
                    .clicked()
                {
                    let _ = job.control.set_paused(!paused);
                }
                if ui
                    .add_enabled(!finishing, egui::Button::new("Stop and save"))
                    .clicked()
                {
                    job.control.request_stop();
                }
                if ui.button("Abort (keep partial data)").clicked() {
                    self.abort();
                }
                ui.label(match phase{Phase::Initializing=>"Preparing",Phase::Running=>"Capturing and reconstructing",Phase::Paused=>"Paused · recording",Phase::Stopping=>"Stopping capture",Phase::Finishing|Phase::Complete=>"Finalizing scan",Phase::Failed=>"Finalizing after error"});
            }
        });
        ui.small("While reconstruction is paused, raw recording continues and the configured frame limit still applies.");
        ui.label(format!(
            "Raw captured L/R: {}/{} · Processing drops: {}/{} · Display previews skipped: {}",
            self.recorded[0], self.recorded[1], self.drops[0], self.drops[1], self.skipped
        ));
        ui.label(format!(
            "Tracking: {} · {} fused points · {} displayed",
            self.tracking,
            self.fused,
            self.preview.points.len()
        ));
        ui.label(&self.message);
        if let Some(path) = &self.complete_project
            && ui.add_enabled(can_open,egui::Button::new("Open completed scan in point-cloud editor")).clicked() {
            open=Some(path.clone());
        }
        ui.horizontal(|ui| {
            for (index, texture) in self.cameras.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.label(if index == 0 {
                        "Left IR (declared)"
                    } else {
                        "Right IR (declared)"
                    });
                    if let Some(texture) = texture {
                        ui.add(
                            egui::Image::new(texture)
                                .max_size(egui::vec2(320., 160.))
                                .maintain_aspect_ratio(true),
                        );
                    }
                });
            }
        });
        });
        let available = egui::vec2(
            ui.available_width().max(1.),
            ui.available_height().max(120.),
        );
        let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
        painter.rect_filled(response.rect, 0., egui::Color32::from_rgb(15, 18, 24));
        if response.dragged() {
            if ui.input(|i| i.modifiers.shift) {
                self.pan += response.drag_delta();
            } else {
                self.yaw += response.drag_delta().x * 0.01;
                self.pitch = (self.pitch + response.drag_delta().y * 0.01).clamp(-1.55, 1.55);
            }
        }
        if response.hovered() {
            self.zoom = (self.zoom * (ui.input(|i| i.smooth_scroll_delta.y) * 0.003).exp())
                .clamp(0.01, 1000.);
        }
        draw(
            &self.preview,
            &painter,
            response.rect,
            View {
                yaw: self.yaw,
                pitch: self.pitch,
                zoom: self.zoom,
                pan: self.pan,
            },
            None,
            None,
        );
        open
    }
}
impl Drop for LiveUi {
    fn drop(&mut self) {
        if let Some(job) = &mut self.job {
            job.work.cancel();
            job.control.request_stop();
            let _ = job.worker.join();
        }
    }
}

// Device-free, explicitly labelled startup source for reproducible GUI validation.
fn synthetic_run(
    target: &std::path::Path,
    work: &WorkControl,
    control: &Arc<Control>,
) -> Result<Value, String> {
    synthetic_run_limited(target, work, control, 100)
}
fn synthetic_run_limited(
    target: &std::path::Path,
    work: &WorkControl,
    control: &Arc<Control>,
    maximum_frames: u32,
) -> Result<Value, String> {
    use scancraft_core::{Frame, StreamProfile};
    use scancraft_recording::CaptureContext;
    use scancraft_workflow::live_stereo::{
        InputMode, Processor, Side,
        runner::{self, Source},
    };
    use serde_json::json;
    struct Fixture {
        index: u32,
        maximum_frames: u32,
        side: Side,
        pixels: Vec<u8>,
    }
    impl Source for Fixture {
        fn context(&self) -> Result<CaptureContext, String> {
            Ok(CaptureContext {
                controls: json!({"synthetic":true}),
                calibration_status: "synthetic_fixture_not_a_scanner_capture".into(),
                ..Default::default()
            })
        }
        fn next(&mut self) -> Result<Option<Frame>, String> {
            if self.index == self.maximum_frames {
                return Ok(None);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
            let index = self.index;
            self.index += 1;
            let pts = 1000 + index * 33333;
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&pts.to_le_bytes());
            Ok(Some(Frame {
                sequence: index,
                host_timestamp_us: u64::from(pts) + self.side.index() as u64 * 10,
                flags: 0,
                metadata,
                data: self.pixels.clone(),
            }))
        }
        fn statistics(&self) -> Value {
            json!({"synthetic":true,"hardware_accessed":false})
        }
        fn close(self) -> Result<(), String> {
            Ok(())
        }
    }
    struct Temporary(PathBuf);
    impl Drop for Temporary {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let temp = Temporary(
        scancraft_workflow::output::staging(
            &std::env::temp_dir().join("scancraft-live-gui-fixture"),
        )
        .map_err(|e| e.to_string())?,
    );
    let settings = temp.0.join("scan.json");
    std::fs::write(&settings,serde_json::to_vec(&json!({"schema_version":1,"session_id":"synthetic-gui","fps":30,"maximum_frames_per_camera":maximum_frames,"maximum_host_delta_us":1000,
        "tracking_samples":32,"voxel_mm":0.1,"maximum_voxels":1000,"tracking":{"max_correspondence_mm":10.0,"max_rms_mm":0.01,"minimum_overlap":0.8,"max_step_translation_mm":1.0,"max_step_rotation_rad":0.1,"iterations":40,"tolerance_mm":1e-8}})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
    let mut processor = Processor::open(
        &root.join("calibration.rapcal"),
        &root.join("request.json"),
        &settings,
        InputMode::SyntheticFixture,
        work,
    )
    .map_err(|e| e.to_string())?;
    runner::bundle_controlled(
        &mut processor,
        target,
        work,
        move |side, _profile: StreamProfile| {
            let pixels = image::open(root.join(if side.index() == 0 {
                "left.png"
            } else {
                "right.png"
            }))
            .map_err(|e| e.to_string())?
            .to_luma8()
            .into_raw();
            Ok(Fixture {
                index: 0,
                maximum_frames,
                side,
                pixels,
            })
        },
        || Ok(()),
        control,
    )
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    #[test]
    fn completion_refreshes_counters_and_final_preview_after_poll_boundary() {
        let root = scancraft_workflow::output::staging(
            &std::env::temp_dir().join("live-ui-completion-regression"),
        )
        .unwrap();
        let target = root.join("scan");
        let out = target.clone();
        let (release, wait) = std::sync::mpsc::channel();
        let mut ui = LiveUi::default();
        ui.spawn(target, true, move |work, control| {
            wait.recv().map_err(|e| e.to_string())?;
            synthetic_run_limited(&out, &work, &control, 1)
        });
        assert_eq!(ui.job.as_ref().unwrap().control.recorded_frames(), [0, 0]);
        let failed = ui.tick_poll(&egui::Context::default(), |worker| {
            // Producer cannot update anything before the consumer reaches poll.
            release.send(()).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                if let Some(result) = worker.poll() {
                    break Some(result);
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            }
        });
        assert!(!failed);
        assert_eq!(ui.recorded, [1, 1]);
        assert!(ui.job.is_none());
        assert!(ui.complete_project.is_some());
        assert_eq!(ui.fused, 3);
        assert_eq!(ui.preview.total, 3);
        assert_eq!(ui.preview.points.len(), 3);
        assert_eq!(ui.tracking, "Tracking");
        assert!(ui.cameras.iter().all(Option::is_some));
        std::fs::remove_dir_all(root).unwrap();
    }
}
