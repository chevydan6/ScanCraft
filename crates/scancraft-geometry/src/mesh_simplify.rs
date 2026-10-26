//! Constrained shortest-edge decimation with cumulative source-vertex bounds.
use crate::{
    mesh::{Mesh, triangle_normal},
    work::{Stage, WorkControl},
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, BinaryHeap},
};
const MAX_VERTICES: usize = 100_000;
const MAX_FACES: usize = 200_000;
const MAX_EVENTS: usize = 1_000_000;
const MAX_QUEUE: usize = 1_000_000;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub target_triangles: usize,
    pub maximum_displacement_mm: f64,
    pub maximum_normal_change_degrees: f64,
    pub preserve_crease_degrees: f64,
    pub maximum_collapses: usize,
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.target_triangles == 0
            || !self.maximum_displacement_mm.is_finite()
            || self.maximum_displacement_mm < 0.
            || !self.maximum_normal_change_degrees.is_finite()
            || !(0. ..90.).contains(&self.maximum_normal_change_degrees)
            || !self.preserve_crease_degrees.is_finite()
            || !(0. ..=180.).contains(&self.preserve_crease_degrees)
            || !(1..=100_000).contains(&self.maximum_collapses)
        {
            return Err(
                "invalid simplification target, geometric bounds or collapse budget".into(),
            );
        }
        Ok(())
    }
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub original_triangles: usize,
    pub result_triangles: usize,
    pub collapsed_edges: usize,
    pub locked_vertices: usize,
    pub candidate_events: usize,
    pub stale_events: usize,
    pub rejected_candidates: BTreeMap<String, usize>,
    pub maximum_source_vertex_displacement_mm: f64,
    pub target_reached: bool,
    pub stopped_reason: String,
}
pub struct Simplified {
    pub mesh: Mesh,
    pub source_to_output: Vec<u32>,
    pub report: Report,
}
#[derive(Clone)]
struct Entry {
    cost: f64,
    edge: [u32; 2],
    epochs: [u64; 2],
}
impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Entry {}
impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.edge.cmp(&self.edge))
            .then_with(|| other.epochs.cmp(&self.epochs))
    }
}
fn key(a: u32, b: u32) -> [u32; 2] {
    if a < b { [a, b] } else { [b, a] }
}
fn edge_map(mesh: &Mesh, work: &WorkControl) -> Result<BTreeMap<[u32; 2], Vec<usize>>, String> {
    let mut map: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (i, t) in mesh.triangles.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        for e in 0..3 {
            map.entry(key(t[e], t[(e + 1) % 3])).or_default().push(i);
        }
    }
    Ok(map)
}
struct State {
    positions: Vec<[f64; 3]>,
    faces: Vec<Option<[u32; 3]>>,
    incident: Vec<BTreeSet<usize>>,
    clusters: Vec<Vec<u32>>,
    alive: Vec<bool>,
    epochs: Vec<u64>,
    locked: Vec<bool>,
}
impl State {
    fn neighbors(&self, v: u32) -> BTreeSet<u32> {
        self.incident[v as usize]
            .iter()
            .flat_map(|&i| self.faces[i].unwrap())
            .filter(|&n| n != v)
            .collect()
    }
    fn push(&self, edge: [u32; 2], heap: &mut BinaryHeap<Entry>) -> bool {
        let [u, v] = edge.map(|x| x as usize);
        if !self.alive[u] || !self.alive[v] || self.locked[u] || self.locked[v] {
            return true;
        }
        if self.incident[u]
            .intersection(&self.incident[v])
            .take(3)
            .count()
            != 2
        {
            return true;
        }
        if heap.len() == MAX_QUEUE {
            return false;
        }
        if let Ok(cost) = crate::distance_mm(self.positions[u], self.positions[v]) {
            heap.push(Entry {
                cost,
                edge,
                epochs: [self.epochs[u], self.epochs[v]],
            });
        }
        true
    }
}
struct Proposal {
    position: [f64; 3],
    faces: Vec<(usize, Option<[u32; 3]>)>,
    affected: BTreeSet<u32>,
}
fn propose(
    state: &State,
    source: &Mesh,
    normals: &[[f64; 3]],
    edge: [u32; 2],
    config: &Config,
    work: &WorkControl,
) -> Result<Result<Proposal, &'static str>, String> {
    let [u, v] = edge;
    if state.incident[u as usize].len() > 256 || state.incident[v as usize].len() > 256 {
        return Ok(Err("local_valence_budget"));
    }
    let faces: BTreeSet<_> = state.incident[u as usize]
        .union(&state.incident[v as usize])
        .copied()
        .collect();
    if faces.len() > 256 {
        return Ok(Err("local_valence_budget"));
    }
    let edge_faces: Vec<_> = state.incident[u as usize]
        .intersection(&state.incident[v as usize])
        .copied()
        .collect();
    if edge_faces.len() != 2 {
        return Ok(Err("edge_incidence"));
    }
    let nu = state.neighbors(u);
    let nv = state.neighbors(v);
    let common: BTreeSet<_> = nu.intersection(&nv).copied().collect();
    let opposite: BTreeSet<_> = edge_faces
        .iter()
        .flat_map(|&i| state.faces[i].unwrap())
        .filter(|&n| n != u && n != v)
        .collect();
    if common != opposite {
        return Ok(Err("link_condition"));
    }
    let mut changes = Vec::new();
    let mut keys = BTreeSet::new();
    for &i in &faces {
        let t = state.faces[i].unwrap();
        if t.contains(&u) && t.contains(&v) {
            changes.push((i, None));
            continue;
        }
        let mapped = t.map(|x| if x == v { u } else { x });
        let mut sorted = mapped;
        sorted.sort_unstable();
        if !keys.insert(sorted) {
            return Ok(Err("duplicate_face"));
        }
        changes.push((i, Some(mapped)));
    }
    let a = state.positions[u as usize];
    let b = state.positions[v as usize];
    let midpoint = std::array::from_fn(|axis| a[axis] / 2. + b[axis] / 2.);
    let cosine = config.maximum_normal_change_degrees.to_radians().cos();
    let mut best = None;
    let mut within_displacement = false;
    for position in [midpoint, a, b] {
        work.check()?;
        let mut maximum = 0_f64;
        let mut allowed = true;
        for (i, &original) in state.clusters[u as usize]
            .iter()
            .chain(&state.clusters[v as usize])
            .enumerate()
        {
            if i % 1024 == 0 {
                work.check()?;
            }
            match crate::distance_mm(position, source.vertices[original as usize]) {
                Ok(distance) if distance <= config.maximum_displacement_mm => {
                    maximum = maximum.max(distance)
                }
                _ => {
                    allowed = false;
                    break;
                }
            }
        }
        if !allowed {
            continue;
        }
        within_displacement = true;
        for &(i, t) in &changes {
            if let Some(t) = t {
                let p = t.map(|index| {
                    if index == u {
                        position
                    } else {
                        state.positions[index as usize]
                    }
                });
                let Ok(normal) = triangle_normal(p[0], p[1], p[2]) else {
                    allowed = false;
                    break;
                };
                let dot = (0..3)
                    .map(|axis| normal[axis] * normals[i][axis])
                    .sum::<f64>();
                if dot + 1e-12 < cosine {
                    allowed = false;
                    break;
                }
            }
        }
        if allowed && best.as_ref().is_none_or(|(_, old)| maximum < *old) {
            best = Some((position, maximum));
        }
    }
    let Some((position, _)) = best else {
        return Ok(Err(if within_displacement {
            "normal_or_degeneracy"
        } else {
            "source_displacement"
        }));
    };
    let mut affected: BTreeSet<_> = nu.union(&nv).copied().collect();
    affected.insert(u);
    affected.insert(v);
    Ok(Ok(Proposal {
        position,
        faces: changes,
        affected,
    }))
}
pub fn simplify(source: &Mesh, config: &Config, work: &WorkControl) -> Result<Simplified, String> {
    config.validate()?;
    source.validate(work)?;
    if source.vertices.len() > MAX_VERTICES || source.triangles.len() > MAX_FACES {
        return Err(
            "simplification currently supports at most 100000 vertices and 200000 triangles".into(),
        );
    }
    let topology = source.topology(work)?;
    if topology.duplicate_faces > 0
        || topology.nonmanifold_edges > 0
        || topology.inconsistent_winding_edges > 0
        || topology.nonmanifold_vertices > 0
    {
        return Err("repair duplicate/winding/nonmanifold topology before simplification".into());
    }
    let edges = edge_map(source, work)?;
    let mut normals = Vec::with_capacity(source.triangles.len());
    let mut incident = vec![BTreeSet::new(); source.vertices.len()];
    for (i, t) in source.triangles.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        normals.push(triangle_normal(
            source.vertices[t[0] as usize],
            source.vertices[t[1] as usize],
            source.vertices[t[2] as usize],
        )?);
        for &v in t {
            incident[v as usize].insert(i);
        }
    }
    let mut locked = vec![false; source.vertices.len()];
    let mut protected = BTreeSet::new();
    let cosine = config.preserve_crease_degrees.to_radians().cos();
    for (i, (&edge, faces)) in edges.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let lock = faces.len() == 1
            || (faces.len() == 2
                && (0..3)
                    .map(|a| normals[faces[0]][a] * normals[faces[1]][a])
                    .sum::<f64>()
                    + 1e-12
                    < cosine);
        if lock {
            for v in edge {
                locked[v as usize] = true;
            }
            protected.insert(edge);
        }
    }
    let mut report = Report {
        original_triangles: source.triangles.len(),
        result_triangles: source.triangles.len(),
        collapsed_edges: 0,
        locked_vertices: locked.iter().filter(|&&v| v).count(),
        candidate_events: 0,
        stale_events: 0,
        rejected_candidates: BTreeMap::new(),
        maximum_source_vertex_displacement_mm: 0.,
        target_reached: false,
        stopped_reason: String::new(),
    };
    let mut state = State {
        positions: source.vertices.clone(),
        faces: source.triangles.iter().copied().map(Some).collect(),
        incident,
        clusters: (0..source.vertices.len()).map(|v| vec![v as u32]).collect(),
        alive: vec![true; source.vertices.len()],
        epochs: vec![0; source.vertices.len()],
        locked,
    };
    let mut heap = BinaryHeap::new();
    for (i, &edge) in edges.keys().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        state.push(edge, &mut heap);
    }
    let mut face_count = source.triangles.len();
    let mut reason = "constraints";
    while face_count > config.target_triangles {
        work.update(
            Stage::Processing,
            report.collapsed_edges,
            config.maximum_collapses,
        )?;
        if report.collapsed_edges == config.maximum_collapses {
            reason = "collapse_budget";
            break;
        }
        if report.candidate_events + report.stale_events == MAX_EVENTS {
            reason = "candidate_budget";
            break;
        }
        let Some(entry) = heap.pop() else {
            break;
        };
        let [u, v] = entry.edge;
        let [a, b] = [u as usize, v as usize];
        if !state.alive[a] || !state.alive[b] || entry.epochs != [state.epochs[a], state.epochs[b]]
        {
            report.stale_events += 1;
            continue;
        }
        report.candidate_events += 1;
        let proposal = match propose(&state, source, &normals, entry.edge, config, work)? {
            Ok(p) => p,
            Err(cause) => {
                *report.rejected_candidates.entry(cause.into()).or_default() += 1;
                continue;
            }
        };
        state.positions[a] = proposal.position;
        for (i, next) in proposal.faces {
            for old in state.faces[i].unwrap() {
                state.incident[old as usize].remove(&i);
            }
            state.faces[i] = next;
            if let Some(t) = next {
                for node in t {
                    state.incident[node as usize].insert(i);
                }
            }
        }
        state.alive[b] = false;
        let moved = std::mem::take(&mut state.clusters[b]);
        state.clusters[a].extend(moved);
        face_count -= 2;
        report.collapsed_edges += 1;
        for &node in &proposal.affected {
            state.epochs[node as usize] += 1;
        }
        let mut update = BTreeSet::new();
        for &node in &proposal.affected {
            work.check()?;
            if state.alive[node as usize]
                && !state.locked[node as usize]
                && state.incident[node as usize].len() <= 256
            {
                for other in state.neighbors(node) {
                    update.insert(key(node, other));
                }
            }
        }
        let mut full = false;
        for edge in update {
            if !state.push(edge, &mut heap) {
                full = true;
                break;
            }
        }
        if full {
            reason = "queue_budget";
            break;
        }
    }
    let mut remap = vec![0; source.vertices.len()];
    let mut vertices = Vec::new();
    for (i, &alive) in state.alive.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        if alive {
            remap[i] = vertices.len() as u32;
            vertices.push(state.positions[i]);
        }
    }
    let mut source_to_output = vec![0; source.vertices.len()];
    for (i, cluster) in state.clusters.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        for &original in cluster {
            source_to_output[original as usize] = remap[i];
        }
    }
    let triangles = state
        .faces
        .iter()
        .flatten()
        .map(|t| t.map(|v| remap[v as usize]))
        .collect();
    let mesh = Mesh {
        vertices,
        triangles,
    };
    let result_topology = mesh.topology(work)?;
    if result_topology.duplicate_faces > 0
        || result_topology.nonmanifold_edges > 0
        || result_topology.inconsistent_winding_edges > 0
        || result_topology.nonmanifold_vertices > 0
    {
        return Err("simplification result failed indexed topology validation".into());
    }
    let final_edges = edge_map(&mesh, work)?;
    let boundaries: BTreeSet<_> = final_edges
        .iter()
        .filter(|(_, faces)| faces.len() == 1)
        .map(|(&edge, _)| edge)
        .collect();
    let expected: BTreeSet<_> = edges
        .iter()
        .filter(|(_, faces)| faces.len() == 1)
        .map(|(&[a, b], _)| key(source_to_output[a as usize], source_to_output[b as usize]))
        .collect();
    if boundaries != expected {
        return Err("simplification changed the indexed boundary graph".into());
    }
    for (i, [a, b]) in protected.into_iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        if !final_edges.contains_key(&key(
            source_to_output[a as usize],
            source_to_output[b as usize],
        )) {
            return Err("simplification removed a protected feature edge".into());
        }
    }
    let face_keys: BTreeSet<_> = mesh
        .triangles
        .iter()
        .map(|&t| {
            let mut t = t;
            t.sort_unstable();
            t
        })
        .collect();
    let mut used = vec![false; mesh.vertices.len()];
    for t in &mesh.triangles {
        for &v in t {
            used[v as usize] = true;
        }
    }
    for (i, t) in source.triangles.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let mut image = t.map(|v| source_to_output[v as usize]);
        image.sort_unstable();
        let valid = if image[0] == image[2] {
            used[image[0] as usize]
        } else if image[0] == image[1] || image[1] == image[2] {
            final_edges.contains_key(&[image[0], image[2]])
        } else {
            face_keys.contains(&image)
        };
        if !valid {
            return Err("simplification lost a mapped source-face image".into());
        }
    }
    let euler_before = (source.vertices.len() - topology.isolated_vertices) as i64
        - edges.len() as i64
        + source.triangles.len() as i64;
    let euler_after = (mesh.vertices.len() - result_topology.isolated_vertices) as i64
        - final_edges.len() as i64
        + mesh.triangles.len() as i64;
    if euler_before != euler_after
        || crate::mesh_components::analyze(source, work)?
            .components
            .len()
            != crate::mesh_components::analyze(&mesh, work)?
                .components
                .len()
    {
        return Err(
            "simplification changed indexed Euler characteristic or component count".into(),
        );
    }
    for (i, &v) in source_to_output.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let displacement = crate::distance_mm(source.vertices[i], mesh.vertices[v as usize])?;
        if displacement > config.maximum_displacement_mm {
            return Err("simplification exceeded cumulative source-vertex displacement".into());
        }
        report.maximum_source_vertex_displacement_mm = report
            .maximum_source_vertex_displacement_mm
            .max(displacement);
    }
    report.result_triangles = mesh.triangles.len();
    report.target_reached = mesh.triangles.len() <= config.target_triangles;
    report.stopped_reason = if report.target_reached {
        "target_met"
    } else {
        reason
    }
    .into();
    work.check()?;
    Ok(Simplified {
        mesh,
        source_to_output,
        report,
    })
}
