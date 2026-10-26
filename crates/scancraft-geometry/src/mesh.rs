//! Native indexed triangle geometry in millimeters, with edge and vertex topology.
use crate::work::{Stage, WorkControl};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{self, Write},
};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mesh {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Topology {
    pub boundary_edges: usize,
    pub nonmanifold_edges: usize,
    pub inconsistent_winding_edges: usize,
    pub duplicate_faces: usize,
    pub isolated_vertices: usize,
    pub nonmanifold_vertices: usize,
    pub nonmanifold_vertex_ids: Vec<u32>,
    pub vertices_with_disconnected_fans: usize,
    pub vertices_with_invalid_boundary_degree: usize,
    pub maximum_vertex_fans: u32,
    /// Legacy edge-closure/winding/duplicate/usage check; no vertex fans or intersections.
    pub edge_closed: bool,
    /// Indexed edge/vertex conditions only. No self-intersection, shell nesting
    /// or geometric coincidence test; not a watertightness qualification.
    pub closed_manifold_candidate: bool,
}
pub fn triangle_normal(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Result<[f64; 3], String> {
    let u: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
    let v: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
    let scale = u.iter().chain(&v).map(|x| x.abs()).fold(0., f64::max);
    if !scale.is_finite() || scale == 0. {
        return Err("degenerate or overflowing triangle".into());
    }
    let u = u.map(|x| x / scale);
    let v = v.map(|x| x / scale);
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = n[0].hypot(n[1]).hypot(n[2]);
    if !length.is_finite() || length <= 1e-12 {
        return Err("degenerate or ill-conditioned triangle".into());
    }
    Ok(n.map(|x| x / length))
}
impl Mesh {
    pub fn validate(&self, work: &WorkControl) -> Result<(), String> {
        if self.vertices.len() > 1_000_000 || self.triangles.len() > 2_000_000 {
            return Err("mesh exceeds vertex/triangle budget".into());
        }
        for (i, p) in self.vertices.iter().enumerate() {
            if i % 1024 == 0 {
                work.check()?;
            }
            if !p.iter().all(|x| x.is_finite()) {
                return Err("mesh contains nonfinite vertex".into());
            }
        }
        for (i, t) in self.triangles.iter().enumerate() {
            work.update(Stage::Validating, i, self.triangles.len())?;
            if t.iter().any(|j| *j as usize >= self.vertices.len())
                || t[0] == t[1]
                || t[1] == t[2]
                || t[0] == t[2]
            {
                return Err("invalid mesh triangle indices".into());
            }
            triangle_normal(
                self.vertices[t[0] as usize],
                self.vertices[t[1] as usize],
                self.vertices[t[2] as usize],
            )?;
        }
        work.check()
    }
    pub fn topology(&self, work: &WorkControl) -> Result<Topology, String> {
        self.validate(work)?;
        let mut edges: BTreeMap<[u32; 2], (usize, i64, [u32; 2])> = BTreeMap::new();
        let mut links = crate::mesh_vertex_links::CornerLinks::new(self.triangles.len());
        let mut faces = HashSet::new();
        let mut duplicate_faces = 0;
        let mut used = vec![false; self.vertices.len()];
        for (i, t) in self.triangles.iter().enumerate() {
            work.update(Stage::Processing, i, self.triangles.len())?;
            let mut sorted = *t;
            sorted.sort_unstable();
            if !faces.insert(sorted) {
                duplicate_faces += 1;
            }
            for j in t {
                used[*j as usize] = true;
            }
            for j in 0..3 {
                let a = t[j];
                let b = t[(j + 1) % 3];
                let corners = [(i * 3 + j) as u32, (i * 3 + (j + 1) % 3) as u32];
                let (key, sign, corners) = if a < b {
                    ([a, b], 1, corners)
                } else {
                    ([b, a], -1, [corners[1], corners[0]])
                };
                let value = edges.entry(key).or_insert((0, 0, corners));
                if value.0 > 0 {
                    links.join(value.2[0], corners[0]);
                    links.join(value.2[1], corners[1]);
                }
                value.0 += 1;
                value.1 += sign;
            }
        }
        let boundary_edges = edges.values().filter(|e| e.0 == 1).count();
        let nonmanifold_edges = edges.values().filter(|e| e.0 > 2).count();
        let inconsistent_winding_edges = edges.values().filter(|e| e.0 == 2 && e.1 != 0).count();
        let fans = links.fan_counts(self, work)?;
        let mut boundary_degree = vec![0u32; self.vertices.len()];
        let mut bad_edge = vec![false; self.vertices.len()];
        for (edge, (count, _, _)) in &edges {
            work.check()?;
            for &v in edge {
                if *count == 1 {
                    boundary_degree[v as usize] += 1;
                }
                if *count > 2 {
                    bad_edge[v as usize] = true;
                }
            }
        }
        let mut nonmanifold_vertex_ids = Vec::new();
        let mut vertices_with_disconnected_fans = 0;
        let mut vertices_with_invalid_boundary_degree = 0;
        for i in 0..self.vertices.len() {
            if i % 1024 == 0 {
                work.check()?;
            }
            let disconnected = fans[i] > 1;
            let bad_boundary = boundary_degree[i] != 0 && boundary_degree[i] != 2;
            vertices_with_disconnected_fans += usize::from(disconnected);
            vertices_with_invalid_boundary_degree += usize::from(bad_boundary);
            if used[i] && (disconnected || bad_boundary || bad_edge[i]) {
                nonmanifold_vertex_ids.push(i as u32);
            }
        }
        let edge_closed = !self.triangles.is_empty()
            && boundary_edges == 0
            && nonmanifold_edges == 0
            && inconsistent_winding_edges == 0
            && duplicate_faces == 0
            && used.iter().all(|x| *x);
        work.check()?;
        Ok(Topology {
            boundary_edges,
            nonmanifold_edges,
            inconsistent_winding_edges,
            duplicate_faces,
            isolated_vertices: used.iter().filter(|x| !**x).count(),
            closed_manifold_candidate: edge_closed && nonmanifold_vertex_ids.is_empty(),
            nonmanifold_vertices: nonmanifold_vertex_ids.len(),
            nonmanifold_vertex_ids,
            vertices_with_disconnected_fans,
            vertices_with_invalid_boundary_degree,
            maximum_vertex_fans: fans.into_iter().max().unwrap_or(0),
            edge_closed,
        })
    }
    pub fn write_stl(&self, mut output: impl Write, work: &WorkControl) -> io::Result<()> {
        self.validate(work).map_err(io::Error::other)?;
        writeln!(output, "solid open_scancraft_mm")?;
        for t in &self.triangles {
            work.check().map_err(io::Error::other)?;
            let [a, b, c] = t.map(|i| self.vertices[i as usize]);
            let n = triangle_normal(a, b, c).map_err(io::Error::other)?;
            writeln!(
                output,
                "facet normal {} {} {}\nouter loop",
                n[0], n[1], n[2]
            )?;
            for p in [a, b, c] {
                writeln!(output, "vertex {} {} {}", p[0], p[1], p[2])?;
            }
            writeln!(output, "endloop\nendfacet")?;
        }
        work.check().map_err(io::Error::other)?;
        writeln!(output, "endsolid open_scancraft_mm")
    }
    pub fn write_obj(&self, mut output: impl Write, work: &WorkControl) -> io::Result<()> {
        self.validate(work).map_err(io::Error::other)?;
        writeln!(output, "# units millimeters")?;
        for p in &self.vertices {
            work.check().map_err(io::Error::other)?;
            writeln!(output, "v {} {} {}", p[0], p[1], p[2])?;
        }
        for t in &self.triangles {
            work.check().map_err(io::Error::other)?;
            writeln!(output, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1)?;
        }
        work.check().map_err(io::Error::other)
    }
}
#[derive(Debug)]
pub struct GridMesh {
    pub mesh: Mesh,
    pub incomplete_cells: usize,
    pub rejected_triangles: usize,
}
/// Row-major sample grid. A cell needs all four samples; missing data stays open.
/// Winding follows [top-left,bottom-left,top-right] (toward -Z on an XY plane).
pub fn from_grid(
    width: u32,
    height: u32,
    samples: &[Option<[f64; 3]>],
    maximum_edge_mm: f64,
    work: &WorkControl,
) -> Result<GridMesh, String> {
    work.check()?;
    let count = u64::from(width) * u64::from(height);
    if width < 2
        || height < 2
        || count > 1_000_000
        || count as usize != samples.len()
        || !maximum_edge_mm.is_finite()
        || maximum_edge_mm <= 0.
    {
        return Err("invalid organized grid dimensions or edge limit".into());
    }
    let mut vertices = Vec::new();
    let mut map = Vec::with_capacity(samples.len());
    for p in samples {
        work.check()?;
        map.push(if let Some(p) = p {
            if !p.iter().all(|v| v.is_finite()) {
                return Err("nonfinite organized sample".into());
            }
            let index = vertices.len() as u32;
            vertices.push(*p);
            Some(index)
        } else {
            None
        });
    }
    let distance = |a: usize, b: usize| {
        let a = samples[a].unwrap();
        let b = samples[b].unwrap();
        (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
    };
    let mut triangles = Vec::new();
    let mut incomplete_cells = 0;
    let mut rejected_triangles = 0;
    for y in 0..height - 1 {
        work.update(Stage::Processing, y as usize, height as usize - 1)?;
        for x in 0..width - 1 {
            if x.is_multiple_of(256) {
                work.check()?;
            }
            let a = (y * width + x) as usize;
            let b = a + 1;
            let c = a + width as usize;
            let d = c + 1;
            if [a, b, c, d].iter().any(|i| map[*i].is_none()) {
                incomplete_cells += 1;
                continue;
            }
            let candidates = if distance(a, d) < distance(b, c) {
                [[a, c, d], [a, d, b]]
            } else {
                [[a, c, b], [b, c, d]]
            };
            for t in candidates {
                if [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]
                    .iter()
                    .any(|(a, b)| distance(*a, *b) > maximum_edge_mm)
                    || triangle_normal(
                        samples[t[0]].unwrap(),
                        samples[t[1]].unwrap(),
                        samples[t[2]].unwrap(),
                    )
                    .is_err()
                {
                    rejected_triangles += 1;
                    continue;
                }
                triangles.push(t.map(|i| map[i].unwrap()));
            }
        }
    }
    let mesh = Mesh {
        vertices,
        triangles,
    };
    mesh.validate(work)?;
    Ok(GridMesh {
        mesh,
        incomplete_cells,
        rejected_triangles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tetra() -> Mesh {
        Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
        }
    }
    #[test]
    fn grid_preserves_holes_rejects_discontinuities_and_has_expected_orientation() {
        let work = WorkControl::default();
        let samples = [
            Some([0., 0., 10.]),
            Some([1., 0., 10.]),
            Some([0., 1., 10.]),
            Some([1., 1., 10.]),
        ];
        let result = from_grid(2, 2, &samples, 2., &work).unwrap();
        assert_eq!(result.mesh.triangles.len(), 2);
        let topology = result.mesh.topology(&work).unwrap();
        assert_eq!(topology.boundary_edges, 4);
        assert_eq!(topology.inconsistent_winding_edges, 0);
        assert!(!topology.edge_closed);
        for t in &result.mesh.triangles {
            assert_eq!(
                triangle_normal(
                    result.mesh.vertices[t[0] as usize],
                    result.mesh.vertices[t[1] as usize],
                    result.mesh.vertices[t[2] as usize]
                )
                .unwrap(),
                [0., 0., -1.]
            );
        }
        let mut hole = samples;
        hole[3] = None;
        let result = from_grid(2, 2, &hole, 2., &work).unwrap();
        assert!(result.mesh.triangles.is_empty());
        assert_eq!(result.incomplete_cells, 1);
        let result = from_grid(2, 2, &samples, 1.1, &work).unwrap();
        assert!(result.mesh.triangles.is_empty());
        assert_eq!(result.rejected_triangles, 2);
        let flat = [Some([0.; 3]); 4];
        let result = from_grid(2, 2, &flat, 2., &work).unwrap();
        assert_eq!(result.rejected_triangles, 2);
    }
    #[test]
    fn topology_reports_boundaries_nonmanifold_winding_duplicates_and_isolated_vertices() {
        let work = WorkControl::default();
        let mesh = tetra();
        assert!(mesh.topology(&work).unwrap().edge_closed);
        let mut open = mesh.clone();
        open.triangles.pop();
        assert_eq!(open.topology(&work).unwrap().boundary_edges, 3);
        let mut duplicate = mesh.clone();
        duplicate.triangles.push(mesh.triangles[0]);
        let t = duplicate.topology(&work).unwrap();
        assert_eq!(t.duplicate_faces, 1);
        assert_eq!(t.nonmanifold_edges, 3);
        assert!(!t.edge_closed);
        let mut reversed = mesh.clone();
        reversed.triangles[0].swap(0, 1);
        assert_eq!(
            reversed.topology(&work).unwrap().inconsistent_winding_edges,
            3
        );
        let mut isolated = mesh;
        isolated.vertices.push([100.; 3]);
        assert_eq!(isolated.topology(&work).unwrap().isolated_vertices, 1);
        assert!(!isolated.topology(&work).unwrap().edge_closed);
    }
    #[test]
    fn exporters_preserve_f64_coordinates_and_indexing_under_independent_text_parse() {
        let work = WorkControl::default();
        let mut mesh = tetra();
        mesh.vertices[1][0] = f64::from_bits(0.1f64.to_bits() + 1);
        let mut stl = Vec::new();
        mesh.write_stl(&mut stl, &work).unwrap();
        let vertices: Vec<[f64; 3]> = String::from_utf8(stl)
            .unwrap()
            .lines()
            .filter_map(|line| line.strip_prefix("vertex "))
            .map(|line| {
                let p: Vec<f64> = line
                    .split_whitespace()
                    .map(|v| v.parse().unwrap())
                    .collect();
                p.try_into().unwrap()
            })
            .collect();
        let expected: Vec<_> = mesh
            .triangles
            .iter()
            .flat_map(|t| t.map(|i| mesh.vertices[i as usize]))
            .collect();
        assert_eq!(vertices, expected);
        let mut obj = Vec::new();
        mesh.write_obj(&mut obj, &work).unwrap();
        let obj = String::from_utf8(obj).unwrap();
        let vertices: Vec<[f64; 3]> = obj
            .lines()
            .filter_map(|l| l.strip_prefix("v "))
            .map(|l| {
                l.split_whitespace()
                    .map(|v| v.parse::<f64>().unwrap())
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap()
            })
            .collect();
        assert_eq!(vertices, mesh.vertices);
        let faces: Vec<[u32; 3]> = obj
            .lines()
            .filter_map(|l| l.strip_prefix("f "))
            .map(|l| {
                l.split_whitespace()
                    .map(|v| v.parse::<u32>().unwrap() - 1)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap()
            })
            .collect();
        assert_eq!(faces, mesh.triangles);
    }
    #[test]
    fn invalid_geometry_and_cancellation_fail_without_exported_mesh_bytes() {
        let work = WorkControl::default();
        let mut invalid = tetra();
        invalid.triangles[0][0] = 99;
        let mut out = Vec::new();
        assert!(invalid.write_stl(&mut out, &work).is_err());
        assert!(out.is_empty());
        invalid = tetra();
        invalid.vertices[0][0] = f64::NAN;
        assert!(invalid.validate(&work).is_err());
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(tetra().write_obj(&mut out, &cancelled).is_err());
        assert!(out.is_empty());
        assert!(from_grid(2, 2, &[None; 4], f64::NAN, &work).is_err());
        assert!(from_grid(1, 4, &[None; 4], 1., &work).is_err());
        assert!(from_grid(2, 2, &[None; 4], 1., &cancelled).is_err());
    }
}
