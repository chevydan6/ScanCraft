use super::*;
use scancraft_core::{Frame, StreamProfile};
use std::{
    cell::Cell,
    fs,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
    request: Value,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "scancraft-target-pair-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let pattern = [
            [0., 0.],
            [3., 0.],
            [0., 2.],
            [4., 3.],
            [0.3, 0.6],
            [1.2, 1.5],
            [2.8, 0.7],
            [3.1, 2.3],
            [1.8, 2.8],
            [3.7, 1.2],
        ];
        let target:Vec<_> = pattern.iter().enumerate().map(|(i,p)| json!({"pattern":p,"anchor":i<4,"coordinates":if i<4 {Value::Null} else {json!([p[0]*7.3,p[1]*7.3])}})).collect();
        for (side, interface) in [("left", 0), ("right", 2)] {
            let mut data = vec![0; 220 * 180];
            for (i, p) in pattern.iter().enumerate() {
                let cx = (p[0] * 40. + 20.) as i32 + if interface == 2 { 4 } else { 0 };
                let cy = (p[1] * 40. + 20.) as i32;
                let radius = if i < 4 { 8 } else { 4 };
                for y in cy - radius..=cy + radius {
                    for x in cx - radius..=cx + radius {
                        if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= radius * radius {
                            data[(y * 220 + x) as usize] = 220;
                        }
                    }
                }
            }
            let mut writer = Writer::with_context(
                File::create(root.join(format!("{side}.raptor"))).unwrap(),
                StreamProfile {
                    interface,
                    width: 220,
                    height: 180,
                    fourcc: *b"GREY",
                    fps: 30,
                },
                Default::default(),
            )
            .unwrap();
            for index in 0..2 {
                let matched = index == usize::from(interface == 0);
                let mut metadata = vec![0; 16];
                metadata[10] = 6;
                metadata[11] = 4;
                metadata[12..].copy_from_slice(
                    &(if matched {
                        123u32
                    } else {
                        if interface == 0 { 45 } else { 223 }
                    })
                    .to_le_bytes(),
                );
                writer
                    .push(&Frame {
                        sequence: index as u32,
                        host_timestamp_us: 1000
                            + index as u64 * 1000
                            + if interface == 2 { 1002 } else { 0 },
                        flags: 0,
                        metadata,
                        data: data.clone(),
                    })
                    .unwrap();
            }
            writer.finish().unwrap();
        }
        Self {
            root,
            request: json!({"schema_version":1,"target_units":"unverified","target":target,
            "left":{"recording":"left.raptor","index":1},"right":{"recording":"right.raptor","index":0},
            "maximum_host_delta_us":5,"detection":{"threshold":100,"minimum_area":10,"maximum_area":300,"maximum_components":100,"anchor_area_above":100,"maximum_shape_rms_px":1.},
            "association":{"seed_radius_px":2.,"refined_radius_px":1.,"minimum_matches":8,"minimum_support_margin":2}}),
        }
    }
    fn save(&self) -> PathBuf {
        let path = self.root.join("request.json");
        fs::write(&path, serde_json::to_vec_pretty(&self.request).unwrap()).unwrap();
        path
    }
    fn run(&self, name: &str) -> Result<Value, Error> {
        run(
            &self.save(),
            &self.root.join(name),
            &WorkControl::default(),
            &|| false,
        )
    }
}
fn read(path: PathBuf) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
#[test]
fn explicit_pair_retains_observations_raw_frames_and_unit_boundary() {
    let mut f = Fixture::new();
    let manifest = f.run("unverified").unwrap();
    assert_eq!(manifest["fit_views_exported"], false);
    assert!(!f.root.join("unverified/left-view.json").exists());
    let report = read(f.root.join("unverified/measurements.json"));
    assert_eq!(report["sensor_pts"], 123);
    assert_eq!(report["host_delta_us"], 2);
    assert_eq!(report["stereo_observations"].as_array().unwrap().len(), 6);
    for flag in [
        "units_verified",
        "physical_accuracy_qualified",
        "source_device_binding_verified",
        "synchronization_verified",
        "calibration_activated",
    ] {
        assert_eq!(report[flag], false);
    }
    for (side, index) in [("left", 1), ("right", 0)] {
        let work = WorkControl::default();
        let original = Playback::open(&f.root.join(format!("{side}.raptor")), &work)
            .unwrap()
            .select_raw(index, &work)
            .unwrap();
        let copy = Playback::open(
            &f.root.join(format!("unverified/{side}-frame.raptor")),
            &work,
        )
        .unwrap()
        .select_raw(0, &work)
        .unwrap();
        assert_eq!(original.frame, copy.frame);
        assert_eq!(original.profile, copy.profile);
        assert_eq!(
            serde_json::to_value(original.context).unwrap(),
            serde_json::to_value(copy.context).unwrap()
        );
    }
    f.request["target_units"] = json!("declared_millimeters");
    assert_eq!(f.run("declared").unwrap()["fit_views_exported"], true);
    let left: scancraft_calibration::planar::PlanarView =
        serde_json::from_value(read(f.root.join("declared/left-view.json"))).unwrap();
    let stereo: scancraft_calibration::stereo_fit::StereoView =
        serde_json::from_value(read(f.root.join("declared/stereo-view.json"))).unwrap();
    assert_eq!(left.observations.len(), 6);
    assert_eq!(stereo.observations.len(), 6);
    assert_eq!(left.observations[0].target_xy_mm, [0.3 * 7.3, 0.6 * 7.3]);
    assert_eq!(left.observations[0].pixel, [32., 44.]);
    assert_eq!(stereo.observations[0].right_pixel, [36., 44.]);
    for entry in manifest["artifacts"].as_array().unwrap() {
        let data = fs::read(
            f.root
                .join("unverified")
                .join(entry["role"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(entry["bytes"], data.len());
        assert_eq!(entry["crc32"], crc32fast::hash(&data));
    }
}
#[test]
fn pairing_integrity_and_request_errors_do_not_publish() {
    for case in 0..5 {
        let mut f = Fixture::new();
        match case {
            0 => f.request["left"]["index"] = json!(0),
            1 => f.request["maximum_host_delta_us"] = json!(1),
            2 => f.request["left"]["recording"] = json!("right.raptor"),
            3 => f.request["unexpected"] = json!(true),
            _ => {
                let path = f.root.join("left.raptor");
                let mut data = fs::read(&path).unwrap();
                data.pop();
                fs::write(path, data).unwrap();
            }
        }
        assert!(f.run("out").is_err(), "case {case}");
        assert!(!f.root.join("out").exists());
    }
}
#[test]
fn cancellation_replacement_and_existing_destinations_are_safe() {
    let f = Fixture::new();
    let input = f.save();
    let out = f.root.join("out");
    let work = WorkControl::default();
    assert!(run(&input, &out, &work, &|| true).is_err());
    assert!(!out.exists());
    std::os::unix::fs::symlink(f.root.join("absent"), &out).unwrap();
    assert!(
        run(&input, &out, &work, &|| false)
            .unwrap_err()
            .to_string()
            .contains("already exists")
    );
    fs::remove_file(&out).unwrap();
    let changed = Cell::new(false);
    let result = run(&input, &out, &work, &|| {
        if !changed.get()
            && fs::read_dir(&f.root).unwrap().any(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("out.partial-")
            })
        {
            let original = f.root.join("left.raptor");
            let replacement = f.root.join("replacement");
            fs::copy(&original, &replacement).unwrap();
            fs::rename(replacement, original).unwrap();
            changed.set(true);
        }
        false
    });
    assert!(changed.get());
    assert!(result.unwrap_err().to_string().contains("source changed"));
    assert!(!out.exists());
}

#[test]
fn shape_rejections_remain_visible_and_cannot_become_fit_inputs() {
    let mut f = Fixture::new();
    f.request["target_units"] = json!("declared_millimeters");
    f.request["detection"]["maximum_shape_rms_px"] = json!(1e-12);
    let manifest = f.run("rejected").unwrap();
    let report = read(f.root.join("rejected/measurements.json"));
    assert_eq!(manifest["fit_views_exported"], false);
    assert_eq!(report["minimum_fit_point_count_met"], false);
    assert_eq!(
        report["cameras"][0]["raw_observations"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    assert_eq!(
        report["cameras"][0]["association"]["matches"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    assert!(report["stereo_observations"].as_array().unwrap().is_empty());
    assert!(!f.root.join("rejected/stereo-view.json").exists());
}
