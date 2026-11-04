//! Existing component/conic/stereo methods, with complete Cartesian diagnostics.
use super::{Error, MarkerConfig, cloud, line};
use scancraft_calibration::stereo::{StereoCalibration, TriangulationLimits};
use scancraft_geometry::{Cloud, spatial::SpatialIndex, work::WorkControl};
use scancraft_image::{GrayImage, components, shape};
use scancraft_recording::selection::SelectedFrame;
use serde_json::json;
use std::io::Write;

/// Raw first-camera coordinates from the last accepted tracking update.
/// No world transform, extrapolation or claim of physical marker identity.
pub(super) struct Prior {
    pub frame_id: u64,
    pub positions: Vec<[f64; 3]>,
}

type Candidate = (usize, usize, [f64; 3]);

pub(super) struct Prepared {
    accepted: Vec<Candidate>,
    dimensions: [usize; 2],
    pub attempts: usize,
}

impl Prepared {
    pub fn select(
        &self,
        prior: Option<(scancraft_geometry::session::recovery::Reference<'_>, f64)>,
        out: &mut impl Write,
        work: &WorkControl,
    ) -> Result<Vec<[f64; 3]>, Error> {
        select(&self.accepted, self.dimensions, prior, out, work)
    }
}

fn select(
    accepted: &[Candidate],
    dimensions: [usize; 2],
    prior: Option<(scancraft_geometry::session::recovery::Reference<'_>, f64)>,
    out: &mut impl Write,
    work: &WorkControl,
) -> Result<Vec<[f64; 3]>, Error> {
    let index = prior
        .map(|(p, _)| SpatialIndex::build(p.positions, work))
        .transpose()?;
    let mut counts = [vec![0; dimensions[0]], vec![0; dimensions[1]]];
    let mut eligible = Vec::with_capacity(accepted.len());
    if let Some((p, distance)) = prior {
        if !(3..=512).contains(&p.positions.len()) || !distance.is_finite() || distance <= 0. {
            return Err("invalid temporal marker support reference".into());
        }
        line(
            out,
            &json!({"kind":"temporal_reference","frame_id":p.frame_id,
            "points":p.positions.len(),"maximum_distance_model_units":distance,
            "motion_prediction_used":false,"marker_identity_verified":false}),
        )?;
    }
    for &(i, j, position) in accepted {
        work.check()?;
        let supported = if let (Some(index), Some((_, limit))) = (&index, prior) {
            let neighbor = index.nearest(position, work)?;
            let supported = neighbor.is_some_and(|n| n.distance_mm <= limit);
            line(
                out,
                &json!({"kind":"temporal_support","first":i,"second":j,
                "supported":supported,"nearest_prior_index":neighbor.map(|n|n.index),
                "nearest_prior_distance_model_units":neighbor.map(|n|n.distance_mm)}),
            )?;
            supported
        } else {
            true
        };
        if supported {
            counts[0][i] += 1;
            counts[1][j] += 1;
        }
        eligible.push((i, j, position, supported));
    }
    let mut points = Vec::new();
    for (i, j, position, supported) in eligible {
        work.check()?;
        let unique = supported && counts[0][i] == 1 && counts[1][j] == 1;
        line(
            out,
            &json!({"kind":"uniqueness","first":i,"second":j,"mutual_unique":unique}),
        )?;
        if unique {
            points.push(position);
        }
    }
    Ok(points)
}

fn observations(
    frame: &SelectedFrame,
    config: &MarkerConfig,
    work: &WorkControl,
) -> Result<(Vec<[f64; 2]>, serde_json::Value), Error> {
    let image = GrayImage::from_frame(&frame.profile, &frame.frame)?;
    let found = components::detect(
        image,
        components::Config {
            threshold: config.threshold_exclusive,
            minimum_area: config.minimum_area,
            maximum_area: config.maximum_area,
            maximum_components: config.maximum_components,
        },
        work,
    )?;
    if found.observations.len() > 512 {
        return Err(
            "marker shape processing exceeds 512 observations; no truncation performed".into(),
        );
    }
    let mut pixels = Vec::new();
    let mut rows = Vec::new();
    for (index, observation) in found.observations.iter().enumerate() {
        work.check()?;
        let fit = shape::measure(image, config.threshold_exclusive, observation, work);
        work.check()?;
        let accepted = fit
            .as_ref()
            .is_ok_and(|f| f.rms_sampson <= config.maximum_shape_rms_px);
        let selected_index = accepted.then_some(pixels.len());
        if accepted {
            pixels.push(observation.pixel);
        }
        rows.push(json!({"index":index,"selected_index":selected_index,
            "pixel":observation.pixel,"area":observation.area,"bounds_inclusive":observation.bounds,
            "peak":observation.peak,"saturated_pixels":observation.saturated_pixels,
            "shape":fit,"shape_accepted":accepted}));
    }
    Ok((
        pixels,
        json!({"interface":frame.profile.interface,
        "components_examined":found.components_examined,"foreground_pixels":found.foreground_pixels,
        "rejected_area":found.rejected_area,"rejected_border":found.rejected_border,
        "observations":rows,"physical_center_verified":false}),
    ))
}

pub(super) fn prepare(
    frames: &[SelectedFrame; 2],
    rig: &StereoCalibration,
    config: &MarkerConfig,
    budget: &mut usize,
    out: &mut impl Write,
    work: &WorkControl,
) -> Result<Prepared, Error> {
    let (left, left_report) = observations(&frames[0], config, work)?;
    let (right, right_report) = observations(&frames[1], config, work)?;
    line(
        out,
        &json!({"kind":"components","first":left_report,"second":right_report}),
    )?;
    let attempts = left.len() * right.len();
    *budget = budget
        .checked_sub(attempts)
        .ok_or("sequence stereo attempt budget exceeded")?;
    let stereo = rig.prepare(TriangulationLimits {
        min_depth_z_mm: config.minimum_depth_model_units,
        max_depth_z_mm: config.maximum_depth_model_units,
        min_ray_angle_rad: config.minimum_ray_angle_rad,
        max_ray_gap_mm: config.maximum_ray_gap_model_units,
        max_reprojection_error_px: config.maximum_reprojection_error_px,
    })?;
    let mut accepted = Vec::new();
    for (i, l) in left.iter().enumerate() {
        for (j, r) in right.iter().enumerate() {
            work.check()?;
            let result = stereo.triangulate(*l, *r);
            let report = match result {
                Ok(p) => {
                    accepted.push((i, j, p.position_mm));
                    json!({"position_model_units":p.position_mm,"ray_gap_model_units":p.ray_gap_mm,
                        "ray_angle_rad":p.ray_angle_rad,"first_reprojection_px":p.left_reprojection_px,
                        "second_reprojection_px":p.right_reprojection_px})
                }
                Err(error) => json!({"error":error}),
            };
            line(
                out,
                &json!({"kind":"stereo_pair","first":i,"second":j,"result":report}),
            )?;
        }
    }
    Ok(Prepared {
        accepted,
        dimensions: [left.len(), right.len()],
        attempts,
    })
}

pub(super) fn reconstruct(
    frames: &[SelectedFrame; 2],
    rig: &StereoCalibration,
    config: &MarkerConfig,
    prior: Option<(&Prior, f64)>,
    budget: &mut usize,
    out: &mut impl Write,
    work: &WorkControl,
) -> Result<Cloud, Error> {
    let prepared = prepare(frames, rig, config, budget, out, work)?;
    let points = prepared.select(
        prior.map(|(p, d)| {
            (
                scancraft_geometry::session::recovery::Reference {
                    frame_id: p.frame_id,
                    positions: &p.positions,
                },
                d,
            )
        }),
        out,
        work,
    )?;
    line(
        out,
        &json!({"kind":"tracking_observations","positions_model_units":points,
        "stereo_attempts":prepared.attempts,"marker_identity_verified":false}),
    )?;
    Ok(cloud(points))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    #[test]
    fn prior_support_retains_distances_ambiguity_original_points_and_cancellation() {
        let prior = Prior {
            frame_id: 7,
            positions: vec![[0., 0., 0.], [0., 10., 0.], [10., 0., 0.]],
        };
        let candidates = vec![
            (0, 0, [0., 0., 0.]),
            (0, 1, [0., 0., 100.]),
            (1, 2, [0., 10., 0.]),
            (2, 3, [10., 0., 0.]),
        ];
        let work = WorkControl::default();
        let baseline = select(&candidates, [3, 4], None, &mut Vec::new(), &work).unwrap();
        assert_eq!(baseline, vec![[0., 10., 0.], [10., 0., 0.]]);
        let mut bytes = Vec::new();
        let points = select(
            &candidates,
            [3, 4],
            Some((
                scancraft_geometry::session::recovery::Reference {
                    frame_id: prior.frame_id,
                    positions: &prior.positions,
                },
                3.,
            )),
            &mut bytes,
            &work,
        )
        .unwrap();
        assert_eq!(points, prior.positions);
        let rows: Vec<Value> = String::from_utf8(bytes)
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(rows[0]["frame_id"], 7);
        assert_eq!(rows[2]["supported"], false);
        assert_eq!(rows[2]["nearest_prior_distance_model_units"], 100.);
        let ambiguous = vec![(0, 0, [0., 0., 0.]), (0, 1, [1., 0., 0.])];
        assert!(
            select(
                &ambiguous,
                [1, 2],
                Some((
                    scancraft_geometry::session::recovery::Reference {
                        frame_id: prior.frame_id,
                        positions: &prior.positions
                    },
                    3.
                )),
                &mut Vec::new(),
                &work
            )
            .unwrap()
            .is_empty()
        );
        work.cancel();
        assert!(
            select(
                &candidates,
                [3, 4],
                Some((
                    scancraft_geometry::session::recovery::Reference {
                        frame_id: prior.frame_id,
                        positions: &prior.positions
                    },
                    3.
                )),
                &mut Vec::new(),
                &work
            )
            .is_err()
        );
    }
}
