//! Indexed boundary contours, not automatic hole classification or repair.
use crate::{
    mesh::{Mesh, Topology},
    work::{Stage, WorkControl},
};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

pub const MAX_BOUNDARY_EDGES: usize = 1_000_000;
pub const MAX_BOUNDARY_COMPONENTS: usize = 100_000;
#[derive(Debug, Serialize)]
pub struct BoundaryLoop {
    /// Deterministic contour ID for this exact indexed mesh; invalid after edits.
    pub id: usize,
    /// Starts at the lowest vertex ID, follows existing face-edge orientation.
    /// Closing edge from last back to first is implicit.
    pub vertices: Vec<u32>,
    pub perimeter_mm: f64,
    pub minimum_mm: [f64; 3],
    pub maximum_mm: [f64; 3],
}
#[derive(Debug, Serialize)]
pub struct AmbiguousBoundary {
    pub first_vertex: u32,
    pub vertices: usize,
    pub edges: usize,
    pub vertices_with_degree_not_two: usize,
    pub direction_conflicts: usize,
}
#[derive(Debug, Serialize)]
pub struct Boundaries {
    pub topology: Topology,
    pub boundary_edges: usize,
    pub loops: Vec<BoundaryLoop>,
    pub ambiguous_components: Vec<AmbiguousBoundary>,
}
/// Only edges incident to exactly one face enter the boundary graph. Vertex
/// positions are never welded. Connected graphs with branching or inconsistent
/// directions are reported separately, never forced into contours. A simple
/// indexed cycle may be an intentional opening, outer border or spatially
/// self-intersecting contour; it is not proof of a fillable hole.
pub fn analyze(mesh: &Mesh, work: &WorkControl) -> Result<Boundaries, String> {
    let topology = mesh.topology(work)?;
    let mut incidence: BTreeMap<[u32; 2], (usize, [u32; 2])> = BTreeMap::new();
    for (i, t) in mesh.triangles.iter().enumerate() {
        work.update(Stage::Indexing, i, mesh.triangles.len())?;
        for edge in [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]] {
            let mut key = edge;
            key.sort_unstable();
            let value = incidence.entry(key).or_insert((0, edge));
            value.0 += 1;
        }
    }
    let mut edges = Vec::new();
    for (_, (count, edge)) in incidence {
        work.check()?;
        if count == 1 {
            if edges.len() == MAX_BOUNDARY_EDGES {
                return Err("mesh boundary edge budget exceeded".into());
            }
            edges.push(edge);
        }
    }
    let mut adjacency: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (i, edge) in edges.iter().enumerate() {
        work.check()?;
        for v in edge {
            adjacency.entry(*v).or_default().push(i);
        }
    }
    let mut visited = vec![false; edges.len()];
    let mut seen = HashSet::new();
    let mut loops = Vec::new();
    let mut ambiguous_components = Vec::new();
    for first in 0..edges.len() {
        work.check()?;
        if visited[first] {
            continue;
        }
        if loops.len() + ambiguous_components.len() == MAX_BOUNDARY_COMPONENTS {
            return Err("mesh boundary component budget exceeded".into());
        }
        let start = edges[first][0];
        seen.insert(start);
        let mut stack = vec![start];
        let mut vertices = Vec::new();
        let mut component_edges = 0;
        while let Some(v) = stack.pop() {
            work.update(Stage::Processing, seen.len(), adjacency.len())?;
            vertices.push(v);
            for &index in &adjacency[&v] {
                if !visited[index] {
                    visited[index] = true;
                    component_edges += 1;
                }
                let edge = edges[index];
                let other = if edge[0] == v { edge[1] } else { edge[0] };
                if seen.insert(other) {
                    stack.push(other);
                }
            }
        }
        vertices.sort_unstable();
        let degree_conflicts = vertices.iter().filter(|v| adjacency[v].len() != 2).count();
        let direction_conflicts = vertices
            .iter()
            .filter(|&&v| {
                adjacency[&v].iter().filter(|&&i| edges[i][0] == v).count() != 1
                    || adjacency[&v].iter().filter(|&&i| edges[i][1] == v).count() != 1
            })
            .count();
        if degree_conflicts != 0 || direction_conflicts != 0 {
            ambiguous_components.push(AmbiguousBoundary {
                first_vertex: vertices[0],
                vertices: vertices.len(),
                edges: component_edges,
                vertices_with_degree_not_two: degree_conflicts,
                direction_conflicts,
            });
            continue;
        }
        let start = vertices[0];
        let mut current = start;
        let mut contour = Vec::with_capacity(vertices.len());
        let mut minimum_mm = mesh.vertices[start as usize];
        let mut maximum_mm = minimum_mm;
        let mut perimeter_mm = 0f64;
        loop {
            work.check()?;
            contour.push(current);
            let outgoing = *adjacency[&current]
                .iter()
                .find(|&&i| edges[i][0] == current)
                .unwrap();
            let next = edges[outgoing][1];
            let a = mesh.vertices[current as usize];
            let b = mesh.vertices[next as usize];
            let distance = (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]);
            perimeter_mm += distance;
            if !perimeter_mm.is_finite() {
                return Err("boundary perimeter overflow".into());
            }
            for axis in 0..3 {
                minimum_mm[axis] = minimum_mm[axis].min(a[axis]);
                maximum_mm[axis] = maximum_mm[axis].max(a[axis]);
            }
            current = next;
            if current == start {
                break;
            }
            if contour.len() >= component_edges {
                return Err("boundary traversal did not close".into());
            }
        }
        if contour.len() != component_edges {
            return Err("boundary traversal omitted indexed edges".into());
        }
        loops.push(BoundaryLoop {
            id: loops.len(),
            vertices: contour,
            perimeter_mm,
            minimum_mm,
            maximum_mm,
        });
    }
    work.check()?;
    Ok(Boundaries {
        topology,
        boundary_edges: edges.len(),
        loops,
        ambiguous_components,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ring() -> Mesh {
        let vertices = vec![
            [0., 0., 0.],
            [10., 0., 0.],
            [10., 10., 0.],
            [0., 10., 0.],
            [3., 3., 0.],
            [7., 3., 0.],
            [7., 7., 0.],
            [3., 7., 0.],
        ];
        let mut triangles = Vec::new();
        for i in 0..4u32 {
            let j = (i + 1) % 4;
            triangles.push([i, j, 4 + i]);
            triangles.push([j, 4 + j, 4 + i]);
        }
        Mesh {
            vertices,
            triangles,
        }
    }
    #[test]
    fn large_vertex_branch_stays_ambiguous_and_perimeter_overflow_fails() {
        let mut mesh = Mesh {
            vertices: vec![[0.; 3]],
            triangles: Vec::new(),
        };
        for i in 0..2000 {
            let angle = (i as f64) * std::f64::consts::TAU / 2000.;
            let a = mesh.vertices.len() as u32;
            mesh.vertices.push([angle.cos(), angle.sin(), 0.]);
            mesh.vertices
                .push([(angle + 0.0005).cos(), (angle + 0.0005).sin(), 0.]);
            mesh.triangles.push([0, a, a + 1]);
        }
        let analysis = analyze(&mesh, &WorkControl::default()).unwrap();
        assert!(analysis.loops.is_empty());
        assert_eq!(analysis.ambiguous_components.len(), 1);
        assert_eq!(analysis.boundary_edges, 6000);
        assert_eq!(
            analysis.ambiguous_components[0].vertices_with_degree_not_two,
            1
        );
        let huge = Mesh {
            vertices: vec![[0., 0., 0.], [1e308, 0., 0.], [0., 1e308, 0.]],
            triangles: vec![[0, 1, 2]],
        };
        assert!(
            analyze(&huge, &WorkControl::default())
                .unwrap_err()
                .contains("perimeter overflow")
        );
    }
    #[test]
    fn distinguishes_outer_and_inner_contours_without_calling_either_a_hole() {
        let mesh = ring();
        let original = mesh.clone();
        let a = analyze(&mesh, &WorkControl::default()).unwrap();
        assert_eq!(a.boundary_edges, 8);
        assert_eq!(a.loops.len(), 2);
        assert!(a.ambiguous_components.is_empty());
        assert_eq!(a.loops[0].vertices, vec![0, 1, 2, 3]);
        assert_eq!(a.loops[1].vertices, vec![4, 7, 6, 5]);
        assert_eq!(a.loops[0].perimeter_mm, 40.);
        assert_eq!(a.loops[1].perimeter_mm, 16.);
        assert_eq!(a.loops[1].minimum_mm, [3., 3., 0.]);
        assert_eq!(mesh, original);
        let mut reordered = mesh.clone();
        reordered.triangles.reverse();
        assert_eq!(
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(analyze(&reordered, &WorkControl::default()).unwrap()).unwrap()
        );
    }
    #[test]
    fn closed_mesh_has_no_boundary_and_removed_face_has_oriented_contour() {
        let mut mesh = Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
        };
        assert_eq!(
            analyze(&mesh, &WorkControl::default()).unwrap().loops.len(),
            0
        );
        mesh.triangles.pop();
        let a = analyze(&mesh, &WorkControl::default()).unwrap();
        assert_eq!(a.boundary_edges, 3);
        assert_eq!(a.loops[0].vertices, vec![0, 2, 3]);
    }
    #[test]
    fn touching_contours_and_bad_winding_are_ambiguous_not_forced_into_loops() {
        let mesh = Mesh {
            vertices: vec![
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 1., 0.],
                [-1., 0., 0.],
                [0., -1., 0.],
            ],
            triangles: vec![[0, 1, 2], [0, 3, 4]],
        };
        let a = analyze(&mesh, &WorkControl::default()).unwrap();
        assert!(a.loops.is_empty());
        assert_eq!(a.ambiguous_components.len(), 1);
        assert_eq!(a.ambiguous_components[0].vertices_with_degree_not_two, 1);
        let mut mesh = ring();
        mesh.triangles[0].swap(0, 1);
        let a = analyze(&mesh, &WorkControl::default()).unwrap();
        assert!(a.topology.inconsistent_winding_edges > 0);
        assert!(!a.ambiguous_components.is_empty());
        let work = WorkControl::default();
        work.cancel();
        assert!(analyze(&mesh, &work).is_err());
        mesh.vertices[0][0] = f64::NAN;
        assert!(analyze(&mesh, &WorkControl::default()).is_err());
    }
}
