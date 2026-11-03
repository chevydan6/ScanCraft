#!/usr/bin/env python3
"""Test-only fixed-intrinsic stereo fit comparison, not scanner qualification.

OpenCV stereoCalibrate(CALIB_FIX_INTRINSIC) independently optimizes the common
relative pose and target poses. Supplied lens coefficients are fixed, not tested
as physically correct. Nonzero-skew cases use truth/NumPy, not unsupported OpenCV.
"""
import argparse
import hashlib
import json
from pathlib import Path

import cv2
import numpy as np
from compare_brown_calibration_opencv import coefficients, matrix, project


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('cases', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    raw = args.cases.read_bytes()
    fixture = json.loads(raw)
    if fixture.get('schema') != 1 or fixture.get('provenance') != 'synthetic_fixture_not_scanner_calibration':
        raise ValueError('Expected explicitly synthetic, versioned fixture')
    if sorted(c['name'] for c in fixture['cases']) != ['exact_nonzero_skew', 'exact_zero_skew', 'noisy_zero_skew']:
        raise ValueError('Expected exactly three distinct fixture cases')
    results = []
    for case in fixture['cases']:
        request, fit, truth = case['request'], case['fit'], case['truth']
        exact = case['noise_amplitude_px'] == 0
        if fit['fixed_left'] != request['fixed_left'] or fit['fixed_right'] != request['fixed_right']:
            raise ValueError('Fitter changed supposedly fixed lenses')
        r = np.asarray(fit['right_from_left']['rotation'])
        t = np.asarray(fit['right_from_left']['translation_mm'])
        rt = np.asarray(truth['right_from_left']['rotation'])
        tt = np.asarray(truth['right_from_left']['translation_mm'])
        residuals, pose_errors, rotation_errors = [], [], []
        for view, result, true_pose in zip(request['views'], fit['views'], truth['left_from_target'], strict=True):
            pose = result['left_from_target']
            pl = np.array([p['target_xy_mm'] + [0.] for p in view['observations']])
            lr, lt = np.asarray(pose['rotation']), np.asarray(pose['translation_mm'])
            left_camera_points = pl @ lr.T + lt
            right_camera_points = left_camera_points @ r.T + t
            if not np.all(left_camera_points[:, 2] > 0) or not np.all(right_camera_points[:, 2] > 0):
                raise ValueError('Fitted target has nonpositive depths')
            per_camera = []
            for camera, rotation, translation, key in [(request['fixed_left'], lr, lt, 'left_pixel'),
                                                        (request['fixed_right'], r @ lr, r @ lt + t, 'right_pixel')]:
                pixels = np.asarray([p[key] for p in view['observations']])
                errors = np.linalg.norm(project(pl, rotation, translation, matrix(camera), coefficients(camera['distortion'])) - pixels, axis=1)
                per_camera.append(errors)
                residuals.extend(errors)
            for key, errors in zip(['left', 'right'], per_camera, strict=True):
                if abs(float(np.sqrt(np.mean(errors**2))) - result[key+'_rms_px']) > 1e-9:
                    raise ValueError('Per-camera RMS inconsistent with independent projection')
            pose_errors.append(float(np.linalg.norm(lt-true_pose['translation_mm'])))
            rotation_errors.append(float(np.linalg.norm(lr-true_pose['rotation'])))
        rms = float(np.sqrt(np.mean(np.square(residuals))))
        holdout_error = max(float(np.linalg.norm(np.asarray(p['reconstructed_mm'])-p['ground_truth_mm'])) for p in case['holdouts'])
        error = {'rig_rotation_frobenius_vs_truth': float(np.linalg.norm(r-rt)),
                 'rig_translation_vs_truth_mm': float(np.linalg.norm(t-tt)),
                 'target_translation_vs_truth_mm': max(pose_errors),
                 'target_rotation_frobenius_vs_truth': max(rotation_errors),
                 'rms_recomputation_px': abs(rms-fit['rms_px']),
                 'maximum_recomputation_px': abs(float(max(residuals))-fit['max_error_px']),
                 'final_rms_px': rms, 'holdout_triangulation_vs_truth_mm': holdout_error,
                 'scaled_gradient_inf': fit['scaled_gradient_inf']}
        limit = {'rig_rotation_frobenius_vs_truth': 1e-8 if exact else 1e-3,
                 'rig_translation_vs_truth_mm': 1e-5 if exact else 0.1,
                 'target_translation_vs_truth_mm': 1e-5 if exact else 0.1,
                 'target_rotation_frobenius_vs_truth': 1e-8 if exact else 1e-3,
                 'rms_recomputation_px': 1e-9, 'maximum_recomputation_px': 1e-9,
                 'final_rms_px': 1e-7 if exact else 0.03,
                 'holdout_triangulation_vs_truth_mm': 1e-4 if exact else 0.25,
                 'scaled_gradient_inf': 1e-6}
        reference = None
        if request['fixed_left']['skew'] == request['fixed_right']['skew'] == 0:
            objects = [np.array([p['target_xy_mm'] + [0.] for p in v['observations']], dtype=np.float32) for v in request['views']]
            left = [np.array([p['left_pixel'] for p in v['observations']], dtype=np.float32) for v in request['views']]
            right = [np.array([p['right_pixel'] for p in v['observations']], dtype=np.float32) for v in request['views']]
            result = cv2.stereoCalibrate(objects, left, right,
                matrix(request['fixed_left']), coefficients(request['fixed_left']['distortion']),
                matrix(request['fixed_right']), coefficients(request['fixed_right']['distortion']),
                (request['fixed_left']['width'], request['fixed_left']['height']), flags=cv2.CALIB_FIX_INTRINSIC,
                criteria=(cv2.TERM_CRITERIA_COUNT | cv2.TERM_CRITERIA_EPS, 200, 1e-12))
            cv_rms, cv_r, cv_t = result[0], result[5], result[6].ravel()
            error.update({'opencv_rotation_discrepancy': float(np.linalg.norm(r-cv_r)),
                          'opencv_translation_discrepancy_mm': float(np.linalg.norm(t-cv_t)),
                          'opencv_rms_discrepancy_px': abs(rms-float(cv_rms))})
            limit.update({'opencv_rotation_discrepancy': 1e-5,
                          'opencv_translation_discrepancy_mm': 1e-3, 'opencv_rms_discrepancy_px': 1e-4})
            reference = {'method': 'stereoCalibrate CALIB_FIX_INTRINSIC, float32 observations',
                         'rms_px': float(cv_rms), 'right_from_left_rotation': cv_r.tolist(), 'translation_mm': cv_t.tolist()}
        passed = all(np.isfinite(value) and value <= limit[key] for key, value in error.items())
        passed = passed and fit['scaled_information_min_eigenvalue'] >= 1e-10
        results.append({'name': case['name'], 'views': len(request['views']),
                        'points_per_view': len(request['views'][0]['observations']),
                        'holdout_points': len(case['holdouts']), 'initial_rms_px': fit['initial_rms_px'],
                        'accepted_iterations': fit['accepted_iterations'], 'error': error, 'limit': limit,
                        'opencv_reference': reference, 'pass': bool(passed)})
    report = {'schema': 1, 'scope': 'synthetic fixed-intrinsic stereo fitting and held-out triangulation; not physical qualification',
              'fixture_sha256': hashlib.sha256(raw).hexdigest(), 'opencv_version': cv2.__version__, 'numpy_version': np.__version__,
              'cases': results, 'pass': all(r['pass'] for r in results)}
    args.output.write_text(json.dumps(report, indent=2, allow_nan=False)+'\n')
    print(json.dumps(report, indent=2, allow_nan=False))
    if not report['pass']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
