//! Paired rectification maps retain their exact source/output geometry.
use crate::{GrayImage, MaskedImage, UndistortMap};
use scancraft_calibration::rectification::StereoRectification;
use scancraft_geometry::work::WorkControl;

pub struct StereoImageMaps {
    rectification: StereoRectification,
    left: UndistortMap,
    right: UndistortMap,
}
impl StereoImageMaps {
    pub fn build(rectification: StereoRectification, work: &WorkControl) -> Result<Self, String> {
        let source = rectification.source();
        let output = rectification.geometry();
        let left = UndistortMap::build(
            &source.left,
            &output.left,
            rectification.left_from_rectified().rotation,
            work,
        )?;
        let right = UndistortMap::build(
            &source.right,
            &output.right,
            rectification.right_from_rectified().rotation,
            work,
        )?;
        Ok(Self {
            rectification,
            left,
            right,
        })
    }
    pub fn rectification(&self) -> &StereoRectification {
        &self.rectification
    }
    /// Return neither image if one fails. Large maps use one scoped helper when
    /// at least two CPUs are reported. Small maps/resource failures stay sequential.
    /// Shared progress is advisory and may interleave. Does not establish capture timing.
    pub fn apply(
        &self,
        left: GrayImage<'_>,
        right: GrayImage<'_>,
        work: &WorkControl,
    ) -> Result<(MaskedImage, MaskedImage), String> {
        // Measured crossover policy, not a universal throughput guarantee.
        if self.left.coordinates().len() < 320 * 240
            || self.right.coordinates().len() < 320 * 240
            || !std::thread::available_parallelism().is_ok_and(|n| n.get() >= 2)
        {
            return self.apply_sequential(left, right, work);
        }
        if left.size() != self.left.source_size || right.size() != self.right.source_size {
            return Err("source image dimensions disagree with calibration map".into());
        }
        work.check()?;
        std::thread::scope(|scope| {
            let worker = match std::thread::Builder::new()
                .name("scancraft-left-remap".into())
                .spawn_scoped(scope, || self.left.apply(left, work))
            {
                Ok(worker) => worker,
                // Resource-constrained environments retain the portable sequential path.
                Err(_) => return self.apply_sequential(left, right, work),
            };
            let right = self.right.apply(right, work);
            // Join even when the right map returned an error. Neither image escapes alone.
            let left = worker
                .join()
                .map_err(|_| "left remap worker panicked".to_string())?;
            Ok((left?, right?))
        })
    }
    /// Explicit one-thread reference/fallback; identical map sampling and arithmetic.
    pub fn apply_sequential(
        &self,
        left: GrayImage<'_>,
        right: GrayImage<'_>,
        work: &WorkControl,
    ) -> Result<(MaskedImage, MaskedImage), String> {
        let left = self.left.apply(left, work)?;
        let right = self.right.apply(right, work)?;
        Ok((left, right))
    }
    /// Coordinate-buffer bytes only, excluding outputs, allocator overhead and process RSS.
    pub fn coordinate_storage_bytes(&self) -> usize {
        std::mem::size_of_val(self.left.coordinates())
            + std::mem::size_of_val(self.right.coordinates())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_calibration::{
        Brown5, CameraModel,
        stereo::{RigidPose, StereoCalibration},
    };
    fn fixture(width: u32, height: u32) -> (StereoImageMaps, [Vec<u8>; 2]) {
        let left = CameraModel {
            width,
            height,
            fx: 200.,
            fy: 201.,
            cx: f64::from(width - 1) / 2.,
            cy: f64::from(height - 1) / 2.,
            skew: 0.01,
            distortion: Brown5 {
                k1: -0.04,
                k2: 0.01,
                k3: 0.0002,
                p1: 0.0001,
                p2: -0.0002,
            },
            valid_radius: 1.3,
        };
        let mut right = left.clone();
        right.fx = 203.;
        right.cx += 0.8;
        let angle = 0.015_f64;
        let source = StereoCalibration {
            left: left.clone(),
            right,
            right_from_left: RigidPose {
                rotation: [
                    [angle.cos(), 0., angle.sin()],
                    [0., 1., 0.],
                    [-angle.sin(), 0., angle.cos()],
                ],
                translation_mm: [-45., 0.1, 0.3],
            },
        };
        let mut grid = left;
        grid.skew = 0.;
        grid.distortion = Brown5::ZERO;
        let model = StereoRectification::build(&source, grid).unwrap();
        let maps = StereoImageMaps::build(model, &WorkControl::default()).unwrap();
        let data = [0, 1].map(|side| {
            (0..width as usize * height as usize)
                .map(|i| ((i * 13 + i / width as usize * 7 + side * 53) % 256) as u8)
                .collect()
        });
        (maps, data)
    }
    fn compare(width: u32, height: u32) {
        let (maps, data) = fixture(width, height);
        let work = WorkControl::default();
        let images = [
            GrayImage::new(width, height, &data[0]).unwrap(),
            GrayImage::new(width, height, &data[1]).unwrap(),
        ];
        let reference = maps.apply_sequential(images[0], images[1], &work).unwrap();
        let output = maps.apply(images[0], images[1], &work).unwrap();
        for (a, b) in [(&reference.0, &output.0), (&reference.1, &output.1)] {
            assert_eq!(a.valid(), b.valid());
            assert_eq!(a.size(), b.size());
            assert!(
                a.intensity()
                    .iter()
                    .zip(b.intensity())
                    .all(|(a, b)| a.to_bits() == b.to_bits())
            );
        }
        assert_eq!(
            maps.coordinate_storage_bytes(),
            2 * width as usize * height as usize * std::mem::size_of::<Option<[f64; 2]>>()
        );
        if width >= 320 {
            assert!(output.1.valid().iter().any(|v| !*v));
            assert!(output.1.valid().iter().any(|v| *v));
        }
    }
    #[test]
    fn large_distorted_rotated_pair_matches_sequential_bits_and_masks() {
        compare(320, 240);
    }
    #[test]
    fn small_pair_keeps_sequential_contract() {
        compare(7, 5);
    }
    #[test]
    fn large_pair_rejects_wrong_source_and_cancel_without_returning_partial_images() {
        let (maps, data) = fixture(320, 240);
        let image = GrayImage::new(320, 240, &data[0]).unwrap();
        let wrong = GrayImage::new(1, 1, &[1]).unwrap();
        let work = WorkControl::default();
        assert!(maps.apply(image, wrong, &work).is_err());
        work.cancel();
        assert!(
            maps.apply(image, image, &work)
                .err()
                .unwrap()
                .contains("cancelled")
        );
    }
}
