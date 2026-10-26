use scancraft_geometry::{
    mesh::Mesh,
    mesh_section::{Source, intersect},
    work::WorkControl,
};
fn cube() -> Mesh {
    Mesh {
        vertices: vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ],
        triangles: vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    }
}
#[test]
fn cube_midsection_is_closed_unit_square_without_changing_source() {
    let mesh = cube();
    let original = mesh.clone();
    let section = intersect(
        &mesh,
        [0., 0., 0.5],
        [0., 0., 1.],
        0.,
        &WorkControl::default(),
    )
    .unwrap();
    assert_eq!(mesh, original);
    assert_eq!(section.nodes.len(), 8);
    assert_eq!(section.segments.len(), 8);
    assert_eq!(section.contours.len(), 1);
    assert!(section.contours[0].closed);
    assert_eq!(section.contours[0].nodes.len(), 8);
    assert_eq!(section.total_segment_length_mm, 4.);
    assert_eq!(section.contours[0].length_mm, 4.);
    assert!(section.ambiguous_components.is_empty());
    assert!(section.nodes.iter().all(|p| p.position_mm[2] == 0.5));
    let scaled = intersect(
        &mesh,
        [0., 0., 0.5],
        [0., 0., f64::MAX],
        0.,
        &WorkControl::default(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(section).unwrap(),
        serde_json::to_value(scaled).unwrap()
    );
}
#[test]
fn open_section_duplicate_faces_and_coincident_unwelded_sheets() {
    let mut mesh = Mesh {
        vertices: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
    };
    let section = intersect(
        &mesh,
        [0.5, 0., 0.],
        [1., 0., 0.],
        0.,
        &WorkControl::default(),
    )
    .unwrap();
    assert_eq!(section.contours.len(), 1);
    assert!(!section.contours[0].closed);
    assert_eq!(section.contours[0].length_mm, 1.);
    assert_eq!(section.nodes.len(), 3);
    mesh.triangles.push(mesh.triangles[0]);
    let duplicate = intersect(
        &mesh,
        [0.5, 0., 0.],
        [1., 0., 0.],
        0.,
        &WorkControl::default(),
    )
    .unwrap();
    assert_eq!(duplicate.duplicate_segments, 1);
    assert_eq!(duplicate.total_segment_length_mm, 1.);
    mesh.triangles.pop();
    mesh.vertices.extend(mesh.vertices.clone());
    mesh.triangles.extend(vec![[4, 5, 6], [4, 6, 7]]);
    let doubled = intersect(
        &mesh,
        [0.5, 0., 0.],
        [1., 0., 0.],
        0.,
        &WorkControl::default(),
    )
    .unwrap();
    assert_eq!(doubled.contours.len(), 2);
    assert_eq!(doubled.nodes.len(), 6);
    assert_eq!(doubled.total_segment_length_mm, 2.);
}
#[test]
fn coplanar_faces_tangent_vertices_and_tolerance_are_explicit() {
    let mesh = cube();
    let base = intersect(&mesh, [0.; 3], [0., 0., 1.], 0., &WorkControl::default()).unwrap();
    assert_eq!(base.coplanar_triangles, 2);
    assert_eq!(base.contours[0].length_mm, 4.);
    assert!(
        base.nodes
            .iter()
            .all(|n| matches!(n.source, Source::Vertex { .. }))
    );
    let tangent = intersect(&mesh, [0.; 3], [1., 1., 1.], 0., &WorkControl::default()).unwrap();
    assert!(tangent.tangent_triangles > 0);
    assert!(tangent.segments.is_empty());
    let near = intersect(
        &mesh,
        [0., 0., 1e-6],
        [0., 0., 1.],
        1e-5,
        &WorkControl::default(),
    )
    .unwrap();
    assert_eq!(near.coplanar_triangles, 2);
    assert!(near.nodes.iter().all(|n| n.position_mm[2] == 0.));
    let miss = intersect(
        &mesh,
        [0., 0., 2.],
        [0., 0., 1.],
        0.,
        &WorkControl::default(),
    )
    .unwrap();
    assert!(miss.nodes.is_empty());
}
#[test]
fn branching_graph_is_not_forced_into_contours() {
    let mesh = Mesh {
        vertices: vec![
            [0., 0., 0.],
            [1., 1., 0.],
            [-1., 1., 0.],
            [1., 0., 1.],
            [-1., 0., 1.],
            [1., -1., 0.],
            [-1., -1., 0.],
        ],
        triangles: vec![[0, 1, 2], [0, 3, 4], [0, 5, 6]],
    };
    let section = intersect(&mesh, [0.; 3], [1., 0., 0.], 0., &WorkControl::default()).unwrap();
    assert_eq!(section.segments.len(), 3);
    assert!(section.contours.is_empty());
    assert_eq!(section.ambiguous_components.len(), 1);
    assert_eq!(section.ambiguous_components[0].branching_nodes, 1);
}
#[test]
fn invalid_planes_meshes_and_cancellation_fail() {
    let mesh = cube();
    for normal in [[0.; 3], [f64::NAN, 0., 1.]] {
        assert!(intersect(&mesh, [0.; 3], normal, 0., &WorkControl::default()).is_err());
    }
    for tolerance in [-1., f64::INFINITY, f64::NAN] {
        assert!(
            intersect(
                &mesh,
                [0.; 3],
                [0., 0., 1.],
                tolerance,
                &WorkControl::default()
            )
            .is_err()
        );
    }
    let mut invalid = mesh.clone();
    invalid.triangles[0][0] = 999;
    assert!(intersect(&invalid, [0.; 3], [0., 0., 1.], 0., &WorkControl::default()).is_err());
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert!(intersect(&mesh, [0.; 3], [0., 0., 1.], 0., &cancelled).is_err());
}
#[test]
fn sections_support_extreme_scales_and_distant_origins() {
    for scale in [1e-100, 1e150] {
        let mut mesh = cube();
        for p in &mut mesh.vertices {
            for v in p {
                *v *= scale;
            }
        }
        let section = intersect(
            &mesh,
            [0., 0., 0.5 * scale],
            [0., 0., 1.],
            0.,
            &WorkControl::default(),
        )
        .unwrap();
        assert!((section.total_segment_length_mm / scale - 4.).abs() < 1e-12);
        assert!(section.contours[0].closed);
    }
    let mut mesh = cube();
    let origin = [1e12, -2e12, 3e12];
    for p in &mut mesh.vertices {
        for a in 0..3 {
            p[a] += origin[a];
        }
    }
    let section = intersect(
        &mesh,
        [origin[0], origin[1], origin[2] + 0.5],
        [0., 0., 1.],
        0.,
        &WorkControl::default(),
    )
    .unwrap();
    assert_eq!(section.total_segment_length_mm, 4.);
}
