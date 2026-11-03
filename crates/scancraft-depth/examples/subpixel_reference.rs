//! Reproducible fractional-image fixture for an independent numerical oracle.
#[path = "../tests/support/subpixel_scene.rs"]
mod scene;
use scancraft_depth::matching::{MatchConfig, match_rectified};
use scancraft_geometry::work::WorkControl;
use scancraft_image::MaskedImage;
use serde_json::json;
fn main() {
    let mut cases = Vec::new();
    for disparity in [4.2, 4.25, 4.3, -4.25] {
        let (rig, left, right) = scene::fixture(disparity);
        let l = MaskedImage::new(96, 48, left.clone(), vec![true; 96 * 48]).unwrap();
        let r = MaskedImage::new(96, 48, right.clone(), vec![true; 96 * 48]).unwrap();
        let config = scene::config(disparity);
        let seeds = [[32, 18], [48, 24], [64, 30]];
        let coarse = match_rectified(
            &rig,
            &l,
            &r,
            &seeds,
            MatchConfig {
                subpixel: None,
                ..config
            },
            &WorkControl::default(),
        )
        .unwrap();
        let refined =
            match_rectified(&rig, &l, &r, &seeds, config, &WorkControl::default()).unwrap();
        cases.push(json!({"disparity":disparity,"rig":rig,"left":left,"right":right,"seeds":seeds,"coarse":coarse,"refined":refined}));
    }
    println!(
        "{}",
        serde_json::to_string(&json!({"synthetic_only":true,"cases":cases})).unwrap()
    );
}
