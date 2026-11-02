//! Image-only planar target association from four distinguishable anchor regions.
//! Original enumeration and matching code using the existing normalized DLT solver.
//! Coordinates describe a supplied pattern, not a physical calibration or unit scale.
use crate::planar::homography;
use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub position: [f64; 2],
    /// Exactly four model and four observed points must have this class.
    pub anchor: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub seed_radius_px: f64,
    pub refined_radius_px: f64,
    /// Includes anchors; at least eight matches are required.
    pub minimum_matches: usize,
    /// Winning seed support must exceed every competitor by this count (>= 1).
    pub minimum_support_margin: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Match {
    pub target_index: usize,
    pub observation_index: usize,
    pub residual_px: f64,
}

#[derive(Debug, Serialize)]
pub struct Association {
    pub matches: Vec<Match>,
    pub seed_support: usize,
    pub runner_up_support: usize,
    /// Row-major pattern-to-image homography; arbitrary homogeneous scale.
    pub homography: [[f64; 3]; 3],
}

fn check(cancelled: &mut impl FnMut() -> bool) -> Result<(), String> {
    if cancelled() {
        Err("target association cancelled".into())
    } else {
        Ok(())
    }
}

fn anchors(points: &[Point]) -> Result<Vec<usize>, String> {
    let mut indices = Vec::new();
    for (i, point) in points.iter().enumerate() {
        if point
            .position
            .iter()
            .any(|x| !x.is_finite() || x.abs() > 1e6)
        {
            return Err("nonfinite or oversized target association coordinates".into());
        }
        if points[..i].iter().any(|p| p.position == point.position) {
            return Err("duplicate target association coordinates".into());
        }
        if point.anchor {
            indices.push(i);
        }
    }
    if indices.len() != 4 {
        return Err("target association requires exactly four anchors in each input".into());
    }
    Ok(indices)
}

/// Reject ambiguous neighborhoods and many-to-one matches instead of resolving
/// them by input order. The entire supplied pattern must stay on one side of the
/// projective horizon. Missing isolated regular points are allowed.
fn assign(
    target: &[Point],
    observed: &[Point],
    h: &Matrix3<f64>,
    radius: f64,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<Option<Vec<Match>>, String> {
    let mut matches = Vec::new();
    let mut used = vec![false; observed.len()];
    let mut sign = 0.;
    for (target_index, point) in target.iter().enumerate() {
        check(cancelled)?;
        let p = h * Vector3::new(point.position[0], point.position[1], 1.);
        if !p.iter().all(|x| x.is_finite()) || p.z.abs() <= 1e-12 * p.norm() {
            return Ok(None);
        }
        if sign != 0. && p.z.signum() != sign {
            return Ok(None);
        }
        sign = p.z.signum();
        let pixel = [p.x / p.z, p.y / p.z];
        let mut found = None;
        for (observation_index, observation) in observed.iter().enumerate() {
            if point.anchor != observation.anchor {
                continue;
            }
            let error =
                (pixel[0] - observation.position[0]).hypot(pixel[1] - observation.position[1]);
            if error <= radius {
                if found.is_some() || used[observation_index] {
                    return Ok(None);
                }
                found = Some(Match {
                    target_index,
                    observation_index,
                    residual_px: error,
                });
            }
        }
        if let Some(m) = found {
            used[m.observation_index] = true;
            matches.push(m);
        } else if point.anchor {
            return Ok(None);
        }
    }
    Ok(Some(matches))
}

/// Enumerate the 24 four-anchor permutations, require a unique support winner,
/// fit one normalized DLT homography to its matches, then associate again using
/// the tighter radius. No camera intrinsics, stereo geometry or known dimensions
/// participate. The caller supplies classes and pattern coordinates explicitly.
///
/// Bounds: 8..=128 target points, 8..=256 observations, finite coordinates within
/// +/-1e6, radii <=1e5. Work is O(24 * targets * observations) plus one bounded
/// DLT fit. Cancellation is checked per candidate/target and around SVD; a single
/// existing bounded SVD call cannot be interrupted. No partial result is returned.
/// Input indices are stable identities only within the caller's supplied arrays.
pub fn associate(
    target: &[Point],
    observed: &[Point],
    config: Config,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Association, String> {
    check(&mut cancelled)?;
    if !(8..=128).contains(&target.len())
        || !(8..=256).contains(&observed.len())
        || !config.seed_radius_px.is_finite()
        || config.seed_radius_px <= 0.
        || config.seed_radius_px > 1e5
        || !config.refined_radius_px.is_finite()
        || config.refined_radius_px <= 0.
        || config.refined_radius_px > config.seed_radius_px
        || config.minimum_matches < 8
        || config.minimum_matches > target.len().min(observed.len())
        || config.minimum_support_margin == 0
        || config.minimum_support_margin > target.len()
    {
        return Err("invalid target association bounds".into());
    }
    let model_anchors = anchors(target)?;
    let image_anchors = anchors(observed)?;
    let source: Vec<_> = model_anchors.iter().map(|&i| target[i].position).collect();
    let mut candidates = Vec::new();
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                if a == b || a == c || b == c {
                    continue;
                }
                let d = 6 - a - b - c;
                check(&mut cancelled)?;
                let destination = [a, b, c, d].map(|i| observed[image_anchors[i]].position);
                let Ok(h) = homography(&source, &destination) else {
                    continue;
                };
                if let Some(matches) =
                    assign(target, observed, &h, config.seed_radius_px, &mut cancelled)?
                {
                    candidates.push(matches);
                }
            }
        }
    }
    candidates.sort_by_key(|matches| std::cmp::Reverse(matches.len()));
    let best = candidates
        .first()
        .ok_or("no valid target anchor hypothesis")?;
    let runner_up_support = candidates.get(1).map_or(0, Vec::len);
    if best.len() < config.minimum_matches
        || best.len() - runner_up_support < config.minimum_support_margin
    {
        return Err("insufficient or ambiguous target support".into());
    }
    check(&mut cancelled)?;
    let source: Vec<_> = best
        .iter()
        .map(|m| target[m.target_index].position)
        .collect();
    let destination: Vec<_> = best
        .iter()
        .map(|m| observed[m.observation_index].position)
        .collect();
    let h = homography(&source, &destination)?;
    let matches = assign(
        target,
        observed,
        &h,
        config.refined_radius_px,
        &mut cancelled,
    )?
    .ok_or("ambiguous refined target association")?;
    // Refinement may find additional points, but cannot silently relabel a seed.
    if matches.len() < config.minimum_matches
        || best.iter().any(|seed| {
            !matches.iter().any(|m| {
                m.target_index == seed.target_index && m.observation_index == seed.observation_index
            })
        })
    {
        return Err("refinement lost or changed a seed association".into());
    }
    check(&mut cancelled)?;
    Ok(Association {
        matches,
        seed_support: best.len(),
        runner_up_support,
        homography: std::array::from_fn(|i| std::array::from_fn(|j| h[(i, j)])),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model() -> Vec<Point> {
        [
            [0., 0.],
            [3., 0.],
            [0., 2.],
            [4., 3.],
            [0.3, 0.6],
            [1.2, 1.5],
            [2.8, 0.7],
            [3.1, 2.3],
            [1.8, 2.8],
            [3.7, 1.2],
        ]
        .into_iter()
        .enumerate()
        .map(|(i, position)| Point {
            position,
            anchor: i < 4,
        })
        .collect()
    }
    fn projected(points: &[Point], mirror: bool) -> Vec<Point> {
        points
            .iter()
            .map(|p| {
                let [x, y] = p.position;
                let w = 1. + 0.03 * x + 0.02 * y;
                let u = (50. * x - 18. * y + 320.) / w;
                Point {
                    position: [
                        if mirror { 900. - u } else { u },
                        (17. * x + 60. * y + 140.) / w,
                    ],
                    anchor: p.anchor,
                }
            })
            .collect()
    }
    fn config() -> Config {
        Config {
            seed_radius_px: 2.,
            refined_radius_px: 1.,
            minimum_matches: 8,
            minimum_support_margin: 2,
        }
    }
    #[test]
    fn projective_mirrored_and_shuffled_inputs_keep_exact_indices() {
        let target = model();
        for mirror in [false, true] {
            let mut obs = projected(&target, mirror);
            obs.reverse();
            let result = associate(&target, &obs, config(), || false).unwrap();
            assert_eq!(result.matches.len(), target.len());
            for m in result.matches {
                assert_eq!(m.observation_index, target.len() - 1 - m.target_index);
                assert!(m.residual_px < 1e-9);
            }
        }
    }
    #[test]
    fn partial_pattern_and_unrelated_clutter_are_allowed() {
        let target = model();
        let mut obs = projected(&target, false);
        obs.pop();
        obs.pop();
        obs.push(Point {
            position: [1., 1.],
            anchor: false,
        });
        let r = associate(&target, &obs, config(), || false).unwrap();
        assert_eq!(r.matches.len(), 8);
        assert!(
            r.matches
                .iter()
                .all(|m| m.target_index == m.observation_index)
        );
    }
    #[test]
    fn symmetric_target_is_rejected_even_with_perfect_residuals() {
        let target: Vec<_> = [
            [0., 0.],
            [4., 0.],
            [4., 4.],
            [0., 4.],
            [1., 1.],
            [3., 1.],
            [3., 3.],
            [1., 3.],
        ]
        .into_iter()
        .enumerate()
        .map(|(i, position)| Point {
            position,
            anchor: i < 4,
        })
        .collect();
        assert!(
            associate(&target, &projected(&target, false), config(), || false)
                .unwrap_err()
                .contains("ambiguous")
        );
    }
    #[test]
    fn nearby_competing_observation_does_not_win_by_input_order() {
        let target = model();
        let mut obs = projected(&target, false);
        let mut extra = obs[4];
        extra.position[0] += 0.1;
        obs.push(extra);
        assert!(associate(&target, &obs, config(), || false).is_err());
        obs.reverse();
        assert!(associate(&target, &obs, config(), || false).is_err());
    }
    #[test]
    fn malformed_degenerate_bounds_and_cancellation_fail_closed() {
        let target = model();
        let obs = projected(&target, false);
        assert!(associate(&target, &obs, config(), || true).is_err());
        let mut calls = 0;
        assert!(
            associate(&target, &obs, config(), || {
                calls += 1;
                calls > 12
            })
            .unwrap_err()
            .contains("cancelled")
        );
        let mut invalid = target.clone();
        invalid[0].position[0] = f64::NAN;
        assert!(associate(&invalid, &obs, config(), || false).is_err());
        invalid = target.clone();
        invalid[1].position = invalid[0].position;
        assert!(associate(&invalid, &obs, config(), || false).is_err());
        invalid = target.clone();
        invalid[0].anchor = false;
        assert!(associate(&invalid, &obs, config(), || false).is_err());
        invalid = target.clone();
        for (i, p) in invalid[..4].iter_mut().enumerate() {
            p.position = [i as f64, 0.];
        }
        assert!(associate(&invalid, &obs, config(), || false).is_err());
        let mut c = config();
        c.minimum_support_margin = 0;
        assert!(associate(&target, &obs, c, || false).is_err());
        c = config();
        c.refined_radius_px = f64::NAN;
        assert!(associate(&target, &obs, c, || false).is_err());
        assert!(associate(&vec![target[0]; 129], &obs, config(), || false).is_err());
    }

    #[test]
    fn noisy_refinement_keeps_correspondences_and_rejects_an_unmet_tolerance() {
        let target = model();
        let mut obs = projected(&target, false);
        for (i, p) in obs.iter_mut().enumerate() {
            p.position[0] += (i as f64 * 1.71).sin() * 0.1;
            p.position[1] += (i as f64 * 2.31).cos() * 0.1;
        }
        let r = associate(&target, &obs, config(), || false).unwrap();
        assert_eq!(r.matches.len(), target.len());
        assert!(
            r.matches
                .iter()
                .all(|m| m.target_index == m.observation_index)
        );
        assert!(r.matches.iter().any(|m| m.residual_px > 0.01));
        let mut strict = config();
        strict.refined_radius_px = 1e-6;
        assert!(associate(&target, &obs, strict, || false).is_err());
    }

    #[test]
    fn projective_horizon_and_many_to_one_neighborhoods_are_rejected() {
        let mut target = model();
        let obs = projected(&target, false);
        let crossing = Matrix3::new(1., 0., 0., 0., 1., 0., 1., 0., -1.);
        assert!(
            assign(&target, &obs, &crossing, 1000., &mut || false)
                .unwrap()
                .is_none()
        );
        let mut same = target.clone();
        target[5].position = [0.31, 0.6];
        same[5].position = [100., 100.];
        assert!(
            assign(&target, &same, &Matrix3::identity(), 0.02, &mut || false)
                .unwrap()
                .is_none()
        );
    }
}
