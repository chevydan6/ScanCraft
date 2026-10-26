//! Observed pose-field mathematics, separate from calibrated camera geometry.
//! The tested depth-engine prefix uses radians and Rz(RZ)*Rx(RX)*Ry(RY).
//! Translation stays in raw source units. Neither transform direction nor
//! physical translation units are established; this must not create StereoRig.

/// Row-major homogeneous matrix for column vectors, using the observed
/// numerical prefix's rotation convention and unscaled translation fields.
pub fn matrix_raw_units(
    rotation_fields: [f32; 3],
    translation_fields: [f32; 3],
) -> Result<[[f64; 4]; 4], String> {
    if !rotation_fields
        .iter()
        .chain(&translation_fields)
        .all(|v| v.is_finite())
    {
        return Err("nonfinite factory pose field".into());
    }
    let [x, y, z] = rotation_fields.map(f64::from);
    let (sx, cx) = x.sin_cos();
    let (sy, cy) = y.sin_cos();
    let (sz, cz) = z.sin_cos();
    let rx = nalgebra::Matrix3::new(1., 0., 0., 0., cx, -sx, 0., sx, cx);
    let ry = nalgebra::Matrix3::new(cy, 0., sy, 0., 1., 0., -sy, 0., cy);
    let rz = nalgebra::Matrix3::new(cz, -sz, 0., sz, cz, 0., 0., 0., 1.);
    let rotation = rz * rx * ry;
    Ok(std::array::from_fn(|row| {
        std::array::from_fn(|col| match (row, col) {
            (0..=2, 0..=2) => rotation[(row, col)],
            (0..=2, 3) => f64::from(translation_fields[row]),
            (3, 3) => 1.,
            _ => 0.,
        })
    }))
}

/// Observed ordered-pair composition: T(second) * inverse(T(first)).
/// Uses unscaled source translation units. The physical camera/world meaning
/// and capture-mode applicability still require independent verification.
pub fn relative_matrix_raw_units(
    first_rotation: [f32; 3],
    first_translation: [f32; 3],
    second_rotation: [f32; 3],
    second_translation: [f32; 3],
) -> Result<[[f64; 4]; 4], String> {
    let first = matrix_raw_units(first_rotation, first_translation)?;
    let second = matrix_raw_units(second_rotation, second_translation)?;
    let rotation: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| (0..3).map(|k| second[i][k] * first[j][k]).sum())
    });
    let translation: [f64; 3] = std::array::from_fn(|i| {
        second[i][3] - (0..3).map(|j| rotation[i][j] * first[j][3]).sum::<f64>()
    });
    Ok(std::array::from_fn(|i| {
        std::array::from_fn(|j| match (i, j) {
            (0..=2, 0..=2) => rotation[i][j],
            (0..=2, 3) => translation[i],
            (3, 3) => 1.,
            _ => 0.,
        })
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct Fixture {
        synthetic_only: bool,
        cases: Vec<Case>,
    }
    #[derive(Deserialize)]
    struct Case {
        rotation_fields: [f32; 3],
        translation_fields: [f32; 3],
        matrix_row_major: [[f64; 4]; 4],
    }
    #[test]
    fn matches_isolated_numerical_prefix_for_identity_axes_mixed_angles_and_translation() {
        let fixture: Fixture = serde_json::from_str(include_str!(
            "../../../evidence/factory-pose-prefix-cases.json"
        ))
        .unwrap();
        assert!(fixture.synthetic_only);
        assert_eq!(fixture.cases.len(), 6);
        for case in fixture.cases {
            let actual = matrix_raw_units(case.rotation_fields, case.translation_fields).unwrap();
            for (a, b) in actual
                .iter()
                .flatten()
                .zip(case.matrix_row_major.iter().flatten())
            {
                assert!((a - b).abs() < 1e-12, "{a} differs from reference {b}");
            }
        }
    }
    #[derive(Deserialize)]
    struct RelativeFixture {
        synthetic_only: bool,
        cases: Vec<RelativeCase>,
    }
    #[derive(Deserialize)]
    struct RelativeCase {
        first_rotation: [f32; 3],
        first_translation: [f32; 3],
        second_rotation: [f32; 3],
        second_translation: [f32; 3],
        relative_matrix: [[f64; 4]; 4],
    }
    #[test]
    fn ordered_relative_composition_matches_isolated_reference() {
        let fixture: RelativeFixture = serde_json::from_str(include_str!(
            "../../../evidence/factory-relative-pose-cases.json"
        ))
        .unwrap();
        assert!(fixture.synthetic_only);
        assert_eq!(fixture.cases.len(), 6);
        for case in fixture.cases {
            let actual = relative_matrix_raw_units(
                case.first_rotation,
                case.first_translation,
                case.second_rotation,
                case.second_translation,
            )
            .unwrap();
            for (a, b) in actual
                .iter()
                .flatten()
                .zip(case.relative_matrix.iter().flatten())
            {
                assert!((a - b).abs() < 1e-11, "{a} differs from reference {b}");
            }
        }
    }
    #[test]
    fn relative_composition_rejects_nonfinite_fields_in_either_pose() {
        for i in 0..12 {
            let mut values = [0f32; 12];
            values[i] = f32::NAN;
            assert!(
                relative_matrix_raw_units(
                    values[..3].try_into().unwrap(),
                    values[3..6].try_into().unwrap(),
                    values[6..9].try_into().unwrap(),
                    values[9..].try_into().unwrap()
                )
                .is_err()
            );
        }
        let same = relative_matrix_raw_units(
            [0.2, -0.3, 0.4],
            [10., -20., 30.],
            [0.2, -0.3, 0.4],
            [10., -20., 30.],
        )
        .unwrap();
        for (i, row) in same.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                assert!((*value - if i == j { 1. } else { 0. }).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn rejects_nonfinite_inputs_without_assigning_physical_units() {
        for i in 0..6 {
            for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut r = [0.; 3];
                let mut t = [0.; 3];
                if i < 3 {
                    r[i] = invalid;
                } else {
                    t[i - 3] = invalid;
                }
                assert!(matrix_raw_units(r, t).is_err());
            }
        }
        let m = matrix_raw_units([0.; 3], [10., -20., 30.]).unwrap();
        assert_eq!([m[0][3], m[1][3], m[2][3]], [10., -20., 30.]);
    }
}

#[cfg(test)]
mod rectification_reference_tests {
    use super::*;
    #[derive(serde::Deserialize)]
    struct Fixture {
        synthetic_only: bool,
        cases: Vec<Case>,
    }
    #[derive(serde::Deserialize)]
    struct Case {
        first_rotation: [f32; 3],
        first_translation: [f32; 3],
        second_rotation: [f32; 3],
        second_translation: [f32; 3],
        reference_rotations_finite: bool,
        first_rectification: Option<[[f64; 3]; 3]>,
        second_rectification: Option<[[f64; 3]; 3]>,
    }
    #[test]
    fn ordered_relative_transform_satisfies_observed_rectification_relations() {
        let fixture: Fixture = serde_json::from_str(include_str!(
            "../tests/fixtures/factory-rectification-prefix.json"
        ))
        .unwrap();
        assert!(fixture.synthetic_only);
        assert_eq!(fixture.cases.len(), 6);
        for (index, case) in fixture.cases.into_iter().enumerate() {
            let t = relative_matrix_raw_units(
                case.first_rotation,
                case.first_translation,
                case.second_rotation,
                case.second_translation,
            )
            .unwrap();
            assert!(t.iter().flatten().all(|x| x.is_finite()));
            if !case.reference_rotations_finite {
                assert_eq!(index, 0);
                assert!(case.first_rectification.is_none());
                continue; // The reference's identity-rotation degeneracy is not native behavior.
            }
            let first = case.first_rectification.unwrap();
            let second = case.second_rectification.unwrap();
            for i in 0..3 {
                for j in 0..3 {
                    let value = (0..3).map(|k| second[i][k] * t[k][j]).sum::<f64>();
                    assert!((value - first[i][j]).abs() < 1e-12);
                }
            }
            let shifted: [f64; 3] =
                std::array::from_fn(|i| (0..3).map(|k| second[i][k] * t[k][3]).sum());
            assert!(shifted[0] > 0. && shifted[1].abs() < 1e-12 && shifted[2].abs() < 1e-12);
            let baseline = t[0][3].hypot(t[1][3]).hypot(t[2][3]);
            assert!((shifted[0] - baseline).abs() < 1e-12);
        }
    }
}
