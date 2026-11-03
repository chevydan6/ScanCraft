//! Normalized general-conic least squares followed by ellipse validation.
//! Original implementation, not Fitzgibbon's ellipse-constrained direct method.
//! General conic model/reference: Fitzgibbon et al., PAMI 1999, section 2,
//! https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/ellipse-pami.pdf
use nalgebra::{DMatrix, Matrix2, Vector2, linalg::SVD};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Fit {
    pub center: [f64; 2],
    /// Major then minor semiaxis, in the input coordinate units.
    pub semiaxes: [f64; 2],
    /// Unit major-axis direction; sign is arbitrary, orientation undefined for circles.
    pub major_direction: [f64; 2],
    pub samples: usize,
    /// First-order |F| / ||gradient F||, not exact orthogonal distance.
    pub rms_sampson: f64,
    pub maximum_sampson: f64,
}

fn check(cancelled: &mut impl FnMut() -> bool) -> Result<(), String> {
    if cancelled() {
        Err("ellipse fit cancelled".into())
    } else {
        Ok(())
    }
}

/// Fit 6..=4096 finite boundary samples with coordinates bounded to +/-1e6.
/// Center/RMS-radius normalization precedes six-column homogeneous SVD, with
/// unit coefficient norm. Nonelliptical, rank-deficient or ill-conditioned fits
/// return errors. No physical circle-center or complete-contour claim is made.
/// Cancellation is checked around the bounded SVD and per 128 input/residual
/// samples; an individual SVD call is not interruptible. No partial fit is returned.
pub fn fit(points: &[[f64; 2]], mut cancelled: impl FnMut() -> bool) -> Result<Fit, String> {
    check(&mut cancelled)?;
    if !(6..=4096).contains(&points.len())
        || points
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1e6)
    {
        return Err("invalid ellipse sample count or coordinates".into());
    }
    let n = points.len() as f64;
    let origin = points
        .iter()
        .fold(Vector2::zeros(), |a, p| a + Vector2::from(*p) / n);
    let scale = (points
        .iter()
        .map(|p| (Vector2::from(*p) - origin).norm_squared() / n)
        .sum::<f64>())
    .sqrt();
    if !scale.is_finite() || scale <= 1e-9 {
        return Err("coincident ellipse samples".into());
    }
    let mut design = DMatrix::zeros(points.len(), 6);
    for (i, p) in points.iter().enumerate() {
        if i % 128 == 0 {
            check(&mut cancelled)?;
        }
        let v = (Vector2::from(*p) - origin) / scale;
        let row = [v.x * v.x, v.x * v.y, v.y * v.y, v.x, v.y, 1.];
        for j in 0..6 {
            design[(i, j)] = row[j];
        }
    }
    let svd =
        SVD::try_new(design, false, true, 1e-14, 10_000).ok_or("ellipse SVD did not converge")?;
    check(&mut cancelled)?;
    let s = svd.singular_values;
    if s.iter().any(|v| !v.is_finite()) || s[0] <= 0. || s[4] / s[0] < 1e-8 || s[5] / s[4] > 0.5 {
        return Err("degenerate or inconsistent ellipse samples".into());
    }
    let v = svd.v_t.ok_or("missing ellipse singular vectors")?;
    let q = Matrix2::new(v[(5, 0)], v[(5, 1)] / 2., v[(5, 1)] / 2., v[(5, 2)]);
    let linear = Vector2::new(v[(5, 3)], v[(5, 4)]);
    let center = -0.5 * q.try_inverse().ok_or("singular conic quadratic")? * linear;
    let level = (center.transpose() * q * center)[0] - v[(5, 5)];
    if !level.is_finite() || level.abs() < 1e-14 {
        return Err("degenerate conic level".into());
    }
    let metric = q / level;
    let eigen = metric.symmetric_eigen();
    if eigen
        .eigenvalues
        .iter()
        .any(|x| !x.is_finite() || *x <= 1e-12)
    {
        return Err("fitted conic is not a bounded ellipse".into());
    }
    let major = usize::from(eigen.eigenvalues[1] < eigen.eigenvalues[0]);
    let semiaxes = [
        scale / eigen.eigenvalues[major].sqrt(),
        scale / eigen.eigenvalues[1 - major].sqrt(),
    ];
    let center_px = origin + scale * center;
    if center_px.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
        || semiaxes
            .iter()
            .any(|v| !v.is_finite() || *v < 1e-6 || *v > 1e6)
    {
        return Err("unrepresentable ellipse geometry".into());
    }
    let mut squared = 0.;
    let mut maximum: f64 = 0.;
    for (i, p) in points.iter().enumerate() {
        if i % 128 == 0 {
            check(&mut cancelled)?;
        }
        let delta = (Vector2::from(*p) - origin) / scale - center;
        let gradient = 2. * metric * delta;
        let residual =
            ((delta.transpose() * metric * delta)[0] - 1.).abs() / gradient.norm() * scale;
        if !residual.is_finite() {
            return Err("undefined ellipse distance approximation".into());
        }
        squared += residual * residual / n;
        maximum = maximum.max(residual);
    }
    if !squared.is_finite() {
        return Err("ellipse residual overflow".into());
    }
    check(&mut cancelled)?;
    Ok(Fit {
        center: center_px.into(),
        semiaxes,
        major_direction: eigen.eigenvectors.column(major).into_owned().into(),
        samples: points.len(),
        rms_sampson: squared.sqrt(),
        maximum_sampson: maximum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn points(a: f64, b: f64) -> Vec<[f64; 2]> {
        (0..100)
            .map(|i| {
                let t = i as f64 * std::f64::consts::TAU / 100.;
                let (x, y) = (a * t.cos(), b * t.sin());
                [300. + 0.8 * x - 0.6 * y, -20. + 0.6 * x + 0.8 * y]
            })
            .collect()
    }
    #[test]
    fn analytic_ellipse_and_circle_preserve_centers_axes_and_residuals() {
        for (a, b) in [(17., 8.), (9., 9.)] {
            let r = fit(&points(a, b), || false).unwrap();
            assert!((r.center[0] - 300.).abs() < 1e-9 && (r.center[1] + 20.).abs() < 1e-9);
            assert!((r.semiaxes[0] - a).abs() < 1e-9 && (r.semiaxes[1] - b).abs() < 1e-9);
            assert!(r.maximum_sampson < 1e-9);
            if a != b {
                assert!(
                    (r.major_direction[0] * 0.8 + r.major_direction[1] * 0.6).abs() > 1. - 1e-9
                );
            }
        }
    }
    #[test]
    fn straight_cut_in_a_circle_increases_residual_without_repairing_center() {
        let clean = points(15., 15.);
        let mut cut = clean.clone();
        for p in &mut cut {
            p[0] = p[0].max(296.);
        }
        let r = fit(&cut, || false).unwrap();
        assert!(r.rms_sampson > 0.5);
        assert!((r.center[0] - 300.).abs() > 1.);
    }
    #[test]
    fn lines_hyperbolae_nonfinite_oversize_and_cancellation_are_errors() {
        let line: Vec<_> = (0..20).map(|i| [i as f64, 2. * i as f64]).collect();
        assert!(fit(&line, || false).is_err());
        let hyperbola: Vec<_> = (0..30)
            .map(|i| {
                let t = -1. + i as f64 / 15.;
                [t.cosh(), t.sinh()]
            })
            .collect();
        assert!(fit(&hyperbola, || false).is_err());
        let mut bad = points(4., 2.);
        bad[0][0] = f64::NAN;
        assert!(fit(&bad, || false).is_err());
        assert!(fit(&[[0., 0.]; 4097], || false).is_err());
        assert!(fit(&[[0., 0.]; 8], || false).is_err());
        assert!(fit(&points(4., 2.), || true).is_err());
        let mut calls = 0;
        assert!(
            fit(&points(4., 2.), || {
                calls += 1;
                calls > 2
            })
            .unwrap_err()
            .contains("cancelled")
        );
    }
    #[test]
    fn similarity_transform_and_input_order_preserve_fit() {
        let p = points(17., 8.);
        let baseline = fit(&p, || false).unwrap();
        let q: Vec<_> = p
            .iter()
            .rev()
            .map(|p| [p[0] * 3. + 9000., p[1] * 3. - 7000.])
            .collect();
        let r = fit(&q, || false).unwrap();
        for i in 0..2 {
            assert!((r.semiaxes[i] - 3. * baseline.semiaxes[i]).abs() < 1e-8);
        }
        assert!((r.center[0] - (baseline.center[0] * 3. + 9000.)).abs() < 1e-8);
    }
}
