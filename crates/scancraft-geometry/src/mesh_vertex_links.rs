//! Corner components of indexed vertex links. Coordinates are never welded.
use crate::{mesh::Mesh, work::WorkControl};
pub(crate) struct CornerLinks {
    parent: Vec<u32>,
    rank: Vec<u8>,
}
impl CornerLinks {
    // Caller has already checked Mesh's two-million-face bound.
    pub(crate) fn new(faces: usize) -> Self {
        let count = faces * 3;
        Self {
            parent: (0..count as u32).collect(),
            rank: vec![0; count],
        }
    }
    fn root(&mut self, mut i: usize) -> usize {
        while self.parent[i] as usize != i {
            self.parent[i] = self.parent[self.parent[i] as usize];
            i = self.parent[i] as usize;
        }
        i
    }
    pub(crate) fn join(&mut self, a: u32, b: u32) {
        let mut a = self.root(a as usize);
        let mut b = self.root(b as usize);
        if a == b {
            return;
        }
        if self.rank[a] < self.rank[b] || (self.rank[a] == self.rank[b] && a > b) {
            std::mem::swap(&mut a, &mut b);
        }
        self.parent[b] = a as u32;
        if self.rank[a] == self.rank[b] {
            self.rank[a] += 1;
        }
    }
    pub(crate) fn fan_counts(&self, mesh: &Mesh, work: &WorkControl) -> Result<Vec<u32>, String> {
        let mut counts = vec![0; mesh.vertices.len()];
        for (i, &parent) in self.parent.iter().enumerate() {
            if i % 1024 == 0 {
                work.check()?;
            }
            if parent as usize == i {
                counts[mesh.triangles[i / 3][i % 3] as usize] += 1;
            }
        }
        Ok(counts)
    }
    pub(crate) fn corner_roots(mut self, work: &WorkControl) -> Result<Vec<u32>, String> {
        for i in 0..self.parent.len() {
            if i % 1024 == 0 {
                work.check()?;
            }
            self.parent[i] = self.root(i) as u32;
        }
        Ok(self.parent)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pinched() -> Mesh {
        serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../../evidence/mesh-vertex-pinch-input.json"
            ))
            .unwrap()["mesh"]
                .clone(),
        )
        .unwrap()
    }
    #[test]
    fn closed_edge_test_does_not_hide_disconnected_vertex_links() {
        let mesh = pinched();
        let original = mesh.clone();
        let t = mesh.topology(&WorkControl::default()).unwrap();
        assert!(t.edge_closed);
        assert_eq!(t.nonmanifold_edges, 0);
        assert_eq!(t.boundary_edges, 0);
        assert_eq!(t.nonmanifold_vertex_ids, vec![0]);
        assert_eq!(t.vertices_with_disconnected_fans, 1);
        assert_eq!(t.maximum_vertex_fans, 2);
        assert!(!t.closed_manifold_candidate);
        assert_eq!(mesh, original);
    }
    #[test]
    fn fill_refuses_a_valid_boundary_when_source_has_a_separate_pinched_vertex() {
        let mut ring: Mesh = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../../evidence/mesh-boundary-ring-input.json"
            ))
            .unwrap()["mesh"]
                .clone(),
        )
        .unwrap();
        let pinch = pinched();
        let offset = ring.vertices.len() as u32;
        ring.vertices
            .extend(pinch.vertices.iter().map(|p| p.map(|v| v + 20.)));
        ring.triangles
            .extend(pinch.triangles.iter().map(|t| t.map(|v| v + offset)));
        let work = WorkControl::default();
        let t = ring.topology(&work).unwrap();
        assert_eq!(t.nonmanifold_edges, 0);
        assert_eq!(t.nonmanifold_vertex_ids, vec![offset]);
        let original = ring.clone();
        assert!(
            crate::mesh_fill::fill(&ring, 1, 0.001, &work)
                .unwrap_err()
                .contains("nonmanifold")
        );
        assert_eq!(ring, original);
    }
    #[test]
    fn boundary_bowtie_is_flagged_and_split_indices_keep_shells_separate() {
        let mut mesh = pinched();
        mesh.vertices.push(mesh.vertices[0]);
        for t in &mut mesh.triangles[4..] {
            for v in t {
                if *v == 0 {
                    *v = 7;
                }
            }
        }
        let t = mesh.topology(&WorkControl::default()).unwrap();
        assert_eq!(t.nonmanifold_vertices, 0);
        assert_eq!(t.maximum_vertex_fans, 1);
        assert!(t.closed_manifold_candidate);
        let open = Mesh {
            vertices: vec![
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 1., 0.],
                [-1., 0., 0.],
                [0., -1., 0.],
            ],
            triangles: vec![[0, 1, 2], [0, 3, 4]],
        };
        let t = open.topology(&WorkControl::default()).unwrap();
        assert_eq!(t.nonmanifold_vertex_ids, vec![0]);
        assert_eq!(t.vertices_with_invalid_boundary_degree, 1);
    }
    #[test]
    fn manifold_patch_and_empty_or_isolated_vertices_have_explicit_classification() {
        let mut mesh = Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            triangles: vec![[0, 1, 2]],
        };
        let t = mesh.topology(&WorkControl::default()).unwrap();
        assert_eq!(t.nonmanifold_vertices, 0);
        assert_eq!(t.maximum_vertex_fans, 1);
        assert!(!t.closed_manifold_candidate);
        mesh.vertices.push([9.; 3]);
        let t = mesh.topology(&WorkControl::default()).unwrap();
        assert_eq!(t.isolated_vertices, 1);
        assert_eq!(t.nonmanifold_vertices, 0);
        let empty = Mesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
        }
        .topology(&WorkControl::default())
        .unwrap();
        assert_eq!(empty.maximum_vertex_fans, 0);
        assert!(!empty.closed_manifold_candidate);
        let work = WorkControl::default();
        work.cancel();
        assert!(mesh.topology(&work).is_err());
    }
    #[test]
    fn nonmanifold_edge_flags_both_endpoints_and_duplicate_faces_block_candidate() {
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
        let t = mesh.topology(&WorkControl::default()).unwrap();
        assert_eq!(t.nonmanifold_edges, 1);
        assert_eq!(t.nonmanifold_vertex_ids, vec![1, 2]);
        assert_eq!(t.vertices_with_disconnected_fans, 0);
        assert!(!t.closed_manifold_candidate);
        let duplicate = Mesh {
            vertices: mesh.vertices[..3].to_vec(),
            triangles: vec![[0, 1, 2], [2, 1, 0]],
        };
        let t = duplicate.topology(&WorkControl::default()).unwrap();
        assert_eq!(t.duplicate_faces, 1);
        assert!(!t.closed_manifold_candidate);
    }
}
