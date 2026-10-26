//! Image preparation -> sparse correspondence -> original-camera triangulation.
//! The caller supplies source calibration, declared provenance and seed pixels;
//! no hardware timing, projector state or physical accuracy is inferred.
use crate::{
    MatchSource, PixelMatch, Reconstruction,
    matching::{AutoSeeds, MatchConfig, MatchResult, match_rectified, select_seeds},
    reconstruct, validate_source,
};
use scancraft_calibration::{
    CameraModel, rectification::StereoRectification, stereo::TriangulationLimits,
    storage::CalibrationRecord,
};
use scancraft_geometry::work::WorkControl;
use scancraft_image::{GrayImage, stereo::StereoImageMaps};
use serde::Serialize;

pub struct StereoProcessor {
    calibration: CalibrationRecord,
    maps: StereoImageMaps,
}
pub struct StereoRequest<'a> {
    pub left: GrayImage<'a>,
    pub right: GrayImage<'a>,
    pub source: MatchSource,
    /// Integer seed coordinates in the chosen rectified output grid.
    pub seeds: &'a [[u32; 2]],
    pub matching: MatchConfig,
    /// Depth bounds and reprojection limits are in the original camera frame.
    pub triangulation: TriangulationLimits,
}
#[derive(Debug, Clone, Serialize)]
pub struct MappedMatch {
    pub seed_index: usize,
    pub rectified_match_index: usize,
    pub observations: PixelMatch,
}
#[derive(Debug, Clone, Serialize)]
pub struct MappingFailure {
    pub seed_index: usize,
    pub rectified_match_index: usize,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct StereoReport {
    pub schema_version: u32,
    /// This processing API cannot verify the capture/exposure relationship.
    pub synchronization_verified: bool,
    pub physical_accuracy_qualified: bool,
    pub calibration: CalibrationRecord,
    pub source: MatchSource,
    pub rectification: StereoRectification,
    pub matching: MatchResult,
    pub seeds: Vec<[u32; 2]>,
    pub automatic_sampling: Option<AutoSeeds>,
    pub mapped_matches: Vec<MappedMatch>,
    pub mapping_failures: Vec<MappingFailure>,
    /// None when matching/mapping produced no observations. Rejection diagnostics
    /// are still retained; never manufacture vertices for a blank/ambiguous pair.
    pub reconstruction: Option<Reconstruction>,
}
impl StereoProcessor {
    pub fn build(
        calibration: CalibrationRecord,
        output_grid: CameraModel,
        work: &WorkControl,
    ) -> Result<Self, String> {
        calibration.validate()?;
        let rectification = StereoRectification::build(&calibration.geometry, output_grid)?;
        let maps = StereoImageMaps::build(rectification, work)?;
        Ok(Self { calibration, maps })
    }
    pub fn rectification(&self) -> &StereoRectification {
        self.maps.rectification()
    }
    pub fn process(
        &self,
        request: StereoRequest<'_>,
        work: &WorkControl,
    ) -> Result<StereoReport, String> {
        self.process_inner(request, None, work)
    }
    pub fn process_automatic(
        &self,
        request: StereoRequest<'_>,
        sampling: AutoSeeds,
        work: &WorkControl,
    ) -> Result<StereoReport, String> {
        if !request.seeds.is_empty() {
            return Err("automatic sampling cannot be combined with explicit seeds".into());
        }
        self.process_inner(request, Some(sampling), work)
    }
    fn process_inner(
        &self,
        request: StereoRequest<'_>,
        sampling: Option<AutoSeeds>,
        work: &WorkControl,
    ) -> Result<StereoReport, String> {
        validate_source(&self.calibration, &request.source)?;
        request.triangulation.validate()?;
        work.check()?;
        let (left, right) = self.maps.apply(request.left, request.right, work)?;
        let rectification = self.rectification();
        let seeds = match sampling {
            Some(s) => select_seeds(rectification.geometry(), &left, request.matching, s, work)?,
            None => request.seeds.to_vec(),
        };
        let matching = if sampling.is_some() && seeds.is_empty() {
            MatchResult {
                rectified_geometry: rectification.geometry().clone(),
                config: request.matching,
                accepted: Vec::new(),
                rejected: Vec::new(),
            }
        } else {
            match_rectified(
                rectification.geometry(),
                &left,
                &right,
                &seeds,
                request.matching,
                work,
            )?
        };
        let mut mapped_matches = Vec::new();
        let mut mapping_failures = Vec::new();
        for (rectified_match_index, m) in matching.accepted.iter().enumerate() {
            work.check()?;
            match rectification
                .source_pixels(m.left_pixel.map(f64::from), m.right_pixel.map(f64::from))
            {
                Ok((left_pixel, right_pixel)) => mapped_matches.push(MappedMatch {
                    seed_index: m.seed_index,
                    rectified_match_index,
                    observations: PixelMatch {
                        left_pixel,
                        right_pixel,
                        color: None,
                    },
                }),
                Err(reason) => mapping_failures.push(MappingFailure {
                    seed_index: m.seed_index,
                    rectified_match_index,
                    reason,
                }),
            }
        }
        let observations: Vec<_> = mapped_matches
            .iter()
            .map(|m| m.observations.clone())
            .collect();
        let reconstruction = if observations.is_empty() {
            None
        } else {
            Some(reconstruct(
                &self.calibration,
                request.source.clone(),
                &observations,
                request.triangulation,
                work,
            )?)
        };
        work.check()?;
        Ok(StereoReport {
            schema_version: 1,
            synchronization_verified: false,
            physical_accuracy_qualified: false,
            calibration: self.calibration.clone(),
            source: request.source,
            rectification: rectification.clone(),
            matching,
            seeds,
            automatic_sampling: sampling,
            mapped_matches,
            mapping_failures,
            reconstruction,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_calibration::{
        Brown5,
        stereo::{RigidPose, StereoCalibration},
        storage::{DeviceBinding, Provenance},
    };
    fn fixture(vertical: bool) -> (CalibrationRecord, CameraModel, Vec<u8>, Vec<u8>) {
        let grid = CameraModel {
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
        let mut camera = grid.clone();
        let pose = if vertical {
            camera.width = 32;
            camera.height = 64;
            camera.cx = 15.5;
            camera.cy = 31.5;
            RigidPose {
                rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                translation_mm: [0., -20., 0.],
            }
        } else {
            RigidPose {
                rotation: [[-1., 0., 0.], [0., -1., 0.], [0., 0., 1.]],
                translation_mm: [20., 0., 0.],
            }
        };
        let cal = CalibrationRecord {
            schema_version: 1,
            record_id: "synthetic-rotated-images".into(),
            units: "millimeters".into(),
            camera_axes: "x_right_y_down_z_forward".into(),
            provenance: Provenance::SyntheticFixture,
            binding: None,
            geometry: StereoCalibration {
                left: camera.clone(),
                right: camera,
                right_from_left: pose,
            },
            notes: "synthetic fixture only".into(),
        };
        let mut state = 0x12345678u32;
        let left: Vec<u8> = (0..2048)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                10 + (state % 91) as u8
            })
            .collect();
        let mut right = vec![0; 2048];
        for y in 0..32 {
            for x in 0..60 {
                right[y * 64 + x] = 2 * left[y * 64 + x + 4] + 20;
            }
        }
        if vertical {
            let rotate = |data: Vec<u8>| {
                let mut out = vec![0; 2048];
                for y in 0..32 {
                    for x in 0..64 {
                        out[x * 32 + 31 - y] = data[y * 64 + x];
                    }
                }
                out
            };
            (cal, grid, rotate(left), rotate(right))
        } else {
            right.reverse();
            (cal, grid, left, right)
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
        }
    }
    fn limits() -> TriangulationLimits {
        TriangulationLimits {
            min_depth_z_mm: 10.,
            max_depth_z_mm: 1000.,
            min_ray_angle_rad: 0.0001,
            max_ray_gap_mm: 0.01,
            max_reprojection_error_px: 0.01,
        }
    }
    #[test]
    fn rotated_camera_images_match_and_triangulate_with_original_calibration() {
        for vertical in [false, true] {
            let (cal, grid, left, right) = fixture(vertical);
            let width = cal.geometry.left.width;
            let height = cal.geometry.left.height;
            let processor =
                StereoProcessor::build(cal.clone(), grid, &WorkControl::default()).unwrap();
            let seeds = [[0, 0], [20, 10], [36, 20], [50, 24]];
            let report = processor
                .process(
                    StereoRequest {
                        left: GrayImage::new(width, height, &left).unwrap(),
                        right: GrayImage::new(width, height, &right).unwrap(),
                        source: MatchSource::SyntheticFixture,
                        seeds: &seeds,
                        matching: config(),
                        triangulation: limits(),
                    },
                    &WorkControl::default(),
                )
                .unwrap();
            assert_eq!(report.calibration, cal);
            assert_eq!(report.schema_version, 1);
            assert!(!report.synchronization_verified && !report.physical_accuracy_qualified);
            assert_eq!(report.matching.accepted.len(), 3);
            assert_eq!(report.matching.rejected[0].seed_index, 0);
            assert!(report.mapping_failures.is_empty());
            let reconstructed = report.reconstruction.unwrap();
            assert!(reconstructed.rejected.is_empty());
            for point in reconstructed.accepted {
                let mapped = &report.mapped_matches[point.source_index];
                let seed = seeds[mapped.seed_index];
                assert_eq!(mapped.seed_index, mapped.rectified_match_index + 1);
                let rect = [
                    (f64::from(seed[0]) - 31.5) * 5.,
                    (f64::from(seed[1]) - 15.5) * 5.,
                    400.,
                ];
                let expected = if vertical {
                    [-rect[1], rect[0], rect[2]]
                } else {
                    rect
                };
                for (actual, expected) in point.geometry.position_mm.into_iter().zip(expected) {
                    assert!((actual - expected).abs() < 1e-7);
                }
            }
        }
    }
    #[test]
    fn ambiguous_images_retain_rejections_without_inventing_vertices() {
        let (cal, grid, _, _) = fixture(false);
        let processor = StereoProcessor::build(cal, grid, &WorkControl::default()).unwrap();
        let pixels = vec![20; 2048];
        let image = GrayImage::new(64, 32, &pixels).unwrap();
        let report = processor
            .process(
                StereoRequest {
                    left: image,
                    right: image,
                    source: MatchSource::SyntheticFixture,
                    seeds: &[[30, 16]],
                    matching: config(),
                    triangulation: limits(),
                },
                &WorkControl::default(),
            )
            .unwrap();
        assert_eq!(report.matching.rejected.len(), 1);
        assert!(report.reconstruction.is_none());
        assert!(report.mapped_matches.is_empty());
    }
    #[test]
    fn pipeline_enforces_source_binding_dimensions_and_cancellation() {
        let (cal, grid, left, right) = fixture(false);
        let processor = StereoProcessor::build(cal, grid.clone(), &WorkControl::default()).unwrap();
        let left = GrayImage::new(64, 32, &left).unwrap();
        let right = GrayImage::new(64, 32, &right).unwrap();
        let request = || StereoRequest {
            left,
            right,
            source: MatchSource::SyntheticFixture,
            seeds: &[[30, 16]],
            matching: config(),
            triangulation: limits(),
        };
        let work = WorkControl::default();
        work.cancel();
        assert!(processor.process(request(), &work).is_err());
        let mut req = request();
        req.right = GrayImage::new(1, 1, &[0]).unwrap();
        assert!(processor.process(req, &WorkControl::default()).is_err());
        let mut req = request();
        req.source = MatchSource::Observations(DeviceBinding {
            serial: "test".into(),
            firmware: "test".into(),
            mode: "test".into(),
            left_size: [64, 32],
            right_size: [64, 32],
        });
        assert!(processor.process(req, &WorkControl::default()).is_err());
        assert!(StereoProcessor::build(fixture(false).0, grid, &work).is_err());
    }
}
