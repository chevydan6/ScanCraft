//! Synthetic-only reference fixture; no scanner, file input or installed calibration.
use scancraft_image::{
    GrayImage,
    corners::{Options, refine_corner},
};
fn main() {
    let mut cases = Vec::new();
    for (angle, noise) in [(0_f64, 0_f64), (0.35, 0.), (-0.65, 2.)] {
        for inverted in [false, true] {
            for truth in [[30.3, 31.7], [31.65, 29.2]] {
                let (s, c) = angle.sin_cos();
                let data: Vec<u8> = (0..4096)
                    .map(|i| {
                        let x = (i % 64) as f64 - truth[0];
                        let y = (i / 64) as f64 - truth[1];
                        let u = c * x + s * y;
                        let v = -s * x + c * y;
                        let perturbation =
                            noise * ((i as f64 * 1.73).sin() + (i as f64 * 0.317).cos()) * 0.5;
                        let value = 128. + 95. * (u / 1.2).tanh() * (v / 1.2).tanh() + perturbation;
                        (if inverted { 255. - value } else { value })
                            .round()
                            .clamp(0., 255.) as u8
                    })
                    .collect();
                let seed = [truth[0] + 0.7, truth[1] - 0.6];
                let result = refine_corner(
                    GrayImage::new(64, 64, &data).unwrap(),
                    seed,
                    Options::default(),
                    &|| false,
                )
                .unwrap();
                cases.push(serde_json::json!({"angle_radians":angle,"noise_amplitude":noise,
                    "inverted":inverted,"truth":truth,"seed":seed,"image":data,
                    "rust_pixel":result.pixel,"iterations":result.iterations,
                    "min_eigenvalue":result.min_eigenvalue,"eigenvalue_ratio":result.eigenvalue_ratio}));
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"schema_version":1,"synthetic_only":true,
        "width":64,"height":64,"radius":5,"epsilon_px":0.0001,"max_iterations":40,"cases":cases})
    );
}
