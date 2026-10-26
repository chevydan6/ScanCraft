//! Exact edge-connected triangle components; no proximity welding or size heuristic.
use crate::{
    mesh::Mesh,
    work::{Stage, WorkControl},
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, Serialize)]
pub struct Component {
    pub id: usize,
    pub first_triangle: usize,
    pub triangles: usize,
    pub minimum_mm: [f64; 3],
    pub maximum_mm: [f64; 3],
}
pub struct Components {
    pub face_component: Vec<usize>,
    pub components: Vec<Component>,
}
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}
fn union(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
    let mut a = root(parent, a);
    let mut b = root(parent, b);
    if a == b {
        return;
    }
    if rank[a] < rank[b] || (rank[a] == rank[b] && a > b) {
        std::mem::swap(&mut a, &mut b);
    }
    parent[b] = a;
    if rank[a] == rank[b] {
        rank[a] += 1;
    }
}
pub fn analyze(mesh: &Mesh, work: &WorkControl) -> Result<Components, String> {
    mesh.validate(work)?;
    let mut parent: Vec<_> = (0..mesh.triangles.len()).collect();
    let mut rank = vec![0u8; parent.len()];
    let mut edges = HashMap::new();
    for (i, t) in mesh.triangles.iter().enumerate() {
        work.update(Stage::Indexing, i, mesh.triangles.len())?;
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let key = if a < b { [a, b] } else { [b, a] };
            if let Some(other) = edges.get(&key) {
                union(&mut parent, &mut rank, i, *other);
            } else {
                edges.insert(key, i);
            }
        }
    }
    drop(edges);
    drop(rank);
    let mut ids = HashMap::new();
    let mut components: Vec<Component> = Vec::new();
    let mut face_component = Vec::with_capacity(mesh.triangles.len());
    for (i, t) in mesh.triangles.iter().enumerate() {
        work.update(Stage::Processing, i, mesh.triangles.len())?;
        let key = root(&mut parent, i);
        let id = if let Some(id) = ids.get(&key) {
            *id
        } else {
            if components.len() == 100_000 {
                return Err("component analysis exceeds 100000 component budget".into());
            }
            let id = components.len();
            let p = mesh.vertices[t[0] as usize];
            components.push(Component {
                id,
                first_triangle: i,
                triangles: 0,
                minimum_mm: p,
                maximum_mm: p,
            });
            ids.insert(key, id);
            id
        };
        let info = &mut components[id];
        info.triangles += 1;
        for v in t {
            let p = mesh.vertices[*v as usize];
            for (axis, value) in p.into_iter().enumerate() {
                info.minimum_mm[axis] = info.minimum_mm[axis].min(value);
                info.maximum_mm[axis] = info.maximum_mm[axis].max(value);
            }
        }
        face_component.push(id);
    }
    work.check()?;
    Ok(Components {
        face_component,
        components,
    })
}
pub struct ComponentSelection {
    pub mesh: Mesh,
    pub component_count: usize,
    pub selected: Vec<Component>,
    pub removed_triangles: usize,
}
/// Explicit component IDs refer to first-triangle order. Keep/delete preserves
/// source face order and surviving coordinates; unused vertices are compacted.
pub fn select(
    mesh: &Mesh,
    ids: &[usize],
    keep: bool,
    work: &WorkControl,
) -> Result<ComponentSelection, String> {
    if ids.is_empty() || ids.len() > 100_000 {
        return Err("select 1..100000 explicit component IDs".into());
    }
    let analysis = analyze(mesh, work)?;
    let mut chosen = HashSet::new();
    for id in ids {
        if *id >= analysis.components.len() || !chosen.insert(*id) {
            return Err("invalid or duplicate component ID".into());
        }
    }
    let mut triangles = Vec::new();
    let mut used = vec![false; mesh.vertices.len()];
    for (i, t) in mesh.triangles.iter().enumerate() {
        work.update(Stage::Processing, i, mesh.triangles.len())?;
        if chosen.contains(&analysis.face_component[i]) == keep {
            triangles.push(*t);
            for v in t {
                used[*v as usize] = true;
            }
        }
    }
    let mut vertices = Vec::new();
    let mut map = vec![0u32; mesh.vertices.len()];
    for (i, p) in mesh.vertices.iter().enumerate() {
        if i.is_multiple_of(1024) {
            work.check()?;
        }
        if used[i] {
            map[i] = vertices.len() as u32;
            vertices.push(*p);
        }
    }
    for t in &mut triangles {
        work.check()?;
        *t = t.map(|i| map[i as usize]);
    }
    let removed_triangles = mesh.triangles.len() - triangles.len();
    let result = Mesh {
        vertices,
        triangles,
    };
    result.validate(work)?;
    let selected = analysis
        .components
        .iter()
        .filter(|c| chosen.contains(&c.id))
        .cloned()
        .collect();
    Ok(ComponentSelection {
        mesh: result,
        component_count: analysis.components.len(),
        selected,
        removed_triangles,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Mesh {
        Mesh {
            vertices: vec![
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 1., 0.],
                [1., 1., 0.],
                [0., 0., 1.],
                [0., 1., 1.],
                [10., 0., 0.],
                [11., 0., 0.],
                [10., 1., 0.],
                [999.; 3],
            ],
            triangles: vec![[0, 2, 1], [1, 2, 3], [0, 4, 5], [6, 7, 8]],
        }
    }
    #[test]
    fn shared_edges_connect_but_vertex_touch_and_geometric_proximity_do_not() {
        let mesh = fixture();
        let w = WorkControl::default();
        let a = analyze(&mesh, &w).unwrap();
        assert_eq!(a.face_component, [0, 0, 1, 2]);
        assert_eq!(
            a.components.iter().map(|c| c.triangles).collect::<Vec<_>>(),
            [2, 1, 1]
        );
        assert_eq!(a.components[2].minimum_mm, [10., 0., 0.]);
        assert_eq!(a.components[2].maximum_mm, [11., 1., 0.]);
        assert_eq!(analyze(&mesh, &w).unwrap().face_component, a.face_component);
        let mut coincident = mesh;
        coincident
            .vertices
            .extend([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]);
        coincident.triangles.push([10, 11, 12]);
        assert_eq!(analyze(&coincident, &w).unwrap().components.len(), 4);
    }
    #[test]
    fn keep_remove_exact_faces_compaction_and_undo_do_not_drop_small_unselected_parts() {
        let mesh = fixture();
        let w = WorkControl::default();
        let kept = select(&mesh, &[2], true, &w).unwrap();
        assert_eq!(kept.mesh.vertices, mesh.vertices[6..9]);
        assert_eq!(kept.mesh.triangles, vec![[0, 1, 2]]);
        assert_eq!(kept.removed_triangles, 3);
        let removed = select(&mesh, &[0], false, &w).unwrap();
        assert_eq!(removed.mesh.triangles.len(), 2);
        assert_eq!(removed.selected[0].triangles, 2);
        let mut editor = crate::mesh_edit::MeshEditor::new(mesh.clone(), 8, &w).unwrap();
        editor.replace(removed.mesh, &w).unwrap();
        editor.undo(&w).unwrap();
        assert_eq!(editor.mesh(), &mesh);
        assert!(
            select(&mesh, &[0, 1, 2], false, &w)
                .unwrap()
                .mesh
                .vertices
                .is_empty()
        );
    }
    #[test]
    fn invalid_selection_and_cancellation_return_no_partial_result() {
        let m = fixture();
        let w = WorkControl::default();
        for ids in [&[][..], &[0, 0][..], &[99][..]] {
            assert!(select(&m, ids, true, &w).is_err());
        }
        let c = WorkControl::default();
        c.cancel();
        assert!(analyze(&m, &c).is_err());
        assert!(select(&m, &[0], true, &c).is_err());
        assert_eq!(m.triangles.len(), 4);
    }
}
