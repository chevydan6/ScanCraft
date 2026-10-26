#!/usr/bin/env python3
"""Compare native statistical filtering with independent brute-force NumPy distances."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import numpy as np


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=Path('target/release/scancraft-cli'))
    args = parser.parse_args()
    args.output.mkdir()
    grid = [[x, y, 0.] for x in range(10) for y in range(10)] + [[100., 100., 100.], [-100., -100., -100.]]
    anisotropic = [[i*.01, (i*17 % 11)*.2, (i*7 % 13)*.03] for i in range(100)] + [[9., 9., 9.]]
    mixed = [[i*.1, j*.1, 0.] for i in range(5) for j in range(5)] + [[2., 2., 0.], [4., 4., 0.], [6., 6., 0.]]
    cases = [('analytic', [[0., 0., 0.], [1., 0., 0.], [10., 0., 0.]], 1, 1.),
             ('labeled_grid', grid, 4, 1.), ('coincident', [[0., 0., 0.]]*8, 7, 0.),
             ('tied_cube', [[x, y, z] for x in (0., 1.) for y in (0., 1.) for z in (0., 1.)], 3, 0.),
             ('anisotropic', anisotropic, 8, 1.5), ('mixed_density_not_noise_truth', mixed, 3, 1.)]
    checks = []
    for name, points, k, sigma in cases:
        source = args.output/(name+'.ply')
        header = f'ply\nformat ascii 1.0\ncomment units millimeters\nelement vertex {len(points)}\nproperty double x\nproperty double y\nproperty double z\nproperty double nx\nproperty double ny\nproperty double nz\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nend_header\n'
        source.write_text(header+'\n'.join(' '.join(map(str, [*p, 0., 0., 1., 80, 180, 240])) for p in points)+'\n')
        before = source.read_bytes()
        result = args.output/(name+'-result')
        command = [str(args.binary.resolve()), 'filter-statistical', str(source.resolve()), str(k), str(sigma), str(result.resolve())]
        output = subprocess.run(command, capture_output=True, text=True)
        (args.output/(name+'.stderr')).write_text(output.stderr)
        output.check_returncode()
        assert source.read_bytes() == before
        report = json.loads((result/'report.json').read_text())
        actual = report['statistics']
        p = np.asarray(points)
        distances = np.linalg.norm(p[:, None, :]-p[None, :, :], axis=2)
        np.fill_diagonal(distances, np.inf)
        means = np.mean(np.sort(distances, axis=1)[:, :k], axis=1)
        mean, deviation = float(np.mean(means)), float(np.std(means, ddof=1))
        threshold = mean+sigma*deviation
        outliers = np.flatnonzero(means > threshold).tolist()
        np.testing.assert_allclose(actual['mean_neighbor_distance_mm'], means, rtol=2e-12, atol=1e-12)
        np.testing.assert_allclose([actual['global_mean_mm'], actual['sample_standard_deviation_mm'], actual['threshold_mm']], [mean, deviation, threshold], rtol=2e-12, atol=1e-12)
        assert actual['outlier_indices'] == outliers
        checks.append({'name': name, 'points': len(points), 'k': k, 'sigma': sigma, 'outlier_indices': outliers,
                       'maximum_mean_distance_difference_mm': float(np.max(np.abs(means-actual['mean_neighbor_distance_mm']))),
                       'threshold_mm': threshold, 'input_bytes_unchanged': True, 'classification_match': True})
    report = {'schema_version': 1, 'scope': 'six synthetic exact k-distance statistical comparisons; classification is not physical noise ground truth',
              'passed': True, 'physical_accuracy_qualified': False, 'python': platform.python_version(), 'numpy': np.__version__,
              'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'cases': checks}
    (args.output/'result.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
