use scancraft_geometry::{
    Cloud, Point,
    selection::{OrthographicProjection, ScreenShape, Visibility, select_screen},
    work::WorkControl,
};
use std::time::Instant;
fn main() {
    let cloud = Cloud {
        points: (0..1000000)
            .map(|i| Point {
                position: [
                    ((i % 1000) as f64 + 0.5 - 500.) / 375.,
                    (500. - ((i / 1000) as f64 + 0.5)) / 375.,
                    0.,
                ],
                color: None,
                normal: None,
            })
            .collect(),
    };
    let projection = OrthographicProjection::new(
        ([-1.; 3], [1.; 3]),
        [0.; 2],
        1.,
        [0., 0., 1000., 1000.],
        [0.; 2],
    )
    .unwrap();
    let shape = ScreenShape::Rectangle {
        min: [250., 250.],
        max: [750., 750.],
    };
    let start = Instant::now();
    let indices = select_screen(
        &cloud,
        &projection,
        &shape,
        Visibility::Through,
        1e-9,
        &WorkControl::default(),
    )
    .unwrap();
    let elapsed = start.elapsed().as_secs_f64() * 1000.;
    let mut mask = vec![false; 1000000];
    for i in &indices {
        assert!(!mask[*i]);
        mask[*i] = true;
    }
    let mut fp = 0;
    let mut fn_ = 0;
    for (i, selected) in mask.iter().enumerate() {
        let expected = (250..750).contains(&(i % 1000)) && (250..750).contains(&(i / 1000));
        if *selected && !expected {
            fp += 1;
        } else if !selected && expected {
            fn_ += 1;
        }
    }
    let accuracy = 1. - (fp + fn_) as f64 / 1000000.;
    println!(
        "{{\"dataset\":\"analytic million-point rectangle\",\"points\":1000000,\"selected\":{},\"false_positive\":{fp},\"false_negative\":{fn_},\"accuracy\":{accuracy},\"selection_ms\":{elapsed},\"build\":\"release\",\"scope\":\"single orthographic synthetic case, not full selection qualification\"}}",
        indices.len()
    );
    assert_eq!((fp, fn_), (0, 0));
}
