#!/usr/bin/env python3
"""Independent test-only Brown5 fit validation; no hardware/physical qualification.

OpenCV's calibrateCamera refines the same zero-skew Brown5 projection for the
zero-skew cases, from the same supplied pinhole seed, using float32 observations.
The nonzero-skew case is validated against truth and NumPy projection only.
https://docs.opencv.org/4.13.0/d9/d0c/group__calib3d.html
"""
import argparse
import hashlib
import json
from pathlib import Path

import cv2
import numpy as np


def matrix(k):
    return np.array([[k['fx'], k['skew'], k['cx']], [0., k['fy'], k['cy']], [0., 0., 1.]])


def coefficients(d):
    return np.array([d['k1'], d['k2'], d['p1'], d['p2'], d['k3']])


def project(points, rotation, translation, k, d):
    p = points @ rotation.T + translation
    if not np.all(np.isfinite(p)) or not np.all(p[:, 2] > 0):
        raise ValueError('Invalid fitted target depths')
    x, y = (p[:, :2] / p[:, 2:]).T
    r2 = x*x + y*y
    k1, k2, p1, p2, k3 = d
    radial = 1 + k1*r2 + k2*r2**2 + k3*r2**3
    xd = x*radial + 2*p1*x*y + p2*(r2 + 2*x*x)
    yd = y*radial + p1*(r2 + 2*y*y) + 2*p2*x*y
    return np.column_stack((k[0, 0]*xd + k[0, 1]*yd + k[0, 2], k[1, 1]*yd + k[1, 2]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('cases', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    source = args.cases.read_bytes()
    fixture = json.loads(source)
    if fixture.get('schema') != 1 or fixture.get('provenance') != 'synthetic_fixture_not_scanner_calibration':
        raise ValueError('Expected explicitly synthetic, versioned data')
    names = [c['name'] for c in fixture['cases']]
    if sorted(names) != sorted(['exact_zero_skew', 'exact_nonzero_skew', 'noisy_zero_skew']):
        raise ValueError('Expected exactly the three reference cases')
    results = []
    for case in fixture['cases']:
        fit, truth = case['refinement'], case['truth']
        views = case['request']['views']
        k, d = matrix(fit['intrinsics']), coefficients(fit['distortion'])
        truth_k, truth_d = matrix(truth['camera']), coefficients(truth['camera']['distortion'])
        exact = case['noise_amplitude_px'] == 0
        error = {'intrinsics_vs_truth_px': float(np.max(np.abs(k-truth_k))),
                 'distortion_vs_truth': float(np.max(np.abs(d-truth_d))),
                 'scaled_gradient_inf': fit['scaled_gradient_inf']}
        limit = {'intrinsics_vs_truth_px': 1e-4 if exact else 1.,
                 'distortion_vs_truth': 1e-4 if exact else 0.1,
                 'scaled_gradient_inf': 1e-6}
        residuals, translations, rotations = [], [], []
        for view, output_pose, truth_pose in zip(views, fit['views'], truth['poses'], strict=True):
            pose = output_pose['camera_from_target']
            r, t = np.array(pose['rotation']), np.array(pose['translation_mm'])
            points = np.array([p['target_xy_mm'] + [0.] for p in view['observations']])
            pixels = np.array([p['pixel'] for p in view['observations']])
            residuals.extend(np.linalg.norm(project(points, r, t, k, d)-pixels, axis=1))
            translations.append(float(np.linalg.norm(t-truth_pose['translation_mm'])))
            rotations.append(float(np.linalg.norm(r-truth_pose['rotation'])))
        rms = float(np.sqrt(np.mean(np.square(residuals))))
        error.update({'translation_vs_truth_mm': max(translations), 'rotation_frobenius_vs_truth': max(rotations),
                      'rms_recomputation_px': abs(rms-fit['rms_px']),
                      'max_recomputation_px': abs(float(max(residuals))-fit['max_error_px']),
                      'final_rms_px': rms})
        limit.update({'translation_vs_truth_mm': 1e-4 if exact else 1.,
                      'rotation_frobenius_vs_truth': 1e-6 if exact else 0.01,
                      'rms_recomputation_px': 1e-9, 'max_recomputation_px': 1e-9,
                      'final_rms_px': 1e-6 if exact else 0.03})
        reference = None
        if truth['camera']['skew'] == 0:
            objects = [np.array([p['target_xy_mm'] + [0.] for p in v['observations']], dtype=np.float32) for v in views]
            images = [np.array([p['pixel'] for p in v['observations']], dtype=np.float32) for v in views]
            seed = matrix(case['initialization']['intrinsics'])
            seed[0, 1] = 0
            cv_rms, cv_k, cv_d, _, _ = cv2.calibrateCamera(
                objects, images, tuple(case['request']['image_size']), seed, np.zeros(5),
                flags=cv2.CALIB_USE_INTRINSIC_GUESS,
                criteria=(cv2.TERM_CRITERIA_COUNT | cv2.TERM_CRITERIA_EPS, 200, 1e-12))
            error.update({'opencv_intrinsics_discrepancy_px': float(np.max(np.abs(cv_k-k))),
                          'opencv_distortion_discrepancy': float(np.max(np.abs(cv_d.ravel()-d))),
                          'opencv_rms_discrepancy_px': abs(float(cv_rms)-rms)})
            limit.update({'opencv_intrinsics_discrepancy_px': 0.02,
                          'opencv_distortion_discrepancy': 0.002, 'opencv_rms_discrepancy_px': 1e-4})
            reference = {'intrinsics': cv_k.tolist(), 'distortion_k1_k2_p1_p2_k3': cv_d.ravel().tolist(),
                         'rms_px': float(cv_rms), 'method': 'calibrateCamera full Brown5, zero skew, common seed, float32 observations'}
        passed = all(np.isfinite(value) and value <= limit[key] for key, value in error.items())
        passed = passed and fit['rms_px'] < fit['initial_rms_px'] and fit['scaled_information_min_eigenvalue'] >= 1e-10
        results.append({'name': case['name'], 'views': len(views), 'points_per_view': len(views[0]['observations']),
                        'initial_rms_px': fit['initial_rms_px'], 'error': error, 'limit': limit,
                        'scaled_information_min_eigenvalue': fit['scaled_information_min_eigenvalue'],
                        'accepted_iterations': fit['accepted_iterations'], 'opencv_reference': reference, 'pass': bool(passed)})
    result = {'schema': 1, 'scope': 'synthetic Brown5 refinement only; not Raptor optical binding/physical accuracy',
              'fixture_sha256': hashlib.sha256(source).hexdigest(), 'opencv_version': cv2.__version__,
              'numpy_version': np.__version__, 'cases': results, 'pass': all(r['pass'] for r in results)}
    args.output.write_text(json.dumps(result, indent=2, allow_nan=False)+'\n')
    print(json.dumps(result, indent=2, allow_nan=False))
    if not result['pass']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
