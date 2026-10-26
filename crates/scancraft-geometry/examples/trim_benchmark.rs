use scancraft_geometry::{Cloud, Editor, Point};
use std::time::Instant;
fn main() {
    let cloud = Cloud {
        points: (0..1_000_000)
            .map(|i| Point {
                position: [(i % 1000) as f64, (i / 1000) as f64, 0.],
                color: None,
                normal: None,
            })
            .collect(),
    };
    let indices: Vec<_> = (0..1_000_000).step_by(2).collect();
    let mut editor = Editor::new(cloud, 1).unwrap();
    let start = Instant::now();
    editor.trim(&indices, false).unwrap();
    let ms = start.elapsed().as_secs_f64() * 1000.;
    assert_eq!(editor.cloud.points.len(), 500_000);
    assert!(editor.undo());
    assert_eq!(editor.cloud.points.len(), 1_000_000);
    println!(
        "{{\"points\":1000000,\"selected\":500000,\"trim_ms\":{ms},\"undo_restored_count\":1000000,\"build\":\"release\",\"threshold_500ms_pass\":{}}}",
        ms <= 500.
    );
}
