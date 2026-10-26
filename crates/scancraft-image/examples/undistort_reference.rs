//! Synthetic fixture only; coefficients do not describe the Raptor.
use scancraft_calibration::{Brown5, CameraModel};
use scancraft_geometry::work::WorkControl;
use scancraft_image::{GrayImage, UndistortMap};
use serde_json::json;
fn main() {
    let source = CameraModel {
        width: 64,
        height: 48,
        fx: 49.1,
        fy: 48.7,
        cx: 31.7,
        cy: 23.2,
        skew: 0.,
        distortion: Brown5 {
            k1: -0.12,
            k2: 0.03,
            k3: 0.002,
            p1: 0.001,
            p2: -0.0007,
        },
        valid_radius: 1.2,
    };
    let mut output = source.clone();
    output.distortion = Brown5::ZERO;
    output.fx = 47.3;
    output.fy = 46.9;
    let a = 0.04f64;
    let rotation = [
        [a.cos(), 0., a.sin()],
        [0., 1., 0.],
        [-a.sin(), 0., a.cos()],
    ];
    let pixels: Vec<u8> = (0..source.height)
        .flat_map(|y| (0..source.width).map(move |x| (2 * x + 2 * y) as u8))
        .collect();
    let map = UndistortMap::build(&source, &output, rotation, &WorkControl::default()).unwrap();
    let image = map
        .apply(
            GrayImage::new(source.width, source.height, &pixels).unwrap(),
            &WorkControl::default(),
        )
        .unwrap();
    println!(
        "{}",
        serde_json::to_string(
            &json!({"provenance":"synthetic_fixture_not_scanner_calibration",
        "source":source,"output":output,"source_from_output":rotation,"source_pixels":pixels,
        "coordinates":map.coordinates(),"intensity":image.intensity(),"valid":image.valid()})
        )
        .unwrap()
    );
}
