//! Shape diagnostics for measured bright components; no physical center guarantee.
use crate::{
    GrayImage,
    components::{Config, Observation, detect},
};
use scancraft_calibration::ellipse::{Fit, fit};
use scancraft_geometry::work::WorkControl;

/// Revalidate an observation against the same image/threshold, then fit the
/// foreground pixel centers adjacent to background in the four cardinal directions.
/// The padded region is limited to 65,536 pixels, its contour to 4,096 samples.
/// Extra components, clipping, altered observations and resource limits are errors.
/// Hole boundaries participate; this is not subpixel contour extraction.
pub fn measure(
    image: GrayImage<'_>,
    threshold: u8,
    observation: &Observation,
    work: &WorkControl,
) -> Result<Fit, String> {
    work.check()?;
    let [x0, y0, x1, y1] = observation.bounds;
    let [width, height] = image.size();
    if threshold == 255
        || x0 == 0
        || y0 == 0
        || x0 > x1
        || y0 > y1
        || x1 >= width - 1
        || y1 >= height - 1
    {
        return Err("invalid or border-touching shape bounds".into());
    }
    let w = x1 - x0 + 3;
    let h = y1 - y0 + 3;
    if u64::from(w) * u64::from(h) > 65_536 {
        return Err("shape region exceeds pixel limit".into());
    }
    let mut data = Vec::with_capacity((w * h) as usize);
    for y in y0 - 1..=y1 + 1 {
        work.check()?;
        let start = (y * width + x0 - 1) as usize;
        data.extend_from_slice(&image.data[start..start + w as usize]);
    }
    let patch = GrayImage::new(w, h, &data)?;
    let found = detect(
        patch,
        Config {
            threshold,
            minimum_area: 1,
            maximum_area: w * h,
            maximum_components: 2,
        },
        work,
    )?;
    if found.components_examined != 1 || found.observations.len() != 1 {
        return Err("shape region is not one isolated component".into());
    }
    let mut actual = found.observations[0].clone();
    actual.pixel[0] += f64::from(x0 - 1);
    actual.pixel[1] += f64::from(y0 - 1);
    for i in [0, 2] {
        actual.bounds[i] += x0 - 1;
        actual.bounds[i + 1] += y0 - 1;
    }
    if actual.bounds != observation.bounds
        || actual.area != observation.area
        || actual.peak != observation.peak
        || actual.saturated_pixels != observation.saturated_pixels
        || actual
            .pixel
            .iter()
            .zip(observation.pixel)
            .any(|(a, b)| !b.is_finite() || (a - b).abs() > 1e-9)
    {
        return Err("shape observation disagrees with source image".into());
    }
    let mut boundary = Vec::new();
    for y in 1..h - 1 {
        work.check()?;
        for x in 1..w - 1 {
            let i = (y * w + x) as usize;
            if data[i] > threshold
                && [i - 1, i + 1, i - w as usize, i + w as usize]
                    .iter()
                    .any(|&j| data[j] <= threshold)
            {
                if boundary.len() == 4096 {
                    return Err("shape boundary exceeds sample limit".into());
                }
                boundary.push([f64::from(x + x0 - 1), f64::from(y + y0 - 1)]);
            }
        }
    }
    fit(&boundary, || work.check().is_err())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn image(cut: bool) -> Vec<u8> {
        (0..65)
            .flat_map(|y| {
                (0..65).map(move |x| {
                    if (x - 32i32).pow(2) + (y - 32i32).pow(2) <= 15 * 15 && (!cut || x >= 28) {
                        200
                    } else {
                        0
                    }
                })
            })
            .collect()
    }
    fn observe(data: &[u8]) -> Observation {
        detect(
            GrayImage::new(65, 65, data).unwrap(),
            Config {
                threshold: 100,
                minimum_area: 10,
                maximum_area: 1000,
                maximum_components: 10,
            },
            &WorkControl::default(),
        )
        .unwrap()
        .observations
        .remove(0)
    }
    #[test]
    fn round_and_cut_shapes_report_residuals_without_changing_raw_centroids() {
        let mut residuals = Vec::new();
        for cut in [false, true] {
            let data = image(cut);
            let o = observe(&data);
            let original = o.clone();
            let f = measure(
                GrayImage::new(65, 65, &data).unwrap(),
                100,
                &o,
                &WorkControl::default(),
            )
            .unwrap();
            assert_eq!(o, original);
            residuals.push(f.rms_sampson);
        }
        assert!(residuals[0] < 0.5 && residuals[1] > 0.5);
    }
    #[test]
    fn altered_observation_source_and_cancellation_are_rejected() {
        let mut data = image(false);
        let mut o = observe(&data);
        let work = WorkControl::default();
        work.cancel();
        assert!(measure(GrayImage::new(65, 65, &data).unwrap(), 100, &o, &work).is_err());
        o.area += 1;
        assert!(
            measure(
                GrayImage::new(65, 65, &data).unwrap(),
                100,
                &o,
                &WorkControl::default()
            )
            .is_err()
        );
        o.area -= 1;
        data[32 * 65 + 32] = 0;
        assert!(
            measure(
                GrayImage::new(65, 65, &data).unwrap(),
                100,
                &o,
                &WorkControl::default()
            )
            .is_err()
        );
        o.bounds = [0, 0, u32::MAX, u32::MAX];
        assert!(
            measure(
                GrayImage::new(65, 65, &data).unwrap(),
                100,
                &o,
                &WorkControl::default()
            )
            .is_err()
        );
    }

    #[test]
    fn unrelated_pixels_in_region_and_oversized_region_are_errors() {
        let mut data = image(false);
        let o = observe(&data);
        data[18 * 65 + 18] = 200;
        assert!(
            measure(
                GrayImage::new(65, 65, &data).unwrap(),
                100,
                &o,
                &WorkControl::default()
            )
            .is_err()
        );
        let large = vec![200; 300 * 300];
        let mut oversized = o;
        oversized.bounds = [1, 1, 298, 298];
        assert!(
            measure(
                GrayImage::new(300, 300, &large).unwrap(),
                100,
                &oversized,
                &WorkControl::default()
            )
            .unwrap_err()
            .contains("pixel limit")
        );
    }

    #[test]
    fn interior_hole_boundary_is_measured_rather_than_silently_filled() {
        let mut data = image(false);
        data[32 * 65 + 32] = 0;
        let o = observe(&data);
        let fitted = measure(
            GrayImage::new(65, 65, &data).unwrap(),
            100,
            &o,
            &WorkControl::default(),
        )
        .unwrap();
        assert!(fitted.rms_sampson > 0.5);
    }
}
