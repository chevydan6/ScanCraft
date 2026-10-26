#[path = "support/subpixel_scene.rs"]
mod scene;
use scancraft_calibration::{
    stereo::TriangulationLimits,
    storage::{CalibrationRecord, Provenance},
};
use scancraft_depth::{
    MatchSource,
    matching::{MatchConfig, MatchedFeature, Rejection, match_rectified},
    pipeline::{StereoProcessor, StereoRequest},
};
use scancraft_geometry::work::WorkControl;
use scancraft_image::{GrayImage, MaskedImage};
fn image(values: Vec<f32>) -> MaskedImage {
    MaskedImage::new(96, 48, values, vec![true; 96 * 48]).unwrap()
}
#[test]
fn fractional_signed_pairs_improve_pixel_localization_without_changing_integer_winners() {
    let seeds = [[32, 18], [48, 24], [64, 30]];
    for disparity in [4.2, 4.25, 4.3, -4.25] {
        let (rig, l, r) = scene::fixture(disparity);
        let left = image(l);
        let right = image(r);
        let config = scene::config(disparity);
        let mut integer = config;
        integer.subpixel = None;
        let coarse = match_rectified(
            &rig,
            &left,
            &right,
            &seeds,
            integer,
            &WorkControl::default(),
        )
        .unwrap();
        let refined =
            match_rectified(&rig, &left, &right, &seeds, config, &WorkControl::default()).unwrap();
        assert_eq!(
            coarse.accepted.len(),
            seeds.len(),
            "{disparity}: {:?}",
            coarse.rejected
        );
        assert_eq!(
            refined.accepted.len(),
            seeds.len(),
            "{disparity}: {:?}",
            refined.rejected
        );
        for (c, r) in coarse.accepted.iter().zip(&refined.accepted) {
            assert_eq!(c.left_pixel, r.left_pixel);
            assert_eq!(c.right_pixel, r.right_pixel);
            let expected = f64::from(r.left_pixel[0]) - disparity;
            let error = (r.pixel_match().right_pixel[0] - expected).abs();
            assert!(error < 0.05, "{disparity}: pixel error {error}");
            assert!(error < (f64::from(c.right_pixel[0]) - expected).abs() * 0.25);
            assert!(r.subpixel.as_ref().unwrap().forward_offset_px.abs() <= 0.5);
        }
    }
}
#[test]
fn requested_refinement_rejects_search_boundary_invalid_support_and_reverse_inconsistency() {
    let (rig, l, r) = scene::fixture(4.25);
    let left = image(l);
    let right = image(r.clone());
    let seed = [[40, 24]];
    let mut config = scene::config(4.25);
    config.min_disparity = 4;
    let result =
        match_rectified(&rig, &left, &right, &seed, config, &WorkControl::default()).unwrap();
    assert_eq!(result.rejected[0].reason, Rejection::SubpixelUnavailable);
    config = scene::config(4.25);
    let mut mask = vec![true; 96 * 48];
    mask[24 * 96 + 28] = false;
    let right_masked = MaskedImage::new(96, 48, r, mask).unwrap();
    let mut integer = config;
    integer.subpixel = None;
    assert_eq!(
        match_rectified(
            &rig,
            &left,
            &right_masked,
            &seed,
            integer,
            &WorkControl::default()
        )
        .unwrap()
        .accepted
        .len(),
        1
    );
    assert_eq!(
        match_rectified(
            &rig,
            &left,
            &right_masked,
            &seed,
            config,
            &WorkControl::default()
        )
        .unwrap()
        .rejected[0]
            .reason,
        Rejection::SubpixelUnavailable
    );
    config.subpixel.as_mut().unwrap().max_bidirectional_error_px = 0.;
    let result = match_rectified(
        &rig,
        &left,
        &right,
        &[[32, 18], [48, 24], [64, 30]],
        config,
        &WorkControl::default(),
    )
    .unwrap();
    assert!(
        result
            .rejected
            .iter()
            .any(|r| r.reason == Rejection::SubpixelReverseMismatch)
    );
    config.min_correlation = 1.;
    assert_eq!(
        match_rectified(&rig, &left, &right, &seed, config, &WorkControl::default())
            .unwrap()
            .rejected[0]
            .reason,
        Rejection::LowCorrelation
    );
}
#[test]
fn legacy_json_defaults_and_invalid_subpixel_requests_preserve_contracts() {
    let (rig, l, r) = scene::fixture(4.25);
    let left = image(l);
    let right = image(r);
    let mut value = serde_json::to_value(scene::config(4.25)).unwrap();
    value.as_object_mut().unwrap().remove("subpixel");
    let legacy: MatchConfig = serde_json::from_value(value.clone()).unwrap();
    assert!(legacy.subpixel.is_none());
    assert_eq!(serde_json::to_value(legacy).unwrap(), value);
    let result = match_rectified(
        &rig,
        &left,
        &right,
        &[[40, 24]],
        legacy,
        &WorkControl::default(),
    )
    .unwrap();
    let value = serde_json::to_value(&result.accepted[0]).unwrap();
    assert!(value.get("subpixel").is_none());
    let restored: MatchedFeature = serde_json::from_value(value).unwrap();
    assert!(restored.subpixel.is_none());
    assert_eq!(
        restored.pixel_match().right_pixel,
        restored.right_pixel.map(f64::from)
    );
    for (curvature, error) in [
        (0., 0.1),
        (f64::NAN, 0.1),
        (4.1, 0.1),
        (0.1, -1.),
        (0.1, f64::INFINITY),
    ] {
        let mut config = scene::config(4.25);
        let s = config.subpixel.as_mut().unwrap();
        s.min_curvature = curvature;
        s.max_bidirectional_error_px = error;
        assert!(
            match_rectified(
                &rig,
                &left,
                &right,
                &[[40, 24]],
                config,
                &WorkControl::default()
            )
            .is_err()
        );
    }
    let work = WorkControl::default();
    work.cancel();
    assert!(match_rectified(&rig, &left, &right, &[[40, 24]], scene::config(4.25), &work).is_err());
}
#[test]
fn offline_pipeline_triangulates_refined_observations_and_retains_unqualified_flags() {
    let (rig, l, r) = scene::fixture(4.25);
    let grid = rig.left.clone();
    let left: Vec<_> = l.iter().map(|v| v.round() as u8).collect();
    let right: Vec<_> = r.iter().map(|v| v.round() as u8).collect();
    let record = CalibrationRecord {
        schema_version: 1,
        record_id: "synthetic-subpixel-plane".into(),
        units: "millimeters".into(),
        camera_axes: "x_right_y_down_z_forward".into(),
        provenance: Provenance::SyntheticFixture,
        binding: None,
        geometry: rig,
        notes: "synthetic only".into(),
    };
    let processor = StereoProcessor::build(record, grid, &WorkControl::default()).unwrap();
    let config = scene::config(4.25);
    let mut errors = Vec::new();
    for matching in [
        MatchConfig {
            subpixel: None,
            ..config
        },
        config,
    ] {
        let report = processor
            .process(
                StereoRequest {
                    left: GrayImage::new(96, 48, &left).unwrap(),
                    right: GrayImage::new(96, 48, &right).unwrap(),
                    source: MatchSource::SyntheticFixture,
                    seeds: &[[32, 18], [48, 24], [64, 30]],
                    matching,
                    triangulation: TriangulationLimits {
                        min_depth_z_mm: 10.,
                        max_depth_z_mm: 1000.,
                        min_ray_angle_rad: 0.0001,
                        max_ray_gap_mm: 0.01,
                        max_reprojection_error_px: 0.01,
                    },
                },
                &WorkControl::default(),
            )
            .unwrap();
        assert!(!report.synchronization_verified);
        assert!(!report.physical_accuracy_qualified);
        assert!(report.mapping_failures.is_empty());
        let reconstruction = report.reconstruction.as_ref().unwrap();
        assert_eq!(reconstruction.accepted.len(), 3);
        for (m, mapped) in report.matching.accepted.iter().zip(&report.mapped_matches) {
            assert!(
                (m.pixel_match().right_pixel[0] - mapped.observations.right_pixel[0]).abs() < 1e-9
            );
        }
        errors.push(
            reconstruction
                .accepted
                .iter()
                .map(|p| (p.geometry.position_mm[2] - 2400. / 4.25).abs())
                .sum::<f64>()
                / 3.,
        );
    }
    assert!(
        errors[1] < errors[0] * 0.2,
        "integer/refined mean depth errors: {errors:?}"
    );
}
