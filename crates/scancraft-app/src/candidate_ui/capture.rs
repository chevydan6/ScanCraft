//! GUI-owned command/status state; the helper owns scanner handles and cleanup.
use crate::{
    capture_worker::Worker,
    laser_display::{Origin, Snapshot},
    laser_process::{self, Completion, Control},
};
use eframe::egui;
use std::{
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};
struct Job {
    worker: Worker<Completion>,
    control: Arc<Control>,
    target: PathBuf,
    received: bool,
}
#[derive(Default)]
pub(super) struct Capture {
    files: [String; 3],
    job: Option<Job>,
    status: String,
}
pub(super) struct Update {
    pub snapshot: Option<Snapshot>,
    pub first: bool,
    pub target: PathBuf,
    pub completion: Option<Result<Completion, String>>,
    pub parent_omissions: u64,
}
impl Capture {
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn request_stop(&self) {
        if let Some(j) = &self.job {
            j.control.stop.store(true, Ordering::Relaxed);
        }
    }
    pub fn abort(&self) {
        if let Some(j) = &self.job {
            j.control.work.cancel();
            j.control.stop.store(true, Ordering::Relaxed);
        }
    }
    pub fn start(&mut self, paths: Vec<PathBuf>, origin: Origin) -> Result<(), String> {
        if self.is_busy() {
            return Err("a laser capture is already pending".into());
        }
        let expected = if origin == Origin::Native { 3 } else { 5 };
        if paths.len() != expected {
            return Err("invalid laser capture path count".into());
        }
        let target = paths.last().unwrap().clone();
        self.files = [
            paths[0].display().to_string(),
            paths[1].display().to_string(),
            target.display().to_string(),
        ];
        let control = Arc::new(Control::new(origin));
        let remote = control.clone();
        let worker = Worker::spawn(move || laser_process::run(&paths, origin, &remote))
            .map_err(|e| e.to_string())?;
        self.job = Some(Job {
            worker,
            control,
            target,
            received: false,
        });
        self.status = "Starting acquisition helper…".into();
        Ok(())
    }
    pub fn tick(&mut self) -> Option<Update> {
        let job = self.job.as_mut()?;
        let mut snapshot = job.control.take();
        let completion = job.worker.poll();
        // A final preview may arrive between the first mailbox read and worker
        // completion. Drain again before dropping the last owning controller.
        if completion.is_some()
            && let Some(last) = job.control.take()
        {
            snapshot = Some(last);
        }
        let first = !job.received && snapshot.is_some();
        job.received |= snapshot.is_some();
        self.status = job.control.status();
        let update = Update {
            snapshot,
            first,
            target: job.target.clone(),
            completion,
            parent_omissions: job.control.parent_omissions.load(Ordering::Relaxed),
        };
        if update.completion.is_some() {
            self.job = None;
        }
        Some(update)
    }
    #[cfg(test)]
    pub(super) fn test_job(
        &mut self,
        run: impl FnOnce(Arc<Control>) -> Result<Completion, String> + Send + 'static,
    ) -> Arc<Control> {
        let control = Arc::new(Control::new(Origin::RecordingReplay));
        let remote = control.clone();
        self.job = Some(Job {
            worker: Worker::spawn(move || run(remote)).unwrap(),
            control: control.clone(),
            target: PathBuf::from("synthetic replay"),
            received: false,
        });
        control
    }
    pub fn show(&mut self, ui: &mut egui::Ui, can_start: bool) -> Option<Vec<PathBuf>> {
        let mut start = false;
        egui::CollapsingHeader::new("Native laser capture")
            .default_open(self.is_busy())
            .show(ui, |ui| {
                ui.small("Raw recording and single-pair diagnostic preview. Illumination follows the explicit capture request. Tracking, fusion and physical accuracy remain unqualified.");
                ui.add_enabled_ui(can_start && !self.is_busy(), |ui| {
                    egui::Grid::new("native-laser-files").num_columns(2).show(ui, |ui| {
                        for (label, value) in ["Camera model (JSON)", "Capture request (JSON)", "New capture folder"].into_iter().zip(&mut self.files) {
                            ui.label(label);
                            ui.add(egui::TextEdit::singleline(value).desired_width(520.));
                            ui.end_row();
                        }
                    });
                    start = ui.add_enabled(self.files.iter().all(|p| !p.trim().is_empty()), egui::Button::new("Start native laser capture")).clicked();
                });
                if let Some(job) = &self.job {
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!job.control.stop.load(Ordering::Relaxed), egui::Button::new("Stop and save capture")).clicked() {
                            self.request_stop();
                        }
                        if ui.button("Abort processing (keep partial data)").clicked() {
                            self.abort();
                        }
                        ui.spinner();
                    });
                    ui.label(&self.status);
                }
            });
        start.then(|| self.files.iter().map(|s| PathBuf::from(s.trim())).collect())
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        if let Some(mut j) = self.job.take() {
            j.control.teardown.store(true, Ordering::Relaxed);
            j.control.work.cancel();
            j.control.stop.store(true, Ordering::Relaxed);
            let _ = j.worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn final_preview_and_completion_are_retained_without_polling_every_frame() {
        let mut capture = Capture::default();
        let (ready, wait) = std::sync::mpsc::channel();
        let control = capture.test_job(move |c| {
            c.offer(crate::laser_display::tests::snapshot(
                Origin::RecordingReplay,
                0,
            ))?;
            c.offer(crate::laser_display::tests::snapshot(
                Origin::RecordingReplay,
                1,
            ))?;
            ready.send(()).unwrap();
            Ok(Completion {
                schema_version: 1,
                origin: Origin::RecordingReplay,
                recorded: [2; 2],
                drops: [0; 2],
                processed: 2,
                failed: 0,
                pending: [0; 2],
                helper_omissions: 0,
            })
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        wait.recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let mut snapshot = None;
        let mut first = false;
        let update = loop {
            let mut u = capture.tick().unwrap();
            if u.snapshot.is_some() {
                snapshot = u.snapshot.take();
                first |= u.first;
            }
            if u.completion.is_some() {
                break u;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        };
        assert!(first);
        assert_eq!(snapshot.unwrap().indices, [1; 2]);
        assert!(update.completion.unwrap().is_ok());
        assert_eq!(update.parent_omissions, 1);
        assert!(!capture.is_busy());
        assert!(control.take().is_none());
    }
    #[test]
    fn native_form_dispatch_requires_explicit_start_and_complete_paths() {
        let mut capture = Capture::default();
        capture.files = [
            "model.json".into(),
            "request.json".into(),
            "new capture".into(),
        ];
        let context = egui::Context::default();
        let mut dispatched = None;
        let draw = |capture: &mut Capture,
                    time: f64,
                    click: Option<egui::Pos2>,
                    enabled: bool,
                    dispatched: &mut Option<Vec<PathBuf>>| {
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
                egui::CentralPanel::default().show(ctx, |ui| {
                    *dispatched = capture.show(ui, enabled);
                });
            })
        };
        let pos = |out: &egui::FullOutput, label: &str| {
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::epaint::Shape::Text(t) if t.galley.job.text == label => {
                        Some(t.pos + t.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .unwrap()
        };
        let out = draw(&mut capture, 0., None, true, &mut dispatched);
        assert!(dispatched.is_none());
        let _ = draw(
            &mut capture,
            1.,
            Some(pos(&out, "Native laser capture")),
            true,
            &mut dispatched,
        );
        let out = draw(&mut capture, 2., None, true, &mut dispatched);
        let button = pos(&out, "Start native laser capture");
        let _ = draw(&mut capture, 3., Some(button), false, &mut dispatched);
        assert!(dispatched.is_none());
        let out = draw(&mut capture, 4., None, true, &mut dispatched);
        let button = pos(&out, "Start native laser capture");
        let _ = draw(&mut capture, 5., Some(button), true, &mut dispatched);
        assert_eq!(
            dispatched.unwrap(),
            vec![
                PathBuf::from("model.json"),
                PathBuf::from("request.json"),
                PathBuf::from("new capture")
            ]
        );
        assert!(!capture.is_busy());
    }
}
