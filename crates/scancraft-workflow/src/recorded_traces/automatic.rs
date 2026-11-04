//! Recording/CRC/source guards and diagnostic publication around the pure pair engine.
use super::{Continuity, Detection, ExpectedFrame, Inputs, observation_report, validate_frames};
use crate::{
    ArtifactWriter, Digest, Error, artifact, bounded,
    calibration_bundle::ensure_new,
    output::{publish, staging},
    recording_playback::Playback,
    recording_reconstruction::lineage,
};
use scancraft_calibration::{
    laser_surface::QuadraticSheet,
    laser_trace,
    stereo::{StereoCalibration, TriangulationLimits},
};
use scancraft_depth::laser_pair::{self, FrameInput, PairInfo, Query, Settings, Sink};
use scancraft_geometry::work::{Stage, WorkControl};
use scancraft_recording::{Writer, selection::SelectedFrame};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    #[serde(default)]
    retain_valid_domain_segments: bool,
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

struct LimitedWriter<W> {
    inner: W,
    remaining: usize,
}
impl<W: Write> Write for LimitedWriter<W> {
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
struct Diagnostics<'a> {
    inputs: &'a Inputs,
    target: &'a Path,
    geometry_bytes: &'a [u8],
    request_bytes: &'a [u8],
    settings: &'a Settings,
    sources: &'a [Playback; 2],
    frames: &'a [SelectedFrame; 2],
    work: &'a WorkControl,
    maximum_report_bytes: usize,
    partial: Option<PathBuf>,
    artifacts: Vec<Digest>,
    matches: Option<LimitedWriter<ArtifactWriter<'a>>>,
}
impl Diagnostics<'_> {
    fn begin_inner(&mut self, info: PairInfo<'_>) -> Result<(), Error> {
        self.sources[0].check_source()?;
        self.sources[1].check_source()?;
        self.work.check()?;
        let sources = [
            lineage(
                &self.inputs.first_recording,
                self.sources[0].version(),
                &self.frames[0],
            )?,
            lineage(
                &self.inputs.second_recording,
                self.sources[1].version(),
                &self.frames[1],
            )?,
        ];
        let mut report = json!({"schema_version":1,"kind":"recorded_sheet_supported_matching","sources":sources,
            "first":observation_report(&info.sides[0]),"second":observation_report(&info.sides[1]),
            "first_query_observation_indices":info.first_query_observation_indices,"second_matcher_trace_ids":info.second_matcher_trace_ids,
            "second_matcher_trace_pixels":info.second_matcher_trace_pixels,"reflect_x":self.settings.reflect_x,
            "observation_coordinates":"raw image pixel centers, x right and y down",
            "matching_coordinates":"supplied camera pixels after explicit reflections",
            "interpolation_space":"undistorted normalized coordinates",
            "signal_units":if self.settings.detection.opening_radius.is_some(){"raw minus row morphological opening, 8-bit contrast"}else{"original raw 8-bit intensity"},
            "saturation_units":"original raw 8-bit intensity","raw_sensor_pts_equal":true,"clock_domains_verified":false,"synchronization_verified":false,
            "geometry_binding_verified":false,"geometry_translation_units_verified":false,"physical_accuracy_qualified":false,"projector_line_identity_verified":false,"calibration_activated":false});
        if let Some(rejections) = info.domain_rejections {
            report["domain_rejections"] = serde_json::to_value(rejections)?;
            report["uniqueness_scope"] = json!("retained_valid_domain_segments");
        }
        self.work.check()?;
        let partial = staging(self.target)?;
        self.partial = Some(partial.clone());
        for (name, bytes) in [
            ("geometry.json", self.geometry_bytes),
            ("request.json", self.request_bytes),
        ] {
            self.artifacts
                .push(artifact(&partial, name, self.work, |out| {
                    out.write_all(bytes)?;
                    Ok(())
                })?);
        }
        for (name, frame) in ["first-frame.raptor", "second-frame.raptor"]
            .into_iter()
            .zip(self.frames)
        {
            self.artifacts
                .push(artifact(&partial, name, self.work, |out| {
                    let mut writer = Writer::with_context(
                        out,
                        frame.profile.clone(),
                        frame.context.clone().unwrap_or_default(),
                    )?;
                    writer.push(&frame.frame)?;
                    writer.finish()?;
                    Ok(())
                })?);
        }
        self.artifacts
            .push(artifact(&partial, "observations.json", self.work, |out| {
                serde_json::to_writer_pretty(out, &report)?;
                Ok(())
            })?);
        self.matches = Some(LimitedWriter {
            inner: ArtifactWriter::open(&partial, "matches.ndjson", self.work)?,
            remaining: self.maximum_report_bytes,
        });
        Ok(())
    }
    fn finish(&mut self, report: laser_pair::Report) -> Result<Value, Error> {
        let partial = self
            .partial
            .as_ref()
            .ok_or("missing diagnostic staging directory")?;
        self.artifacts.push(
            self.matches
                .take()
                .ok_or("missing diagnostic match writer")?
                .inner
                .finish()?,
        );
        let points_report = json!({"schema_version":1,"coordinate_frame":"supplied first-camera frame","units_verified":false,"physical_accuracy_qualified":false,"projector_line_identity_verified":false,"cross_observation_uniqueness_verified":false,"points":report.candidates});
        self.artifacts.push(artifact(
            partial,
            "candidate-points.json",
            self.work,
            |out| {
                serde_json::to_writer_pretty(out, &points_report)?;
                Ok(())
            },
        )?);
        let mut manifest = json!({"schema_version":1,"kind":"recorded_sheet_supported_matching","queries":report.queries,"candidate_points":report.candidates.len(),"query_failures":report.query_failures,"queries_without_unique_match":report.queries_without_unique_match,"conservative_total_work_bound":report.admission.conservative_total_work_bound,"artifacts":self.artifacts,"physical_accuracy_qualified":false,"units_verified":false,"geometry_binding_verified":false,"projector_line_identity_verified":false,"calibration_activated":false});
        if let Some(summary) = report.domain_rejections {
            manifest["domain_rejections"] = serde_json::to_value(summary)?;
            manifest["uniqueness_scope"] = json!("retained_valid_domain_segments");
        }
        if report.admission.uses_projected_roots {
            manifest["coarse_total_work_bound"] = json!(report.admission.coarse_total_work_bound);
            manifest["work_bound_method"] = json!("per_pixel_projected_roots");
            manifest["projected_root_preparation_failures"] =
                json!(report.admission.projected_root_preparation_failures);
        }
        artifact(partial, "manifest.json", self.work, |out| {
            serde_json::to_writer_pretty(out, &manifest)?;
            Ok(())
        })?;
        self.work.update(Stage::Finishing, 0, 0)?;
        File::open(partial)?.sync_all()?;
        self.sources[0].check_source()?;
        self.sources[1].check_source()?;
        self.work.check()?;
        publish(partial, self.target)?;
        Ok(manifest)
    }
}
impl Sink for Diagnostics<'_> {
    fn begin(&mut self, info: PairInfo<'_>) -> Result<(), String> {
        self.begin_inner(info).map_err(|e| e.to_string())
    }
    fn query(&mut self, query: &Query) -> Result<(), String> {
        let out = self
            .matches
            .as_mut()
            .ok_or("missing diagnostic match writer")?;
        let mut record = json!({"first_observation":query.first_observation,"first_calibration_pixel":query.first_calibration_pixel,"result":query.result});
        if let Some(rejections) = &query.rejected_projections {
            record["rejected_projections"] =
                serde_json::to_value(rejections).map_err(|e| e.to_string())?;
        }
        serde_json::to_writer(&mut *out, &record).map_err(|e| e.to_string())?;
        out.write_all(b"\n").map_err(|e| e.to_string())
    }
}

/// Preserve whole-recording/source/CRC guards and streaming diagnostic publication.
/// Pure frame computation is shared with future in-memory acquisition consumers;
/// source history, filesystem checks and byte caps remain this adapter's duties.
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
    let settings = Settings {
        interfaces: request.expected_frames.each_ref().map(|f| f.interface),
        reflect_x: request.reflect_x,
        maximum_host_delta_us: request.maximum_host_delta_us,
        detection: request.detection,
        continuity: request.continuity,
        sheets: request.sheets,
        triangulation: request.triangulation,
        matching: request.matching,
        maximum_queries: request.maximum_queries,
        maximum_total_work: request.maximum_total_work,
    };
    let processor = laser_pair::Processor::new(&geometry, &settings)?
        .with_valid_domain_segments(request.retain_valid_domain_segments);
    let mut sources = [
        Playback::open(&inputs.first_recording, work)?,
        Playback::open(&inputs.second_recording, work)?,
    ];
    let frames = [
        sources[0].select_raw(inputs.first_index, work)?,
        sources[1].select_raw(inputs.second_index, work)?,
    ];
    validate_frames(
        &frames,
        &geometry,
        &request.expected_frames,
        request.maximum_host_delta_us,
    )?;
    let mut diagnostics = Diagnostics {
        inputs,
        target,
        geometry_bytes: &geometry_bytes,
        request_bytes: &request_bytes,
        settings: &settings,
        sources: &sources,
        frames: &frames,
        work,
        maximum_report_bytes: request.maximum_report_bytes,
        partial: None,
        artifacts: Vec::new(),
        matches: None,
    };
    let manifest = (|| -> Result<Value, Error> {
        let report = processor.process(
            [
                FrameInput {
                    profile: &frames[0].profile,
                    frame: &frames[0].frame,
                },
                FrameInput {
                    profile: &frames[1].profile,
                    frame: &frames[1].frame,
                },
            ],
            work,
            &mut diagnostics,
        )?;
        diagnostics.finish(report)
    })()
    .map_err(|e| match &diagnostics.partial {
        Some(partial) => format!(
            "{e}; incomplete automatic trace bundle retained at {}",
            partial.display()
        ),
        None => e.to_string(),
    })?;
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
    fn explicit_domain_policy_preserves_valid_results_and_persists_diagnostics() {
        let temp = Temp::new();
        let (inputs, mut request) = fixture(&temp);
        let strict = temp.0.join("strict");
        let baseline = run(&inputs, &strict, &WorkControl::default()).unwrap();
        assert!(baseline.get("domain_rejections").is_none());
        request["retain_valid_domain_segments"] = json!(true);
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let target = temp.0.join("domain");
        let manifest = run(&inputs, &target, &WorkControl::default()).unwrap();
        assert_eq!(
            manifest["domain_rejections"],
            json!({"rejected_trace_vertices":0,"rejected_trace_segments":0,"rejected_sheet_projections":0})
        );
        assert_eq!(manifest["candidate_points"], baseline["candidate_points"]);
        assert_eq!(
            fs::read(strict.join("candidate-points.json")).unwrap(),
            fs::read(target.join("candidate-points.json")).unwrap()
        );
        let observed = read(target.join("observations.json"));
        assert_eq!(
            observed["domain_rejections"]["valid_runs"],
            json!([{"trace_index":0,"first_vertex":0,"vertex_count":32}])
        );
        for (a, mut b) in lines(strict.join("matches.ndjson"))
            .into_iter()
            .zip(lines(target.join("matches.ndjson")))
        {
            assert_eq!(
                b.as_object_mut().unwrap().remove("rejected_projections"),
                Some(json!([]))
            );
            assert_eq!(a, b);
        }
        let loaded = crate::candidate_bundle::load(&target, &WorkControl::default()).unwrap();
        assert_eq!(
            loaded.report["domain_rejections"],
            manifest["domain_rejections"]
        );
        assert_eq!(loaded.report["numerical_recomputation_verified"], false);
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
