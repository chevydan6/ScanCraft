//! Offline numerical hypotheses, never calibrated geometry or a model export.
use scancraft_calibration::{
    CameraModel, factory_brown, factory_laser,
    laser_surface::{QuadraticSheet, RaySheetIntersections},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeSet, fs::OpenOptions, io::Read, path::Path};

const SHIFTS: [f64; 4] = [0., 200., 400., 600.];
const MAX_COMPARISONS: usize = 100_000_000;
#[derive(Deserialize)]
struct Profile {
    interface: u8,
    width: u32,
    height: u32,
}
#[derive(Deserialize)]
struct Observation {
    pixel: [f64; 2],
}
#[derive(Deserialize)]
struct Detections {
    schema_version: u32,
    sensor_pts: u64,
    profile: Profile,
    source_integrity: String,
    source_header_crc_protected: bool,
    accepted_observations: usize,
    observations: Vec<Observation>,
    payload_crc32: u32,
    metadata_crc32: u32,
}
fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800 | 0x80000); // O_NONBLOCK | O_CLOEXEC
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err("requires regular input files".into());
    }
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("diagnostic input exceeds size limit".into());
    }
    Ok(bytes)
}
fn detections(path: &Path, interface: u8) -> Result<Detections, Box<dyn std::error::Error>> {
    let d: Detections = serde_json::from_slice(&read_bounded(path, 8 * 1024 * 1024)?)?;
    if d.schema_version != 1
        || d.profile.interface != interface
        || (d.profile.width, d.profile.height) != (1920, 1200)
        || d.source_integrity != "all frames and footer validated"
        || !d.source_header_crc_protected
        || d.accepted_observations != d.observations.len()
        || d.observations.len() > 5000
        || d.observations.iter().any(|o| {
            !o.pixel.iter().all(|v| v.is_finite())
                || o.pixel[0] < 0.
                || o.pixel[0] >= 1920.
                || o.pixel[1] < 0.
                || o.pixel[1] >= 1200.
        })
    {
        return Err("unsupported or inconsistent stripe observation input".into());
    }
    Ok(d)
}
fn reflect_observations_x(d: &mut Detections) -> Result<(), String> {
    for o in &mut d.observations {
        o.pixel = scancraft_calibration::orientation::reflect_pixel_x(
            [d.profile.width, d.profile.height],
            o.pixel,
        )?;
    }
    Ok(())
}
fn camera(lens: &factory_laser::LaserLensFields) -> Result<CameraModel, String> {
    let [fx, fy, cx, cy] = lens.intrinsic_values.map(f64::from);
    let c = CameraModel {
        width: lens.width.into(),
        height: lens.height.into(),
        fx,
        fy,
        cx,
        cy,
        skew: 0.,
        distortion: factory_brown::brown5_if_supported(lens.coefficient_values)?,
        valid_radius: 0.8,
    };
    c.validate()?;
    Ok(c)
}
#[cfg(test)]
fn nearest(prediction: [f64; 2], observations: &[Observation], shift: f64) -> Option<usize> {
    let mut best = None;
    let mut distance = 9f64;
    for (i, o) in observations.iter().enumerate() {
        let dx = (o.pixel[0] + shift).rem_euclid(1920.) - prediction[0];
        let dy = o.pixel[1] - prediction[1];
        let squared = dx * dx + dy * dy;
        if squared <= distance {
            best = Some(i);
            distance = squared;
        }
    }
    best
}
struct RowIndex<'a> {
    observations: &'a [Observation],
    rows: Vec<Vec<usize>>,
}
impl<'a> RowIndex<'a> {
    fn new(observations: &'a [Observation]) -> Self {
        let mut rows = vec![Vec::new(); 1200];
        for (i, o) in observations.iter().enumerate() {
            rows[o.pixel[1] as usize].push(i);
        }
        Self { observations, rows }
    }
    fn nearest(
        &self,
        prediction: [f64; 2],
        shift: f64,
        budget: &mut usize,
    ) -> Result<Option<usize>, String> {
        let low = (prediction[1] - 3.).max(0.) as usize;
        let high = ((prediction[1] + 3.) as usize).min(1199);
        let mut best = None;
        let mut distance = 9f64;
        for row in &self.rows[low..=high] {
            for &i in row {
                *budget = budget.checked_add(1).ok_or("comparison budget overflow")?;
                if *budget > MAX_COMPARISONS {
                    return Err("comparison budget exceeded; no partial result".into());
                }
                let o = &self.observations[i];
                let dx = (o.pixel[0] + shift).rem_euclid(1920.) - prediction[0];
                let dy = o.pixel[1] - prediction[1];
                let squared = dx * dx + dy * dy;
                if squared < distance
                    || (squared == distance && best.is_none_or(|previous| i > previous))
                {
                    best = Some(i);
                    distance = squared;
                }
            }
        }
        Ok(best)
    }
}
#[derive(Default, Serialize)]
struct ShiftSupport {
    horizontal_shift_pixels: f64,
    supported_predictions: usize,
    supported_first_observations: usize,
    supported_second_observations: usize,
    first_observations_with_multiple_predictions: usize,
    greedy_one_to_one_pairs: usize,
}
fn distinct_pairs(edges: &mut [(f64, usize, usize)]) -> usize {
    edges.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut first = BTreeSet::new();
    let mut second = BTreeSet::new();
    let mut count = 0;
    for &(_, a, b) in edges.iter() {
        if !first.contains(&a) && !second.contains(&b) {
            first.insert(a);
            second.insert(b);
            count += 1;
        }
    }
    count
}
fn audit(
    record: &factory_laser::LaserRecordFields,
    left: &Detections,
    right: &Detections,
    budget: &mut usize,
) -> Result<serde_json::Value, String> {
    let cameras = [camera(&record.lenses[0])?, camera(&record.lenses[1])?];
    let prepared = [cameras[0].prepare()?, cameras[1].prepare()?];
    let pose = &record.transforms[0];
    let row_index = RowIndex::new(&right.observations);
    let mut counts: [ShiftSupport; 4] = std::array::from_fn(|i| ShiftSupport {
        horizontal_shift_pixels: SHIFTS[i],
        ..Default::default()
    });
    let mut left_counts = vec![[0usize; 4]; left.observations.len()];
    let mut right_indices: [BTreeSet<usize>; 4] = std::array::from_fn(|_| BTreeSet::new());
    let mut edges: [Vec<(f64, usize, usize)>; 4] = std::array::from_fn(|_| Vec::new());
    let mut per_sheet = Vec::new();
    let mut predicted = 0;
    let mut two_root_rays = 0;
    let mut ray_rejections = 0;
    let mut intersection_rejections = 0;
    let mut projection_rejections = 0;
    for (index, coefficients) in record.laser_coefficient_values.iter().enumerate() {
        if coefficients.iter().all(|v| *v == 0.) {
            continue;
        }
        if coefficients[4..].iter().any(|v| *v != 0.) {
            return Err("nonzero trailing coefficients cannot use four-field hypothesis".into());
        }
        let [a, b, c, d] = std::array::from_fn(|i| f64::from(coefficients[i]));
        let sheet = QuadraticSheet {
            quadratic_y: a,
            linear_y: b,
            linear_z: c,
            offset: d,
        };
        let mut sheet_support = [0usize; 4];
        let mut sheet_predictions = 0;
        for (left_index, observation) in left.observations.iter().enumerate() {
            let ray = match prepared[0].ray(observation.pixel) {
                Ok(r) => r,
                Err(_) => {
                    ray_rejections += 1;
                    continue;
                }
            };
            let roots = match sheet.intersect([ray[0] / ray[2], ray[1] / ray[2]], [100., 1000.]) {
                Ok(RaySheetIntersections::None) => continue,
                Ok(RaySheetIntersections::One(a)) => [Some(a), None],
                Ok(RaySheetIntersections::Two(a)) => {
                    two_root_rays += 1;
                    [Some(a[0]), Some(a[1])]
                }
                Err(_) => {
                    intersection_rejections += 1;
                    continue;
                }
            };
            for point in roots.into_iter().flatten() {
                let transformed = std::array::from_fn(|i| {
                    (0..3)
                        .map(|j| f64::from(pose.matrix_values[3 * i + j]) * point[j])
                        .sum::<f64>()
                        + f64::from(pose.translation_values[i])
                });
                let pixel = match prepared[1].project(transformed) {
                    Ok(p) if cameras[1].contains_pixel(p) => p,
                    _ => {
                        projection_rejections += 1;
                        continue;
                    }
                };
                predicted += 1;
                sheet_predictions += 1;
                for (s, shift) in SHIFTS.iter().enumerate() {
                    if let Some(right_index) = row_index.nearest(pixel, *shift, budget)? {
                        counts[s].supported_predictions += 1;
                        sheet_support[s] += 1;
                        left_counts[left_index][s] += 1;
                        right_indices[s].insert(right_index);
                        let p = right.observations[right_index].pixel;
                        let distance =
                            ((p[0] + shift).rem_euclid(1920.) - pixel[0]).hypot(p[1] - pixel[1]);
                        edges[s].push((distance, left_index, right_index));
                    }
                }
            }
        }
        per_sheet.push(
            json!({"factory_group_index":index,"predictions_in_second_camera_image":sheet_predictions,
            "supported_prediction_counts_by_shift":sheet_support}),
        );
    }
    for s in 0..4 {
        counts[s].supported_first_observations = left_counts.iter().filter(|c| c[s] > 0).count();
        counts[s].first_observations_with_multiple_predictions =
            left_counts.iter().filter(|c| c[s] > 1).count();
        counts[s].supported_second_observations = right_indices[s].len();
        counts[s].greedy_one_to_one_pairs = distinct_pairs(&mut edges[s]);
    }
    Ok(
        json!({"record_offset":record.record_offset,"label":record.label,
        "first_camera_input_interface":left.profile.interface,"second_camera_input_interface":right.profile.interface,
        "predictions_in_second_camera_image":predicted,"two_root_rays":two_root_rays,
        "ray_rejections":ray_rejections,"intersection_numeric_rejections":intersection_rejections,
        "projection_rejections":projection_rejections,"supports_by_shift":counts,"per_sheet":per_sheet}),
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 4 && !(args.len() == 5 && args[4] == "--mirror-x") {
        return Err(
            "usage: laser_sheet_hypotheses FACTORY_BIN LEFT_STRIPES_JSON RIGHT_STRIPES_JSON [--mirror-x]".into(),
        );
    }
    let records = factory_laser::inspect(&read_bounded(Path::new(&args[1]), 8192)?)?;
    let mut left = detections(Path::new(&args[2]), 0)?;
    let mut right = detections(Path::new(&args[3]), 2)?;
    if left.sensor_pts != right.sensor_pts {
        return Err("saved sensor PTS differ".into());
    }
    let mirror_x = args.len() == 5;
    if mirror_x {
        reflect_observations_x(&mut left)?;
        reflect_observations_x(&mut right)?;
    }
    let mut budget = 0;
    let mut reports = Vec::new();
    for record in records {
        if record.lenses[..2]
            .iter()
            .all(|l| (l.width, l.height) == (1920, 1200))
        {
            reports.push(audit(&record, &left, &right, &mut budget)?);
            reports.push(audit(&record, &right, &left, &mut budget)?);
        }
    }
    let mut report = json!({
        "diagnostic_only":true,"factory_envelope_crc_verified":true,
        "observation_json_integrity_claims_checked_not_recordings_reverified":true,
        "sensor_pts_equal":true,"exposure_synchronization_verified":false,
        "left_observations":left.observations.len(),"right_observations":right.observations.len(),
        "source_payload_crc32":[left.payload_crc32,right.payload_crc32],
        "source_metadata_crc32":[left.metadata_crc32,right.metadata_crc32],
        "hypotheses":{"surface":"X=aY^2+bY+cZ+d from first four floats, trailing six zero",
            "camera_slots":"first/second lens; stored row-major R and T",
            "brown_order":"k1,k2,k3,k4,k5,k6,p1,p2","numeric_radius":0.8,
            "depth_interval_raw_units":[100.,1000.],"support_radius_pixels":3.,
            "horizontal_shifts_pixels":SHIFTS},
        "greedy_pair_count":"distance-sorted, no observation reused; not maximum matching or verified correspondence",
        "physical_camera_assignment_verified":false,"mode_binding_verified":false,
        "projector_line_identity_verified":false,"translation_units_verified":false,
        "physical_accuracy_verified":false,"comparisons":budget,"comparison_limit":MAX_COMPARISONS,
        "records":reports
    });
    if mirror_x {
        report["hypotheses"]["image_orientation"] = json!(
            "both input observations reflected as width-1-x before geometry; source bytes/CRCs unchanged"
        );
        report["image_orientation_verified"] = json!(false);
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shifted_control_changes_support_and_wraps_at_image_edge() {
        let observations = vec![
            Observation {
                pixel: [1919., 100.],
            },
            Observation {
                pixel: [200., 200.],
            },
        ];
        assert_eq!(nearest([1918., 101.], &observations, 0.), Some(0));
        assert_eq!(nearest([1918., 101.], &observations, 200.), None);
        assert_eq!(nearest([199., 100.], &observations, 200.), Some(0));
        assert_eq!(nearest([200., 204.], &observations, 0.), None);
    }
    #[test]
    fn exact_row_index_matches_brute_force_including_ties_and_boundaries() {
        let mut observations = vec![
            Observation { pixel: [100., 0.] },
            Observation {
                pixel: [100., 1199.],
            },
            Observation {
                pixel: [100., 600.],
            },
            Observation {
                pixel: [100., 600.],
            },
        ];
        let mut seed = 9u32;
        for _ in 0..300 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let x = f64::from(seed % 192000) / 100.;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let y = f64::from(seed % 120000) / 100.;
            observations.push(Observation { pixel: [x, y] });
        }
        let index = RowIndex::new(&observations);
        for o in &observations {
            for shift in SHIFTS {
                for dy in [-3., 0., 3.] {
                    let pixel = [
                        (o.pixel[0] + shift).rem_euclid(1920.),
                        (o.pixel[1] + dy).clamp(0., 1199.),
                    ];
                    assert_eq!(
                        index.nearest(pixel, shift, &mut 0).unwrap(),
                        nearest(pixel, &observations, shift)
                    );
                }
            }
        }
    }
    #[test]
    fn repeated_observations_cannot_inflate_distinct_pair_count() {
        let mut edges = vec![(1., 0, 0), (2., 1, 0), (3., 2, 0), (0.5, 0, 0)];
        assert_eq!(distinct_pairs(&mut edges), 1);
        edges.push((0.1, 1, 1));
        assert_eq!(distinct_pairs(&mut edges), 2);
    }
    #[test]
    fn synthetic_prediction_supports_correct_order_and_refuses_budget_or_extra_coefficients() {
        let lens = || factory_laser::LaserLensFields {
            intrinsic_values: [800., 800., 960., 600.],
            width: 1920,
            height: 1200,
            coefficient_values: [0.; 8],
        };
        let transform = || factory_laser::LaserTransformFields {
            matrix_values: [1., 0., 0., 0., 1., 0., 0., 0., 1.],
            translation_values: [-20., 0., 0.],
        };
        let mut record = factory_laser::LaserRecordFields {
            label: "SYNTHETIC_TEST".into(),
            record_offset: 60,
            image_size_fields: [[1920, 1200], [1280, 800]],
            lenses: std::array::from_fn(|_| lens()),
            transforms: std::array::from_fn(|_| transform()),
            laser_coefficient_values: [[0.; 10]; 32],
            stored_essential: [[0.; 3]; 3],
            stored_fundamental: [[0.; 3]; 3],
        };
        record.laser_coefficient_values[0][3] = 100.;
        let input = |interface, pixel| Detections {
            schema_version: 1,
            sensor_pts: 1,
            profile: Profile {
                interface,
                width: 1920,
                height: 1200,
            },
            source_integrity: "all frames and footer validated".into(),
            source_header_crc_protected: true,
            accepted_observations: 1,
            observations: vec![Observation { pixel }],
            payload_crc32: 0,
            metadata_crc32: 0,
        };
        let a = input(0, [1160., 600.]);
        let b = input(2, [1120., 600.]);
        let report = audit(&record, &a, &b, &mut 0).unwrap();
        assert_eq!(report["supports_by_shift"][0]["supported_predictions"], 1);
        assert_eq!(report["supports_by_shift"][0]["greedy_one_to_one_pairs"], 1);
        assert_eq!(report["supports_by_shift"][1]["supported_predictions"], 0);
        let reversed = audit(&record, &b, &a, &mut 0).unwrap();
        assert_eq!(reversed["supports_by_shift"][0]["supported_predictions"], 0);
        let mut mirrored_a = input(0, [759., 600.]);
        let mut mirrored_b = input(2, [799., 600.]);
        reflect_observations_x(&mut mirrored_a).unwrap();
        reflect_observations_x(&mut mirrored_b).unwrap();
        let reflected = audit(&record, &mirrored_a, &mirrored_b, &mut 0).unwrap();
        assert_eq!(
            reflected["supports_by_shift"][0]["supported_predictions"],
            1
        );
        reflect_observations_x(&mut mirrored_a).unwrap();
        assert_eq!(mirrored_a.observations[0].pixel, [759., 600.]);
        let mut outside = input(0, [1919.5, 600.]);
        assert!(reflect_observations_x(&mut outside).is_err());
        let mut exhausted_budget = MAX_COMPARISONS;
        assert!(audit(&record, &a, &b, &mut exhausted_budget).is_err());
        record.laser_coefficient_values[0][4] = 1.;
        assert!(audit(&record, &a, &b, &mut 0).is_err());
    }
}
