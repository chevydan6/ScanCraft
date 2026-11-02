//! Real helper process, synthetic recordings only; never opens scanner hardware.
use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::{Reader, Writer};
use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture(root: &std::path::Path) -> [Frame; 2] {
    let camera = json!({"width":10,"height":5,"fx":10.,"fy":10.,"cx":4.5,"cy":2.,"skew":0.,"valid_radius":1.,"distortion":{"k1":0.,"k2":0.,"k3":0.,"p1":0.,"p2":0.}});
    let mut second = camera.clone();
    second["cy"] = json!(1.5);
    fs::write(root.join("geometry.json"),serde_json::to_vec(&json!({"left":camera,"right":second,"right_from_left":{"rotation":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"translation_mm":[-1.,0.,0.]}})).unwrap()).unwrap();
    let request = json!({"schema_version":1,"fps":30,"maximum_frames_per_camera":1,"maximum_recording_bytes_per_camera":2000000,"maximum_pair_log_bytes":1000000,
        "processing":{"interfaces":[0,2],"reflect_x":[false,false],"maximum_host_delta_us":1000,
        "detection":{"background":10,"minimum_peak":30,"maximum_width":3,"saturation":255,"maximum_observations":100,"opening_radius":null},
        "continuity":{"maximum_row_gap":1,"maximum_horizontal_step":1.,"maximum_slope":0.,"minimum_span_rows":2,"minimum_observations":2},
        "sheets":[{"quadratic_y":0.,"linear_y":0.,"linear_z":0.,"offset":1.5}],
        "triangulation":{"min_depth_z_mm":1.,"max_depth_z_mm":100.,"min_ray_angle_rad":0.0001,"max_ray_gap_mm":0.01,"max_reprojection_error_px":0.01},
        "matching":{"intersection":{"maximum_segment_length_px":3.,"normalized_tolerance":1e-10,"minimum_plane_sine":1e-8},"maximum_sheet_distance_px":0.25,"maximum_work":1000},"maximum_queries":100,"maximum_total_work":10000}});
    fs::write(
        root.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let frames = [6, 5].map(|x| {
        let mut data = vec![0; 50];
        for y in 0..5 {
            data[y * 10 + x] = 80;
        }
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&100u32.to_le_bytes());
        Frame {
            sequence: 0,
            host_timestamp_us: 1000,
            flags: 0,
            metadata,
            data,
        }
    });
    for (i, name) in ["first.raptor", "second.raptor"].iter().enumerate() {
        let mut w = Writer::new(
            fs::File::create(root.join(name)).unwrap(),
            StreamProfile {
                interface: if i == 0 { 0 } else { 2 },
                width: 10,
                height: 5,
                fourcc: *b"GREY",
                fps: 30,
            },
        )
        .unwrap();
        w.push(&frames[i]).unwrap();
        w.finish().unwrap();
    }
    frames
}
fn helper(root: &std::path::Path) -> (bool, Vec<(u8, Value)>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_scancraft"))
        .arg("--laser-helper-replay")
        .args([
            root.join("geometry.json"),
            root.join("request.json"),
            root.join("first.raptor"),
            root.join("second.raptor"),
            root.join("result"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let control = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut b = Vec::new();
        output.read_to_end(&mut b).unwrap();
        b
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if Instant::now() > deadline {
            drop(control);
            let _ = child.wait();
            panic!("device-free helper timed out");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    drop(control);
    let bytes = reader.join().unwrap();
    let mut remaining = bytes.as_slice();
    let mut packets = Vec::new();
    while !remaining.is_empty() {
        assert!(remaining.len() >= 5);
        let length = u32::from_le_bytes(remaining[1..5].try_into().unwrap()) as usize;
        assert!(length <= 2 * 1024 * 1024);
        assert!(remaining.len() >= 5 + length);
        packets.push((
            remaining[0],
            serde_json::from_slice(&remaining[5..5 + length]).unwrap(),
        ));
        remaining = &remaining[5 + length..];
    }
    (status.success(), packets)
}
#[test]
fn actual_replay_helper_preserves_raw_data_preview_candidates_and_no_overwrite() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = Temp(std::env::temp_dir().join(format!(
        "scancraft-laser-process-{}-{stamp}",
        std::process::id()
    )));
    fs::create_dir(&root.0).unwrap();
    let frames = fixture(&root.0);
    let (ok, packets) = helper(&root.0);
    assert!(ok, "{packets:?}");
    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0].0, 5);
    assert_eq!(packets[1].0, 6);
    let preview = &packets[0].1;
    let completion = &packets[1].1["Ok"];
    assert_eq!(preview["origin"], "recording_replay");
    assert_eq!(preview["indices"], json!([0, 0]));
    assert_eq!(preview["positions"].as_array().unwrap().len(), 4);
    assert_eq!(completion["origin"], "recording_replay");
    assert_eq!(completion["recorded"], json!([1, 1]));
    assert_eq!(completion["processed"], 1);
    let result = root.0.join("result");
    let report: Value =
        serde_json::from_slice(&fs::read(result.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["raw_replay_integrity"], "PASS");
    assert_eq!(report["physical_accuracy_qualified"], false);
    assert_eq!(report["geometry_exported"], false);
    let pair: Value =
        serde_json::from_slice(&fs::read(result.join("pairs.ndjson")).unwrap()).unwrap();
    let expected = pair["result"]["Ok"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["position_in_supplied_geometry_units"].clone())
        .collect::<Vec<_>>();
    assert_eq!(preview["positions"], json!(expected));
    for (i, name) in ["first.raptor", "second.raptor"].iter().enumerate() {
        let mut r = Reader::new(fs::File::open(result.join(name)).unwrap()).unwrap();
        assert_eq!(
            r.context.as_ref().unwrap().calibration_status,
            "recording_replay_unverified"
        );
        assert_eq!(r.read_frame().unwrap().unwrap(), frames[i]);
        assert!(r.read_frame().unwrap().is_none());
        assert!(!result.join("points.ply").exists());
    }
    let before = fs::read(result.join("report.json")).unwrap();
    let (ok, packets) = helper(&root.0);
    assert!(!ok);
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].0, 6);
    assert!(packets[0].1["Err"].is_string());
    assert_eq!(fs::read(result.join("report.json")).unwrap(), before);
}
