#!/usr/bin/env python3
"""Independent numerical reference. OpenCV is test-only, never an application dependency."""
import argparse
import json
from pathlib import Path
import cv2
import numpy as np


def camera(model):
    if model['skew'] != 0:
        raise ValueError('This reference comparison requires zero skew; skew has separate analytical Rust tests.')
    matrix = np.array([[model['fx'], 0, model['cx']], [0, model['fy'], model['cy']], [0, 0, 1.]], dtype=np.float64)
    d = model['distortion']
    coefficients = np.array([d['k1'], d['k2'], d['p1'], d['p2'], d['k3']], dtype=np.float64)
    return matrix, coefficients


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('cases', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    fixture = json.loads(args.cases.read_text())
    if fixture['provenance'] != 'synthetic_fixture_not_scanner_calibration':
        raise ValueError('Expected an explicitly synthetic test fixture')
    rig = fixture['rig']
    left_k, left_d = camera(rig['left'])
    right_k, right_d = camera(rig['right'])
    rotation = np.asarray(rig['right_from_left']['rotation'], dtype=np.float64)
    translation = np.asarray(rig['right_from_left']['translation_mm'], dtype=np.float64)
    rotation_vector = cv2.Rodrigues(rotation)[0]
    left_p = np.column_stack((np.eye(3), np.zeros(3)))
    right_p = np.column_stack((rotation, translation))
    criteria = (cv2.TERM_CRITERIA_COUNT | cv2.TERM_CRITERIA_EPS, 100, 1e-14)
    errors = {'projection_px': [], 'ray_normalized': [], 'triangulation_mm': [], 'ground_truth_mm': []}
    for case in fixture['cases']:
        point = np.asarray(case['ground_truth_mm'], dtype=np.float64).reshape(1, 1, 3)
        left_pixel = np.asarray(case['left_pixel'], dtype=np.float64).reshape(1, 1, 2)
        right_pixel = np.asarray(case['right_pixel'], dtype=np.float64).reshape(1, 1, 2)
        lp = cv2.projectPoints(point, np.zeros(3), np.zeros(3), left_k, left_d)[0]
        rp = cv2.projectPoints(point, rotation_vector, translation, right_k, right_d)[0]
        errors['projection_px'].extend([float(np.linalg.norm(lp-left_pixel)), float(np.linalg.norm(rp-right_pixel))])
        left_u = cv2.undistortPoints(left_pixel,left_k,left_d,criteria=criteria).reshape(2, 1)
        right_u = cv2.undistortPoints(right_pixel,right_k,right_d,criteria=criteria).reshape(2, 1)
        for expected, normalized in [(case['left_ray'], left_u), (case['right_ray'], right_u)]:
            ray = np.array([*normalized.ravel(), 1.]); ray /= np.linalg.norm(ray)
            errors['ray_normalized'].append(float(np.linalg.norm(ray-np.asarray(expected))))
        homogeneous = cv2.triangulatePoints(left_p, right_p, left_u, right_u).ravel()
        reference = homogeneous[:3]/homogeneous[3]
        native = np.asarray(case['triangulated']['position_mm'])
        errors['triangulation_mm'].append(float(np.linalg.norm(reference-native)))
        errors['ground_truth_mm'].append(float(np.linalg.norm(native-point.ravel())))
    maxima = {name: max(values) for name, values in errors.items()}
    thresholds = {'projection_px': 1e-8, 'ray_normalized': 1e-10, 'triangulation_mm': 1e-6, 'ground_truth_mm': 1e-6}
    passed = all(np.isfinite(value) and value <= thresholds[name] for name, value in maxima.items())
    result = {'opencv_version': cv2.__version__, 'numpy_version': np.__version__, 'cases': len(fixture['cases']),
              'scope': 'synthetic numerical agreement only; not real scanner or dimensional qualification',
              'max_error': maxima, 'threshold': thresholds, 'pass': bool(passed)}
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
