use super::*;
use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::Writer;
use std::{
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
            "scancraft-board-session-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let mut target = Vec::new();
        for x in 0..7 {
            for y in 0..6 {
                let anchor = [(3, 2), (4, 2), (2, 3), (4, 3)].contains(&(x, y));
                target.push(json!({"pattern":[x,y],"anchor":anchor,"coordinates":if anchor {Value::Null}else{json!([x as f64*20.,y as f64*20.])}}));
            }
        }
        for pose in 0..7 {
            for (side, interface) in [("left", 0), ("right", 2)] {
                let mut data = vec![0; 800 * 600];
                let (sx, cx) = (0.4 * (pose as f64 * 0.8).sin()).sin_cos();
                let (sy, cy) = (0.35 * (pose as f64 * 0.6).cos()).sin_cos();
                for t in &target {
                    let x = (t["pattern"][0].as_f64().unwrap() - 3.) * 20.;
                    let y = (t["pattern"][1].as_f64().unwrap() - 2.5) * 20.;
                    let point = [
                        cy * x + sy * sx * y - 30. + pose as f64 * 8.
                            - if interface == 2 { 60. } else { 0. },
                        cx * y,
                        -sy * x + cy * sx * y + 260. + pose as f64 * 15.,
                    ];
                    let px = 520. * point[0] / point[2] + 391.;
                    let py = 500. * point[1] / point[2] + 299.;
                    let radius = if t["anchor"] == true { 6. } else { 3. };
                    for yy in (py - radius - 1.) as usize..=(py + radius + 1.) as usize {
                        for xx in (px - radius - 1.) as usize..=(px + radius + 1.) as usize {
                            if (xx as f64 - px).powi(2) + (yy as f64 - py).powi(2)
                                <= radius * radius
                            {
                                data[yy * 800 + xx] = 220;
                            }
                        }
                    }
                }
                let mut metadata = vec![0; 16];
                metadata[10] = 6;
                metadata[11] = 4;
                metadata[12..16].copy_from_slice(&(123 + pose as u32 * 1000).to_le_bytes());
                let mut writer = Writer::with_context(
                    File::create(root.join(format!("{pose}-{side}.raptor"))).unwrap(),
                    StreamProfile {
                        interface,
                        width: 800,
                        height: 600,
                        fourcc: *b"GREY",
                        fps: 30,
                    },
                    Default::default(),
                )
                .unwrap();
                writer
                    .push(&Frame {
                        sequence: pose as u32,
                        host_timestamp_us: 1000 + pose as u64 * 1000 + u64::from(interface),
                        flags: 0,
                        metadata,
                        data,
                    })
                    .unwrap();
                writer.finish().unwrap();
            }
            let r = json!({"schema_version":1,"target_units":"declared_millimeters","target":target,
                "left":{"recording":format!("{pose}-left.raptor"),"index":0},"right":{"recording":format!("{pose}-right.raptor"),"index":0},"maximum_host_delta_us":5,
                "detection":{"threshold":100,"minimum_area":10,"maximum_area":300,"maximum_components":100,"anchor_area_above":70,"maximum_shape_rms_px":1.},
                "association":{"seed_radius_px":6.,"refined_radius_px":3.,"minimum_matches":20,"minimum_support_margin":3}});
            fs::write(
                root.join(format!("pose-{pose}.json")),
                serde_json::to_vec(&r).unwrap(),
            )
            .unwrap();
        }
        Self {
            root,
            request: json!({"schema_version":1,"training":(0..6).map(|i|format!("pose-{i}.json")).collect::<Vec<_>>(),"validation":["pose-6.json"],"fit_intrinsics":true,"initial_max_reprojection_error_px":20.,"refinement":{"max_iterations":80,"max_reprojection_error_px":2.,"estimate_skew":false}}),
        }
    }
    fn save(&self) -> PathBuf {
        let p = self.root.join("session.json");
        fs::write(&p, serde_json::to_vec_pretty(&self.request).unwrap()).unwrap();
        p
    }
    fn run(&self, name: &str) -> Result<Value, Error> {
        run(
            &self.save(),
            &self.root.join(name),
            &WorkControl::default(),
            &|| false,
        )
    }
    fn units(&self, units: &str) {
        for i in 0..7 {
            let p = self.root.join(format!("pose-{i}.json"));
            let mut r: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
            r["target_units"] = json!(units);
            fs::write(p, serde_json::to_vec(&r).unwrap()).unwrap();
        }
    }
}
fn read(p: PathBuf) -> Value {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
#[test]
fn session_fits_only_training_and_preserves_complete_validation_observations() {
    let f = Fixture::new();
    let m = f.run("out").unwrap();
    assert_eq!(m["intrinsics_fitted"], true);
    assert_eq!(m["units_verified"], false);
    assert_eq!(m["validation_used_in_fitting"], false);
    let d = read(f.root.join("out/dataset.json"));
    assert_eq!(d["poses"].as_array().unwrap().len(), 7);
    for side in ["left", "right"] {
        let req = read(f.root.join(format!("out/{side}-intrinsics-request.json")));
        assert_eq!(req["views"].as_array().unwrap().len(), 6);
        let fit = read(f.root.join(format!("out/{side}-fit/fit.json")));
        assert_eq!(fit["refinement"]["views"].as_array().unwrap().len(), 6);
        assert!(fit["refinement"]["rms_px"].as_f64().unwrap() < 1.);
        assert_eq!(fit["physical_accuracy_qualified"], false);
    }
    assert_eq!(d["validation"][0].as_array().unwrap().len(), 1);
    assert!(f.root.join("out/validation-00/left-frame.raptor").is_file());
    let exact = fs::read(f.root.join("out/dataset.json")).unwrap();
    assert!(f.run("out").is_err());
    assert_eq!(exact, fs::read(f.root.join("out/dataset.json")).unwrap());
    for a in m["artifacts"].as_array().unwrap() {
        let bytes = fs::read(f.root.join("out").join(a["role"].as_str().unwrap())).unwrap();
        assert_eq!(a["bytes"], bytes.len());
        assert_eq!(a["crc32"], crc32fast::hash(&bytes));
    }
}
#[test]
fn unknown_units_remain_raw_and_cannot_trigger_metric_fitting() {
    let mut f = Fixture::new();
    f.units("unverified");
    f.request["fit_intrinsics"] = json!(false);
    let m = f.run("raw").unwrap();
    assert_eq!(m["intrinsics_fitted"], false);
    assert_eq!(m["intrinsic_requests_exported"], false);
    assert!(!f.root.join("raw/left-intrinsics-request.json").exists());
    f.request["fit_intrinsics"] = json!(true);
    assert!(
        f.run("bad")
            .unwrap_err()
            .to_string()
            .contains("requires declared millimeters")
    );
    assert!(!f.root.join("bad").exists());
}
#[test]
fn duplicate_validation_and_changed_target_do_not_publish() {
    let mut f = Fixture::new();
    f.request["fit_intrinsics"] = json!(false);
    fs::copy(
        f.root.join("pose-0.json"),
        f.root.join("copied-request.json"),
    )
    .unwrap();
    f.request["validation"] = json!(["copied-request.json"]);
    assert!(
        f.run("reused")
            .unwrap_err()
            .to_string()
            .contains("camera-frame fingerprint")
    );
    assert!(!f.root.join("reused").exists());
    f.request["validation"] = json!(["pose-6.json"]);
    let p = f.root.join("pose-6.json");
    let mut r = read(p.clone());
    r["target"][0]["coordinates"] = json!([1., 0.]);
    fs::write(p, serde_json::to_vec(&r).unwrap()).unwrap();
    assert!(
        f.run("changed")
            .unwrap_err()
            .to_string()
            .contains("target definition")
    );
    assert!(!f.root.join("changed").exists());
    assert!(
        run(
            &f.save(),
            &f.root.join("cancelled"),
            &WorkControl::default(),
            &|| true
        )
        .is_err()
    );
    assert!(!f.root.join("cancelled").exists());
}
