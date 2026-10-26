//! Observed forward rational Brown math. Evaluation alone does not validate
//! invertibility, calibration coverage, image binding or physical accuracy.
use crate::Brown5;

/// Coefficient order: k1,k2,k3,k4,k5,k6,p1,p2. Input/output are normalized
/// optical coordinates. This does not create a validated CameraModel.
pub fn forward(coefficients: [f32; 8], point: [f64; 2]) -> Result<[f64; 2], String> {
    if !coefficients.iter().all(|v| v.is_finite()) || !point.iter().all(|v| v.is_finite()) {
        return Err("nonfinite Brown input".into());
    }
    let [k1, k2, k3, k4, k5, k6, p1, p2] = coefficients.map(f64::from);
    let [x, y] = point;
    let r2 = x * x + y * y;
    let r4 = r2 * r2;
    let r6 = r4 * r2;
    let denominator = 1. + k4 * r2 + k5 * r4 + k6 * r6;
    if !denominator.is_finite() || denominator <= 0. {
        return Err("unsupported Brown denominator".into());
    }
    let radial = (1. + k1 * r2 + k2 * r4 + k3 * r6) / denominator;
    let output = [
        x * radial + 2. * p1 * x * y + p2 * (r2 + 2. * x * x),
        y * radial + p1 * (r2 + 2. * y * y) + 2. * p2 * x * y,
    ];
    if !output.iter().all(|v| v.is_finite()) {
        return Err("Brown evaluation overflow".into());
    }
    Ok(output)
}

/// Lossless coefficient reduction only. Caller must still establish the
/// supported optical domain and validate CameraModel before inverse mapping.
pub fn brown5_if_supported(coefficients: [f32; 8]) -> Result<Brown5, String> {
    if !coefficients.iter().all(|v| v.is_finite()) {
        return Err("nonfinite Brown coefficient".into());
    }
    if coefficients[3..6].iter().any(|v| *v != 0.) {
        return Err("rational denominator cannot be represented by Brown5".into());
    }
    Ok(Brown5 {
        k1: f64::from(coefficients[0]),
        k2: f64::from(coefficients[1]),
        k3: f64::from(coefficients[2]),
        p1: f64::from(coefficients[6]),
        p2: f64::from(coefficients[7]),
    })
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
        coefficients: [f32; 8],
        samples: Vec<Sample>,
    }
    #[derive(Deserialize)]
    struct Sample {
        normalized_input: [f64; 2],
        forward_output: [f64; 2],
    }
    #[test]
    fn forward_and_supported_brown5_match_isolated_vendor_samples() {
        let fixture: Fixture =
            serde_json::from_str(include_str!("../../../evidence/factory-brown-cases.json"))
                .unwrap();
        assert!(fixture.synthetic_only);
        assert_eq!(fixture.cases.len(), 6);
        let mut samples = 0;
        for case in fixture.cases {
            for sample in case.samples {
                samples += 1;
                let output = forward(case.coefficients, sample.normalized_input).unwrap();
                for (a, b) in output.iter().zip(sample.forward_output) {
                    assert!((a - b).abs() < 2e-7);
                }
                if let Ok(brown) = brown5_if_supported(case.coefficients) {
                    let expected = brown
                        .map(sample.normalized_input[0], sample.normalized_input[1])
                        .unwrap()
                        .0;
                    for (a, b) in output.iter().zip(expected) {
                        assert!((a - b).abs() < 1e-14);
                    }
                }
            }
        }
        assert_eq!(samples, 210);
    }
    #[test]
    fn guards_nonfinite_poles_overflow_and_lossy_reduction() {
        for i in 0..8 {
            let mut c = [0.; 8];
            c[i] = f32::NAN;
            assert!(forward(c, [0., 0.]).is_err());
            assert!(brown5_if_supported(c).is_err());
        }
        for i in 3..6 {
            let mut c = [0.; 8];
            c[i] = f32::MIN_POSITIVE;
            assert!(brown5_if_supported(c).is_err());
        }
        let mut pole = [0.; 8];
        pole[3] = -1.;
        assert!(forward(pole, [1., 0.]).is_err());
        assert!(forward(pole, [2., 0.]).is_err());
        assert!(forward([0.; 8], [f64::MAX, 1.]).is_err());
        assert!(forward([0.; 8], [f64::NAN, 0.]).is_err());
        assert_eq!(forward([0.; 8], [0.2, -0.3]).unwrap(), [0.2, -0.3]);
    }
}
