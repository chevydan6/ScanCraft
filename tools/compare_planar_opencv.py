#!/usr/bin/env python3
"""Test-only synthetic reference; neither installs nor qualifies calibration.

OpenCV calibrateCamera refines a zero-skew, fixed-zero-distortion model. Rust's
initializer estimates skew and does not refine. Compare under explicitly stated
numerical tolerances, not bitwise equivalence or a performance/accuracy contest.
https://docs.opencv.org/4.13.0/d9/d0c/group__calib3d.html
"""
import argparse
import hashlib
import json
from pathlib import Path

import cv2
import numpy as np


def intrinsic_matrix(values):
    return np.array([[values['fx'], values['skew'], values['cx']],
                     [0., values['fy'], values['cy']], [0., 0., 1.]], dtype=np.float64)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('cases', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    fixture_bytes = args.cases.read_bytes()
    fixture = json.loads(fixture_bytes)
    if fixture.get('schema') != 1 or fixture.get('provenance') != 'synthetic_fixture_not_scanner_calibration':
        raise ValueError('Expected versioned, explicitly synthetic observations')
    results = []
    for case in fixture['cases']:
        estimate = case['estimate']
        k = intrinsic_matrix(estimate['intrinsics'])
        truth_k = intrinsic_matrix(case['truth'])
        exact = case['noise_amplitude_px'] == 0
        errors = {'intrinsics_vs_truth_px': float(np.max(np.abs(k - truth_k)))}
        limits = {'intrinsics_vs_truth_px': 1e-6 if exact else 2.}
        pose_translation_errors, pose_rotation_errors, residuals = [], [], []
        for view, pose, truth_pose in zip(case['observations'], estimate['views'], case['truth']['poses'], strict=True):
            r = np.asarray(pose['camera_from_target']['rotation'])
            t = np.asarray(pose['camera_from_target']['translation_mm'])
            pose_translation_errors.append(float(np.linalg.norm(t - truth_pose['translation_mm'])))
            pose_rotation_errors.append(float(np.linalg.norm(r - truth_pose['rotation'])))
            points = np.asarray([p['target_xy_mm'] + [0.] for p in view['observations']])
            pixels = np.asarray([p['pixel'] for p in view['observations']])
            camera_points = points @ r.T + t
            if not np.all(camera_points[:, 2] > 0):
                raise ValueError('Nonpositive synthetic target depth')
            projected = (camera_points / camera_points[:, 2:]) @ k.T
            residuals.extend(np.linalg.norm(projected[:, :2] - pixels, axis=1))
        errors.update({'max_translation_vs_truth_mm': max(pose_translation_errors),
                       'max_rotation_frobenius_vs_truth': max(pose_rotation_errors),
                       'reported_rms_discrepancy_px': abs(float(np.sqrt(np.mean(np.square(residuals)))) - estimate['rms_px']),
                       'reported_max_discrepancy_px': abs(float(max(residuals)) - estimate['max_error_px'])})
        limits.update({'max_translation_vs_truth_mm': 1e-6 if exact else 2.,
                       'max_rotation_frobenius_vs_truth': 1e-8 if exact else 0.01,
                       'reported_rms_discrepancy_px': 1e-9, 'reported_max_discrepancy_px': 1e-9})
        reference = None
        if case['truth']['skew'] == 0:
            objects = [np.asarray([p['target_xy_mm'] + [0.] for p in v['observations']], dtype=np.float32) for v in case['observations']]
            images = [np.asarray([p['pixel'] for p in v['observations']], dtype=np.float32) for v in case['observations']]
            flags = cv2.CALIB_ZERO_TANGENT_DIST | cv2.CALIB_FIX_K1 | cv2.CALIB_FIX_K2 | cv2.CALIB_FIX_K3
            rms, cv_k, distortion, _, _ = cv2.calibrateCamera(
                objects, images, tuple(estimate['image_size']), None, np.zeros(5), flags=flags,
                criteria=(cv2.TERM_CRITERIA_COUNT | cv2.TERM_CRITERIA_EPS, 200, 1e-12))
            errors['opencv_intrinsics_vs_truth_px'] = float(np.max(np.abs(cv_k - truth_k)))
            errors['rust_vs_opencv_intrinsics_px'] = float(np.max(np.abs(k - cv_k)))
            limits['opencv_intrinsics_vs_truth_px'] = 0.02 if exact else 2.
            limits['rust_vs_opencv_intrinsics_px'] = 0.02 if exact else 2.
            if not np.all(distortion == 0):
                raise ValueError('Reference failed fixed-zero-distortion contract')
            reference = {'method': 'OpenCV calibrateCamera, refined zero-skew/fixed zero distortion; float32 observations',
                         'rms_px': float(rms), 'matrix': cv_k.tolist()}
        passed = all(np.isfinite(v) and v <= limits[name] for name, v in errors.items())
        results.append({'name': case['name'], 'views': len(case['observations']),
                        'points_per_view': len(case['observations'][0]['observations']),
                        'error': errors, 'limit': limits, 'opencv_reference': reference,
                        'pass': bool(passed)})
    if len(results) != 3 or len({r['name'] for r in results}) != 3:
        raise ValueError('Expected all three distinct reference cases')
    result = {'schema': 1, 'scope': 'synthetic numerical initializer validation only; not physical Raptor calibration or accuracy',
              'fixture_sha256': hashlib.sha256(fixture_bytes).hexdigest(),
              'opencv_version': cv2.__version__, 'numpy_version': np.__version__,
              'cases': results, 'pass': all(r['pass'] for r in results)}
    args.output.write_text(json.dumps(result, indent=2, allow_nan=False) + '\n')
    print(json.dumps(result, indent=2, allow_nan=False))
    if not result['pass']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
