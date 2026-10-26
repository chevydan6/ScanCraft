//! Explicit indexed vertex-fan splits. Duplicated vertices keep identical coordinates.
use crate::{
    mesh::Mesh,
    mesh_vertex_links::CornerLinks,
    work::{Stage, WorkControl},
};
use serde::Serialize;
use std::collections::{HashMap, HashSet, hash_map::Entry};
#[derive(Debug, Serialize)]
pub struct VertexFanSplit {
    pub original_vertex: u32,
    /// First entry retains the original index; others are appended indices.
    pub fan_vertices: Vec<u32>,
    pub first_triangles: Vec<usize>,
}
#[derive(Debug)]
pub struct SplitFans {
    pub mesh: Mesh,
    pub vertices_added: usize,
    pub splits: Vec<VertexFanSplit>,
}
/// Every selected vertex must have multiple edge-disconnected incident-face
/// fans. The first fan in face order keeps its index. Others receive exact
/// coordinate copies. No face, winding, position or geometric gap is changed.
/// Nonmanifold edges/duplicate faces require separate repair first.
pub fn split(source: &Mesh, ids: &[u32], work: &WorkControl) -> Result<SplitFans, String> {
    let topology = source.topology(work)?;
    if ids.is_empty() || ids.len() > 100_000 {
        return Err("select 1..100000 explicit vertex IDs".into());
    }
    let chosen: HashSet<_> = ids.iter().copied().collect();
    if chosen.len() != ids.len() {
        return Err("duplicate selected vertex ID".into());
    }
    if chosen.iter().any(|v| *v as usize >= source.vertices.len()) {
        return Err("selected vertex ID outside mesh".into());
    }
    if topology.nonmanifold_edges > 0 || topology.duplicate_faces > 0 {
        return Err("resolve nonmanifold edges/duplicate faces before vertex-fan splitting".into());
    }
    let mut links = CornerLinks::new(source.triangles.len());
    let mut edges: HashMap<[u32; 2], [u32; 2]> = HashMap::new();
    for (i, t) in source.triangles.iter().enumerate() {
        work.update(Stage::Indexing, i, source.triangles.len())?;
        for j in 0..3 {
            let a = t[j];
            let b = t[(j + 1) % 3];
            let corners = [(3 * i + j) as u32, (3 * i + (j + 1) % 3) as u32];
            let (key, corners) = if a < b {
                ([a, b], corners)
            } else {
                ([b, a], [corners[1], corners[0]])
            };
            if let Some(first) = edges.get(&key) {
                links.join(first[0], corners[0]);
                links.join(first[1], corners[1]);
            } else {
                edges.insert(key, corners);
            }
        }
    }
    drop(edges);
    let roots = links.corner_roots(work)?;
    let mut first_corner = HashMap::new();
    let mut fan_counts = vec![0u32; source.vertices.len()];
    let mut extra = 0;
    for (i, &root) in roots.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let vertex = source.triangles[i / 3][i % 3];
        if !chosen.contains(&vertex) {
            continue;
        }
        if let Entry::Vacant(e) = first_corner.entry((vertex, root)) {
            e.insert(i);
            if fan_counts[vertex as usize] > 0 {
                extra += 1;
                if source.vertices.len() + extra > 1_000_000 {
                    return Err("fan split exceeds vertex budget".into());
                }
            }
            fan_counts[vertex as usize] += 1;
        }
    }
    if chosen.iter().any(|v| fan_counts[*v as usize] < 2) {
        return Err("every selected vertex must have multiple disconnected fans".into());
    }
    let mut fans: Vec<_> = first_corner
        .into_iter()
        .map(|((v, root), first)| (v, root, first))
        .collect();
    fans.sort_unstable_by_key(|&(v, _, first)| (v, first));
    let mut mesh = source.clone();
    let mut splits: Vec<VertexFanSplit> = Vec::new();
    let mut mapping = HashMap::new();
    for (v, root, first) in fans {
        work.check()?;
        let index = if splits.last().is_none_or(|s| s.original_vertex != v) {
            splits.push(VertexFanSplit {
                original_vertex: v,
                fan_vertices: Vec::new(),
                first_triangles: Vec::new(),
            });
            v
        } else {
            let index = mesh.vertices.len() as u32;
            mesh.vertices.push(source.vertices[v as usize]);
            index
        };
        let info = splits.last_mut().unwrap();
        info.fan_vertices.push(index);
        info.first_triangles.push(first / 3);
        mapping.insert((v, root), index);
    }
    for (i, &root) in roots.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Processing, i, roots.len())?;
        }
        let v = source.triangles[i / 3][i % 3];
        if let Some(&index) = mapping.get(&(v, root)) {
            mesh.triangles[i / 3][i % 3] = index;
        }
    }
    let result = mesh.topology(work)?;
    if result.nonmanifold_edges != 0
        || result.duplicate_faces != 0
        || result.boundary_edges != topology.boundary_edges
        || result.inconsistent_winding_edges != topology.inconsistent_winding_edges
        || result.isolated_vertices != topology.isolated_vertices
        || !topology
            .nonmanifold_vertex_ids
            .iter()
            .copied()
            .filter(|v| !chosen.contains(v))
            .eq(result.nonmanifold_vertex_ids.iter().copied())
        || splits
            .iter()
            .flat_map(|s| &s.fan_vertices)
            .any(|v| result.nonmanifold_vertex_ids.binary_search(v).is_ok())
    {
        return Err("fan split failed selected-vertex manifold checks".into());
    }
    work.check()?;
    Ok(SplitFans {
        mesh,
        vertices_added: extra,
        splits,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pinch() -> Mesh {
        serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../../evidence/mesh-vertex-pinch-input.json"
            ))
            .unwrap()["mesh"]
                .clone(),
        )
        .unwrap()
    }
    fn assert_same_faces(a: &Mesh, b: &Mesh) {
        assert_eq!(a.triangles.len(), b.triangles.len());
        for (x, y) in a.triangles.iter().zip(&b.triangles) {
            for i in 0..3 {
                assert_eq!(
                    a.vertices[x[i] as usize].map(f64::to_bits),
                    b.vertices[y[i] as usize].map(f64::to_bits)
                );
            }
        }
    }
    #[test]
    fn pinched_shells_split_indices_with_bit_exact_face_positions_and_no_gap() {
        let mut source = pinch();
        source.vertices[0][0] = -0.;
        let original = source.clone();
        let work = WorkControl::default();
        let p = split(&source, &[0], &work).unwrap();
        assert_eq!(p.vertices_added, 1);
        assert_eq!(p.splits[0].fan_vertices, vec![0, 7]);
        assert_eq!(p.splits[0].first_triangles, vec![0, 4]);
        assert_same_faces(&source, &p.mesh);
        assert_eq!(
            p.mesh.vertices[0].map(f64::to_bits),
            p.mesh.vertices[7].map(f64::to_bits)
        );
        assert_eq!(source, original);
        let mut stl_before = Vec::new();
        let mut stl_after = Vec::new();
        source.write_stl(&mut stl_before, &work).unwrap();
        p.mesh.write_stl(&mut stl_after, &work).unwrap();
        assert_eq!(stl_before, stl_after); // STL carries triangle geometry, not shared vertex IDs.
        let mut obj = Vec::new();
        p.mesh.write_obj(&mut obj, &work).unwrap();
        assert!(
            String::from_utf8(obj)
                .unwrap()
                .lines()
                .any(|line| line.starts_with("f 8 "))
        );
        let t = p.mesh.topology(&work).unwrap();
        assert_eq!(t.nonmanifold_vertices, 0);
        assert!(t.closed_manifold_candidate);
        assert!(split(&p.mesh, &[0], &work).is_err());
    }
    #[test]
    fn only_selected_vertex_is_split_and_invalid_selection_or_cancellation_fails() {
        let mut source = pinch();
        let second = pinch();
        let offset = source.vertices.len() as u32;
        source.vertices.extend(second.vertices);
        source
            .triangles
            .extend(second.triangles.iter().map(|t| t.map(|v| v + offset)));
        let work = WorkControl::default();
        let p = split(&source, &[offset], &work).unwrap();
        assert_eq!(p.vertices_added, 1);
        assert_same_faces(&source, &p.mesh);
        assert_eq!(
            p.mesh.topology(&work).unwrap().nonmanifold_vertex_ids,
            vec![0]
        );
        let both = split(&source, &[0, offset], &work).unwrap();
        let reordered = split(&source, &[offset, 0], &work).unwrap();
        assert_eq!(both.mesh, reordered.mesh);
        for ids in [vec![], vec![0, 0], vec![1], vec![999]] {
            assert!(split(&source, &ids, &work).is_err());
        }
        let mut duplicate = pinch();
        duplicate.triangles.push(duplicate.triangles[0]);
        assert!(split(&duplicate, &[0], &work).is_err());
        work.cancel();
        assert!(split(&source, &[0], &work).is_err());
    }
    #[test]
    fn open_bowtie_becomes_two_boundary_fans_without_changing_faces() {
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
        let p = split(&mesh, &[0], &WorkControl::default()).unwrap();
        assert_same_faces(&mesh, &p.mesh);
        let b = crate::mesh_boundaries::analyze(&p.mesh, &WorkControl::default()).unwrap();
        assert_eq!(b.loops.len(), 2);
        assert!(b.ambiguous_components.is_empty());
    }
    #[test]
    fn vertex_budget_and_nonmanifold_edges_refuse_without_source_mutation() {
        let mut source = pinch();
        source.vertices.resize(1_000_000, [99.; 3]);
        assert!(
            split(&source, &[0], &WorkControl::default())
                .unwrap_err()
                .contains("vertex budget")
        );
        assert_eq!(source.vertices.len(), 1_000_000);
        let mesh = Mesh {
            vertices: vec![
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 1., 0.],
                [1., 1., 0.],
                [0., 0., 1.],
            ],
            triangles: vec![[0, 1, 2], [1, 2, 3], [1, 2, 4]],
        };
        assert!(
            split(&mesh, &[1], &WorkControl::default())
                .unwrap_err()
                .contains("nonmanifold edges")
        );
    }
    #[test]
    fn splitting_pinched_component_allows_separate_explicit_ring_fill() {
        let mut source: Mesh = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../../evidence/mesh-boundary-ring-input.json"
            ))
            .unwrap()["mesh"]
                .clone(),
        )
        .unwrap();
        let other = pinch();
        let offset = source.vertices.len() as u32;
        source
            .vertices
            .extend(other.vertices.iter().map(|p| p.map(|v| v + 20.)));
        source
            .triangles
            .extend(other.triangles.iter().map(|t| t.map(|v| v + offset)));
        let work = WorkControl::default();
        assert!(crate::mesh_fill::fill(&source, 1, 0.001, &work).is_err());
        let repaired = split(&source, &[offset], &work).unwrap();
        assert_same_faces(&source, &repaired.mesh);
        let filled = crate::mesh_fill::fill(&repaired.mesh, 1, 0.001, &work).unwrap();
        assert_eq!(filled.added_triangles, 2);
        assert_eq!(filled.mesh.topology(&work).unwrap().boundary_edges, 4);
        assert_eq!(
            source.topology(&work).unwrap().nonmanifold_vertex_ids,
            vec![offset]
        );
    }
}
