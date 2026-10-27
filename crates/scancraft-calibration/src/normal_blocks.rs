//! Shared small normal-equation blocks; current consumers have 6 or 10 globals.
//! Independent standard Schur elimination, not imported solver code. Observations
//! are accumulated by callers; no observations-by-all-parameters Jacobian is kept.
use nalgebra::{SMatrix, SVector};
pub(crate) type PoseVector = SVector<f64, 6>;
/// One independent six-parameter target pose, coupled to G shared parameters.
pub(crate) struct ViewNormal<const G: usize> {
    pub pose: SMatrix<f64, 6, 6>,
    pub cross: SMatrix<f64, G, 6>,
    pub gradient: PoseVector,
}
/// Caller-assembled symmetric normal equations, before damping/elimination.
pub(crate) struct NormalSystem<const G: usize> {
    pub global: SMatrix<f64, G, G>,
    pub gradient: SVector<f64, G>,
    pub views: Vec<ViewNormal<G>>,
}
/// Jacobi-normalized gradient infinity norm, shared by convergence checks.
pub(crate) fn scaled_gradient_inf<const G: usize>(system: &NormalSystem<G>) -> f64 {
    let mut value = (0..G)
        .map(|i| system.gradient[i].abs() / system.global[(i, i)].max(1e-18).sqrt())
        .fold(0., f64::max);
    for view in &system.views {
        for i in 0..6 {
            value = value.max(view.gradient[i].abs() / view.pose[(i, i)].max(1e-18).sqrt());
        }
    }
    value
}

fn damped<const N: usize>(mut h: SMatrix<f64, N, N>, damping: f64) -> SMatrix<f64, N, N> {
    for i in 0..N {
        h[(i, i)] += damping * h[(i, i)].max(1e-18);
    }
    h
}
/// Eliminate damped independent pose blocks and back-substitute a joint LM step.
pub(crate) fn solve_step<const G: usize>(
    system: &NormalSystem<G>,
    damping: f64,
) -> Result<(SVector<f64, G>, Vec<PoseVector>), String> {
    let mut schur = damped(system.global, damping);
    let mut rhs = -system.gradient;
    let mut solved = Vec::with_capacity(system.views.len());
    for view in &system.views {
        let factor = damped(view.pose, damping)
            .cholesky()
            .ok_or("singular damped pose block")?;
        let cross = factor.solve(&view.cross.transpose());
        let gradient = factor.solve(&view.gradient);
        schur -= view.cross * cross;
        rhs += view.cross * gradient;
        solved.push((cross, gradient));
    }
    let step = ((schur + schur.transpose()) * 0.5)
        .cholesky()
        .ok_or("singular damped global system")?
        .solve(&rhs);
    let poses: Vec<_> = solved
        .into_iter()
        .map(|(cross, gradient)| -gradient - cross * step)
        .collect();
    if !step
        .iter()
        .chain(poses.iter().flat_map(|p| p.iter()))
        .all(|v| v.is_finite())
    {
        return Err("calibration step overflow".into());
    }
    Ok((step, poses))
}

/// Undamped scaled Schur rank diagnostic; not covariance or physical confidence.
pub(crate) fn information_min_eigenvalue<const G: usize>(
    system: &NormalSystem<G>,
) -> Result<f64, String> {
    let mut schur = system.global;
    for view in &system.views {
        let factor = view.pose.cholesky().ok_or("unobservable target pose")?;
        schur -= view.cross * factor.solve(&view.cross.transpose());
    }
    schur = (schur + schur.transpose()) * 0.5;
    let diagonal = schur.diagonal();
    if diagonal.iter().any(|d| !d.is_finite() || *d <= 0.) {
        return Err("unobservable global calibration parameters".into());
    }
    for i in 0..G {
        for j in 0..G {
            schur[(i, j)] /= diagonal[i].sqrt() * diagonal[j].sqrt();
        }
    }
    let eigen = nalgebra::linalg::SymmetricEigen::try_new(
        nalgebra::DMatrix::from_column_slice(G, G, schur.as_slice()),
        1e-14,
        1000,
    )
    .ok_or("information eigensystem did not converge")?;
    let minimum = eigen.eigenvalues.min();
    if !minimum.is_finite() || minimum < 1e-10 {
        return Err("weak or degenerate calibration information".into());
    }
    Ok(minimum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{DMatrix, DVector};
    fn dense_comparison<const G: usize>() {
        let mut system = NormalSystem {
            global: SMatrix::<f64, G, G>::identity(),
            gradient: SVector::<f64, G>::zeros(),
            views: Vec::new(),
        };
        for view in 0..3 {
            let mut block = ViewNormal {
                pose: SMatrix::<f64, 6, 6>::identity(),
                cross: SMatrix::<f64, G, 6>::zeros(),
                gradient: PoseVector::zeros(),
            };
            for row in 0..(G + 12) {
                let g = SVector::<f64, G>::from_fn(|i, _| ((row + 1) * (i + 1)) as f64 * 0.17)
                    .map(f64::sin);
                let p =
                    PoseVector::from_fn(|i, _| (((row + 1) * (i + 2) + view) as f64 * 0.23).cos());
                let r = (row + view) as f64 * 0.03;
                system.global += g * g.transpose();
                system.gradient += g * r;
                block.pose += p * p.transpose();
                block.cross += g * p.transpose();
                block.gradient += p * r;
            }
            system.views.push(block);
        }
        for damping in [0., 0.001, 10.] {
            let n = G + 18;
            let mut dense = DMatrix::zeros(n, n);
            let mut gradient = DVector::zeros(n);
            for i in 0..G {
                gradient[i] = system.gradient[i];
                for j in 0..G {
                    dense[(i, j)] = system.global[(i, j)];
                }
            }
            for (v, block) in system.views.iter().enumerate() {
                let offset = G + v * 6;
                for i in 0..6 {
                    gradient[offset + i] = block.gradient[i];
                    for j in 0..6 {
                        dense[(offset + i, offset + j)] = block.pose[(i, j)];
                    }
                    for j in 0..G {
                        dense[(j, offset + i)] = block.cross[(j, i)];
                        dense[(offset + i, j)] = block.cross[(j, i)];
                    }
                }
            }
            for i in 0..n {
                dense[(i, i)] += damping * dense[(i, i)].max(1e-18);
            }
            let expected = dense.cholesky().unwrap().solve(&(-gradient));
            let (g, p) = solve_step(&system, damping).unwrap();
            for i in 0..G {
                assert!((g[i] - expected[i]).abs() < 1e-10);
            }
            for v in 0..3 {
                for i in 0..6 {
                    assert!((p[v][i] - expected[G + v * 6 + i]).abs() < 1e-10);
                }
            }
        }
    }
    #[test]
    fn schur_matches_independent_full_dense_system_for_both_consumers() {
        dense_comparison::<6>();
        dense_comparison::<10>();
    }
}
