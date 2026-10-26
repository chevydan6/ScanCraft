//! Background original-preserving recording recovery, independent of camera capture.
use eframe::egui;
use scancraft_geometry::work::WorkControl;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread::{self, JoinHandle},
};
struct Completion {
    frames: u64,
    stop: String,
    header_crc: bool,
}
struct Job {
    work: Arc<WorkControl>,
    rx: Receiver<Result<Completion, String>>,
    thread: JoinHandle<()>,
}
pub struct RecoveryUi {
    input: String,
    output: String,
    maximum_frames: u64,
    open: bool,
    message: String,
    job: Option<Job>,
}
impl Default for RecoveryUi {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            maximum_frames: scancraft_recording::recovery::MAX_RECOVERY_FRAMES,
            open: false,
            message: "Recover a validated prefix into a new bundle; original files stay unchanged."
                .into(),
            job: None,
        }
    }
}
impl RecoveryUi {
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn cancel(&self) {
        if let Some(job) = &self.job {
            job.work.cancel();
        }
    }
    pub fn start_paths(&mut self, input: PathBuf, output: PathBuf, maximum_frames: u64) {
        if self.job.is_some() {
            return;
        }
        self.input = input.display().to_string();
        self.output = output.display().to_string();
        self.maximum_frames = maximum_frames;
        self.open = true;
        let work = Arc::new(WorkControl::default());
        let control = work.clone();
        let (tx, rx) = mpsc::channel();
        match thread::Builder::new()
            .name("scancraft-recording-recovery".into())
            .spawn(move || {
                let result = (|| -> Result<Completion, String> {
                    let summary = scancraft_workflow::recording_recovery::run(
                        &input,
                        &output,
                        maximum_frames,
                        &control,
                        &|| Ok(()),
                    )
                    .map_err(|e| e.to_string())?;
                    Ok(Completion {
                        frames: summary["recovered_frames"]
                            .as_u64()
                            .ok_or("missing recovery count")?,
                        header_crc: summary["source_header_crc_verified"]
                            .as_bool()
                            .ok_or("missing source header protection status")?,
                        stop: summary["stop_reason"]
                            .as_str()
                            .ok_or("missing recovery stop reason")?
                            .into(),
                    })
                })();
                let _ = tx.send(result);
            }) {
            Ok(thread) => {
                self.job = Some(Job { work, rx, thread });
                self.message = "Recovering and verifying recording prefix…".into();
            }
            Err(e) => self.message = format!("Cannot start recovery: {e}"),
        }
    }
    pub fn tick(&mut self) {
        let result = match self.job.as_ref().map(|job| job.rx.try_recv()) {
            Some(Ok(result)) => result,
            Some(Err(TryRecvError::Disconnected)) => {
                Err("recovery worker stopped unexpectedly".into())
            }
            _ => return,
        };
        let job = self.job.take().unwrap();
        let joined = job.thread.join();
        self.message = if joined.is_err() {
            "Recovery worker panicked; inspect the new output location. Original source was not written.".into()
        } else {
            match result {
                Ok(completed) => {
                    let status = match completed.stop.as_str() {
                        "source_complete" => "source was already complete",
                        "frame_budget" => "frame limit reached; source completion unverified",
                        _ => "incomplete or damaged suffix omitted",
                    };
                    format!(
                        "Recovered {} validated frame{}; {}. New file replay-verified. Source unchanged. {}Keep report.json with the recovered recording. Bundle: {}",
                        completed.frames,
                        if completed.frames == 1 { "" } else { "s" },
                        status,
                        if completed.header_crc {
                            ""
                        } else {
                            "Legacy source header had no checksum. "
                        },
                        self.output
                    )
                }
                Err(error) => format!("Recovery failed or cancelled: {error}"),
            }
        };
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Recover interrupted recording").default_open(self.open).show(ui,|ui| {
            ui.small("Frames after the first damaged or incomplete frame are omitted. The original is unchanged. Keep the report with the new file.");
            ui.add_enabled_ui(self.job.is_none(),|ui| {
                ui.horizontal(|ui|{ui.label("Source recording / partial:");ui.text_edit_singleline(&mut self.input);});
                ui.horizontal(|ui|{ui.label("New output bundle:");ui.text_edit_singleline(&mut self.output);});
                ui.horizontal(|ui|{ui.label("Maximum frames:");ui.add(egui::DragValue::new(&mut self.maximum_frames).clamp_existing_to_range(false).range(1..=scancraft_recording::recovery::MAX_RECOVERY_FRAMES));if ui.add_enabled(!self.input.trim().is_empty()&&!self.output.trim().is_empty(),egui::Button::new("Recover and verify")).clicked() {self.start_paths(self.input.clone().into(),self.output.clone().into(),self.maximum_frames);}});
            });
            if let Some(job)=&self.job {let (stage,fraction)=job.work.progress();ui.horizontal(|ui|{ui.spinner();ui.add(egui::ProgressBar::new(fraction).text(stage));if ui.button("Cancel recovery").clicked(){job.work.cancel();}});}
            ui.label(&self.message);
        });
    }
}
impl Drop for RecoveryUi {
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
    fn gui_worker_recovers_replays_and_refuses_overwrite_without_source_changes() {
        let root = std::env::temp_dir().join(format!(
            "scancraft-recovery-ui-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.partial");
        let profile = scancraft_core::StreamProfile {
            interface: 0,
            width: 2,
            height: 2,
            fourcc: *b"GREY",
            fps: 30,
        };
        let mut bytes = Vec::new();
        {
            let mut writer = scancraft_recording::Writer::new(&mut bytes, profile).unwrap();
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            writer
                .push(&scancraft_core::Frame {
                    sequence: 0,
                    host_timestamp_us: 1,
                    flags: 0,
                    metadata,
                    data: vec![1; 4],
                })
                .unwrap();
        }
        std::fs::write(&input, &bytes).unwrap();
        let output = root.join("recovered");
        let mut ui = RecoveryUi::default();
        ui.start_paths(input.clone(), output.clone(), 100);
        wait(&mut ui);
        assert!(ui.message.contains("Recovered 1 validated frame"));
        assert!(ui.message.contains("incomplete or damaged suffix omitted"));
        assert_eq!(std::fs::read(&input).unwrap(), bytes);
        let mut reader = scancraft_recording::Reader::new(
            std::fs::File::open(output.join("recovered.raptor")).unwrap(),
        )
        .unwrap();
        assert_eq!(reader.read_frame().unwrap().unwrap().data, vec![1; 4]);
        assert!(reader.read_frame().unwrap().is_none());
        ui.start_paths(input.clone(), output, 100);
        wait(&mut ui);
        assert!(ui.message.contains("already exists"));
        assert_eq!(std::fs::read(input).unwrap(), bytes);
        std::fs::remove_dir_all(root).unwrap();
    }
    fn wait(ui: &mut RecoveryUi) {
        let start = std::time::Instant::now();
        while ui.is_busy() {
            ui.tick();
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
