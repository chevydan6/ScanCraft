//! Curved laser sheet geometry, independent of scanner-specific field binding.
//! Coordinates share the caller's length unit. No conversion from factory
//! records is provided: their model, units and projector identity are unbound.
use serde::{Deserialize, Serialize};

/// Sheet equation X = quadratic_y * Y² + linear_y * Y + linear_z * Z + offset.
/// quadratic_y has inverse-length units; offset has length units. The other
/// coefficients are dimensionless. All coordinates are in the camera frame.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuadraticSheet {
    pub quadratic_y: f64,
    pub linear_y: f64,
    pub linear_z: f64,
    pub offset: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum RaySheetIntersections {
    None,
    One([f64; 3]),
    /// Ordered by positive depth. A caller needs independent correspondence
    /// evidence to resolve this ambiguity; no nearer-root policy is implied.
    Two([[f64; 3]; 2]),
}
impl QuadraticSheet {
    /// Intersect the undistorted ray (nx*Z, ny*Z, Z) with this sheet inside a
    /// positive closed Z interval. Exact linear sheets stay linear: no epsilon
    /// is added to curvature. Stable quadratic roots avoid subtracting nearly
    /// equal values. Contained rays and numerical overflow fail explicitly.
    pub fn intersect(
        self,
        normalized_ray: [f64; 2],
        depth_interval: [f64; 2],
    ) -> Result<RaySheetIntersections, String> {
        let [nx, ny] = normalized_ray;
        let [near, far] = depth_interval;
        if ![
            self.quadratic_y,
            self.linear_y,
            self.linear_z,
            self.offset,
            nx,
            ny,
            near,
            far,
        ]
        .iter()
        .all(|v| v.is_finite())
            || near <= 0.
            || far < near
        {
            return Err("invalid laser sheet, ray or depth interval".into());
        }
        let a = self.quadratic_y * ny * ny;
        let b = self.linear_y.mul_add(ny, self.linear_z) - nx;
        let c = self.offset;
        if !a.is_finite() || !b.is_finite() || (a == 0. && self.quadratic_y != 0. && ny != 0.) {
            return Err("laser sheet coefficient overflow or underflow".into());
        }
        let scale = a.abs().max(b.abs()).max(c.abs());
        if scale == 0. {
            return Err("ray lies in laser sheet; intersection is not unique".into());
        }
        let scaled = [a / scale, b / scale, c / scale];
        if [a, b, c]
            .iter()
            .zip(scaled)
            .any(|(original, normalized)| *original != 0. && normalized == 0.)
        {
            return Err("laser sheet coefficients exceed numerical dynamic range".into());
        }
        let [a, b, c] = scaled;
        let roots = if a == 0. {
            if b == 0. {
                return Ok(RaySheetIntersections::None);
            }
            [-c / b, f64::NAN]
        } else {
            let discriminant = b.mul_add(b, -4. * a * c);
            if discriminant < 0. {
                return Ok(RaySheetIntersections::None);
            }
            if discriminant == 0. {
                [-b / (2. * a), f64::NAN]
            } else {
                let q = -0.5 * (b + discriminant.sqrt().copysign(b));
                [q / a, c / q]
            }
        };
        let mut points = [[0.; 3]; 2];
        let mut count = 0;
        for z in roots {
            // Nonfinite roots are necessarily outside the finite depth range.
            if !z.is_finite() || z < near || z > far {
                continue;
            }
            let point = [nx * z, ny * z, z];
            let az = a * z;
            let residual = az.mul_add(z, b.mul_add(z, c)).abs();
            let magnitude = (az * z).abs() + (b * z).abs() + c.abs();
            if !point.iter().all(|v| v.is_finite())
                || !residual.is_finite()
                || !magnitude.is_finite()
                || residual > 64. * f64::EPSILON * magnitude
            {
                return Err("laser sheet intersection is numerically unstable".into());
            }
            if count == 0 || points[0][2] != z {
                points[count] = point;
                count += 1;
            }
        }
        if count == 2 && points[0][2] > points[1][2] {
            points.swap(0, 1);
        }
        Ok(match count {
            0 => RaySheetIntersections::None,
            1 => RaySheetIntersections::One(points[0]),
            2 => RaySheetIntersections::Two(points),
            _ => unreachable!("quadratic has at most two isolated roots"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Deserialize)]
    struct Reference {
        cases: Vec<Case>,
    }
    #[derive(Deserialize)]
    struct Case {
        coefficients: [f64; 4],
        normalized_ray: [f64; 2],
        reference_roots: [Option<f64>; 2],
    }
    fn sheet([a, b, c, d]: [f64; 4]) -> QuadraticSheet {
        QuadraticSheet {
            quadratic_y: a,
            linear_y: b,
            linear_z: c,
            offset: d,
        }
    }
    fn points(hits: RaySheetIntersections) -> Vec<[f64; 3]> {
        match hits {
            RaySheetIntersections::None => vec![],
            RaySheetIntersections::One(a) => vec![a],
            RaySheetIntersections::Two(a) => a.to_vec(),
        }
    }
    #[test]
    fn agrees_with_seven_synthetic_gpu_reference_cases_in_bounded_domain() {
        let fixture: Reference = serde_json::from_str(include_str!(
            "../tests/fixtures/laser-surface-reference.json"
        ))
        .unwrap();
        assert_eq!(fixture.cases.len(), 7);
        for case in fixture.cases {
            let surface = sheet(case.coefficients);
            let hits = points(surface.intersect(case.normalized_ray, [1., 1000.]).unwrap());
            let mut expected: Vec<_> = case
                .reference_roots
                .into_iter()
                .flatten()
                .filter(|z| (1. ..=1000.).contains(z))
                .collect();
            expected.sort_by(f64::total_cmp);
            expected.dedup();
            assert_eq!(hits.len(), expected.len());
            for (point, z) in hits.iter().zip(expected) {
                // The reference is f32 and adds 1e-10 to curvature; production
                // deliberately solves the actual sheet without that bias.
                assert!((point[2] - z).abs() < 0.001);
                let [x, y, z] = *point;
                let [a, b, c, d] = case.coefficients;
                assert!((x - (a * y * y + b * y + c * z + d)).abs() < 1e-9);
            }
        }
    }
    #[test]
    fn linear_tangent_two_root_and_empty_cases_preserve_ambiguity() {
        assert_eq!(
            sheet([0., 0., 0., 100.])
                .intersect([0.25, 0.1], [100., 500.])
                .unwrap(),
            RaySheetIntersections::One([100., 40., 400.])
        );
        assert_eq!(
            sheet([1., 0., 0., 1.])
                .intersect([2., 1.], [1., 1.])
                .unwrap(),
            RaySheetIntersections::One([2., 1., 1.])
        );
        let roots = points(
            sheet([0.001, 0., 0., 100.])
                .intersect([1., 1.], [100., 1000.])
                .unwrap(),
        );
        assert_eq!(roots.len(), 2);
        assert!(roots[0][2] < roots[1][2]);
        assert_eq!(
            sheet([1., 0., 0., 1.])
                .intersect([0., 1.], [1., 1000.])
                .unwrap(),
            RaySheetIntersections::None
        );
        assert_eq!(
            sheet([0., 0., 0., 100.])
                .intersect([-0.25, 0.], [1., 1000.])
                .unwrap(),
            RaySheetIntersections::None
        );
        assert!(sheet([0.; 4]).intersect([0., 0.], [1., 1000.]).is_err());
    }
    #[test]
    fn stable_small_root_survives_large_root_and_validates_numeric_inputs() {
        let roots = points(
            sheet([1e-12, 0., 0., 1.])
                .intersect([1., 1.], [0.5, 2.])
                .unwrap(),
        );
        assert_eq!(roots.len(), 1);
        assert!((roots[0][2] - 1.).abs() < 2e-12);
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                sheet([value, 0., 0., 1.])
                    .intersect([1., 1.], [1., 1000.])
                    .is_err()
            );
            assert!(
                sheet([0., 0., 0., 1.])
                    .intersect([value, 1.], [1., 1000.])
                    .is_err()
            );
        }
        for interval in [[0., 1.], [-1., 1.], [2., 1.], [1., f64::INFINITY]] {
            assert!(
                sheet([0., 0., 0., 1.])
                    .intersect([1., 1.], interval)
                    .is_err()
            );
        }
        assert!(
            sheet([f64::MAX, 0., 0., 1.])
                .intersect([1., f64::MAX], [1., 2.])
                .is_err()
        );
    }
}
