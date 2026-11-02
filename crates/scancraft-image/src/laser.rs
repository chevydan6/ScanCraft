//! Threshold-run centroids for isolated laser stripes in raw grayscale images.
//! Detections are image observations, not identified projector lines or depth.
use crate::GrayImage;
use scancraft_geometry::work::{Stage, WorkControl};

#[derive(Debug, Clone, Copy)]
pub struct LineConfig {
    /// Exclusive signal threshold (raw intensity, or local top-hat contrast).
    pub background: u8,
    pub minimum_peak: u8,
    pub maximum_width: u32,
    /// Reject clipping at or above this level (255 normally).
    pub saturation: u8,
    /// Abort rather than silently truncate observations.
    pub maximum_observations: usize,
}
#[derive(Debug, Clone, PartialEq)]
pub struct LineObservation {
    /// Integer coordinates denote pixel centers; x is intensity-weighted.
    pub pixel: [f64; 2],
    pub first_x: u32,
    pub last_x: u32,
    pub peak: u8,
    pub integrated_signal: u64,
    pub variance_x: f64,
}
#[derive(Debug, Default)]
pub struct LineDetections {
    pub observations: Vec<LineObservation>,
    pub rejected_border: usize,
    pub rejected_width: usize,
    pub rejected_peak: usize,
    pub rejected_saturation: usize,
}
/// Scan horizontal rows in O(pixels), with bounded output and no source mutation.
/// Connected above-background runs touching either image edge are rejected.
/// Multiple runs in a row remain separate; no line identity or continuity is inferred.
pub fn detect_rows(
    image: GrayImage<'_>,
    config: LineConfig,
    work: &WorkControl,
) -> Result<LineDetections, String> {
    detect_rows_impl(image, image, config, work)
}
fn validate(image: GrayImage<'_>, config: LineConfig) -> Result<(), String> {
    if config.background >= config.minimum_peak
        || config.minimum_peak >= config.saturation
        || config.maximum_width == 0
        || config.maximum_width > image.width
        || !(1..=100_000).contains(&config.maximum_observations)
    {
        return Err("invalid laser stripe detection limits".into());
    }
    Ok(())
}
/// Row-wise white top-hat (raw minus morphological opening), followed by the
/// same bounded run detector. Opening is an erosion then dilation with a
/// symmetric window of 2*radius+1 pixels, truncated at image boundaries.
/// Thresholds/centroid weights refer to contrast; clipping always refers to
/// original raw samples. No source samples are modified or hidden by rescaling.
/// This removes slowly varying backgrounds, not texture/noise or line ambiguity.
pub fn detect_rows_with_opening(
    image: GrayImage<'_>,
    config: LineConfig,
    radius: u32,
    work: &WorkControl,
) -> Result<LineDetections, String> {
    validate(image, config)?;
    if radius == 0 || radius > 128 || 2 * radius + 1 > image.width {
        return Err("opening radius must be 1..128 and fit within image width".into());
    }
    work.check()?;
    let width = image.width as usize;
    let mut contrast = vec![0; image.data.len()];
    let mut eroded = vec![0; width];
    let mut opened = vec![0; width];
    let mut prefix = vec![0; width + 2 * radius as usize];
    let mut suffix = vec![0; prefix.len()];
    for y in 0..image.height as usize {
        work.update(Stage::ImageMapping, y, image.height as usize)?;
        let raw = &image.data[y * width..(y + 1) * width];
        row_extreme::<true>(raw, radius as usize, &mut eroded, &mut prefix, &mut suffix);
        row_extreme::<false>(
            &eroded,
            radius as usize,
            &mut opened,
            &mut prefix,
            &mut suffix,
        );
        for x in 0..width {
            contrast[y * width + x] = raw[x].saturating_sub(opened[x]);
        }
    }
    detect_rows_impl(
        GrayImage::new(image.width, image.height, &contrast)?,
        image,
        config,
        work,
    )
}
// Block prefix/suffix extrema (van Herk/Gil-Werman family), independently written.
// Reference: van Herk (1992), doi:10.1016/0167-8655(92)90069-C; no upstream code.
// Neutral padding makes every window a full block-width interval: it spans at
// most two blocks, so suffix[start] and prefix[end] cover exactly its samples.
// Padding is 255 for min and 0 for max, preserving clipped image-edge windows.
fn row_extreme<const MINIMUM: bool>(
    input: &[u8],
    radius: usize,
    output: &mut [u8],
    prefix: &mut [u8],
    suffix: &mut [u8],
) {
    let neutral = if MINIMUM { 255 } else { 0 };
    let block_width = 2 * radius + 1;
    prefix.fill(neutral);
    prefix[radius..radius + input.len()].copy_from_slice(input);
    suffix.copy_from_slice(prefix);
    for block in prefix.chunks_mut(block_width) {
        let mut value = neutral;
        for pixel in block {
            value = if MINIMUM {
                value.min(*pixel)
            } else {
                value.max(*pixel)
            };
            *pixel = value;
        }
    }
    for block in suffix.chunks_mut(block_width) {
        let mut value = neutral;
        for pixel in block.iter_mut().rev() {
            value = if MINIMUM {
                value.min(*pixel)
            } else {
                value.max(*pixel)
            };
            *pixel = value;
        }
    }
    for (x, value) in output.iter_mut().enumerate() {
        *value = if MINIMUM {
            suffix[x].min(prefix[x + 2 * radius])
        } else {
            suffix[x].max(prefix[x + 2 * radius])
        };
    }
}
fn detect_rows_impl(
    image: GrayImage<'_>,
    raw: GrayImage<'_>,
    config: LineConfig,
    work: &WorkControl,
) -> Result<LineDetections, String> {
    validate(image, config)?;
    let mut result = LineDetections::default();
    for y in 0..image.height {
        work.update(Stage::Processing, y as usize, image.height as usize)?;
        let row =
            &image.data[y as usize * image.width as usize..(y as usize + 1) * image.width as usize];
        let mut x = 0usize;
        while x < row.len() {
            if row[x] <= config.background {
                x += 1;
                continue;
            }
            let start = x;
            let mut peak = 0;
            let mut mass = 0u64;
            let mut moment = 0f64;
            let mut clipped = false;
            while x < row.len() && row[x] > config.background {
                peak = peak.max(row[x]);
                clipped |= raw.data[y as usize * image.width as usize + x] >= config.saturation;
                let weight = u64::from(row[x] - config.background);
                mass += weight;
                moment += weight as f64 * (x - start) as f64;
                x += 1;
            }
            // A run is counted once, under the first applicable rejection reason.
            if start == 0 || x == row.len() {
                result.rejected_border += 1;
                continue;
            }
            if x - start > config.maximum_width as usize {
                result.rejected_width += 1;
                continue;
            }
            if clipped {
                result.rejected_saturation += 1;
                continue;
            }
            if peak < config.minimum_peak {
                result.rejected_peak += 1;
                continue;
            }
            if result.observations.len() == config.maximum_observations {
                return Err("laser stripe observation limit exceeded".into());
            }
            let center = moment / mass as f64;
            let variance_x = row[start..x]
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    f64::from(*value - config.background) * (i as f64 - center).powi(2)
                })
                .sum::<f64>()
                / mass as f64;
            result.observations.push(LineObservation {
                pixel: [start as f64 + center, f64::from(y)],
                first_x: start as u32,
                last_x: (x - 1) as u32,
                peak,
                integrated_signal: mass,
                variance_x,
            });
        }
    }
    work.update(
        Stage::Processing,
        image.height as usize,
        image.height as usize,
    )?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> LineConfig {
        LineConfig {
            background: 10,
            minimum_peak: 30,
            maximum_width: 4,
            saturation: 255,
            maximum_observations: 100,
        }
    }
    #[test]
    fn block_extrema_match_independent_window_extrema_at_every_boundary() {
        let mut seed = 17u32;
        for width in 3usize..40 {
            let input: Vec<_> = (0..width)
                .map(|_| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    (seed >> 24) as u8
                })
                .collect();
            for radius in 1..=((width - 1) / 2).min(8) {
                for minimum in [true, false] {
                    let mut output = vec![0; width];
                    let mut prefix = vec![0; width + 2 * radius];
                    let mut suffix = vec![0; prefix.len()];
                    if minimum {
                        row_extreme::<true>(&input, radius, &mut output, &mut prefix, &mut suffix);
                    } else {
                        row_extreme::<false>(&input, radius, &mut output, &mut prefix, &mut suffix);
                    }
                    for x in 0..width {
                        let slice = &input[x.saturating_sub(radius)..(x + radius + 1).min(width)];
                        let expected = if minimum {
                            *slice.iter().min().unwrap()
                        } else {
                            *slice.iter().max().unwrap()
                        };
                        assert_eq!(output[x], expected);
                    }
                }
            }
        }
    }

    #[test]
    fn block_extrema_cover_every_radius_and_partial_block_edges() {
        let mut seed = 53u32;
        for radius in 1..=128 {
            for width in [2 * radius + 1, 2 * radius + 2, 4 * radius + 1] {
                let random = (0..width)
                    .map(|_| {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        (seed >> 24) as u8
                    })
                    .collect::<Vec<_>>();
                let alternating = (0..width)
                    .map(|x| if x % 2 == 0 { 0 } else { 255 })
                    .collect::<Vec<_>>();
                for input in [random, alternating, vec![0; width], vec![255; width]] {
                    let mut lo = vec![0; width];
                    let mut hi = vec![0; width];
                    let mut prefix = vec![0; width + 2 * radius];
                    let mut suffix = prefix.clone();
                    row_extreme::<true>(&input, radius, &mut lo, &mut prefix, &mut suffix);
                    row_extreme::<false>(&input, radius, &mut hi, &mut prefix, &mut suffix);
                    for x in 0..width {
                        let window = &input[x.saturating_sub(radius)..(x + radius + 1).min(width)];
                        assert_eq!(
                            lo[x],
                            *window.iter().min().unwrap(),
                            "minimum radius={radius} width={width} x={x}"
                        );
                        assert_eq!(
                            hi[x],
                            *window.iter().max().unwrap(),
                            "maximum radius={radius} width={width} x={x}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn complete_opening_detections_match_direct_neighborhood_reference() {
        let mut seed = 731u32;
        for width in [7usize, 25, 257, 263] {
            let data = (0..width * 3)
                .map(|_| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    (seed >> 24) as u8
                })
                .collect::<Vec<_>>();
            let original = data.clone();
            for radius in [1usize, 2, 12, 128].into_iter().filter(|r| 2 * r < width) {
                let mut contrast = vec![0; data.len()];
                for (raw, out) in data
                    .chunks_exact(width)
                    .zip(contrast.chunks_exact_mut(width))
                {
                    let eroded = (0..width)
                        .map(|x| {
                            *raw[x.saturating_sub(radius)..(x + radius + 1).min(width)]
                                .iter()
                                .min()
                                .unwrap()
                        })
                        .collect::<Vec<_>>();
                    for x in 0..width {
                        let opened = *eroded[x.saturating_sub(radius)..(x + radius + 1).min(width)]
                            .iter()
                            .max()
                            .unwrap();
                        out[x] = raw[x].saturating_sub(opened);
                    }
                }
                let raw = GrayImage::new(width as u32, 3, &data).unwrap();
                let contrast = GrayImage::new(width as u32, 3, &contrast).unwrap();
                let config = LineConfig {
                    background: 30,
                    minimum_peak: 50,
                    maximum_width: width as u32,
                    saturation: 255,
                    maximum_observations: 100000,
                };
                let expected =
                    detect_rows_impl(contrast, raw, config, &WorkControl::default()).unwrap();
                let actual =
                    detect_rows_with_opening(raw, config, radius as u32, &WorkControl::default())
                        .unwrap();
                assert_eq!(actual.observations, expected.observations);
                assert_eq!(
                    (
                        actual.rejected_border,
                        actual.rejected_width,
                        actual.rejected_peak,
                        actual.rejected_saturation
                    ),
                    (
                        expected.rejected_border,
                        expected.rejected_width,
                        expected.rejected_peak,
                        expected.rejected_saturation
                    )
                );
                assert_eq!(data, original);
            }
        }
    }
    #[test]
    fn local_contrast_preserves_raw_clipping_and_recovers_stripe_over_pedestal() {
        let mut data = [200u8; 15];
        data[4] = 250;
        data[10] = 255;
        let original = data;
        let config = LineConfig {
            background: 2,
            minimum_peak: 20,
            maximum_width: 3,
            saturation: 255,
            maximum_observations: 10,
        };
        let image = GrayImage::new(15, 1, &data).unwrap();
        assert!(
            detect_rows(image, config, &WorkControl::default())
                .unwrap()
                .observations
                .is_empty()
        );
        let output = detect_rows_with_opening(image, config, 2, &WorkControl::default()).unwrap();
        assert_eq!(output.rejected_saturation, 1);
        assert_eq!(output.observations.len(), 1);
        assert_eq!(output.observations[0].pixel, [4., 0.]);
        assert_eq!(output.observations[0].peak, 50);
        assert_eq!(output.observations[0].integrated_signal, 48);
        assert_eq!(data, original);
    }
    #[test]
    fn local_opening_handles_gradient_and_keeps_cancellation_and_limits() {
        let mut data: Vec<_> = (0..41).map(|x| 100 + x as u8).collect();
        for center in [10, 30] {
            for (offset, extra) in [20, 40, 20].into_iter().enumerate() {
                data[center + offset - 1] += extra;
            }
        }
        let image = GrayImage::new(41, 1, &data).unwrap();
        let config = LineConfig {
            background: 3,
            minimum_peak: 20,
            maximum_width: 8,
            saturation: 255,
            maximum_observations: 10,
        };
        let output = detect_rows_with_opening(image, config, 4, &WorkControl::default()).unwrap();
        assert_eq!(output.observations.len(), 2);
        for (observation, expected) in output.observations.iter().zip([10., 30.]) {
            assert!((observation.pixel[0] - expected).abs() < 0.2);
        }
        for radius in [0, 21, 129] {
            assert!(
                detect_rows_with_opening(image, config, radius, &WorkControl::default()).is_err()
            );
        }
        let work = WorkControl::default();
        work.cancel();
        assert!(
            detect_rows_with_opening(image, config, 4, &work)
                .unwrap_err()
                .contains("cancelled")
        );
        let limited = LineConfig {
            maximum_observations: 1,
            ..config
        };
        assert!(detect_rows_with_opening(image, limited, 4, &WorkControl::default()).is_err());
    }
    #[test]
    fn analytic_subpixel_centroid_multiple_stripes_and_source_preservation() {
        let data = [10, 20, 40, 30, 10, 10, 50, 10];
        let original = data;
        let output = detect_rows(
            GrayImage::new(8, 1, &data).unwrap(),
            config(),
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(output.observations.len(), 2);
        let a = &output.observations[0];
        assert!((a.pixel[0] - 13. / 6.).abs() < 1e-14);
        assert!((a.variance_x - 17. / 36.).abs() < 1e-14);
        assert_eq!(a.integrated_signal, 60);
        assert_eq!(output.observations[1].pixel, [6., 0.]);
        assert_eq!(data, original);
    }
    #[test]
    fn rejects_borders_width_clipping_and_weak_runs_without_merging() {
        let data = [40, 10, 20, 20, 20, 20, 20, 10, 255, 10, 20, 10, 40];
        let out = detect_rows(
            GrayImage::new(13, 1, &data).unwrap(),
            config(),
            &WorkControl::default(),
        )
        .unwrap();
        assert!(out.observations.is_empty());
        assert_eq!(
            (
                out.rejected_border,
                out.rejected_width,
                out.rejected_saturation,
                out.rejected_peak
            ),
            (2, 1, 1, 1)
        );
    }
    #[test]
    fn cancellation_invalid_limits_and_overflow_never_return_partial_results() {
        let data = [10, 40, 10, 40, 10];
        let image = GrayImage::new(5, 1, &data).unwrap();
        let mut c = config();
        c.maximum_observations = 1;
        assert!(
            detect_rows(image, c, &WorkControl::default())
                .unwrap_err()
                .contains("limit exceeded")
        );
        c = config();
        c.saturation = c.minimum_peak;
        assert!(detect_rows(image, c, &WorkControl::default()).is_err());
        let work = WorkControl::default();
        work.cancel();
        assert!(
            detect_rows(image, config(), &work)
                .unwrap_err()
                .contains("cancelled")
        );
    }
}
