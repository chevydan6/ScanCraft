use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::{CaptureContext, Writer};
use std::{fs, path::PathBuf, process::Command};
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn native_recorded_sequence_saves_fused_project_and_preserves_input_bytes() {
    let temp = Temp(
        scancraft_workflow::output::staging(
            &std::env::temp_dir().join("scancraft-sequence-cli-test"),
        )
        .unwrap(),
    );
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/offline-cli-dataset");
    for name in ["calibration.rapcal", "request.json"] {
        fs::copy(source.join(name), temp.0.join(name)).unwrap();
    }
    for (name, interface) in [("left", 0), ("right", 2)] {
        let image = image::open(source.join(format!("{name}.png")))
            .unwrap()
            .to_luma8();
        let mut writer = Writer::with_context(
            Vec::new(),
            StreamProfile {
                interface,
                width: image.width(),
                height: image.height(),
                fourcc: *b"GREY",
                fps: 30,
            },
            CaptureContext {
                controls: serde_json::json!({"synthetic":true}),
                ..Default::default()
            },
        )
        .unwrap();
        for i in 0..3u32 {
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&(1000 + i * 33333).to_le_bytes());
            writer
                .push(&Frame {
                    sequence: i,
                    host_timestamp_us: 1000 + u64::from(i) * 33333,
                    flags: 0,
                    metadata,
                    data: image.as_raw().clone(),
                })
                .unwrap();
        }
        fs::write(
            temp.0.join(format!("{name}.raptor")),
            writer.finish().unwrap(),
        )
        .unwrap();
    }
    fs::write(temp.0.join("sequence.json"),serde_json::to_vec(&serde_json::json!({"schema_version":1,"session_id":"native-cli-sequence",
        "tracking":{"max_correspondence_mm":2.,"max_rms_mm":0.01,"minimum_overlap":0.8,"max_step_translation_mm":1.,"max_step_rotation_rad":0.1,"iterations":20,"tolerance_mm":1e-8},
        "voxel_mm":0.1,"maximum_voxels":1000,"pairs":[
            {"id":1,"left_index":0,"right_index":0,"tracking_indices":[0,1,2]},
            {"id":2,"left_index":1,"right_index":1,"tracking_indices":[0,1,2]},
            {"id":3,"left_index":2,"right_index":2,"tracking_indices":[0,1,2]}]})).unwrap()).unwrap();
    let names = [
        "calibration.rapcal",
        "request.json",
        "left.raptor",
        "right.raptor",
        "sequence.json",
    ];
    let originals: Vec<_> = names
        .iter()
        .map(|n| fs::read(temp.0.join(n)).unwrap())
        .collect();
    let target = temp.0.join("result");
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_scancraft-cli"))
            .arg("reconstruct-recorded-sequence")
            .args([
                temp.0.join("calibration.rapcal"),
                temp.0.join("left.raptor"),
                temp.0.join("right.raptor"),
                temp.0.join("request.json"),
                temp.0.join("sequence.json"),
                target.clone(),
            ])
            .output()
            .unwrap()
    };
    let result = run();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let summary: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(summary["integrated_frames"], 3);
    assert_eq!(summary["physical_accuracy_qualified"], false);
    let project = fs::read(target.join("project.orproj")).unwrap();
    let editor = scancraft_geometry::formats::load_project(project.as_slice()).unwrap();
    assert_eq!(
        editor.cloud.points.len() as u64,
        summary["points"].as_u64().unwrap()
    );
    assert!(target.join("frame-0002/right-frame.raptor").is_file());
    assert!(!run().status.success());
    assert_eq!(fs::read(target.join("project.orproj")).unwrap(), project);
    for (name, original) in names.iter().zip(originals) {
        assert_eq!(fs::read(temp.0.join(name)).unwrap(), original);
    }
}
