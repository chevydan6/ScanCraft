#!/usr/bin/env python3
"""Known-3D synthetic reference for match_laser_traces; Python standard library.

Projection is evaluated here independently. Exact known points, rather than the
native triangulator's residuals, provide the expected reconstructed positions.
No scanner model or recorded private data is included.
"""
import argparse
import itertools
import json
import math
import subprocess


def project(camera, point):
    x, y = point[0] / point[2], point[1] / point[2]
    r2 = x * x + y * y
    d = camera["distortion"]
    radial = 1 + d["k1"] * r2 + d["k2"] * r2**2 + d["k3"] * r2**3
    xd = x * radial + 2 * d["p1"] * x * y + d["p2"] * (r2 + 2 * x * x)
    yd = y * radial + d["p1"] * (r2 + 2 * y * y) + 2 * d["p2"] * x * y
    return [camera["fx"] * xd + camera["skew"] * yd + camera["cx"],
            camera["fy"] * yd + camera["cy"]]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/examples/match_laser_traces")
    args = parser.parse_args()
    count, max_position_error, max_pixel_error = 0, 0.0, 0.0
    for scale, angle, distortion, y in itertools.product(
            [0.5, 1.0, 2.0], [0.0, 0.03], [False, True], [-20.0, 10.0, 30.0]):
        camera = {"width": 640, "height": 480, "fx": 500.0, "fy": 510.0,
                  "cx": 320.0, "cy": 240.0, "skew": 2.0, "valid_radius": 1.0,
                  "distortion": {"k1": 0.05 if distortion else 0.0,
                                 "k2": -0.002 if distortion else 0.0,
                                 "p1": 0.001 if distortion else 0.0,
                                 "p2": -0.001 if distortion else 0.0,
                                 "k3": 0.0001 if distortion else 0.0}}
        second = {**camera, "skew": -3.0}
        rotation = [[math.cos(angle), 0, math.sin(angle)], [0, 1, 0],
                    [-math.sin(angle), 0, math.cos(angle)]]
        translation = [-50 * scale, 2 * scale, -scale]

        def transform(point):
            return [sum(rotation[i][j] * point[j] for j in range(3)) + translation[i]
                    for i in range(3)]

        point = [20 * scale, y * scale, 500 * scale]
        traces = []
        # Three points on the SAME first-camera ray, at distinct depths. Each
        # right trace crosses the epipolar line. Sheet support must select depth 1.
        for ratio in [0.8, 1.0, 1.3]:
            center = [v * ratio for v in point]
            traces.append([project(second, transform([center[0], center[1] + dy * scale, center[2]]))
                           for dy in [-2, 2]])
        correct = 1
        if count % 2:
            traces = [traces[1], traces[2], traces[0]]
            correct = 0
        request = {
            "calibration": {"left": camera, "right": second,
                            "right_from_left": {"rotation": rotation,
                                                "translation_mm": translation}},
            "limits": {"min_depth_z_mm": 100 * scale, "max_depth_z_mm": 1000 * scale,
                       "min_ray_angle_rad": 0.0001, "max_ray_gap_mm": 0.01 * scale,
                       "max_reprojection_error_px": 0.01},
            "first_pixel": project(camera, point), "right_traces": traces,
            "sheets": [{"quadratic_y": 0, "linear_y": 0, "linear_z": 0, "offset": point[0]}],
            "config": {"intersection": {"maximum_segment_length_px": 16,
                                        "normalized_tolerance": 1e-10, "minimum_plane_sine": 1e-8},
                       "maximum_sheet_distance_px": 1, "maximum_work": 1000}}
        run = subprocess.run([args.binary], input=json.dumps(request), text=True,
                             capture_output=True, check=True)
        result = json.loads(run.stdout)["result"]
        assert result["unique"] == {"trace_index": correct, "crossing_index": 0, "support_index": 0}
        assert all(len(trace["crossings"]) == 1 for trace in result["traces"])
        crossing = result["traces"][correct]["crossings"][0]
        recovered = crossing["triangulation"]["Ok"]["position_mm"]
        position_error = math.dist(recovered, point)
        pixel_error = math.dist(crossing["intersection"]["right_pixel"], project(second, transform(point)))
        assert position_error < 1e-6 * scale, (request, result, position_error)
        assert pixel_error < 1e-7, (request, result, pixel_error)
        max_position_error = max(max_position_error, position_error)
        max_pixel_error = max(max_pixel_error, pixel_error)
        count += 1
    print(json.dumps({"synthetic_cases": count, "max_position_error": max_position_error,
                      "max_pixel_error": max_pixel_error, "physical_accuracy_qualified": False}, indent=2))


if __name__ == "__main__":
    main()
