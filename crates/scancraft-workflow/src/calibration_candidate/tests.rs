use super::*;
use crate::FileInputs;
use std::{fs, path::PathBuf};

struct Temporary(PathBuf);
impl Temporary {
    fn new() -> Self {
        Self(staging(&std::env::temp_dir().join("calibration-handoff-test")).unwrap())
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn dataset() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset")
}
fn request() -> Value {
    let calibration =
        storage::read(File::open(dataset().join("calibration.rapcal")).unwrap()).unwrap();
    let rig = calibration.geometry;
    let views: Vec<_> = (0..5)
        .map(|i| {
            let (s, c) = (i as f64 * 0.15 - 0.3).sin_cos();
            let mut observations = Vec::new();
            for y in 0..5 {
                for x in 0..7 {
                    let xy = [(x as f64 - 3.) * 4., (y as f64 - 2.) * 4.];
                    let point = [
                        c * xy[0] + i as f64 * 2.,
                        xy[1],
                        -s * xy[0] + 100. + i as f64 * 20.,
                    ];
                    observations.push(
                        json!({"target_xy_mm":xy,"left_pixel":rig.left.project(point).unwrap(),
                    "right_pixel":rig.right.project(rig.right_from_left.apply(point)).unwrap()}),
                    );
                }
            }
            json!({"observations":observations})
        })
        .collect();
    json!({"schema_version":1,"record_id":"synthetic-fitted-handoff",
        "target_units":"declared_millimeters","unit_reference":"Synthetic coordinates with explicitly defined millimeter units; not hardware measurements.",
        "binding":{"serial":"synthetic-test","firmware":"synthetic-test","mode":"synthetic-test","left_size":[64,32],"right_size":[64,32]},
        "observations":{"schema_version":1,"provenance":"synthetic_fixture","fixed_left":rig.left,"fixed_right":rig.right,
        "views":views,"fitting":{"max_iterations":80,"max_reprojection_error_px":0.01}}})
}
fn save(root: &Path, value: &Value) -> PathBuf {
    let path = root.join("input.json");
    fs::write(&path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    path
}

#[test]
fn fitted_record_drives_existing_image_reconstruction_without_promoting_synthetic_input() {
    let temp = Temporary::new();
    let input = save(&temp.0, &request());
    let original = fs::read(&input).unwrap();
    let target = temp.0.join("candidate");
    let work = WorkControl::default();
    let manifest = run(&input, &target, &work, &|| false).unwrap();
    assert_eq!(fs::read(&input).unwrap(), original);
    assert_eq!(fs::read(target.join("request.json")).unwrap(), original);
    let record = storage::read(File::open(target.join("calibration.rapcal")).unwrap()).unwrap();
    assert_eq!(record.provenance, Provenance::SyntheticFixture);
    assert!(
        record
            .validate_for_capture(record.binding.as_ref().unwrap())
            .is_err()
    );
    for flag in [
        "units_verified",
        "device_binding_verified",
        "physical_accuracy_qualified",
        "synchronization_verified",
        "calibration_activated",
    ] {
        assert_eq!(manifest[flag], false);
    }
    for a in manifest["artifacts"].as_array().unwrap() {
        let bytes = fs::read(target.join(a["role"].as_str().unwrap())).unwrap();
        assert_eq!(a["bytes"].as_u64().unwrap(), bytes.len() as u64);
        assert_eq!(a["crc32"].as_u64().unwrap(), crc32fast::hash(&bytes) as u64);
    }
    let mut inputs = FileInputs {
        calibration: dataset().join("calibration.rapcal"),
        left: dataset().join("left.png"),
        right: dataset().join("right.png"),
        settings: dataset().join("request.json"),
    };
    let reference = crate::prepare(&inputs, &work)
        .unwrap()
        .create_editor(&temp.0)
        .unwrap();
    inputs.calibration = target.join("calibration.rapcal");
    let prepared = crate::prepare(&inputs, &work).unwrap();
    let reconstructed = prepared.create_editor(&temp.0).unwrap();
    assert_eq!(reference.cloud.points.len(), 3);
    assert_eq!(reconstructed.cloud.points.len(), 3);
    for (a, b) in reference
        .cloud
        .points
        .iter()
        .zip(&reconstructed.cloud.points)
    {
        for axis in 0..3 {
            assert!((a.position[axis] - b.position[axis]).abs() < 1e-5);
        }
    }
    let summary =
        crate::publish_prepared(&prepared, &temp.0.join("reconstruction"), &work).unwrap();
    assert_eq!(summary.points, 3);
    assert!(!summary.physical_accuracy_qualified);
    assert!(run(&input, &target, &work, &|| false).is_err());
    assert_eq!(fs::read(target.join("request.json")).unwrap(), original);
}

#[test]
fn unknown_units_invalid_binding_and_invalid_fit_never_publish_a_candidate() {
    let temp = Temporary::new();
    let mut values = Vec::new();
    let mut unknown = request();
    unknown["target_units"] = json!("unverified");
    values.push(unknown);
    let mut reference = request();
    reference["unit_reference"] = json!(" ");
    values.push(reference);
    let mut binding = request();
    binding["binding"]["left_size"] = json!([128, 32]);
    values.push(binding);
    let mut outlier = request();
    outlier["observations"]["views"][0]["observations"][0]["right_pixel"] = json!([10000., 10000.]);
    values.push(outlier);
    for (i, value) in values.iter().enumerate() {
        let input = save(&temp.0, value);
        let output = temp.0.join(format!("invalid-{i}"));
        assert!(run(&input, &output, &WorkControl::default(), &|| false).is_err());
        assert!(!output.exists());
    }
}

#[test]
fn supplied_physical_declarations_remain_unverified_and_cancellation_cannot_publish() {
    let temp = Temporary::new();
    let mut value = request();
    // Still a generated test fixture: exercise declared-input policy, not hardware.
    value["observations"]["provenance"] = json!("user_supplied_unverified");
    let input = save(&temp.0, &value);
    let output = temp.0.join("candidate");
    let manifest = run(&input, &output, &WorkControl::default(), &|| false).unwrap();
    let record = storage::read(File::open(output.join("calibration.rapcal")).unwrap()).unwrap();
    assert_eq!(record.provenance, Provenance::EstimatedFromObservations);
    assert_eq!(manifest["units_verified"], false);
    assert_eq!(manifest["device_binding_verified"], false);
    assert_eq!(manifest["physical_accuracy_qualified"], false);
    let cancelled = temp.0.join("cancelled");
    let calls = std::sync::atomic::AtomicUsize::new(0);
    assert!(
        run(&input, &cancelled, &WorkControl::default(), &|| calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            > 10)
        .is_err()
    );
    assert!(!cancelled.exists());
}
