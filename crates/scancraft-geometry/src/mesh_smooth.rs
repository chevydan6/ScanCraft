//! Constrained uniform Laplacian Jacobi smoothing. No volume or intersection guarantee.
use crate::{
    mesh::{Mesh, triangle_normal},
    work::{Stage, WorkControl},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub iterations: usize,
    pub relaxation: f64,
    pub maximum_displacement_mm: f64,
    pub maximum_normal_change_degrees: f64,
    pub preserve_crease_degrees: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            iterations: 5,
            relaxation: 0.5,
            maximum_displacement_mm: 0.05,
            maximum_normal_change_degrees: 10.,
            preserve_crease_degrees: 30.,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub completed_iterations: usize,
    pub backtracking_steps: usize,
    pub locked_vertices: usize,
    pub maximum_source_vertex_displacement_mm: f64,
    pub stopped_reason: String,
}
pub fn smooth(
    source: &Mesh,
    config: &Config,
    work: &WorkControl,
) -> Result<(Mesh, Report), String> {
    if !(1..=100).contains(&config.iterations)
        || !config.relaxation.is_finite()
        || !(0. ..=1.).contains(&config.relaxation)
        || config.relaxation == 0.
        || !config.maximum_displacement_mm.is_finite()
        || config.maximum_displacement_mm < 0.
        || !config.maximum_normal_change_degrees.is_finite()
        || !(0. ..90.).contains(&config.maximum_normal_change_degrees)
        || !config.preserve_crease_degrees.is_finite()
        || !(0. ..=180.).contains(&config.preserve_crease_degrees)
    {
        return Err("invalid smoothing configuration".into());
    }
    if source.vertices.len() > 100_000 || source.triangles.len() > 200_000 {
        return Err("smoothing supports at most 100000 vertices and 200000 triangles".into());
    }
    let topology = source.topology(work)?;
    if topology.duplicate_faces > 0
        || topology.inconsistent_winding_edges > 0
        || topology.nonmanifold_edges > 0
        || topology.nonmanifold_vertices > 0
    {
        return Err("repair duplicate/winding/nonmanifold topology before smoothing".into());
    }
    let mut edges: BTreeMap<[u32; 2], Vec<usize>> = BTreeMap::new();
    let mut normals = Vec::new();
    let mut neighbors = vec![BTreeSet::new(); source.vertices.len()];
    for (i, &t) in source.triangles.iter().enumerate() {
        work.check()?;
        normals.push(triangle_normal(
            source.vertices[t[0] as usize],
            source.vertices[t[1] as usize],
            source.vertices[t[2] as usize],
        )?);
        for j in 0..3 {
            let a = t[j];
            let b = t[(j + 1) % 3];
            let mut e = [a, b];
            e.sort_unstable();
            edges.entry(e).or_default().push(i);
            neighbors[a as usize].insert(b as usize);
            neighbors[b as usize].insert(a as usize);
        }
    }
    let mut locked = vec![false; source.vertices.len()];
    let crease = config.preserve_crease_degrees.to_radians().cos();
    for (e, faces) in edges {
        work.check()?;
        if faces.len() == 1
            || (faces.len() == 2
                && (0..3)
                    .map(|a| normals[faces[0]][a] * normals[faces[1]][a])
                    .sum::<f64>()
                    + 1e-12
                    < crease)
        {
            for v in e {
                locked[v as usize] = true;
            }
        }
    }
    let mut report = Report {
        completed_iterations: 0,
        backtracking_steps: 0,
        locked_vertices: locked.iter().filter(|&&x| x).count(),
        maximum_source_vertex_displacement_mm: 0.,
        stopped_reason: "iterations_completed".into(),
    };
    let mut mesh = source.clone();
    let cosine = config.maximum_normal_change_degrees.to_radians().cos();
    for iteration in 0..config.iterations {
        work.update(Stage::Processing, iteration, config.iterations)?;
        let mut delta = vec![[0.; 3]; mesh.vertices.len()];
        for (i, ns) in neighbors.iter().enumerate() {
            work.check()?;
            if locked[i] || ns.is_empty() {
                continue;
            }
            for (neighbor_index, &j) in ns.iter().enumerate() {
                if neighbor_index % 1024 == 0 {
                    work.check()?;
                }
                for (axis, component) in delta[i].iter_mut().enumerate() {
                    *component +=
                        (mesh.vertices[j][axis] - mesh.vertices[i][axis]) / ns.len() as f64;
                }
            }
        }
        let mut accepted = None;
        let mut step = config.relaxation;
        for attempt in 0..=20 {
            work.check()?;
            let mut candidate = mesh.vertices.clone();
            let mut valid = true;
            let mut changed = false;
            for (i, p) in candidate.iter_mut().enumerate() {
                if i % 1024 == 0 {
                    work.check()?;
                }
                if !locked[i] {
                    for axis in 0..3 {
                        p[axis] += step * delta[i][axis];
                    }
                }
                changed |= *p != mesh.vertices[i];
                if !p.iter().all(|x| x.is_finite())
                    || crate::distance_mm(*p, source.vertices[i])
                        .map_or(true, |d| d > config.maximum_displacement_mm)
                {
                    valid = false;
                    break;
                }
            }
            if valid {
                for (i, t) in mesh.triangles.iter().enumerate() {
                    if i % 1024 == 0 {
                        work.check()?;
                    }
                    match triangle_normal(
                        candidate[t[0] as usize],
                        candidate[t[1] as usize],
                        candidate[t[2] as usize],
                    ) {
                        Ok(n)
                            if (0..3).map(|a| n[a] * normals[i][a]).sum::<f64>() + 1e-12
                                >= cosine => {}
                        _ => {
                            valid = false;
                            break;
                        }
                    }
                }
            }
            if valid {
                accepted = Some((candidate, changed));
                break;
            }
            if attempt < 20 {
                report.backtracking_steps += 1;
                step *= 0.5;
            }
        }
        match accepted {
            Some((vertices, true)) => {
                mesh.vertices = vertices;
                report.completed_iterations += 1;
            }
            Some((_, false)) => {
                report.stopped_reason = "stationary".into();
                break;
            }
            None => {
                report.stopped_reason = "constraints".into();
                break;
            }
        }
    }
    mesh.validate(work)?;
    for (i, &p) in mesh.vertices.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        report.maximum_source_vertex_displacement_mm = report
            .maximum_source_vertex_displacement_mm
            .max(crate::distance_mm(p, source.vertices[i])?);
    }
    Ok((mesh, report))
}
