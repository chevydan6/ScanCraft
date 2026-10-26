//! Independent local geometric cylinder fitting. Not robust or physically qualified.
use crate::{
    measurement,
    work::{Stage, WorkControl},
};
use nalgebra::{DMatrix, DVector, Matrix3, SMatrix, SVector, Vector3};
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cylinder {
    pub axis_point_mm: [f64; 3],
    pub axis_direction: [f64; 3],
    pub radius_mm: f64,
    pub diameter_mm: f64,
    pub geometric_rms_mm: f64,
    pub maximum_absolute_residual_mm: f64,
    pub axial_range_mm: [f64; 2],
    pub jacobian_condition_number: f64,
    pub points: usize,
    pub iterations: usize,
    pub starts_converged: usize,
}
#[derive(Clone)]
struct Model {
    center: Vector3<f64>,
    axis: Vector3<f64>,
    radius: f64,
}
fn basis(axis: Vector3<f64>) -> (Vector3<f64>, Vector3<f64>) {
    let index = (0..3)
        .min_by(|&a, &b| axis[a].abs().total_cmp(&axis[b].abs()))
        .unwrap();
    let mut reference = Vector3::zeros();
    reference[index] = 1.;
    let u = axis.cross(&reference).normalize();
    (u, axis.cross(&u))
}
impl Model {
    fn row(&self, p: Vector3<f64>, u: Vector3<f64>, v: Vector3<f64>) -> Option<(f64, [f64; 5])> {
        let d = p - self.center;
        let axial = self.axis.dot(&d);
        let radial = d - self.axis * axial;
        let rho = radial.norm();
        if !rho.is_finite() || rho <= 1e-14 {
            return None;
        }
        let unit = radial / rho;
        let x = unit.dot(&u);
        let y = unit.dot(&v);
        Some((rho - self.radius, [-x, -y, -axial * x, -axial * y, -1.]))
    }
    fn moved(&self, step: SVector<f64, 5>, u: Vector3<f64>, v: Vector3<f64>) -> Option<Self> {
        let axis = (self.axis + u * step[2] + v * step[3]).normalize();
        let mut center = self.center + u * step[0] + v * step[1];
        center -= axis * axis.dot(&center);
        let radius = self.radius + step[4];
        if !axis.iter().chain(center.iter()).all(|x| x.is_finite())
            || !radius.is_finite()
            || radius <= 1e-12
            || radius > 1e6
            || center.norm() > 1e6
        {
            return None;
        }
        Some(Self {
            axis,
            center,
            radius,
        })
    }
    fn score(&self, points: &[Vector3<f64>], work: &WorkControl) -> Result<f64, String> {
        let (u, v) = basis(self.axis);
        let mut score = 0.;
        for (i, &p) in points.iter().enumerate() {
            if i % 1024 == 0 {
                work.check()?;
            }
            let (r, _) = self
                .row(p, u, v)
                .ok_or("cylinder trial intersects a sample on its axis")?;
            score += r * r / points.len() as f64;
        }
        if !score.is_finite() {
            return Err("cylinder score overflow".into());
        }
        Ok(score)
    }
}
fn initialize(
    points: &[Vector3<f64>],
    axis: Vector3<f64>,
    work: &WorkControl,
) -> Result<Model, String> {
    let (u, v) = basis(axis);
    let projected: Vec<_> = points.iter().map(|p| [p.dot(&u), p.dot(&v), 0.]).collect();
    let circle = measurement::fit_circle(&projected, 0., work)?;
    Ok(Model {
        axis,
        center: u * circle.center_mm[0] + v * circle.center_mm[1],
        radius: circle.radius_mm,
    })
}
fn optimize(
    points: &[Vector3<f64>],
    mut model: Model,
    work: &WorkControl,
) -> Result<(Model, usize, f64), String> {
    let mut score = model.score(points, work)?;
    let mut damping = 1e-4_f64;
    for iteration in 0..80 {
        work.check()?;
        let (u, v) = basis(model.axis);
        let mut h = SMatrix::<f64, 5, 5>::zeros();
        let mut g = SVector::<f64, 5>::zeros();
        for (i, &p) in points.iter().enumerate() {
            if i % 1024 == 0 {
                work.check()?;
            }
            let (r, row) = model.row(p, u, v).ok_or("undefined cylinder derivative")?;
            let j = SVector::from(row);
            h += j * j.transpose() / points.len() as f64;
            g += j * (r / points.len() as f64);
        }
        let mut accepted = None;
        for _ in 0..12 {
            work.check()?;
            let Some(cholesky) = (h + SMatrix::identity() * damping).cholesky() else {
                damping *= 10.;
                continue;
            };
            let mut step = cholesky.solve(&(-g));
            let angular = step[2].hypot(step[3]);
            if angular > 0.35 {
                step *= 0.35 / angular;
            }
            if !step.iter().all(|v| v.is_finite()) {
                return Err("cylinder step overflow".into());
            }
            if step.norm() <= 1e-9 && g.norm() <= 1e-10 {
                let condition = condition(points, &model, work)?;
                return Ok((model, iteration + 1, condition));
            }
            if let Some(trial) = model.moved(step, u, v)
                && let Ok(next_score) = trial.score(points, work)
                && next_score < score
            {
                accepted = Some((trial, next_score));
                break;
            }
            damping *= 10.;
            if !damping.is_finite() || damping > 1e12 {
                break;
            }
        }
        let Some((trial, next_score)) = accepted else {
            return Err("cylinder optimizer stalled before stationarity".into());
        };
        model = trial;
        score = next_score;
        damping = (damping * 0.2).max(1e-12);
    }
    Err("cylinder optimizer exhausted its iteration budget".into())
}
fn condition(points: &[Vector3<f64>], model: &Model, work: &WorkControl) -> Result<f64, String> {
    let (u, v) = basis(model.axis);
    let mut j = DMatrix::zeros(points.len(), 5);
    let mut rhs = DVector::zeros(points.len());
    for (i, &p) in points.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let (residual, row) = model.row(p, u, v).ok_or("undefined cylinder derivative")?;
        rhs[i] = -residual;
        for a in 0..5 {
            j[(i, a)] = row[a];
        }
    }
    let svd = nalgebra::linalg::SVD::try_new(j, true, true, 1e-14, 1000)
        .ok_or("cylinder conditioning SVD failed")?;
    work.check()?;
    let large = svd.singular_values[0];
    let small = svd.singular_values[4];
    if !large.is_finite() || !small.is_finite() || large <= 0. || small <= large * 1e-8 {
        return Err("cylinder parameters are insufficiently conditioned".into());
    }
    let correction = svd
        .solve(&rhs, large * 1e-8)
        .map_err(|_| "cylinder final stationarity solve failed")?;
    if !correction.norm().is_finite() || correction.norm() > 1e-7 {
        return Err("cylinder undamped correction is not stationary".into());
    }
    Ok(large / small)
}
/// Three PCA-axis starts; nonlinear radial-distance minimization over five local
/// parameters. A converged, full-rank candidate must also satisfy the caller's
/// finite nonnegative RMS gate. This is not a global primitive recognizer.
pub fn fit(
    points: &[[f64; 3]],
    maximum_rms_mm: f64,
    work: &WorkControl,
) -> Result<Cylinder, String> {
    work.check()?;
    if !(6..=20_000).contains(&points.len())
        || !points.iter().flatten().all(|v| v.is_finite())
        || !maximum_rms_mm.is_finite()
        || maximum_rms_mm < 0.
    {
        return Err(
            "cylinder fit needs 6..20000 finite surface points and a nonnegative RMS gate".into(),
        );
    }
    let anchor = points[0];
    let count = points.len() as f64;
    let mut mean = [0.; 3];
    for (i, p) in points.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Processing, i, points.len())?;
        }
        for a in 0..3 {
            mean[a] += (p[a] - anchor[a]) / count;
        }
    }
    let origin: [f64; 3] = std::array::from_fn(|a| anchor[a] + mean[a]);
    let mut scale = 0_f64;
    for p in points {
        for a in 0..3 {
            scale = scale.max((p[a] - origin[a]).abs());
        }
    }
    if !origin.iter().all(|v| v.is_finite()) || !scale.is_finite() || scale <= 0. {
        return Err("cylinder data extent is degenerate or overflowing".into());
    }
    let normalized: Vec<_> = points
        .iter()
        .map(|p| {
            Vector3::from(std::array::from_fn::<_, 3, _>(|a| {
                (p[a] - origin[a]) / scale
            }))
        })
        .collect();
    let mut cov = Matrix3::zeros();
    for p in &normalized {
        cov += p * p.transpose() / count;
    }
    let eig =
        nalgebra::linalg::SymmetricEigen::try_new(cov, 1e-14, 1000).ok_or("cylinder PCA failed")?;
    let small = eig.eigenvalues.min();
    let large = eig.eigenvalues.max();
    if !small.is_finite() || !large.is_finite() || large <= 0. || small <= large * 1e-10 {
        return Err("cylinder surface samples must span three well-conditioned dimensions".into());
    }
    let mut best = None;
    let mut converged = 0;
    for start in 0..3 {
        work.check()?;
        let axis = eig.eigenvectors.column(start).into_owned().normalize();
        if let Ok(initial) = initialize(&normalized, axis, work)
            && let Ok((model, iterations, condition)) = optimize(&normalized, initial, work)
        {
            let score = model.score(&normalized, work)?;
            converged += 1;
            if best.as_ref().is_none_or(|(_, _, _, old)| score < *old) {
                best = Some((model, iterations, condition, score));
            }
        }
        work.check()?;
    }
    let Some((mut model, iterations, condition, score)) = best else {
        return Err("no full-rank cylinder start converged".into());
    };
    let rms = score.sqrt() * scale;
    if !rms.is_finite() || rms > maximum_rms_mm {
        return Err("cylinder geometric RMS exceeds the requested limit".into());
    }
    let dominant = (0..3)
        .max_by(|&a, &b| model.axis[a].abs().total_cmp(&model.axis[b].abs()))
        .unwrap();
    if model.axis[dominant] < 0. {
        model.axis = -model.axis;
    }
    let (u, v) = basis(model.axis);
    let mut maximum = 0_f64;
    let mut axial = [f64::INFINITY, f64::NEG_INFINITY];
    for (i, &p) in normalized.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let (r, _) = model.row(p, u, v).ok_or("undefined cylinder residual")?;
        maximum = maximum.max(r.abs());
        let t = model.axis.dot(&(p - model.center)) * scale;
        axial[0] = axial[0].min(t);
        axial[1] = axial[1].max(t);
    }
    let center = std::array::from_fn(|a| origin[a] + scale * model.center[a]);
    let radius = model.radius * scale;
    let max_residual = maximum * scale;
    if !center.iter().chain(axial.iter()).all(|v| v.is_finite())
        || !radius.is_finite()
        || radius <= 0.
        || !(2. * radius).is_finite()
        || !max_residual.is_finite()
    {
        return Err("cylinder result is unrepresentable".into());
    }
    work.check()?;
    Ok(Cylinder {
        axis_point_mm: center,
        axis_direction: model.axis.into(),
        radius_mm: radius,
        diameter_mm: 2. * radius,
        geometric_rms_mm: rms,
        maximum_absolute_residual_mm: max_residual,
        axial_range_mm: axial,
        jacobian_condition_number: condition,
        points: points.len(),
        iterations,
        starts_converged: converged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analytic_tangent_jacobian_matches_independent_central_differences() {
        let axis = Vector3::new(0.2, 0.3, 1.).normalize();
        let mut center = Vector3::new(0.2, -0.1, 0.3);
        center -= axis * axis.dot(&center);
        let model = Model {
            center,
            axis,
            radius: 0.5,
        };
        let point = Vector3::new(0.8, 0.7, -0.4);
        let (u, v) = basis(axis);
        let (_, j) = model.row(point, u, v).unwrap();
        for a in 0..5 {
            let mut step = SVector::zeros();
            step[a] = 1e-6;
            let positive = model.moved(step, u, v).unwrap().row(point, u, v).unwrap().0;
            let negative = model
                .moved(-step, u, v)
                .unwrap()
                .row(point, u, v)
                .unwrap()
                .0;
            assert!(
                ((positive - negative) / 2e-6 - j[a]).abs() < 1e-7,
                "parameter {a}"
            );
        }
    }
    #[test]
    fn nonlinear_refinement_recovers_tilted_axis_center_and_radius() {
        let points: Vec<_> = [-1., 0., 1.]
            .into_iter()
            .flat_map(|z| {
                (0..24).map(move |i| {
                    let t = i as f64 * std::f64::consts::TAU / 24.;
                    Vector3::new(0.5 * t.cos(), 0.5 * t.sin(), z)
                })
            })
            .collect();
        let axis = Vector3::new(0.2, 0.1, 1.).normalize();
        let mut center = Vector3::new(0.1, -0.05, 0.);
        center -= axis * axis.dot(&center);
        let initial = Model {
            center,
            axis,
            radius: 0.4,
        };
        assert!(condition(&points, &initial, &WorkControl::default()).is_err());
        let (result, _, condition) = optimize(&points, initial, &WorkControl::default()).unwrap();
        assert!(result.axis.z.abs() > 1. - 1e-9);
        assert!(result.center.norm() < 1e-8);
        assert!((result.radius - 0.5).abs() < 1e-8);
        assert!(condition < 100.);
    }
}
