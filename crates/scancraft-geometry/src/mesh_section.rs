//! Topology-preserving triangle/plane sections. No coordinate welding or area claims.
use crate::{
    mesh::Mesh,
    work::{Stage, WorkControl},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
const MAX_SEGMENTS: usize = 1_000_000;
const MAX_COMPONENTS: usize = 100_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Source {
    Vertex { index: u32 },
    Edge { vertices: [u32; 2] },
}
#[derive(Debug, Serialize)]
pub struct Node {
    pub position_mm: [f64; 3],
    pub source: Source,
}
#[derive(Debug, Serialize)]
pub struct Contour {
    pub nodes: Vec<u32>,
    pub closed: bool,
    pub length_mm: f64,
}
#[derive(Debug, Serialize)]
pub struct Ambiguous {
    pub nodes: Vec<u32>,
    pub segments: Vec<u32>,
    pub branching_nodes: usize,
}
#[derive(Debug, Serialize)]
pub struct Section {
    pub plane_origin_mm: [f64; 3],
    pub unit_normal: [f64; 3],
    pub tolerance_mm: f64,
    pub nodes: Vec<Node>,
    pub segments: Vec<[u32; 2]>,
    pub contours: Vec<Contour>,
    pub ambiguous_components: Vec<Ambiguous>,
    pub coplanar_triangles: usize,
    pub tangent_triangles: usize,
    pub duplicate_segments: usize,
    pub total_segment_length_mm: f64,
}
/// The tolerance classifies existing vertices as on-plane without moving them.
/// Coplanar triangles have no unique line section and are counted but omitted.
pub fn intersect(
    mesh: &Mesh,
    origin: [f64; 3],
    normal: [f64; 3],
    tolerance: f64,
    work: &WorkControl,
) -> Result<Section, String> {
    mesh.validate(work)?;
    if !origin.iter().chain(normal.iter()).all(|v| v.is_finite())
        || !tolerance.is_finite()
        || tolerance < 0.
    {
        return Err(
            "section plane and tolerance must be finite; tolerance must be nonnegative".into(),
        );
    }
    let scale = normal.iter().map(|v| v.abs()).fold(0_f64, f64::max);
    if scale == 0. {
        return Err("section plane normal must be nonzero".into());
    }
    let scaled = normal.map(|v| v / scale);
    let length = scaled[0].hypot(scaled[1]).hypot(scaled[2]);
    let n = scaled.map(|v| v / length);
    let mut distances = Vec::with_capacity(mesh.vertices.len());
    for (i, p) in mesh.vertices.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let delta: [f64; 3] = std::array::from_fn(|a| p[a] - origin[a]);
        let s = delta.iter().map(|v| v.abs()).fold(0_f64, f64::max);
        if !delta.iter().all(|v| v.is_finite()) {
            return Err("section coordinate difference overflow".into());
        }
        let d = if s == 0. {
            0.
        } else {
            (0..3).map(|a| n[a] * (delta[a] / s)).sum::<f64>() * s
        };
        if !d.is_finite() {
            return Err("section plane distance overflow".into());
        }
        distances.push(if d.abs() <= tolerance { 0. } else { d });
    }
    let mut out = Section {
        plane_origin_mm: origin,
        unit_normal: n,
        tolerance_mm: tolerance,
        nodes: vec![],
        segments: vec![],
        contours: vec![],
        ambiguous_components: vec![],
        coplanar_triangles: 0,
        tangent_triangles: 0,
        duplicate_segments: 0,
        total_segment_length_mm: 0.,
    };
    let mut ids = BTreeMap::new();
    let mut segment_keys = BTreeSet::new();
    for (i, t) in mesh.triangles.iter().enumerate() {
        work.update(Stage::Processing, i, mesh.triangles.len())?;
        let on = t.iter().filter(|&&v| distances[v as usize] == 0.).count();
        if on == 3 {
            out.coplanar_triangles += 1;
            continue;
        }
        let mut hits = BTreeMap::new();
        for &v in t {
            if distances[v as usize] == 0. {
                hits.insert(Source::Vertex { index: v }, mesh.vertices[v as usize]);
            }
        }
        for edge in 0..3 {
            let mut pair = [t[edge], t[(edge + 1) % 3]];
            pair.sort_unstable();
            let [a, b] = pair;
            let da = distances[a as usize];
            let db = distances[b as usize];
            if (da < 0. && db > 0.) || (da > 0. && db < 0.) {
                let s = da.abs().max(db.abs());
                let ratio = (da / s) / (da / s - db / s);
                let p = std::array::from_fn(|axis| {
                    mesh.vertices[a as usize][axis] * (1. - ratio)
                        + mesh.vertices[b as usize][axis] * ratio
                });
                if !p.iter().all(|v| v.is_finite()) {
                    return Err("section intersection overflow".into());
                }
                hits.insert(Source::Edge { vertices: pair }, p);
            }
        }
        if hits.len() == 1 {
            out.tangent_triangles += 1;
            continue;
        }
        if hits.is_empty() {
            continue;
        }
        if hits.len() != 2 {
            return Err("unexpected section intersection count".into());
        }
        let mut endpoints = [0; 2];
        for (j, (source, position_mm)) in hits.into_iter().enumerate() {
            let index = if let Some(&id) = ids.get(&source) {
                id
            } else {
                if out.nodes.len() == MAX_SEGMENTS * 2 {
                    return Err("section node budget exceeded".into());
                }
                let id = out.nodes.len() as u32;
                out.nodes.push(Node {
                    position_mm,
                    source,
                });
                ids.insert(source, id);
                id
            };
            endpoints[j] = index;
        }
        endpoints.sort_unstable();
        if !segment_keys.insert(endpoints) {
            out.duplicate_segments += 1;
            continue;
        }
        if out.segments.len() == MAX_SEGMENTS {
            return Err("section segment budget exceeded".into());
        }
        let length = crate::distance_mm(
            out.nodes[endpoints[0] as usize].position_mm,
            out.nodes[endpoints[1] as usize].position_mm,
        )?;
        if length == 0. {
            return Err("section segment collapsed at available coordinate precision".into());
        }
        out.total_segment_length_mm += length;
        if !out.total_segment_length_mm.is_finite() {
            return Err("section length overflow".into());
        }
        out.segments.push(endpoints);
    }
    let mut adjacency = vec![Vec::new(); out.nodes.len()];
    for (i, &[a, b]) in out.segments.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        adjacency[a as usize].push((b, i as u32));
        adjacency[b as usize].push((a, i as u32));
    }
    let mut visited = vec![false; out.nodes.len()];
    let mut components = 0;
    for first in 0..out.nodes.len() {
        if visited[first] {
            continue;
        }
        work.check()?;
        components += 1;
        if components > MAX_COMPONENTS {
            return Err("section component budget exceeded".into());
        }
        let mut queue = VecDeque::from([first as u32]);
        visited[first] = true;
        let mut nodes = Vec::new();
        let mut edges = BTreeSet::new();
        while let Some(v) = queue.pop_front() {
            if nodes.len() % 1024 == 0 {
                work.check()?;
            }
            nodes.push(v);
            for &(neighbor, edge) in &adjacency[v as usize] {
                edges.insert(edge);
                if !visited[neighbor as usize] {
                    visited[neighbor as usize] = true;
                    queue.push_back(neighbor);
                }
            }
        }
        nodes.sort_unstable();
        let ends: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|&v| adjacency[v as usize].len() == 1)
            .collect();
        let branching = nodes
            .iter()
            .filter(|&&v| adjacency[v as usize].len() > 2)
            .count();
        if branching > 0 || !(ends.is_empty() || ends.len() == 2) {
            out.ambiguous_components.push(Ambiguous {
                nodes,
                segments: edges.into_iter().collect(),
                branching_nodes: branching,
            });
            continue;
        }
        let closed = ends.is_empty();
        let start = ends.first().copied().unwrap_or(nodes[0]);
        let mut path = vec![start];
        let mut previous = None;
        let mut current = start;
        let mut length = 0.;
        loop {
            if path.len() % 1024 == 0 {
                work.check()?;
            }
            let next = adjacency[current as usize]
                .iter()
                .find(|(_, edge)| Some(*edge) != previous);
            let Some(&(neighbor, edge)) = next else {
                break;
            };
            length += crate::distance_mm(
                out.nodes[current as usize].position_mm,
                out.nodes[neighbor as usize].position_mm,
            )?;
            if !length.is_finite() {
                return Err("section contour length overflow".into());
            }
            if neighbor == start {
                break;
            }
            path.push(neighbor);
            previous = Some(edge);
            current = neighbor;
            if !closed && adjacency[current as usize].len() == 1 {
                break;
            }
            if path.len() > nodes.len() {
                return Err("section graph traversal failed".into());
            }
        }
        out.contours.push(Contour {
            nodes: path,
            closed,
            length_mm: length,
        });
    }
    work.check()?;
    Ok(out)
}
