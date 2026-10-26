use scancraft_geometry::{
    mesh::Mesh,
    mesh_smooth::{Config, smooth},
    work::WorkControl,
};
fn bump() -> Mesh {
    let mut vertices = Vec::new();
    for y in 0..5 {
        for x in 0..5 {
            vertices.push([x as f64, y as f64, if x == 2 && y == 2 { 0.3 } else { 0. }]);
        }
    }
    let mut triangles = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            let a = y * 5 + x;
            triangles.extend([[a, a + 1, a + 6], [a, a + 6, a + 5]]);
        }
    }
    Mesh {
        vertices,
        triangles,
    }
}
fn config() -> Config {
    Config {
        iterations: 5,
        relaxation: 0.5,
        maximum_displacement_mm: 0.1,
        maximum_normal_change_degrees: 30.,
        preserve_crease_degrees: 180.,
    }
}
#[test]
fn reduces_bump_with_exact_boundary_topology_and_source_limits() {
    let source = bump();
    let original = source.clone();
    let c = config();
    let work = WorkControl::default();
    let (result, report) = smooth(&source, &c, &work).unwrap();
    assert_eq!(source, original);
    assert_eq!(result.triangles, source.triangles);
    assert!(result.vertices[12][2] < source.vertices[12][2]);
    assert!(report.completed_iterations > 0);
    assert!(report.backtracking_steps > 0);
    for i in 0..25 {
        if i % 5 == 0 || i % 5 == 4 || !(5..20).contains(&i) {
            assert_eq!(result.vertices[i], source.vertices[i]);
        }
        assert!(
            scancraft_geometry::distance_mm(result.vertices[i], source.vertices[i]).unwrap()
                <= c.maximum_displacement_mm
        );
    }
    for t in &source.triangles {
        let normal = |m: &Mesh| {
            scancraft_geometry::mesh::triangle_normal(
                m.vertices[t[0] as usize],
                m.vertices[t[1] as usize],
                m.vertices[t[2] as usize],
            )
            .unwrap()
        };
        let a = normal(&source);
        let b = normal(&result);
        assert!(
            (0..3).map(|i| a[i] * b[i]).sum::<f64>() + 1e-12
                >= c.maximum_normal_change_degrees.to_radians().cos()
        );
    }
    assert_eq!(report.locked_vertices, 16);
    assert_eq!(result.topology(&work).unwrap().boundary_edges, 16);
}
#[test]
fn zero_budget_and_crease_lock_preserve_source() {
    let source = bump();
    let work = WorkControl::default();
    let mut c = config();
    c.maximum_displacement_mm = 0.;
    let (result, r) = smooth(&source, &c, &work).unwrap();
    assert_eq!(result, source);
    assert_eq!(r.stopped_reason, "constraints");
    c.maximum_displacement_mm = 1.;
    c.preserve_crease_degrees = 0.;
    let (result, r) = smooth(&source, &c, &work).unwrap();
    assert_eq!(result.vertices[12], source.vertices[12]);
    assert!(r.locked_vertices > 16);
}
#[test]
fn stationary_unused_vertices_and_invalid_inputs() {
    let mut source = bump();
    for p in &mut source.vertices {
        p[2] = 0.;
    }
    source.vertices.push([100., 200., 300.]);
    let work = WorkControl::default();
    let (result, r) = smooth(&source, &config(), &work).unwrap();
    assert_eq!(result, source);
    assert_eq!(r.stopped_reason, "stationary");
    let mut c = config();
    c.relaxation = f64::NAN;
    assert!(smooth(&source, &c, &work).is_err());
    c = config();
    c.iterations = 101;
    assert!(smooth(&source, &c, &work).is_err());
    source.triangles.push(source.triangles[0]);
    assert!(smooth(&source, &config(), &work).is_err());
}
#[test]
fn cancelled_input_is_immutable() {
    let source = bump();
    let original = source.clone();
    let work = WorkControl::default();
    work.cancel();
    assert!(smooth(&source, &config(), &work).is_err());
    assert_eq!(source, original);
}

#[test]
fn jacobi_one_step_matches_analytic_neighbor_average_and_translates() {
    let source = bump();
    let work = WorkControl::default();
    let mut c = config();
    c.iterations = 1;
    c.maximum_displacement_mm = 1.;
    let (result, r) = smooth(&source, &c, &work).unwrap();
    assert_eq!(r.backtracking_steps, 0);
    assert_eq!(r.completed_iterations, 1);
    assert!((result.vertices[12][2] - 0.15).abs() < 1e-14);
    assert!((result.vertices[6][2] - 0.025).abs() < 1e-14);
    let mut translated = source.clone();
    for p in &mut translated.vertices {
        for x in p {
            *x += 1e6;
        }
    }
    let (moved, _) = smooth(&translated, &c, &work).unwrap();
    for (a, b) in result.vertices.iter().zip(moved.vertices) {
        for axis in 0..3 {
            assert!((a[axis] - (b[axis] - 1e6)).abs() < 1e-9);
        }
    }
}
