//! Explicit planar-contour caps. Original vertices are never flattened or moved.
use crate::{
    mesh::{Mesh, triangle_normal},
    mesh_boundaries,
    work::WorkControl,
};
type P = [f64; 2];
const EPS: f64 = 1e-12;
const BUDGET: usize = 10_000_000;
#[derive(Debug)]
pub struct Filled {
    pub mesh: Mesh,
    pub added_triangles: usize,
    pub maximum_planarity_error_mm: f64,
}
fn cross(a: P, b: P, c: P) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn area(p: &[P]) -> f64 {
    p.iter()
        .zip(p.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
        .sum::<f64>()
        / 2.
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn vector_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(a: [f64; 3]) -> Result<[f64; 3], String> {
    let length = a[0].hypot(a[1]).hypot(a[2]);
    if !length.is_finite() || length <= 1e-12 {
        return Err("ill-conditioned fill contour".into());
    }
    Ok(a.map(|v| v / length))
}
struct Frame {
    origin: [f64; 3],
    scale: f64,
    u: [f64; 3],
    v: [f64; 3],
    n: [f64; 3],
}
impl Frame {
    fn project(&self, p: [f64; 3]) -> Result<([f64; 2], f64), String> {
        let d = sub(p, self.origin).map(|v| v / self.scale);
        let result = ([dot(d, self.u), dot(d, self.v)], dot(d, self.n));
        if !result.0.iter().chain([&result.1]).all(|v| v.is_finite()) {
            return Err("fill projection overflow".into());
        }
        Ok(result)
    }
}
fn frame(points: &[[f64; 3]]) -> Result<Frame, String> {
    let origin = points[0];
    let scale = points
        .iter()
        .flat_map(|p| sub(*p, origin))
        .map(f64::abs)
        .fold(0f64, f64::max);
    if !scale.is_finite() || scale == 0. {
        return Err("invalid fill scale".into());
    }
    let normalized: Vec<_> = points
        .iter()
        .map(|p| sub(*p, origin).map(|v| v / scale))
        .collect();
    let mut normal = [0.; 3];
    for (a, b) in normalized.iter().zip(normalized.iter().cycle().skip(1)) {
        let c = vector_cross(*a, *b);
        for i in 0..3 {
            normal[i] += c[i];
        }
    }
    let n = unit(normal)?;
    let u = unit(normalized[1])?;
    let v = unit(vector_cross(n, u))?;
    // Project first edge into the fitted plane for an orthonormal basis.
    let u = unit(vector_cross(v, n))?;
    Ok(Frame {
        origin,
        scale,
        u,
        v,
        n,
    })
}
fn on_segment(a: P, b: P, p: P) -> bool {
    cross(a, b, p).abs() <= EPS
        && (0..2).all(|i| p[i] >= a[i].min(b[i]) - EPS && p[i] <= a[i].max(b[i]) + EPS)
}
fn intersects(a: P, b: P, c: P, d: P) -> bool {
    let [x, y, z, w] = [
        cross(a, b, c),
        cross(a, b, d),
        cross(c, d, a),
        cross(c, d, b),
    ];
    ((x > EPS && y < -EPS || x < -EPS && y > EPS) && (z > EPS && w < -EPS || z < -EPS && w > EPS))
        || on_segment(a, b, c)
        || on_segment(a, b, d)
        || on_segment(c, d, a)
        || on_segment(c, d, b)
}
fn inside(p: P, t: [P; 3], sign: f64) -> bool {
    (0..3).all(|i| sign * cross(t[i], t[(i + 1) % 3], p) >= -EPS)
}
fn tick(count: &mut usize, work: &WorkControl) -> Result<(), String> {
    *count += 1;
    if *count > BUDGET {
        return Err("fill geometric comparison budget exceeded".into());
    }
    work.check()
}
fn overlap(mut polygon: Vec<P>, clip: [P; 3]) -> Result<bool, String> {
    let sign = area(&clip).signum();
    for i in 0..3 {
        let mut output = Vec::new();
        if polygon.is_empty() {
            return Ok(false);
        }
        for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
            let da = sign * cross(clip[i], clip[(i + 1) % 3], *a);
            let db = sign * cross(clip[i], clip[(i + 1) % 3], *b);
            if !da.is_finite() || !db.is_finite() || !(da - db).is_finite() {
                return Err("fill intersection arithmetic overflow".into());
            }
            if da >= 0. {
                output.push(*a);
            }
            if (da < 0. && db > 0.) || (da > 0. && db < 0.) {
                let ratio = da / (da - db);
                output.push(std::array::from_fn(|j| a[j] + ratio * (b[j] - a[j])));
            }
        }
        polygon = output;
    }
    let result = area(&polygon).abs();
    if !result.is_finite() {
        return Err("fill intersection area overflow".into());
    }
    Ok(result > EPS)
}
fn segment_enters_triangle(a: P, b: P, t: [P; 3]) -> bool {
    let sign = area(&t).signum();
    let mut enter = 0f64;
    let mut exit = 1f64;
    for i in 0..3 {
        let da = sign * cross(t[i], t[(i + 1) % 3], a);
        let db = sign * cross(t[i], t[(i + 1) % 3], b);
        if !da.is_finite() || !db.is_finite() || !(db - da).is_finite() {
            return true;
        }
        let delta = db - da;
        if delta == 0. {
            if da <= EPS {
                return false;
            }
        } else if delta > 0. {
            enter = enter.max((EPS - da) / delta);
        } else {
            exit = exit.min((EPS - da) / delta);
        }
    }
    enter < exit
}
/// Fill one explicitly selected simple contour (3..512 edges), within the
/// supplied planarity tolerance. Non-simple/ill-conditioned polygons fail.
/// Coplanar overlap and noncoplanar sections entering a cap's interior fail.
/// Zero-area contacts along cap boundaries are retained. Predicates use a
/// scale-normalized tolerance; this is not an exact-predicate mesh Boolean engine.
pub fn fill(
    source: &Mesh,
    id: usize,
    planarity_mm: f64,
    work: &WorkControl,
) -> Result<Filled, String> {
    if !planarity_mm.is_finite() || planarity_mm < 0. {
        return Err("invalid fill planarity tolerance".into());
    }
    let analysis = mesh_boundaries::analyze(source, work)?;
    if analysis.topology.duplicate_faces > 0
        || analysis.topology.nonmanifold_edges > 0
        || analysis.topology.nonmanifold_vertices > 0
        || analysis.topology.inconsistent_winding_edges > 0
        || !analysis.ambiguous_components.is_empty()
    {
        return Err("resolve ambiguous/nonmanifold topology before filling".into());
    }
    let contour = analysis
        .loops
        .get(id)
        .ok_or("unknown boundary contour ID")?;
    if !(3..=512).contains(&contour.vertices.len()) {
        return Err("fill requires 3..512 contour edges".into());
    }
    let mut ids = contour.vertices.clone();
    ids.reverse();
    let points: Vec<_> = ids.iter().map(|i| source.vertices[*i as usize]).collect();
    let basis = frame(&points)?;
    let projected: Vec<_> = points
        .iter()
        .map(|p| basis.project(*p))
        .collect::<Result<_, _>>()?;
    let maximum_planarity_error_mm = projected
        .iter()
        .map(|p| p.1.abs() * basis.scale)
        .fold(0f64, f64::max);
    if !maximum_planarity_error_mm.is_finite() || maximum_planarity_error_mm > planarity_mm {
        return Err("contour exceeds explicit planarity tolerance".into());
    }
    let p: Vec<_> = projected.iter().map(|p| p.0).collect();
    let sign = area(&p).signum();
    if area(&p).abs() <= EPS {
        return Err("fill contour has negligible projected area".into());
    }
    let n = ids.len();
    let mut comparisons = 0;
    for i in 0..n {
        for j in i + 1..n {
            tick(&mut comparisons, work)?;
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            if intersects(p[i], p[(i + 1) % n], p[j], p[(j + 1) % n]) {
                return Err("fill contour self-intersects or touches itself".into());
            }
        }
    }
    let mut polygon: Vec<_> = (0..n).collect();
    let mut caps = Vec::new();
    while polygon.len() > 3 {
        let mut ear = None;
        for i in 0..polygon.len() {
            let t = [
                polygon[(i + polygon.len() - 1) % polygon.len()],
                polygon[i],
                polygon[(i + 1) % polygon.len()],
            ];
            tick(&mut comparisons, work)?;
            if sign * cross(p[t[0]], p[t[1]], p[t[2]]) <= EPS {
                continue;
            }
            let mut occupied = false;
            for &v in &polygon {
                tick(&mut comparisons, work)?;
                if !t.contains(&v) && inside(p[v], t.map(|k| p[k]), sign) {
                    occupied = true;
                    break;
                }
            }
            if !occupied {
                ear = Some((i, t));
                break;
            }
        }
        let (i, t) = ear.ok_or("no stable triangulation of selected contour")?;
        caps.push(t.map(|i| ids[i]));
        polygon.remove(i);
    }
    if sign * cross(p[polygon[0]], p[polygon[1]], p[polygon[2]]) <= EPS {
        return Err("final fill triangle is ill-conditioned in projection".into());
    }
    caps.push([ids[polygon[0]], ids[polygon[1]], ids[polygon[2]]]);
    if source.triangles.len() + caps.len() > 2_000_000 {
        return Err("fill exceeds triangle budget".into());
    }
    for cap in &caps {
        let cap_points = cap.map(|i| source.vertices[i as usize]);
        triangle_normal(cap_points[0], cap_points[1], cap_points[2])?;
        let f = frame(&cap_points)?;
        let target = cap_points.map(|p| f.project(p).map(|v| v.0));
        let target = [target[0].clone()?, target[1].clone()?, target[2].clone()?];
        for face in &source.triangles {
            tick(&mut comparisons, work)?;
            let vertices = face.map(|i| source.vertices[i as usize]);
            if (0..3).any(|axis| {
                let lo = cap_points
                    .iter()
                    .map(|p| p[axis])
                    .fold(f64::INFINITY, f64::min);
                let hi = cap_points
                    .iter()
                    .map(|p| p[axis])
                    .fold(f64::NEG_INFINITY, f64::max);
                vertices.iter().all(|p| p[axis] < lo) || vertices.iter().all(|p| p[axis] > hi)
            }) {
                continue;
            }
            let projected: Vec<_> = vertices
                .iter()
                .map(|p| f.project(*p))
                .collect::<Result<_, _>>()?;
            let low = projected.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let high = projected
                .iter()
                .map(|p| p.1)
                .fold(f64::NEG_INFINITY, f64::max);
            if low > EPS || high < -EPS {
                continue;
            }
            if low >= -EPS && high <= EPS {
                if overlap(projected.iter().map(|p| p.0).collect(), target)? {
                    return Err("fill overlaps existing coplanar faces".into());
                }
            } else {
                let mut contacts: Vec<P> = projected
                    .iter()
                    .filter(|p| p.1.abs() <= EPS)
                    .map(|p| p.0)
                    .collect();
                for i in 0..3 {
                    let (a, b) = (projected[i], projected[(i + 1) % 3]);
                    if (a.1 < -EPS && b.1 > EPS) || (a.1 > EPS && b.1 < -EPS) {
                        let ratio = a.1 / (a.1 - b.1);
                        contacts.push(std::array::from_fn(|j| a.0[j] + ratio * (b.0[j] - a.0[j])));
                    }
                }
                if contacts
                    .iter()
                    .any(|&p| segment_enters_triangle(p, p, target))
                    || contacts.iter().enumerate().any(|(i, &a)| {
                        contacts[i + 1..]
                            .iter()
                            .any(|&b| segment_enters_triangle(a, b, target))
                    })
                {
                    return Err("face intersection with fill interior".into());
                }
            }
        }
    }
    let mut mesh = source.clone();
    let added_triangles = caps.len();
    mesh.triangles.extend(caps);
    let topology = mesh.topology(work)?;
    if topology.nonmanifold_edges > 0
        || topology.nonmanifold_vertices > 0
        || topology.inconsistent_winding_edges > 0
        || topology.duplicate_faces > 0
        || topology.boundary_edges + ids.len() != analysis.boundary_edges
    {
        return Err("fill failed edge topology checks".into());
    }
    Ok(Filled {
        mesh,
        added_triangles,
        maximum_planarity_error_mm,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ring() -> Mesh {
        serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../../evidence/mesh-boundary-ring-input.json"
            ))
            .unwrap()["mesh"]
                .clone(),
        )
        .unwrap()
    }
    #[test]
    fn selected_inner_contour_fills_without_moving_vertices_and_outer_overlap_fails() {
        let mesh = ring();
        let original = mesh.clone();
        let work = WorkControl::default();
        let result = fill(&mesh, 1, 0.001, &work).unwrap();
        assert_eq!(result.added_triangles, 2);
        assert_eq!(result.mesh.vertices, mesh.vertices);
        assert_eq!(result.mesh.topology(&work).unwrap().boundary_edges, 4);
        assert_eq!(mesh, original);
        assert!(
            fill(&mesh, 0, 0.001, &work)
                .unwrap_err()
                .contains("overlaps")
        );
        assert!(fill(&mesh, 99, 0.001, &work).is_err());
    }
    #[test]
    fn concave_prism_cap_and_tetrahedron_missing_face_have_consistent_winding() {
        let polygon = [[0., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 2.], [0., 2.]];
        let mut mesh = Mesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
        };
        for z in [0., 1.] {
            for p in polygon {
                mesh.vertices.push([p[0], p[1], z]);
            }
        }
        for i in 0..6u32 {
            let j = (i + 1) % 6;
            mesh.triangles.extend([[i, j, 6 + i], [j, 6 + j, 6 + i]]);
        }
        let work = WorkControl::default();
        let result = fill(&mesh, 1, 0.001, &work).unwrap();
        assert_eq!(result.added_triangles, 4);
        let added_area: f64 = result.mesh.triangles[mesh.triangles.len()..]
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| result.mesh.vertices[i as usize]);
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.
            })
            .sum();
        assert!((added_area - 3.).abs() < 1e-12);
        assert_eq!(result.mesh.topology(&work).unwrap().boundary_edges, 6);
        let mesh = Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3]],
        };
        assert!(
            fill(&mesh, 0, 0.001, &work)
                .unwrap()
                .mesh
                .topology(&work)
                .unwrap()
                .edge_closed
        );
    }
    #[test]
    fn invalid_tolerance_nonplanar_contour_crossing_face_and_cancellation_fail() {
        let mut mesh = ring();
        mesh.vertices[4][2] = 0.2;
        assert!(
            fill(&mesh, 1, 0.001, &WorkControl::default())
                .unwrap_err()
                .contains("planarity")
        );
        for tolerance in [-1., f64::NAN] {
            assert!(fill(&ring(), 1, tolerance, &WorkControl::default()).is_err());
        }
        let mut mesh = ring();
        let base = mesh.vertices.len() as u32;
        mesh.vertices
            .extend([[5., 4., -1.], [5., 6., 1.], [5., 6., -1.]]);
        mesh.triangles.push([base, base + 1, base + 2]);
        assert!(
            fill(&mesh, 1, 0.001, &WorkControl::default())
                .unwrap_err()
                .contains("intersection")
        );
        let work = WorkControl::default();
        work.cancel();
        assert!(fill(&ring(), 1, 0.001, &work).is_err());
    }
    #[test]
    fn segment_interior_predicate_handles_vertex_crossing_and_shared_edges() {
        let t = [[0., 0.], [1., 0.], [0., 1.]];
        assert!(segment_enters_triangle([-100., -100.], [1., 1.], t));
        assert!(!segment_enters_triangle([0., 0.], [1., 0.], t));
        assert!(!segment_enters_triangle([2., 2.], [3., 3.], t));
        assert!(segment_enters_triangle([0.2, 0.2], [0.2, 0.2], t));
    }
}
