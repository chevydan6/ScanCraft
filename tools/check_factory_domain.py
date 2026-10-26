#!/usr/bin/env python3
"""Offline OpenCV inverse-domain reference; private lens JSON is never copied out."""
import argparse
import json
import cv2
import numpy as np

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('private_lenses')
parser.add_argument('native_audit')
a = parser.parse_args()
lenses = json.load(open(a.private_lenses))['lenses']
native = json.load(open(a.native_audit))
reports = []
for lens, audit in zip(lenses, native['lenses'], strict=True):
    assert lens['label'] == audit['label']
    width, height = lens['width'], lens['height']
    assert 0 < width * height <= 4194304
    fx, fy, cx, cy = lens['intrinsic_values']
    k = np.array([[fx, 0, cx], [0, fy, cy], [0, 0, 1]], dtype=np.float64)
    k1, k2, k3, k4, k5, k6, p1, p2 = lens['brown_values']
    assert k4 == k5 == k6 == 0
    distortion = np.array([k1, k2, p1, p2, k3], dtype=np.float64)
    max_radius = 0.
    max_error = 0.
    checked = 0
    for y0 in range(0, height, 32):
        x, y = np.meshgrid(np.arange(width), np.arange(y0, min(y0+32, height)))
        pixels = np.stack([x.ravel(), y.ravel()], axis=1).astype(np.float64).reshape(-1, 1, 2)
        criteria = (cv2.TERM_CRITERIA_COUNT | cv2.TERM_CRITERIA_EPS, 100, 1e-12)
        if hasattr(cv2, "undistortPointsIter"):
            inverse = cv2.undistortPointsIter(pixels, k, distortion, None, None, criteria)
        else:
            inverse = cv2.undistortPoints(pixels, k, distortion, criteria=criteria)
        inverse = inverse.reshape(-1, 2)
        rays = np.column_stack([inverse, np.ones(len(inverse))])
        projected, _ = cv2.projectPoints(rays, np.zeros(3), np.zeros(3), k, distortion)
        error = np.linalg.norm(projected.reshape(-1, 2)-pixels.reshape(-1, 2), axis=1)
        assert np.isfinite(inverse).all() and np.isfinite(error).all()
        max_radius = max(max_radius, float(np.linalg.norm(inverse, axis=1).max()))
        max_error = max(max_error, float(error.max()))
        checked += len(inverse)
    delta = abs(max_radius-audit['maximum_inverse_normalized_radius'])
    assert checked == audit['pixel_centers_checked'] and audit['failed_pixel_centers'] == 0
    assert max_error < 1e-7 and delta < 1e-9
    reports.append(dict(label=lens['label'], checked=checked,
        reference_max_roundtrip_error_px=max_error,
        native_reference_maximum_radius_difference=delta))
print(json.dumps(dict(scope='OpenCV numerical inverse-domain reference only; aggregate comparison, not pointwise ray equality or physical qualification',
    opencv_version=cv2.__version__, physical_accuracy_qualified=False, lenses=reports), indent=2))
