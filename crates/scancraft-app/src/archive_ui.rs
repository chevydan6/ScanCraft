//! Full supplied-recording preservation, independent of camera acquisition.
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
struct Job {
    work: Arc<WorkControl>,
    rx: Receiver<Result<[u64; 2], String>>,
    thread: JoinHandle<()>,
}
#[derive(Default)]
pub struct ArchiveUi {
    paths: [String; 3],
    project_paths: [String; 2],
    with_project: bool,
    open: bool,
    message: String,
    job: Option<Job>,
}
impl ArchiveUi {
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn cancel(&self) {
        if let Some(job) = &self.job {
            job.work.cancel();
        }
    }
    pub fn start_paths(&mut self, left: PathBuf, right: PathBuf, target: PathBuf) {
        self.start_inner(left, right, target, None);
    }
    pub fn start_scan_paths(
        &mut self,
        project: PathBuf,
        sources: PathBuf,
        left: PathBuf,
        right: PathBuf,
        target: PathBuf,
    ) {
        self.start_inner(left, right, target, Some((project, sources)));
    }
    fn start_inner(
        &mut self,
        left: PathBuf,
        right: PathBuf,
        target: PathBuf,
        project: Option<(PathBuf, PathBuf)>,
    ) {
        if self.is_busy() {
            return;
        }
        self.with_project = project.is_some();
        if let Some((document, sources)) = &project {
            self.project_paths = [
                document.display().to_string(),
                sources.display().to_string(),
            ];
        }
        self.paths = [
            left.display().to_string(),
            right.display().to_string(),
            target.display().to_string(),
        ];
        self.open = true;
        let work = Arc::new(WorkControl::default());
        let control = work.clone();
        let (tx, rx) = mpsc::channel();
        match thread::Builder::new()
            .name("scancraft-recording-archive".into())
            .spawn(move || {
                let result = (|| -> Result<[u64; 2], String> {
                    let report = if let Some((document, sources)) = project {
                        let result = scancraft_workflow::scan_project::export_file(
                            &document, &sources, &left, &right, &target, &control,
                        )
                        .map_err(|e| e.to_string())?;
                        result["recording_archive"].clone()
                    } else {
                        scancraft_workflow::recording_archive::archive(
                            &left, &right, &target, &control,
                        )
                        .map_err(|e| e.to_string())?
                    };
                    Ok([
                        report["files"][0]["recording"]["frames"]
                            .as_u64()
                            .ok_or("missing left frame count")?,
                        report["files"][1]["recording"]["frames"]
                            .as_u64()
                            .ok_or("missing right frame count")?,
                    ])
                })();
                let _ = tx.send(result);
            }) {
            Ok(thread) => {
                self.job = Some(Job { work, rx, thread });
                self.message = "Validating and copying complete supplied recordings…".into();
            }
            Err(e) => self.message = format!("Cannot start archive: {e}"),
        }
    }
    pub fn tick(&mut self) {
        let result = match self.job.as_ref().map(|job| job.rx.try_recv()) {
            Some(Ok(result)) => result,
            Some(Err(TryRecvError::Disconnected)) => {
                Err("archive worker stopped unexpectedly".into())
            }
            _ => return,
        };
        let job = self.job.take().unwrap();
        if job.thread.join().is_err() {
            self.message="Archive worker panicked; inspect the new output location. Source files were not written.".into();
            return;
        }
        let project_note = if self.with_project {
            " Editable project, retained reconstruction inputs and selected-frame association included."
        } else {
            ""
        };
        self.message = match result {
            Ok(frames) => format!(
                "Archived {} left and {} right frames. Complete supplied files copied byte-for-byte and replay-verified. Sources unchanged. Capture completeness, synchronization and physical accuracy remain unqualified. Folder: {}{}",
                frames[0], frames[1], self.paths[2], project_note
            ),
            Err(e) => format!("Archive failed or cancelled: {e}"),
        };
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Archive complete supplied recordings").default_open(self.open).show(ui,|ui|{
            ui.small("Preserves every frame in two complete native recording files. A valid footer does not prove the original scan was complete. Keep recovery reports with recovered inputs. No pairing or calibration is inferred.");
            ui.add_enabled_ui(!self.is_busy(),|ui|{
                ui.checkbox(&mut self.with_project, "Include saved editable project and verify selected-frame association");
                if self.with_project {
                    for (label,path) in ["Saved project", "Retained reconstruction inputs folder"].into_iter().zip(self.project_paths.iter_mut()) {
                        ui.horizontal(|ui| {ui.label(label);ui.text_edit_singleline(path);});
                    }
                    ui.small("Save edits first. Exports the saved project; geometry already open in another pane is unchanged.");
                }
                for (label,path) in ["Left recording (interface 0)","Right recording (interface 2)","New archive folder"].into_iter().zip(self.paths.iter_mut()){ui.horizontal(|ui|{ui.label(label);ui.text_edit_singleline(path);});}
                if ui.add_enabled(self.paths.iter().all(|s|!s.trim().is_empty()) && (!self.with_project || self.project_paths.iter().all(|s| !s.trim().is_empty())),egui::Button::new("Validate and archive full recordings")).clicked(){
                    let [left,right,target]=self.paths.clone().map(PathBuf::from);
                    if self.with_project { self.start_scan_paths(self.project_paths[0].clone().into(),self.project_paths[1].clone().into(),left,right,target); }
                    else { self.start_paths(left,right,target); }
                }
            });
            if let Some(job)=&self.job {ui.horizontal(|ui|{ui.spinner();ui.label(job.work.progress().0);if ui.button("Cancel archive").clicked(){job.work.cancel();}});}
            ui.label(&self.message);
        });
    }
}
impl Drop for ArchiveUi {
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
    fn native_worker_archives_every_frame_and_refuses_existing_output() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/recorded-stereo-dataset");
        let target = std::env::temp_dir().join(format!(
            "scancraft-archive-ui-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut ui = ArchiveUi::default();
        ui.start_paths(
            root.join("left.raptor"),
            root.join("right.raptor"),
            target.clone(),
        );
        wait(&mut ui);
        assert!(ui.message.contains("Archived 3 left and 3 right frames"));
        assert_eq!(
            std::fs::read(target.join("left.raptor")).unwrap(),
            std::fs::read(root.join("left.raptor")).unwrap()
        );
        ui.start_paths(
            root.join("left.raptor"),
            root.join("right.raptor"),
            target.clone(),
        );
        wait(&mut ui);
        assert!(ui.message.contains("already exists"));
        drop(ui);
        std::fs::remove_dir_all(target).unwrap();
    }
    #[test]
    fn native_worker_exports_saved_project_with_full_recordings() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence");
        let source = root.join("project-origin-gui-render/result");
        let target =
            std::env::temp_dir().join(format!("scancraft-scan-project-ui-{}", std::process::id()));
        let mut ui = ArchiveUi::default();
        ui.start_scan_paths(
            source.join("project.orproj"),
            source,
            root.join("recorded-stereo-dataset/left.raptor"),
            root.join("recorded-stereo-dataset/right.raptor"),
            target.clone(),
        );
        wait(&mut ui);
        assert!(
            ui.message.contains("selected-frame association included"),
            "{}",
            ui.message
        );
        let editor = scancraft_geometry::formats::load_project(
            std::fs::File::open(target.join("project/project.orproj")).unwrap(),
        )
        .unwrap();
        assert!(editor.origin().is_some());
        assert!(target.join("recordings/right.raptor").is_file());
        drop(ui);
        std::fs::remove_dir_all(target).unwrap();
    }
    fn wait(ui: &mut ArchiveUi) {
        wait_for(ui, std::time::Duration::from_secs(10)).unwrap_or_else(|error| panic!("{error}"));
    }
    fn wait_for(ui: &mut ArchiveUi, timeout: std::time::Duration) -> Result<(), String> {
        let start = std::time::Instant::now();
        loop {
            ui.tick();
            // Observe completion before the deadline: tick can consume the result
            // and join the worker after this test thread was descheduled.
            let Some(job) = &ui.job else {
                return Ok(());
            };
            if start.elapsed() >= timeout {
                let diagnostic = format!(
                    "archive test timed out after {:?} (limit {timeout:?}); progress={:?}; worker_finished={}; with_project={}; paths={:?}; project_paths={:?}; message={:?}",
                    start.elapsed(),
                    job.work.progress(),
                    job.thread.is_finished(),
                    ui.with_project,
                    ui.paths,
                    ui.project_paths,
                    ui.message,
                );
                job.work.cancel();
                return Err(diagnostic);
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    fn wait_consumes_ready_success_and_error_before_expired_deadline() {
        for (result, expected) in [
            (Ok([3, 4]), "Archived 3 left and 4 right frames"),
            (
                Err("deliberate archive failure".into()),
                "Archive failed or cancelled: deliberate archive failure",
            ),
        ] {
            let (tx, rx) = mpsc::channel();
            tx.send(result).unwrap();
            let mut ui = ArchiveUi {
                job: Some(Job {
                    work: Arc::new(WorkControl::default()),
                    rx,
                    thread: thread::spawn(|| {}),
                }),
                paths: Default::default(),
                project_paths: Default::default(),
                with_project: false,
                open: false,
                message: String::new(),
            };
            wait_for(&mut ui, std::time::Duration::ZERO).unwrap();
            assert!(!ui.is_busy());
            assert!(ui.message.contains(expected), "{}", ui.message);
        }
    }

    #[test]
    fn wait_timeout_reports_pending_work_and_requests_cancellation() {
        let work = Arc::new(WorkControl::default());
        work.update(scancraft_geometry::work::Stage::Writing, 1, 2)
            .unwrap();
        let control = work.clone();
        let (tx, rx) = mpsc::channel();
        let mut ui = ArchiveUi {
            paths: ["left".into(), "right".into(), "destination".into()],
            job: Some(Job {
                work: work.clone(),
                rx,
                thread: thread::spawn(move || {
                    while control.check().is_ok() {
                        thread::sleep(std::time::Duration::from_millis(1));
                    }
                    let _ = tx.send(Err("Operation cancelled".into()));
                }),
            }),
            project_paths: Default::default(),
            with_project: false,
            open: false,
            message: String::new(),
        };
        let error = wait_for(&mut ui, std::time::Duration::ZERO).unwrap_err();
        assert!(error.contains("archive test timed out"), "{error}");
        assert!(error.contains("Writing"), "{error}");
        assert!(error.contains("0.5"), "{error}");
        assert!(error.contains("destination"), "{error}");
        assert!(work.check().is_err());
        wait(&mut ui);
        assert!(ui.message.contains("Operation cancelled"));
    }
}
