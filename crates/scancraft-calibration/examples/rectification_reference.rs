//! Synthetic numerical rectification reference; no Raptor coefficients or frames.
use nalgebra::Rotation3;
use scancraft_calibration::{
    Brown5, CameraModel,
    rectification::StereoRectification,
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
    let mut right = camera.clone();
    right.fx = 1100.;
    right.fy = 1050.;
    right.cx = 630.;
    let rotation = Rotation3::from_euler_angles(-0.01, 0.07, 0.02).into_inner();
    let source = StereoCalibration {
        left: camera.clone(),
        right,
        right_from_left: RigidPose {
            rotation: std::array::from_fn(|r| std::array::from_fn(|c| rotation[(r, c)])),
            translation_mm: [-120., 5., 2.],
        },
    };
    let mut grid = camera;
    grid.distortion = Brown5::ZERO;
    grid.fx = 900.;
    grid.fy = 880.;
    let rectification = StereoRectification::build(&source, grid).unwrap();
    let limits = TriangulationLimits {
        min_depth_z_mm: 10.,
        max_depth_z_mm: 2000.,
        min_ray_angle_rad: 0.0001,
        max_ray_gap_mm: 0.01,
        max_reprojection_error_px: 0.01,
    };
    let prepared = source.prepare(limits).unwrap();
    let mut cases = Vec::new();
    for z in [300., 500., 800., 1200.] {
        for x in [-60., -20., 0., 40., 80.] {
            for y in [-50., 0., 50.] {
                let p = [x, y, z];
                let lp = source.left.project(p).unwrap();
                let rp = source
                    .right
                    .project(source.right_from_left.apply(p))
                    .unwrap();
                let (l, r) = rectification.rectify_pixels(lp, rp).unwrap();
                let (sl, sr) = rectification.source_pixels(l, r).unwrap();
                let point = prepared.triangulate(sl, sr).unwrap();
                cases.push(json!({"ground_truth_mm":p,"left_source":lp,"right_source":rp,"left_rectified":l,"right_rectified":r,
            "left_remapped_source":sl,"right_remapped_source":sr,"triangulated_original":point}));
            }
        }
    }
    println!("{}",serde_json::to_string(&json!({"provenance":"synthetic_fixture_not_scanner_calibration","rectification":rectification,"cases":cases})).unwrap());
}
