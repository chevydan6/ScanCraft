#!/usr/bin/env python3
"""Synthetic-only corner measurement oracle; no scanner or installed calibration.

NumPy uses a general linear solve on independently resampled float64 gradients;
OpenCV cornerSubPix is a second, mature implementation, not a runtime dependency.
It has different failure/border policies. These interior test cases compare only
successful refinement and make no hardware, performance or covariance claims.
"""
import argparse
import hashlib
import json
from pathlib import Path
import cv2
import numpy as np


def sample(image, x, y):
    ix, iy = np.floor(x).astype(int), np.floor(y).astype(int)
    dx, dy = x-ix, y-iy
    return ((1-dx)*(1-dy)*image[iy, ix] + dx*(1-dy)*image[iy, ix+1]
            + (1-dx)*dy*image[iy+1, ix] + dx*dy*image[iy+1, ix+1])


def oracle(image, seed, radius, epsilon, iterations):
    dy, dx = np.mgrid[-radius:radius+1, -radius:radius+1]
    weights = np.exp(-(dx**2+dy**2)/radius**2).ravel()
    offsets = np.stack([dx.ravel(), dy.ravel()], axis=1)
    center = np.array(seed, dtype=np.float64)
    for _ in range(iterations):
        px, py = center[0]+dx, center[1]+dy
        gradients = np.stack([(sample(image, px+1, py)-sample(image, px-1, py)).ravel(),
                              (sample(image, px, py+1)-sample(image, px, py-1)).ravel()], axis=1)/510.
        normal = gradients.T @ (weights[:, None]*gradients)
        b = gradients.T @ (weights*np.sum(gradients*offsets, axis=1))
        delta = np.linalg.solve(normal, b)
        center += delta
        if np.linalg.norm(delta) <= epsilon:
            return center
    raise ValueError('Independent oracle did not converge')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('fixture', type=Path)
    parser.add_argument('report', type=Path)
    args = parser.parse_args()
    raw = args.fixture.read_bytes()
    fixture = json.loads(raw)
    if fixture['schema_version'] != 1 or fixture['synthetic_only'] is not True:
        raise ValueError('Expected versioned synthetic fixture')
    cases = []
    for case in fixture['cases']:
        image = np.array(case['image'], dtype=np.uint8).reshape(fixture['height'], fixture['width'])
        rust = np.array(case['rust_pixel'])
        independent = oracle(image.astype(np.float64), case['seed'], fixture['radius'],
                             fixture['epsilon_px'], fixture['max_iterations'])
        cv = cv2.cornerSubPix(image, np.array([[case['seed']]], dtype=np.float32),
                              (fixture['radius'], fixture['radius']), (-1, -1),
                              (cv2.TERM_CRITERIA_EPS | cv2.TERM_CRITERIA_COUNT,
                               fixture['max_iterations'], fixture['epsilon_px'])).reshape(2)
        errors = {'numpy_px': float(np.linalg.norm(rust-independent)),
                  'opencv_px': float(np.linalg.norm(rust-cv)),
                  'truth_px': float(np.linalg.norm(rust-case['truth']))}
        # Numerical comparison tolerances for this fixed synthetic family only.
        limits = {'numpy_px': 0.0001, 'opencv_px': 0.001, 'truth_px': 0.08}
        if not all(np.isfinite(errors[k]) and errors[k] <= v for k, v in limits.items()):
            raise ValueError(f'Corner comparison failed: {errors}')
        cases.append({'angle_radians': case['angle_radians'], 'inverted': case['inverted'],
                      'noise_amplitude': case['noise_amplitude'], 'errors': errors, 'limits': limits})
    report = {'schema_version': 1, 'synthetic_only': True, 'physical_accuracy_qualified': False,
              'fixture_sha256': hashlib.sha256(raw).hexdigest(), 'opencv_version': cv2.__version__,
              'numpy_version': np.__version__, 'cases': cases}
    args.report.write_text(json.dumps(report, indent=2)+'\n')
    print(f'PASS {len(cases)} synthetic corner cases')


if __name__ == '__main__':
    main()
