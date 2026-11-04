//! Existing component/conic/stereo methods, with complete Cartesian diagnostics.
use super::{Error, MarkerConfig, cloud, line};
use scancraft_calibration::stereo::{StereoCalibration, TriangulationLimits};
use scancraft_geometry::{Cloud, work::WorkControl};
use scancraft_image::{GrayImage, components, shape};
use scancraft_recording::selection::SelectedFrame;
use serde_json::json;
use std::io::Write;

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

pub(super) fn reconstruct(
    frames: &[SelectedFrame; 2],
    rig: &StereoCalibration,
    config: &MarkerConfig,
    budget: &mut usize,
    out: &mut impl Write,
    work: &WorkControl,
) -> Result<Cloud, Error> {
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
    let mut left_counts = vec![0; left.len()];
    let mut right_counts = vec![0; right.len()];
    for (i, l) in left.iter().enumerate() {
        for (j, r) in right.iter().enumerate() {
            work.check()?;
            let result = stereo.triangulate(*l, *r);
            let report = match result {
                Ok(p) => {
                    left_counts[i] += 1;
                    right_counts[j] += 1;
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
    let mut points = Vec::new();
    for (i, j, position) in accepted {
        let unique = left_counts[i] == 1 && right_counts[j] == 1;
        line(
            out,
            &json!({"kind":"uniqueness","first":i,"second":j,"mutual_unique":unique}),
        )?;
        if unique {
            points.push(position);
        }
    }
    line(
        out,
        &json!({"kind":"tracking_observations","positions_model_units":points,
        "stereo_attempts":attempts,"marker_identity_verified":false}),
    )?;
    Ok(cloud(points))
}
