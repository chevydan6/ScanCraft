//! Synthetic topology correctness/timing audit, not scanner or live qualification.
use scancraft_geometry::{mesh::Mesh, work::WorkControl};
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 257u32;
    let vertices = (0..width)
        .flat_map(|y| (0..width).map(move |x| [f64::from(x), f64::from(y), 0.]))
        .collect();
    let mut triangles = Vec::new();
    for y in 0..width - 1 {
        for x in 0..width - 1 {
            let a = y * width + x;
            let c = a + width;
            triangles.extend([[a, c, a + 1], [a + 1, c, c + 1]]);
        }
    }
    let mesh = Mesh {
        vertices,
        triangles,
    };
    let mut times = Vec::new();
    let mut result = None;
    for _ in 0..3 {
        let start = std::time::Instant::now();
        let t = mesh.topology(&WorkControl::default())?;
        times.push(start.elapsed().as_secs_f64() * 1000.);
        if t.boundary_edges != 4 * (width as usize - 1)
            || t.nonmanifold_vertices != 0
            || t.isolated_vertices != 0
            || t.maximum_vertex_fans != 1
            || t.closed_manifold_candidate
        {
            return Err("synthetic grid topology audit failed".into());
        }
        result = Some(t);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"scope":"synthetic indexed grid topology audit, no live or physical qualification","units":"millimeters","vertices":mesh.vertices.len(),"triangles":mesh.triangles.len(),"width":width,"elapsed_milliseconds":times,"includes_validation_and_edge_and_vertex_analysis":true,"source_modified":false,"topology":result.unwrap()})
        )?
    );
    Ok(())
}
