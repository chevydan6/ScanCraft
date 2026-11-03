#!/usr/bin/env python3
"""Independent float64 ZNCC and polynomial-fit oracle for synthetic stereo fixtures.
No hardware, vendor binaries or scanner calibration is accessed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import numpy as np


def normalized(image, point, radius):
    x, y = point
    height, width = image.shape
    assert radius <= x < width-radius and radius <= y < height-radius
    values = image[y-radius:y+radius+1, x-radius:x+radius+1].ravel().copy()
    values -= np.mean(values)
    length = np.linalg.norm(values)
    assert length > 0
    return values / length


def fit(template, image, center, radius):
    x, y = center
    scores = np.array([np.dot(template, normalized(image, (x+dx, y), radius)) for dx in (-1, 0, 1)])
    # Independent least-squares quadratic interpolant rather than Rust's closed form.
    a, b, _ = np.polyfit([-1., 0., 1.], scores, 2)
    assert a < 0
    return -b/(2*a), -2*a


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('fixture', type=Path)
    parser.add_argument('report', type=Path)
    args = parser.parse_args()
    payload = args.fixture.read_bytes()
    fixture = json.loads(payload)
    assert fixture['synthetic_only'] is True
    cases = []
    for case in fixture['cases']:
        shape = (case['rig']['left']['height'], case['rig']['left']['width'])
        left = np.asarray(case['left'], dtype=np.float32).astype(np.float64).reshape(shape)
        right = np.asarray(case['right'], dtype=np.float32).astype(np.float64).reshape(shape)
        cfg = case['refined']['config']
        radius = cfg['patch_radius']
        assert not case['refined']['rejected']
        errors, coarse_errors, reference_errors = [], [], []
        for integer, result in zip(case['coarse']['accepted'], case['refined']['accepted'], strict=True):
            l, r = result['left_pixel'], result['right_pixel']
            assert integer['right_pixel'] == r
            template = normalized(left, l, radius)
            candidates = []
            for d in range(cfg['min_disparity'], cfg['max_disparity']+1):
                q = (l[0]-d, l[1])
                candidates.append((float(np.dot(template, normalized(right, q, radius))), q))
            assert max(candidates, key=lambda c: c[0])[1] == tuple(r)
            reverse_template = normalized(right, r, radius)
            reverse_candidates = [(float(np.dot(reverse_template, normalized(left, (r[0]+d, r[1]), radius))), (r[0]+d, r[1]))
                                  for d in range(cfg['min_disparity'], cfg['max_disparity']+1)]
            assert max(reverse_candidates, key=lambda c: c[0])[1] == tuple(l)
            forward, fc = fit(template, right, r, radius)
            reverse, rc = fit(normalized(right, r, radius), left, l, radius)
            actual = result['subpixel']
            expected = [forward, reverse, fc, rc]
            values = [actual[k] for k in ('forward_offset_px', 'reverse_offset_px', 'forward_curvature', 'reverse_curvature')]
            np.testing.assert_allclose(values, expected, atol=2e-10, rtol=2e-10)
            np.testing.assert_allclose(actual['right_pixel'], [r[0]+forward, r[1]], atol=2e-10, rtol=0)
            truth = l[0]-case['disparity']
            errors.append(abs(actual['right_pixel'][0]-truth))
            coarse_errors.append(abs(r[0]-truth))
            reference_errors.append(float(np.max(np.abs(np.asarray(values)-expected))))
        cases.append({'disparity': case['disparity'], 'accepted': len(errors),
                      'maximum_refined_pixel_error': max(errors),
                      'maximum_integer_pixel_error': max(coarse_errors),
                      'maximum_reference_difference': max(reference_errors)})
    report = {'scope': 'synthetic fractional image correlation/fit only; no physical accuracy qualification',
              'numpy_version': np.__version__, 'fixture_sha256': hashlib.sha256(payload).hexdigest(),
              'cases': cases, 'passed': True}
    args.report.write_text(json.dumps(report, indent=2) + chr(10))
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
