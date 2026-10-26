//! Calibrated sparse correspondence reconstruction. Matching and exposure synchronization
//! are upstream requirements; accepting geometry does not establish physical accuracy.
pub mod matching;
pub mod pipeline;
use scancraft_calibration::{
    stereo::{TriangulatedPoint, TriangulationLimits},
    storage::{CalibrationRecord, DeviceBinding, Provenance},
};
use scancraft_geometry::{
    Cloud, Point,
    work::{Stage, WorkControl},
};
use serde::{Deserialize, Serialize};

pub const MAX_MATCHES: usize = 1_000_000;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PixelMatch {
    pub left_pixel: [f64; 2],
    pub right_pixel: [f64; 2],
    pub color: Option<[u8; 3]>,
}
/// Declared origin, not an attestation that observations or exposure timing were verified.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchSource {
    SyntheticFixture,
    Observations(DeviceBinding),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptedPoint {
    pub source_index: usize,
    pub geometry: TriangulatedPoint,
    pub color: Option<[u8; 3]>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedMatch {
    pub source_index: usize,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reconstruction {
    pub calibration_record_id: String,
    pub calibration_provenance: Provenance,
    pub source: MatchSource,
    pub units: String,
    pub camera_axes: String,
    pub limits: TriangulationLimits,
    pub accepted: Vec<AcceptedPoint>,
    pub rejected: Vec<RejectedMatch>,
}
impl Reconstruction {
    /// Positions are in the left camera's +X right, +Y down, +Z forward frame.
    pub fn cloud(&self) -> Cloud {
        Cloud {
            points: self
                .accepted
                .iter()
                .map(|p| Point {
                    position: p.geometry.position_mm,
                    color: p.color,
                    normal: None,
                })
                .collect(),
        }
    }
}
pub(crate) fn validate_source(
    calibration: &CalibrationRecord,
    source: &MatchSource,
) -> Result<(), String> {
    calibration.validate()?;
    match source {
        MatchSource::SyntheticFixture if calibration.provenance != Provenance::SyntheticFixture => {
            return Err("synthetic matches require an explicitly synthetic calibration".into());
        }
        MatchSource::Observations(binding) => calibration.validate_for_capture(binding)?,
        _ => {}
    }
    Ok(())
}
/// Returns no partial result on cancellation or invalid configuration. Individual
/// invalid correspondences are reported separately and never become cloud vertices.
pub fn reconstruct(
    calibration: &CalibrationRecord,
    source: MatchSource,
    matches: &[PixelMatch],
    limits: TriangulationLimits,
    work: &WorkControl,
) -> Result<Reconstruction, String> {
    if matches.is_empty() || matches.len() > MAX_MATCHES {
        return Err("correspondence count must be 1..=1000000".into());
    }
    validate_source(calibration, &source)?;
    let prepared = calibration.geometry.prepare(limits)?;
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for (source_index, m) in matches.iter().enumerate() {
        // Check every point: inverse distortion can be substantially more expensive than
        // a geometry edit, and cancellation must remain responsive with bad matches too.
        work.update(Stage::Triangulating, source_index, matches.len())?;
        match prepared.triangulate(m.left_pixel, m.right_pixel) {
            Ok(geometry) => accepted.push(AcceptedPoint {
                source_index,
                geometry,
                color: m.color,
            }),
            Err(reason) => rejected.push(RejectedMatch {
                source_index,
                reason,
            }),
        }
    }
    work.update(Stage::Triangulating, matches.len(), matches.len())?;
    Ok(Reconstruction {
        calibration_record_id: calibration.record_id.clone(),
        calibration_provenance: calibration.provenance.clone(),
        source,
        units: "millimeters".into(),
        camera_axes: calibration.camera_axes.clone(),
        limits,
        accepted,
        rejected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_calibration::{
        Brown5, CameraModel,
        stereo::{RigidPose, StereoCalibration},
    };
    fn fixture() -> (CalibrationRecord, TriangulationLimits) {
        let camera = CameraModel {
            width: 1280,
            height: 800,
            fx: 1000.,
            fy: 1000.,
            cx: 639.5,
            cy: 399.5,
            skew: 0.,
            distortion: Brown5::ZERO,
            valid_radius: 1.5,
        };
        (
            CalibrationRecord {
                schema_version: 1,
                record_id: "analytic-synthetic".into(),
                units: "millimeters".into(),
                camera_axes: "x_right_y_down_z_forward".into(),
                provenance: Provenance::SyntheticFixture,
                binding: None,
                geometry: StereoCalibration {
                    left: camera.clone(),
                    right: camera,
                    right_from_left: RigidPose {
                        rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                        translation_mm: [-120., 0., 0.],
                    },
                },
                notes: "test only".into(),
            },
            TriangulationLimits {
                min_depth_z_mm: 10.,
                max_depth_z_mm: 2000.,
                min_ray_angle_rad: 0.0001,
                max_ray_gap_mm: 0.1,
                max_reprojection_error_px: 0.1,
            },
        )
    }
    fn good() -> PixelMatch {
        PixelMatch {
            left_pixel: [639.5, 399.5],
            right_pixel: [399.5, 399.5],
            color: Some([12, 34, 56]),
        }
    }
    #[test]
    fn mixed_matches_preserve_indices_quality_colors_and_export_roundtrip() {
        let (cal, limits) = fixture();
        let mut parallel = good();
        parallel.right_pixel = parallel.left_pixel;
        let mut invalid = good();
        invalid.left_pixel[0] = f64::NAN;
        let matches = [parallel, good(), invalid, good()];
        let result = reconstruct(
            &cal,
            MatchSource::SyntheticFixture,
            &matches,
            limits,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(
            result
                .accepted
                .iter()
                .map(|p| p.source_index)
                .collect::<Vec<_>>(),
            [1, 3]
        );
        assert_eq!(
            result
                .rejected
                .iter()
                .map(|p| p.source_index)
                .collect::<Vec<_>>(),
            [0, 2]
        );
        for p in &result.accepted {
            assert!((p.geometry.position_mm[2] - 500.).abs() < 1e-8);
            assert!(p.geometry.position_mm[0].abs() < 1e-8);
            assert!(p.geometry.position_mm[1].abs() < 1e-8);
            assert!(p.geometry.ray_gap_mm < 1e-8);
        }
        let cloud = result.cloud();
        cloud.validate().unwrap();
        assert!(cloud.points.iter().all(|p| p.color == Some([12, 34, 56])));
        let mut bytes = Vec::new();
        scancraft_geometry::write_ply(&cloud, &mut bytes).unwrap();
        assert_eq!(
            scancraft_geometry::formats::read_ply(std::io::Cursor::new(bytes), None).unwrap(),
            cloud
        );
    }
    #[test]
    fn source_binding_mismatch_and_synthetic_observation_are_errors() {
        let (mut cal, limits) = fixture();
        let binding = DeviceBinding {
            serial: "fixture".into(),
            firmware: "test".into(),
            mode: "test".into(),
            left_size: [1280, 800],
            right_size: [1280, 800],
        };
        cal.binding = Some(binding.clone());
        assert!(
            reconstruct(
                &cal,
                MatchSource::Observations(binding.clone()),
                &[good()],
                limits,
                &WorkControl::default()
            )
            .is_err()
        );
        cal.provenance = Provenance::Imported;
        assert!(
            reconstruct(
                &cal,
                MatchSource::SyntheticFixture,
                &[good()],
                limits,
                &WorkControl::default()
            )
            .is_err()
        );
        let mut other = binding.clone();
        other.mode = "other".into();
        assert!(
            reconstruct(
                &cal,
                MatchSource::Observations(other),
                &[good()],
                limits,
                &WorkControl::default()
            )
            .is_err()
        );
        assert!(
            reconstruct(
                &cal,
                MatchSource::Observations(binding),
                &[good()],
                limits,
                &WorkControl::default()
            )
            .is_ok()
        );
    }
    #[test]
    fn cancellation_and_empty_input_never_return_partial_clouds() {
        let (cal, limits) = fixture();
        let work = WorkControl::default();
        work.cancel();
        assert!(
            reconstruct(
                &cal,
                MatchSource::SyntheticFixture,
                &[good()],
                limits,
                &work
            )
            .unwrap_err()
            .contains("cancelled")
        );
        assert!(
            reconstruct(
                &cal,
                MatchSource::SyntheticFixture,
                &[],
                limits,
                &WorkControl::default()
            )
            .is_err()
        );
    }
    #[test]
    fn all_invalid_is_an_explicit_empty_result_with_all_reasons() {
        let (cal, limits) = fixture();
        let mut m = good();
        m.right_pixel = m.left_pixel;
        let result = reconstruct(
            &cal,
            MatchSource::SyntheticFixture,
            &[m],
            limits,
            &WorkControl::default(),
        )
        .unwrap();
        assert!(result.accepted.is_empty());
        assert!(result.cloud().points.is_empty());
        assert_eq!(result.rejected.len(), 1);
        assert!(!result.rejected[0].reason.is_empty());
    }
}
