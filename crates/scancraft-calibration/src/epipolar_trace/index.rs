//! Independently implemented conservative culling of normalized trace segments.
//! Bounds include both endpoints of every segment. No candidate is selected here.
use nalgebra::Vector3;

pub(super) const BLOCK_SEGMENTS: usize = 64;
pub(super) const MAX_TRACE_VERTICES: usize = 4096;
#[derive(Debug)]
pub(super) struct Block {
    pub first: usize,
    pub end: usize,
    pub min: [f64; 2],
    pub max: [f64; 2],
}
impl Block {
    pub(super) fn may_cross(&self, line: Vector3<f64>, tolerance: f64) -> Option<bool> {
        let magnitude = line.x.abs() * self.min[0].abs().max(self.max[0].abs())
            + line.y.abs() * self.min[1].abs().max(self.max[1].abs())
            + line.z.abs()
            + 1.;
        if !magnitude.is_finite() || magnitude > f64::MAX / 16. {
            return None;
        }
        let low = line.x
            * if line.x >= 0. {
                self.min[0]
            } else {
                self.max[0]
            }
            + line.y
                * if line.y >= 0. {
                    self.min[1]
                } else {
                    self.max[1]
                }
            + line.z;
        let high = line.x
            * if line.x >= 0. {
                self.max[0]
            } else {
                self.min[0]
            }
            + line.y
                * if line.y >= 0. {
                    self.max[1]
                } else {
                    self.min[1]
                }
            + line.z;
        // With round-to-nearest f64, at most three products and two additions
        // form a distance. The bound uses the sum of absolute term maxima; 32ε
        // covers rounding in that sum, both extrema, and the scalar dot product.
        // The +1 term also dominates subnormal absolute error. MAX/16 headroom
        // rules out an intermediate overflow. Uncertain bounds retain the block.
        let margin = 32. * f64::EPSILON * magnitude;
        let threshold = tolerance + margin;
        if !low.is_finite() || !high.is_finite() || !threshold.is_finite() {
            return None;
        }
        Some(!(low > threshold || high < -threshold))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_rejection_is_conservative_for_rounding_tolerance_and_extreme_scales() {
        let mut seed = 0x518e_a235_91fa_b66du64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 11) as f64 / ((1u64 << 53) as f64)) * 2. - 1.
        };
        let mut rejected = 0;
        let mut fallback = 0;
        for exponent in [-1000, -500, -20, 0, 20, 500, 1000] {
            for _ in 0..500 {
                let scale = 2_f64.powi(exponent);
                let points = (0..65)
                    .map(|_| [next() * scale, next() * scale])
                    .collect::<Vec<_>>();
                let mut min = [f64::INFINITY; 2];
                let mut max = [f64::NEG_INFINITY; 2];
                for p in &points {
                    for axis in 0..2 {
                        min[axis] = min[axis].min(p[axis]);
                        max[axis] = max[axis].max(p[axis]);
                    }
                }
                let block = Block {
                    first: 0,
                    end: 64,
                    min,
                    max,
                };
                let a = next();
                let b = next();
                // Query through a sampled vertex, immediately around its tolerance,
                // as well as on either side and with risky arithmetic magnitudes.
                let through = -(a * points[32][0] + b * points[32][1]);
                for c in [
                    through,
                    through + 1e-10,
                    through - 1e-10,
                    4. * scale,
                    -4. * scale,
                    f64::MAX,
                ] {
                    let line = Vector3::new(a, b, c);
                    match block.may_cross(line, 1e-10) {
                        None => fallback += 1,
                        Some(true) => {}
                        Some(false) => {
                            rejected += 1;
                            let distances = points
                                .iter()
                                .map(|p| line.dot(&Vector3::new(p[0], p[1], 1.)))
                                .collect::<Vec<_>>();
                            assert!(distances.iter().all(|v| v.is_finite() && v.abs() > 1e-10));
                            assert!(
                                distances.iter().all(
                                    |v| v.is_sign_positive() == distances[0].is_sign_positive()
                                )
                            );
                        }
                    }
                }
            }
        }
        assert!(rejected > 1000 && fallback > 1000);
    }

    #[test]
    fn numerical_margin_keeps_close_and_cancelling_terms() {
        for y in [0., 1e-10, 1e-10 + f64::EPSILON, -1e-10] {
            let block = Block {
                first: 0,
                end: 64,
                min: [0., y],
                max: [1., y],
            };
            assert_eq!(block.may_cross(Vector3::new(0., 1., 0.), 1e-10), Some(true));
        }
        let block = Block {
            first: 0,
            end: 64,
            min: [1e150, 1e150],
            max: [1e150, 1e150],
        };
        assert_eq!(
            block.may_cross(Vector3::new(1., -1., 0.), 1e-10),
            Some(true)
        );
        assert_eq!(block.may_cross(Vector3::new(f64::MAX, 1., 0.), 1e-10), None);
    }
}
