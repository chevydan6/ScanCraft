use scancraft_geometry::{Cloud, Point};
use serde_json::{Value, json};
use std::{
    fs,
    io::Cursor,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "scancraft-sequence-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn setup(&self) -> Value {
        let world = [
            [0.25, 0.25, 0.25],
            [10.25, 0.25, 0.25],
            [0.25, 20.25, 0.25],
            [0.25, 0.25, 30.25],
            [4.25, 5.25, 8.25],
            [-6.75, 2.25, 4.25],
        ];
        let mut frames = Vec::new();
        for (id, shift) in [0., -0.2, 1000., -0.4].into_iter().enumerate() {
            let cloud = Cloud {
                points: world
                    .iter()
                    .map(|p| Point {
                        position: [p[0] + shift, p[1], p[2]],
                        color: Some([10, 20, 30]),
                        normal: None,
                    })
                    .collect(),
            };
            let mut bytes = Vec::new();
            scancraft_geometry::write_ply(&cloud, &mut bytes).unwrap();
            let path = format!("frame-{id}.ply");
            fs::write(self.0.join(&path), bytes).unwrap();
            frames.push(json!({"id":id,"path":path,"tracking_indices":[0,1,2,3,4,5]}));
        }
        let settings = json!({"schema_version":1,"session_id":"synthetic-sequence","source":"synthetic_fixture",
            "voxel_mm":1.,"maximum_voxels":100,
            "tracking":{"max_correspondence_mm":2.,"max_rms_mm":0.01,"minimum_overlap":0.8,
            "max_step_translation_mm":1.,"max_step_rotation_rad":0.1,"iterations":20,"tolerance_mm":1e-8},"frames":frames});
        self.settings(&settings);
        settings
    }
    fn settings(&self, settings: &Value) {
        fs::write(
            self.0.join("sequence.json"),
            serde_json::to_vec_pretty(settings).unwrap(),
        )
        .unwrap();
    }
    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("fuse-sequence")
            .arg(self.0.join("sequence.json"))
            .arg(self.0.join("result"))
            .output()
            .unwrap()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn point_plane_dataset(s: &Scratch, with_normals: bool) -> Value {
    let mut settings = s.setup();
    settings["registration_method"] = json!("point_to_plane");
    let mut world = Vec::new();
    for a in [-4., 0., 4.] {
        for b in [-3., 0., 3.] {
            world.extend([
                ([30.25, a + 0.25, b + 0.25], [1., 0., 0.]),
                ([a + 0.25, 40.25, b + 0.25], [0., 1., 0.]),
                ([a + 0.25, b + 0.25, 50.25], [0., 0., 1.]),
            ]);
        }
    }
    for (id, shift) in [0., -0.2, 1000., -0.4].into_iter().enumerate() {
        let cloud = Cloud {
            points: world
                .iter()
                .map(|(p, n)| Point {
                    position: [p[0] + shift, p[1], p[2]],
                    color: Some([10, 20, 30]),
                    normal: with_normals.then_some(*n),
                })
                .collect(),
        };
        let mut bytes = Vec::new();
        scancraft_geometry::write_ply(&cloud, &mut bytes).unwrap();
        fs::write(s.0.join(format!("frame-{id}.ply")), bytes).unwrap();
        settings["frames"][id]["tracking_indices"] = json!((0..world.len()).collect::<Vec<_>>());
    }
    s.settings(&settings);
    settings
}

#[test]
fn point_plane_sequence_fuses_with_explicit_method_and_recovers_after_loss() {
    let s = Scratch::new();
    point_plane_dataset(&s, true);
    let original = fs::read(s.0.join("frame-0.ply")).unwrap();
    let output = s.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(s.0.join("result/report.json")).unwrap()).unwrap();
    assert_eq!(report["registration_method"], "point_to_plane");
    assert_eq!(report["integrated_frames"], 3);
    assert_eq!(report["rejected_frames"], 1);
    assert_eq!(report["frames"][2]["tracking_state"], "Lost");
    assert_eq!(report["frames"][2]["world_from_frame"], Value::Null);
    assert_eq!(report["frames"][3]["accepted"], true);
    assert!(
        (report["frames"][3]["world_from_frame"]["translation_mm"][0]
            .as_f64()
            .unwrap()
            - 0.4)
            .abs()
            < 1e-8
    );
    assert!(report["frames"][3]["rms_mm"].as_f64().unwrap() < 1e-8);
    assert_eq!(report["physical_accuracy_qualified"], false);
    assert_eq!(fs::read(s.0.join("frame-0.ply")).unwrap(), original);
    let cloud = scancraft_geometry::formats::read_ply(
        Cursor::new(fs::read(s.0.join("result/points.ply")).unwrap()),
        None,
    )
    .unwrap();
    assert_eq!(cloud.points.len(), 27);
    assert!(cloud.points.iter().all(|p| p.normal.is_some()));
}

#[test]
fn point_plane_missing_normals_never_publishes_sequence() {
    let s = Scratch::new();
    point_plane_dataset(&s, false);
    let output = s.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing a normal"));
    assert!(!s.0.join("result").exists());
}
#[test]
fn cli_tracks_fuses_reports_loss_and_exports_verified_editable_cloud() {
    let s = Scratch::new();
    s.setup();
    let output = s.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["integrated_frames"], 3);
    assert_eq!(summary["rejected_frames"], 1);
    assert_eq!(summary["points"], 6);
    for a in summary["artifacts"].as_array().unwrap() {
        let bytes = fs::read(s.0.join("result").join(a["role"].as_str().unwrap())).unwrap();
        assert_eq!(a["bytes"], bytes.len() as u64);
        assert_eq!(a["crc32"], u64::from(crc32fast::hash(&bytes)));
    }
    let report: Value =
        serde_json::from_slice(&fs::read(s.0.join("result/report.json")).unwrap()).unwrap();
    assert_eq!(report["physical_accuracy_qualified"], false);
    assert_eq!(report["source"], "synthetic_fixture");
    assert_eq!(report["registration_method"], "point_to_point");
    assert_eq!(report["frames"][2]["tracking_state"], "Lost");
    assert!(report["frames"][2]["world_from_frame"].is_null());
    assert_eq!(report["frames"][2]["integrated"], false);
    assert!(
        (report["frames"][3]["world_from_frame"]["translation_mm"][0]
            .as_f64()
            .unwrap()
            - 0.4)
            .abs()
            < 1e-10
    );
    let editor = scancraft_geometry::formats::load_project(
        fs::File::open(s.0.join("result/project.orproj")).unwrap(),
    )
    .unwrap();
    let cloud = scancraft_geometry::formats::read_ply(
        Cursor::new(fs::read(s.0.join("result/points.ply")).unwrap()),
        None,
    )
    .unwrap();
    assert_eq!(cloud, editor.cloud);
    assert!(cloud.points.iter().all(|p| p.color == Some([10, 20, 30])));
    let manifest = fs::read(s.0.join("result/manifest.json")).unwrap();
    assert!(!s.run().status.success());
    assert_eq!(
        fs::read(s.0.join("result/manifest.json")).unwrap(),
        manifest
    );
}
#[test]
fn failed_frames_configuration_and_cancellation_never_publish_sequence_bundle() {
    for kind in 0..4 {
        let s = Scratch::new();
        let mut settings = s.setup();
        match kind {
            0 => settings["maximum_voxels"] = json!(1),
            1 => {
                fs::write(s.0.join("frame-3.ply"), b"corrupt").unwrap();
            }
            2 => settings["frames"][1]["id"] = json!(0),
            _ => settings["source"] = json!("physically_qualified"),
        }
        s.settings(&settings);
        assert!(!s.run().status.success());
        assert!(!s.0.join("result").exists());
    }
    let s = Scratch::new();
    s.setup();
    let work = scancraft_geometry::work::WorkControl::default();
    work.cancel();
    assert!(
        scancraft_workflow::sequence::run(&s.0.join("sequence.json"), &s.0.join("result"), &work)
            .is_err()
    );
    assert!(!s.0.join("result").exists());
}

#[test]
fn explicit_huber_plane_method_fuses_recovers_and_records_scale() {
    let s = Scratch::new();
    let mut settings = point_plane_dataset(&s, true);
    settings["registration_method"] = json!({"point_to_plane_huber":{"delta_mm":0.05}});
    s.settings(&settings);
    let output = s.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(s.0.join("result/report.json")).unwrap()).unwrap();
    assert_eq!(
        report["registration_method"],
        settings["registration_method"]
    );
    assert_eq!(report["integrated_frames"], 3);
    assert_eq!(report["rejected_frames"], 1);
    assert_eq!(report["frames"][2]["world_from_frame"], Value::Null);
    assert!(
        (report["frames"][3]["world_from_frame"]["translation_mm"][0]
            .as_f64()
            .unwrap()
            - 0.4)
            .abs()
            < 1e-8
    );
}
#[test]
fn invalid_huber_scale_unknown_parameters_and_missing_normals_never_publish() {
    for method in [
        json!({"point_to_plane_huber":{"delta_mm":0.}}),
        json!({"point_to_plane_huber":{"delta_mm":-0.1}}),
        json!({"point_to_plane_huber":{"delta_mm":0.05,"ignored":true}}),
    ] {
        let s = Scratch::new();
        let mut settings = point_plane_dataset(&s, true);
        settings["registration_method"] = method;
        s.settings(&settings);
        assert!(!s.run().status.success());
        assert!(!s.0.join("result").exists());
    }
    let s = Scratch::new();
    let mut settings = point_plane_dataset(&s, false);
    settings["registration_method"] = json!({"point_to_plane_huber":{"delta_mm":0.05}});
    s.settings(&settings);
    assert!(!s.run().status.success());
    assert!(!s.0.join("result").exists());
}
