//! Offline playback jobs never acquire the scanner or write the source.
use eframe::egui;
use scancraft_geometry::work::WorkControl;
use scancraft_workflow::recording_playback::{Playback, View};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread::{self, JoinHandle},
};
struct Completed {
    playback: Playback,
    view: View,
}
struct Job {
    work: Arc<WorkControl>,
    rx: Receiver<Result<Completed, String>>,
    thread: JoinHandle<()>,
}
#[derive(Default)]
pub struct PlaybackUi {
    path: String,
    open: bool,
    job: Option<Job>,
    playback: Option<Playback>,
    texture: Option<egui::TextureHandle>,
    selected: usize,
    total: usize,
    details: String,
    message: String,
}
impl PlaybackUi {
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn cancel(&self) {
        if let Some(job) = &self.job {
            job.work.cancel();
        }
    }
    pub fn start_path(&mut self, path: PathBuf) {
        if self.is_busy() {
            return;
        }
        self.path = path.display().to_string();
        self.open = true;
        self.playback = None;
        self.total = 0;
        self.spawn(move |work| {
            let mut playback = Playback::open(&path, work).map_err(|e| e.to_string())?;
            let view = playback.read(0, work).map_err(|e| e.to_string())?;
            Ok(Completed { playback, view })
        });
    }
    fn seek(&mut self, index: usize) {
        if self.is_busy() {
            return;
        }
        if let Some(mut playback) = self.playback.take() {
            self.spawn(move |work| {
                let view = playback.read(index, work).map_err(|e| e.to_string())?;
                Ok(Completed { playback, view })
            });
        }
    }
    fn spawn(
        &mut self,
        operation: impl FnOnce(&WorkControl) -> Result<Completed, String> + Send + 'static,
    ) {
        self.texture = None;
        self.details.clear();
        self.message = "Validating and loading recording…".into();
        let work = Arc::new(WorkControl::default());
        let control = work.clone();
        let (tx, rx) = mpsc::channel();
        match thread::Builder::new()
            .name("scancraft-recording-playback".into())
            .spawn(move || {
                let _ = tx.send(operation(&control));
            }) {
            Ok(thread) => self.job = Some(Job { work, rx, thread }),
            Err(e) => self.message = format!("Cannot start playback: {e}"),
        }
    }
    pub fn tick(&mut self, ctx: &egui::Context) {
        let result = match self.job.as_ref().map(|j| j.rx.try_recv()) {
            Some(Ok(result)) => result,
            Some(Err(TryRecvError::Disconnected)) => {
                Err("playback worker stopped unexpectedly".into())
            }
            _ => return,
        };
        let job = self.job.take().unwrap();
        let joined = job.thread.join();
        if joined.is_err() {
            self.message = "Playback worker panicked; recording not written.".into();
            return;
        }
        if job.work.check().is_err() {
            self.message = "Playback cancelled; reopen to continue.".into();
            return;
        }
        match result {
            Ok(completed) => {
                let view = completed.view;
                self.selected = view.index;
                self.total = view.total;
                self.details = format!(
                    "{}\nSequence {} · host {} µs · sensor PTS {:?}",
                    view.description, view.sequence, view.host_timestamp_us, view.sensor_pts
                );
                let image = egui::ColorImage::from_rgb(
                    [view.image.width() as usize, view.image.height() as usize],
                    view.image.as_raw(),
                );
                self.texture = Some(ctx.load_texture(
                    "recording-playback",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
                self.playback = Some(completed.playback);
                self.message="All frames and footer validated. Offline image preview; geometry and synchronization are unqualified.".into();
            }
            Err(e) => self.message = format!("Playback failed: {e}. Reopen to validate again."),
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Recording playback").default_open(self.open).show(ui,|ui| {
            ui.small("Open a complete .raptor file or recovered.raptor. Keep recovered files with their report.json; a complete footer does not prove the original scan was complete.");
            ui.add_enabled_ui(!self.is_busy(),|ui|{
                ui.horizontal(|ui|{ui.label("Recording:");ui.text_edit_singleline(&mut self.path);if ui.add_enabled(!self.path.trim().is_empty(),egui::Button::new("Open recording")).clicked(){self.start_path(self.path.clone().into());}});
            });
            if let Some(job)=&self.job {ui.horizontal(|ui|{ui.spinner();ui.label(job.work.progress().0);if ui.button("Cancel playback").clicked(){job.work.cancel();}});ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));}
            if self.playback.is_some() && !self.is_busy() {
                let mut wanted=self.selected;
                ui.horizontal(|ui|{
                    if ui.add_enabled(wanted>0,egui::Button::new("Previous")).clicked(){wanted-=1;}
                    ui.label("Frame (zero based):");
                    ui.add(egui::DragValue::new(&mut wanted).range(0..=self.total-1));
                    ui.label(format!("of {}",self.total));
                    if ui.add_enabled(wanted+1<self.total,egui::Button::new("Next")).clicked(){wanted+=1;}
                    if ui.button("Close recording").clicked(){self.playback=None;self.texture=None;self.details.clear();self.message="Recording closed.".into();}
                });
                if wanted!=self.selected && self.playback.is_some(){self.seek(wanted);}
            }
            ui.label(&self.message);
            if !self.details.is_empty(){ui.small(&self.details);}
            if let Some(texture)=&self.texture {ui.add(egui::Image::new(texture).max_size(egui::vec2(ui.available_width(),300.)).maintain_aspect_ratio(true));}
        });
    }
}
impl Drop for PlaybackUi {
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
    fn worker_loads_navigates_cancels_and_clears_stale_image_on_failure() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-playback-ui-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("source.raptor");
        let mut writer = scancraft_recording::Writer::new(
            Vec::new(),
            scancraft_core::StreamProfile {
                interface: 0,
                width: 2,
                height: 2,
                fourcc: *b"GREY",
                fps: 30,
            },
        )
        .unwrap();
        for i in 0..3u32 {
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&(100 + i * 10).to_le_bytes());
            writer
                .push(&scancraft_core::Frame {
                    sequence: i,
                    host_timestamp_us: 1000 + u64::from(i) * 10,
                    flags: 0,
                    metadata,
                    data: vec![i as u8; 4],
                })
                .unwrap();
        }
        let bytes = writer.finish().unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let ctx = egui::Context::default();
        let mut ui = PlaybackUi::default();
        ui.start_path(path.clone());
        wait(&mut ui, &ctx);
        assert_eq!(ui.total, 3);
        assert_eq!(ui.selected, 0);
        assert!(ui.texture.is_some());
        ui.seek(2);
        assert!(ui.texture.is_none());
        wait(&mut ui, &ctx);
        assert_eq!(ui.selected, 2);
        assert!(ui.texture.is_some());
        ui.seek(0);
        ui.cancel();
        wait(&mut ui, &ctx);
        assert!(ui.message.contains("cancelled"));
        assert!(ui.texture.is_none());
        assert!(ui.playback.is_none());
        ui.start_path(path.clone());
        wait(&mut ui, &ctx);
        let mut corrupt = bytes.clone();
        corrupt.pop();
        std::fs::write(&path, corrupt).unwrap();
        ui.seek(1);
        wait(&mut ui, &ctx);
        assert!(ui.message.contains("source changed"));
        assert!(ui.texture.is_none());
        ui.start_path(path.clone());
        wait(&mut ui, &ctx);
        assert!(ui.message.contains("failed"));
        assert!(ui.texture.is_none());
        std::fs::write(&path, &bytes).unwrap();
        ui.start_path(path.clone());
        wait(&mut ui, &ctx);
        assert!(ui.texture.is_some());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        drop(ui);
        std::fs::remove_dir_all(root).unwrap();
    }
    fn wait(ui: &mut PlaybackUi, ctx: &egui::Context) {
        let start = std::time::Instant::now();
        while ui.is_busy() {
            ui.tick(ctx);
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
