mod archive_ui;
mod capture_worker;
mod cloud_jobs;
mod cloud_ui;
mod measurement_view;
mod mesh_ui;
mod playback_ui;
mod recovery_ui;
mod screen_tools;
// Rust desktop workbench; acquisition/recording is independent of UI throughput.
use eframe::egui;
use scancraft_core::{Frame, StreamProfile};
use scancraft_device::{Capture, SessionLock, discover};
use scancraft_recording::Writer;
use std::{
    fs::OpenOptions,
    io::BufWriter,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, TrySendError},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
fn camera_texture(
    texture: &mut Option<egui::TextureHandle>,
    ctx: &egui::Context,
    rgb: &image::RgbImage,
    statistics: Option<&scancraft_image::diagnostics::GrayStatistics>,
    stretch: bool,
) {
    let size = [rgb.width() as usize, rgb.height() as usize];
    let displayed: std::borrow::Cow<'_, [u8]> = if stretch {
        if let Some(stats) = statistics {
            let lut = stats.contrast_lut();
            std::borrow::Cow::Owned(rgb.as_raw().iter().map(|v| lut[usize::from(*v)]).collect())
        } else {
            std::borrow::Cow::Borrowed(rgb.as_raw())
        }
    } else {
        std::borrow::Cow::Borrowed(rgb.as_raw())
    };
    let color = egui::ColorImage::from_rgb(size, &displayed);
    if let Some(t) = texture {
        t.set(color, egui::TextureOptions::LINEAR);
    } else {
        *texture = Some(ctx.load_texture("camera", color, egui::TextureOptions::LINEAR));
    }
}
struct Running {
    cancel: Arc<AtomicBool>,
    frames: Arc<AtomicU64>,
    preview_drops: Arc<AtomicU64>,
    rx: Receiver<Frame>,
    worker: capture_worker::Worker,
}
struct App {
    recovery: recovery_ui::RecoveryUi,
    archive: archive_ui::ArchiveUi,
    playback: playback_ui::PlaybackUi,
    cloud: cloud_ui::CloudUi,
    editing: bool,
    mesh_active: bool,
    mesh: mesh_ui::MeshUi,
    confirm_close: bool,
    close_allowed: bool,
    running: Option<Running>,
    interface: u8,
    status: String,
    log: String,
    texture: Option<egui::TextureHandle>,
    record: bool,
    folder: String,
    snapshot: Option<image::RgbImage>,
    ir_statistics: Option<scancraft_image::diagnostics::GrayStatistics>,
    stretch_ir: bool,
    smoke_ir: bool,
    smoke: Option<std::time::Instant>,
}
impl Default for App {
    fn default() -> Self {
        Self {
            recovery: Default::default(),
            archive: Default::default(),
            playback: Default::default(),
            cloud: cloud_ui::CloudUi::default(),
            editing: false,
            mesh_active: false,
            mesh: mesh_ui::MeshUi::default(),
            confirm_close: false,
            close_allowed: false,
            running: None,
            interface: 4,
            status: "Ready. Find the scanner to check its USB interfaces.".into(),
            log: String::new(),
            texture: None,
            record: false,
            folder: format!(
                "{}/Documents/ScanCraft",
                std::env::var("HOME").unwrap_or_else(|_| ".".into())
            ),
            snapshot: None,
            ir_statistics: None,
            stretch_ir: false,
            smoke_ir: false,
            smoke: None,
        }
    }
}
impl App {
    fn start(&mut self) {
        if self.running.is_some() {
            return;
        }
        let interface = self.interface;
        let record = self.record;
        let folder = self.folder.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let frames = Arc::new(AtomicU64::new(0));
        let drops = Arc::new(AtomicU64::new(0));
        let (tx, rx) = mpsc::sync_channel(2);
        let c = cancel.clone();
        let f = frames.clone();
        let d = drops.clone();
        let worker = match capture_worker::Worker::spawn(move || {
            let run = || -> Result<String, String> {
                let _locks = SessionLock::acquire().map_err(|e| e.to_string())?;
                let profile = StreamProfile {
                    interface,
                    width: if interface == 4 { 1280 } else { 1920 },
                    height: if interface == 4 { 800 } else { 1200 },
                    fourcc: if interface == 4 { *b"MJPG" } else { *b"GREY" },
                    fps: if interface == 4 { 10 } else { 50 },
                };
                let mut capture = if record {
                    Capture::open_recording(profile.clone())
                } else {
                    Capture::open(profile.clone())
                }
                .map_err(|e| e.to_string())?;
                let output = if record {
                    let folder = PathBuf::from(folder);
                    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
                    let stamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|e| e.to_string())?
                        .as_nanos();
                    Some(folder.join(format!("capture-{stamp}.raptor")))
                } else {
                    None
                };
                let partial = output.as_ref().map(|p| p.with_extension("raptor.partial"));
                let mut recording = if let Some(path) = &partial {
                    Some(
                        Writer::with_context(
                            BufWriter::new(
                                OpenOptions::new()
                                    .write(true)
                                    .create_new(true)
                                    .open(path)
                                    .map_err(|e| e.to_string())?,
                            ),
                            profile,
                            {
                                let mut context = scancraft_recording::CaptureContext::default();
                                if let Some(observed) = capture.recording_observations() {
                                    context
                                        .add_capture_observations(observed)
                                        .map_err(|e| e.to_string())?;
                                }
                                context
                            },
                        )
                        .map_err(|e| e.to_string())?,
                    )
                } else {
                    None
                };
                while !c.load(Ordering::Relaxed) {
                    let frame = capture.next_frame().map_err(|e| e.to_string())?;
                    if let Some(w) = &mut recording {
                        w.push(&frame).map_err(|e| e.to_string())?;
                    }
                    f.fetch_add(1, Ordering::Relaxed);
                    match tx.try_send(frame) {
                        Ok(()) => (),
                        Err(TrySendError::Full(_)) => {
                            d.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(TrySendError::Disconnected(_)) => break,
                    }
                }
                let rejected = capture.rejected;
                let missing = capture.metadata_missing;
                let gaps = capture.sequence_drops;
                capture.close().map_err(|e| e.to_string())?;
                if let Some(w) = recording {
                    let file = w.finish().map_err(|e| e.to_string())?;
                    file.get_ref().sync_all().map_err(|e| e.to_string())?;
                    drop(file);
                    std::fs::hard_link(partial.as_ref().unwrap(), output.as_ref().unwrap())
                        .map_err(|e| e.to_string())?;
                    std::fs::remove_file(partial.as_ref().unwrap()).map_err(|e| e.to_string())?;
                }
                Ok(format!(
                    "Stopped cleanly. Rejected buffers={rejected}, missing metadata={missing}, sequence gaps={gaps}. Recording: {}",
                    output.map_or_else(|| "off".into(), |p| p.display().to_string())
                ))
            };
            run()
        }) {
            Ok(worker) => worker,
            Err(error) => {
                self.status = "Capture not started".into();
                self.log = format!("Could not create capture worker: {error}");
                return;
            }
        };
        self.running = Some(Running {
            cancel,
            frames,
            preview_drops: drops,
            rx,
            worker,
        });
        self.status = "Starting native capture…".into();
    }
    fn receive(&mut self, ctx: &egui::Context) {
        let Some(r) = &self.running else { return };
        let mut latest = None;
        while let Ok(frame) = r.rx.try_recv() {
            latest = Some(frame);
        }
        if let Some(frame) = latest {
            self.ir_statistics = if self.interface == 4 {
                None
            } else {
                scancraft_image::GrayImage::new(1920, 1200, &frame.data)
                    .ok()
                    .map(scancraft_image::diagnostics::statistics)
            };
            let decoded = if self.interface == 4 {
                image::load_from_memory_with_format(&frame.data, image::ImageFormat::Jpeg)
                    .map(|im| im.to_rgb8())
                    .map_err(|e| e.to_string())
            } else {
                image::GrayImage::from_raw(1920, 1200, frame.data)
                    .map(|im| image::DynamicImage::ImageLuma8(im).to_rgb8())
                    .ok_or_else(|| "Invalid grayscale buffer".to_string())
            };
            match decoded {
                Ok(rgb) => {
                    camera_texture(
                        &mut self.texture,
                        ctx,
                        &rgb,
                        self.ir_statistics.as_ref(),
                        self.stretch_ir,
                    );
                    self.snapshot = Some(rgb);
                }
                Err(e) => {
                    self.log = format!("Image decoding failed: {e}");
                    r.cancel.store(true, Ordering::Relaxed);
                }
            }
        }
        self.status = format!(
            "Native acquisition · {} validated frames · {} preview queue drops",
            r.frames.load(Ordering::Relaxed),
            r.preview_drops.load(Ordering::Relaxed)
        );
        if let Some(result) = self.running.as_mut().and_then(|r| r.worker.poll()) {
            self.log = match result {
                Ok(s) => s,
                Err(e) => {
                    format!("Capture failed: {e}. Any .partial file is an incomplete recording.")
                }
            };
            self.running = None;
            self.status = "Capture stopped".into();
        }
        ctx.request_repaint_after(Duration::from_millis(30));
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.cloud.tick();
        self.mesh.tick();
        self.recovery.tick();
        self.archive.tick();
        self.playback.tick(ctx);
        if self.recovery.is_busy() || self.playback.is_busy() || self.archive.is_busy() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        if ctx.input(|i| i.viewport().close_requested())
            && !self.close_allowed
            && (self.cloud.is_dirty()
                || self.cloud.is_busy()
                || self.recovery.is_busy()
                || self.playback.is_busy()
                || self.archive.is_busy())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }
        if self.confirm_close {
            egui::Window::new("Close ScanCraft?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(
                        "There are unsaved geometry changes or an operation is still running.",
                    );
                    if ui.button("Keep open").clicked() {
                        self.confirm_close = false;
                    }
                    if self.cloud.is_busy()
                        || self.recovery.is_busy()
                        || self.playback.is_busy()
                        || self.archive.is_busy()
                    {
                        if ui.button("Cancel operation and keep open").clicked() {
                            self.cloud.cancel();
                            self.recovery.cancel();
                            self.playback.cancel();
                            self.archive.cancel();
                            self.confirm_close = false;
                        }
                    } else if ui.button("Save project and keep open").clicked() {
                        self.cloud.save_project(&self.folder);
                        self.confirm_close = false;
                    }
                    if ui.button("Discard unsaved changes and close").clicked() {
                        self.cloud.cancel();
                        self.recovery.cancel();
                        self.playback.cancel();
                        self.archive.cancel();
                        self.close_allowed = true;
                        self.confirm_close = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
        }
        if self.running.is_some()
            || self.cloud.is_busy()
            || self.mesh.is_busy()
            || self.recovery.is_busy()
            || self.playback.is_busy()
            || self.archive.is_busy()
            || self.smoke.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        self.receive(ctx);
        if let Some(start) = self.smoke {
            if let Some(r) = &self.running {
                if r.frames.load(Ordering::Relaxed) >= 20
                    || start.elapsed() > Duration::from_secs(15)
                {
                    r.cancel.store(true, Ordering::Relaxed);
                }
            } else if !self.smoke_ir || start.elapsed() >= Duration::from_secs(5) {
                if let Some(image) = &self.snapshot {
                    let folder = PathBuf::from(&self.folder);
                    if let Err(e) = std::fs::create_dir_all(&folder).and_then(|_| {
                        image
                            .save(folder.join("gui-smoke.png"))
                            .map_err(std::io::Error::other)
                    }) {
                        eprintln!("Smoke snapshot failed: {e}");
                    }
                }
                self.smoke = None;
                println!(
                    "GUI_SMOKE decoded_image={} result={}",
                    self.snapshot.is_some(),
                    self.log
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        egui::CentralPanel::default().show(ctx,|ui|{
            ui.horizontal(|ui|{
                if ui.selectable_label(!self.mesh_active&&!self.editing,"Cameras").clicked(){self.mesh_active=false;self.editing=false;}
                if ui.selectable_label(!self.mesh_active&&self.editing,"Point clouds").clicked(){self.mesh_active=false;self.editing=true;}
                if ui.selectable_label(self.mesh_active,"Meshes").clicked(){self.mesh_active=true;}
            });
            if self.mesh_active {self.mesh.ui(ui);return;}
            if self.editing {self.cloud.ui(ui,&mut self.folder);return;}
            ui.heading("ScanCraft");ui.label("ArtCraft Ecosystem Candidate · Experimental 0.1");
            ui.label("Calibration deferred. Scanner reconstruction and accuracy qualification are not available yet.");ui.separator();
            let active=self.running.is_some();
            let previous_interface=self.interface;
            ui.horizontal(|ui|{
                ui.add_enabled_ui(!active,|ui|{
                    if ui.button("Find scanner").clicked(){self.log=match discover(){Ok(n)=>format!("{} native video/metadata nodes\n{}",n.len(),n.iter().map(|n|format!("{} · interface {} · {}",n.path.display(),n.interface,if n.metadata{"metadata"}else{"image"})).collect::<Vec<_>>().join("\n")),Err(e)=>e.to_string()};}
                    egui::ComboBox::from_id_salt("camera").selected_text(match self.interface{0=>"Left IR",2=>"Right IR",_=>"Color"}).show_ui(ui,|ui|{ui.selectable_value(&mut self.interface,4,"Color");ui.selectable_value(&mut self.interface,0,"Left IR");ui.selectable_value(&mut self.interface,2,"Right IR");});
                    if ui.button("Start preview").clicked(){self.start();}
                });
                if ui.add_enabled(active,egui::Button::new("Stop")).clicked()&& let Some(r)=&self.running{r.cancel.store(true,Ordering::Relaxed);}
                if ui.add_enabled(self.snapshot.is_some(),egui::Button::new("Save snapshot")).clicked(){
                    let dir=PathBuf::from(&self.folder);let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();let path=dir.join(format!("snapshot-{stamp}.png"));
                    self.log=match std::fs::create_dir_all(&dir).and_then(|_|self.snapshot.as_ref().unwrap().save(&path).map_err(std::io::Error::other)){Ok(())=>format!("Saved {}",path.display()),Err(e)=>format!("Save failed: {e}")};
                }
            });
            ui.add_enabled_ui(!active,|ui|{ui.horizontal(|ui|{ui.checkbox(&mut self.record,"Record raw frames during preview");ui.label("Save folder:");ui.text_edit_singleline(&mut self.folder);});});
            self.recovery.ui(ui);
            self.archive.ui(ui);
            self.playback.ui(ui);
            if previous_interface!=self.interface {self.texture=None;self.snapshot=None;self.ir_statistics=None;}
            if self.interface!=4 {
                if ui.checkbox(&mut self.stretch_ir,"Stretch IR display contrast").changed()
                    && let Some(rgb)=&self.snapshot {
                    camera_texture(&mut self.texture,ctx,rgb,self.ir_statistics.as_ref(),self.stretch_ir);
                }
                if let Some(s)=&self.ir_statistics {ui.label(format!("Raw IR levels · p01 {} · median {} · p99 {} · mean {:.1} · black/white clipping {:.2}% / {:.2}%",s.p01,s.median,s.p99,s.mean,100.*s.black_clipped as f64/s.pixels as f64,100.*s.white_clipped as f64/s.pixels as f64));}
                ui.small("Display stretch does not indicate focus or valid depth. Recordings and snapshots retain raw levels.");
            }
            ui.label(&self.status);ui.separator();
            if let Some(t)=&self.texture{ui.add(egui::Image::new(t).max_size(egui::vec2(ui.available_width(),(ui.available_height()-140.).max(100.))).maintain_aspect_ratio(true));}else{ui.label("No live camera image yet. Select a camera and start preview.");}
            ui.separator();egui::ScrollArea::vertical().max_height(110.).show(ui,|ui|{ui.monospace(&self.log);});
        });
    }
}
impl Drop for App {
    fn drop(&mut self) {
        if let Some(r) = &mut self.running {
            r.cancel.store(true, Ordering::Relaxed);
            let _ = r.worker.join();
        }
    }
}
#[derive(Debug, PartialEq)]
enum Startup {
    StatisticalPreview {
        path: PathBuf,
        neighbors: usize,
        sigma: f64,
    },
    ExportScanProject {
        project: PathBuf,
        sources: PathBuf,
        left: PathBuf,
        right: PathBuf,
        target: PathBuf,
    },
    ArchiveRecordings {
        left: PathBuf,
        right: PathBuf,
        target: PathBuf,
    },
    ExportBundle {
        project: PathBuf,
        sources: PathBuf,
        target: PathBuf,
    },
    VerifySources {
        project: PathBuf,
        directory: PathBuf,
    },
    ReconstructRecordings {
        inputs: scancraft_workflow::recording_reconstruction::Inputs,
        output: PathBuf,
    },
    Playback(PathBuf),
    Empty,
    File(PathBuf),
    Mesh(PathBuf),
    MeshBoundaries(PathBuf),
    MeshNormals(PathBuf),
    CloudNormals(PathBuf),
    MeasurementOutlines(PathBuf),
    CoordinateFrame(PathBuf),
    FrameDocument(PathBuf),
    FitCylinder {
        path: PathBuf,
        maximum_rms_mm: f64,
    },
    MeshSection {
        path: PathBuf,
        origin: [f64; 3],
        normal: [f64; 3],
        tolerance: f64,
    },
    RecoverRecording {
        input: PathBuf,
        output: PathBuf,
        maximum_frames: u64,
    },
    MeshImport {
        input: PathBuf,
        target: PathBuf,
        config: scancraft_geometry::mesh_import::Config,
    },
    MeshSmooth {
        path: PathBuf,
        config: scancraft_geometry::mesh_smooth::Config,
    },
    MeshSimplify {
        path: PathBuf,
        config: scancraft_geometry::mesh_simplify::Config,
    },
    MeshPlane {
        path: PathBuf,
        normal: [f64; 3],
        offset: f64,
    },
    Reconstruct {
        inputs: scancraft_workflow::FileInputs,
        output: PathBuf,
    },
    FuseSequence {
        settings: PathBuf,
        output: PathBuf,
    },
    Smoke,
    SmokeIr,
    Help,
}
fn startup_args(args: Vec<std::ffi::OsString>) -> Result<Startup, String> {
    match args.as_slice() {
        [] => Ok(Startup::Empty),
        [flag] if flag == "--help" || flag == "-h" => Ok(Startup::Help),
        [flag] if flag == "--smoke-test" => Ok(Startup::Smoke),
        [flag] if flag == "--smoke-test-ir" => Ok(Startup::SmokeIr),
        [flag,calibration,left,right,settings,output] if flag=="--reconstruct" => Ok(Startup::Reconstruct {inputs:scancraft_workflow::FileInputs {calibration:calibration.into(),left:left.into(),right:right.into(),settings:settings.into()},output:output.into()}),
        [flag,settings,output] if flag=="--fuse-sequence" => Ok(Startup::FuseSequence {settings:settings.into(),output:output.into()}),
        [flag,calibration,left,li,right,ri,settings,output] if flag=="--reconstruct-recordings" => {
            let index=|s:&std::ffi::OsString|s.to_str().ok_or("invalid recording index")?.parse::<usize>().map_err(|_|"invalid recording index");
            Ok(Startup::ReconstructRecordings{inputs:scancraft_workflow::recording_reconstruction::Inputs{calibration:calibration.into(),left:left.into(),left_index:index(li)?,right:right.into(),right_index:index(ri)?,settings:settings.into()},output:output.into()})
        }
        [flag,project,sources,left,right,target] if flag=="--export-scan-project"=>Ok(Startup::ExportScanProject{project:project.into(),sources:sources.into(),left:left.into(),right:right.into(),target:target.into()}),
        [flag,path,k,sigma] if flag=="--statistical-preview"=>{
            let neighbors=k.to_str().ok_or("invalid k")?.parse::<usize>().map_err(|_|"invalid k")?;
            let sigma=sigma.to_str().ok_or("invalid sigma")?.parse::<f64>().map_err(|_|"invalid sigma")?;
            if !(1..=1024).contains(&neighbors) || !sigma.is_finite() || sigma<0. {return Err("statistical preview requires k 1..1024 and finite nonnegative sigma".into());}
            Ok(Startup::StatisticalPreview{path:path.into(),neighbors,sigma})
        }
        [flag,left,right,target] if flag=="--archive-recordings"=>Ok(Startup::ArchiveRecordings{left:left.into(),right:right.into(),target:target.into()}),
        [flag,project,sources,target] if flag=="--export-project-bundle" => Ok(Startup::ExportBundle{project:project.into(),sources:sources.into(),target:target.into()}),
        [flag,project,directory] if flag=="--verify-source-bundle" => Ok(Startup::VerifySources{project:project.into(),directory:directory.into()}),
        [flag,path] if flag=="--play-recording" => Ok(Startup::Playback(path.into())),
        [flag,path,ox,oy,oz,nx,ny,nz,tolerance] if flag=="--mesh-section" => {
            let parse=|v:&std::ffi::OsString|v.to_str().ok_or_else(||"section values must be text".to_owned()).and_then(|s|s.parse::<f64>().map_err(|_|"invalid section value".to_owned()));
            Ok(Startup::MeshSection {path:path.into(),origin:[parse(ox)?,parse(oy)?,parse(oz)?],normal:[parse(nx)?,parse(ny)?,parse(nz)?],tolerance:parse(tolerance)?})
        }
        [flag,input,maximum,output] if flag=="--recover-recording" => Ok(Startup::RecoverRecording {input:input.into(),output:output.into(),maximum_frames:maximum.to_str().ok_or("invalid recovery budget")?.parse().map_err(|_|"invalid recovery budget")?}),
        [flag,input,format,units,welding,target] if flag=="--import-mesh" => {
            let text=|s:&std::ffi::OsString|s.to_str().ok_or("invalid UTF-8 import parameter").map(str::to_owned);
            Ok(Startup::MeshImport {input:input.into(),target:target.into(),config:scancraft_geometry::mesh_import::Config::from_labels(&text(format)?,&text(units)?,&text(welding)?)?})
        }
        [flag,path,iterations,relaxation,drift,normal,crease] if flag=="--mesh-smooth-preview" => {
            let text=|s:&std::ffi::OsString| s.to_str().map(str::to_owned).ok_or("invalid UTF-8 parameter");
            Ok(Startup::MeshSmooth {path:path.into(),config:scancraft_geometry::mesh_smooth::Config {iterations:text(iterations)?.parse().map_err(|_|"invalid iterations")?,relaxation:text(relaxation)?.parse().map_err(|_|"invalid relaxation")?,maximum_displacement_mm:text(drift)?.parse().map_err(|_|"invalid displacement")?,maximum_normal_change_degrees:text(normal)?.parse().map_err(|_|"invalid normal bound")?,preserve_crease_degrees:text(crease)?.parse().map_err(|_|"invalid crease bound")?}})
        }
        [flag,path,target,drift,normal,crease,budget] if flag=="--mesh-simplify-preview" => {
            let text=|v:&std::ffi::OsString|v.to_str().ok_or("simplification values must be text").map(str::to_owned);
            Ok(Startup::MeshSimplify {path:path.into(),config:scancraft_geometry::mesh_simplify::Config {target_triangles:text(target)?.parse().map_err(|_|"invalid target")?,maximum_displacement_mm:text(drift)?.parse().map_err(|_|"invalid displacement")?,maximum_normal_change_degrees:text(normal)?.parse().map_err(|_|"invalid normal bound")?,preserve_crease_degrees:text(crease)?.parse().map_err(|_|"invalid crease bound")?,maximum_collapses:text(budget)?.parse().map_err(|_|"invalid budget")?}})
        }
        [flag,path,nx,ny,nz,offset] if flag=="--mesh-plane-preview" => {
            let parse=|v:&std::ffi::OsString|v.to_str().ok_or_else(||"plane values must be text".to_owned()).and_then(|s|s.parse::<f64>().map_err(|_|"invalid plane value".to_owned()));
            Ok(Startup::MeshPlane {path:path.into(),normal:[parse(nx)?,parse(ny)?,parse(nz)?],offset:parse(offset)?})
        }
        [flag,path] if flag=="--mesh" => Ok(Startup::Mesh(path.into())),
        [flag,path] if flag=="--mesh-boundaries" => Ok(Startup::MeshBoundaries(path.into())),
        [flag,path,rms] if flag=="--fit-cylinder" => Ok(Startup::FitCylinder {path:path.into(),maximum_rms_mm:rms.to_str().ok_or("invalid cylinder RMS")?.parse::<f64>().map_err(|_|"invalid cylinder RMS")?}),
        [flag,path] if flag=="--frame-document" => Ok(Startup::FrameDocument(path.into())),
        [flag,path] if flag=="--coordinate-frame" => Ok(Startup::CoordinateFrame(path.into())),
        [flag,path] if flag=="--measurement-outlines" => Ok(Startup::MeasurementOutlines(path.into())),
        [flag,path] if flag=="--cloud-normals" => Ok(Startup::CloudNormals(path.into())),
        [flag,path] if flag=="--mesh-normals" => Ok(Startup::MeshNormals(path.into())),
        [flag, path] if flag == "--" => Ok(Startup::File(PathBuf::from(path))),
        [path] if !path.to_string_lossy().starts_with('-') => {
            Ok(Startup::File(PathBuf::from(path)))
        }
        _ => Err("Usage: scancraft [PROJECT_OR_PLY_FILE] | --reconstruct CALIBRATION LEFT.PNG RIGHT.PNG SETTINGS NEW_RESULT_FOLDER | --fuse-sequence SETTINGS NEW_RESULT_FOLDER | --mesh MESH_JSON | --mesh-section MESH_OR_PROJECT OX OY OZ NX NY NZ TOLERANCE_MM | --mesh-boundaries MESH_OR_PROJECT | --fit-cylinder PROJECT_OR_PLY_FILE MAX_RMS_MM | --frame-document FRAME_FILE | --coordinate-frame PROJECT_OR_PLY_FILE | --measurement-outlines PROJECT_OR_PLY_FILE | --cloud-normals PROJECT_OR_PLY_FILE | --mesh-normals MESH_OR_PROJECT | --reconstruct-recordings CALIBRATION LEFT_RECORDING LEFT_INDEX RIGHT_RECORDING RIGHT_INDEX SETTINGS NEW_FOLDER | --statistical-preview PROJECT_OR_PLY K SIGMA_MULTIPLIER | --export-scan-project PROJECT SOURCE_BUNDLE LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY | --archive-recordings LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY | --export-project-bundle PROJECT SOURCE_BUNDLE NEW_DIRECTORY | --verify-source-bundle PROJECT DIRECTORY | --play-recording RECORDING | --recover-recording INPUT MAX_FRAMES NEW_FOLDER | --import-mesh FILE obj|stl mm|in|m separate|weld-exact NEW_FOLDER | --mesh-smooth-preview MESH_OR_PROJECT ITERATIONS RELAXATION MAX_MOVE_MM MAX_NORMAL_DEG CREASE_DEG | --mesh-simplify-preview MESH_OR_PROJECT TARGET MAX_MOVE_MM MAX_NORMAL_DEG CREASE_DEG MAX_COLLAPSES | --mesh-plane-preview MESH_JSON NX NY NZ OFFSET | --help | --smoke-test | --smoke-test-ir".into()),
    }
}
fn main() -> eframe::Result {
    let startup = match startup_args(std::env::args_os().skip(1).collect()) {
        Ok(Startup::Help) => {
            println!(
                "Usage: scancraft [PROJECT_OR_PLY_FILE]\n       scancraft --reconstruct CALIBRATION LEFT.PNG RIGHT.PNG SETTINGS NEW_RESULT_FOLDER\n       scancraft --fuse-sequence SETTINGS NEW_RESULT_FOLDER\n       scancraft --mesh MESH_JSON\n       scancraft --mesh-section MESH_OR_PROJECT OX OY OZ NX NY NZ TOLERANCE_MM | --mesh-boundaries MESH_OR_PROJECT\n       scancraft --fit-cylinder PROJECT_OR_PLY_FILE MAX_RMS_MM | --frame-document FRAME_FILE | --coordinate-frame PROJECT_OR_PLY_FILE | --measurement-outlines PROJECT_OR_PLY_FILE | --cloud-normals PROJECT_OR_PLY_FILE\n       scancraft --mesh-normals MESH_OR_PROJECT\n       scancraft --reconstruct-recordings CALIBRATION LEFT_RECORDING LEFT_INDEX RIGHT_RECORDING RIGHT_INDEX SETTINGS NEW_FOLDER | --statistical-preview PROJECT_OR_PLY K SIGMA_MULTIPLIER | --export-scan-project PROJECT SOURCE_BUNDLE LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY | --archive-recordings LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY | --export-project-bundle PROJECT SOURCE_BUNDLE NEW_DIRECTORY | --verify-source-bundle PROJECT DIRECTORY | --play-recording RECORDING | --recover-recording INPUT MAX_FRAMES NEW_FOLDER | --import-mesh FILE obj|stl mm|in|m separate|weld-exact NEW_FOLDER | --mesh-smooth-preview MESH_OR_PROJECT ITERATIONS RELAXATION MAX_MOVE_MM MAX_NORMAL_DEG CREASE_DEG | --mesh-simplify-preview MESH_OR_PROJECT TARGET MAX_MOVE_MM MAX_NORMAL_DEG CREASE_DEG MAX_COLLAPSES | --mesh-plane-preview MESH_JSON NX NY NZ OFFSET\nOpen geometry without starting camera acquisition. Files without units require an explicit choice in the import panel."
            );
            return Ok(());
        }
        Ok(startup) => startup,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    eframe::run_native(
        "ScanCraft",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1150., 850.]),
            ..Default::default()
        },
        Box::new(move |_| {
            let mut app = App::default();
            match startup {
                Startup::StatisticalPreview {
                    path,
                    neighbors,
                    sigma,
                } => {
                    app.editing = true;
                    app.cloud.statistical_startup(path, neighbors, sigma);
                }
                Startup::ExportScanProject {
                    project,
                    sources,
                    left,
                    right,
                    target,
                } => app
                    .archive
                    .start_scan_paths(project, sources, left, right, target),
                Startup::ArchiveRecordings {
                    left,
                    right,
                    target,
                } => app.archive.start_paths(left, right, target),
                Startup::ExportBundle {
                    project,
                    sources,
                    target,
                } => {
                    app.editing = true;
                    app.cloud.export_bundle_startup(project, sources, target);
                }
                Startup::VerifySources { project, directory } => {
                    app.editing = true;
                    app.cloud.verify_sources_startup(project, directory);
                }
                Startup::ReconstructRecordings { inputs, output } => {
                    app.editing = true;
                    app.cloud.recording_reconstruction_startup(inputs, output);
                }
                Startup::Playback(path) => app.playback.start_path(path),
                Startup::RecoverRecording {
                    input,
                    output,
                    maximum_frames,
                } => app.recovery.start_paths(input, output, maximum_frames),
                Startup::MeshPlane {
                    path,
                    normal,
                    offset,
                } => {
                    app.mesh_active = true;
                    app.mesh.open_plane_preview(path, normal, offset);
                }
                Startup::Mesh(path) => {
                    app.mesh_active = true;
                    app.mesh.open(path);
                }
                Startup::MeshNormals(path) => {
                    app.mesh_active = true;
                    app.mesh.open_with_normals(path);
                }
                Startup::MeshSection {
                    path,
                    origin,
                    normal,
                    tolerance,
                } => {
                    app.mesh_active = true;
                    app.mesh.open_with_section(path, origin, normal, tolerance);
                }
                Startup::MeshImport {
                    input,
                    target,
                    config,
                } => {
                    app.mesh_active = true;
                    app.mesh.open_external_import(input, target, config);
                }
                Startup::MeshSmooth { path, config } => {
                    app.mesh_active = true;
                    app.mesh.open_smooth_preview(path, config);
                }
                Startup::MeshSimplify { path, config } => {
                    app.mesh_active = true;
                    app.mesh.open_simplify_preview(path, config);
                }
                Startup::MeshBoundaries(path) => {
                    app.mesh_active = true;
                    app.mesh.open_with_boundary_analysis(path);
                }
                Startup::FitCylinder {
                    path,
                    maximum_rms_mm,
                } => {
                    app.editing = true;
                    app.cloud.open_cylinder_startup(path, maximum_rms_mm);
                }
                Startup::FrameDocument(path) => {
                    app.editing = true;
                    app.cloud.open_saved_frame(path);
                }
                Startup::CoordinateFrame(path) => {
                    app.editing = true;
                    app.cloud.open_frame_startup(path);
                }
                Startup::MeasurementOutlines(path) => {
                    app.editing = true;
                    app.cloud.open_measurements_startup(path);
                }
                Startup::CloudNormals(path) => {
                    app.editing = true;
                    app.cloud.open_normals_startup(path);
                }
                Startup::File(path) => {
                    app.editing = true;
                    app.cloud.open_startup(path);
                }
                Startup::FuseSequence { settings, output } => {
                    app.editing = true;
                    app.cloud.sequence_startup(settings, output);
                }
                Startup::Reconstruct { inputs, output } => {
                    app.editing = true;
                    app.cloud.reconstruct_startup(inputs, output);
                }
                Startup::Smoke | Startup::SmokeIr => {
                    if startup == Startup::SmokeIr {
                        app.interface = 0;
                        app.stretch_ir = true;
                        app.smoke_ir = true;
                    }
                    app.folder = std::env::var("SCANCRAFT_SMOKE_OUTPUT").unwrap_or_else(|_| {
                        std::env::temp_dir()
                            .join("scancraft-smoke")
                            .display()
                            .to_string()
                    });
                    app.record = true;
                    app.smoke = Some(std::time::Instant::now());
                    app.start();
                }
                Startup::Empty | Startup::Help => {}
            }
            Ok(Box::new(app))
        }),
    )
}
#[cfg(test)]
mod startup_tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<std::ffi::OsString> {
        values.iter().map(std::ffi::OsString::from).collect()
    }
    #[test]
    fn statistical_preview_parameters_are_explicit_and_validated() {
        assert!(matches!(
            startup_args(
                ["--statistical-preview", "cloud.ply", "4", "1"]
                    .map(Into::into)
                    .to_vec()
            )
            .unwrap(),
            Startup::StatisticalPreview {
                neighbors: 4,
                sigma: 1.,
                ..
            }
        ));
        for value in ["-1", "NaN", "inf"] {
            assert!(
                startup_args(
                    ["--statistical-preview", "cloud.ply", "4", value]
                        .map(Into::into)
                        .to_vec()
                )
                .is_err()
            );
        }
    }
    #[test]
    fn scan_project_export_requires_project_sources_both_recordings_and_output() {
        assert!(matches!(
            startup_args(
                [
                    "--export-scan-project",
                    "project",
                    "sources",
                    "left",
                    "right",
                    "new"
                ]
                .map(Into::into)
                .to_vec()
            )
            .unwrap(),
            Startup::ExportScanProject { .. }
        ));
        assert!(
            startup_args(
                ["--export-scan-project", "project", "sources", "left", "new"]
                    .map(Into::into)
                    .to_vec()
            )
            .is_err()
        );
    }
    #[test]
    fn full_recording_archive_requires_both_sources_and_new_directory() {
        assert!(matches!(
            startup_args(vec![
                "--archive-recordings".into(),
                "left".into(),
                "right".into(),
                "new".into()
            ]),
            Ok(Startup::ArchiveRecordings { .. })
        ));
        assert!(
            startup_args(vec![
                "--archive-recordings".into(),
                "left".into(),
                "right".into()
            ])
            .is_err()
        );
    }
    #[test]
    fn bundle_export_requires_project_source_and_new_output() {
        assert!(matches!(
            startup_args(vec![
                "--export-project-bundle".into(),
                "project".into(),
                "source".into(),
                "new".into()
            ]),
            Ok(Startup::ExportBundle { .. })
        ));
        assert!(
            startup_args(vec![
                "--export-project-bundle".into(),
                "project".into(),
                "source".into()
            ])
            .is_err()
        );
    }
    #[test]
    fn source_verification_requires_explicit_project_and_directory() {
        assert!(matches!(
            startup_args(vec![
                "--verify-source-bundle".into(),
                "project".into(),
                "bundle".into()
            ]),
            Ok(Startup::VerifySources { .. })
        ));
        assert!(startup_args(vec!["--verify-source-bundle".into(), "project".into()]).is_err());
    }
    #[test]
    fn recorded_reconstruction_requires_explicit_numeric_indices() {
        let args = vec![
            "--reconstruct-recordings",
            "calibration",
            "left",
            "1",
            "right",
            "2",
            "settings",
            "out",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        assert!(
            matches!(startup_args(args),Ok(Startup::ReconstructRecordings{inputs,..}) if inputs.left_index==1 && inputs.right_index==2)
        );
        let args = vec![
            "--reconstruct-recordings",
            "calibration",
            "left",
            "-1",
            "right",
            "2",
            "settings",
            "out",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        assert!(startup_args(args).is_err());
    }
    #[test]
    fn recording_playback_startup_is_explicit_and_requires_one_path() {
        assert!(
            matches!(startup_args(vec!["--play-recording".into(),"example.raptor".into()]), Ok(Startup::Playback(p)) if p==std::path::Path::new("example.raptor"))
        );
        assert!(startup_args(vec!["--play-recording".into()]).is_err());
    }
    #[test]
    fn opening_files_is_distinct_from_explicit_hardware_smoke_mode() {
        assert_eq!(startup_args(args(&[])).unwrap(), Startup::Empty);
        assert_eq!(
            startup_args(args(&["project.orproj"])).unwrap(),
            Startup::File("project.orproj".into())
        );
        assert_eq!(
            startup_args(args(&["--smoke-test"])).unwrap(),
            Startup::Smoke
        );
        assert_eq!(
            startup_args(args(&["--", "--smoke-test"])).unwrap(),
            Startup::File("--smoke-test".into())
        );
        assert_eq!(startup_args(args(&["--help"])).unwrap(), Startup::Help);
        assert_eq!(
            startup_args(args(&["--mesh", "mesh.json"])).unwrap(),
            Startup::Mesh("mesh.json".into())
        );
        assert_eq!(
            startup_args(args(&[
                "--import-mesh",
                "source.stl",
                "stl",
                "in",
                "weld-exact",
                "new-import"
            ]))
            .unwrap(),
            Startup::MeshImport {
                input: "source.stl".into(),
                target: "new-import".into(),
                config: scancraft_geometry::mesh_import::Config {
                    format: scancraft_geometry::mesh_import::Format::Stl,
                    units: scancraft_geometry::mesh_import::Units::Inches,
                    weld_exact: true
                }
            }
        );
        for (format, units, welding) in [
            ("bad", "mm", "separate"),
            ("obj", "unknown", "separate"),
            ("obj", "mm", "auto"),
        ] {
            assert!(
                startup_args(args(&[
                    "--import-mesh",
                    "input",
                    format,
                    units,
                    welding,
                    "result"
                ]))
                .is_err()
            );
        }
        assert!(matches!(
            startup_args(args(&[
                "--recover-recording",
                "source.partial",
                "100",
                "new-result"
            ]))
            .unwrap(),
            Startup::RecoverRecording {
                maximum_frames: 100,
                ..
            }
        ));
        assert!(
            startup_args(args(&[
                "--recover-recording",
                "source.partial",
                "bad",
                "new-result"
            ]))
            .is_err()
        );
        assert!(startup_args(args(&["--mesh"])).is_err());
        assert_eq!(
            startup_args(args(&["--mesh-boundaries", "mesh.json"])).unwrap(),
            Startup::MeshBoundaries("mesh.json".into())
        );
        assert!(startup_args(args(&["--mesh-boundaries"])).is_err());
        assert_eq!(
            startup_args(args(&["--mesh-normals", "mesh.json"])).unwrap(),
            Startup::MeshNormals("mesh.json".into())
        );
        assert!(startup_args(args(&["--mesh-normals"])).is_err());
        assert!(matches!(
            startup_args(args(&[
                "--mesh-plane-preview",
                "mesh.json",
                "1",
                "0",
                "0",
                "-0.5"
            ]))
            .unwrap(),
            Startup::MeshPlane {
                normal: [1., 0., 0.],
                offset: -0.5,
                ..
            }
        ));
        assert!(
            startup_args(args(&[
                "--mesh-plane-preview",
                "mesh.json",
                "bad",
                "0",
                "0",
                "0"
            ]))
            .is_err()
        );
        assert_eq!(
            startup_args(args(&["--fuse-sequence", "settings.json", "result"])).unwrap(),
            Startup::FuseSequence {
                settings: "settings.json".into(),
                output: "result".into()
            }
        );
        assert!(startup_args(args(&["--fuse-sequence", "settings.json"])).is_err());
        assert_eq!(
            startup_args(args(&["--smoke-test-ir"])).unwrap(),
            Startup::SmokeIr
        );
        assert!(matches!(
            startup_args(args(&[
                "--reconstruct",
                "calibration",
                "left",
                "right",
                "settings",
                "output"
            ]))
            .unwrap(),
            Startup::Reconstruct { .. }
        ));
        assert!(startup_args(args(&["--reconstruct", "calibration"])).is_err());
        assert!(startup_args(args(&["--unknown"])).is_err());
        assert!(startup_args(args(&["one", "two"])).is_err());
    }
}

#[cfg(test)]
mod capture_recovery_tests {
    use super::*;
    fn running(run: impl FnOnce() -> Result<String, String> + Send + 'static) -> Running {
        let (_sender, rx) = mpsc::channel();
        Running {
            cancel: Arc::new(AtomicBool::new(false)),
            frames: Arc::new(AtomicU64::new(0)),
            preview_drops: Arc::new(AtomicU64::new(0)),
            rx,
            worker: capture_worker::Worker::spawn(run).unwrap(),
        }
    }
    fn finish(app: &mut App) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let context = egui::Context::default();
        while app.running.is_some() {
            app.receive(&context);
            assert!(
                std::time::Instant::now() < deadline,
                "capture failed to leave running state"
            );
            std::thread::yield_now();
        }
    }
    #[test]
    fn capture_panic_restores_restart_state_and_preserves_partial_recording() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "scancraft-panic-{}-{}.raptor.partial",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let profile = StreamProfile {
            interface: 0,
            width: 2,
            height: 2,
            fourcc: *b"GREY",
            fps: 30,
        };
        let writer = Writer::new(file, profile).unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut app = App::default();
        app.running = Some(running(move || {
            let _recording = writer;
            panic!("injected capture failure");
        }));
        finish(&mut app);
        assert_eq!(app.status, "Capture stopped");
        assert!(app.log.contains("panicked"));
        assert!(app.log.contains("incomplete recording"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut reader =
            scancraft_recording::Reader::new(std::fs::File::open(&path).unwrap()).unwrap();
        assert!(
            reader.read_frame().is_err(),
            "partial recording must not become a completed recording"
        );
        // Start/interface controls use running.is_some(); a replacement worker
        // can now complete. This test never opens a scanner or changes controls.
        app.running = Some(running(|| Ok("replacement stopped".into())));
        finish(&mut app);
        assert_eq!(app.log, "replacement stopped");
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn ordinary_capture_error_stops_and_remains_visible() {
        let mut app = App::default();
        app.running = Some(running(|| Err("injected capture error".into())));
        finish(&mut app);
        assert_eq!(app.status, "Capture stopped");
        assert!(app.log.contains("injected capture error"));
    }
}
