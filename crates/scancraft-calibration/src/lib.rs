//! Independent camera geometry. Valid numerical models are not physical accuracy qualification.
//! Camera axes: +X right, +Y down, +Z forward; extrinsic translations are millimeters.
use serde::{Deserialize, Serialize};

pub mod epipolar;
pub mod factory;
pub mod factory_brown;
pub mod factory_laser;
pub mod factory_named;
pub mod factory_pose;
pub mod laser_surface;
mod normal_blocks;
pub mod orientation;
pub mod planar;
mod projection_differential;
pub mod rectification;
pub mod refinement;
pub mod stereo;
pub mod stereo_fit;
pub mod storage;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Brown5 {
    pub k1: f64,
    pub k2: f64,
    pub k3: f64,
    pub p1: f64,
    pub p2: f64,
}
impl Brown5 {
    pub const ZERO: Self = Self {
        k1: 0.,
        k2: 0.,
        k3: 0.,
        p1: 0.,
        p2: 0.,
    };
    /// Normalized projection and analytic Jacobian. No pixel scaling is applied here.
    fn map(self, x: f64, y: f64) -> Result<([f64; 2], [[f64; 2]; 2]), String> {
        let r2 = x * x + y * y;
        let r4 = r2 * r2;
        let radial = 1. + self.k1 * r2 + self.k2 * r4 + self.k3 * r4 * r2;
        let derivative = self.k1 + 2. * self.k2 * r2 + 3. * self.k3 * r4;
        let dx = 2. * x * derivative;
        let dy = 2. * y * derivative;
        let point = [
            x * radial + 2. * self.p1 * x * y + self.p2 * (r2 + 2. * x * x),
            y * radial + self.p1 * (r2 + 2. * y * y) + 2. * self.p2 * x * y,
        ];
        let jacobian = [
            [
                radial + x * dx + 2. * self.p1 * y + 6. * self.p2 * x,
                x * dy + 2. * self.p1 * x + 2. * self.p2 * y,
            ],
            [
                y * dx + 2. * self.p1 * x + 2. * self.p2 * y,
                radial + y * dy + 6. * self.p1 * y + 2. * self.p2 * x,
            ],
        ];
        if !point
            .iter()
            .chain(jacobian.iter().flatten())
            .all(|v| v.is_finite())
        {
            return Err("distortion calculation overflow".into());
        }
        Ok((point, jacobian))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CameraModel {
    pub width: u32,
    pub height: u32,
    pub fx: f64,
    pub fy: f64,
    pub cx: f64,
    pub cy: f64,
    pub skew: f64,
    pub distortion: Brown5,
    /// Radius of the supported undistorted normalized-image disk. Must come from calibration coverage.
    pub valid_radius: f64,
}
/// Min of a cubic over [0,1], including all interior stationary points.
fn cubic_min(c: [f64; 4]) -> Result<f64, String> {
    if !c.iter().all(|v| v.is_finite()) {
        return Err("distortion-domain polynomial overflow".into());
    }
    let mut candidates = vec![0., 1.];
    let (a, b, d) = (3. * c[3], 2. * c[2], c[1]);
    let scale = a.abs().max(b.abs()).max(d.abs());
    if scale > 0. {
        let (a, b, d) = (a / scale, b / scale, d / scale);
        if a == 0. {
            if b != 0. {
                let u = -d / b;
                if (0. ..=1.).contains(&u) {
                    candidates.push(u);
                }
            }
        } else {
            let discriminant = b * b - 4. * a * d;
            if discriminant >= 0. {
                let q = -0.5 * (b + discriminant.sqrt().copysign(b));
                if q == 0. {
                    candidates.push(0.);
                } else {
                    for u in [q / a, d / q] {
                        if (0. ..=1.).contains(&u) {
                            candidates.push(u);
                        }
                    }
                }
            }
        }
    }
    let minimum = candidates
        .into_iter()
        .map(|u| ((c[3] * u + c[2]) * u + c[1]) * u + c[0])
        .fold(f64::INFINITY, f64::min);
    if !minimum.is_finite() {
        return Err("invalid distortion-domain minimum".into());
    }
    Ok(minimum)
}
/// Immutable validated camera view for repeated projection without per-pixel model checks.
pub struct PreparedCamera<'a> {
    model: &'a CameraModel,
}
impl PreparedCamera<'_> {
    pub fn project(&self, point_mm: [f64; 3]) -> Result<[f64; 2], String> {
        self.model.project_validated(point_mm)
    }
    pub fn ray(&self, pixel: [f64; 2]) -> Result<[f64; 3], String> {
        self.model.ray_validated(pixel)
    }
}
impl CameraModel {
    pub fn prepare(&self) -> Result<PreparedCamera<'_>, String> {
        self.validate()?;
        Ok(PreparedCamera { model: self })
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0
            || self.height == 0
            || ![
                self.fx,
                self.fy,
                self.cx,
                self.cy,
                self.skew,
                self.valid_radius,
                self.distortion.k1,
                self.distortion.k2,
                self.distortion.k3,
                self.distortion.p1,
                self.distortion.p2,
            ]
            .iter()
            .all(|v| v.is_finite())
            || self.fx <= 0.
            || self.fy <= 0.
            || self.valid_radius <= 0.
        {
            return Err("invalid camera dimensions or coefficients".into());
        }
        let s = self.valid_radius * self.valid_radius;
        let k = self.distortion;
        let radial = [1., k.k1 * s, k.k2 * s * s, k.k3 * s * s * s];
        let derivative = [1., 3. * radial[1], 5. * radial[2], 7. * radial[3]];
        let minimum = cubic_min(radial)?.min(cubic_min(derivative)?);
        // A conservative symmetric-Jacobian bound guarantees strong monotonicity on the disk.
        let tangential_bound = 8. * self.valid_radius * (k.p1.abs() + k.p2.abs());
        let numeric_margin = 1e-10_f64
            .max(256. * f64::EPSILON * derivative.iter().map(|v| v.abs()).fold(1., f64::max));
        if !tangential_bound.is_finite() || minimum <= tangential_bound + numeric_margin {
            return Err(
                "folded or insufficiently conditioned distortion model over its supported disk"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn contains_pixel(&self, p: [f64; 2]) -> bool {
        p.iter().all(|v| v.is_finite())
            && p[0] >= -0.5
            && p[1] >= -0.5
            && p[0] < f64::from(self.width) - 0.5
            && p[1] < f64::from(self.height) - 0.5
    }
    pub fn project(&self, point_mm: [f64; 3]) -> Result<[f64; 2], String> {
        self.validate()?;
        self.project_validated(point_mm)
    }
    pub(crate) fn project_validated(&self, point_mm: [f64; 3]) -> Result<[f64; 2], String> {
        if !point_mm.iter().all(|v| v.is_finite()) || point_mm[2] <= 0. {
            return Err("point must be finite and in front of the camera".into());
        }
        let x = point_mm[0] / point_mm[2];
        let y = point_mm[1] / point_mm[2];
        if !x.is_finite() || !y.is_finite() || x.hypot(y) > self.valid_radius {
            return Err("point is outside the calibrated lens domain".into());
        }
        let (d, _) = self.distortion.map(x, y)?;
        let pixel = [
            self.fx * d[0] + self.skew * d[1] + self.cx,
            self.fy * d[1] + self.cy,
        ];
        if !pixel.iter().all(|v| v.is_finite()) {
            return Err("pixel projection overflow".into());
        }
        Ok(pixel)
    }
    pub fn ray(&self, pixel: [f64; 2]) -> Result<[f64; 3], String> {
        self.validate()?;
        self.ray_validated(pixel)
    }
    pub(crate) fn ray_validated(&self, pixel: [f64; 2]) -> Result<[f64; 3], String> {
        if !self.contains_pixel(pixel) {
            return Err("observation is outside the calibrated image".into());
        }
        let yd = (pixel[1] - self.cy) / self.fy;
        let xd = (pixel[0] - self.cx - self.skew * yd) / self.fx;
        if !xd.is_finite() || !yd.is_finite() {
            return Err("inverse intrinsic matrix overflow".into());
        }
        let mut x = xd;
        let mut y = yd;
        let initial_radius = x.hypot(y);
        if initial_radius > self.valid_radius {
            x *= self.valid_radius / initial_radius;
            y *= self.valid_radius / initial_radius;
        }
        for _ in 0..40 {
            let (d, j) = self.distortion.map(x, y)?;
            let residual = [d[0] - xd, d[1] - yd];
            let norm = residual[0].hypot(residual[1]);
            if norm <= 1e-12 {
                if x.hypot(y) > self.valid_radius * (1. + 1e-12) {
                    return Err("undistorted point outside calibrated domain".into());
                }
                let length = x.hypot(y).hypot(1.);
                return Ok([x / length, y / length, 1. / length]);
            }
            let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
            if !det.is_finite() || det <= 1e-15 {
                return Err("singular inverse distortion Jacobian".into());
            }
            let step = [
                (j[1][1] * residual[0] - j[0][1] * residual[1]) / det,
                (-j[1][0] * residual[0] + j[0][0] * residual[1]) / det,
            ];
            let mut accepted = false;
            for damping in 0..16 {
                let factor = 0.5_f64.powi(damping);
                let candidate = [x - factor * step[0], y - factor * step[1]];
                if candidate[0].hypot(candidate[1]) > self.valid_radius {
                    continue;
                }
                if let Ok((value, _)) = self.distortion.map(candidate[0], candidate[1])
                    && (value[0] - xd).hypot(value[1] - yd) < norm
                {
                    x = candidate[0];
                    y = candidate[1];
                    accepted = true;
                    break;
                }
            }
            if !accepted {
                return Err("inverse distortion did not converge within calibrated domain".into());
            }
        }
        Err("inverse distortion iteration limit exceeded".into())
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    pub fn camera() -> CameraModel {
        CameraModel {
            width: 1280,
            height: 800,
            fx: 1000.,
            fy: 980.,
            cx: 639.5,
            cy: 399.5,
            skew: 0.,
            distortion: Brown5 {
                k1: -0.04,
                k2: 0.01,
                k3: 0.0002,
                p1: 0.0001,
                p2: -0.0002,
            },
            valid_radius: 1.5,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Vector3;
    #[test]
    fn analytic_jacobian_matches_finite_differences() {
        let k = fixtures::camera().distortion;
        let p = [0.3, -0.2];
        let (_, j) = k.map(p[0], p[1]).unwrap();
        for a in 0..2 {
            let mut left = p;
            let mut right = p;
            left[a] -= 1e-6;
            right[a] += 1e-6;
            let l = k.map(left[0], left[1]).unwrap().0;
            let r = k.map(right[0], right[1]).unwrap().0;
            for row in 0..2 {
                assert!(((r[row] - l[row]) / 2e-6 - j[row][a]).abs() < 1e-9);
            }
        }
    }
    #[test]
    fn distorted_projection_and_ray_roundtrip_with_skew() {
        let mut c = fixtures::camera();
        c.skew = 1.3;
        for x in -4..=4 {
            for y in -3..=3 {
                let p = [f64::from(x) * 30., f64::from(y) * 25., 500.];
                let pixel = c.project(p).unwrap();
                let ray = c.ray(pixel).unwrap();
                let expected = Vector3::from(p).normalize();
                assert!((Vector3::from(ray) - expected).norm() < 1e-11);
            }
        }
    }
    #[test]
    fn zero_distortion_is_analytic_pinhole() {
        let mut c = fixtures::camera();
        c.distortion = Brown5::ZERO;
        assert_eq!(c.project([50., 0., 500.]).unwrap(), [739.5, 399.5]);
        let r = c.ray([739.5, 399.5]).unwrap();
        assert!((r[0] / r[2] - 0.1).abs() < 1e-14);
    }
    #[test]
    fn nonzero_skew_has_independent_analytic_pixel_value() {
        let mut c = fixtures::camera();
        c.distortion = Brown5::ZERO;
        c.skew = 2.;
        let p = c.project([0., 50., 500.]).unwrap();
        assert!((p[0] - 639.7).abs() < 1e-12);
        assert!((p[1] - 497.5).abs() < 1e-12);
    }
    #[test]
    fn reject_folding_nonfinite_and_out_of_domain() {
        let mut c = fixtures::camera();
        c.distortion.k1 = -2.;
        assert!(c.validate().is_err());
        let mut c = fixtures::camera();
        c.fx = f64::NAN;
        assert!(c.ray([0., 0.]).is_err());
        let c = fixtures::camera();
        assert!(c.project([0., 0., -1.]).is_err());
        assert!(c.ray([-1., 0.]).is_err());
        assert!(c.project([1e308, 0., 1.]).is_err());
    }
    #[test]
    fn cubic_domain_check_finds_interior_minimum() {
        assert!((cubic_min([1., -4., 4., 0.]).unwrap()).abs() < 1e-14);
        assert_eq!(cubic_min([1., 0., 0., 0.]).unwrap(), 1.);
    }
}
