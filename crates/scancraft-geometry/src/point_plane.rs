//! Cancellable CPU point-to-plane ICP. Input normals must already describe the
//! target surface; this does not estimate normals or qualify scanner geometry.
use crate::{
    registration::Rigid,
    spatial::SpatialIndex,
    work::{Stage, WorkControl},
};
use nalgebra::{DMatrix, DVector, Rotation3, Vector3};

/// Huber delta is an explicit residual scale in millimeters, never auto-estimated.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Loss {
    #[default]
    Quadratic,
    Huber {
        delta_mm: f64,
    },
}
impl Loss {
    pub fn validate(self) -> Result<(), String> {
        if let Self::Huber { delta_mm } = self
            && (!delta_mm.is_finite() || delta_mm <= 0.)
        {
            return Err("Huber residual scale must be finite and positive millimeters".into());
        }
        Ok(())
    }
    fn weight(self, residual: f64) -> f64 {
        match self {
            Self::Quadratic => 1.,
            Self::Huber { delta_mm } => {
                if residual.abs() <= delta_mm {
                    1.
                } else {
                    delta_mm / residual.abs()
                }
            }
        }
    }
    fn cost(self, residual: f64) -> f64 {
        match self {
            Self::Huber { delta_mm } if residual.abs() > delta_mm => {
                delta_mm * (residual.abs() - 0.5 * delta_mm)
            }
            _ => 0.5 * residual * residual,
        }
    }
}
fn objective(
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    normals: &[Vector3<f64>],
    pose: &Rigid,
    loss: Loss,
) -> Result<(f64, usize), String> {
    let mut mean = 0.;
    let mut downweighted = 0;
    for ((p, q), n) in source.iter().zip(target).zip(normals) {
        let residual = n.dot(&(Vector3::from(pose.apply(*p)) - Vector3::from(*q)));
        mean += loss.cost(residual) / (source.len() as f64);
        downweighted += usize::from(loss.weight(residual) < 1.);
    }
    if !mean.is_finite() {
        return Err("point-to-plane objective overflow".into());
    }
    Ok((mean, downweighted))
}
pub struct Config {
    pub maximum_distance_mm: f64,
    pub maximum_iterations: usize,
    pub tolerance_mm: f64,
    pub maximum_rotation_step_rad: f64,
}
pub struct Report {
    pub transform: Rigid,
    pub point_to_plane_rms_mm: f64,
    /// Mean rho(r), in square millimeters; not a physical accuracy metric.
    pub objective_mean_mm2: f64,
    pub downweighted_correspondences: usize,
    pub iterations: usize,
    pub correspondences: usize,
    pub converged: bool,
}
fn unit(normal: [f64; 3]) -> Result<Vector3<f64>, String> {
    let length = normal[0].hypot(normal[1]).hypot(normal[2]);
    if !length.is_finite() || length <= 0. {
        return Err("target normal must be finite and nonzero".into());
    }
    Ok(Vector3::from(normal) / length)
}
fn rms(
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    normals: &[Vector3<f64>],
    pose: &Rigid,
) -> Result<f64, String> {
    let mut norm = 0f64;
    for ((p, q), n) in source.iter().zip(target).zip(normals) {
        let residual = n.dot(&(Vector3::from(pose.apply(*p)) - Vector3::from(*q)));
        norm = norm.hypot(residual);
    }
    let value = norm / (source.len() as f64).sqrt();
    if !value.is_finite() {
        return Err("point-to-plane residual overflow".into());
    }
    Ok(value)
}

/// One linearized update for fixed correspondences. Centering/scaling makes
/// rotation and translation columns comparable. Rank-deficient constraints,
/// excessive rotation and increasing exact residual are rejected.
pub fn step(
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    normals: &[[f64; 3]],
    maximum_rotation_step_rad: f64,
    work: &WorkControl,
) -> Result<(Rigid, f64), String> {
    step_with_loss(
        source,
        target,
        normals,
        maximum_rotation_step_rad,
        Loss::Quadratic,
        work,
    )
}
pub fn step_with_loss(
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    normals: &[[f64; 3]],
    maximum_rotation_step_rad: f64,
    loss: Loss,
    work: &WorkControl,
) -> Result<(Rigid, f64), String> {
    work.check()?;
    loss.validate()?;
    let count = source.len();
    if !(6..=10_000).contains(&count)
        || target.len() != count
        || normals.len() != count
        || !source.iter().chain(target).flatten().all(|v| v.is_finite())
        || !maximum_rotation_step_rad.is_finite()
        || maximum_rotation_step_rad <= 0.
        || maximum_rotation_step_rad > 0.5
    {
        return Err("invalid point-to-plane correspondence set or rotation limit".into());
    }
    let normals: Vec<_> = normals.iter().map(|n| unit(*n)).collect::<Result<_, _>>()?;
    let center = source.iter().fold(Vector3::zeros(), |sum, p| {
        sum + Vector3::from(*p) / (count as f64)
    });
    let scale = source
        .iter()
        .map(|p| {
            let d = Vector3::from(*p) - center;
            d[0].hypot(d[1]).hypot(d[2])
        })
        .fold(0f64, f64::max);
    if !scale.is_finite() || scale <= 0. {
        return Err("degenerate point-to-plane source extent".into());
    }
    let mut a = DMatrix::zeros(count, 6);
    let mut b = DVector::zeros(count);
    for (i, ((p, q), n)) in source.iter().zip(target).zip(&normals).enumerate() {
        if i % 64 == 0 {
            work.check()?;
        }
        let cross = ((Vector3::from(*p) - center) / scale).cross(n);
        for j in 0..3 {
            a[(i, j)] = cross[j];
            a[(i, j + 3)] = n[j];
        }
        b[i] = n.dot(&(Vector3::from(*q) - Vector3::from(*p)));
        let weight = loss.weight(b[i]).sqrt();
        for j in 0..6 {
            a[(i, j)] *= weight;
        }
        b[i] *= weight;
    }
    if !a.iter().chain(b.iter()).all(|v| v.is_finite()) {
        return Err("point-to-plane system overflow".into());
    }
    let svd = nalgebra::linalg::SVD::try_new(a, true, true, 1e-14, 1000)
        .ok_or("point-to-plane SVD did not converge")?;
    work.check()?;
    let largest = svd.singular_values[0];
    if !largest.is_finite()
        || largest <= 0.
        || svd
            .singular_values
            .iter()
            .any(|s| !s.is_finite() || *s <= largest * 1e-8)
    {
        return Err("point-to-plane pose is underconstrained or ill-conditioned".into());
    }
    let delta = svd
        .solve(&b, largest * 1e-8)
        .map_err(|_| "point-to-plane solve failed")?;
    let omega = Vector3::new(delta[0], delta[1], delta[2]) / scale;
    if !omega.iter().chain(delta.iter()).all(|v| v.is_finite())
        || omega.norm() > maximum_rotation_step_rad
    {
        return Err("point-to-plane rotation step exceeds local alignment limit".into());
    }
    let rotation = Rotation3::from_scaled_axis(omega).into_inner();
    let translation = center + Vector3::new(delta[3], delta[4], delta[5]) - rotation * center;
    let pose = Rigid {
        rotation,
        translation,
    };
    pose.validate()?;
    let before = rms(source, target, &normals, &Rigid::identity())?;
    let after = rms(source, target, &normals, &pose)?;
    if loss == Loss::Quadratic {
        if after > before + 1e-10 * before.max(1.) {
            return Err("point-to-plane update increased exact residual".into());
        }
    } else {
        let (before, _) = objective(source, target, &normals, &Rigid::identity(), loss)?;
        let (after, _) = objective(source, target, &normals, &pose, loss)?;
        if after > before + 1e-10 * before + f64::MIN_POSITIVE {
            return Err("point-to-plane update increased exact robust objective".into());
        }
    }
    work.check()?;
    Ok((pose, after))
}

pub fn icp(
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    target_normals: &[[f64; 3]],
    initial: Rigid,
    config: Config,
    work: &WorkControl,
) -> Result<Report, String> {
    icp_with_loss(
        source,
        target,
        target_normals,
        initial,
        config,
        Loss::Quadratic,
        work,
    )
}
pub fn icp_with_loss(
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    target_normals: &[[f64; 3]],
    initial: Rigid,
    config: Config,
    loss: Loss,
    work: &WorkControl,
) -> Result<Report, String> {
    work.check()?;
    loss.validate()?;
    initial.validate()?;
    if !(6..=10_000).contains(&source.len())
        || !(6..=10_000).contains(&target.len())
        || target.len() != target_normals.len()
        || !source.iter().chain(target).flatten().all(|v| v.is_finite())
        || !config.maximum_distance_mm.is_finite()
        || config.maximum_distance_mm <= 0.
        || !config.tolerance_mm.is_finite()
        || config.tolerance_mm <= 0.
        || config.maximum_iterations == 0
        || config.maximum_iterations > 1000
        || !config.maximum_rotation_step_rad.is_finite()
        || config.maximum_rotation_step_rad <= 0.
        || config.maximum_rotation_step_rad > 0.5
    {
        return Err("invalid point-to-plane ICP inputs or limits".into());
    }
    for normal in target_normals {
        unit(*normal)?;
    }
    let index = SpatialIndex::build(target, work)?;
    let mut transform = initial;
    let mut previous = f64::INFINITY;
    let mut error = f64::INFINITY;
    let mut matches = 0;
    let mut objective_mean_mm2 = f64::INFINITY;
    let mut downweighted_correspondences = 0;
    for iteration in 1..=config.maximum_iterations {
        work.update(Stage::Processing, iteration - 1, config.maximum_iterations)?;
        let mut a = Vec::new();
        let mut b = Vec::new();
        let mut normals = Vec::new();
        for point in source {
            work.check()?;
            let p = transform.apply(*point);
            if let Some(neighbor) = index.nearest(p, work)?
                && neighbor.distance_mm <= config.maximum_distance_mm
            {
                a.push(p);
                b.push(target[neighbor.index]);
                normals.push(target_normals[neighbor.index]);
            }
        }
        matches = a.len();
        if matches < 6 || matches * 2 < source.len() {
            return Err("insufficient point-to-plane ICP overlap".into());
        }
        let (delta, rms) = step_with_loss(
            &a,
            &b,
            &normals,
            config.maximum_rotation_step_rad,
            loss,
            work,
        )?;
        let normalized: Vec<_> = normals.iter().map(|n| unit(*n)).collect::<Result<_, _>>()?;
        (objective_mean_mm2, downweighted_correspondences) =
            objective(&a, &b, &normalized, &delta, loss)?;
        let convergence_error = if loss == Loss::Quadratic {
            rms
        } else {
            (2. * objective_mean_mm2).sqrt()
        };
        if !convergence_error.is_finite() {
            return Err("robust convergence metric overflow".into());
        }
        let mut displacement = 0f64;
        for point in &a {
            let shift = Vector3::from(delta.apply(*point)) - Vector3::from(*point);
            displacement = displacement.hypot(shift[0].hypot(shift[1]).hypot(shift[2]));
        }
        displacement /= (a.len() as f64).sqrt();
        if !displacement.is_finite() {
            return Err("point-to-plane displacement overflow".into());
        }
        transform = delta.compose(&transform);
        transform.validate()?;
        error = rms;
        if (previous - convergence_error).abs() <= config.tolerance_mm
            && displacement <= config.tolerance_mm
        {
            work.check()?;
            return Ok(Report {
                transform,
                point_to_plane_rms_mm: error,
                objective_mean_mm2,
                downweighted_correspondences,
                iterations: iteration,
                correspondences: matches,
                converged: true,
            });
        }
        previous = convergence_error;
    }
    work.check()?;
    Ok(Report {
        transform,
        point_to_plane_rms_mm: error,
        objective_mean_mm2,
        downweighted_correspondences,
        iterations: config.maximum_iterations,
        correspondences: matches,
        converged: false,
    })
}
