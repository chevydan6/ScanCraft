//! Single-workstation synthetic benchmark. No Raptor accuracy or live FPS claims.
use scancraft_calibration::{Brown5, CameraModel};
use scancraft_geometry::work::WorkControl;
use scancraft_image::{GrayImage, UndistortMap};
use serde_json::json;
use std::{hint::black_box, time::Instant};
fn main() {
    let camera = CameraModel {
        width: 1920,
        height: 1200,
        fx: 1382.317,
        fy: 1379.117,
        cx: 960.713,
        cy: 602.332,
        skew: 0.,
        distortion: Brown5 {
            k1: -0.04,
            k2: 0.01,
            k3: 0.0002,
            p1: 0.0001,
            p2: -0.0002,
        },
        valid_radius: 1.2,
    };
    let mut grid = camera.clone();
    grid.distortion = Brown5::ZERO;
    let work = WorkControl::default();
    let start = Instant::now();
    let map = UndistortMap::build(
        &camera,
        &grid,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        &work,
    )
    .unwrap();
    let map_build_ms = start.elapsed().as_secs_f64() * 1000.;
    let data: Vec<u8> = (0..1920 * 1200)
        .map(|i| ((i * 37 + i / 1920 * 19) % 256) as u8)
        .collect();
    let image = GrayImage::new(1920, 1200, &data).unwrap();
    let mut times = Vec::new();
    for i in 0..21 {
        let start = Instant::now();
        let output = black_box(map.apply(black_box(image), &work).unwrap());
        let ms = start.elapsed().as_secs_f64() * 1000.;
        black_box(output.intensity());
        if i > 0 {
            times.push(ms);
        }
    }
    times.sort_by(f64::total_cmp);
    println!("{}",serde_json::to_string_pretty(&json!({
        "scope":"single release synthetic image-preparation benchmark; not scanner/live qualification",
        "image_size":[1920,1200],"measured_runs":times.len(),"warmup_runs":1,
        "map_build_ms":map_build_ms,"apply_median_ms":(times[9]+times[10])/2.,"apply_p95_ms":times[18],
        "apply_min_ms":times[0],"apply_max_ms":times[19],"apply_includes_output_allocation":true,
        "valid_pixels":map.coordinates().iter().filter(|p|p.is_some()).count(),
        "map_coordinate_storage_bytes":std::mem::size_of_val(map.coordinates())
    })).unwrap());
}
