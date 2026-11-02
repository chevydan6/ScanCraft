//! Recording-backed, seeded trace correspondence diagnostics. No calibration
//! activation, physical-unit inference, point-cloud export or identity attestation.
mod automatic;
pub use automatic::run as run_automatic;

use crate::{
    Error, artifact, bounded,
    calibration_bundle::ensure_new,
    output::{publish, staging},
    recording_playback::Playback,
    recording_reconstruction::lineage,
};
use scancraft_calibration::{
    epipolar_trace::{self, PreparedTrace},
    orientation::reflect_pixel_x,
    stereo::StereoCalibration,
};
use scancraft_geometry::work::WorkControl;
use scancraft_image::GrayImage;
use scancraft_recording::{Writer, selection::SelectedFrame};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inputs {
    pub geometry: PathBuf,
    pub first_recording: PathBuf,
    pub first_index: usize,
    pub second_recording: PathBuf,
    pub second_index: usize,
    pub request: PathBuf,
}
use scancraft_depth::laser_pair::{Continuity, Detection, Observations};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedFrame {
    interface: u8,
    payload_crc32: u32,
    metadata_crc32: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Seed {
    first_observation: usize,
    second_observation: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    expected_frames: [ExpectedFrame; 2],
    /// Explicit raw-image-to-supplied-camera hypothesis, never inferred from USB names.
    reflect_x: [bool; 2],
    maximum_host_delta_us: u64,
    detection: Detection,
    continuity: Continuity,
    intersection: epipolar_trace::Config,
    seed_radius_px: f64,
    seeds: Vec<Seed>,
}
fn detect(
    frame: &SelectedFrame,
    detection: &Detection,
    continuity: &Continuity,
    work: &WorkControl,
) -> Result<Observations, Error> {
    Ok(scancraft_depth::laser_pair::detect(
        GrayImage::from_frame(&frame.profile, &frame.frame)?,
        detection,
        continuity,
        work,
    )?)
}
fn observation_report(side: &Observations) -> Value {
    json!({"observations":side.detections.observations.iter().map(|o|json!({
        "pixel":o.pixel,"first_x":o.first_x,"last_x":o.last_x,"peak":o.peak,
        "integrated_signal":o.integrated_signal,"variance_x":o.variance_x})).collect::<Vec<_>>(),
        "rejections":{"border":side.detections.rejected_border,"width":side.detections.rejected_width,
            "peak":side.detections.rejected_peak,"saturation":side.detections.rejected_saturation},
        "traces":side.traces.traces.iter().map(|t|json!({"observation_indices":t.observation_indices,
            "span_rows":t.span_rows,"continuity_supported":t.continuity_supported})).collect::<Vec<_>>(),
        "continuity_ambiguous_observations":side.traces.ambiguous_observations,
        "continuity_comparisons":side.traces.comparisons})
}

fn validate_frames(
    frames: &[SelectedFrame; 2],
    geometry: &StereoCalibration,
    expected_frames: &[ExpectedFrame; 2],
    maximum_host_delta_us: u64,
) -> Result<(), Error> {
    for (side, frame) in frames.iter().enumerate() {
        let expected = &expected_frames[side];
        let camera = if side == 0 {
            &geometry.left
        } else {
            &geometry.right
        };
        if frame.profile.interface != expected.interface
            || frame.profile.fourcc != *b"GREY"
            || [frame.profile.width, frame.profile.height] != [camera.width, camera.height]
            || crc32fast::hash(&frame.frame.data) != expected.payload_crc32
            || crc32fast::hash(&frame.frame.metadata) != expected.metadata_crc32
        {
            return Err("selected frame disagrees with expected profile or CRC binding".into());
        }
    }
    let pts = [
        scancraft_core::uvc_pts(&frames[0].frame.metadata)?,
        scancraft_core::uvc_pts(&frames[1].frame.metadata)?,
    ];
    if pts[0] != pts[1]
        || frames[0]
            .frame
            .host_timestamp_us
            .abs_diff(frames[1].frame.host_timestamp_us)
            > maximum_host_delta_us
    {
        return Err("selected frames fail raw-PTS/host-window pairing requirements".into());
    }
    Ok(())
}

/// Recompute detections from whole-file-validated selected recordings. Seeds refer
/// to those raw observation indices, not externally supplied pixel coordinates.
/// Require matching raw PTS and a bounded host-time delta, without claiming known
/// clock domains or simultaneous exposures. Preserve exact inputs and selected
/// frames in a no-overwrite bundle. Filesystem calls are not wall-clock-preempted.
pub fn run(inputs: &Inputs, target: &Path, work: &WorkControl) -> Result<Value, Error> {
    work.check()?;
    ensure_new(target)?;
    let geometry_bytes = bounded(&inputs.geometry, 1_048_576, work)?;
    let geometry: StereoCalibration = serde_json::from_slice(&geometry_bytes)?;
    let prepared = PreparedTrace::new(&geometry)?;
    let request_bytes = bounded(&inputs.request, 1_048_576, work)?;
    let request: Request = serde_json::from_slice(&request_bytes)?;
    if request.schema_version != 1
        || !(1..=4096).contains(&request.seeds.len())
        || !request.seed_radius_px.is_finite()
        || request.seed_radius_px <= 0.
        || request.seed_radius_px > 32.
        || request.maximum_host_delta_us > 1_000_000
        || !matches!(
            request.expected_frames.each_ref().map(|f| f.interface),
            [0, 2] | [2, 0]
        )
    {
        return Err("invalid recorded trace request bounds or interface order".into());
    }
    let mut used_first = HashSet::new();
    let mut used_second = HashSet::new();
    for seed in &request.seeds {
        if !used_first.insert(seed.first_observation)
            || !used_second.insert(seed.second_observation)
        {
            return Err("recorded trace seeds must declare one-to-one observation indices".into());
        }
    }
    let mut first_source = Playback::open(&inputs.first_recording, work)?;
    let mut second_source = Playback::open(&inputs.second_recording, work)?;
    let frames = [
        first_source.select_raw(inputs.first_index, work)?,
        second_source.select_raw(inputs.second_index, work)?,
    ];
    validate_frames(
        &frames,
        &geometry,
        &request.expected_frames,
        request.maximum_host_delta_us,
    )?;
    let sides = [
        detect(&frames[0], &request.detection, &request.continuity, work)?,
        detect(&frames[1], &request.detection, &request.continuity, work)?,
    ];
    let map_pixel = |side: usize, pixel: [f64; 2]| -> Result<[f64; 2], String> {
        if request.reflect_x[side] {
            reflect_pixel_x(
                [frames[side].profile.width, frames[side].profile.height],
                pixel,
            )
        } else {
            Ok(pixel)
        }
    };
    let mut results = Vec::new();
    let mut candidate_pairs = Vec::new();
    for (seed_id, seed) in request.seeds.iter().enumerate() {
        work.check()?;
        let first = sides[0]
            .detections
            .observations
            .get(seed.first_observation)
            .ok_or("first seed observation index out of bounds")?;
        let second = sides[1]
            .detections
            .observations
            .get(seed.second_observation)
            .ok_or("second seed observation index out of bounds")?;
        let (first_trace, _) = sides[0].membership[seed.first_observation];
        let (second_trace, offset) = sides[1].membership[seed.second_observation];
        let trace = &sides[1].traces.traces[second_trace];
        let mut result = json!({"seed_index":seed_id,"first_observation":seed.first_observation,
            "second_seed_observation":seed.second_observation,"first_trace":first_trace,
            "second_trace":second_trace,"candidate_pair":false});
        if !sides[0].traces.traces[first_trace].continuity_supported
            || !trace.continuity_supported
            || trace.observation_indices.len() < 2
        {
            result["reason"] = json!("seed lacks supported non-singleton continuity");
            results.push(result);
            continue;
        }
        let indices = &trace.observation_indices
            [offset.saturating_sub(1)..(offset + 2).min(trace.observation_indices.len())];
        let vertices = indices
            .iter()
            .map(|&i| map_pixel(1, sides[1].detections.observations[i].pixel))
            .collect::<Result<Vec<_>, _>>()?;
        let first_pixel = map_pixel(0, first.pixel)?;
        let second_seed = map_pixel(1, second.pixel)?;
        result["second_trace_vertex_observation_indices"] = json!(indices);
        result["second_trace_vertices_calibration_pixels"] = json!(vertices);
        result["first_calibration_pixel"] = json!(first_pixel);
        match prepared.intersections(first_pixel, &vertices, request.intersection, &mut || {
            work.check().is_err()
        }) {
            Err(error) => {
                work.check()?;
                result["reason"] = json!(error);
            }
            Ok(intersections) => {
                let near = intersections
                    .intersections
                    .iter()
                    .enumerate()
                    .filter_map(|(i, h)| {
                        ((h.right_pixel[0] - second_seed[0])
                            .hypot(h.right_pixel[1] - second_seed[1])
                            <= request.seed_radius_px)
                            .then_some(i)
                    })
                    .collect::<Vec<_>>();
                if intersections.ambiguous_segments.is_empty() && near.len() == 1 {
                    let hit = &intersections.intersections[near[0]];
                    candidate_pairs.push(json!({"seed_index":seed_id,"first_calibration_pixel":first_pixel,
                        "second_calibration_pixel":hit.right_pixel,"second_raw_image_pixel":map_pixel(1,hit.right_pixel)?,
                        "selected_intersection_index":near[0],"interpolated":hit.vertex_index.is_none()}));
                    result["candidate_pair"] = json!(true);
                    result["selected_intersection_index"] = json!(near[0]);
                } else {
                    result["reason"] = json!(
                        "no unique nonambiguous intersection within the supplied seed neighborhood"
                    );
                }
                result["near_seed_intersection_indices"] = json!(near);
                result["intersections"] = serde_json::to_value(intersections)?;
            }
        }
        results.push(result);
    }
    first_source.check_source()?;
    second_source.check_source()?;
    work.check()?;
    let sources = [
        lineage(&inputs.first_recording, first_source.version(), &frames[0])?,
        lineage(
            &inputs.second_recording,
            second_source.version(),
            &frames[1],
        )?,
    ];
    let report = json!({"schema_version":1,"kind":"recorded_seeded_trace_correspondence",
        "scope":"candidate image correspondences only; no reconstructed geometry",
        "interpolation_space":"undistorted normalized coordinates, projected back to camera pixels",
        "observation_coordinates":"raw image pixel centers, x right and y down",
        "signal_units":if request.detection.opening_radius.is_some(){"raw minus row morphological opening, 8-bit contrast"}else{"original raw 8-bit intensity"},
        "saturation_units":"original raw 8-bit intensity",
        "sources":sources,"reflect_x":request.reflect_x,"raw_sensor_pts_equal":true,
        "clock_domains_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,
        "geometry_binding_verified":false,"geometry_translation_units_verified":false,"projector_line_identity_verified":false,
        "calibration_activated":false,"candidate_pairs":candidate_pairs,"seed_results":results,
        "first":observation_report(&sides[0]),"second":observation_report(&sides[1])});
    let partial = staging(target)?;
    let manifest=(||->Result<Value,Error>{
        let mut artifacts=Vec::new();
        for (name,bytes) in [("geometry.json",&geometry_bytes),("request.json",&request_bytes)] {
            artifacts.push(artifact(&partial,name,work,|out|{out.write_all(bytes)?;Ok(())})?);
        }
        for (name,frame) in ["first-frame.raptor","second-frame.raptor"].into_iter().zip(&frames) {
            artifacts.push(artifact(&partial,name,work,|out|{
                let mut writer=Writer::with_context(out,frame.profile.clone(),frame.context.clone().unwrap_or_default())?;
                writer.push(&frame.frame)?;writer.finish()?;Ok(())
            })?);
        }
        artifacts.push(artifact(&partial,"correspondences.json",work,|out|{serde_json::to_writer_pretty(out,&report)?;Ok(())})?);
        let manifest=json!({"schema_version":1,"kind":"recorded_seeded_trace_correspondence",
            "seed_count":request.seeds.len(),"candidate_pairs":report["candidate_pairs"].as_array().unwrap().len(),
            "artifacts":artifacts,"physical_accuracy_qualified":false,"projector_line_identity_verified":false,
            "geometry_binding_verified":false,"calibration_activated":false});
        artifact(&partial,"manifest.json",work,|out|{serde_json::to_writer_pretty(out,&manifest)?;Ok(())})?;
        File::open(&partial)?.sync_all()?;first_source.check_source()?;second_source.check_source()?;
        work.check()?;publish(&partial,target)?;Ok(manifest)
    })().map_err(|e|format!("{e}; incomplete correspondence bundle retained at {}",partial.display()))?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent)?.sync_all().map_err(|e| {
        format!(
            "bundle published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_calibration::{Brown5, CameraModel, stereo::RigidPose};
    use scancraft_core::{Frame, StreamProfile};
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    pub(super) struct Temp(pub(super) PathBuf);
    impl Temp {
        pub(super) fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let p = std::env::temp_dir().join(format!(
                "scancraft-recorded-traces-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    pub(super) fn fixture(temp: &Temp) -> (Inputs, Value) {
        let camera = CameraModel {
            width: 32,
            height: 32,
            fx: 100.,
            fy: 100.,
            cx: 15.5,
            cy: 15.5,
            skew: 0.,
            distortion: Brown5::ZERO,
            valid_radius: 1.,
        };
        let mut geometry = StereoCalibration {
            left: camera.clone(),
            right: camera,
            right_from_left: RigidPose {
                rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                translation_mm: [-10., 0., 0.],
            },
        };
        geometry.right.cy = 15.;
        let inputs = Inputs {
            geometry: temp.0.join("geometry.json"),
            first_recording: temp.0.join("first.raptor"),
            first_index: 0,
            second_recording: temp.0.join("second.raptor"),
            second_index: 0,
            request: temp.0.join("request.json"),
        };
        fs::write(
            &inputs.geometry,
            serde_json::to_vec_pretty(&geometry).unwrap(),
        )
        .unwrap();
        let mut expected = Vec::new();
        for (interface, x, path) in [
            (0, 20, &inputs.first_recording),
            (2, 16, &inputs.second_recording),
        ] {
            let mut data = vec![0; 32 * 32];
            for y in 0..32 {
                data[y * 32 + x] = 80;
            }
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&100u32.to_le_bytes());
            expected.push(json!({"interface":interface,"payload_crc32":crc32fast::hash(&data),"metadata_crc32":crc32fast::hash(&metadata)}));
            let mut writer = Writer::new(
                Vec::new(),
                StreamProfile {
                    interface,
                    width: 32,
                    height: 32,
                    fourcc: *b"GREY",
                    fps: 30,
                },
            )
            .unwrap();
            for i in 0..2 {
                metadata[12..16].copy_from_slice(&(100u32 + i).to_le_bytes());
                writer
                    .push(&Frame {
                        sequence: i,
                        host_timestamp_us: 1000 + 33333 * u64::from(i),
                        flags: 0,
                        metadata: metadata.clone(),
                        data: data.clone(),
                    })
                    .unwrap();
            }
            fs::write(path, writer.finish().unwrap()).unwrap();
        }
        let request = json!({"schema_version":1,"expected_frames":expected,"reflect_x":[false,false],"maximum_host_delta_us":1000,
            "detection":{"background":10,"minimum_peak":30,"maximum_width":3,"saturation":255,"maximum_observations":1000,"opening_radius":null},
            "continuity":{"maximum_row_gap":2,"maximum_horizontal_step":1.,"maximum_slope":0.,"minimum_span_rows":5,"minimum_observations":4},
            "intersection":{"maximum_segment_length_px":3.,"normalized_tolerance":1e-10,"minimum_plane_sine":1e-8},
            "seed_radius_px":2.,"seeds":[{"first_observation":10,"second_observation":10}]});
        fs::write(
            &inputs.request,
            serde_json::to_vec_pretty(&request).unwrap(),
        )
        .unwrap();
        (inputs, request)
    }
    #[test]
    fn bundle_preserves_frames_sources_candidates_and_interpolation_lineage() {
        let temp = Temp::new();
        let (inputs, _) = fixture(&temp);
        let target = temp.0.join("out");
        let manifest = run(&inputs, &target, &WorkControl::default()).unwrap();
        assert_eq!(manifest["candidate_pairs"], 1);
        assert_eq!(
            fs::read(&inputs.geometry).unwrap(),
            fs::read(target.join("geometry.json")).unwrap()
        );
        assert_eq!(
            fs::read(&inputs.request).unwrap(),
            fs::read(target.join("request.json")).unwrap()
        );
        let report: Value =
            serde_json::from_slice(&fs::read(target.join("correspondences.json")).unwrap())
                .unwrap();
        assert_eq!(
            report["first"]["observations"].as_array().unwrap().len(),
            32
        );
        assert_eq!(
            report["second"]["observations"].as_array().unwrap().len(),
            32
        );
        assert_eq!(report["candidate_pairs"][0]["interpolated"], true);
        assert!(
            (report["candidate_pairs"][0]["second_raw_image_pixel"][1]
                .as_f64()
                .unwrap()
                - 9.5)
                .abs()
                < 1e-9
        );
        assert_eq!(
            report["seed_results"][0]["second_trace_vertex_observation_indices"],
            json!([9, 10, 11])
        );
        assert_eq!(report["synchronization_verified"], false);
        assert_eq!(report["geometry_binding_verified"], false);
        for artifact in manifest["artifacts"].as_array().unwrap() {
            let bytes = fs::read(target.join(artifact["role"].as_str().unwrap())).unwrap();
            assert_eq!(artifact["bytes"], bytes.len());
            assert_eq!(artifact["crc32"], crc32fast::hash(&bytes));
        }
        for (original, saved) in [
            (&inputs.first_recording, "first-frame.raptor"),
            (&inputs.second_recording, "second-frame.raptor"),
        ] {
            let mut source =
                scancraft_recording::Reader::new(File::open(original).unwrap()).unwrap();
            let mut copy =
                scancraft_recording::Reader::new(File::open(target.join(saved)).unwrap()).unwrap();
            let a = source.read_frame().unwrap().unwrap();
            let b = copy.read_frame().unwrap().unwrap();
            assert_eq!(a.data, b.data);
            assert_eq!(a.metadata, b.metadata);
            assert_eq!(a.host_timestamp_us, b.host_timestamp_us);
            assert!(copy.read_frame().unwrap().is_none());
        }
        let original = fs::read(target.join("manifest.json")).unwrap();
        assert!(run(&inputs, &target, &WorkControl::default()).is_err());
        assert_eq!(original, fs::read(target.join("manifest.json")).unwrap());
    }
    #[test]
    fn mismatched_bindings_pairing_indices_and_duplicate_seeds_fail_before_publication() {
        let temp = Temp::new();
        let (mut inputs, request) = fixture(&temp);
        let target = temp.0.join("out");
        let mut bad = request.clone();
        bad["expected_frames"][0]["payload_crc32"] = json!(0);
        fs::write(&inputs.request, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(
            run(&inputs, &target, &WorkControl::default())
                .unwrap_err()
                .to_string()
                .contains("CRC binding")
        );
        bad = request.clone();
        bad["seeds"] = json!([{"first_observation":10,"second_observation":10},{"first_observation":10,"second_observation":11}]);
        fs::write(&inputs.request, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(
            run(&inputs, &target, &WorkControl::default())
                .unwrap_err()
                .to_string()
                .contains("one-to-one")
        );
        bad = request.clone();
        bad["seeds"][0]["first_observation"] = json!(1000);
        fs::write(&inputs.request, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(run(&inputs, &target, &WorkControl::default()).is_err());
        // Correct second-frame CRC, but intentionally different PTS/host selection.
        let mut source = Playback::open(&inputs.second_recording, &WorkControl::default()).unwrap();
        let second = source.select_raw(1, &WorkControl::default()).unwrap();
        bad = request;
        bad["expected_frames"][1]["metadata_crc32"] =
            json!(crc32fast::hash(&second.frame.metadata));
        fs::write(&inputs.request, serde_json::to_vec(&bad).unwrap()).unwrap();
        inputs.second_index = 1;
        assert!(
            run(&inputs, &target, &WorkControl::default())
                .unwrap_err()
                .to_string()
                .contains("pairing")
        );
        assert!(!target.exists());
    }
    #[test]
    fn late_recording_corruption_and_precancellation_never_publish() {
        let temp = Temp::new();
        let (inputs, _) = fixture(&temp);
        let target = temp.0.join("out");
        let work = WorkControl::default();
        work.cancel();
        assert!(run(&inputs, &target, &work).is_err());
        let mut bytes = fs::read(&inputs.second_recording).unwrap();
        bytes.pop();
        fs::write(&inputs.second_recording, bytes).unwrap();
        assert!(run(&inputs, &target, &WorkControl::default()).is_err());
        assert!(!target.exists());
    }
    #[test]
    fn unsupported_continuity_and_coincident_segments_remain_rejected_evidence() {
        let temp = Temp::new();
        let (inputs, request) = fixture(&temp);
        let mut unsupported = request.clone();
        unsupported["continuity"]["minimum_observations"] = json!(33);
        fs::write(&inputs.request, serde_json::to_vec(&unsupported).unwrap()).unwrap();
        let target = temp.0.join("unsupported");
        assert_eq!(
            run(&inputs, &target, &WorkControl::default()).unwrap()["candidate_pairs"],
            0
        );
        let report: Value =
            serde_json::from_slice(&fs::read(target.join("correspondences.json")).unwrap())
                .unwrap();
        assert!(
            report["seed_results"][0]["reason"]
                .as_str()
                .unwrap()
                .contains("continuity")
        );
        assert_eq!(
            report["second"]["observations"].as_array().unwrap().len(),
            32
        );
        let mut geometry: StereoCalibration =
            serde_json::from_slice(&fs::read(&inputs.geometry).unwrap()).unwrap();
        geometry.right_from_left.translation_mm = [0., -10., 0.];
        geometry.right.cx = 11.5;
        fs::write(&inputs.geometry, serde_json::to_vec(&geometry).unwrap()).unwrap();
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let target = temp.0.join("coincident");
        assert_eq!(
            run(&inputs, &target, &WorkControl::default()).unwrap()["candidate_pairs"],
            0
        );
        let report: Value =
            serde_json::from_slice(&fs::read(target.join("correspondences.json")).unwrap())
                .unwrap();
        assert_eq!(
            report["seed_results"][0]["intersections"]["ambiguous_segments"],
            json!([0, 1])
        );
    }
    #[test]
    fn explicit_reflection_changes_geometry_pixels_but_not_raw_observations() {
        let temp = Temp::new();
        let (inputs, mut request) = fixture(&temp);
        request["reflect_x"] = json!([true, true]);
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let target = temp.0.join("reflected");
        assert_eq!(
            run(&inputs, &target, &WorkControl::default()).unwrap()["candidate_pairs"],
            1
        );
        let report: Value =
            serde_json::from_slice(&fs::read(target.join("correspondences.json")).unwrap())
                .unwrap();
        assert_eq!(
            report["first"]["observations"][10]["pixel"],
            json!([20., 10.])
        );
        assert_eq!(
            report["candidate_pairs"][0]["first_calibration_pixel"],
            json!([11., 10.])
        );
        assert_eq!(
            report["candidate_pairs"][0]["second_calibration_pixel"][0],
            15.
        );
        assert_eq!(
            report["candidate_pairs"][0]["second_raw_image_pixel"][0],
            16.
        );
    }
}
