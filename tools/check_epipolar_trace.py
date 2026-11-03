#!/usr/bin/env python3
"""Synthetic known-point reference for native normalized-polyline intersections."""
import argparse
import json
from pathlib import Path
import subprocess

import numpy as np


def camera():
    return {"width": 1280, "height": 800, "fx": 1000., "fy": 980.,
            "cx": 639.5, "cy": 399.5, "skew": 1.3, "valid_radius": 1.5,
            "distortion": {"k1": -.04, "k2": .01, "k3": .0002,
                           "p1": .0001, "p2": -.0002}}


def project(model, point):
    x, y = np.array(point[:2])/point[2]
    k = model["distortion"]
    radius = x*x+y*y
    radial = 1+k["k1"]*radius+k["k2"]*radius**2+k["k3"]*radius**3
    xd = x*radial+2*k["p1"]*x*y+k["p2"]*(radius+2*x*x)
    yd = y*radial+k["p1"]*(radius+2*y*y)+2*k["p2"]*x*y
    return [model["fx"]*xd+model["skew"]*yd+model["cx"],
            model["fy"]*yd+model["cy"]]


def check(example):
    random = np.random.default_rng(97241)
    maximum_pixel_error = maximum_fraction_error = 0.
    for case in range(32):
        angle = random.uniform(-.04, .04)
        c, s = np.cos(angle), np.sin(angle)
        rotation = np.array([[c, 0., s], [0., 1., 0.], [-s, 0., c]])
        translation = np.array([-80., 3., 6.])
        point = np.array([random.uniform(-80, 80), random.uniform(-60, 60),
                          random.uniform(350, 900)])
        other = rotation@point+translation
        left, right = camera(), camera()
        right["fx"] += case
        right["fy"] -= case
        # A segment constructed about the independently known projected point.
        normal = np.cross(translation, rotation@point)[:2]
        direction = normal/np.linalg.norm(normal)*.003
        center = other[:2]/other[2]
        trace = [project(right, [*(center-direction), 1.]),
                 project(right, [*(center+3*direction), 1.])]
        request = {"calibration": {"left": left, "right": right,
                                  "right_from_left": {"rotation": rotation.tolist(),
                                                      "translation_mm": translation.tolist()}},
                   "first_pixel": project(left, point), "right_trace": trace,
                   "config": {"maximum_segment_length_px": 64.,
                              "normalized_tolerance": 1e-10, "minimum_plane_sine": 1e-8}}
        run = subprocess.run([str(example)], input=json.dumps(request).encode(),
                             capture_output=True, check=True, timeout=60)
        report = json.loads(run.stdout)
        assert report["physical_accuracy_qualified"] is False
        assert not report["result"]["ambiguous_segments"]
        assert len(report["result"]["intersections"]) == 1
        hit = report["result"]["intersections"][0]
        assert hit["vertex_index"] is None
        pixel_error = float(np.linalg.norm(np.array(hit["right_pixel"])-project(right, other)))
        fraction_error = abs(hit["fraction"]-.25)
        assert pixel_error < 1e-6 and fraction_error < 1e-8
        maximum_pixel_error = max(maximum_pixel_error, pixel_error)
        maximum_fraction_error = max(maximum_fraction_error, fraction_error)
    return {"synthetic_only": True, "cases_passed": 32,
            "maximum_pixel_error": maximum_pixel_error,
            "maximum_fraction_error": maximum_fraction_error,
            "physical_accuracy_qualified": False, "numpy": np.__version__}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("example", type=Path)
    args = parser.parse_args()
    print(json.dumps(check(args.example.resolve()), indent=2))
