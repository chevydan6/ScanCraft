use scancraft_geometry::{
    mesh::Mesh,
    mesh_simplify::{Config, simplify},
    work::WorkControl,
};
fn config(target: usize, displacement: f64) -> Config {
    Config {
        target_triangles: target,
        maximum_displacement_mm: displacement,
        maximum_normal_change_degrees: 30.,
        preserve_crease_degrees: 30.,
        maximum_collapses: 1000,
    }
}
fn grid(n: usize) -> Mesh {
    let vertices = (0..n)
        .flat_map(|y| (0..n).map(move |x| [x as f64, y as f64, 0.]))
        .collect();
    let mut triangles = Vec::new();
    for y in 0..n - 1 {
        for x in 0..n - 1 {
            let a = (y * n + x) as u32;
            let b = a + 1;
            let c = a + n as u32;
            let d = c + 1;
            triangles.extend([[a, b, d], [a, d, c]]);
        }
    }
    Mesh {
        vertices,
        triangles,
    }
}
fn octahedron() -> Mesh {
    Mesh {
        vertices: vec![
            [1., 0., 0.],
            [-1., 0., 0.],
            [0., 1., 0.],
            [0., -1., 0.],
            [0., 0., 1.],
            [0., 0., -1.],
        ],
        triangles: vec![
            [4, 0, 2],
            [4, 2, 1],
            [4, 1, 3],
            [4, 3, 0],
            [5, 2, 0],
            [5, 1, 2],
            [5, 3, 1],
            [5, 0, 3],
        ],
    }
}
#[test]
fn planar_decimation_preserves_exact_boundary_and_cumulative_vertex_budget() {
    let source = grid(8);
    let original = source.clone();
    let result = simplify(&source, &config(70, 1.5), &WorkControl::default()).unwrap();
    assert_eq!(source, original);
    assert!(result.report.collapsed_edges > 0);
    assert!(result.report.target_reached);
    assert!(result.mesh.triangles.len() <= 70);
    assert_eq!(result.source_to_output.len(), source.vertices.len());
    assert!(result.report.maximum_source_vertex_displacement_mm <= 1.5);
    for (i, p) in source.vertices.iter().enumerate() {
        let mapped = result.mesh.vertices[result.source_to_output[i] as usize];
        assert!(scancraft_geometry::distance_mm(*p, mapped).unwrap() <= 1.5);
        if p[0] == 0. || p[1] == 0. || p[0] == 7. || p[1] == 7. {
            assert_eq!(*p, mapped);
        }
    }
    let topology = result.mesh.topology(&WorkControl::default()).unwrap();
    assert_eq!(topology.boundary_edges, 28);
    assert_eq!(topology.nonmanifold_vertices, 0);
    assert_eq!(topology.nonmanifold_edges, 0);
    assert_eq!(topology.duplicate_faces, 0);
}
#[test]
fn zero_budget_and_protected_creases_report_unreachable_targets_without_changes() {
    let source = grid(6);
    let result = simplify(&source, &config(10, 0.), &WorkControl::default()).unwrap();
    assert_eq!(result.mesh, source);
    assert!(!result.report.target_reached);
    assert_eq!(result.report.collapsed_edges, 0);
    let source = octahedron();
    let result = simplify(&source, &config(4, 10.), &WorkControl::default()).unwrap();
    assert_eq!(result.mesh, source);
    assert_eq!(result.report.locked_vertices, 6);
    assert!(!result.report.target_reached);
}
#[test]
fn closed_surface_collapse_preserves_manifold_and_stops_at_explicit_work_budget() {
    let source = octahedron();
    let mut c = config(1, 2.);
    c.preserve_crease_degrees = 180.;
    c.maximum_normal_change_degrees = 89.;
    c.maximum_collapses = 1;
    let result = simplify(&source, &c, &WorkControl::default()).unwrap();
    assert_eq!(result.report.collapsed_edges, 1);
    assert_eq!(result.report.stopped_reason, "collapse_budget");
    assert_eq!(result.mesh.triangles.len(), 6);
    assert!(
        result
            .mesh
            .topology(&WorkControl::default())
            .unwrap()
            .closed_manifold_candidate
    );
}
#[test]
fn tetrahedral_duplicate_faces_prevent_topology_destroying_collapse() {
    let source = Mesh {
        vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        triangles: vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
    };
    let mut c = config(1, 10.);
    c.preserve_crease_degrees = 180.;
    c.maximum_normal_change_degrees = 89.;
    let result = simplify(&source, &c, &WorkControl::default()).unwrap();
    assert_eq!(result.mesh, source);
    assert_eq!(result.report.collapsed_edges, 0);
    assert!(
        result
            .report
            .rejected_candidates
            .contains_key("duplicate_face")
    );
}
#[test]
fn invalid_inputs_and_cancellation_leave_source_unchanged() {
    let source = grid(4);
    let original = source.clone();
    let mut c = config(10, 1.);
    c.maximum_normal_change_degrees = 90.;
    assert!(simplify(&source, &c, &WorkControl::default()).is_err());
    c = config(10, f64::NAN);
    assert!(simplify(&source, &c, &WorkControl::default()).is_err());
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(simplify(&source, &config(10, 1.), &cancelled).is_err());
    assert_eq!(source, original);
    let mut invalid = source.clone();
    invalid.triangles.push(invalid.triangles[0]);
    assert!(simplify(&invalid, &config(10, 1.), &WorkControl::default()).is_err());
}
fn subdivide(source: Mesh) -> Mesh {
    let mut vertices = source.vertices;
    let mut mids = std::collections::BTreeMap::new();
    let mut triangles = Vec::new();
    for [a, b, c] in source.triangles {
        let mut midpoint = |u: u32, v: u32| {
            let key = if u < v { [u, v] } else { [v, u] };
            *mids.entry(key).or_insert_with(|| {
                let p: [f64; 3] = std::array::from_fn(|i| {
                    (vertices[u as usize][i] + vertices[v as usize][i]) / 2.
                });
                let length = p[0].hypot(p[1]).hypot(p[2]);
                let index = vertices.len() as u32;
                vertices.push(p.map(|x| x / length));
                index
            })
        };
        let ab = midpoint(a, b);
        let bc = midpoint(b, c);
        let ca = midpoint(c, a);
        triangles.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
    }
    Mesh {
        vertices,
        triangles,
    }
}
#[test]
fn curved_closed_surface_retains_source_face_normal_bounds_after_multiple_collapses() {
    let source = subdivide(octahedron());
    let mut c = config(20, 0.6);
    c.preserve_crease_degrees = 100.;
    c.maximum_normal_change_degrees = 30.;
    let result = simplify(&source, &c, &WorkControl::default()).unwrap();
    assert!(result.report.collapsed_edges > 1);
    assert!(
        result
            .mesh
            .topology(&WorkControl::default())
            .unwrap()
            .closed_manifold_candidate
    );
    for t in &source.triangles {
        let mapped = t.map(|v| result.source_to_output[v as usize]);
        if mapped[0] == mapped[1] || mapped[1] == mapped[2] || mapped[2] == mapped[0] {
            continue;
        }
        let old = t.map(|v| source.vertices[v as usize]);
        let new = mapped.map(|v| result.mesh.vertices[v as usize]);
        let a = scancraft_geometry::mesh::triangle_normal(old[0], old[1], old[2]).unwrap();
        let b = scancraft_geometry::mesh::triangle_normal(new[0], new[1], new[2]).unwrap();
        assert!((0..3).map(|i| a[i] * b[i]).sum::<f64>() + 1e-12 >= 30_f64.to_radians().cos());
    }
}
#[test]
fn inner_opening_and_unused_source_vertex_are_preserved() {
    let mut source = grid(8);
    source.triangles.retain(|t| {
        !t.iter().all(|&v| {
            let p = source.vertices[v as usize];
            (3. ..=5.).contains(&p[0]) && (3. ..=5.).contains(&p[1])
        })
    });
    let before =
        scancraft_geometry::mesh_boundaries::analyze(&source, &WorkControl::default()).unwrap();
    assert_eq!(before.loops.len(), 2);
    let result = simplify(&source, &config(76, 1.5), &WorkControl::default()).unwrap();
    assert!(result.report.collapsed_edges > 0);
    let after = scancraft_geometry::mesh_boundaries::analyze(&result.mesh, &WorkControl::default())
        .unwrap();
    assert_eq!(after.loops.len(), 2);
    assert_eq!(
        after.topology.boundary_edges,
        before.topology.boundary_edges
    );
    assert_eq!(after.topology.isolated_vertices, 1);
    assert_eq!(
        result.mesh.vertices[result.source_to_output[4 * 8 + 4] as usize],
        [4., 4., 0.]
    );
}
#[test]
fn coincident_indexed_components_are_not_welded() {
    let mut source = octahedron();
    let other = octahedron();
    source.vertices.extend(other.vertices);
    source
        .triangles
        .extend(other.triangles.into_iter().map(|t| t.map(|v| v + 6)));
    let mut c = config(8, 2.);
    c.preserve_crease_degrees = 180.;
    c.maximum_normal_change_degrees = 89.;
    let result = simplify(&source, &c, &WorkControl::default()).unwrap();
    assert!(result.report.collapsed_edges > 0);
    assert_eq!(
        scancraft_geometry::mesh_components::analyze(&result.mesh, &WorkControl::default())
            .unwrap()
            .components
            .len(),
        2
    );
    let first: std::collections::BTreeSet<_> =
        result.source_to_output[..6].iter().copied().collect();
    assert!(
        result.source_to_output[6..]
            .iter()
            .all(|v| !first.contains(v))
    );
}
#[test]
fn cancellation_after_processing_begins_does_not_publish_partial_geometry() {
    let source = grid(100);
    let original = source.clone();
    let work = WorkControl::default();
    let mut c = config(1000, 3.);
    c.maximum_collapses = 10000;
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| simplify(&source, &c, &work));
        let start = std::time::Instant::now();
        loop {
            let (stage, progress) = work.progress();
            if stage == "Processing" && progress > 0. {
                break;
            }
            assert!(
                !worker.is_finished(),
                "fixture completed before cancellation observation"
            );
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        work.cancel();
        assert!(worker.join().unwrap().is_err());
    });
    assert_eq!(source, original);
}
