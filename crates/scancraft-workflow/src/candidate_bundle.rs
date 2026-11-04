//! Read-only candidate coordinates with artifact integrity and retained-frame checks.
//! This does not recompute matching or qualify numerical/physical correctness.
mod frame_view;
use crate::bounded_json::Items;
use crate::{Error, recording_playback::Playback, scan_project_load::Guard};
pub use frame_view::FrameView;
use scancraft_calibration::stereo::StereoCalibration;
use scancraft_depth::laser_pair::MAX_QUERIES;
use scancraft_geometry::work::WorkControl;
use serde::{Deserialize, de::IgnoredAny};
use serde_json::{Value, json};
use std::{collections::HashSet, path::Path};

#[derive(Deserialize)]
enum ObservationKind {
    #[serde(rename = "recorded_sheet_supported_matching")]
    Matching,
}
#[derive(Deserialize)]
enum CandidateFrame {
    #[serde(rename = "supplied first-camera frame")]
    First,
}
#[derive(Deserialize)]
struct ObservationSide {
    observations: Items<IgnoredAny, 100000>,
}
#[derive(Deserialize)]
struct ObservationSummary {
    schema_version: u32,
    #[serde(rename = "kind")]
    _kind: ObservationKind,
    first: ObservationSide,
    raw_sensor_pts_equal: bool,
    clock_domains_verified: bool,
    synchronization_verified: bool,
    geometry_binding_verified: bool,
    geometry_translation_units_verified: bool,
    physical_accuracy_qualified: bool,
    projector_line_identity_verified: bool,
    calibration_activated: bool,
}
#[derive(Deserialize)]
struct CandidatePoint {
    first_observation: usize,
    position_in_supplied_geometry_units: [f64; 3],
}
#[derive(Deserialize)]
struct CandidateData {
    schema_version: u32,
    #[serde(rename = "coordinate_frame")]
    _frame: CandidateFrame,
    units_verified: bool,
    physical_accuracy_qualified: bool,
    projector_line_identity_verified: bool,
    cross_observation_uniqueness_verified: bool,
    points: Items<CandidatePoint, MAX_QUERIES>,
}

#[derive(Debug)]
pub struct Loaded {
    pub positions: Vec<[f64; 3]>,
    pub first_observations: Vec<usize>,
    pub report: Value,
    pub frames: [FrameView; 2],
}

fn count(value: &Value, key: &str, max: u64) -> Result<usize, Error> {
    value[key]
        .as_u64()
        .filter(|&n| n <= max)
        .map(|n| n as usize)
        .ok_or_else(|| format!("invalid candidate count or index: {key}").into())
}
fn false_flags(value: &Value, keys: &[&str]) -> Result<(), Error> {
    if keys.iter().any(|key| value[key] != false) {
        return Err(
            "candidate diagnostics cannot assert unverified physical/model guarantees".into(),
        );
    }
    Ok(())
}

/// Open a single-pair result or the first pair of a complete saved capture.
/// Both loaders check their own artifact/frame bindings and return bounded
/// diagnostic points; neither recomputes matching or qualifies physical geometry.
/// Input identity is checked around loading; CRC is not authentication. Filesystem
/// operations have no hard time deadline. Use load_at to select another capture pair.
pub fn load(directory: &Path, work: &WorkControl) -> Result<Loaded, Error> {
    load_at(directory, 0, work)
}
/// Open either a single-pair result (index zero) or a completed capture pair.
/// Routing does not confer trust: the selected loader validates its own layout.
pub fn load_at(directory: &Path, pair_index: u64, work: &WorkControl) -> Result<Loaded, Error> {
    work.check()?;
    let mut guard = Guard::default();
    guard.open(directory, true, 0)?;
    let bytes = guard
        .read(&directory.join("manifest.json"), 1_048_576, true, work)?
        .0;
    #[derive(Deserialize)]
    struct Kind {
        kind: String,
    }
    let kind: Kind = serde_json::from_slice(&bytes)?;
    let loaded = match kind.kind.as_str() {
        "recorded_sheet_supported_matching" if pair_index == 0 => {
            load_inner(directory, work, &|| Ok(()))?
        }
        "recorded_sheet_supported_matching" => {
            return Err("single-pair result only has pair zero".into());
        }
        "laser_candidate_capture_artifacts" => {
            crate::laser_stream::bundle::load_pair(directory, pair_index, work)?
        }
        _ => return Err("unsupported candidate bundle schema/kind".into()),
    };
    guard.check()?;
    work.check()?;
    Ok(loaded)
}

/// A validated single-pair snapshot for numerical processing. Full original raw
/// frames are separate from display-only thumbnails. The retained manifest binds
/// all seven source artifacts; it does not authenticate them or qualify geometry.
pub struct RecordedPair {
    pub candidates: Loaded,
    pub frames: [scancraft_recording::selection::SelectedFrame; 2],
    pub geometry: StereoCalibration,
    pub manifest: Value,
}

pub fn load_recorded_pair(directory: &Path, work: &WorkControl) -> Result<RecordedPair, Error> {
    load_recorded_inner(directory, work, &|| Ok(()))
}

fn load_inner(
    directory: &Path,
    work: &WorkControl,
    before_final: &dyn Fn() -> Result<(), Error>,
) -> Result<Loaded, Error> {
    Ok(load_recorded_inner(directory, work, before_final)?.candidates)
}

fn load_recorded_inner(
    directory: &Path,
    work: &WorkControl,
    before_final: &dyn Fn() -> Result<(), Error>,
) -> Result<RecordedPair, Error> {
    work.check()?;
    let mut guard = Guard::default();
    guard.open(directory, true, 0)?;
    let bytes = guard
        .read(&directory.join("manifest.json"), 1_048_576, true, work)?
        .0;
    let manifest: Value = serde_json::from_slice(&bytes)?;
    if manifest["schema_version"] != 1 || manifest["kind"] != "recorded_sheet_supported_matching" {
        return Err("unsupported candidate bundle schema/kind".into());
    }
    false_flags(
        &manifest,
        &[
            "physical_accuracy_qualified",
            "units_verified",
            "geometry_binding_verified",
            "projector_line_identity_verified",
            "calibration_activated",
        ],
    )?;
    let queries = count(&manifest, "queries", MAX_QUERIES as u64)?;
    let candidates = count(&manifest, "candidate_points", queries as u64)?;
    let failures = count(&manifest, "query_failures", queries as u64)?;
    let unmatched = count(&manifest, "queries_without_unique_match", queries as u64)?;
    if candidates + failures + unmatched != queries {
        return Err("candidate query counters disagree".into());
    }
    let domain_rejections = manifest
        .get("domain_rejections")
        .map(|value| {
            let summary: scancraft_depth::laser_pair::DomainSummary =
                serde_json::from_value(value.clone())?;
            if manifest["uniqueness_scope"] != "retained_valid_domain_segments"
                || summary.rejected_trace_vertices > 65536
                || summary.rejected_trace_segments > 65536
                || summary.rejected_sheet_projections > queries * 256
            {
                return Err(Error::from("invalid domain exclusion summary"));
            }
            Ok(summary)
        })
        .transpose()?;
    let artifacts = manifest["artifacts"]
        .as_array()
        .filter(|a| a.len() == 7)
        .ok_or("candidate bundle requires exactly seven artifacts")?;
    let mut seen = HashSet::new();
    let mut geometry = None;
    let mut request = None;
    let mut observations = None;
    let mut points = None;
    for a in artifacts {
        work.check()?;
        let name = a["role"]
            .as_str()
            .ok_or("missing candidate artifact role")?;
        let (limit, retain) = match name {
            "geometry.json" | "request.json" => (1_048_576, true),
            "first-frame.raptor" | "second-frame.raptor" => (32 * 1024 * 1024, false),
            "observations.json" => (128 * 1024 * 1024, true),
            "matches.ndjson" => (128 * 1024 * 1024, false),
            "candidate-points.json" => (4 * 1024 * 1024, true),
            _ => return Err("unexpected candidate artifact role/path".into()),
        };
        if !seen.insert(name.to_string()) {
            return Err("duplicate candidate artifact role".into());
        }
        let expected = a["bytes"]
            .as_u64()
            .filter(|&n| n <= limit)
            .ok_or("invalid candidate artifact byte bound")?;
        let crc = a["crc32"]
            .as_u64()
            .filter(|&n| n <= u64::from(u32::MAX))
            .ok_or("invalid candidate artifact CRC")?;
        let (data, bytes, actual_crc) = guard.read(&directory.join(name), limit, retain, work)?;
        if bytes != expected || u64::from(actual_crc) != crc {
            return Err(format!("candidate artifact length/CRC mismatch: {name}").into());
        }
        match name {
            "geometry.json" => geometry = Some(data),
            "request.json" => request = Some(data),
            "observations.json" => observations = Some(data),
            "candidate-points.json" => points = Some(data),
            _ => {}
        }
    }
    let geometry: StereoCalibration = serde_json::from_slice(&geometry.ok_or("missing geometry")?)?;
    geometry.validate()?;
    let request: Value = serde_json::from_slice(&request.ok_or("missing request")?)?;
    let domain_requested = match request.get("retain_valid_domain_segments") {
        None | Some(Value::Bool(false)) => false,
        Some(Value::Bool(true)) => true,
        _ => return Err("invalid domain policy in retained request".into()),
    };
    if domain_requested != domain_rejections.is_some()
        || (!domain_requested && manifest.get("uniqueness_scope").is_some())
    {
        return Err("candidate domain policy and summary disagree".into());
    }
    let observations: ObservationSummary =
        serde_json::from_slice(&observations.ok_or("missing observations")?)?;
    let points: CandidateData = serde_json::from_slice(&points.ok_or("missing points")?)?;
    if request["schema_version"] != 1
        || observations.schema_version != 1
        || points.schema_version != 1
    {
        return Err("candidate artifact schemas disagree".into());
    }
    if [
        observations.clock_domains_verified,
        observations.synchronization_verified,
        observations.geometry_binding_verified,
        observations.geometry_translation_units_verified,
        observations.physical_accuracy_qualified,
        observations.projector_line_identity_verified,
        observations.calibration_activated,
        points.units_verified,
        points.physical_accuracy_qualified,
        points.projector_line_identity_verified,
        points.cross_observation_uniqueness_verified,
    ]
    .into_iter()
    .any(|flag| flag)
    {
        return Err(
            "candidate diagnostics cannot assert unverified physical/model guarantees".into(),
        );
    }
    if !observations.raw_sensor_pts_equal {
        return Err("candidate bundle lacks declared raw-PTS pairing".into());
    }
    let expected = request["expected_frames"]
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or("missing candidate frame bindings")?;
    let maximum_delta = request["maximum_host_delta_us"]
        .as_u64()
        .filter(|&n| n <= 1_000_000)
        .ok_or("invalid candidate host window")?;
    let mut frames = Vec::new();
    for (side, name) in ["first-frame.raptor", "second-frame.raptor"]
        .into_iter()
        .enumerate()
    {
        work.check()?;
        let mut playback = Playback::open(&directory.join(name), work)?;
        if playback.len() != 1 {
            return Err("candidate retained recording must contain exactly one frame".into());
        }
        let frame = playback.select_raw(0, work)?;
        let camera = if side == 0 {
            &geometry.left
        } else {
            &geometry.right
        };
        if frame.profile.fourcc != *b"GREY"
            || [frame.profile.width, frame.profile.height] != [camera.width, camera.height]
            || expected[side]["interface"] != frame.profile.interface
            || expected[side]["payload_crc32"] != crc32fast::hash(&frame.frame.data)
            || expected[side]["metadata_crc32"] != crc32fast::hash(&frame.frame.metadata)
        {
            return Err("retained candidate frame disagrees with geometry/request binding".into());
        }
        playback.check_source()?;
        frames.push(frame);
    }
    if !matches!(
        [frames[0].profile.interface, frames[1].profile.interface],
        [0, 2] | [2, 0]
    ) || scancraft_core::uvc_pts(&frames[0].frame.metadata)?
        != scancraft_core::uvc_pts(&frames[1].frame.metadata)?
        || frames[0]
            .frame
            .host_timestamp_us
            .abs_diff(frames[1].frame.host_timestamp_us)
            > maximum_delta
    {
        return Err("retained candidate frames fail pairing checks".into());
    }
    let first_count = observations.first.observations.0.len();
    let frame_views = [
        frame_view::prepare(&frames[0], work)?,
        frame_view::prepare(&frames[1], work)?,
    ];
    let values = points.points.0;
    if values.len() != candidates {
        return Err("candidate point count mismatch".into());
    }
    let mut positions = Vec::with_capacity(values.len());
    let mut indices = Vec::with_capacity(values.len());
    let mut used = HashSet::new();
    for point in values {
        work.check()?;
        let index = point.first_observation;
        if index >= first_count || !used.insert(index) {
            return Err("invalid/duplicate candidate first-observation reference".into());
        }
        if !point
            .position_in_supplied_geometry_units
            .iter()
            .all(|v| v.is_finite())
        {
            return Err("candidate coordinate is not finite".into());
        }
        positions.push(point.position_in_supplied_geometry_units);
        indices.push(index);
    }
    before_final()?;
    guard.check()?;
    work.check()?;
    let mut report = json!({"queries":queries,"candidate_points":candidates,
        "query_failures":failures,"queries_without_unique_match":unmatched,"artifact_integrity_verified":true,
        "retained_frame_bindings_verified":true,"historical_original_recordings_verified":false,
        "numerical_recomputation_verified":false,"geometry_binding_verified":false,
        "physical_accuracy_qualified":false,"units_verified":false});
    if let Some(summary) = domain_rejections {
        report["domain_rejections"] = serde_json::to_value(summary)?;
        report["uniqueness_scope"] = json!("retained_valid_domain_segments");
    }
    let frames = frames
        .try_into()
        .map_err(|_| "expected two retained frames")?;
    Ok(RecordedPair {
        candidates: Loaded {
            positions,
            first_observations: indices,
            frames: frame_views,
            report,
        },
        frames,
        geometry,
        manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_core::{Frame, StreamProfile};
    use scancraft_recording::Writer;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    struct Temp(std::path::PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let p = std::env::temp_dir().join(format!(
                "scancraft-candidates-{}-{}",
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
    fn write(path: &Path, value: &Value) {
        fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }
    fn read(path: &Path) -> Value {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }
    fn redigest(root: &Path, name: &str) {
        let mut manifest = read(&root.join("manifest.json"));
        let data = fs::read(root.join(name)).unwrap();
        let entry = manifest["artifacts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|a| a["role"] == name)
            .unwrap();
        entry["bytes"] = json!(data.len());
        entry["crc32"] = json!(crc32fast::hash(&data));
        write(&root.join("manifest.json"), &manifest);
    }
    fn fixture(t: &Temp) -> std::path::PathBuf {
        let root = t.0.join("bundle");
        fs::create_dir(&root).unwrap();
        let camera = json!({"width":4,"height":3,"fx":10.,"fy":10.,"cx":1.5,"cy":1.,"skew":0.,"valid_radius":1.,"distortion":{"k1":0.,"k2":0.,"p1":0.,"p2":0.,"k3":0.}});
        write(
            &root.join("geometry.json"),
            &json!({"left":camera,"right":camera,"right_from_left":{"rotation":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"translation_mm":[-1.,0.,0.]}}),
        );
        let mut bindings = Vec::new();
        for (name, interface) in [("first-frame.raptor", 0), ("second-frame.raptor", 2)] {
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&100u32.to_le_bytes());
            let data = vec![30; 12];
            bindings.push(json!({"interface":interface,"payload_crc32":crc32fast::hash(&data),"metadata_crc32":crc32fast::hash(&metadata)}));
            let mut writer = Writer::new(
                Vec::new(),
                StreamProfile {
                    interface,
                    width: 4,
                    height: 3,
                    fourcc: *b"GREY",
                    fps: 30,
                },
            )
            .unwrap();
            writer
                .push(&Frame {
                    sequence: 0,
                    host_timestamp_us: 1000,
                    flags: 0,
                    metadata,
                    data,
                })
                .unwrap();
            fs::write(root.join(name), writer.finish().unwrap()).unwrap();
        }
        write(
            &root.join("request.json"),
            &json!({"schema_version":1,"expected_frames":bindings,"maximum_host_delta_us":1000}),
        );
        write(
            &root.join("observations.json"),
            &json!({"schema_version":1,"kind":"recorded_sheet_supported_matching","first":{"observations":[{"pixel":[1.,1.]}]},"raw_sensor_pts_equal":true,"clock_domains_verified":false,"synchronization_verified":false,"geometry_binding_verified":false,"geometry_translation_units_verified":false,"physical_accuracy_qualified":false,"projector_line_identity_verified":false,"calibration_activated":false}),
        );
        write(
            &root.join("candidate-points.json"),
            &json!({"schema_version":1,"coordinate_frame":"supplied first-camera frame","units_verified":false,"physical_accuracy_qualified":false,"projector_line_identity_verified":false,"cross_observation_uniqueness_verified":false,"points":[{"first_observation":0,"position_in_supplied_geometry_units":[1.,2.,3.]}]}),
        );
        fs::write(
            root.join("matches.ndjson"),
            b"{\"diagnostic_fixture\":true}\n",
        )
        .unwrap();
        let artifacts = [
            "geometry.json",
            "request.json",
            "first-frame.raptor",
            "second-frame.raptor",
            "observations.json",
            "matches.ndjson",
            "candidate-points.json",
        ]
        .map(|name| {
            let data = fs::read(root.join(name)).unwrap();
            json!({"role":name,"bytes":data.len(),"crc32":crc32fast::hash(&data)})
        });
        write(
            &root.join("manifest.json"),
            &json!({"schema_version":1,"kind":"recorded_sheet_supported_matching","queries":1,"candidate_points":1,"query_failures":0,"queries_without_unique_match":0,"artifacts":artifacts,"physical_accuracy_qualified":false,"units_verified":false,"geometry_binding_verified":false,"projector_line_identity_verified":false,"calibration_activated":false}),
        );
        root
    }
    #[test]
    fn loads_raw_coordinates_and_checksums_without_attesting_matching_or_units() {
        let t = Temp::new();
        let root = fixture(&t);
        let before = fs::read(root.join("candidate-points.json")).unwrap();
        let loaded = load(&root, &WorkControl::default()).unwrap();
        assert_eq!(loaded.positions, vec![[1., 2., 3.]]);
        assert_eq!(loaded.first_observations, vec![0]);
        assert_eq!(loaded.report["artifact_integrity_verified"], true);
        assert_eq!(loaded.report["retained_frame_bindings_verified"], true);
        assert_eq!(loaded.report["numerical_recomputation_verified"], false);
        assert_eq!(loaded.report["units_verified"], false);
        assert_eq!(loaded.frames.each_ref().map(|f| f.interface), [0, 2]);
        for frame in &loaded.frames {
            assert_eq!(frame.source_size, [4, 3]);
            assert_eq!(frame.statistics.pixels, 12);
            assert_eq!(frame.statistics.median, 30);
            assert_eq!(frame.statistics.white_clipped, 0);
            assert_eq!(frame.pixels, vec![30; 12]);
        }
        assert_eq!(
            fs::read(root.join("candidate-points.json")).unwrap(),
            before
        );
    }
    #[test]
    fn corruption_duplicate_roles_traversal_and_symlinks_are_errors() {
        let t = Temp::new();
        let root = fixture(&t);
        let original = read(&root.join("manifest.json"));
        for role in ["../request.json", "/etc/passwd", "geometry.json"] {
            let mut m = original.clone();
            m["artifacts"][1]["role"] = json!(role);
            write(&root.join("manifest.json"), &m);
            assert!(load(&root, &WorkControl::default()).is_err());
        }
        write(&root.join("manifest.json"), &original);
        fs::write(root.join("matches.ndjson"), b"changed").unwrap();
        assert!(
            load(&root, &WorkControl::default())
                .unwrap_err()
                .to_string()
                .contains("CRC")
        );
        redigest(&root, "matches.ndjson");
        fs::rename(root.join("request.json"), t.0.join("request.json")).unwrap();
        std::os::unix::fs::symlink(t.0.join("request.json"), root.join("request.json")).unwrap();
        assert!(load(&root, &WorkControl::default()).is_err());
    }
    #[test]
    fn false_qualifications_bad_counts_and_references_are_rejected() {
        let t = Temp::new();
        let root = fixture(&t);
        let original = read(&root.join("manifest.json"));
        for (key, value) in [
            ("physical_accuracy_qualified", json!(true)),
            ("units_verified", json!(true)),
            ("queries", json!(MAX_QUERIES + 1)),
            ("candidate_points", json!(0)),
        ] {
            let mut m = original.clone();
            m[key] = value;
            write(&root.join("manifest.json"), &m);
            assert!(load(&root, &WorkControl::default()).is_err());
        }
        write(&root.join("manifest.json"), &original);
        let mut points = read(&root.join("candidate-points.json"));
        points["points"][0]["first_observation"] = json!(1);
        write(&root.join("candidate-points.json"), &points);
        redigest(&root, "candidate-points.json");
        assert!(load(&root, &WorkControl::default()).is_err());
    }
    #[test]
    fn frame_binding_and_complete_retained_recording_are_required() {
        let t = Temp::new();
        let root = fixture(&t);
        let mut request = read(&root.join("request.json"));
        request["expected_frames"][0]["payload_crc32"] = json!(0);
        write(&root.join("request.json"), &request);
        redigest(&root, "request.json");
        assert!(
            load(&root, &WorkControl::default())
                .unwrap_err()
                .to_string()
                .contains("binding")
        );
        let t = Temp::new();
        let root = fixture(&t);
        let mut data = fs::read(root.join("second-frame.raptor")).unwrap();
        data.pop();
        fs::write(root.join("second-frame.raptor"), data).unwrap();
        redigest(&root, "second-frame.raptor");
        assert!(load(&root, &WorkControl::default()).is_err());
    }
    #[test]
    fn cancellation_and_mutation_before_commit_never_return_a_loaded_view() {
        let t = Temp::new();
        let root = fixture(&t);
        let work = WorkControl::default();
        work.cancel();
        assert!(load(&root, &work).is_err());
        assert!(
            load_inner(&root, &WorkControl::default(), &|| {
                fs::write(root.join("candidate-points.json"), b"changed")?;
                Ok(())
            })
            .unwrap_err()
            .to_string()
            .contains("changed")
        );
    }

    #[test]
    fn observation_and_point_limits_apply_during_typed_deserialization() {
        let oversized_observations = format!(
            "[{}]",
            std::iter::repeat_n("null", 100001)
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(
            serde_json::from_str::<Items<IgnoredAny, 100000>>(&oversized_observations).is_err()
        );
        let point = r#"{"first_observation":0,"position_in_supplied_geometry_units":[1,2,3]}"#;
        let oversized_points = format!(
            "[{}]",
            std::iter::repeat_n(point, MAX_QUERIES + 1)
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(
            serde_json::from_str::<Items<CandidatePoint, MAX_QUERIES>>(&oversized_points).is_err()
        );
    }
}
