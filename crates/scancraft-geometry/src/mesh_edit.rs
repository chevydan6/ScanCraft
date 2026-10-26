//! Transactional surface editing. Plane trims split shared edges and never cap openings.
use crate::{
    mesh::{Mesh, triangle_normal},
    work::{Stage, WorkControl},
};
use std::collections::{BTreeMap, VecDeque};
#[derive(Debug)]
pub struct PlaneTrim {
    pub mesh: Mesh,
    pub cut_edges: usize,
    pub rejected_slivers: usize,
}
/// Keep normal dot position + offset <= 0. The plane is normalized internally.
/// Source is immutable; output compacts unreferenced vertices and leaves cuts open.
pub fn trim_plane(
    source: &Mesh,
    normal: [f64; 3],
    offset_mm: f64,
    work: &WorkControl,
) -> Result<PlaneTrim, String> {
    source.validate(work)?;
    let length = normal[0].hypot(normal[1]).hypot(normal[2]);
    if !normal.iter().all(|v| v.is_finite())
        || !offset_mm.is_finite()
        || !length.is_finite()
        || length <= 1e-12
    {
        return Err("invalid mesh trim plane".into());
    }
    let n = normal.map(|v| v / length);
    let offset = offset_mm / length;
    if !offset.is_finite() {
        return Err("plane normalization overflow".into());
    }
    let mut distances = Vec::with_capacity(source.vertices.len());
    for p in &source.vertices {
        work.check()?;
        let distance = n[0] * p[0] + n[1] * p[1] + n[2] * p[2] + offset;
        if !distance.is_finite() {
            return Err("plane distance overflow".into());
        }
        distances.push(distance);
    }
    let mut vertices = source.vertices.clone();
    let mut triangles = Vec::new();
    let mut intersections = BTreeMap::new();
    let mut rejected_slivers = 0;
    for (i, t) in source.triangles.iter().enumerate() {
        work.update(Stage::Processing, i, source.triangles.len())?;
        let mut polygon = Vec::with_capacity(4);
        for edge in 0..3 {
            let a = t[edge];
            let b = t[(edge + 1) % 3];
            let da = distances[a as usize];
            let db = distances[b as usize];
            if da <= 0. {
                polygon.push(a);
            }
            if (da < 0. && db > 0.) || (da > 0. && db < 0.) {
                let key = if a < b { [a, b] } else { [b, a] };
                let index = if let Some(index) = intersections.get(&key) {
                    *index
                } else {
                    if vertices.len() == 1_000_000 {
                        return Err("plane trim exceeds vertex budget".into());
                    }
                    let scale = da.abs().max(db.abs());
                    let ratio = (da / scale) / (da / scale - db / scale);
                    let point = std::array::from_fn(|axis| {
                        vertices[a as usize][axis] * (1. - ratio)
                            + vertices[b as usize][axis] * ratio
                    });
                    if !point.iter().all(|v| v.is_finite()) {
                        return Err("plane intersection overflow".into());
                    }
                    let index = vertices.len() as u32;
                    vertices.push(point);
                    intersections.insert(key, index);
                    index
                };
                polygon.push(index);
            }
        }
        for j in 1..polygon.len().saturating_sub(1) {
            let t = [polygon[0], polygon[j], polygon[j + 1]];
            if triangle_normal(
                vertices[t[0] as usize],
                vertices[t[1] as usize],
                vertices[t[2] as usize],
            )
            .is_err()
            {
                rejected_slivers += 1;
                continue;
            }
            if triangles.len() == 2_000_000 {
                return Err("plane trim exceeds triangle budget".into());
            }
            triangles.push(t);
        }
    }
    let mut used = vec![false; vertices.len()];
    for t in &triangles {
        for i in t {
            used[*i as usize] = true;
        }
    }
    let mut map = vec![0u32; vertices.len()];
    let mut compact = Vec::new();
    for (i, p) in vertices.into_iter().enumerate() {
        if i.is_multiple_of(1024) {
            work.check()?;
        }
        if used[i] {
            map[i] = compact.len() as u32;
            compact.push(p);
        }
    }
    for t in &mut triangles {
        work.check()?;
        *t = t.map(|i| map[i as usize]);
    }
    let mesh = Mesh {
        vertices: compact,
        triangles,
    };
    mesh.validate(work)?;
    Ok(PlaneTrim {
        mesh,
        cut_edges: intersections.len(),
        rejected_slivers,
    })
}
const HISTORY_BYTES: usize = 128 * 1024 * 1024;
fn payload_bytes(mesh: &Mesh) -> usize {
    mesh.vertices.len() * 24 + mesh.triangles.len() * 12
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshEditor {
    mesh: Mesh,
    undo: VecDeque<Mesh>,
    redo: Vec<Mesh>,
    limit: usize,
}
#[derive(Debug)]
pub struct EditCommit {
    pub evicted_undo_steps: usize,
}
impl MeshEditor {
    pub fn validate_history(&self, work: &WorkControl) -> Result<(), String> {
        if !(1..=32).contains(&self.limit) || self.undo.len() + self.redo.len() > self.limit {
            return Err("invalid mesh history count or limit".into());
        }
        let bytes = payload_bytes(&self.mesh)
            + self
                .undo
                .iter()
                .chain(self.redo.iter())
                .map(payload_bytes)
                .sum::<usize>();
        if bytes > HISTORY_BYTES {
            return Err("mesh history payload exceeds 128 MiB".into());
        }
        self.mesh.validate(work)?;
        for mesh in self.undo.iter().chain(self.redo.iter()) {
            mesh.validate(work)?;
        }
        work.check()
    }
    pub fn new(mesh: Mesh, history_limit: usize, work: &WorkControl) -> Result<Self, String> {
        if !(1..=32).contains(&history_limit) {
            return Err("mesh history limit must be 1..32".into());
        }
        mesh.validate(work)?;
        Ok(Self {
            mesh,
            undo: VecDeque::new(),
            redo: Vec::new(),
            limit: history_limit,
        })
    }
    pub fn from_history(
        mesh: Mesh,
        undo: VecDeque<Mesh>,
        redo: Vec<Mesh>,
        limit: usize,
        work: &WorkControl,
    ) -> Result<Self, String> {
        let editor = Self {
            mesh,
            undo,
            redo,
            limit,
        };
        editor.validate_history(work)?;
        Ok(editor)
    }
    pub fn into_history(self) -> (Mesh, VecDeque<Mesh>, Vec<Mesh>, usize) {
        (self.mesh, self.undo, self.redo, self.limit)
    }
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }
    pub fn previous_mesh(&self) -> Option<&Mesh> {
        self.undo.back()
    }
    pub fn history_depth(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }
    /// A validated proposal commits atomically after the last cancellation check.
    /// History evictions are explicit in the return value; geometry is never approximated.
    pub fn replace(&mut self, proposal: Mesh, work: &WorkControl) -> Result<EditCommit, String> {
        proposal.validate(work)?;
        work.check()?;
        let old = std::mem::replace(&mut self.mesh, proposal);
        self.redo.clear();
        self.undo.push_back(old);
        let mut bytes: usize =
            payload_bytes(&self.mesh) + self.undo.iter().map(payload_bytes).sum::<usize>();
        let mut evicted = 0;
        while self.undo.len() > self.limit || bytes > HISTORY_BYTES {
            bytes -= payload_bytes(&self.undo.pop_front().unwrap());
            evicted += 1;
        }
        Ok(EditCommit {
            evicted_undo_steps: evicted,
        })
    }
    pub fn undo(&mut self, work: &WorkControl) -> Result<bool, String> {
        work.check()?;
        let Some(old) = self.undo.pop_back() else {
            return Ok(false);
        };
        self.redo.push(std::mem::replace(&mut self.mesh, old));
        Ok(true)
    }
    pub fn redo(&mut self, work: &WorkControl) -> Result<bool, String> {
        work.check()?;
        let Some(next) = self.redo.pop() else {
            return Ok(false);
        };
        self.undo.push_back(std::mem::replace(&mut self.mesh, next));
        Ok(true)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CleanupOptions {
    pub duplicate_faces: bool,
    pub unused_vertices: bool,
}
pub struct MeshCleanup {
    pub mesh: Mesh,
    pub removed_faces: usize,
    pub removed_vertices: usize,
}
/// Explicit cleanup only: duplicate identity ignores winding and retains the
/// first original face. No welding, hole filling or position changes.
pub fn cleanup(
    source: &Mesh,
    options: CleanupOptions,
    work: &WorkControl,
) -> Result<MeshCleanup, String> {
    source.validate(work)?;
    if !options.duplicate_faces && !options.unused_vertices {
        return Err("select at least one mesh cleanup operation".into());
    }
    let mut seen = std::collections::HashSet::new();
    let mut triangles = Vec::new();
    let mut removed_faces = 0;
    for (i, t) in source.triangles.iter().enumerate() {
        work.update(Stage::Processing, i, source.triangles.len())?;
        let mut key = *t;
        key.sort_unstable();
        if options.duplicate_faces && !seen.insert(key) {
            removed_faces += 1;
        } else {
            triangles.push(*t);
        }
    }
    let mut used = vec![!options.unused_vertices; source.vertices.len()];
    for t in &triangles {
        for i in t {
            used[*i as usize] = true;
        }
    }
    let mut vertices = Vec::new();
    let mut map = vec![0u32; source.vertices.len()];
    for (i, p) in source.vertices.iter().enumerate() {
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
    let removed_vertices = source.vertices.len() - vertices.len();
    let mesh = Mesh {
        vertices,
        triangles,
    };
    mesh.validate(work)?;
    Ok(MeshCleanup {
        mesh,
        removed_faces,
        removed_vertices,
    })
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    #[test]
    fn cleanup_preserves_first_winding_coordinates_and_exact_undo() {
        let source = Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [99.; 3]],
            triangles: vec![[0, 2, 1], [2, 1, 0], [1, 2, 0]],
        };
        let w = WorkControl::default();
        let result = cleanup(
            &source,
            CleanupOptions {
                duplicate_faces: true,
                unused_vertices: true,
            },
            &w,
        )
        .unwrap();
        assert_eq!(result.removed_faces, 2);
        assert_eq!(result.removed_vertices, 1);
        assert_eq!(result.mesh.vertices, &source.vertices[..3]);
        assert_eq!(result.mesh.triangles, vec![[0, 2, 1]]);
        let mut editor = MeshEditor::new(source.clone(), 8, &w).unwrap();
        editor.replace(result.mesh.clone(), &w).unwrap();
        editor.undo(&w).unwrap();
        assert_eq!(editor.mesh(), &source);
        editor.redo(&w).unwrap();
        assert_eq!(editor.mesh(), &result.mesh);
        let only_faces = cleanup(
            &source,
            CleanupOptions {
                duplicate_faces: true,
                unused_vertices: false,
            },
            &w,
        )
        .unwrap();
        assert_eq!(only_faces.mesh.vertices, source.vertices);
        let only_vertices = cleanup(
            &source,
            CleanupOptions {
                duplicate_faces: false,
                unused_vertices: true,
            },
            &w,
        )
        .unwrap();
        assert_eq!(only_vertices.mesh.triangles, source.triangles);
    }
    #[test]
    fn cleanup_rejects_empty_operation_invalid_source_and_cancelled_work() {
        let source = Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            triangles: vec![[0, 1, 2]],
        };
        let w = WorkControl::default();
        assert!(
            cleanup(
                &source,
                CleanupOptions {
                    duplicate_faces: false,
                    unused_vertices: false
                },
                &w
            )
            .is_err()
        );
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            cleanup(
                &source,
                CleanupOptions {
                    duplicate_faces: true,
                    unused_vertices: true
                },
                &cancelled
            )
            .is_err()
        );
        let mut bad = source.clone();
        bad.triangles[0][0] = 999;
        assert!(
            cleanup(
                &bad,
                CleanupOptions {
                    duplicate_faces: true,
                    unused_vertices: true
                },
                &w
            )
            .is_err()
        );
        assert_eq!(source.vertices.len(), 3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn square() -> Mesh {
        Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
            triangles: vec![[0, 2, 1], [1, 2, 3]],
        }
    }
    fn area(mesh: &Mesh) -> f64 {
        mesh.triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
                let u: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
                let v: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                n[0].hypot(n[1]).hypot(n[2]) / 2.
            })
            .sum()
    }
    #[test]
    fn planar_cut_splits_shared_edge_once_preserves_area_and_leaves_boundary_open() {
        let work = WorkControl::default();
        let original = square();
        let result = trim_plane(&original, [1., 0., 0.], -0.5, &work).unwrap();
        assert_eq!(result.cut_edges, 3);
        assert_eq!(result.rejected_slivers, 0);
        assert_eq!(result.mesh.triangles.len(), 3);
        assert!(result.mesh.vertices.iter().all(|p| p[0] <= 0.5));
        assert!((area(&result.mesh) - 0.5).abs() < 1e-14);
        assert_eq!(
            result
                .mesh
                .vertices
                .iter()
                .filter(|p| **p == [0.5, 0.5, 0.])
                .count(),
            1
        );
        let topology = result.mesh.topology(&work).unwrap();
        assert_eq!(topology.boundary_edges, 5);
        assert_eq!(topology.isolated_vertices, 0);
        assert_eq!(topology.inconsistent_winding_edges, 0);
        assert!(!topology.edge_closed);
        assert_eq!(original, square());
        let scaled = trim_plane(&original, [2., 0., 0.], -1., &work).unwrap();
        assert_eq!(scaled.mesh, result.mesh);
    }
    #[test]
    fn oblique_cut_of_closed_tetrahedron_produces_shared_open_boundary_without_caps() {
        let source = Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
        };
        let work = WorkControl::default();
        let cut = trim_plane(&source, [1., 1., 1.], -0.5, &work).unwrap();
        assert_eq!(cut.cut_edges, 3);
        assert_eq!(cut.mesh.vertices.len(), 4);
        assert_eq!(cut.mesh.triangles.len(), 3);
        let topology = cut.mesh.topology(&work).unwrap();
        assert_eq!(topology.boundary_edges, 3);
        assert_eq!(topology.nonmanifold_edges, 0);
        assert_eq!(topology.inconsistent_winding_edges, 0);
        assert!(
            cut.mesh
                .vertices
                .iter()
                .all(|p| p.iter().sum::<f64>() <= 0.5 + 1e-14)
        );
        assert!((area(&cut.mesh) - 0.375).abs() < 1e-14);
    }
    #[test]
    fn on_plane_vertices_empty_results_and_invalid_planes_are_explicit() {
        let work = WorkControl::default();
        let source = square();
        assert_eq!(
            trim_plane(&source, [1., 0., 0.], -1., &work).unwrap().mesh,
            source
        );
        assert!(
            trim_plane(&source, [1., 0., 0.], 0., &work)
                .unwrap()
                .mesh
                .triangles
                .is_empty()
        );
        assert!(
            trim_plane(&source, [1., 0., 0.], 1., &work)
                .unwrap()
                .mesh
                .vertices
                .is_empty()
        );
        assert!(trim_plane(&source, [0.; 3], 0., &work).is_err());
        assert!(trim_plane(&source, [1., 0., 0.], f64::NAN, &work).is_err());
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(trim_plane(&source, [1., 0., 0.], -0.5, &cancelled).is_err());
        assert_eq!(source, square());
    }
    #[test]
    fn undo_redo_restore_exact_mesh_across_one_hundred_cycles_and_invalid_proposals() {
        let work = WorkControl::default();
        let source = square();
        let cut = trim_plane(&source, [1., 0., 0.], -0.5, &work).unwrap().mesh;
        let mut editor = MeshEditor::new(source.clone(), 2, &work).unwrap();
        editor.replace(cut.clone(), &work).unwrap();
        for _ in 0..100 {
            assert!(editor.undo(&work).unwrap());
            assert_eq!(editor.mesh(), &source);
            assert!(editor.redo(&work).unwrap());
            assert_eq!(editor.mesh(), &cut);
        }
        let mut invalid = cut.clone();
        invalid.triangles[0][0] = 999;
        assert!(editor.replace(invalid, &work).is_err());
        assert_eq!(editor.history_depth(), (1, 0));
        assert_eq!(editor.mesh(), &cut);
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(editor.undo(&cancelled).is_err());
        assert_eq!(editor.mesh(), &cut);
        editor.undo(&work).unwrap();
        editor.replace(source.clone(), &work).unwrap();
        assert!(!editor.redo(&work).unwrap());
        editor.replace(source.clone(), &work).unwrap();
        let commit = editor.replace(cut, &work).unwrap();
        assert_eq!(commit.evicted_undo_steps, 1);
        assert_eq!(editor.history_depth(), (2, 0));
    }
}
