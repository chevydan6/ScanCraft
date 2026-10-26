#!/usr/bin/env python3
"""Independent OpenCV map reference; Python/OpenCV are test-only dependencies."""
import argparse
import json
from pathlib import Path
import cv2
import numpy as np


def matrix(c):
    assert c['skew'] == 0, 'This reference only covers zero source/output skew'
    return np.array([[c['fx'], 0, c['cx']], [0, c['fy'], c['cy']], [0, 0, 1]], dtype=np.float64)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('fixture', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    fixture = json.loads(args.fixture.read_text())
    assert fixture['provenance'] == 'synthetic_fixture_not_scanner_calibration'
    source, output = fixture['source'], fixture['output']
    d = source['distortion']
    distortion = np.array([d['k1'], d['k2'], d['p1'], d['p2'], d['k3']])
    # OpenCV R is output-from-source. The Rust API explicitly takes its inverse.
    rotation = np.array(fixture['source_from_output'], dtype=np.float64).T
    mx, my = cv2.initUndistortRectifyMap(matrix(source), distortion, rotation,
                                        matrix(output), (output['width'], output['height']), cv2.CV_32FC1)
    native_valid = np.array(fixture['valid'], dtype=bool)
    ref_valid = ((mx >= 0) & (my >= 0) & (mx <= source['width']-1) & (my <= source['height']-1)).ravel()
    # This fixture lies inside both models' supported disks: every pixel can be compared.
    assert native_valid.size == ref_valid.size and np.array_equal(native_valid, ref_valid)
    coords = np.array([p if p is not None else [np.nan, np.nan] for p in fixture['coordinates']])
    reference = np.column_stack([mx.ravel(), my.ravel()]).astype(np.float64)
    max_map_error = float(np.max(np.abs(coords[native_valid]-reference[native_valid])))
    image = np.array(fixture['source_pixels'], dtype=np.float32).reshape(source['height'], source['width'])
    # Bilinear interpolation computed independently from the OpenCV source map.
    x, y = reference[native_valid].T
    x0, y0 = np.floor(x).astype(int), np.floor(y).astype(int)
    x1, y1 = np.minimum(x0+1, source['width']-1), np.minimum(y0+1, source['height']-1)
    dx, dy = x-x0, y-y0
    bilinear = ((1-dx)*(1-dy)*image[y0,x0] + dx*(1-dy)*image[y0,x1]
                + (1-dx)*dy*image[y1,x0] + dx*dy*image[y1,x1])
    native = np.array(fixture['intensity'])[native_valid]
    max_bilinear_error = float(np.max(np.abs(native-bilinear)))
    remapped = cv2.remap(image, mx, my, cv2.INTER_LINEAR).ravel()[native_valid]
    max_remap_error = float(np.max(np.abs(native-remapped)))
    # Allow a 1/32-pixel coefficient table used by some OpenCV backends/versions,
    # in addition to CV_32F map rounding. This image has a 2-unit/pixel gradient
    # per axis. Record the observed error rather than assume table quantization.
    thresholds = {'map_px': 1e-5, 'bilinear_intensity': 3e-5, 'opencv_quantized_intensity': 0.063}
    errors = {'map_px': max_map_error, 'bilinear_intensity': max_bilinear_error,
              'opencv_quantized_intensity': max_remap_error}
    passed = all(np.isfinite(e) and e <= thresholds[k] for k,e in errors.items())
    result = {'scope': 'synthetic undistortion/rotation only; not Raptor calibration or exposure qualification',
              'opencv_version': cv2.__version__, 'numpy_version': np.__version__,
              'valid_pixels': int(native_valid.sum()), 'invalid_pixels': int((~native_valid).sum()),
              'validity_mask_equal': True, 'max_error': errors, 'threshold': thresholds, 'pass': bool(passed)}
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
