//! Angle-weighted normals on explicit indexed surface adjacency; no geometry edits.
use crate::{
    mesh::{Mesh, triangle_normal},
    work::{Stage, WorkControl},
};
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub maximum_deviation_rad: f64,
    #[serde(default)]
    pub viewpoint_mm: Option<[f64; 3]>,
}
pub struct Report {
    pub normals: Vec<Option<[f64; 3]>>,
    pub estimated: usize,
    pub unsupported: usize,
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let length = v[0].hypot(v[1]).hypot(v[2]);
    (length.is_finite() && length > 1e-12).then(|| v.map(|x| x / length))
}
pub fn estimate(mesh: &Mesh, config: Config, work: &WorkControl) -> Result<Report, String> {
    work.check()?;
    if !config.maximum_deviation_rad.is_finite()
        || config.maximum_deviation_rad <= 0.
        || config.maximum_deviation_rad > std::f64::consts::FRAC_PI_2
        || config
            .viewpoint_mm
            .is_some_and(|v| !v.iter().all(|x| x.is_finite()))
    {
        return Err("invalid surface-normal deviation or viewpoint".into());
    }
    mesh.validate(work)?;
    let topology = mesh.topology(work)?;
    if topology.duplicate_faces > 0
        || topology.nonmanifold_edges > 0
        || topology.inconsistent_winding_edges > 0
    {
        return Err(
            "surface normals require unique faces and consistent manifold-edge winding".into(),
        );
    }
    let mut sums = vec![[0.; 3]; mesh.vertices.len()];
    for (i, face) in mesh.triangles.iter().enumerate() {
        work.update(Stage::Processing, i, mesh.triangles.len())?;
        let p = face.map(|v| mesh.vertices[v as usize]);
        let normal = triangle_normal(p[0], p[1], p[2])?;
        for corner in 0..3 {
            let a = std::array::from_fn(|j| p[(corner + 1) % 3][j] - p[corner][j]);
            let b = std::array::from_fn(|j| p[(corner + 2) % 3][j] - p[corner][j]);
            // Normalize by component scale before length to retain tiny valid triangles.
            let normalized = |v: [f64; 3]| {
                let scale = v.iter().map(|x| x.abs()).fold(0f64, f64::max);
                unit(v.map(|x| x / scale)).ok_or("invalid normal corner direction")
            };
            let angle = dot(normalized(a)?, normalized(b)?).clamp(-1., 1.).acos();
            let sum = &mut sums[face[corner] as usize];
            for j in 0..3 {
                sum[j] += angle * normal[j];
            }
        }
    }
    let mut normals: Vec<_> = sums.into_iter().map(unit).collect();
    for id in topology.nonmanifold_vertex_ids {
        normals[id as usize] = None;
    }
    let minimum_dot = config.maximum_deviation_rad.cos();
    for (i, face) in mesh.triangles.iter().enumerate() {
        work.update(Stage::Processing, i, mesh.triangles.len())?;
        let p = face.map(|v| mesh.vertices[v as usize]);
        let face_normal = triangle_normal(p[0], p[1], p[2])?;
        for vertex in face {
            let normal = &mut normals[*vertex as usize];
            if normal.is_some_and(|n| dot(n, face_normal) < minimum_dot - 1e-12) {
                *normal = None;
            }
        }
    }
    if let Some(viewpoint) = config.viewpoint_mm {
        for (i, (normal, point)) in normals.iter_mut().zip(&mesh.vertices).enumerate() {
            if i % 1024 == 0 {
                work.check()?;
            }
            if let Some(n) = normal {
                let direction = std::array::from_fn(|j| viewpoint[j] - point[j]);
                let facing = unit(direction).map(|v| dot(*n, v));
                match facing {
                    Some(value) if value.abs() > 1e-12 => {
                        if value < 0. {
                            *n = n.map(|x| -x);
                        }
                    }
                    _ => {
                        *normal = None;
                    }
                }
            }
        }
    }
    work.check()?;
    let estimated = normals.iter().filter(|n| n.is_some()).count();
    Ok(Report {
        unsupported: normals.len() - estimated,
        normals,
        estimated,
    })
}
