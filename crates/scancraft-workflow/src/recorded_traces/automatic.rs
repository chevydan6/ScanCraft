//! Record-backed automatic sheet-supported matching; diagnostic coordinates only.
use super::{
    Continuity, Detection, ExpectedFrame, Inputs, detect, observation_report, validate_frames,
};
use crate::{
    Error, artifact, bounded,
    calibration_bundle::ensure_new,
    output::{publish, staging},
    recording_playback::Playback,
    recording_reconstruction::lineage,
};
use scancraft_calibration::{
    laser_surface::QuadraticSheet,
    laser_trace::{self, PreparedMatcher},
    orientation::reflect_pixel_x,
    stereo::{StereoCalibration, TriangulationLimits},
};
use scancraft_geometry::work::{Stage, WorkControl};
use scancraft_recording::Writer;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    expected_frames: [ExpectedFrame; 2],
    reflect_x: [bool; 2],
    maximum_host_delta_us: u64,
    detection: Detection,
    continuity: Continuity,
    sheets: Vec<QuadraticSheet>,
    triangulation: TriangulationLimits,
    matching: laser_trace::Config,
    maximum_queries: usize,
    /// Conservative matching bound. Try vertices + segments * twice the sheet
    /// count, then actual projected roots if needed, before any matching begins.
    maximum_total_work: usize,
    maximum_report_bytes: usize,
}

struct LimitedWriter<'a, W> {
    inner: &'a mut W,
    remaining: usize,
}
impl<W: Write> Write for LimitedWriter<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(io::Error::other("matching report exceeds byte budget"));
        }
        let n = self.inner.write(bytes)?;
        self.remaining -= n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Process all continuity-supported first-camera observations against all supported
/// second traces. Retain detections, rejected queries and interpolation lineage.
/// Result coordinates use unqualified supplied geometry units; no PLY/project or
/// physical calibration is created. Filesystem calls have no hard time bound.
pub fn run(inputs: &Inputs, target: &Path, work: &WorkControl) -> Result<Value, Error> {
    work.check()?;
    ensure_new(target)?;
    let geometry_bytes = bounded(&inputs.geometry, 1_048_576, work)?;
    let geometry: StereoCalibration = serde_json::from_slice(&geometry_bytes)?;
    let request_bytes = bounded(&inputs.request, 1_048_576, work)?;
    let request: Request = serde_json::from_slice(&request_bytes)?;
    request.matching.validate()?;
    if request.schema_version != 1
        || request.maximum_host_delta_us > 1_000_000
        || !matches!(
            request.expected_frames.each_ref().map(|f| f.interface),
            [0, 2] | [2, 0]
        )
        || !(1..=128).contains(&request.sheets.len())
        || !(1..=4096).contains(&request.maximum_queries)
        || !(1..=100_000_000).contains(&request.maximum_total_work)
        || !(1..=134_217_728).contains(&request.maximum_report_bytes)
        || request.sheets.iter().any(|s| {
            ![s.quadratic_y, s.linear_y, s.linear_z, s.offset]
                .iter()
                .all(|v| v.is_finite())
        })
    {
        return Err("invalid automatic recorded trace request bounds".into());
    }
    let matcher = PreparedMatcher::new(&geometry, request.triangulation)?;
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
    let first_indices = sides[0]
        .traces
        .traces
        .iter()
        .filter(|t| t.continuity_supported)
        .flat_map(|t| t.observation_indices.iter().copied())
        .collect::<Vec<_>>();
    if first_indices.len() > request.maximum_queries {
        return Err(
            "automatic trace query limit exceeded; no observations were silently sampled".into(),
        );
    }
    let trace_ids = sides[1]
        .traces
        .traces
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.continuity_supported.then_some(i))
        .collect::<Vec<_>>();
    if trace_ids.len() > 128 {
        return Err("automatic second-trace limit exceeded".into());
    }
    let mut traces = Vec::new();
    let mut vertices = 0usize;
    let mut segments = 0usize;
    for &id in &trace_ids {
        work.check()?;
        let indices = &sides[1].traces.traces[id].observation_indices;
        if !(2..=4096).contains(&indices.len()) {
            return Err("unsupported second-trace vertex count".into());
        }
        vertices += indices.len();
        segments += indices.len() - 1;
        traces.push(
            indices
                .iter()
                .map(|&i| map_pixel(1, sides[1].detections.observations[i].pixel))
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    if vertices > 65536 {
        return Err("automatic trace vertex limit exceeded".into());
    }
    let prepared_queries = if traces.is_empty() {
        None
    } else {
        Some(
            matcher.prepare_traces(&traces, &request.sheets, request.matching, &mut || {
                work.check().is_err()
            }),
        )
    };
    let per_query_bound = vertices + segments * 2 * request.sheets.len();
    let coarse_work_bound = per_query_bound
        .checked_mul(first_indices.len())
        .ok_or("automatic work bound overflow")?;
    // Preserve the cheap admission path and its existing output for ordinary
    // inputs. When it rejects, count actual projected roots before any matching.
    let mut total_work_bound = coarse_work_bound;
    let mut prepared_pixels = None;
    let mut projection_failures = 0usize;
    if coarse_work_bound > request.maximum_total_work {
        let Some(Ok(prepared)) = &prepared_queries else {
            return Err("automatic total work budget exceeded before matching".into());
        };
        total_work_bound = 0;
        let mut pixels = Vec::with_capacity(first_indices.len());
        for (query, &index) in first_indices.iter().enumerate() {
            work.update(Stage::Validating, query, first_indices.len())?;
            let pixel = map_pixel(0, sides[0].detections.observations[index].pixel)?;
            let prepared = prepared.prepare_pixel(pixel, &mut || work.check().is_err());
            work.check()?;
            match &prepared {
                Ok(pixel) => {
                    total_work_bound = total_work_bound
                        .checked_add(pixel.work_bound())
                        .ok_or("automatic work bound overflow")?;
                }
                // These errors precede any counted matching work and are retained.
                Err(_) => projection_failures += 1,
            }
            if total_work_bound > request.maximum_total_work {
                return Err("automatic total work budget exceeded before matching".into());
            }
            pixels.push(prepared);
        }
        prepared_pixels = Some(pixels.into_iter());
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
    let report = json!({"schema_version":1,"kind":"recorded_sheet_supported_matching","sources":sources,
        "first":observation_report(&sides[0]),"second":observation_report(&sides[1]),
        "first_query_observation_indices":first_indices,"second_matcher_trace_ids":trace_ids,
        "second_matcher_trace_pixels":traces,"reflect_x":request.reflect_x,
        "observation_coordinates":"raw image pixel centers, x right and y down",
        "matching_coordinates":"supplied camera pixels after explicit reflections",
        "interpolation_space":"undistorted normalized coordinates",
        "signal_units":if request.detection.opening_radius.is_some(){"raw minus row morphological opening, 8-bit contrast"}else{"original raw 8-bit intensity"},
        "saturation_units":"original raw 8-bit intensity","raw_sensor_pts_equal":true,
        "clock_domains_verified":false,"synchronization_verified":false,
        "geometry_binding_verified":false,"geometry_translation_units_verified":false,
        "physical_accuracy_qualified":false,"projector_line_identity_verified":false,"calibration_activated":false});
    work.check()?;
    let partial = staging(target)?;
    let manifest=(||->Result<Value,Error>{
        let mut artifacts=Vec::new();
        for (name,bytes) in [("geometry.json",&geometry_bytes),("request.json",&request_bytes)] {
            artifacts.push(artifact(&partial,name,work,|out|{out.write_all(bytes)?;Ok(())})?);
        }
        for (name,frame) in ["first-frame.raptor","second-frame.raptor"].into_iter().zip(&frames) {
            artifacts.push(artifact(&partial,name,work,|out|{let mut writer=Writer::with_context(out,frame.profile.clone(),frame.context.clone().unwrap_or_default())?;writer.push(&frame.frame)?;writer.finish()?;Ok(())})?);
        }
        artifacts.push(artifact(&partial,"observations.json",work,|out|{serde_json::to_writer_pretty(out,&report)?;Ok(())})?);
        let mut points=Vec::new();let mut failures=0usize;let mut no_unique=0usize;
        artifacts.push(artifact(&partial,"matches.ndjson",work,|out|{
            let mut out=LimitedWriter {inner:out,remaining:request.maximum_report_bytes};
            for (query,&index) in first_indices.iter().enumerate() {
                work.update(Stage::Matching,query,first_indices.len())?;let pixel=map_pixel(0,sides[0].detections.observations[index].pixel)?;
                let result=match &mut prepared_pixels {
                    Some(pixels)=>match pixels.next().ok_or("inconsistent prepared query count")? {
                        Ok(pixel)=>pixel.execute(&mut ||work.check().is_err()),
                        Err(error)=>Err(error),
                    },
                    None=>match &prepared_queries {
                        None=>Err("no supported second-camera traces".into()),
                        Some(Err(error))=>Err(error.clone()),
                        Some(Ok(prepared))=>prepared.match_pixel(pixel,&mut ||work.check().is_err()),
                    },
                };
                work.check()?;
                match &result {
                    Ok(matches)=>if let Some(selected)=matches.unique {
                        let crossing=&matches.traces[selected.trace_index].crossings[selected.crossing_index];
                        let point=crossing.triangulation.as_ref().map_err(|e|format!("inconsistent selected triangulation: {e}"))?;
                        let support=&crossing.support[selected.support_index];
                        let projection=&matches.projections[support.projection_index];
                        points.push(json!({"first_observation":index,"first_trace":sides[0].membership[index].0,
                            "selection":selected,"second_trace":trace_ids[selected.trace_index],
                            "second_calibration_pixel":crossing.intersection.right_pixel,
                            "second_raw_image_pixel":map_pixel(1,crossing.intersection.right_pixel)?,
                            "sheet_index":projection.sheet_index,"sheet_root_index":projection.root_index,
                            "position_in_supplied_geometry_units":point.position_mm}));
                    }else{no_unique+=1;},
                    Err(_)=>{failures+=1;}
                }
                serde_json::to_writer(&mut out,&json!({"first_observation":index,"first_calibration_pixel":pixel,"result":result}))?;
                out.write_all(b"\n")?;
            }
            work.update(Stage::Matching,first_indices.len(),first_indices.len())?;
            Ok(())
        })?);
        let points_report=json!({"schema_version":1,"coordinate_frame":"supplied first-camera frame",
            "units_verified":false,"physical_accuracy_qualified":false,"projector_line_identity_verified":false,
            "cross_observation_uniqueness_verified":false,"points":points});
        artifacts.push(artifact(&partial,"candidate-points.json",work,|out|{serde_json::to_writer_pretty(out,&points_report)?;Ok(())})?);
        let mut manifest=json!({"schema_version":1,"kind":"recorded_sheet_supported_matching",
            "queries":first_indices.len(),"candidate_points":points.len(),"query_failures":failures,
            "queries_without_unique_match":no_unique,"conservative_total_work_bound":total_work_bound,
            "artifacts":artifacts,"physical_accuracy_qualified":false,"units_verified":false,
            "geometry_binding_verified":false,"projector_line_identity_verified":false,"calibration_activated":false});
        if prepared_pixels.is_some() {
            manifest["coarse_total_work_bound"]=json!(coarse_work_bound);
            manifest["work_bound_method"]=json!("per_pixel_projected_roots");
            manifest["projected_root_preparation_failures"]=json!(projection_failures);
        }
        artifact(&partial,"manifest.json",work,|out|{serde_json::to_writer_pretty(out,&manifest)?;Ok(())})?;
        work.update(Stage::Finishing,0,0)?;File::open(&partial)?.sync_all()?;first_source.check_source()?;second_source.check_source()?;work.check()?;publish(&partial,target)?;Ok(manifest)
    })().map_err(|e|format!("{e}; incomplete automatic trace bundle retained at {}",partial.display()))?;
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
    use super::super::tests::{Temp, fixture as seeded_fixture};
    use super::*;
    use std::fs;

    fn fixture(temp: &Temp) -> (Inputs, Value) {
        let (inputs, mut request) = seeded_fixture(temp);
        let r = request.as_object_mut().unwrap();
        let intersection = r.remove("intersection").unwrap();
        r.remove("seeds");
        r.remove("seed_radius_px");
        r.insert(
            "sheets".into(),
            json!([{"quadratic_y":0.,"linear_y":0.,"linear_z":0.,"offset":11.25}]),
        );
        r.insert("triangulation".into(),json!({"min_depth_z_mm":100.,"max_depth_z_mm":500.,"min_ray_angle_rad":0.0001,"max_ray_gap_mm":0.01,"max_reprojection_error_px":0.01}));
        r.insert(
            "matching".into(),
            json!({"intersection":intersection,"maximum_sheet_distance_px":1.,"maximum_work":1000}),
        );
        r.insert("maximum_queries".into(), json!(100));
        r.insert("maximum_total_work".into(), json!(10000));
        r.insert("maximum_report_bytes".into(), json!(1000000));
        fs::write(
            &inputs.request,
            serde_json::to_vec_pretty(&request).unwrap(),
        )
        .unwrap();
        (inputs, request)
    }
    fn read(path: impl AsRef<Path>) -> Value {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }
    fn lines(path: impl AsRef<Path>) -> Vec<Value> {
        fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }
    #[test]
    fn whole_recordings_produce_known_geometry_and_complete_lineage_without_seeds() {
        let temp = Temp::new();
        let (inputs, _) = fixture(&temp);
        let target = temp.0.join("out");
        let source = fs::read(&inputs.first_recording).unwrap();
        let manifest = run(&inputs, &target, &WorkControl::default()).unwrap();
        assert_eq!(manifest["queries"], 32);
        assert_eq!(manifest["candidate_points"], 31);
        assert_eq!(manifest["query_failures"], 0);
        assert_eq!(manifest["queries_without_unique_match"], 1);
        assert_eq!(manifest["conservative_total_work_bound"], 3008);
        assert_eq!(source, fs::read(&inputs.first_recording).unwrap());
        for (name, path) in [
            ("geometry.json", &inputs.geometry),
            ("request.json", &inputs.request),
        ] {
            assert_eq!(
                fs::read(path).unwrap(),
                fs::read(target.join(name)).unwrap()
            );
        }
        for artifact in manifest["artifacts"].as_array().unwrap() {
            let bytes = fs::read(target.join(artifact["role"].as_str().unwrap())).unwrap();
            assert_eq!(bytes.len() as u64, artifact["bytes"].as_u64().unwrap());
            assert_eq!(
                u64::from(crc32fast::hash(&bytes)),
                artifact["crc32"].as_u64().unwrap()
            );
        }
        let observations = read(target.join("observations.json"));
        assert_eq!(observations["second_matcher_trace_ids"], json!([0]));
        assert_eq!(
            observations["second"]["traces"][0]["observation_indices"],
            json!((0..32).collect::<Vec<_>>())
        );
        let matches = lines(target.join("matches.ndjson"));
        assert_eq!(matches.len(), 32);
        let hit = &matches[10]["result"]["Ok"]["traces"][0]["crossings"][0];
        assert_eq!(hit["intersection"]["segment_index"], 9);
        assert!((hit["intersection"]["fraction"].as_f64().unwrap() - 0.5).abs() < 1e-9);
        let points = read(target.join("candidate-points.json"));
        assert_eq!(points["units_verified"], false);
        for point in points["points"].as_array().unwrap() {
            let p = point["position_in_supplied_geometry_units"]
                .as_array()
                .unwrap();
            assert!((p[0].as_f64().unwrap() - 11.25).abs() < 1e-8);
            assert!((p[2].as_f64().unwrap() - 250.).abs() < 1e-8);
        }
        assert!(run(&inputs, &target, &WorkControl::default()).is_err());
    }
    #[test]
    fn duplicate_sheet_hypotheses_retain_ambiguity_without_output_points() {
        let temp = Temp::new();
        let (inputs, mut request) = fixture(&temp);
        let sheet = request["sheets"][0].clone();
        request["sheets"].as_array_mut().unwrap().push(sheet);
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let target = temp.0.join("ambiguous");
        let report = run(&inputs, &target, &WorkControl::default()).unwrap();
        assert_eq!(report["candidate_points"], 0);
        assert_eq!(report["queries_without_unique_match"], 32);
        let result = lines(target.join("matches.ndjson"));
        assert_eq!(result[10]["result"]["Ok"]["supported_combinations"], 2);
        assert!(result[10]["result"]["Ok"]["unique"].is_null());
    }
    #[test]
    fn bounds_and_recording_failures_never_publish() {
        let temp = Temp::new();
        let (mut inputs, request) = fixture(&temp);
        let target = temp.0.join("out");
        let mut cases = Vec::new();
        for (key, value) in [
            ("maximum_queries", json!(31)),
            ("maximum_total_work", json!(2015)),
            ("maximum_report_bytes", json!(134217729)),
            ("sheets", json!([])),
        ] {
            let mut r = request.clone();
            r[key] = value;
            cases.push(r);
        }
        let mut bad = request.clone();
        bad["expected_frames"][0]["payload_crc32"] = json!(0);
        cases.push(bad);
        let mut bad = request.clone();
        bad["matching"]["intersection"]["normalized_tolerance"] = json!(0.);
        bad["continuity"]["minimum_observations"] = json!(33);
        cases.push(bad);
        for r in cases {
            fs::write(&inputs.request, serde_json::to_vec(&r).unwrap()).unwrap();
            assert!(run(&inputs, &target, &WorkControl::default()).is_err());
            assert!(!target.exists());
        }
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        inputs.first_index = 99;
        assert!(run(&inputs, &target, &WorkControl::default()).is_err());
        inputs.first_index = 0;
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
    fn streaming_report_budget_retains_partial_without_publishing() {
        let temp = Temp::new();
        let (inputs, mut request) = fixture(&temp);
        request["maximum_report_bytes"] = json!(1);
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let target = temp.0.join("out");
        let error = run(&inputs, &target, &WorkControl::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("byte budget"));
        assert!(error.contains("incomplete automatic trace bundle retained"));
        assert!(!target.exists());
        let partial = fs::read_dir(&temp.0)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("out.partial-")
            })
            .unwrap();
        assert!(fs::metadata(partial.join("matches.ndjson")).unwrap().len() <= 1);
        assert!(!partial.join("manifest.json").exists());
    }
    #[test]
    fn tighter_projected_bound_admits_exact_limit_without_changing_matches_or_caps() {
        let temp = Temp::new();
        let (inputs, mut request) = fixture(&temp);
        let baseline = temp.0.join("baseline");
        run(&inputs, &baseline, &WorkControl::default()).unwrap();
        request["maximum_total_work"] = json!(2016);
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let target = temp.0.join("tight");
        let report = run(&inputs, &target, &WorkControl::default()).unwrap();
        assert_eq!(report["conservative_total_work_bound"], 2016);
        assert_eq!(report["coarse_total_work_bound"], 3008);
        assert_eq!(report["work_bound_method"], "per_pixel_projected_roots");
        assert_eq!(report["projected_root_preparation_failures"], 0);
        assert_eq!(report["candidate_points"], 31);
        for name in ["matches.ndjson", "candidate-points.json"] {
            assert_eq!(
                fs::read(target.join(name)).unwrap(),
                fs::read(baseline.join(name)).unwrap()
            );
        }
        request["maximum_total_work"] = json!(2015);
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let target = temp.0.join("too-low");
        assert!(run(&inputs, &target, &WorkControl::default()).is_err());
        assert!(!target.exists());
        assert!(!fs::read_dir(&temp.0).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("too-low.partial-")
        }));
    }
    #[test]
    fn projected_admission_retains_early_geometry_errors_without_counted_matching() {
        let temp = Temp::new();
        let (inputs, mut request) = fixture(&temp);
        let mut geometry = read(&inputs.geometry);
        geometry["left"]["valid_radius"] = json!(0.0001);
        fs::write(&inputs.geometry, serde_json::to_vec(&geometry).unwrap()).unwrap();
        let baseline = temp.0.join("baseline");
        let high = run(&inputs, &baseline, &WorkControl::default()).unwrap();
        assert_eq!(high["query_failures"], 32);
        request["maximum_total_work"] = json!(1);
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let target = temp.0.join("tight");
        let report = run(&inputs, &target, &WorkControl::default()).unwrap();
        assert_eq!(report["conservative_total_work_bound"], 0);
        assert_eq!(report["projected_root_preparation_failures"], 32);
        assert_eq!(report["query_failures"], 32);
        assert_eq!(report["candidate_points"], 0);
        assert_eq!(
            fs::read(target.join("matches.ndjson")).unwrap(),
            fs::read(baseline.join("matches.ndjson")).unwrap()
        );
    }
    #[test]
    fn reflection_preserves_raw_pixels_and_empty_continuity_is_explicit() {
        let temp = Temp::new();
        let (inputs, mut request) = fixture(&temp);
        request["reflect_x"] = json!([true, true]);
        request["sheets"][0]["offset"] = json!(-11.25);
        let mut geometry = read(&inputs.geometry);
        geometry["right_from_left"]["translation_mm"] = json!([10., 0., 0.]);
        fs::write(&inputs.geometry, serde_json::to_vec(&geometry).unwrap()).unwrap();
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let target = temp.0.join("reflected");
        let report = run(&inputs, &target, &WorkControl::default()).unwrap();
        assert_eq!(report["candidate_points"], 31);
        let points = read(target.join("candidate-points.json"));
        assert_eq!(points["points"][0]["second_raw_image_pixel"][0], 16.);
        assert_eq!(points["points"][0]["second_calibration_pixel"][0], 15.);
        assert_eq!(
            read(target.join("observations.json"))["first"]["observations"][0]["pixel"],
            json!([20., 0.])
        );
        request["continuity"]["minimum_observations"] = json!(33);
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let target = temp.0.join("unsupported");
        let report = run(&inputs, &target, &WorkControl::default()).unwrap();
        assert_eq!(report["queries"], 0);
        assert_eq!(report["candidate_points"], 0);
        assert_eq!(
            read(target.join("observations.json"))["first"]["observations"]
                .as_array()
                .unwrap()
                .len(),
            32
        );
        assert!(lines(target.join("matches.ndjson")).is_empty());
    }
}
