//! Independently derived three-sample quadratic peak fit, not an OpenCV port.
//! Integer matching/uniqueness/reverse checks remain prerequisites. No fitting
//! beyond search limits, invalid support or a non-strict/flat peak is accepted.
use super::{MatchConfig, patch};
use scancraft_calibration::CameraModel;
use scancraft_geometry::work::WorkControl;
use scancraft_image::MaskedImage;
use serde::{Deserialize, Serialize};

/// Opt-in additional acceptance gates. Unsupported fits are rejected, never
/// silently downgraded to integer observations when this mode is requested.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubpixelConfig {
    /// Minimum 2*center - minus - plus in ZNCC score units, in (0,4].
    pub min_curvature: f64,
    /// Maximum abs(forward offset + reverse offset), in [0,1] pixels.
    pub max_bidirectional_error_px: f64,
}
impl SubpixelConfig {
    pub(super) fn validate(self) -> Result<(), String> {
        if !self.min_curvature.is_finite()
            || !(0. ..=4.).contains(&self.min_curvature)
            || self.min_curvature == 0.
            || !self.max_bidirectional_error_px.is_finite()
            || !(0. ..=1.).contains(&self.max_bidirectional_error_px)
        {
            return Err("invalid subpixel curvature or bidirectional pixel error".into());
        }
        Ok(())
    }
}
/// Estimated right-grid position and local peak diagnostics. These are not an
/// attestation of a physical correspondence, synchronized exposure or accuracy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubpixelRefinement {
    pub right_pixel: [f64; 2],
    pub forward_offset_px: f64,
    pub reverse_offset_px: f64,
    pub forward_curvature: f64,
    pub reverse_curvature: f64,
}
pub(super) struct Peak {
    pub offset: f64,
    pub curvature: f64,
}
fn peak(scores: [f64; 3], minimum: f64) -> Option<Peak> {
    let [minus, center, plus] = scores;
    if !scores
        .iter()
        .all(|s| s.is_finite() && (-1. ..=1.).contains(s))
        || !minimum.is_finite()
        || minimum <= 0.
        || minimum > 4.
        || center <= minus
        || center <= plus
    {
        return None;
    }
    let curvature = 2. * center - minus - plus;
    if curvature < minimum {
        return None;
    }
    let offset = 0.5 * (plus - minus) / curvature;
    (offset.is_finite() && offset.abs() <= 0.5).then_some(Peak { offset, curvature })
}
pub(super) fn offset(
    template: &[f64],
    target: &MaskedImage,
    camera: &CameraModel,
    center: [u32; 2],
    score: f64,
    config: MatchConfig,
    work: &WorkControl,
) -> Result<Option<Peak>, String> {
    let Some(settings) = config.subpixel else {
        return Ok(None);
    };
    let Some(minus) = center[0].checked_sub(1) else {
        return Ok(None);
    };
    let Some(plus) = center[0].checked_add(1) else {
        return Ok(None);
    };
    let mut scores = [0., score, 0.];
    for (index, x) in [(0, minus), (2, plus)] {
        work.check()?;
        let Some(values) = patch(target, [x, center[1]], config, camera) else {
            return Ok(None);
        };
        scores[index] = template
            .iter()
            .zip(values)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            .clamp(-1., 1.);
    }
    Ok(peak(scores, settings.min_curvature))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analytic_quadratic_peak_sign_curvature_and_invalid_shapes() {
        let p = peak([0.5, 0.95, 0.8], 0.01).unwrap();
        assert!((p.offset - 0.25).abs() < 1e-14);
        assert!((p.curvature - 0.6).abs() < 1e-14);
        assert!((peak([0.8, 0.95, 0.5], 0.01).unwrap().offset + 0.25).abs() < 1e-14);
        assert_eq!(peak([0.5, 1., 0.5], 0.01).unwrap().offset, 0.);
        for scores in [
            [0.5, 0.5, 0.5],
            [0.5, 0.4, 0.3],
            [0.5, f64::NAN, 0.3],
            [-2., 1., 0.],
        ] {
            assert!(peak(scores, 0.01).is_none());
        }
        assert!(peak([0.5, 0.95, 0.8], 0.7).is_none());
    }
}
