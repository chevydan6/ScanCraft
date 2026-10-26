#[path = "../../scancraft-depth/tests/support/subpixel_scene.rs"]
mod scene;
use scancraft_calibration::storage::{self, CalibrationRecord, Provenance};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "scancraft-subpixel-cli-{}-{}",
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
#[test]
fn cli_settings_refine_geometry_and_preserve_original_inputs_and_saved_project() {
    let tmp = Temp::new();
    let (rig, left, right) = scene::fixture(4.25);
    let record = CalibrationRecord {
        schema_version: 1,
        record_id: "synthetic-cli-subpixel".into(),
        units: "millimeters".into(),
        camera_axes: "x_right_y_down_z_forward".into(),
        provenance: Provenance::SyntheticFixture,
        binding: None,
        geometry: rig.clone(),
        notes: "no scanner data".into(),
    };
    storage::write(
        &record,
        fs::File::create(tmp.0.join("calibration.rapcal")).unwrap(),
    )
    .unwrap();
    for (name, data) in [("left.png", left), ("right.png", right)] {
        let bytes = data.into_iter().map(|x| x.round() as u8).collect();
        image::GrayImage::from_raw(96, 48, bytes)
            .unwrap()
            .save(tmp.0.join(name))
            .unwrap();
    }
    let input_bytes: Vec<_> = ["calibration.rapcal", "left.png", "right.png"]
        .iter()
        .map(|name| fs::read(tmp.0.join(name)).unwrap())
        .collect();
    let mut errors = Vec::new();
    for enabled in [false, true] {
        let mut matching = scene::config(4.25);
        if !enabled {
            matching.subpixel = None;
        }
        let request = serde_json::json!({"schema_version":1,"source":"synthetic_fixture","output_grid":rig.left,
            "matching":matching,"seeds":[[32,18],[48,24],[64,30]],"triangulation":{"min_depth_z_mm":10.,"max_depth_z_mm":1000.,"min_ray_angle_rad":0.0001,"max_ray_gap_mm":0.01,"max_reprojection_error_px":0.01}});
        let settings = tmp.0.join(if enabled {
            "subpixel.json"
        } else {
            "integer.json"
        });
        let bytes = serde_json::to_vec(&request).unwrap();
        fs::write(&settings, &bytes).unwrap();
        let target = tmp.0.join(if enabled { "subpixel" } else { "integer" });
        let run = || {
            Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
                .arg("reconstruct-stereo")
                .arg(tmp.0.join("calibration.rapcal"))
                .arg(tmp.0.join("left.png"))
                .arg(tmp.0.join("right.png"))
                .arg(&settings)
                .arg(&target)
                .output()
                .unwrap()
        };
        let result = run();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(target.join("report.json")).unwrap()).unwrap();
        assert_eq!(report["physical_accuracy_qualified"], false);
        assert_eq!(report["synchronization_verified"], false);
        let feature = &report["matching"]["accepted"][0];
        if enabled {
            assert!(feature["subpixel"].is_object());
        } else {
            assert!(feature.get("subpixel").is_none());
        }
        let project = fs::read(target.join("project.orproj")).unwrap();
        let editor = scancraft_geometry::formats::load_project(project.as_slice()).unwrap();
        assert_eq!(editor.cloud.points.len(), 3);
        errors.push(
            editor
                .cloud
                .points
                .iter()
                .map(|p| (p.position[2] - 2400. / 4.25).abs())
                .sum::<f64>()
                / 3.,
        );
        assert!(!run().status.success());
        assert_eq!(fs::read(target.join("project.orproj")).unwrap(), project);
        assert_eq!(fs::read(&settings).unwrap(), bytes);
    }
    assert!(errors[1] < errors[0] * 0.2, "{errors:?}");
    for (name, bytes) in ["calibration.rapcal", "left.png", "right.png"]
        .iter()
        .zip(input_bytes)
    {
        assert_eq!(fs::read(tmp.0.join(name)).unwrap(), bytes);
    }
}
