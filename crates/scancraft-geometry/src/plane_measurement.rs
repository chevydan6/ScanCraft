//! Point-to-fitted-plane measurements in declared millimeters.
use crate::work::{Stage, WorkControl};
use nalgebra::{Matrix3, Vector3};
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointPlane {
    pub origin_mm: [f64; 3],
    pub normal: [f64; 3],
    pub signed_distance_mm: f64,
    pub distance_mm: f64,
    pub foot_mm: [f64; 3],
    pub plane_rms_mm: f64,
    pub plane_points: usize,
}
/// PCA plane excluding the explicitly indexed probe. The dominant normal
/// component is positive; signed distance uses this numerical convention.
pub fn measure(
    points: &[[f64; 3]],
    probe: usize,
    maximum_plane_rms_mm: f64,
    work: &WorkControl,
) -> Result<PointPlane, String> {
    work.check()?;
    if !(4..=100_001).contains(&points.len())
        || probe >= points.len()
        || !points.iter().flatten().all(|v| v.is_finite())
        || !maximum_plane_rms_mm.is_finite()
        || maximum_plane_rms_mm < 0.
    {
        return Err("point-plane measurement requires one probe, 3..100000 finite plane points and a nonnegative RMS limit".into());
    }
    let count = points.len() - 1;
    let anchor = points[usize::from(probe == 0)];
    let mut mean = [0.; 3];
    for (i, p) in points.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Processing, i, points.len())?;
        }
        if i != probe {
            for a in 0..3 {
                mean[a] += (p[a] - anchor[a]) / count as f64;
            }
        }
    }
    let origin: [f64; 3] = std::array::from_fn(|a| anchor[a] + mean[a]);
    let mut scale = 0_f64;
    for (i, p) in points.iter().enumerate() {
        if i != probe {
            for a in 0..3 {
                scale = scale.max((p[a] - origin[a]).abs());
            }
        }
    }
    if !origin.iter().all(|v| v.is_finite()) || !scale.is_finite() || scale <= 0. {
        return Err("plane extent is degenerate or overflowing".into());
    }
    let local = |p: [f64; 3]| {
        Vector3::from(std::array::from_fn::<_, 3, _>(|a| {
            (p[a] - origin[a]) / scale
        }))
    };
    let mut center = Vector3::zeros();
    let mut cov = Matrix3::zeros();
    for (i, &p) in points.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        if i != probe {
            let d = local(p);
            center += d / count as f64;
            cov += d * d.transpose() / count as f64;
        }
    }
    cov -= center * center.transpose();
    let eig = nalgebra::linalg::SymmetricEigen::try_new(cov, 1e-14, 1000)
        .ok_or("plane eigensolver did not converge")?;
    let mut order = [0, 1, 2];
    order.sort_unstable_by(|&a, &b| eig.eigenvalues[a].total_cmp(&eig.eigenvalues[b]));
    let [small, middle, large] = order.map(|i| eig.eigenvalues[i]);
    if !large.is_finite()
        || large <= 0.
        || middle <= large * 1e-12
        || middle - small <= large * 1e-10
    {
        return Err("plane points are collinear or have an ambiguous normal".into());
    }
    let mut n = eig.eigenvectors.column(order[0]).into_owned().normalize();
    let dominant = (0..3)
        .max_by(|&a, &b| n[a].abs().total_cmp(&n[b].abs()))
        .unwrap();
    if n[dominant] < 0. {
        n = -n;
    }
    let mut residual_norm = 0_f64;
    for (i, &p) in points.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        if i != probe {
            residual_norm = residual_norm.hypot(n.dot(&(local(p) - center)));
        }
    }
    let rms = (residual_norm / (count as f64).sqrt()) * scale;
    let signed = n.dot(&(local(points[probe]) - center)) * scale;
    let foot = std::array::from_fn(|a| points[probe][a] - signed * n[a]);
    let origin = std::array::from_fn(|a| origin[a] + center[a] * scale);
    if !rms.is_finite()
        || rms > maximum_plane_rms_mm
        || !signed.is_finite()
        || !foot.iter().chain(origin.iter()).all(|v| v.is_finite())
    {
        return Err("point-plane result exceeds the RMS limit or is unrepresentable".into());
    }
    work.check()?;
    Ok(PointPlane {
        origin_mm: origin,
        normal: n.into(),
        signed_distance_mm: signed,
        distance_mm: signed.abs(),
        foot_mm: foot,
        plane_rms_mm: rms,
        plane_points: count,
    })
}
