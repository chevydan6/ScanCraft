#!/usr/bin/env python3
"""Optional SciPy diagnostic; never imported by the native application.
Uses deterministic synthetic cases and a separate stereographic parameterization.
Failures remain in the report and cause a nonzero exit; no cases are selected away.
"""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import time
import zlib
import numpy as np
import scipy
from scipy.optimize import least_squares


def frame(parameters):
    """Five parameters: two stereographic axis values, two offsets and radius."""
    s, t, cx, cy, radius = parameters
    denom = 1.0 + s*s + t*t
    axis = np.array([2*s, 2*t, 1-s*s-t*t]) / denom
    x, y, z = axis
    u = np.array([1-x*x/(1+z), -x*y/(1+z), -x])
    v = np.array([-x*y/(1+z), 1-y*y/(1+z), -y])
    return axis, u*cx + v*cy, radius, u, v


def residual(parameters, points):
    axis, center, radius, _, _ = frame(parameters)
    d = points - center
    radial = d - np.outer(d @ axis, axis)
    return np.linalg.norm(radial, axis=1) - radius


def reference(points):
    origin = points.mean(axis=0)
    scale = np.sqrt(np.mean(np.sum((points-origin)**2, axis=1)))
    p = (points-origin) / scale
    _, vectors = np.linalg.eigh(p.T @ p)
    axes = list(vectors.T) + [np.array(v, float) for v in
                              [(1, 1, 1), (1, -1, 1), (-1, 1, 1), (0.2, 0.7, 1)]]
    candidates = []
    diagnostics = []
    for axis in axes:
        axis = axis / np.linalg.norm(axis)
        if axis[2] < 0:
            axis = -axis
        s, t = axis[:2] / (1+axis[2])
        _, _, _, u, v = frame([s, t, 0, 0, 1])
        x, y = p @ u, p @ v
        solution = np.linalg.lstsq(np.column_stack([2*x, 2*y, np.ones(len(p))]),
                                   x*x+y*y, rcond=None)[0]
        radius = np.sqrt(max(1e-12, solution[2] + solution[0]**2 + solution[1]**2))
        start = np.array([s, t, solution[0], solution[1], radius])
        try:
            fit = least_squares(residual, start, args=(p,), jac='3-point', method='trf',
                                bounds=([-2, -2, -1e3, -1e3, 1e-8],
                                        [2, 2, 1e3, 1e3, 1e3]),
                                ftol=1e-13, xtol=1e-13, gtol=1e-13,
                                x_scale='jac', loss='linear', max_nfev=1500)
            singular = np.linalg.svd(fit.jac, compute_uv=False)
            full_rank = bool(singular[-1] > singular[0]*1e-8)
            correction = np.linalg.norm(np.linalg.lstsq(fit.jac, -fit.fun, rcond=1e-8)[0])
            stationary = bool(np.isfinite(correction) and correction <= 1e-7)
            diagnostics.append({'success': bool(fit.success), 'full_rank': full_rank,
                                'stationary': stationary, 'undamped_correction_norm': float(correction),
                                'cost': float(fit.cost), 'nfev': int(fit.nfev),
                                'optimality': float(fit.optimality)})
            if fit.success and full_rank and stationary:
                candidates.append(fit)
        except (ValueError, RuntimeError, np.linalg.LinAlgError) as error:
            diagnostics.append({'success': False, 'error': str(error)})
    if not candidates:
        raise RuntimeError('no converged full-rank independent reference start')
    fit = min(candidates, key=lambda f: f.cost)
    axis, center, radius, _, _ = frame(fit.x)
    return {'axis': axis, 'center': origin+scale*center, 'radius': radius*scale,
            'rms': np.sqrt(np.mean(fit.fun**2))*scale, 'starts': diagnostics}


def cases():
    rng = np.random.default_rng(20261026)
    specs = [('long_clean', 3., 24., 2*np.pi, 0., 'wall'),
             ('squat_clean', 3., 1., 2*np.pi, 0., 'wall'),
             ('full_radial_noise', 8., 18., 2*np.pi, .025, 'wall'),
             ('partial_90_noise', 5., 20., np.pi/2, .005, 'wall'),
             ('partial_45_noise', 5., 12., np.pi/4, .002, 'wall'),
             ('asymmetric_height', 4., 15., 1.7, .01, 'skew'),
             ('elliptic_form', 6., 12., 2*np.pi, .005, 'ellipse'),
             ('tapered_form', 6., 18., 2*np.pi, .005, 'taper')]
    output = []
    for index, (name, radius, height, arc, noise, form) in enumerate(specs):
        theta = np.tile(np.linspace(.2, .2+arc, 40, endpoint=False), 4)
        z = np.repeat(np.linspace(-height/2, height/2, 4), 40)
        if form == 'skew':
            z += .4*np.cos(2*theta) + .2*np.sin(theta)
        r = radius + rng.normal(0, noise, len(theta))
        if form == 'taper':
            r += .03*z
        x, y = r*np.cos(theta), r*np.sin(theta)
        if form == 'ellipse':
            x *= 1.04
            y *= .96
        p = np.column_stack([x, y, z])
        # A deterministic proper rotation independent of the native fitter.
        q, _ = np.linalg.qr(rng.normal(size=(3, 3)))
        if np.linalg.det(q) < 0:
            q[:, 0] *= -1
        p = p @ q.T + np.array([100+13*index, -70+3*index, 40-index])
        output.append({'name': name, 'points': p.tolist(), 'maximum_rms_mm': 2.0})
    return output


def compare(native, ref, points):
    axis = np.asarray(native['axis_direction'])
    center = np.asarray(native['axis_point_mm'])
    mean = points.mean(axis=0)
    native_closest = center + axis*np.dot(mean-center, axis)
    ref_closest = ref['center'] + ref['axis']*np.dot(mean-ref['center'], ref['axis'])
    angle = np.arctan2(np.linalg.norm(np.cross(axis, ref['axis'])),
                      abs(np.dot(axis, ref['axis'])))
    d = points-center
    rms_recomputed = np.sqrt(np.mean((np.linalg.norm(d-np.outer(d @ axis, axis), axis=1)
                                      - native['radius_mm'])**2))
    span = np.max(np.linalg.norm(points-mean, axis=1))
    metrics = {'axis_unit_norm_error': abs(float(np.linalg.norm(axis))-1.),
               'diameter_difference_mm': abs(native['diameter_mm']-2*ref['radius']),
               'axis_angle_radians': float(angle),
               'axis_point_at_centroid_difference_mm': float(np.linalg.norm(native_closest-ref_closest)),
               'rms_difference_mm': abs(native['geometric_rms_mm']-ref['rms']),
               'native_rms_recomputation_difference_mm': abs(native['geometric_rms_mm']-rms_recomputed)}
    limits = {'axis_unit_norm_error': 1e-8,
              'diameter_difference_mm': 1e-5*max(1., 2*ref['radius']),
              'axis_angle_radians': 1e-5,
              'axis_point_at_centroid_difference_mm': 1e-5*max(1., ref['radius'], span),
              'rms_difference_mm': 1e-7*max(1., ref['rms']),
              'native_rms_recomputation_difference_mm': 1e-7*max(1., span)}
    return metrics, limits, all(metrics[key] <= limit for key, limit in limits.items())


def sensitivity_checks(native, ref, points):
    baseline = compare(native, ref, points)[2]
    wrong_diameter = dict(native)
    wrong_diameter['diameter_mm'] += .1
    wrong_axis = dict(native)
    wrong_axis['axis_direction'] = [v*1.1 for v in native['axis_direction']]
    return {'baseline_passes': baseline,
            'wrong_diameter_rejected': not compare(wrong_diameter, ref, points)[2],
            'nonunit_axis_rejected': not compare(wrong_axis, ref, points)[2]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    parser.add_argument('--native', type=Path, default=Path('target/release/examples/cylinder_reference'))
    args = parser.parse_args()
    script_sha256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    native_sha256 = hashlib.sha256(args.native.read_bytes()).hexdigest()
    args.output.mkdir()
    request = {'schema_version': 1, 'units': 'millimeters', 'cases': cases()}
    data = json.dumps(request, allow_nan=False).encode()
    input_path = args.output/'inputs.json'
    input_path.write_bytes(data)
    start = time.monotonic()
    proc = subprocess.run([str(args.native.resolve()), str(input_path.resolve())],
                          capture_output=True, text=True, timeout=120, check=True)
    native_seconds = time.monotonic()-start
    (args.output/'native.json').write_text(proc.stdout)
    native = json.loads(proc.stdout)
    assert hashlib.sha256(args.native.read_bytes()).hexdigest() == native_sha256
    assert native['schema_version'] == 1 and native['units'] == 'millimeters'
    assert native['physical_accuracy_qualified'] is False
    assert native['input_crc32'] == zlib.crc32(data)
    assert len(native['cases']) == len(request['cases'])
    by_name = {r['name']: r for r in native['cases']}
    assert set(by_name) == {c['name'] for c in request['cases']}
    reports = []
    sensitivity = {}
    for case in request['cases']:
        points = np.asarray(case['points'])
        start = time.monotonic()
        try:
            ref = reference(points)
        except (ValueError, RuntimeError, np.linalg.LinAlgError) as error:
            reports.append({'name': case['name'], 'points': len(points),
                            'status': 'reference_failed', 'passed': False, 'error': str(error)})
            continue
        report = {'name': case['name'], 'points': len(points), 'reference_seconds': time.monotonic()-start,
                  'reference': {key: value.tolist() if isinstance(value, np.ndarray) else value
                                for key, value in ref.items()}}
        result = by_name[case['name']]
        if result['status'] != 'ok':
            report.update(status='native_rejected', error=result['error'], passed=False)
        else:
            metrics, limits, passed = compare(result['fit'], ref, points)
            if case['name'] == 'long_clean':
                sensitivity = sensitivity_checks(result['fit'], ref, points)
            report.update(status='compared', passed=passed, metrics=metrics, limits=limits)
        reports.append(report)
    summary = {'schema_version': 1, 'physical_accuracy_qualified': False,
               'scope': 'deterministic synthetic independent SciPy comparison; not NIST testing',
               'input_sha256': hashlib.sha256(data).hexdigest(),
               'native_binary_sha256': native_sha256, 'harness_sha256': script_sha256,
               'python': platform.python_version(), 'numpy': np.__version__, 'scipy': scipy.__version__,
               'native_seconds': native_seconds, 'passed': sum(r['passed'] for r in reports),
               'total': len(reports), 'sensitivity_checks': sensitivity, 'cases': reports}
    (args.output/'report.json').write_text(json.dumps(summary, indent=2, allow_nan=False)+'\n')
    sensitivity_passed = bool(sensitivity) and all(sensitivity.values())
    print(json.dumps({'passed': summary['passed'], 'total': summary['total'], 'sensitivity_passed': sensitivity_passed}))
    raise SystemExit(0 if summary['passed'] == summary['total'] and sensitivity_passed else 1)


if __name__ == '__main__':
    main()
