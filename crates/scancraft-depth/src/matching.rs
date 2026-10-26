//! Sparse integer-pixel ZNCC matching with opt-in bounded subpixel refinement on explicitly calibrated rectified images.
//! No exposure synchronization, feature detection or stereo rectification is inferred.
use scancraft_calibration::{Brown5, CameraModel, stereo::StereoCalibration};
use scancraft_geometry::work::{Stage, WorkControl};
use scancraft_image::MaskedImage;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

mod subpixel;
pub use subpixel::{SubpixelConfig, SubpixelRefinement};

const MAX_SEEDS: usize = 100_000;
const MAX_SAMPLE_VISITS: usize = 100_000_000;
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchConfig {
    pub patch_radius: u32,
    /// Signed integer disparity x_left - x_right, inclusive bounds.
    pub min_disparity: i32,
    pub max_disparity: i32,
    /// Population standard deviation in the input 0..255 intensity scale.
    pub min_patch_stddev: f64,
    pub min_correlation: f64,
    /// Absolute ZNCC score margin over every other valid disparity candidate.
    pub min_uniqueness_margin: f64,
    /// None preserves the legacy integer-only method and serialized output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subpixel: Option<SubpixelConfig>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedFeature {
    pub seed_index: usize,
    pub left_pixel: [u32; 2],
    /// Integer winner retained for lineage; use pixel_match() for geometry.
    pub right_pixel: [u32; 2],
    pub correlation: f64,
    pub second_correlation: f64,
    pub reverse_correlation: f64,
    pub reverse_second_correlation: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subpixel: Option<SubpixelRefinement>,
}
impl MatchedFeature {
    /// Geometry observations use the fractional right coordinate when available.
    /// The left observation remains the supplied integer seed, in the same grid.
    pub fn pixel_match(&self) -> crate::PixelMatch {
        crate::PixelMatch {
            left_pixel: self.left_pixel.map(f64::from),
            right_pixel: self.subpixel.as_ref().map_or_else(
                || self.right_pixel.map(f64::from),
                |refined| refined.right_pixel,
            ),
            color: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rejection {
    InvalidOrLowTextureSeed,
    InsufficientCandidates,
    LowCorrelation,
    Ambiguous,
    ReverseRejected,
    ReverseMismatch,
    SubpixelUnavailable,
    SubpixelReverseMismatch,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedSeed {
    pub seed_index: usize,
    pub reason: Rejection,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub rectified_geometry: StereoCalibration,
    pub config: MatchConfig,
    pub accepted: Vec<MatchedFeature>,
    pub rejected: Vec<RejectedSeed>,
}

fn validate_rig(rig: &StereoCalibration) -> Result<(), String> {
    rig.validate()?;
    let l = &rig.left;
    let r = &rig.right;
    // Exact metadata checks are deliberate: near-rectified is not rectified.
    // A computed rectification must explicitly construct its ideal output rig.
    if l.distortion != Brown5::ZERO
        || r.distortion != Brown5::ZERO
        || l.skew != 0.
        || r.skew != 0.
        || l.width != r.width
        || l.height != r.height
        || l.fx != r.fx
        || l.fy != r.fy
        || l.cx != r.cx
        || l.cy != r.cy
        || rig.right_from_left.rotation != [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
        || rig.right_from_left.translation_mm[1] != 0.
        || rig.right_from_left.translation_mm[2] != 0.
    {
        return Err("matcher requires an explicit equal-intrinsics, zero-skew, horizontal rectified stereo rig".into());
    }
    Ok(())
}
fn validate_config(config: MatchConfig, width: u32, seeds: usize) -> Result<(), String> {
    let span = i64::from(config.max_disparity) - i64::from(config.min_disparity) + 1;
    if !(1..=15).contains(&config.patch_radius)
        || !(2..=1024).contains(&span)
        || i64::from(config.min_disparity).abs() >= i64::from(width)
        || i64::from(config.max_disparity).abs() >= i64::from(width)
        || !config.min_patch_stddev.is_finite()
        || config.min_patch_stddev <= 0.
        || config.min_patch_stddev > 127.5
        || !config.min_correlation.is_finite()
        || !(0. ..=1.).contains(&config.min_correlation)
        || !config.min_uniqueness_margin.is_finite()
        || !(0. ..=2.).contains(&config.min_uniqueness_margin)
        || config.min_uniqueness_margin == 0.
        || !(1..=MAX_SEEDS).contains(&seeds)
    {
        return Err("invalid sparse stereo matching configuration or seed count".into());
    }
    if let Some(settings) = config.subpixel {
        settings.validate()?;
        if span < 3 {
            return Err("subpixel refinement needs at least three disparities".into());
        }
    }
    let side = 2 * config.patch_radius as usize + 1;
    // Two templates + two full searches; optional refinement reads four more
    // candidate patches. The work budget must include all additional samples.
    let patches = 2 * (span as usize + 1) + if config.subpixel.is_some() { 4 } else { 0 };
    let visits = seeds
        .checked_mul(patches)
        .and_then(|n| n.checked_mul(side * side));
    if visits.is_none_or(|n| n > MAX_SAMPLE_VISITS) {
        return Err("matching request exceeds 100 million bidirectional patch sample visits; split into bounded batches".into());
    }
    Ok(())
}
/// Deterministic row-major sampling of fully valid, textured left-image patches.
/// This proposes observations only; it does not establish correspondence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoSeeds {
    pub spacing: u32,
    pub maximum_seeds: usize,
}
pub fn select_seeds(
    rig: &StereoCalibration,
    left: &MaskedImage,
    config: MatchConfig,
    sampling: AutoSeeds,
    work: &WorkControl,
) -> Result<Vec<[u32; 2]>, String> {
    validate_rig(rig)?;
    validate_config(config, rig.left.width, sampling.maximum_seeds)?;
    if left.size() != [rig.left.width, rig.left.height] || sampling.spacing == 0 {
        return Err("invalid automatic sampling dimensions or spacing".into());
    }
    let [width, height] = left.size();
    let radius = config.patch_radius;
    let candidates =
        u64::from(width.div_ceil(sampling.spacing)) * u64::from(height.div_ceil(sampling.spacing));
    if candidates * u64::from(2 * radius + 1).pow(2) > MAX_SAMPLE_VISITS as u64 {
        return Err("automatic sampling exceeds 100 million patch sample visits".into());
    }
    let mut seeds = Vec::new();
    for y in (radius..height.saturating_sub(radius)).step_by(sampling.spacing as usize) {
        work.check()?;
        for x in (radius..width.saturating_sub(radius)).step_by(sampling.spacing as usize) {
            work.check()?;
            if patch(left, [x, y], config, &rig.left).is_some() {
                seeds.push([x, y]);
                if seeds.len() == sampling.maximum_seeds {
                    return Ok(seeds);
                }
            }
        }
    }
    Ok(seeds)
}
/// Centered, unit-length patch. A single invalid support pixel rejects the patch.
fn patch(
    image: &MaskedImage,
    center: [u32; 2],
    config: MatchConfig,
    camera: &CameraModel,
) -> Option<Vec<f64>> {
    let radius = config.patch_radius;
    let [width, height] = image.size();
    let [x, y] = center;
    if x < radius
        || y < radius
        || u64::from(x) + u64::from(radius) >= u64::from(width)
        || u64::from(y) + u64::from(radius) >= u64::from(height)
    {
        return None;
    }
    for px in [x - radius, x + radius] {
        for py in [y - radius, y + radius] {
            let nx = (f64::from(px) - camera.cx) / camera.fx;
            let ny = (f64::from(py) - camera.cy) / camera.fy;
            if nx.hypot(ny) > camera.valid_radius {
                return None;
            }
        }
    }
    let side = 2 * radius + 1;
    let mut values = Vec::with_capacity((side * side) as usize);
    for py in y - radius..=y + radius {
        for px in x - radius..=x + radius {
            values.push(f64::from(image.sample(px, py)?));
        }
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    for v in &mut values {
        *v -= mean;
    }
    let energy = values.iter().map(|v| v * v).sum::<f64>();
    if energy / (values.len() as f64) < config.min_patch_stddev.powi(2) || energy <= 0. {
        return None;
    }
    let length = energy.sqrt();
    for v in &mut values {
        *v /= length;
    }
    Some(values)
}
struct Best {
    position: [u32; 2],
    score: f64,
    second: f64,
}
fn search(
    template: &[f64],
    target: &MaskedImage,
    camera: &CameraModel,
    center: [u32; 2],
    direction: i64,
    config: MatchConfig,
    work: &WorkControl,
) -> Result<Result<Best, Rejection>, String> {
    let mut best: Option<Best> = None;
    let mut candidates = 0;
    for disparity in config.min_disparity..=config.max_disparity {
        work.check()?;
        let x = i64::from(center[0]) + direction * i64::from(disparity);
        if x < 0 || x >= i64::from(target.size()[0]) {
            continue;
        }
        let position = [x as u32, center[1]];
        let Some(values) = patch(target, position, config, camera) else {
            continue;
        };
        candidates += 1;
        let score = template
            .iter()
            .zip(values)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            .clamp(-1., 1.);
        if let Some(current) = best.as_mut() {
            if score > current.score {
                current.second = current.score;
                current.score = score;
                current.position = position;
            } else {
                current.second = current.second.max(score);
            }
        } else {
            best = Some(Best {
                position,
                score,
                second: -1.,
            });
        }
    }
    if candidates < 2 {
        return Ok(Err(Rejection::InsufficientCandidates));
    }
    let best = best.unwrap();
    if best.score < config.min_correlation {
        return Ok(Err(Rejection::LowCorrelation));
    }
    if best.score - best.second < config.min_uniqueness_margin {
        return Ok(Err(Rejection::Ambiguous));
    }
    Ok(Ok(best))
}
/// Each accepted seed has a unique forward winner and a reverse winner at its
/// original integer coordinate. Report indices refer to the original seed list.
/// Geometry validation does not prove the images came from this rig or exposure.
pub fn match_rectified(
    rig: &StereoCalibration,
    left: &MaskedImage,
    right: &MaskedImage,
    seeds: &[[u32; 2]],
    config: MatchConfig,
    work: &WorkControl,
) -> Result<MatchResult, String> {
    validate_rig(rig)?;
    if left.size() != [rig.left.width, rig.left.height]
        || right.size() != [rig.right.width, rig.right.height]
    {
        return Err("matching image dimensions disagree with rectified calibration".into());
    }
    validate_config(config, rig.left.width, seeds.len())?;
    let unique: HashSet<_> = seeds.iter().collect();
    if unique.len() != seeds.len() {
        return Err("duplicate stereo seed coordinates".into());
    }
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for (seed_index, &seed) in seeds.iter().enumerate() {
        work.update(Stage::Matching, seed_index, seeds.len())?;
        let outcome = (|| -> Result<Result<MatchedFeature, Rejection>, String> {
            let Some(template) = patch(left, seed, config, &rig.left) else {
                return Ok(Err(Rejection::InvalidOrLowTextureSeed));
            };
            let forward = match search(&template, right, &rig.right, seed, -1, config, work)? {
                Ok(v) => v,
                Err(reason) => return Ok(Err(reason)),
            };
            let reverse_template = patch(right, forward.position, config, &rig.right).unwrap();
            let reverse = match search(
                &reverse_template,
                left,
                &rig.left,
                forward.position,
                1,
                config,
                work,
            )? {
                Ok(v) => v,
                Err(_) => return Ok(Err(Rejection::ReverseRejected)),
            };
            if reverse.position != seed {
                return Ok(Err(Rejection::ReverseMismatch));
            }
            let subpixel = if let Some(settings) = config.subpixel {
                let disparity = i64::from(seed[0]) - i64::from(forward.position[0]);
                if disparity <= i64::from(config.min_disparity)
                    || disparity >= i64::from(config.max_disparity)
                {
                    return Ok(Err(Rejection::SubpixelUnavailable));
                }
                let Some(f) = subpixel::offset(
                    &template,
                    right,
                    &rig.right,
                    forward.position,
                    forward.score,
                    config,
                    work,
                )?
                else {
                    return Ok(Err(Rejection::SubpixelUnavailable));
                };
                let Some(r) = subpixel::offset(
                    &reverse_template,
                    left,
                    &rig.left,
                    seed,
                    reverse.score,
                    config,
                    work,
                )?
                else {
                    return Ok(Err(Rejection::SubpixelUnavailable));
                };
                if (f.offset + r.offset).abs() > settings.max_bidirectional_error_px {
                    return Ok(Err(Rejection::SubpixelReverseMismatch));
                }
                Some(SubpixelRefinement {
                    right_pixel: [
                        f64::from(forward.position[0]) + f.offset,
                        f64::from(seed[1]),
                    ],
                    forward_offset_px: f.offset,
                    reverse_offset_px: r.offset,
                    forward_curvature: f.curvature,
                    reverse_curvature: r.curvature,
                })
            } else {
                None
            };
            Ok(Ok(MatchedFeature {
                seed_index,
                left_pixel: seed,
                right_pixel: forward.position,
                correlation: forward.score,
                second_correlation: forward.second,
                reverse_correlation: reverse.score,
                reverse_second_correlation: reverse.second,
                subpixel,
            }))
        })()?;
        match outcome {
            Ok(feature) => accepted.push(feature),
            Err(reason) => rejected.push(RejectedSeed { seed_index, reason }),
        }
    }
    work.update(Stage::Matching, seeds.len(), seeds.len())?;
    Ok(MatchResult {
        rectified_geometry: rig.clone(),
        config,
        accepted,
        rejected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MatchSource, reconstruct};
    use scancraft_calibration::{
        stereo::{RigidPose, TriangulationLimits},
        storage::{CalibrationRecord, Provenance},
    };
    fn rig() -> StereoCalibration {
        let camera = CameraModel {
            width: 64,
            height: 32,
            fx: 80.,
            fy: 80.,
            cx: 31.5,
            cy: 15.5,
            skew: 0.,
            distortion: Brown5::ZERO,
            valid_radius: 1.,
        };
        StereoCalibration {
            left: camera.clone(),
            right: camera,
            right_from_left: RigidPose {
                rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                translation_mm: [-20., 0., 0.],
            },
        }
    }
    fn config() -> MatchConfig {
        MatchConfig {
            patch_radius: 2,
            min_disparity: 1,
            max_disparity: 8,
            min_patch_stddev: 5.,
            min_correlation: 0.9,
            min_uniqueness_margin: 0.05,
            subpixel: None,
        }
    }
    fn noise(seed: u32) -> Vec<f32> {
        let mut state = seed;
        (0..64 * 32)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                10. + (state % 91) as f32
            })
            .collect()
    }
    fn image(values: Vec<f32>) -> MaskedImage {
        MaskedImage::new(64, 32, values, vec![true; 64 * 32]).unwrap()
    }
    fn pair() -> (MaskedImage, MaskedImage) {
        let left = noise(0x12345678);
        let mut right = noise(0x87654321);
        for y in 0..32 {
            for x in 0..60 {
                right[y * 64 + x] = 2. * left[y * 64 + x + 4] + 20.;
            }
        }
        (image(left), image(right))
    }
    #[test]
    fn refinement_extra_patch_reads_are_included_in_work_budget() {
        let mut c = config();
        c.max_disparity = 100;
        assert!(validate_config(c, 4096, 19800).is_ok());
        c.subpixel = Some(SubpixelConfig {
            min_curvature: 0.001,
            max_bidirectional_error_px: 0.1,
        });
        assert!(
            validate_config(c, 4096, 19800)
                .unwrap_err()
                .contains("100 million")
        );
        c.min_disparity = 1;
        c.max_disparity = 2;
        assert!(
            validate_config(c, 4096, 1)
                .unwrap_err()
                .contains("three disparities")
        );
    }
    #[test]
    fn automatic_seeds_are_deterministic_masked_bounded_and_cancellable() {
        let sampling = AutoSeeds {
            spacing: 8,
            maximum_seeds: 5,
        };
        let (left, _) = pair();
        let expected = vec![[2, 2], [10, 2], [18, 2], [26, 2], [34, 2]];
        let selected =
            select_seeds(&rig(), &left, config(), sampling, &WorkControl::default()).unwrap();
        assert_eq!(selected, expected);
        let mut mask = vec![true; 64 * 32];
        mask[2 * 64 + 2] = false;
        let masked = MaskedImage::new(64, 32, noise(0x12345678), mask).unwrap();
        let selected =
            select_seeds(&rig(), &masked, config(), sampling, &WorkControl::default()).unwrap();
        assert_eq!(selected[0], [10, 2]);
        assert_eq!(selected.len(), 5);
        assert!(
            select_seeds(
                &rig(),
                &image(vec![20.; 64 * 32]),
                config(),
                sampling,
                &WorkControl::default()
            )
            .unwrap()
            .is_empty()
        );
        let work = WorkControl::default();
        work.cancel();
        assert!(
            select_seeds(&rig(), &left, config(), sampling, &work)
                .unwrap_err()
                .contains("cancelled")
        );
    }
    #[test]
    fn shifted_affine_intensity_images_reconstruct_analytic_plane() {
        let (left, right) = pair();
        let rig = rig();
        let seeds: Vec<_> = (6..27)
            .step_by(5)
            .flat_map(|y| (12..55).step_by(6).map(move |x| [x, y]))
            .collect();
        let matched = match_rectified(
            &rig,
            &left,
            &right,
            &seeds,
            config(),
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(matched.accepted.len(), seeds.len());
        assert!(matched.rejected.is_empty());
        for m in &matched.accepted {
            assert_eq!(m.left_pixel, seeds[m.seed_index]);
            assert_eq!(m.right_pixel, [m.left_pixel[0] - 4, m.left_pixel[1]]);
            assert!((m.correlation - 1.).abs() < 1e-12);
            assert!((m.reverse_correlation - 1.).abs() < 1e-12);
        }
        let record = CalibrationRecord {
            schema_version: 1,
            record_id: "synthetic-shifted-image-plane".into(),
            units: "millimeters".into(),
            camera_axes: "x_right_y_down_z_forward".into(),
            provenance: Provenance::SyntheticFixture,
            binding: None,
            geometry: rig,
            notes: "not scanner data".into(),
        };
        let limits = TriangulationLimits {
            min_depth_z_mm: 10.,
            max_depth_z_mm: 1000.,
            min_ray_angle_rad: 0.0001,
            max_ray_gap_mm: 0.01,
            max_reprojection_error_px: 0.01,
        };
        let points: Vec<_> = matched
            .accepted
            .iter()
            .map(MatchedFeature::pixel_match)
            .collect();
        let result = reconstruct(
            &record,
            MatchSource::SyntheticFixture,
            &points,
            limits,
            &WorkControl::default(),
        )
        .unwrap();
        assert!(result.rejected.is_empty());
        for p in result.cloud().points {
            assert!((p.position[2] - 400.).abs() < 1e-8);
            assert!(p.position.iter().all(|v| v.is_finite()));
        }
    }
    #[test]
    fn repeated_texture_and_low_texture_are_rejected() {
        let repeated = image(
            (0..2048)
                .map(|i| if i % 2 == 0 { 30. } else { 180. })
                .collect(),
        );
        let result = match_rectified(
            &rig(),
            &repeated,
            &repeated,
            &[[30, 16]],
            config(),
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(result.rejected[0].reason, Rejection::Ambiguous);
        let flat = image(vec![20.; 2048]);
        let result = match_rectified(
            &rig(),
            &flat,
            &flat,
            &[[30, 16]],
            config(),
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(
            result.rejected[0].reason,
            Rejection::InvalidOrLowTextureSeed
        );
    }
    #[test]
    fn masks_borders_and_insufficient_candidates_preserve_original_seed_indices() {
        let (left, right) = pair();
        let mut validity = vec![true; 2048];
        validity[16 * 64 + 30] = false;
        let left = MaskedImage::new(64, 32, left.intensity().to_vec(), validity).unwrap();
        let result = match_rectified(
            &rig(),
            &left,
            &right,
            &[[0, 0], [30, 16], [44, 20]],
            config(),
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(
            result
                .rejected
                .iter()
                .map(|s| s.seed_index)
                .collect::<Vec<_>>(),
            [0, 1]
        );
        assert_eq!(result.accepted[0].seed_index, 2);
        let invalid = MaskedImage::new(64, 32, vec![0.; 2048], vec![false; 2048]).unwrap();
        let result = match_rectified(
            &rig(),
            &left,
            &invalid,
            &[[44, 20]],
            config(),
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(result.rejected[0].reason, Rejection::InsufficientCandidates);
    }
    #[test]
    fn independently_unrelated_images_fail_score_threshold() {
        let left = image(noise(12));
        let right = image(noise(346));
        let mut config = config();
        config.min_correlation = 0.999;
        let result = match_rectified(
            &rig(),
            &left,
            &right,
            &[[30, 16]],
            config,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(result.rejected[0].reason, Rejection::LowCorrelation);
    }
    #[test]
    fn reverse_match_does_not_accept_an_occluded_forward_winner() {
        let mut left = noise(42);
        let mut right = noise(9234);
        for y in 0..5 {
            for x in 0..5 {
                let pattern = left[(14 + y) * 64 + 34 + x];
                right[(14 + y) * 64 + 24 + x] = pattern;
                left[(14 + y) * 64 + 28 + x] = pattern + if (x + y) % 2 == 0 { 10. } else { -5. };
            }
        }
        let mut config = config();
        config.max_disparity = 12;
        config.min_correlation = 0.8;
        config.min_uniqueness_margin = 0.005;
        let result = match_rectified(
            &rig(),
            &image(left),
            &image(right),
            &[[30, 16]],
            config,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(result.rejected[0].reason, Rejection::ReverseMismatch);
    }
    #[test]
    fn reverse_ambiguity_and_only_one_candidate_do_not_pass_uniqueness() {
        let mut left = noise(555);
        let mut right = noise(666);
        let mut template = Vec::new();
        for y in 14..19 {
            for x in 28..33 {
                template.push(left[y * 64 + x]);
            }
        }
        for y in 0..5 {
            for x in 0..5 {
                let v = template[y * 5 + x];
                left[(14 + y) * 64 + 34 + x] = v;
                right[(14 + y) * 64 + 24 + x] = v;
            }
        }
        let mut cfg = config();
        cfg.max_disparity = 12;
        let result = match_rectified(
            &rig(),
            &image(left),
            &image(right),
            &[[30, 16]],
            cfg,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(result.rejected[0].reason, Rejection::ReverseRejected);
        let (left, right) = pair();
        let mut mask = vec![false; 2048];
        for y in 14..19 {
            for x in 24..29 {
                mask[y * 64 + x] = true;
            }
        }
        let right = MaskedImage::new(64, 32, right.intensity().to_vec(), mask).unwrap();
        let result = match_rectified(
            &rig(),
            &left,
            &right,
            &[[30, 16]],
            config(),
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(result.rejected[0].reason, Rejection::InsufficientCandidates);
    }
    #[test]
    fn wrong_geometry_size_invalid_config_duplicates_and_cancellation_fail() {
        let (left, right) = pair();
        let mut rig = rig();
        rig.right.distortion.k1 = 0.01;
        assert!(
            match_rectified(
                &rig,
                &left,
                &right,
                &[[30, 16]],
                config(),
                &WorkControl::default()
            )
            .is_err()
        );
        rig.right.distortion = Brown5::ZERO;
        rig.right_from_left.translation_mm[1] = 0.001;
        assert!(
            match_rectified(
                &rig,
                &left,
                &right,
                &[[30, 16]],
                config(),
                &WorkControl::default()
            )
            .is_err()
        );
        rig.right_from_left.translation_mm[1] = 0.;
        let wrong = MaskedImage::new(1, 1, vec![0.], vec![true]).unwrap();
        assert!(
            match_rectified(
                &rig,
                &left,
                &wrong,
                &[[30, 16]],
                config(),
                &WorkControl::default()
            )
            .is_err()
        );
        let mut bad = config();
        bad.min_disparity = i32::MIN;
        bad.max_disparity = i32::MAX;
        assert!(
            match_rectified(
                &rig,
                &left,
                &right,
                &[[30, 16]],
                bad,
                &WorkControl::default()
            )
            .is_err()
        );
        assert!(
            match_rectified(
                &rig,
                &left,
                &right,
                &[[30, 16], [30, 16]],
                config(),
                &WorkControl::default()
            )
            .is_err()
        );
        let work = WorkControl::default();
        work.cancel();
        assert!(
            match_rectified(&rig, &left, &right, &[[30, 16]], config(), &work)
                .unwrap_err()
                .contains("cancelled")
        );
        let mut expensive = config();
        expensive.patch_radius = 15;
        assert!(validate_config(expensive, 64, 100_000).is_err());
    }
    #[test]
    fn opposite_baseline_and_negative_disparity_are_supported() {
        let (left, right) = pair();
        let mut rig = rig();
        rig.right_from_left.translation_mm[0] = 20.;
        let mut config = config();
        config.min_disparity = -8;
        config.max_disparity = -1;
        let result = match_rectified(
            &rig,
            &right,
            &left,
            &[[26, 16]],
            config,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(result.accepted[0].right_pixel, [30, 16]);
    }
    #[test]
    fn supported_lens_disk_is_checked_for_entire_patch() {
        let (left, right) = pair();
        let mut rig = rig();
        rig.left.valid_radius = 0.01;
        let result = match_rectified(
            &rig,
            &left,
            &right,
            &[[30, 16]],
            config(),
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(
            result.rejected[0].reason,
            Rejection::InvalidOrLowTextureSeed
        );
    }
}
