use scancraft_geometry::work::WorkControl;
use scancraft_image::{
    GrayImage,
    laser::{self, LineConfig},
};

#[test]
fn exact_rows_singletons_and_involution() {
    for (width, height, source, expected) in [
        (3, 2, vec![1, 2, 3, 4, 5, 6], vec![3, 2, 1, 6, 5, 4]),
        (
            4,
            3,
            (0..12).collect(),
            vec![3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8],
        ),
        (1, 3, vec![7, 8, 9], vec![7, 8, 9]),
    ] {
        let original = source.clone();
        let mut scratch = vec![255; source.len()];
        GrayImage::new(width, height, &source)
            .unwrap()
            .reflect_x_into(&mut scratch)
            .unwrap();
        assert_eq!(scratch, expected);
        assert_eq!(source, original);
        let mut restored = vec![0; source.len()];
        GrayImage::new(width, height, &scratch)
            .unwrap()
            .reflect_x_into(&mut restored)
            .unwrap();
        assert_eq!(restored, source);
    }
}

#[test]
fn wrong_scratch_length_is_rejected_before_mutation() {
    let image = GrayImage::new(3, 2, &[1, 2, 3, 4, 5, 6]).unwrap();
    for size in [0, 5, 7] {
        let mut scratch = vec![99; size];
        assert!(image.reflect_x_into(&mut scratch).is_err());
        assert_eq!(scratch, vec![99; size]);
    }
}

#[test]
fn raw_and_local_opening_detections_agree_with_pixel_reflection() {
    let mut source = vec![0; 128];
    for y in 0..2 {
        for x in 0..64 {
            source[y * 64 + x] = 10 + (x / 4) as u8;
        }
        source[y * 64] = 100; // Border rejection must move to the opposite border.
        source[y * 64 + 10] = 100;
        source[y * 64 + 11] = 140;
        source[y * 64 + 12] = 110;
        source[y * 64 + 42] = 255; // Raw clipping must still reject after local opening.
    }
    let original = source.clone();
    let image = GrayImage::new(64, 2, &source).unwrap();
    let mut scratch = vec![0; source.len()];
    image.reflect_x_into(&mut scratch).unwrap();
    let mirrored = GrayImage::new(64, 2, &scratch).unwrap();
    let config = LineConfig {
        background: 30,
        minimum_peak: 50,
        maximum_width: 6,
        saturation: 255,
        maximum_observations: 100,
    };
    for opening in [false, true] {
        let detect = |im| {
            if opening {
                laser::detect_rows_with_opening(im, config, 4, &WorkControl::default())
            } else {
                laser::detect_rows(im, config, &WorkControl::default())
            }
        };
        let a = detect(image).unwrap();
        let b = detect(mirrored).unwrap();
        assert_eq!(a.observations.len(), 2);
        assert_eq!(a.observations.len(), b.observations.len());
        assert_eq!(a.rejected_border, b.rejected_border);
        assert_eq!(a.rejected_saturation, b.rejected_saturation);
        for observation in a.observations {
            let expected =
                scancraft_calibration::orientation::reflect_pixel_x([64, 2], observation.pixel)
                    .unwrap();
            let other = b
                .observations
                .iter()
                .find(|o| o.pixel[1] == expected[1] && (o.pixel[0] - expected[0]).abs() < 1e-10)
                .unwrap();
            assert_eq!(other.first_x, 63 - observation.last_x);
            assert_eq!(other.last_x, 63 - observation.first_x);
            assert_eq!(other.peak, observation.peak);
            assert_eq!(other.integrated_signal, observation.integrated_signal);
            assert!((other.variance_x - observation.variance_x).abs() < 1e-9);
        }
    }
    assert_eq!(source, original);
}
