//! Seeded subpixel target-corner refinement, not target detection or calibration.
//!
//! Independent implementation of the gradient-orthogonality least-squares method
//! documented at https://docs.opencv.org/4.1.2/dd/d1a/group__imgproc__feature.html
//! (cornerSubPix). No upstream implementation is copied. Integer coordinates are
//! pixel centers; image samples remain immutable and no border extrapolation is used.
use crate::GrayImage;

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub radius: u32,
    pub max_iterations: u32,
    pub epsilon_px: f64,
    pub max_shift_px: f64,
    /// Minimum eigenvalue of weighted mean gradient products, with intensity /255.
    pub min_eigenvalue: f64,
    pub min_eigenvalue_ratio: f64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            radius: 5,
            max_iterations: 40,
            epsilon_px: 1e-4,
            max_shift_px: 4.,
            min_eigenvalue: 1e-5,
            min_eigenvalue_ratio: 0.01,
        }
    }
}
impl Options {
    fn validate(self) -> Result<(), String> {
        if !(2..=15).contains(&self.radius)
            || !(1..=100).contains(&self.max_iterations)
            || !self.epsilon_px.is_finite()
            || !(1e-6..=1.).contains(&self.epsilon_px)
            || !self.max_shift_px.is_finite()
            || self.max_shift_px <= 0.
            || self.max_shift_px > f64::from(self.radius)
            || !self.min_eigenvalue.is_finite()
            || self.min_eigenvalue <= 0.
            || !self.min_eigenvalue_ratio.is_finite()
            || !(1e-6..=1.).contains(&self.min_eigenvalue_ratio)
        {
            return Err("invalid corner refinement options".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy)]
pub struct RefinedCorner {
    pub pixel: [f64; 2],
    pub iterations: u32,
    /// Last solved window's diagnostics, not positional covariance or accuracy.
    pub min_eigenvalue: f64,
    pub eigenvalue_ratio: f64,
}
/// Refine an already associated, approximate corner in a packed 8-bit image.
///
/// Gaussian-weighted normal equations minimize sum w*(gradient dot (q-p))^2.
/// Each iteration resamples the window bilinearly at its current center, using
/// central differences one pixel apart. Rank/contrast, border, displacement and
/// nonconvergence checks reject rather than silently returning an unchanged seed.
/// Cancellation is checked before work and once per iteration (<=961 samples).
/// This does not detect/index a board, establish physical scale or validate source binding.
pub fn refine_corner(
    image: GrayImage<'_>,
    seed: [f64; 2],
    options: Options,
    cancelled: &dyn Fn() -> bool,
) -> Result<RefinedCorner, String> {
    options.validate()?;
    if !seed.iter().all(|v| v.is_finite()) {
        return Err("nonfinite corner seed".into());
    }
    let mut center = seed;
    let radius = i32::try_from(options.radius).unwrap();
    let scale = f64::from(options.radius);
    let mut weights = Vec::with_capacity(((2 * radius + 1) * (2 * radius + 1)) as usize);
    for y in -radius..=radius {
        for x in -radius..=radius {
            weights.push((x, y, (-(f64::from(x * x + y * y)) / (scale * scale)).exp()));
        }
    }
    let weight_sum: f64 = weights.iter().map(|p| p.2).sum();
    for iteration in 1..=options.max_iterations {
        if cancelled() {
            return Err("Corner refinement cancelled".into());
        }
        let margin = scale + 1.;
        if center[0] < margin
            || center[1] < margin
            || center[0] > f64::from(image.width) - 1. - margin
            || center[1] > f64::from(image.height) - 1. - margin
        {
            return Err("corner window extends beyond image".into());
        }
        let (mut xx, mut xy, mut yy, mut bx, mut by) = (0., 0., 0., 0., 0.);
        let sample = |x, y| {
            image
                .bilinear([x, y])
                .map(f64::from)
                .ok_or("corner sample outside image")
        };
        for &(x, y, w) in &weights {
            let (dx, dy) = (f64::from(x), f64::from(y));
            let (px, py) = (center[0] + dx, center[1] + dy);
            let gx = (sample(px + 1., py)? - sample(px - 1., py)?) / 510.;
            let gy = (sample(px, py + 1.)? - sample(px, py - 1.)?) / 510.;
            let (a, b, c) = (w * gx * gx, w * gx * gy, w * gy * gy);
            xx += a;
            xy += b;
            yy += c;
            bx += a * dx + b * dy;
            by += b * dx + c * dy;
        }
        let determinant = xx * yy - xy * xy;
        let trace = xx + yy;
        let largest = 0.5 * (trace + ((xx - yy) * (xx - yy) + 4. * xy * xy).sqrt());
        let smallest = if largest > 0. {
            determinant / largest
        } else {
            0.
        };
        let ratio = if largest > 0. { smallest / largest } else { 0. };
        if !determinant.is_finite()
            || smallest / weight_sum < options.min_eigenvalue
            || ratio < options.min_eigenvalue_ratio
        {
            return Err("corner window has insufficient contrast or two-dimensional rank".into());
        }
        let delta = [
            (yy * bx - xy * by) / determinant,
            (xx * by - xy * bx) / determinant,
        ];
        let next = [center[0] + delta[0], center[1] + delta[1]];
        if !next.iter().all(|v| v.is_finite())
            || (next[0] - seed[0]).hypot(next[1] - seed[1]) > options.max_shift_px
        {
            return Err("corner refinement exceeds allowed seed displacement".into());
        }
        if delta[0].hypot(delta[1]) <= options.epsilon_px {
            // The final candidate must retain the same complete gradient window.
            if next[0] < margin
                || next[1] < margin
                || next[0] > f64::from(image.width) - 1. - margin
                || next[1] > f64::from(image.height) - 1. - margin
            {
                return Err("refined corner window extends beyond image".into());
            }
            return Ok(RefinedCorner {
                pixel: next,
                iterations: iteration,
                min_eigenvalue: smallest / weight_sum,
                eigenvalue_ratio: ratio,
            });
        }
        center = next;
    }
    Err("corner refinement did not converge within iteration limit".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(center: [f64; 2], inverted: bool) -> Vec<u8> {
        (0..64 * 64)
            .map(|i| {
                let x = (i % 64) as f64 - center[0];
                let y = (i / 64) as f64 - center[1];
                let v = 128. + 95. * (x / 1.2).tanh() * (y / 1.2).tanh();
                (if inverted { 255. - v } else { v }).round() as u8
            })
            .collect()
    }
    #[test]
    fn recovers_shifted_corners_under_polarity_and_subpixel_offsets() {
        for truth in [[30., 31.], [30.3, 31.7], [31.65, 29.2]] {
            for inverted in [false, true] {
                let data = fixture(truth, inverted);
                let copy = data.clone();
                let image = GrayImage::new(64, 64, &data).unwrap();
                let result = refine_corner(
                    image,
                    [truth[0] + 0.7, truth[1] - 0.6],
                    Options::default(),
                    &|| false,
                )
                .unwrap();
                assert!(
                    (result.pixel[0] - truth[0]).hypot(result.pixel[1] - truth[1]) < 0.04,
                    "{result:?}"
                );
                assert!(result.min_eigenvalue > 0.);
                assert!(result.eigenvalue_ratio > 0.1);
                assert_eq!(data, copy);
            }
        }
    }
    #[test]
    fn flat_and_single_edges_are_rejected() {
        for data in [
            vec![80; 4096],
            (0..4096)
                .map(|i| if i % 64 < 32 { 20 } else { 230 })
                .collect(),
        ] {
            assert!(
                refine_corner(
                    GrayImage::new(64, 64, &data).unwrap(),
                    [32., 32.],
                    Options::default(),
                    &|| false
                )
                .unwrap_err()
                .contains("rank")
            );
        }
    }
    #[test]
    fn borders_invalid_options_and_nonfinite_seeds_fail() {
        let data = fixture([30., 31.], false);
        let image = GrayImage::new(64, 64, &data).unwrap();
        for seed in [[f64::NAN, 31.], [30., f64::INFINITY], [2., 30.], [60., 30.]] {
            assert!(refine_corner(image, seed, Options::default(), &|| false).is_err());
        }
        for options in [
            Options {
                radius: 0,
                ..Options::default()
            },
            Options {
                max_iterations: 0,
                ..Options::default()
            },
            Options {
                epsilon_px: f64::NAN,
                ..Options::default()
            },
            Options {
                max_shift_px: 6.,
                ..Options::default()
            },
            Options {
                min_eigenvalue_ratio: 0.,
                ..Options::default()
            },
        ] {
            assert!(refine_corner(image, [30., 31.], options, &|| false).is_err());
        }
    }
    #[test]
    fn displacement_cancellation_and_nonconvergence_are_explicit() {
        let data = fixture([30.3, 31.7], false);
        let image = GrayImage::new(64, 64, &data).unwrap();
        assert!(
            refine_corner(
                image,
                [31., 31.],
                Options {
                    max_shift_px: 0.1,
                    ..Options::default()
                },
                &|| false
            )
            .unwrap_err()
            .contains("displacement")
        );
        assert!(
            refine_corner(image, [31., 31.], Options::default(), &|| true)
                .unwrap_err()
                .contains("cancelled")
        );
        assert!(
            refine_corner(
                image,
                [31., 31.],
                Options {
                    max_iterations: 1,
                    epsilon_px: 1e-6,
                    ..Options::default()
                },
                &|| false
            )
            .unwrap_err()
            .contains("converge")
        );
    }
}
