//! Independent native image preparation. Integer coordinates denote pixel centers.
//! No guessed calibration, lens scaling, projector state, or exposure normalization.
use scancraft_calibration::{Brown5, CameraModel, stereo::RigidPose};
use scancraft_core::{Frame, StreamProfile, validate_frame};
use scancraft_geometry::work::{Stage, WorkControl};

pub mod diagnostics;
pub mod laser;
pub mod stereo;

pub const MAX_PIXELS: usize = 4_194_304;
fn pixels(width: u32, height: u32) -> Result<usize, String> {
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or("image size overflow")?;
    if width == 0 || height == 0 || count > MAX_PIXELS {
        return Err("image dimensions must contain 1..=4194304 pixels".into());
    }
    Ok(count)
}
/// Borrowed packed 8-bit monochrome image. Payload is not copied.
#[derive(Clone, Copy)]
pub struct GrayImage<'a> {
    width: u32,
    height: u32,
    data: &'a [u8],
}
impl<'a> GrayImage<'a> {
    pub fn new(width: u32, height: u32, data: &'a [u8]) -> Result<Self, String> {
        if pixels(width, height)? != data.len() {
            return Err("packed grayscale image length does not match dimensions".into());
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }
    /// Requires the existing raw-frame validity checks, including genuine sensor PTS.
    pub fn from_frame(profile: &StreamProfile, frame: &'a Frame) -> Result<Self, String> {
        validate_frame(profile, frame)?;
        if profile.fourcc != *b"GREY" {
            return Err("image preparation currently accepts only packed GREY frames".into());
        }
        Self::new(profile.width, profile.height, &frame.data)
    }
    pub fn size(&self) -> [u32; 2] {
        [self.width, self.height]
    }
    /// Copy each row in reverse X order into reusable packed GREY scratch space.
    /// Source frame bytes/metadata are unchanged. This explicit image operation
    /// does not select a device orientation or transform its calibration.
    /// Invalid output length fails before any destination sample is changed.
    pub fn reflect_x_into(&self, destination: &mut [u8]) -> Result<(), String> {
        if destination.len() != self.data.len() {
            return Err("reflection output length differs from packed image".into());
        }
        let width = self.width as usize;
        for (source, output) in self
            .data
            .chunks_exact(width)
            .zip(destination.chunks_exact_mut(width))
        {
            for (sample, slot) in source.iter().rev().zip(output) {
                *slot = *sample;
            }
        }
        Ok(())
    }
    fn bilinear(&self, [x, y]: [f64; 2]) -> Option<f32> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.
            || y < 0.
            || x > f64::from(self.width - 1)
            || y > f64::from(self.height - 1)
        {
            return None;
        }
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width as usize - 1);
        let y1 = (y0 + 1).min(self.height as usize - 1);
        let dx = x - x0 as f64;
        let dy = y - y0 as f64;
        let at = |x, y| f64::from(self.data[y * self.width as usize + x]);
        let a = at(x0, y0) * (1. - dx) + at(x1, y0) * dx;
        let b = at(x0, y1) * (1. - dx) + at(x1, y1) * dx;
        Some((a * (1. - dy) + b * dy) as f32)
    }
}
/// Output intensities retain fractional 8-bit values. Invalid pixels have a zero
/// storage value but must be accessed through the validity mask or `sample`.
pub struct MaskedImage {
    width: u32,
    height: u32,
    intensity: Vec<f32>,
    valid: Vec<bool>,
}
impl MaskedImage {
    /// Construct a bounded image for processing. Invalid storage is canonicalized
    /// to zero; valid floating intensities use the 8-bit 0..=255 scale.
    pub fn new(
        width: u32,
        height: u32,
        mut intensity: Vec<f32>,
        valid: Vec<bool>,
    ) -> Result<Self, String> {
        let count = pixels(width, height)?;
        if intensity.len() != count
            || valid.len() != count
            || intensity
                .iter()
                .any(|v| !v.is_finite() || !(0. ..=255.).contains(v))
        {
            return Err("masked image dimensions, intensities or validity are invalid".into());
        }
        for (v, valid) in intensity.iter_mut().zip(&valid) {
            if !valid {
                *v = 0.;
            }
        }
        Ok(Self {
            width,
            height,
            intensity,
            valid,
        })
    }
    pub fn size(&self) -> [u32; 2] {
        [self.width, self.height]
    }
    pub fn intensity(&self) -> &[f32] {
        &self.intensity
    }
    pub fn valid(&self) -> &[bool] {
        &self.valid
    }
    pub fn sample(&self, x: u32, y: u32) -> Option<f32> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = y as usize * self.width as usize + x as usize;
        self.valid[i].then_some(self.intensity[i])
    }
}
/// Destination-to-source map. Rotation maps output-camera rays into the source
/// camera; it is not a stereo translation or a physical camera calibration estimate.
pub struct UndistortMap {
    source_size: [u32; 2],
    output_size: [u32; 2],
    coordinates: Vec<Option<[f64; 2]>>,
}
impl UndistortMap {
    pub fn build(
        source: &CameraModel,
        output: &CameraModel,
        source_from_output: [[f64; 3]; 3],
        work: &WorkControl,
    ) -> Result<Self, String> {
        pixels(source.width, source.height)?;
        let count = pixels(output.width, output.height)?;
        let source_prepared = source.prepare()?;
        output.validate()?;
        if output.distortion != Brown5::ZERO {
            return Err("output grid must be an ideal pinhole camera".into());
        }
        RigidPose {
            rotation: source_from_output,
            translation_mm: [0.; 3],
        }
        .validate()?;
        let mut coordinates = Vec::with_capacity(count);
        for y in 0..output.height {
            for x in 0..output.width {
                let index = y as usize * output.width as usize + x as usize;
                if index.is_multiple_of(1024) {
                    work.update(Stage::ImageMapping, index, count)?;
                }
                let ny = (f64::from(y) - output.cy) / output.fy;
                let nx = (f64::from(x) - output.cx - output.skew * ny) / output.fx;
                let ray = [nx, ny, 1.];
                let source_ray =
                    source_from_output.map(|row| row.iter().zip(ray).map(|(a, b)| a * b).sum());
                let coordinate = if nx.hypot(ny) <= output.valid_radius {
                    source_prepared.project(source_ray).ok().and_then(|p| {
                        // Absorb only floating-point roundoff at border centers. A genuine
                        // outside sample stays invalid; there is no extrapolated padding.
                        let end = [f64::from(source.width - 1), f64::from(source.height - 1)];
                        let epsilon =
                            64. * f64::EPSILON * f64::from(source.width.max(source.height));
                        p.iter()
                            .zip(end)
                            .all(|(v, end)| v.is_finite() && *v >= -epsilon && *v <= end + epsilon)
                            .then(|| [p[0].clamp(0., end[0]), p[1].clamp(0., end[1])])
                    })
                } else {
                    None
                };
                coordinates.push(coordinate);
            }
        }
        work.update(Stage::ImageMapping, count, count)?;
        Ok(Self {
            source_size: [source.width, source.height],
            output_size: [output.width, output.height],
            coordinates,
        })
    }
    pub fn size(&self) -> [u32; 2] {
        self.output_size
    }
    pub fn coordinates(&self) -> &[Option<[f64; 2]>] {
        &self.coordinates
    }
    pub fn apply(&self, source: GrayImage<'_>, work: &WorkControl) -> Result<MaskedImage, String> {
        if source.size() != self.source_size {
            return Err("source image dimensions disagree with calibration map".into());
        }
        let mut intensity = Vec::with_capacity(self.coordinates.len());
        let mut valid = Vec::with_capacity(self.coordinates.len());
        for (i, p) in self.coordinates.iter().enumerate() {
            if i % 1024 == 0 {
                work.update(Stage::Resampling, i, self.coordinates.len())?;
            }
            let value = p.and_then(|p| source.bilinear(p));
            intensity.push(value.unwrap_or(0.));
            valid.push(value.is_some());
        }
        work.update(
            Stage::Resampling,
            self.coordinates.len(),
            self.coordinates.len(),
        )?;
        Ok(MaskedImage {
            width: self.output_size[0],
            height: self.output_size[1],
            intensity,
            valid,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const IDENTITY: [[f64; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    fn camera() -> CameraModel {
        CameraModel {
            width: 7,
            height: 5,
            fx: 4.,
            fy: 4.,
            cx: 3.,
            cy: 2.,
            skew: 0.,
            distortion: Brown5::ZERO,
            valid_radius: 2.,
        }
    }
    #[test]
    fn masked_image_constructor_rejects_malformed_data_and_keeps_invalid_samples_absent() {
        assert!(MaskedImage::new(2, 1, vec![1.], vec![true; 2]).is_err());
        assert!(MaskedImage::new(2, 1, vec![1., 2.], vec![true]).is_err());
        for v in [f32::NAN, f32::INFINITY, -1., 256.] {
            assert!(MaskedImage::new(1, 1, vec![v], vec![true]).is_err());
        }
        let image = MaskedImage::new(2, 1, vec![12.5, 99.], vec![true, false]).unwrap();
        assert_eq!(image.sample(0, 0), Some(12.5));
        assert_eq!(image.sample(1, 0), None);
        assert_eq!(image.intensity(), [12.5, 0.]);
    }
    #[test]
    fn identity_preserves_every_pixel_including_last_row_and_column() {
        let c = camera();
        let data: Vec<_> = (0..35).map(|v| v * 7).collect();
        let map = UndistortMap::build(&c, &c, IDENTITY, &WorkControl::default()).unwrap();
        let out = map
            .apply(
                GrayImage::new(7, 5, &data).unwrap(),
                &WorkControl::default(),
            )
            .unwrap();
        assert!(out.valid().iter().all(|v| *v));
        for (i, v) in data.iter().enumerate() {
            assert_eq!(out.intensity()[i], f32::from(*v));
        }
        assert_eq!(out.sample(6, 4), Some(238.));
        assert_eq!(out.sample(7, 4), None);
    }
    #[test]
    fn identity_with_nonbinary_intrinsics_keeps_edge_pixels() {
        let mut c = camera();
        c.width = 1920;
        c.height = 1200;
        c.fx = 1382.317;
        c.fy = 1379.117;
        c.cx = 960.713;
        c.cy = 602.332;
        let map = UndistortMap::build(&c, &c, IDENTITY, &WorkControl::default()).unwrap();
        assert!(map.coordinates().iter().all(Option::is_some));
    }
    #[test]
    fn bilinear_ramp_shift_and_invalid_border_are_analytic() {
        let c = camera();
        let mut grid = c.clone();
        grid.cx -= 0.25;
        grid.cy -= 0.5;
        let data: Vec<u8> = (0..5)
            .flat_map(|y| (0..7).map(move |x| 10 * x + 20 * y))
            .collect();
        let map = UndistortMap::build(&c, &grid, IDENTITY, &WorkControl::default()).unwrap();
        let out = map
            .apply(
                GrayImage::new(7, 5, &data).unwrap(),
                &WorkControl::default(),
            )
            .unwrap();
        for y in 0..5 {
            for x in 0..7 {
                if x < 6 && y < 4 {
                    assert_eq!(
                        out.sample(x, y),
                        Some(10. * (x as f32 + 0.25) + 20. * (y as f32 + 0.5))
                    );
                } else {
                    assert_eq!(out.sample(x, y), None);
                }
            }
        }
        assert_eq!(out.intensity()[34], 0.);
        assert!(!out.valid()[34]);
    }
    #[test]
    fn bilinear_singleton_and_invalid_coordinates() {
        let image = GrayImage::new(1, 1, &[29]).unwrap();
        assert_eq!(image.bilinear([0., 0.]), Some(29.));
        for p in [[f64::NAN, 0.], [0., f64::INFINITY], [-0.01, 0.], [0., 0.01]] {
            assert_eq!(image.bilinear(p), None);
        }
    }
    #[test]
    fn invalid_models_rotation_and_oversize_fail_before_mapping() {
        let c = camera();
        let mut grid = c.clone();
        grid.distortion.k1 = 0.01;
        assert!(UndistortMap::build(&c, &grid, IDENTITY, &WorkControl::default()).is_err());
        assert!(UndistortMap::build(&c, &c, [[1.; 3]; 3], &WorkControl::default()).is_err());
        assert!(GrayImage::new(0, 1, &[]).is_err());
        assert!(GrayImage::new(u32::MAX, u32::MAX, &[]).is_err());
        assert!(GrayImage::new(7, 5, &[0; 34]).is_err());
    }
    #[test]
    fn coverage_mask_and_turned_away_rays_do_not_become_black_observations() {
        let mut c = camera();
        c.valid_radius = 0.1;
        let map = UndistortMap::build(&c, &c, IDENTITY, &WorkControl::default()).unwrap();
        assert_eq!(map.coordinates().iter().filter(|v| v.is_some()).count(), 1);
        let back = [[-1., 0., 0.], [0., 1., 0.], [0., 0., -1.]];
        let map = UndistortMap::build(&c, &c, back, &WorkControl::default()).unwrap();
        assert!(map.coordinates().iter().all(Option::is_none));
    }
    #[test]
    fn distorted_skewed_map_agrees_with_inverse_camera_rays() {
        let mut source = camera();
        source.skew = 0.13;
        source.distortion = Brown5 {
            k1: -0.04,
            k2: 0.01,
            k3: 0.001,
            p1: 0.0002,
            p2: -0.0003,
        };
        let mut output = source.clone();
        output.distortion = Brown5::ZERO;
        output.skew = 0.07;
        let a = 0.04f64;
        let rotation = [
            [a.cos(), 0., a.sin()],
            [0., 1., 0.],
            [-a.sin(), 0., a.cos()],
        ];
        let map = UndistortMap::build(&source, &output, rotation, &WorkControl::default()).unwrap();
        let prepared = source.prepare().unwrap();
        let mut checked = 0;
        for (i, p) in map.coordinates().iter().enumerate() {
            if let Some(p) = p {
                let ray = output.ray([(i % 7) as f64, (i / 7) as f64]).unwrap();
                let expected = rotation.map(|r| r.iter().zip(ray).map(|(a, b)| a * b).sum::<f64>());
                let actual = prepared.ray(*p).unwrap();
                for axis in 0..3 {
                    assert!((actual[axis] - expected[axis]).abs() < 1e-10);
                }
                assert_eq!(
                    prepared.project(expected).unwrap(),
                    source.project(expected).unwrap()
                );
                checked += 1;
            }
        }
        assert!(checked > 20);
    }
    #[test]
    fn source_rotation_direction_is_checked_by_known_ninety_degree_rotation() {
        let mut c = camera();
        c.width = 5;
        c.cx = 2.;
        let rotation = [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]];
        let map = UndistortMap::build(&c, &c, rotation, &WorkControl::default()).unwrap();
        assert_eq!(map.coordinates()[2 * 5 + 3], Some([2., 3.]));
    }
    #[test]
    fn cancellation_and_wrong_source_size_fail_without_partial_output() {
        let c = camera();
        let work = WorkControl::default();
        work.cancel();
        assert!(UndistortMap::build(&c, &c, IDENTITY, &work).is_err());
        let map = UndistortMap::build(&c, &c, IDENTITY, &WorkControl::default()).unwrap();
        let image = GrayImage::new(7, 5, &[0; 35]).unwrap();
        assert!(map.apply(image, &work).is_err());
        assert!(
            map.apply(GrayImage::new(1, 1, &[0]).unwrap(), &WorkControl::default())
                .is_err()
        );
    }
    #[test]
    fn raw_frame_adapter_keeps_core_validation_and_borrows_payload() {
        let profile = StreamProfile {
            interface: 0,
            width: 7,
            height: 5,
            fourcc: *b"GREY",
            fps: 50,
        };
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12] = 1;
        let mut frame = Frame {
            sequence: 1,
            host_timestamp_us: 1,
            flags: 0,
            metadata,
            data: vec![0; 35],
        };
        let image = GrayImage::from_frame(&profile, &frame).unwrap();
        assert_eq!(image.data.as_ptr(), frame.data.as_ptr());
        frame.flags = 0x40;
        assert!(GrayImage::from_frame(&profile, &frame).is_err());
        frame.flags = 0;
        frame.metadata.clear();
        assert!(GrayImage::from_frame(&profile, &frame).is_err());
    }
}
