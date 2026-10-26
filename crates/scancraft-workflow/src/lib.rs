//! Shared bounded, cancellable offline reconstruction and atomic bundle publication.
pub mod frame_document;
pub mod mesh_document;
pub mod mesh_history;
pub mod mesh_trim;
pub mod meshing;
pub mod output;
pub mod scan_project;
pub mod scan_project_load;
pub mod section_document;
pub mod sequence;
pub mod statistical;
pub mod stereo_surface;
use crate::output::{publish, staging};
use image::{ColorType, ImageDecoder, codecs::png::PngDecoder};
use scancraft_calibration::{CameraModel, stereo::TriangulationLimits, storage};
use scancraft_depth::{
    MatchSource,
    matching::{AutoSeeds, MatchConfig},
    pipeline::{StereoProcessor, StereoReport, StereoRequest},
};
use scancraft_geometry::{
    Cloud, Editor,
    formats::save_project,
    work::{Stage, WorkControl},
    write_ply,
};
use scancraft_image::{GrayImage, MAX_PIXELS};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Cursor, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
pub type Error = Box<dyn std::error::Error + Send + Sync>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    source: MatchSource,
    output_grid: CameraModel,
    #[serde(default)]
    seeds: Vec<[u32; 2]>,
    #[serde(default)]
    automatic_sampling: Option<AutoSeeds>,
    matching: MatchConfig,
    triangulation: TriangulationLimits,
    #[serde(default)]
    meshing: Option<stereo_surface::SurfaceConfig>,
    #[serde(default)]
    surface_normals: Option<scancraft_geometry::mesh_normals::Config>,
}
#[derive(Debug, Serialize)]
pub struct Digest {
    role: String,
    bytes: u64,
    crc32: u32,
}
#[derive(Debug, Serialize)]
pub struct Summary {
    pub schema_version: u32,
    pub status: &'static str,
    pub points: usize,
    pub mesh_triangles: usize,
    pub rejected_seeds: usize,
    pub mapping_failures: usize,
    pub rejected_geometry: usize,
    pub synchronization_verified: bool,
    pub physical_accuracy_qualified: bool,
    pub inputs: Vec<Digest>,
    pub artifacts: Vec<Digest>,
}
fn bounded(path: &Path, limit: usize, work: &WorkControl) -> Result<Vec<u8>, Error> {
    work.check()?;
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > limit as u64 {
        return Err(format!(
            "{} must be a regular file of at most {limit} bytes",
            path.display()
        )
        .into());
    }
    let mut data = Vec::new();
    let mut buffer = [0; 65536];
    loop {
        work.update(Stage::Reading, data.len(), metadata.len() as usize)?;
        let remaining = (limit + 1 - data.len()).min(buffer.len());
        let n = file.read(&mut buffer[..remaining])?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..n]);
        if data.len() > limit {
            return Err(format!("{} exceeds {limit} byte input limit", path.display()).into());
        }
    }
    work.check()?;
    Ok(data)
}

fn digest(role: &str, bytes: &[u8]) -> Digest {
    Digest {
        role: role.into(),
        bytes: bytes.len() as u64,
        crc32: crc32fast::hash(bytes),
    }
}
fn gray_png(bytes: &[u8], expected: [u32; 2]) -> Result<Vec<u8>, Error> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(expected[0].min(MAX_PIXELS as u32));
    limits.max_image_height = Some(expected[1].min(MAX_PIXELS as u32));
    limits.max_alloc = Some(32 * 1024 * 1024);
    let decoder = PngDecoder::with_limits(Cursor::new(bytes), limits)?;
    let (width, height) = decoder.dimensions();
    if [width, height] != expected
        || u64::from(width) * u64::from(height) > MAX_PIXELS as u64
        || decoder.color_type() != ColorType::L8
    {
        return Err("input must be an 8-bit grayscale PNG with exact source calibration dimensions and bounded pixel count".into());
    }
    let mut pixels = vec![0; decoder.total_bytes() as usize];
    decoder.read_image(&mut pixels)?;
    Ok(pixels)
}
fn artifact(
    path: &Path,
    name: &str,
    work: &WorkControl,
    write: impl FnOnce(&mut CheckedWriter<'_>) -> Result<(), Error>,
) -> Result<Digest, Error> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path.join(name))?;
    let mut out = CheckedWriter {
        inner: BufWriter::new(file),
        work,
        bytes: 0,
    };
    write(&mut out)?;
    out.flush()?;
    out.inner.get_ref().sync_all()?;
    drop(out);
    let mut file = File::open(path.join(name))?;
    let mut bytes = 0;
    let mut hash = crc32fast::Hasher::new();
    let mut buffer = [0; 65536];
    loop {
        work.check()?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        bytes += n as u64;
    }
    Ok(Digest {
        role: name.into(),
        bytes,
        crc32: hash.finalize(),
    })
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInputs {
    pub calibration: PathBuf,
    pub left: PathBuf,
    pub right: PathBuf,
    pub settings: PathBuf,
}
pub struct Prepared {
    report: StereoReport,
    cloud: Option<Cloud>,
    normal_cloud: Option<Cloud>,
    surface: Option<stereo_surface::StereoSurface>,
    inputs: Vec<Digest>,
    recording_sources: Option<recording_reconstruction::Sources>,
}
impl Prepared {
    /// Preserve initial recorded inputs in both the saved and live editable document.
    pub fn create_editor(&self, bundle: &Path) -> Result<Editor, Error> {
        let cloud = self.cloud.as_ref().ok_or("reconstruction has no cloud")?;
        let mut editor = Editor::new(cloud.clone(), 32)?;
        if let Some(sources) = &self.recording_sources {
            editor.attach_origin(sources.project_origin(&self.report.calibration, bundle)?)?;
        }
        Ok(editor)
    }
    pub fn cloud(&self) -> Option<&Cloud> {
        self.cloud.as_ref()
    }
    pub fn report(&self) -> &StereoReport {
        &self.report
    }
}
pub fn prepare(files: &FileInputs, work: &WorkControl) -> Result<Prepared, Error> {
    let calibration_bytes = bounded(&files.calibration, 1024 * 1024 + 16, work)?;
    let calibration = storage::read(calibration_bytes.as_slice())?;
    let left_bytes = bounded(&files.left, 16 * 1024 * 1024, work)?;
    let right_bytes = bounded(&files.right, 16 * 1024 * 1024, work)?;
    let request_bytes = bounded(&files.settings, 4 * 1024 * 1024, work)?;
    let request: Request = serde_json::from_slice(&request_bytes)?;
    if request.schema_version != 1 {
        return Err("unsupported reconstruction request schema".into());
    }
    work.check()?;
    let left = gray_png(
        &left_bytes,
        [
            calibration.geometry.left.width,
            calibration.geometry.left.height,
        ],
    )?;
    work.check()?;
    let right = gray_png(
        &right_bytes,
        [
            calibration.geometry.right.width,
            calibration.geometry.right.height,
        ],
    )?;
    work.check()?;
    process_images(
        calibration,
        request,
        &left,
        &right,
        vec![
            digest("calibration", &calibration_bytes),
            digest("left_png", &left_bytes),
            digest("right_png", &right_bytes),
            digest("request", &request_bytes),
        ],
        work,
    )
}
fn process_images(
    calibration: storage::CalibrationRecord,
    request: Request,
    left: &[u8],
    right: &[u8],
    inputs: Vec<Digest>,
    work: &WorkControl,
) -> Result<Prepared, Error> {
    let processor = StereoProcessor::build(calibration, request.output_grid, work)?;
    let source = processor.rectification().source();
    let stereo_request = StereoRequest {
        left: GrayImage::new(source.left.width, source.left.height, left)?,
        right: GrayImage::new(source.right.width, source.right.height, right)?,
        source: request.source,
        seeds: &request.seeds,
        matching: request.matching,
        triangulation: request.triangulation,
    };
    let report = match request.automatic_sampling {
        Some(sampling) => processor.process_automatic(stereo_request, sampling, work)?,
        None => processor.process(stereo_request, work)?,
    };
    let cloud = report.reconstruction.as_ref().map(|r| r.cloud());
    let mut normal_cloud = None;
    let mut surface = request
        .meshing
        .map(|config| stereo_surface::build(&report, config, work))
        .transpose()?;
    if let Some(config) = request.surface_normals {
        let surface = surface
            .as_mut()
            .ok_or("surface_normals requires explicit meshing")?;
        let normals = scancraft_geometry::mesh_normals::estimate(&surface.grid.mesh, config, work)?;
        let mut supported = Vec::new();
        let mut supported_indices = Vec::new();
        if let Some(cloud) = cloud.as_ref() {
            if surface.vertex_cloud_indices.len() != normals.normals.len() {
                return Err("normal lineage length mismatch".into());
            }
            for (vertex, (index, normal)) in surface
                .vertex_cloud_indices
                .iter()
                .zip(&normals.normals)
                .enumerate()
            {
                let point = cloud
                    .points
                    .get(*index)
                    .ok_or("normal cloud lineage out of bounds")?;
                if point.position != surface.grid.mesh.vertices[vertex] {
                    return Err("normal cloud lineage position mismatch".into());
                }
                if let Some(normal) = normal {
                    let mut point = point.clone();
                    point.normal = Some(*normal);
                    supported.push(point);
                    supported_indices.push(*index);
                }
            }
        }
        normal_cloud = Some(Cloud { points: supported });
        surface.diagnostics["surface_normals"] = serde_json::json!({"config":config,"estimated":normals.estimated,
            "cloud_indices":supported_indices,"artifact":(normals.estimated>0).then_some("surface-normal-points.ply"),
            "unsupported":normals.unsupported,"method":"angle-weighted explicit indexed adjacency","physical_accuracy_qualified":false});
    }
    work.check()?;
    Ok(Prepared {
        report,
        cloud,
        normal_cloud,
        surface,
        inputs,
        recording_sources: None,
    })
}
/// The final cancellation check precedes atomic publication. After publication,
/// success is committed; a later cancellation request does not retract artifacts.
pub fn publish_prepared(
    prepared: &Prepared,
    target: &Path,
    work: &WorkControl,
) -> Result<Summary, Error> {
    work.check()?;
    if fs::symlink_metadata(target).is_ok() {
        return Err("output directory already exists; choose a new path".into());
    }
    let cloud = prepared.cloud();
    let report = prepared.report();
    let points = cloud.as_ref().map_or(0, |c| c.points.len());
    let mut summary = Summary {
        schema_version: 1,
        status: if points > 0 {
            "geometry_produced"
        } else {
            "no_geometry"
        },
        points,
        mesh_triangles: prepared
            .surface
            .as_ref()
            .map_or(0, |s| s.grid.mesh.triangles.len()),
        rejected_seeds: report.matching.rejected.len(),
        mapping_failures: report.mapping_failures.len(),
        rejected_geometry: report
            .reconstruction
            .as_ref()
            .map_or(0, |r| r.rejected.len()),
        synchronization_verified: false,
        physical_accuracy_qualified: false,
        inputs: prepared
            .inputs
            .iter()
            .map(|d| Digest {
                role: d.role.clone(),
                bytes: d.bytes,
                crc32: d.crc32,
            })
            .collect(),
        artifacts: Vec::new(),
    };
    let partial = staging(target)?;
    let result = (|| -> Result<(), Error> {
        summary
            .artifacts
            .push(artifact(&partial, "report.json", work, |out| {
                serde_json::to_writer_pretty(out, &report)?;
                Ok(())
            })?);
        if let Some(sources) = &prepared.recording_sources {
            sources.write_artifacts(&partial, work, &mut summary.artifacts)?;
        }
        if let Some(cloud) = cloud.filter(|c| !c.points.is_empty()) {
            summary
                .artifacts
                .push(artifact(&partial, "points.ply", work, |out| {
                    write_ply(cloud, out)?;
                    Ok(())
                })?);
            let editor = prepared.create_editor(target)?;
            summary
                .artifacts
                .push(artifact(&partial, "project.orproj", work, |out| {
                    save_project(&editor, out)?;
                    Ok(())
                })?);
        }
        if let Some(cloud) = prepared
            .normal_cloud
            .as_ref()
            .filter(|c| !c.points.is_empty())
        {
            summary.artifacts.push(artifact(
                &partial,
                "surface-normal-points.ply",
                work,
                |out| {
                    write_ply(cloud, out)?;
                    Ok(())
                },
            )?);
        }
        if let Some(surface) = &prepared.surface {
            summary
                .artifacts
                .push(artifact(&partial, "mesh-report.json", work, |out| {
                    serde_json::to_writer_pretty(out, &surface.diagnostics)?;
                    Ok(())
                })?);
            if !surface.grid.mesh.triangles.is_empty() {
                summary.artifacts.push(artifact(&partial,"mesh.json",work,|out|{serde_json::to_writer_pretty(out,&serde_json::json!({"schema_version":1,"units":"millimeters","mesh":surface.grid.mesh}))?;Ok(())})?);
                summary
                    .artifacts
                    .push(artifact(&partial, "mesh.stl", work, |out| {
                        surface.grid.mesh.write_stl(out, work)?;
                        Ok(())
                    })?);
                summary
                    .artifacts
                    .push(artifact(&partial, "mesh.obj", work, |out| {
                        surface.grid.mesh.write_obj(out, work)?;
                        Ok(())
                    })?);
            }
        }
        artifact(&partial, "manifest.json", work, |out| {
            serde_json::to_writer_pretty(out, &summary)?;
            Ok(())
        })?;
        File::open(&partial)?.sync_all()?;
        work.check()?;
        publish(&partial, target)?;
        Ok(())
    })();
    if let Err(e) = result {
        return Err(format!("{e}; incomplete output preserved at {}", partial.display()).into());
    }
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "bundle published at {}, parent directory sync failed: {e}",
            target.display()
        )
    })?;
    Ok(summary)
}

struct CheckedWriter<'a> {
    inner: BufWriter<File>,
    work: &'a WorkControl,
    bytes: usize,
}
impl Write for CheckedWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.work
            .update(Stage::Writing, self.bytes, 0)
            .map_err(io::Error::other)?;
        let n = self.inner.write(bytes)?;
        self.bytes += n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.work.check().map_err(io::Error::other)?;
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;
    fn fixture() -> FileInputs {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
        FileInputs {
            calibration: root.join("calibration.rapcal"),
            left: root.join("left.png"),
            right: root.join("right.png"),
            settings: root.join("request.json"),
        }
    }
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            Self(staging(&std::env::temp_dir().join("scancraft-workflow-test")).unwrap())
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn cancellation_after_preparation_never_publishes_an_output_bundle() {
        let work = WorkControl::default();
        let prepared = prepare(&fixture(), &work).unwrap();
        assert_eq!(prepared.cloud().unwrap().points.len(), 3);
        let temp = Temp::new();
        let target = temp.0.join("result");
        work.cancel();
        assert!(
            publish_prepared(&prepared, &target, &work)
                .unwrap_err()
                .to_string()
                .contains("cancelled")
        );
        assert!(!target.exists());
        assert!(temp.0.read_dir().unwrap().next().is_none());
    }
    #[test]
    fn input_limits_and_special_files_fail_without_blocking() {
        use std::os::unix::ffi::OsStrExt;
        let temp = Temp::new();
        let work = WorkControl::default();
        let path = temp.0.join("large");
        fs::write(&path, [0; 16]).unwrap();
        assert!(bounded(&path, 8, &work).is_err());
        assert!(bounded(&temp.0, 8, &work).is_err());
        let fifo = temp.0.join("fifo");
        let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        // SAFETY: owned NUL-terminated pathname remains live through this syscall.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(bounded(&fifo, 1024, &work).is_err());
        work.cancel();
        assert!(
            prepare(&fixture(), &work)
                .err()
                .unwrap()
                .to_string()
                .contains("cancelled")
        );
    }
    #[test]
    fn artifact_writer_stops_accepting_bytes_after_cancellation() {
        let temp = Temp::new();
        let path = temp.0.join("partial");
        let work = WorkControl::default();
        let mut writer = CheckedWriter {
            inner: BufWriter::new(File::create(&path).unwrap()),
            work: &work,
            bytes: 0,
        };
        writer.write_all(b"accepted").unwrap();
        work.cancel();
        assert!(
            writer
                .write_all(b"rejected")
                .unwrap_err()
                .to_string()
                .contains("cancelled")
        );
        assert!(writer.flush().is_err());
        drop(writer);
        assert_eq!(fs::read(path).unwrap(), b"accepted");
    }
    #[test]
    fn png_validation_rejects_color_mismatched_truncated_and_huge_headers() {
        let mut gray = Vec::new();
        image::codecs::png::PngEncoder::new(&mut gray)
            .write_image(&[1, 2, 3, 4], 2, 2, ColorType::L8.into())
            .unwrap();
        assert_eq!(gray_png(&gray, [2, 2]).unwrap(), [1, 2, 3, 4]);
        assert!(gray_png(&gray, [1, 1]).is_err());
        let mut rgb = Vec::new();
        image::codecs::png::PngEncoder::new(&mut rgb)
            .write_image(&[0; 12], 2, 2, ColorType::Rgb8.into())
            .unwrap();
        assert!(gray_png(&rgb, [2, 2]).is_err());
        assert!(gray_png(&gray[..20], [2, 2]).is_err());
        gray[16..20].copy_from_slice(&800_000u32.to_be_bytes());
        gray[20..24].copy_from_slice(&800_000u32.to_be_bytes());
        let crc = crc32fast::hash(&gray[12..29]);
        gray[29..33].copy_from_slice(&crc.to_be_bytes());
        assert!(gray_png(&gray, [800_000, 800_000]).is_err());
    }
    #[test]
    fn atomic_publication_refuses_a_target_that_appears_after_staging() {
        let root =
            std::env::temp_dir().join(format!("scancraft-publish-race-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let target = root.join("result");
        let partial = staging(&target).unwrap();
        fs::write(partial.join("sentinel"), b"original").unwrap();
        fs::create_dir(&target).unwrap();
        assert_eq!(
            publish(&partial, &target).unwrap_err().kind(),
            std::io::ErrorKind::AlreadyExists
        );
        assert!(target.read_dir().unwrap().next().is_none());
        assert_eq!(fs::read(partial.join("sentinel")).unwrap(), b"original");
        fs::remove_dir(&target).unwrap();
        publish(&partial, &target).unwrap();
        assert_eq!(fs::read(target.join("sentinel")).unwrap(), b"original");
        fs::remove_dir_all(root).unwrap();
    }
}

pub mod recording_recovery;

pub mod recording_playback;

pub mod recording_reconstruction;

pub mod source_verification;

pub mod project_bundle;

pub mod recording_archive;

pub mod recording_timing;
