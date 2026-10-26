//! Edge-consistent face winding with an explicit per-component sign choice.
//! No outward/inward classification, welding or coordinate modification.
use crate::{
    mesh::Mesh,
    work::{Stage, WorkControl},
};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Serialize)]
pub struct ComponentOrientation {
    pub id: usize,
    pub first_triangle: usize,
    pub triangles: usize,
    pub reverse_seed: bool,
    pub faces_flipped: usize,
}
pub struct Oriented {
    pub mesh: Mesh,
    pub faces_flipped: usize,
    pub components: Vec<ComponentOrientation>,
}
/// Assigns parity constraints across indexed edges. Each component keeps its
/// first triangle's sign unless explicitly selected for reversal. Nonorientable
/// constraints, >2 faces per edge and duplicate faces fail without partial output.
pub fn orient(
    source: &Mesh,
    reverse_components: &[usize],
    work: &WorkControl,
) -> Result<Oriented, String> {
    source.validate(work)?;
    if reverse_components.len() > 100_000 {
        return Err("orientation component selection exceeds budget".into());
    }
    let chosen: HashSet<_> = reverse_components.iter().copied().collect();
    if chosen.len() != reverse_components.len() {
        return Err("duplicate orientation component ID".into());
    }
    let count = source.triangles.len();
    let mut neighbors = vec![[(0u32, false); 3]; count];
    let mut degrees = vec![0u8; count];
    let mut edges: HashMap<[u32; 2], (u32, bool, bool)> = HashMap::new();
    let mut faces = HashSet::new();
    for (i, t) in source.triangles.iter().enumerate() {
        work.update(Stage::Indexing, i, count)?;
        let mut sorted = *t;
        sorted.sort_unstable();
        if !faces.insert(sorted) {
            return Err("remove duplicate faces before orientation repair".into());
        }
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let key = if a < b { [a, b] } else { [b, a] };
            let forward = a < b;
            if let Some((previous, direction, paired)) = edges.get_mut(&key) {
                if *paired {
                    return Err("nonmanifold edge prevents orientation repair".into());
                }
                let other = *previous as usize;
                let parity = forward == *direction;
                neighbors[i][degrees[i] as usize] = (*previous, parity);
                degrees[i] += 1;
                neighbors[other][degrees[other] as usize] = (i as u32, parity);
                degrees[other] += 1;
                *paired = true;
            } else {
                edges.insert(key, (i as u32, forward, false));
            }
        }
    }
    drop(edges);
    drop(faces);
    let mut flips = vec![None; count];
    let mut components = Vec::new();
    let mut queue = VecDeque::new();
    let mut component_of = vec![0usize; count];
    let mut processed = 0;
    for first in 0..count {
        work.check()?;
        if flips[first].is_some() {
            continue;
        }
        if components.len() == 100_000 {
            return Err("orientation exceeds component budget".into());
        }
        let id = components.len();
        flips[first] = Some(false);
        queue.push_back(first);
        let mut triangles = 0;
        while let Some(face) = queue.pop_front() {
            work.update(Stage::Processing, processed, count)?;
            processed += 1;
            triangles += 1;
            component_of[face] = id;
            let flip = flips[face].unwrap();
            for &(neighbor, parity) in &neighbors[face][..degrees[face] as usize] {
                let neighbor = neighbor as usize;
                let required = flip ^ parity;
                match flips[neighbor] {
                    Some(previous) if previous != required => {
                        return Err(
                            "nonorientable face constraints; no consistent winding exists".into(),
                        );
                    }
                    Some(_) => {}
                    None => {
                        flips[neighbor] = Some(required);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        components.push(ComponentOrientation {
            id,
            first_triangle: first,
            triangles,
            reverse_seed: chosen.contains(&id),
            faces_flipped: 0,
        });
    }
    if chosen.iter().any(|id| *id >= components.len()) {
        return Err("unknown orientation component ID".into());
    }
    let mut mesh = source.clone();
    let mut faces_flipped = 0;
    for (i, t) in mesh.triangles.iter_mut().enumerate() {
        work.check()?;
        let component = component_of[i];
        if flips[i].unwrap() ^ components[component].reverse_seed {
            t.swap(1, 2);
            faces_flipped += 1;
            components[component].faces_flipped += 1;
        }
    }
    let topology = mesh.topology(work)?;
    if topology.inconsistent_winding_edges != 0
        || topology.nonmanifold_edges != 0
        || topology.duplicate_faces != 0
    {
        return Err("orientation result failed edge consistency checks".into());
    }
    work.check()?;
    Ok(Oriented {
        mesh,
        faces_flipped,
        components,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn patch() -> Mesh {
        Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
            triangles: vec![[0, 1, 2], [1, 2, 3]],
        }
    }
    #[test]
    fn repairs_parity_preserving_seed_coordinates_and_is_idempotent() {
        let source = patch();
        let original = source.clone();
        let work = WorkControl::default();
        let p = orient(&source, &[], &work).unwrap();
        assert_eq!(p.faces_flipped, 1);
        assert_eq!(p.mesh.vertices, source.vertices);
        assert_eq!(p.mesh.triangles[0], source.triangles[0]);
        assert_eq!(p.mesh.triangles[1], [1, 3, 2]);
        assert_eq!(source, original);
        assert_eq!(orient(&p.mesh, &[], &work).unwrap().faces_flipped, 0);
        let reversed = orient(&source, &[0], &work).unwrap();
        assert_eq!(reversed.mesh.triangles, vec![[0, 2, 1], [1, 2, 3]]);
    }
    #[test]
    fn explicit_components_match_first_face_connectivity_and_reject_invalid_selections() {
        let mut source = patch();
        source
            .vertices
            .extend([[3., 0., 0.], [4., 0., 0.], [3., 1., 0.]]);
        source.triangles.push([4, 5, 6]);
        let work = WorkControl::default();
        let p = orient(&source, &[1], &work).unwrap();
        assert_eq!(p.components.len(), 2);
        assert_eq!(p.components[1].first_triangle, 2);
        assert_eq!(p.mesh.triangles[2], [4, 6, 5]);
        assert_eq!(p.faces_flipped, 2);
        assert!(orient(&source, &[1, 1], &work).is_err());
        assert!(orient(&source, &[2], &work).is_err());
        let mut duplicate = patch();
        duplicate.triangles.push([2, 1, 0]);
        assert!(orient(&duplicate, &[], &work).is_err());
        let mut nonmanifold = patch();
        nonmanifold.vertices.push([0., 0., 1.]);
        nonmanifold.triangles.push([1, 2, 4]);
        assert!(orient(&nonmanifold, &[], &work).is_err());
        work.cancel();
        assert!(orient(&source, &[], &work).is_err());
    }
    #[test]
    fn mobius_strip_is_refused_instead_of_partially_oriented() {
        let mut mesh = Mesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
        };
        let n = 8u32;
        for i in 0..n {
            let theta = f64::from(i) * std::f64::consts::TAU / f64::from(n);
            for u in [-0.2, 0.2] {
                mesh.vertices.push([
                    (1. + u * (theta / 2.).cos()) * theta.cos(),
                    (1. + u * (theta / 2.).cos()) * theta.sin(),
                    u * (theta / 2.).sin(),
                ]);
            }
        }
        for i in 0..n {
            let (c, d) = if i + 1 == n {
                (1, 0)
            } else {
                (2 * i + 2, 2 * i + 3)
            };
            mesh.triangles
                .extend([[2 * i, c, 2 * i + 1], [2 * i + 1, c, d]]);
        }
        let original = mesh.clone();
        let error = orient(&mesh, &[], &WorkControl::default()).err().unwrap();
        assert!(error.contains("nonorientable"));
        assert_eq!(mesh, original);
    }
}
