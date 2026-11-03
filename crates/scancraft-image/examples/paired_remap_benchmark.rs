//! Two independent synthetic image maps. Exact-output comparison; no live FPS claims.
use scancraft_calibration::{
    Brown5, CameraModel,
    rectification::StereoRectification,
    stereo::{RigidPose, StereoCalibration},
};
use scancraft_geometry::work::WorkControl;
use scancraft_image::{GrayImage, MaskedImage, stereo::StereoImageMaps};
use serde_json::json;
use std::{hint::black_box, time::Instant};
type Pair = (MaskedImage, MaskedImage);
fn maps(width: u32, height: u32) -> StereoImageMaps {
    let sx = f64::from(width) / 1920.;
    let sy = f64::from(height) / 1200.;
    let cameras = [0., 1.].map(|offset| CameraModel {
        width,
        height,
        fx: (1382.317 + offset) * sx,
        fy: 1379.117 * sy,
        cx: 960.713 * sx,
        cy: 602.332 * sy,
        skew: 0.01 * offset * sx,
        distortion: Brown5 {
            k1: -0.04,
            k2: 0.01,
            k3: 0.0002,
            p1: 0.0001,
            p2: -0.0002,
        },
        valid_radius: 1.2,
    });
    let source = StereoCalibration {
        left: cameras[0].clone(),
        right: cameras[1].clone(),
        right_from_left: RigidPose {
            rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            translation_mm: [-45., 0., 0.],
        },
    };
    let mut grid = source.left.clone();
    grid.distortion = Brown5::ZERO;
    grid.skew = 0.;
    StereoImageMaps::build(
        StereoRectification::build(&source, grid).unwrap(),
        &WorkControl::default(),
    )
    .unwrap()
}
fn sequential(maps: &StereoImageMaps, images: [GrayImage<'_>; 2], work: &WorkControl) -> Pair {
    maps.apply_sequential(images[0], images[1], work).unwrap()
}
fn concurrent(maps: &StereoImageMaps, images: [GrayImage<'_>; 2], work: &WorkControl) -> Pair {
    maps.apply(images[0], images[1], work).unwrap()
}

fn exact(expected: &Pair, actual: &Pair) {
    for (expected, actual) in [(&expected.0, &actual.0), (&expected.1, &actual.1)] {
        assert_eq!(expected.size(), actual.size());
        assert_eq!(expected.valid(), actual.valid());
        assert_eq!(expected.intensity().len(), actual.intensity().len());
        assert!(
            expected
                .intensity()
                .iter()
                .zip(actual.intensity())
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }
}
fn summary(mut times: Vec<f64>) -> serde_json::Value {
    times.sort_by(f64::total_cmp);
    json!({"runs":times.len(),"median_ms":(times[9]+times[10])/2.,"p95_ms":times[18],"min_ms":times[0],"max_ms":times[19]})
}
fn benchmark(width: u32, height: u32) -> serde_json::Value {
    let start = Instant::now();
    let maps = maps(width, height);
    let build_ms = start.elapsed().as_secs_f64() * 1000.;
    let inputs = [0, 1].map(|side| {
        (0..width as usize * height as usize)
            .map(|i| ((i * 37 + i / width as usize * 19 + side * 53) % 256) as u8)
            .collect::<Vec<_>>()
    });
    let images = [
        GrayImage::new(width, height, &inputs[0]).unwrap(),
        GrayImage::new(width, height, &inputs[1]).unwrap(),
    ];
    let work = WorkControl::default();
    let reference = sequential(&maps, images, &work);
    exact(&reference, &concurrent(&maps, images, &work));
    let mut seq = Vec::new();
    let mut par = Vec::new();
    // Alternate order each round to limit a fixed warm/cache/order bias.
    for round in 0..20 {
        for parallel in if round % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let start = Instant::now();
            let output = black_box(if parallel {
                concurrent(black_box(&maps), black_box(images), &work)
            } else {
                sequential(black_box(&maps), black_box(images), &work)
            });
            let elapsed = start.elapsed().as_secs_f64() * 1000.;
            exact(&reference, &output); // Validation is outside the measured interval.
            if parallel {
                par.push(elapsed);
            } else {
                seq.push(elapsed);
            }
        }
    }
    json!({
        "image_size":[width,height],"map_build_ms":build_ms,
        "sequential":summary(seq),"adaptive_production":summary(par),
        "alternating_measurement_order":true,"each_measured_pair_matches_reference_bits_and_masks":true,
        "comparison_outside_timing":true,"output_allocation_and_thread_spawn_join_included":true,
        "map_construction_excluded":true,"warmups_per_mode":1,
        "map_coordinate_storage_bytes":maps.coordinate_storage_bytes()
    })
}
fn main() {
    let cases = [[64, 32], [320, 240], [640, 480], [1920, 1200]].map(|[w, h]| benchmark(w, h));
    println!("{}",serde_json::to_string_pretty(&json!({
        "scope":"production StereoImageMaps synthetic paired remapping benchmark; not hardware, accuracy or live throughput qualification",
        "available_parallelism_hint":std::thread::available_parallelism().ok().map(|n|n.get()),
        "cases":cases
    })).unwrap());
}
