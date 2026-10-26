#!/usr/bin/env python3
"""Check our explicit rectification basis using OpenCV sparse-point transforms."""
import argparse
import json
from pathlib import Path
import cv2
import numpy as np


def camera(c):
    assert c['skew'] == 0
    k = np.array([[c['fx'], 0, c['cx']], [0, c['fy'], c['cy']], [0, 0, 1]], dtype=np.float64)
    d = c['distortion']
    return k, np.array([d['k1'], d['k2'], d['p1'], d['p2'], d['k3']])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('fixture', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    fixture = json.loads(args.fixture.read_text())
    assert fixture['provenance'] == 'synthetic_fixture_not_scanner_calibration'
    rect = fixture['rectification']
    source, ideal = rect['source'], rect['rectified']
    lk, ld = camera(source['left'])
    rk, rd = camera(source['right'])
    ik, _ = camera(ideal['left'])
    left_rotation = np.array(rect['left_from_rectified']['rotation'])
    right_rotation = np.array(rect['right_from_rectified']['rotation'])
    original_rotation = np.array(source['right_from_left']['rotation'])
    original_translation = np.array(source['right_from_left']['translation_mm'])
    rotation_error = float(np.linalg.norm(right_rotation.T@original_rotation@left_rotation-np.eye(3)))
    translation_error = float(np.linalg.norm(right_rotation.T@original_translation-np.array(ideal['right_from_left']['translation_mm'])))
    criteria = (cv2.TERM_CRITERIA_COUNT | cv2.TERM_CRITERIA_EPS, 100, 1e-14)
    errors = {'rectified_pixel_px': [], 'source_roundtrip_px': [], 'epipolar_y_px': [],
              'original_triangulation_mm': [], 'ground_truth_mm': []}
    for case in fixture['cases']:
        normalized = []
        for side, k, d, rotation in [('left', lk, ld, left_rotation), ('right', rk, rd, right_rotation)]:
            source_pixel = np.array(case[side+'_source'], dtype=np.float64).reshape(1, 1, 2)
            expected = np.array(case[side+'_rectified'])
            projected = cv2.undistortPoints(source_pixel, k, d, R=rotation.T, P=ik, criteria=criteria).ravel()
            errors['rectified_pixel_px'].append(float(np.linalg.norm(projected-expected)))
            # Independently map the rectified ideal ray back through source Brown optics.
            ray = rotation@np.linalg.solve(ik, np.array([*expected, 1.]))
            source_again = cv2.projectPoints(ray.reshape(1, 1, 3), np.zeros(3), np.zeros(3), k, d)[0].ravel()
            errors['source_roundtrip_px'].append(float(np.linalg.norm(source_again-np.array(case[side+'_remapped_source']))))
            normalized.append(cv2.undistortPoints(source_pixel, k, d, criteria=criteria).reshape(2, 1))
        errors['epipolar_y_px'].append(abs(case['left_rectified'][1]-case['right_rectified'][1]))
        homogeneous = cv2.triangulatePoints(np.column_stack([np.eye(3), np.zeros(3)]),
                                           np.column_stack([original_rotation, original_translation]), *normalized).ravel()
        ref_point = homogeneous[:3]/homogeneous[3]
        native = np.array(case['triangulated_original']['position_mm'])
        errors['original_triangulation_mm'].append(float(np.linalg.norm(native-ref_point)))
        errors['ground_truth_mm'].append(float(np.linalg.norm(native-np.array(case['ground_truth_mm']))))
    maxima = {k: max(v) for k, v in errors.items()}
    maxima.update({'rotation_composition': rotation_error, 'translation_composition_mm': translation_error})
    thresholds = {'rectified_pixel_px': 1e-8, 'source_roundtrip_px': 1e-8, 'epipolar_y_px': 1e-8,
                  'original_triangulation_mm': 1e-6, 'ground_truth_mm': 1e-6,
                  'rotation_composition': 1e-12, 'translation_composition_mm': 1e-10}
    passed = all(np.isfinite(v) and v <= thresholds[k] for k, v in maxima.items())
    result = {'scope': 'synthetic rectification with our explicit basis; not equality to OpenCV stereoRectify basis, physical Raptor calibration or accuracy',
              'opencv_version': cv2.__version__, 'numpy_version': np.__version__, 'cases': len(fixture['cases']),
              'max_error': maxima, 'threshold': thresholds, 'pass': bool(passed)}
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
