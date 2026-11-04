use super::*;
use scancraft_core::{Frame as RawFrame, StreamProfile};
use scancraft_recording::Writer;
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(output::staging(&std::env::temp_dir().join("tracked-candidates-test")).unwrap())
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn write(path: &Path, v: &Value) {
    fs::write(path, serde_json::to_vec(v).unwrap()).unwrap();
}
fn fixture(root: &Path, index: usize, surface: bool, markers: bool, shift: i32) -> PathBuf {
    let path = root.join(format!("pair-{index}"));
    fs::create_dir(&path).unwrap();
    let camera = json!({"width":64,"height":64,"fx":100.,"fy":100.,"cx":32.,"cy":32.,"skew":0.,"valid_radius":2.,
        "distortion":{"k1":0.,"k2":0.,"p1":0.,"p2":0.,"k3":0.}});
    write(
        &path.join("geometry.json"),
        &json!({"left":camera,"right":camera,
        "right_from_left":{"rotation":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"translation_mm":[-1.,0.,0.]}}),
    );
    let mut bindings = Vec::new();
    for (name, interface, disparity) in
        [("first-frame.raptor", 0, 0), ("second-frame.raptor", 2, -5)]
    {
        let mut data = vec![0; 4096];
        if markers {
            for (x, y) in [(20, 20), (45, 25), (25, 45)] {
                for dy in -3i32..=3 {
                    for dx in -3i32..=3 {
                        if dx * dx + dy * dy <= 9 {
                            data[((y + dy) * 64 + x + dx + disparity + shift) as usize] = 200;
                        }
                    }
                }
            }
        }
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&(100 + index as u32).to_le_bytes());
        bindings.push(json!({"interface":interface,"payload_crc32":crc32fast::hash(&data),"metadata_crc32":crc32fast::hash(&metadata)}));
        let mut writer = Writer::new(
            Vec::new(),
            StreamProfile {
                interface,
                width: 64,
                height: 64,
                fourcc: *b"GREY",
                fps: 30,
            },
        )
        .unwrap();
        writer
            .push(&RawFrame {
                sequence: index as u32,
                host_timestamp_us: 1000 + index as u64,
                flags: 0,
                metadata,
                data,
            })
            .unwrap();
        fs::write(path.join(name), writer.finish().unwrap()).unwrap();
    }
    write(
        &path.join("request.json"),
        &json!({"schema_version":1,"expected_frames":bindings,"maximum_host_delta_us":1000}),
    );
    write(
        &path.join("observations.json"),
        &json!({"schema_version":1,"kind":"recorded_sheet_supported_matching",
        "first":{"observations":[{}]},"raw_sensor_pts_equal":true,"clock_domains_verified":false,"synchronization_verified":false,
        "geometry_binding_verified":false,"geometry_translation_units_verified":false,"physical_accuracy_qualified":false,
        "projector_line_identity_verified":false,"calibration_activated":false}),
    );
    let points = if surface {
        json!([{"first_observation":0,"position_in_supplied_geometry_units":[0.31,0.37,20.33]}])
    } else {
        json!([])
    };
    write(
        &path.join("candidate-points.json"),
        &json!({"schema_version":1,"coordinate_frame":"supplied first-camera frame",
        "units_verified":false,"physical_accuracy_qualified":false,"projector_line_identity_verified":false,
        "cross_observation_uniqueness_verified":false,"points":points}),
    );
    fs::write(path.join("matches.ndjson"), b"fixture\n").unwrap();
    let artifacts = [
        "geometry.json",
        "request.json",
        "first-frame.raptor",
        "second-frame.raptor",
        "observations.json",
        "matches.ndjson",
        "candidate-points.json",
    ]
    .map(|role| {
        let b = fs::read(path.join(role)).unwrap();
        json!({"role":role,"bytes":b.len(),"crc32":crc32fast::hash(&b)})
    });
    write(
        &path.join("manifest.json"),
        &json!({"schema_version":1,"kind":"recorded_sheet_supported_matching","queries":1,
        "candidate_points":usize::from(surface),"query_failures":0,"queries_without_unique_match":usize::from(!surface),
        "artifacts":artifacts,"physical_accuracy_qualified":false,"units_verified":false,
        "geometry_binding_verified":false,"projector_line_identity_verified":false,"calibration_activated":false}),
    );
    path
}
fn request(paths: &[PathBuf]) -> Value {
    json!({"schema_version":1,"session_id":"test","markers":{"threshold_exclusive":100,"minimum_area":5,"maximum_area":100,
        "maximum_components":1000,"maximum_shape_rms_px":0.5,"minimum_depth_model_units":1.,"maximum_depth_model_units":100.,
        "minimum_ray_angle_rad":0.0001,"maximum_ray_gap_model_units":0.01,"maximum_reprojection_error_px":0.1},
        "tracking":{"max_correspondence_model_units":0.5,"max_rms_model_units":0.01,"minimum_overlap":0.7,
        "max_step_translation_model_units":0.5,"max_step_rotation_rad":0.1,"iterations":30,"tolerance_model_units":1e-8},
        "voxel_model_units":0.25,"maximum_voxels":1000,"maximum_stereo_attempts":1000,
        "frames":paths.iter().enumerate().map(|(id,bundle)|json!({"id":id,"bundle":bundle})).collect::<Vec<_>>()})
}
fn rows(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
#[test]
fn exact_retained_frames_track_but_only_surface_points_fuse_with_explicit_empty_frames() {
    let t = Temp::new();
    let paths = (0..3)
        .map(|i| fixture(&t.0, i, i != 1, true, 0))
        .collect::<Vec<_>>();
    let settings = t.0.join("request.json");
    write(&settings, &request(&paths));
    let target = t.0.join("out");
    let result = run(&settings, &target, &WorkControl::default()).unwrap();
    assert_eq!(result["integrated_frames"], 2);
    assert_eq!(result["tracking_only_frames"], 1);
    assert_eq!(result["fused_candidates"], 1);
    assert_eq!(result["units_verified"], false);
    let points: Value =
        serde_json::from_slice(&fs::read(target.join("fused-candidates.json")).unwrap()).unwrap();
    let p = points["positions_model_units"][0].as_array().unwrap();
    for (got, expected) in p.iter().zip([0.31, 0.37, 20.33]) {
        assert!((got.as_f64().unwrap() - expected).abs() < 1e-10);
    }
    let steps = rows(&target.join("frames.ndjson"));
    assert_eq!(steps[1]["status"], "tracking_only");
    assert_eq!(steps[1]["last_integrated"], 0);
    assert_eq!(steps[2]["tracking_points"], 3);
    assert!(!target.join("points.ply").exists());
    assert!(!target.join("project.orproj").exists());
    assert!(run(&settings, &target, &WorkControl::default()).is_err());
}
#[test]
fn unavailable_and_rejected_tracking_never_copy_old_pose_or_integrate_surface() {
    let t = Temp::new();
    let paths = vec![
        fixture(&t.0, 0, true, true, 0),
        fixture(&t.0, 1, true, false, 0),
        fixture(&t.0, 2, true, true, 10),
        fixture(&t.0, 3, true, true, 0),
    ];
    let settings = t.0.join("request.json");
    write(&settings, &request(&paths));
    let target = t.0.join("out");
    let report = run(&settings, &target, &WorkControl::default()).unwrap();
    assert_eq!(report["tracking_unavailable_frames"], 1);
    assert_eq!(report["tracking_rejected_frames"], 1);
    assert_eq!(report["integrated_frames"], 2);
    let frames = rows(&target.join("frames.ndjson"));
    assert!(frames[1]["pose"].is_null());
    assert!(frames[2]["pose"].is_null());
    assert_eq!(frames[2]["last_integrated"], 0);
    assert_eq!(frames[3]["status"], "integrated");
}
#[test]
fn cancellation_limits_corruption_and_duplicate_sources_never_publish() {
    let t = Temp::new();
    let pair = fixture(&t.0, 0, true, true, 0);
    let settings = t.0.join("request.json");
    let value = request(std::slice::from_ref(&pair));
    write(&settings, &value);
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(run(&settings, &t.0.join("cancel"), &cancelled).is_err());
    assert!(!t.0.join("cancel").exists());
    let mut limited = value.clone();
    limited["maximum_stereo_attempts"] = json!(1);
    write(&settings, &limited);
    assert!(
        run(&settings, &t.0.join("budget"), &WorkControl::default())
            .unwrap_err()
            .to_string()
            .contains("budget")
    );
    assert!(!t.0.join("budget").exists());
    write(&settings, &request(&[pair.clone(), pair.clone()]));
    assert!(
        run(&settings, &t.0.join("duplicate"), &WorkControl::default())
            .unwrap_err()
            .to_string()
            .contains("timestamps")
    );
    assert!(!t.0.join("duplicate").exists());
    write(&settings, &value);
    fs::write(pair.join("matches.ndjson"), b"corrupt").unwrap();
    assert!(
        run(&settings, &t.0.join("corrupt"), &WorkControl::default())
            .unwrap_err()
            .to_string()
            .contains("CRC")
    );
    assert!(!t.0.join("corrupt").exists());
}

#[test]
fn changing_geometry_and_nonmonotonic_ids_cannot_form_a_mixed_scan() {
    let t = Temp::new();
    let paths = vec![
        fixture(&t.0, 0, true, true, 0),
        fixture(&t.0, 1, true, true, 0),
    ];
    let settings = t.0.join("settings.json");
    let mut req = request(&paths);
    req["frames"][1]["id"] = json!(0);
    write(&settings, &req);
    assert!(run(&settings, &t.0.join("ids"), &WorkControl::default()).is_err());
    assert!(!t.0.join("ids").exists());
    write(&settings, &request(&paths));
    let geometry = paths[1].join("geometry.json");
    let mut g: Value = serde_json::from_slice(&fs::read(&geometry).unwrap()).unwrap();
    g["right_from_left"]["translation_mm"][0] = json!(-2.);
    write(&geometry, &g);
    let mpath = paths[1].join("manifest.json");
    let mut m: Value = serde_json::from_slice(&fs::read(&mpath).unwrap()).unwrap();
    let bytes = fs::read(geometry).unwrap();
    m["artifacts"][0]["bytes"] = json!(bytes.len());
    m["artifacts"][0]["crc32"] = json!(crc32fast::hash(&bytes));
    write(&mpath, &m);
    assert!(
        run(&settings, &t.0.join("mixed"), &WorkControl::default())
            .unwrap_err()
            .to_string()
            .contains("geometry/interface")
    );
    assert!(!t.0.join("mixed").exists());
}
