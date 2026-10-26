//! Threshold-run centroids for isolated laser stripes in raw grayscale images.
//! Detections are image observations, not identified projector lines or depth.
use crate::GrayImage;
use scancraft_geometry::work::{Stage, WorkControl};
use std::collections::VecDeque;

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
    let mut queue = VecDeque::with_capacity(2 * radius as usize + 1);
    for y in 0..image.height as usize {
        work.update(Stage::ImageMapping, y, image.height as usize)?;
        let raw = &image.data[y * width..(y + 1) * width];
        row_extreme(raw, radius as usize, true, &mut eroded, &mut queue);
        row_extreme(&eroded, radius as usize, false, &mut opened, &mut queue);
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
// Monotone queue: each sample enters/leaves at most once, independent of radius.
fn row_extreme(
    input: &[u8],
    radius: usize,
    minimum: bool,
    output: &mut [u8],
    queue: &mut VecDeque<usize>,
) {
    queue.clear();
    let mut end = 0;
    for (x, value) in output.iter_mut().enumerate() {
        let limit = (x + radius + 1).min(input.len());
        while end < limit {
            while let Some(&last) = queue.back() {
                if if minimum {
                    input[last] >= input[end]
                } else {
                    input[last] <= input[end]
                } {
                    queue.pop_back();
                } else {
                    break;
                }
            }
            queue.push_back(end);
            end += 1;
        }
        while queue
            .front()
            .is_some_and(|&first| first < x.saturating_sub(radius))
        {
            queue.pop_front();
        }
        *value = input[*queue.front().unwrap()];
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
    fn monotone_queue_matches_independent_window_extrema_at_every_boundary() {
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
                    row_extreme(&input, radius, minimum, &mut output, &mut VecDeque::new());
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
