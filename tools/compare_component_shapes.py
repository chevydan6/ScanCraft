#!/usr/bin/env python3
"""Optional NumPy SVD reference for native successful shape fits on identical pixels.

Uses the existing OpenCV component/CRC comparison first. Rejected native fits
are counted, not certified by this tool. No physical accuracy claim is made.
"""
import argparse
import json
from pathlib import Path

import cv2
import numpy as np

from compare_components_opencv import compare as compare_components


def compare(image_path, report_path):
    binding = compare_components(image_path, report_path)
    report = json.loads(report_path.read_text())
    image = cv2.imread(str(image_path), cv2.IMREAD_GRAYSCALE)
    threshold = report["parameters"]["threshold_exclusive"]
    checked, rejected = 0, 0
    maximum_center_error = 0.
    for observation in report["observations"]:
        shape = observation["shape"]
        assert shape["physical_center_verified"] is False
        if "fit" not in shape:
            assert shape["rms_gate_passed"] is False and isinstance(shape["error"], str)
            rejected += 1
            continue
        x0, y0, x1, y1 = observation["bounds_inclusive"]
        mask = image[y0-1:y1+2, x0-1:x1+2] > threshold
        interior = mask[1:-1, 1:-1]
        boundary = interior & ~(mask[:-2, 1:-1] & mask[2:, 1:-1]
                                & mask[1:-1, :-2] & mask[1:-1, 2:])
        yy, xx = np.nonzero(boundary)
        points = np.column_stack([xx+x0, yy+y0]).astype(float)
        origin = points.mean(axis=0)
        scale = np.sqrt(np.mean(np.sum((points-origin)**2, axis=1)))
        xy = (points-origin)/scale
        x, y = xy.T
        design = np.column_stack([x*x, x*y, y*y, x, y, np.ones(len(x))])
        _, _, vectors = np.linalg.svd(design, full_matrices=False)
        a, b, c, d, e, f = vectors[-1]
        quadratic = np.array([[a, b/2], [b/2, c]])
        center = -.5*np.linalg.solve(quadratic, [d, e])
        metric = quadratic/(center@quadratic@center-f)
        eigenvalues, _ = np.linalg.eigh(metric)
        assert min(eigenvalues) > 0
        axes = scale/np.sqrt(eigenvalues)
        delta = xy-center
        residuals = np.abs(np.sum((delta@metric)*delta, axis=1)-1) / np.linalg.norm(2*delta@metric, axis=1)*scale
        native = shape["fit"]
        center_error = float(max(abs(np.array(native["center"])-(origin+scale*center))))
        assert center_error < 1e-7
        assert np.allclose(native["semiaxes"], axes, rtol=1e-8, atol=1e-7)
        assert abs(native["rms_sampson"]-np.sqrt(np.mean(residuals**2))) < 1e-7
        assert abs(native["maximum_sampson"]-max(residuals)) < 1e-7
        assert native["samples"] == len(points)
        assert shape["rms_gate_passed"] == (native["rms_sampson"] <= report["shape_parameters"]["maximum_rms_sampson_px"])
        maximum_center_error = max(maximum_center_error, center_error)
        checked += 1
    return {"comparison": "PASS", "successful_fits_checked": checked,
            "native_errors_not_numerically_validated": rejected,
            "maximum_center_difference_pixels": maximum_center_error,
            "numpy": np.__version__, "component_binding": binding,
            "physical_accuracy_qualified": False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image", type=Path)
    parser.add_argument("native_report", type=Path)
    args = parser.parse_args()
    print(json.dumps(compare(args.image, args.native_report), indent=2))
