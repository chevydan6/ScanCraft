//! Numerical measurements in declared millimeter coordinates. No physical qualification.
use crate::work::{Stage, WorkControl};
use nalgebra::{DMatrix, DVector};
/// Unsigned angle in [0, 180] degrees at `vertex`, between two nonzero arms.
/// Each arm is independently scaled before normalization to support very
/// different lengths. Overflowing finite endpoint differences use half-deltas.
pub fn angle_degrees(a: [f64; 3], vertex: [f64; 3], b: [f64; 3]) -> Result<f64, String> {
    if !a
        .iter()
        .chain(vertex.iter())
        .chain(b.iter())
        .all(|x| x.is_finite())
    {
        return Err("angle coordinates must be finite".into());
    }
    let direction = |p: [f64; 3]| -> Result<nalgebra::Vector3<f64>, String> {
        let mut delta: [f64; 3] = std::array::from_fn(|i| p[i] - vertex[i]);
        if !delta.iter().all(|x| x.is_finite()) {
            delta = std::array::from_fn(|i| p[i] / 2. - vertex[i] / 2.);
        }
        let scale = delta.iter().map(|x| x.abs()).fold(0., f64::max);
        if scale == 0. {
            return Err("angle requires two nonzero arms at the vertex".into());
        }
        Ok(nalgebra::Vector3::from(delta.map(|x| x / scale)).normalize())
    };
    let u = direction(a)?;
    let v = direction(b)?;
    Ok(u.cross(&v).norm().atan2(u.dot(&v)).to_degrees())
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sphere {
    pub center_mm: [f64; 3],
    pub radius_mm: f64,
    pub diameter_mm: f64,
    pub geometric_rms_mm: f64,
    pub maximum_absolute_residual_mm: f64,
    pub design_condition_number: f64,
    pub points: usize,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Circle {
    pub center_mm: [f64; 3],
    pub normal: [f64; 3],
    pub radius_mm: f64,
    pub diameter_mm: f64,
    pub plane_rms_mm: f64,
    pub geometric_rms_mm: f64,
    pub maximum_absolute_residual_mm: f64,
    pub design_condition_number: f64,
    pub points: usize,
}
/// PCA supporting plane followed by algebraic circle least squares. The caller
/// must explicitly bound plane RMS; this is not robust or nonlinear fitting.
pub fn fit_circle(
    points: &[[f64; 3]],
    maximum_plane_rms_mm: f64,
    work: &WorkControl,
) -> Result<Circle, String> {
    work.check()?;
    if !(3..=100_000).contains(&points.len())
        || !points.iter().flatten().all(|v| v.is_finite())
        || !maximum_plane_rms_mm.is_finite()
        || maximum_plane_rms_mm < 0.
    {
        return Err(
            "circle fit requires 3..100000 finite points and a nonnegative plane RMS limit".into(),
        );
    }
    let anchor = points[0];
    let count = points.len() as f64;
    let mut mean = [0.; 3];
    for (i, p) in points.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Processing, i, points.len())?;
        }
        for j in 0..3 {
            mean[j] += (p[j] - anchor[j]) / count;
        }
    }
    let origin: [f64; 3] = std::array::from_fn(|j| anchor[j] + mean[j]);
    let mut scale = 0f64;
    for p in points {
        let d: [f64; 3] = std::array::from_fn(|j| p[j] - origin[j]);
        scale = scale.max(d[0].hypot(d[1]).hypot(d[2]));
    }
    if !origin.iter().all(|v| v.is_finite()) || !scale.is_finite() || scale <= 0. {
        return Err("circle extent is degenerate or overflowing".into());
    }
    let normalized: Vec<[f64; 3]> = points
        .iter()
        .map(|p| std::array::from_fn(|j| (p[j] - origin[j]) / scale))
        .collect();
    let plane = crate::registration::fit_plane(&normalized)?;
    work.check()?;
    let plane_rms = plane.rms_mm * scale;
    if !plane_rms.is_finite() || plane_rms > maximum_plane_rms_mm {
        return Err("circle supporting plane exceeds RMS limit".into());
    }
    let n = nalgebra::Vector3::from(plane.normal);
    let axis = (0..3)
        .min_by(|a, b| n[*a].abs().total_cmp(&n[*b].abs()))
        .unwrap();
    let mut reference = nalgebra::Vector3::zeros();
    reference[axis] = 1.;
    let u = n.cross(&reference).normalize();
    let v = n.cross(&u);
    let mut a = DMatrix::zeros(points.len(), 3);
    let mut b = DVector::zeros(points.len());
    for (i, p) in normalized.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let p = nalgebra::Vector3::from(*p);
        let x = u.dot(&p);
        let y = v.dot(&p);
        a[(i, 0)] = 2. * x;
        a[(i, 1)] = 2. * y;
        a[(i, 2)] = 1.;
        b[i] = x * x + y * y;
    }
    if !a.iter().chain(b.iter()).all(|v| v.is_finite()) {
        return Err("circle design matrix overflow".into());
    }
    let svd = nalgebra::linalg::SVD::try_new(a, true, true, 1e-14, 1000)
        .ok_or("circle SVD did not converge")?;
    work.check()?;
    let largest = svd.singular_values[0];
    let smallest = svd.singular_values[2];
    if !largest.is_finite() || !smallest.is_finite() || largest <= 0. || smallest <= largest * 1e-8
    {
        return Err("circle samples are insufficiently conditioned".into());
    }
    let solution = svd
        .solve(&b, largest * 1e-8)
        .map_err(|_| "circle solve failed")?;
    let local_center = u * solution[0] + v * solution[1] - n * plane.offset_mm;
    let center = std::array::from_fn::<_, 3, _>(|j| origin[j] + scale * local_center[j]);
    let radius = scale * (solution[2] + solution[0].powi(2) + solution[1].powi(2)).sqrt();
    if !center.iter().all(|v| v.is_finite())
        || !radius.is_finite()
        || radius <= 0.
        || !(2. * radius).is_finite()
    {
        return Err("circle result is unrepresentable".into());
    }
    let mut norm = 0f64;
    let mut maximum = 0f64;
    for (i, p) in points.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let d = nalgebra::Vector3::from(*p) - nalgebra::Vector3::from(center);
        let radial = u.dot(&d).hypot(v.dot(&d)) - radius;
        let residual = radial.hypot(n.dot(&d));
        if !residual.is_finite() {
            return Err("circle geometric residual overflow".into());
        }
        norm = norm.hypot(residual);
        maximum = maximum.max(residual);
    }
    let rms = norm / count.sqrt();
    if !rms.is_finite() {
        return Err("circle geometric RMS overflow".into());
    }
    work.check()?;
    Ok(Circle {
        center_mm: center,
        normal: plane.normal,
        radius_mm: radius,
        diameter_mm: 2. * radius,
        plane_rms_mm: plane_rms,
        geometric_rms_mm: rms,
        maximum_absolute_residual_mm: maximum,
        design_condition_number: largest / smallest,
        points: points.len(),
    })
}
/// Centered/scaled algebraic least-squares sphere with independently evaluated
/// geometric residuals. Not a robust fit or nonlinear geometric minimizer.
pub fn fit_sphere(points: &[[f64; 3]], work: &WorkControl) -> Result<Sphere, String> {
    work.check()?;
    if !(4..=100_000).contains(&points.len()) || !points.iter().flatten().all(|v| v.is_finite()) {
        return Err("sphere fit requires 4..100000 finite points".into());
    }
    let anchor = points[0];
    let count = points.len() as f64;
    let mut mean = [0.; 3];
    for (i, p) in points.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Processing, i, points.len())?;
        }
        for j in 0..3 {
            mean[j] += (p[j] - anchor[j]) / count;
        }
    }
    let origin = std::array::from_fn::<_, 3, _>(|j| anchor[j] + mean[j]);
    let mut scale = 0f64;
    for p in points {
        let d: [f64; 3] = std::array::from_fn(|j| p[j] - origin[j]);
        scale = scale.max(d[0].hypot(d[1]).hypot(d[2]));
    }
    if !origin.iter().all(|v| v.is_finite()) || !scale.is_finite() || scale <= 0. {
        return Err("sphere fit has degenerate or overflowing extent".into());
    }
    let mut a = DMatrix::zeros(points.len(), 4);
    let mut b = DVector::zeros(points.len());
    for (i, p) in points.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let v: [f64; 3] = std::array::from_fn(|j| (p[j] - origin[j]) / scale);
        for j in 0..3 {
            a[(i, j)] = 2. * v[j];
        }
        a[(i, 3)] = 1.;
        b[i] = v.iter().map(|x| x * x).sum();
    }
    if !a.iter().chain(b.iter()).all(|v| v.is_finite()) {
        return Err("sphere design matrix overflow".into());
    }
    let svd = nalgebra::linalg::SVD::try_new(a, true, true, 1e-14, 1000)
        .ok_or("sphere SVD did not converge")?;
    work.check()?;
    let largest = svd.singular_values[0];
    let smallest = svd.singular_values[3];
    if !largest.is_finite() || !smallest.is_finite() || smallest <= largest * 1e-8 || largest <= 0.
    {
        return Err("sphere samples are coplanar or insufficiently conditioned".into());
    }
    let solution = svd
        .solve(&b, largest * 1e-8)
        .map_err(|_| "sphere solve failed")?;
    let r2 = solution[3] + solution[0].powi(2) + solution[1].powi(2) + solution[2].powi(2);
    let center: [f64; 3] = std::array::from_fn(|j| origin[j] + scale * solution[j]);
    let radius = scale * r2.sqrt();
    if !center.iter().all(|v| v.is_finite())
        || !radius.is_finite()
        || radius <= 0.
        || !(2. * radius).is_finite()
    {
        return Err("sphere result is nonfinite or has nonpositive radius".into());
    }
    let mut residual_norm = 0f64;
    let mut maximum = 0f64;
    for (i, p) in points.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        let d: [f64; 3] = std::array::from_fn(|j| p[j] - center[j]);
        let residual = d[0].hypot(d[1]).hypot(d[2]) - radius;
        if !residual.is_finite() {
            return Err("sphere residual overflow".into());
        }
        residual_norm = residual_norm.hypot(residual);
        maximum = maximum.max(residual.abs());
    }
    let rms = residual_norm / count.sqrt();
    if !rms.is_finite() {
        return Err("sphere RMS overflow".into());
    }
    work.check()?;
    Ok(Sphere {
        center_mm: center,
        radius_mm: radius,
        diameter_mm: 2. * radius,
        geometric_rms_mm: rms,
        maximum_absolute_residual_mm: maximum,
        design_condition_number: largest / smallest,
        points: points.len(),
    })
}

#[cfg(test)]
mod angle_tests {
    use super::angle_degrees;
    #[test]
    fn known_angles_and_arm_exchange() {
        for (b, expected) in [
            ([0., 1., 0.], 90.),
            ([1., 1., 0.], 45.),
            ([-1., 0., 0.], 180.),
            ([2., 0., 0.], 0.),
        ] {
            assert!((angle_degrees([1., 0., 0.], [0.; 3], b).unwrap() - expected).abs() < 1e-12);
            assert!((angle_degrees(b, [0.; 3], [1., 0., 0.]).unwrap() - expected).abs() < 1e-12);
        }
        let angle = angle_degrees([11., 20., 30.], [10., 20., 30.], [10., 20., 32.]).unwrap();
        assert_eq!(angle, 90.);
    }
    #[test]
    fn independent_scales_subnormals_and_overflowing_differences() {
        assert_eq!(
            angle_degrees([1e300, 0., 0.], [0.; 3], [0., 1e-300, 0.]).unwrap(),
            90.
        );
        let tiny = f64::from_bits(1);
        assert_eq!(
            angle_degrees([tiny, 0., 0.], [0.; 3], [0., tiny, 0.]).unwrap(),
            90.
        );
        assert!(
            (angle_degrees(
                [f64::MAX, 0., 0.],
                [-f64::MAX, 0., 0.],
                [-f64::MAX, f64::MAX, 0.]
            )
            .unwrap()
                - 90.)
                .abs()
                < 1e-12
        );
    }
    #[test]
    fn invalid_coordinates_and_zero_arms_are_rejected() {
        assert!(angle_degrees([0.; 3], [0.; 3], [1.; 3]).is_err());
        assert!(angle_degrees([1.; 3], [0.; 3], [0.; 3]).is_err());
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(angle_degrees([bad, 0., 0.], [0.; 3], [1.; 3]).is_err());
            assert!(angle_degrees([1.; 3], [bad, 0., 0.], [1.; 3]).is_err());
            assert!(angle_degrees([1.; 3], [0.; 3], [bad, 0., 0.]).is_err());
        }
    }
}
