//! Emits synthetic numerical cases only. None of these parameters describe a Raptor.
use scancraft_calibration::{
    Brown5, CameraModel,
    stereo::{RigidPose, StereoCalibration, TriangulationLimits},
};
use serde_json::json;
fn main() {
    let camera = CameraModel {
        width: 1280,
        height: 800,
        fx: 1000.,
        fy: 980.,
        cx: 639.5,
        cy: 399.5,
        skew: 0.,
        distortion: Brown5 {
            k1: -0.04,
            k2: 0.01,
            k3: 0.0002,
            p1: 0.0001,
            p2: -0.0002,
        },
        valid_radius: 1.5,
    };
    let angle = 0.03f64;
    let mut right = camera.clone();
    right.fx = 1100.;
    right.fy = 1050.;
    right.cx = 630.;
    let rig = StereoCalibration {
        left: camera,
        right,
        right_from_left: RigidPose {
            rotation: [
                [angle.cos(), 0., angle.sin()],
                [0., 1., 0.],
                [-angle.sin(), 0., angle.cos()],
            ],
            translation_mm: [-120., 2., 0.5],
        },
    };
    let limits = TriangulationLimits {
        min_depth_z_mm: 10.,
        max_depth_z_mm: 2000.,
        min_ray_angle_rad: 0.0001,
        max_ray_gap_mm: 0.1,
        max_reprojection_error_px: 0.1,
    };
    let prepared = rig.prepare(limits).unwrap();
    let mut cases = Vec::new();
    for z in [300., 500., 800., 1200.] {
        for x in [-60., -20., 0., 40., 80.] {
            for y in [-50., 0., 50.] {
                let p = [x, y, z];
                let left = rig.left.project(p).unwrap();
                let right = rig.right.project(rig.right_from_left.apply(p)).unwrap();
                let point = prepared.triangulate(left, right).unwrap();
                cases.push(json!({"ground_truth_mm":p,"left_pixel":left,"right_pixel":right,"left_ray":rig.left.ray(left).unwrap(),"right_ray":rig.right.ray(right).unwrap(),"triangulated":point}));
            }
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"provenance":"synthetic_fixture_not_scanner_calibration","rig":rig,"cases":cases})).unwrap());
}
