//! Bounded eight-connected bright-component measurements in immutable GREY images.
//! Original flood-fill implementation; no upstream code is copied. Components
//! are candidate image regions, not identified markers, circles or calibration.
use crate::GrayImage;
use scancraft_geometry::work::{Stage, WorkControl};

#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Pixels strictly above this raw 8-bit intensity belong to the foreground.
    pub threshold: u8,
    pub minimum_area: u32,
    pub maximum_area: u32,
    /// Counts all components, including rejected noise. Exceeding it is an error.
    pub maximum_components: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    /// Binary area centroid in raw pixel-center coordinates, not a fitted center.
    pub pixel: [f64; 2],
    pub area: u32,
    /// Inclusive [minimum x, minimum y, maximum x, maximum y].
    pub bounds: [u32; 4],
    pub peak: u8,
    /// Raw samples equal to 255; reported rather than corrected or discarded.
    pub saturated_pixels: u32,
}

#[derive(Debug, Default)]
pub struct Detections {
    pub observations: Vec<Observation>,
    pub components_examined: usize,
    pub foreground_pixels: usize,
    pub rejected_area: usize,
    pub rejected_border: usize,
}

/// O(image pixels) eight-neighbor flood fill. Area rejection precedes border
/// rejection; every component is counted exactly once. Border-touching regions
/// are omitted because their centroids may be clipped. Scratch storage contains
/// one byte and one u32 slot per image pixel (at most 20 MiB at MAX_PIXELS), plus
/// bounded output. Cancellation is checked at most every 1,024 scan/fill steps.
/// No partial detections are returned on cancellation or a resource-limit error.
pub fn detect(
    image: GrayImage<'_>,
    config: Config,
    work: &WorkControl,
) -> Result<Detections, String> {
    if config.threshold == 255
        || config.minimum_area == 0
        || config.maximum_area < config.minimum_area
        || config.maximum_area as usize > image.data.len()
        || !(1..=100_000).contains(&config.maximum_components)
    {
        return Err("invalid bright-component detection limits".into());
    }
    work.update(Stage::ImageMapping, 0, image.data.len())?;
    let mut seen = vec![0u8; image.data.len()];
    let mut pending = Vec::<u32>::with_capacity(image.data.len());
    let width = image.width as usize;
    let height = image.height as usize;
    let mut output = Detections::default();
    let mut visits = 0usize;
    for start in 0..image.data.len() {
        if start % 1024 == 0 {
            work.update(Stage::ImageMapping, start, image.data.len())?;
        }
        if seen[start] != 0 || image.data[start] <= config.threshold {
            continue;
        }
        if output.components_examined == config.maximum_components {
            return Err("bright-component count exceeds configured limit".into());
        }
        output.components_examined += 1;
        pending.push(start as u32);
        seen[start] = 1;
        let mut area = 0u32;
        let mut sums = [0u64; 2];
        let mut bounds = [image.width, image.height, 0, 0];
        let mut peak = 0;
        let mut saturated_pixels = 0;
        while let Some(index) = pending.pop() {
            if visits.is_multiple_of(1024) {
                work.check()?;
            }
            visits += 1;
            let index = index as usize;
            let (x, y) = (index % width, index / width);
            let sample = image.data[index];
            area += 1;
            sums[0] += x as u64;
            sums[1] += y as u64;
            bounds[0] = bounds[0].min(x as u32);
            bounds[1] = bounds[1].min(y as u32);
            bounds[2] = bounds[2].max(x as u32);
            bounds[3] = bounds[3].max(y as u32);
            peak = peak.max(sample);
            saturated_pixels += u32::from(sample == 255);
            for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    let neighbor = ny * width + nx;
                    if seen[neighbor] == 0 && image.data[neighbor] > config.threshold {
                        seen[neighbor] = 1;
                        pending.push(neighbor as u32);
                    }
                }
            }
        }
        output.foreground_pixels += area as usize;
        if area < config.minimum_area || area > config.maximum_area {
            output.rejected_area += 1;
        } else if bounds[0] == 0
            || bounds[1] == 0
            || bounds[2] == image.width - 1
            || bounds[3] == image.height - 1
        {
            output.rejected_border += 1;
        } else {
            output.observations.push(Observation {
                pixel: [
                    sums[0] as f64 / f64::from(area),
                    sums[1] as f64 / f64::from(area),
                ],
                area,
                bounds,
                peak,
                saturated_pixels,
            });
        }
    }
    work.update(Stage::ImageMapping, image.data.len(), image.data.len())?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(pixels: u32) -> Config {
        Config {
            threshold: 100,
            minimum_area: 1,
            maximum_area: pixels,
            maximum_components: 100,
        }
    }
    #[test]
    fn diagonal_connections_and_binary_centroid_do_not_weight_brightness() {
        let mut pixels = vec![0; 49];
        pixels[2 * 7 + 2] = 101;
        pixels[3 * 7 + 3] = 255;
        pixels[4 * 7 + 4] = 200;
        pixels[5 * 7 + 1] = 100; // Exclusive threshold.
        let original = pixels.clone();
        let d = detect(
            GrayImage::new(7, 7, &pixels).unwrap(),
            config(49),
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(pixels, original);
        assert_eq!(d.components_examined, 1);
        assert_eq!(d.foreground_pixels, 3);
        assert_eq!(
            d.observations,
            vec![Observation {
                pixel: [3., 3.],
                area: 3,
                bounds: [2, 2, 4, 4],
                peak: 255,
                saturated_pixels: 1
            }]
        );
    }
    #[test]
    fn area_and_border_rejections_partition_all_components() {
        let mut pixels = vec![0; 100];
        for (x, y) in [
            (0, 0),
            (0, 1),
            (3, 3),
            (6, 3),
            (7, 3),
            (6, 6),
            (7, 6),
            (8, 6),
        ] {
            pixels[y * 10 + x] = 150;
        }
        let mut c = config(100);
        c.minimum_area = 2;
        c.maximum_area = 2;
        let d = detect(
            GrayImage::new(10, 10, &pixels).unwrap(),
            c,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(
            (
                d.components_examined,
                d.foreground_pixels,
                d.rejected_area,
                d.rejected_border
            ),
            (4, 8, 2, 1)
        );
        assert_eq!(d.observations.len(), 1);
        assert_eq!(d.observations[0].pixel, [6.5, 3.]);
    }
    #[test]
    fn noise_counts_toward_limit_instead_of_being_silently_truncated() {
        let mut pixels = vec![0; 49];
        pixels[8] = 200;
        pixels[12] = 200;
        let mut c = config(49);
        c.minimum_area = 2;
        c.maximum_components = 1;
        assert!(
            detect(
                GrayImage::new(7, 7, &pixels).unwrap(),
                c,
                &WorkControl::default()
            )
            .unwrap_err()
            .contains("count exceeds")
        );
        c.maximum_components = 2;
        let d = detect(
            GrayImage::new(7, 7, &pixels).unwrap(),
            c,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(d.rejected_area, 2);
    }
    #[test]
    fn invalid_limits_and_pre_cancelled_work_fail() {
        let pixels = vec![0; 25];
        let image = GrayImage::new(5, 5, &pixels).unwrap();
        let c = config(25);
        for bad in [
            Config {
                threshold: 255,
                ..c
            },
            Config {
                minimum_area: 0,
                ..c
            },
            Config {
                maximum_area: 0,
                ..c
            },
            Config {
                maximum_area: 26,
                ..c
            },
            Config {
                maximum_components: 0,
                ..c
            },
            Config {
                maximum_components: 100_001,
                ..c
            },
        ] {
            assert!(detect(image, bad, &WorkControl::default()).is_err());
        }
        let work = WorkControl::default();
        work.cancel();
        assert!(detect(image, c, &work).unwrap_err().contains("cancelled"));
    }
    #[test]
    fn singleton_and_dense_frame_are_border_regions_and_empty_frame_finishes() {
        for (w, h) in [(1, 1), (1, 32), (32, 1), (32, 32)] {
            let pixels = vec![255; (w * h) as usize];
            let d = detect(
                GrayImage::new(w, h, &pixels).unwrap(),
                config(w * h),
                &WorkControl::default(),
            )
            .unwrap();
            assert_eq!(d.components_examined, 1);
            assert_eq!(d.rejected_border, 1);
            assert!(d.observations.is_empty());
        }
        let pixels = vec![0; 25];
        let work = WorkControl::default();
        let d = detect(GrayImage::new(5, 5, &pixels).unwrap(), config(25), &work).unwrap();
        assert_eq!(d.components_examined, 0);
        assert_eq!(work.progress().1, 1.);
    }
}
