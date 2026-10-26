#!/usr/bin/env python3
"""Synthetic ZNCC matching and triangulation reference; no production dependency."""
import argparse
import json
from pathlib import Path
import cv2
import numpy as np


def matrix(c):
    return np.array([[c['fx'], 0, c['cx']], [0, c['fy'], c['cy']], [0, 0, 1]], dtype=np.float64)


def ranked(source, target, center, direction, cfg):
    x, y = center
    r = cfg['patch_radius']
    template = source[y-r:y+r+1, x-r:x+r+1]
    candidates = []
    for d in range(cfg['min_disparity'], cfg['max_disparity']+1):
        tx = x+direction*d
        if tx-r < 0 or tx+r >= target.shape[1]:
            continue
        patch = target[y-r:y+r+1, tx-r:tx+r+1]
        if float(patch.std()) < cfg['min_patch_stddev']:
            continue
        score = float(cv2.matchTemplate(patch, template, cv2.TM_CCOEFF_NORMED)[0, 0])
        candidates.append((score, tx))
    candidates.sort(reverse=True)
    assert len(candidates) >= 2
    return candidates


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('fixture', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    f = json.loads(args.fixture.read_text())
    assert f['provenance'] == 'synthetic_fixture_not_scanner_calibration'
    rig, cfg = f['rig'], f['matching']['config']
    shape = (rig['left']['height'], rig['left']['width'])
    left = np.array(f['left_image'], dtype=np.float32).reshape(shape)
    right = np.array(f['right_image'], dtype=np.float32).reshape(shape)
    assert not f['matching']['rejected'] and not f['reconstruction']['rejected']
    accepted = f['matching']['accepted']
    assert len(accepted) == len(f['seeds']) and len(accepted) > 0
    points = f['reconstruction']['accepted']
    assert len(points) == len(accepted)
    p_left = matrix(rig['left']) @ np.column_stack([np.eye(3), np.zeros(3)])
    pose = rig['right_from_left']
    p_right = matrix(rig['right']) @ np.column_stack([np.array(pose['rotation']), pose['translation_mm']])
    score_errors, geometry_errors, truth_errors, disparities = [], [], [], []
    for i, (m, point) in enumerate(zip(accepted, points)):
        assert m['seed_index'] == i and point['source_index'] == i
        seed = f['seeds'][i]
        expected_disparity = 4 if seed[1] < 16 else 8
        disparities.append(expected_disparity)
        expected_right = [seed[0]-expected_disparity, seed[1]]
        assert m['left_pixel'] == seed and m['right_pixel'] == expected_right
        forward = ranked(left, right, seed, -1, cfg)
        reverse = ranked(right, left, expected_right, 1, cfg)
        assert forward[0][1] == expected_right[0] and reverse[0][1] == seed[0]
        for ranking in [forward, reverse]:
            assert ranking[0][0] >= cfg['min_correlation']
            assert ranking[0][0]-ranking[1][0] >= cfg['min_uniqueness_margin']
        score_errors.extend([abs(forward[0][0]-m['correlation']), abs(forward[1][0]-m['second_correlation']),
                             abs(reverse[0][0]-m['reverse_correlation']), abs(reverse[1][0]-m['reverse_second_correlation'])])
        homogeneous = cv2.triangulatePoints(p_left, p_right,
                                            np.array(seed, dtype=np.float64).reshape(2, 1),
                                            np.array(expected_right, dtype=np.float64).reshape(2, 1)).ravel()
        ref_point = homogeneous[:3]/homogeneous[3]
        native = np.array(point['geometry']['position_mm'])
        z = 80*20/expected_disparity
        truth = np.array([(seed[0]-31.5)*z/80, (seed[1]-15.5)*z/80, z])
        geometry_errors.append(float(np.linalg.norm(native-ref_point)))
        truth_errors.append(float(np.linalg.norm(native-truth)))
    errors = {'zncc_score': max(score_errors), 'triangulation_mm': max(geometry_errors), 'ground_truth_mm': max(truth_errors)}
    thresholds = {'zncc_score': 2e-6, 'triangulation_mm': 1e-6, 'ground_truth_mm': 1e-6}
    passed = all(np.isfinite(e) and e <= thresholds[k] for k,e in errors.items())
    result = {'scope': 'synthetic rectified two-depth images; not physical Raptor accuracy or mode qualification',
              'opencv_version': cv2.__version__, 'numpy_version': np.__version__, 'matched_seeds': len(accepted),
              'disparities_px': sorted(set(disparities)), 'matching_winners_equal': True,
              'max_error': errors, 'threshold': thresholds, 'pass': bool(passed)}
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
