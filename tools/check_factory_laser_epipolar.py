#!/usr/bin/env python3
"""Compare synthetic epipolar probe outputs with independent scalar algebra."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

SHA256 = "1eeafa48017e8b18deb3bd01fe49e9652f3a810a2a64a7eb01bb26eccdc3402a"


def multiply(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def transpose(a):
    return [list(row) for row in zip(*a)]


def inverse_camera(k):
    fx, fy, cx, cy = k
    return [[1 / fx, 0, -cx / fx], [0, 1 / fy, -cy / fy], [0, 0, 1]]


def error(a, b):
    return max(abs(x - y) for ar, br in zip(a, b) for x, y in zip(ar, br))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dll", type=Path)
    parser.add_argument("private_directory", type=Path)
    args = parser.parse_args()
    if hashlib.sha256(args.dll.read_bytes()).hexdigest() != SHA256:
        raise SystemExit("reference DLL hash mismatch")
    results = []
    for index in range(6):
        data = (args.private_directory / f"laser-epipolar-{index}.bin").read_bytes()
        if len(data) != 152:
            raise SystemExit("reference output size mismatch")
        values = struct.unpack("<38f", data)
        if not all(abs(v) < float("inf") for v in values):
            raise SystemExit("nonfinite reference value")
        rotation = [values[j:j + 3] for j in (8, 11, 14)]
        x, y, z = values[17:20]
        essential = multiply([[0, -z, y], [z, 0, -x], [-y, x, 0]], rotation)
        fundamental = multiply(multiply(transpose(inverse_camera(values[4:8])), essential), inverse_camera(values[:4]))
        e_error = error(essential, [values[j:j + 3] for j in (20, 23, 26)])
        f_error = error(fundamental, [values[j:j + 3] for j in (29, 32, 35)])
        if e_error >= 3e-6 or f_error >= 4e-6:
            raise SystemExit(f"formula mismatch in case {index}")
        results.append({"case": index, "essential_max_abs_error": e_error,
                        "fundamental_max_abs_error": f_error})
    print(json.dumps({"reference_sha256": SHA256, "entry_rva": "0x75f0",
                      "stop_rva": "0x7ba1", "synthetic_only": True,
                      "factory_data_used_in_reference": False,
                      "matrix_storage": "row-major", "essential_formula": "[t]x * R",
                      "fundamental_formula": "inverse(K2).transpose() * E * inverse(K1)",
                      "cases": results, "physical_camera_assignment_verified": False,
                      "image_mode_binding_verified": False, "translation_units_verified": False,
                      "laser_coefficient_formula_verified": False}, indent=2))


if __name__ == "__main__":
    main()
