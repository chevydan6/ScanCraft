//! Double-precision rigid registration and plane fitting. Millimeters, right-handed axes.
use nalgebra::{Matrix3, Vector3, linalg::SVD};
#[derive(Debug, Clone)]
pub struct Rigid {
    pub rotation: Matrix3<f64>,
    pub translation: Vector3<f64>,
}
impl Rigid {
    pub fn validate(&self) -> Result<(), String> {
        let orthogonality =
            (self.rotation.transpose() * self.rotation - Matrix3::identity()).norm();
        let determinant = self.rotation.determinant();
        if !self
            .rotation
            .iter()
            .chain(self.translation.iter())
            .all(|x| x.is_finite())
            || !orthogonality.is_finite()
            || !determinant.is_finite()
            || orthogonality > 1e-8
            || (determinant - 1.).abs() > 1e-8
        {
            return Err("transform is not a finite rigid pose".into());
        }
        Ok(())
    }
    pub fn identity() -> Self {
        Self {
            rotation: Matrix3::identity(),
            translation: Vector3::zeros(),
        }
    }
    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        (self.rotation * Vector3::from(p) + self.translation).into()
    }
    /// Composition maps p through other first, then self.
    pub fn compose(&self, other: &Self) -> Self {
        Self {
            rotation: self.rotation * other.rotation,
            translation: self.rotation * other.translation + self.translation,
        }
    }
}
fn check(points: &[[f64; 3]]) -> Result<(), String> {
    if points.len() < 3 || !points.iter().flatten().all(|x| x.is_finite()) {
        return Err("need at least three finite points".into());
    }
    Ok(())
}
fn centroid(points: &[[f64; 3]]) -> Vector3<f64> {
    let n = points.len() as f64;
    points
        .iter()
        .fold(Vector3::zeros(), |sum, p| sum + Vector3::from(*p) / n)
}
/// Kabsch fit for known point correspondences. Degenerate collinear fits fail.
pub fn rigid_fit(source: &[[f64; 3]], target: &[[f64; 3]]) -> Result<(Rigid, f64), String> {
    check(source)?;
    check(target)?;
    if source.len() != target.len() {
        return Err("correspondence counts differ".into());
    }
    let a = centroid(source);
    let b = centroid(target);
    let mut cov = Matrix3::zeros();
    for (p, q) in source.iter().zip(target) {
        cov += (Vector3::from(*p) - a) * (Vector3::from(*q) - b).transpose();
    }
    if !cov.iter().all(|x| x.is_finite()) {
        return Err("covariance overflow".into());
    }
    let svd = SVD::try_new(cov, true, true, 1e-14, 1000).ok_or("SVD failed to converge")?;
    if svd.singular_values[0] <= 0. || svd.singular_values[1] <= svd.singular_values[0] * 1e-12 {
        return Err("collinear or degenerate correspondences".into());
    }
    let u = svd.u.ok_or("SVD missing U")?;
    let vt = svd.v_t.ok_or("SVD missing V")?;
    let mut sign = Matrix3::identity();
    sign[(2, 2)] = (vt.transpose() * u.transpose()).determinant().signum();
    let rotation = vt.transpose() * sign * u.transpose();
    let translation = b - rotation * a;
    let result = Rigid {
        rotation,
        translation,
    };
    let mse = source
        .iter()
        .zip(target)
        .map(|(p, q)| {
            (Vector3::from(result.apply(*p)) - Vector3::from(*q)).norm_squared()
                / source.len() as f64
        })
        .sum::<f64>();
    if !mse.is_finite() {
        return Err("fit residual overflow".into());
    }
    Ok((result, mse.sqrt()))
}

pub struct Plane {
    pub normal: [f64; 3],
    pub offset_mm: f64,
    pub rms_mm: f64,
}
pub fn fit_plane(points: &[[f64; 3]]) -> Result<Plane, String> {
    check(points)?;
    let center = centroid(points);
    let mut cov = Matrix3::zeros();
    for p in points {
        let v = Vector3::from(*p) - center;
        cov += v * v.transpose() / points.len() as f64;
    }
    if !cov.iter().all(|v| v.is_finite()) {
        return Err("plane covariance overflow".into());
    }
    let eig = nalgebra::linalg::SymmetricEigen::try_new(cov, 1e-14, 1000)
        .ok_or("plane fit did not converge")?;
    let mut order = [0, 1, 2];
    order.sort_unstable_by(|a, b| eig.eigenvalues[*a].total_cmp(&eig.eigenvalues[*b]));
    if eig.eigenvalues[order[2]] <= 0.
        || eig.eigenvalues[order[1]] <= eig.eigenvalues[order[2]] * 1e-12
    {
        return Err("plane points are collinear".into());
    }
    let mut normal = eig.eigenvectors.column(order[0]).into_owned();
    let dominant = (0..3)
        .max_by(|a, b| normal[*a].abs().total_cmp(&normal[*b].abs()))
        .unwrap();
    if normal[dominant] < 0. {
        normal = -normal;
    }
    let offset = -normal.dot(&center);
    let rms = points
        .iter()
        .map(|p| (normal.dot(&Vector3::from(*p)) + offset).powi(2) / points.len() as f64)
        .sum::<f64>()
        .sqrt();
    Ok(Plane {
        normal: normal.into(),
        offset_mm: offset,
        rms_mm: rms,
    })
}

pub struct IcpResult {
    pub transform: Rigid,
    pub rms_mm: f64,
    pub iterations: usize,
    pub converged: bool,
    pub correspondences: usize,
}
/// CPU point-to-point ICP with exact indexed nearest neighbors; capped for offline datasets.
/// Requires a suitable initial pose; convergence is not evidence of global correctness.
pub fn icp(
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    initial: Rigid,
    max_distance_mm: f64,
    max_iterations: usize,
    tolerance_mm: f64,
) -> Result<IcpResult, String> {
    icp_with(
        source,
        target,
        initial,
        max_distance_mm,
        max_iterations,
        tolerance_mm,
        &crate::work::WorkControl::default(),
    )
}
/// Cancellable reference ICP; no partial transform is returned on cancellation.
pub fn icp_with(
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    initial: Rigid,
    max_distance_mm: f64,
    max_iterations: usize,
    tolerance_mm: f64,
    work: &crate::work::WorkControl,
) -> Result<IcpResult, String> {
    work.check()?;
    check(source)?;
    check(target)?;
    if source.len() > 10000 || target.len() > 10000 {
        return Err("reference ICP limited to 10,000 points per cloud".into());
    }
    if !max_distance_mm.is_finite()
        || max_distance_mm <= 0.
        || !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || max_iterations == 0
        || max_iterations > 1000
    {
        return Err("invalid ICP parameters".into());
    }
    initial.validate()?;
    let index = crate::spatial::SpatialIndex::build(target, work)?;
    let mut transform = initial;
    let mut previous = f64::INFINITY;
    let mut rms = f64::INFINITY;
    let mut matches = 0;
    for iteration in 1..=max_iterations {
        work.update(
            crate::work::Stage::Processing,
            iteration - 1,
            max_iterations,
        )?;
        let mut a = Vec::new();
        let mut b = Vec::new();
        for point in source {
            work.check()?;
            let p = transform.apply(*point);
            if let Some(neighbor) = index.nearest(p, work)?
                && neighbor.distance_mm <= max_distance_mm
            {
                a.push(p);
                b.push(target[neighbor.index]);
            }
        }
        matches = a.len();
        if matches < 3 || matches * 2 < source.len() {
            return Err("insufficient ICP overlap; alignment rejected".into());
        }
        let (delta, error) = rigid_fit(&a, &b)?;
        transform = delta.compose(&transform);
        rms = error;
        if (previous - rms).abs() <= tolerance_mm {
            work.check()?;
            return Ok(IcpResult {
                transform,
                rms_mm: rms,
                iterations: iteration,
                converged: true,
                correspondences: matches,
            });
        }
        previous = rms;
    }
    work.check()?;
    Ok(IcpResult {
        transform,
        rms_mm: rms,
        iterations: max_iterations,
        converged: false,
        correspondences: matches,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_rotation_entries_cannot_bypass_validation_through_nan_arithmetic() {
        let pose = Rigid {
            rotation: Matrix3::new(
                f64::MAX,
                f64::MAX,
                f64::MAX,
                f64::MAX,
                -f64::MAX,
                f64::MAX,
                f64::MAX,
                f64::MAX,
                -f64::MAX,
            ),
            translation: Vector3::zeros(),
        };
        assert!(pose.rotation.iter().all(|v| v.is_finite()));
        assert!(pose.validate().is_err());
        assert!(Rigid::identity().validate().is_ok());
    }
    fn points() -> Vec<[f64; 3]> {
        vec![
            [0., 0., 0.],
            [10., 0., 0.],
            [0., 20., 0.],
            [0., 0., 30.],
            [4., 5., 8.],
            [-7., 2., 4.],
        ]
    }
    #[test]
    fn kabsch_known_rotation_and_translation() {
        let p = points();
        let angle = 0.3f64;
        let expected = Rigid {
            rotation: Matrix3::new(
                angle.cos(),
                -angle.sin(),
                0.,
                angle.sin(),
                angle.cos(),
                0.,
                0.,
                0.,
                1.,
            ),
            translation: Vector3::new(2., -3., 4.),
        };
        let q: Vec<_> = p.iter().map(|p| expected.apply(*p)).collect();
        let (fit, rms) = rigid_fit(&p, &q).unwrap();
        assert!(rms < 1e-10);
        assert!((fit.translation - expected.translation).norm() < 1e-10);
        assert!((fit.rotation - expected.rotation).norm() < 1e-10);
        assert!((fit.rotation.determinant() - 1.).abs() < 1e-12);
    }
    #[test]
    fn rejects_collinear() {
        assert!(rigid_fit(&[[0.; 3], [1.; 3], [2.; 3]], &[[0.; 3], [1.; 3], [2.; 3]]).is_err());
    }
    #[test]
    fn fits_plane_exactly() {
        let points = vec![[0., 0., 3.], [1., 0., 3.], [0., 1., 3.], [2., 3., 3.]];
        let p = fit_plane(&points).unwrap();
        assert!(p.rms_mm < 1e-12);
        assert!((p.normal[2] - 1.).abs() < 1e-12);
        assert!((p.offset_mm + 3.).abs() < 1e-12);
    }
    #[test]
    fn icp_small_translation() {
        let p = points();
        let q: Vec<_> = p
            .iter()
            .map(|p| [p[0] + 0.1, p[1] - 0.15, p[2] + 0.2])
            .collect();
        let fit = icp(&p, &q, Rigid::identity(), 2., 30, 1e-10).unwrap();
        assert!(fit.converged);
        assert!(fit.rms_mm < 1e-10);
        assert!((fit.transform.translation - Vector3::new(0.1, -0.15, 0.2)).norm() < 1e-10);
    }
    #[test]
    fn bad_overlap_rejected() {
        let p = points();
        let q: Vec<_> = p
            .iter()
            .map(|p| [p[0] + 100., p[1] + 100., p[2] + 100.])
            .collect();
        assert!(icp(&p, &q, Rigid::identity(), 1., 20, 1e-6).is_err());
    }
}
