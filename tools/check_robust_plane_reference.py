#!/usr/bin/env python3
"""Synthetic fixed-correspondence Huber fit vs independent SciPy nonlinear solve."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import numpy as np
import scipy
from scipy.optimize import least_squares
from scipy.spatial.transform import Rotation


def objective(residual, delta):
    r = np.abs(residual)
    return float(np.sum(np.where(r <= delta, 0.5*r*r, delta*(r-0.5*delta))))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=Path('target/release/examples/robust_plane_reference'))
    args = parser.parse_args()
    args.output.mkdir()
    target, normals = [], []
    for a in [-6., -2., 2., 6.]:
        for b in [-5., 0., 5.]:
            target.extend([[30., a, b], [a, 40., b], [a, b, 50.]])
            normals.extend(np.eye(3).tolist())
    target, normals = np.array(target), np.array(normals)
    rotation = Rotation.from_rotvec([.02, -.015, .025]).as_matrix()
    source = (target - [.3, -.2, .4]) @ rotation
    cases = []
    for name, delta, contamination, normal_scale in [
        ('clean', .02, 0., 1.), ('small_scale', .005, 6., 1.),
        ('unit_test_scale', .02, 6., 1.), ('large_scale', .1, 6., 1.),
        ('reversed_scaled_normals', .02, 6., -7.), ('negative_outliers', .02, -6., 1.)
    ]:
        q = target.copy()
        q[::9] += contamination * normals[::9]
        cases.append({'name': name, 'source': source.tolist(), 'target': q.tolist(),
                      'normals': (normals*normal_scale).tolist(), 'delta_mm': delta, 'iterations': 60})
    request = args.output/'request.json'
    request.write_text(json.dumps({'schema_version': 1, 'units': 'millimeters', 'cases': cases}, indent=2)+'\n')
    output = subprocess.run([str(args.binary.resolve()), str(request.resolve())], capture_output=True, text=True)
    (args.output/'native.stderr').write_text(output.stderr)
    output.check_returncode()
    native = json.loads(output.stdout)
    (args.output/'native.json').write_text(json.dumps(native, indent=2)+'\n')
    checks = []
    for case, actual in zip(cases, native['cases'], strict=True):
        assert actual['name'] == case['name']
        p, q, n = (np.asarray(case[k]) for k in ('source', 'target', 'normals'))
        n = n/np.linalg.norm(n, axis=1)[:, None]
        def residual(x):
            predicted = p @ Rotation.from_rotvec(x[:3]).as_matrix().T + x[3:]
            return np.sum((predicted-q)*n, axis=1)
        reference = least_squares(residual, np.zeros(6), loss='huber', f_scale=case['delta_mm'],
                                  jac='3-point', xtol=1e-13, ftol=1e-13, gtol=1e-13, max_nfev=10000)
        assert reference.success
        expected = p @ Rotation.from_rotvec(reference.x[:3]).as_matrix().T + reference.x[3:]
        predicted = p @ np.asarray(actual['rotation']).T + actual['translation_mm']
        point_error = float(np.max(np.linalg.norm(predicted-expected, axis=1)))
        costs = [objective(np.sum((p-q)*n, axis=1), case['delta_mm'])]
        for step in actual['trace']:
            points = p @ np.asarray(step['rotation']).T + step['translation_mm']
            costs.append(objective(np.sum((points-q)*n, axis=1), case['delta_mm']))
        assert all(b <= a + 1e-10*a + 1e-25 for a, b in zip(costs, costs[1:]))
        assert point_error < 2e-5, (case['name'], point_error)
        assert abs(costs[-1]-reference.cost) < 1e-8
        checks.append({'name': case['name'], 'delta_mm': case['delta_mm'], 'points': len(p),
                       'maximum_native_reference_point_difference_mm': point_error,
                       'native_cost_mm2': costs[-1], 'scipy_cost_mm2': float(reference.cost),
                       'nonincreasing_fixed_correspondence_cost': True,
                       'clean_point_rms_bias_mm': float(np.sqrt(np.mean(np.sum((predicted-target)**2, axis=1)))),
                       'native_iterations': actual['iterations'], 'scipy_evaluations': reference.nfev})
    report = {'schema_version': 1, 'scope': 'six synthetic fixed-correspondence local Huber fits; no nearest-neighbor/global/hardware qualification',
              'passed': True, 'physical_accuracy_qualified': False, 'python': platform.python_version(),
              'numpy': np.__version__, 'scipy': scipy.__version__, 'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'cases': checks}
    (args.output/'result.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
