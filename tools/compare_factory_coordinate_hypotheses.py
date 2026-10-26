#!/usr/bin/env python3
"""Compare private decoded factory fields numerically; never activate calibration.

Only differences/aggregate residuals are printed. Raw factory values stay private.
Reflection here is a basis hypothesis, not a verified physical camera transform.
"""
import argparse
import json
import math
from pathlib import Path


def finite_array(values, length):
    if len(values) != length or not all(math.isfinite(v) for v in values):
        raise ValueError("unexpected nonfinite/numeric factory field shape")
    return values


def matrix_difference(a, b):
    return max(abs(x - y) for ar, br in zip(a, b) for x, y in zip(ar, br))


def compare(named, laser):
    if named.get("checksum_verified") is not True or laser.get("checksum_verified") is not True:
        raise ValueError("requires CRC-verified native CLI decoded fields")
    lenses = named["lenses"]
    if len(lenses) != 3:
        raise ValueError("requires three named lens slots")
    pose = named["second_times_inverse_first_raw_units"]
    if len(pose) != 4 or any(len(row) != 4 for row in pose) or pose[3] != [0, 0, 0, 1]:
        raise ValueError("unexpected named relative matrix")
    for row in pose:
        finite_array(row, 4)
    named_rotation = [row[:3] for row in pose[:3]]
    sign = [-1, 1, 1]
    records = []
    for index, record in enumerate(laser["records"]):
        if len(record["lenses"]) != 3:
            raise ValueError("requires three laser lens slots")
        if any((a["width"], a["height"]) != (b["width"], b["height"])
               for a, b in zip(record["lenses"], lenses)):
            continue  # Cropped modes are not assigned to full-frame named slots.
        raw = finite_array(record["transforms"][0]["matrix_values"], 9)
        rotation = [raw[i:i + 3] for i in (0, 3, 6)]
        translation = finite_array(record["transforms"][0]["translation_values"], 3)
        reflected = [[sign[i] * sign[j] * rotation[i][j] for j in range(3)] for i in range(3)]
        lens_metrics = []
        for a, b in zip(record["lenses"], lenses):
            x = finite_array(a["intrinsic_values"], 4)
            y = finite_array(b["intrinsic_values"], 4)
            ca = finite_array(a["coefficient_values"], 8)
            cb = finite_array(b["brown_values"], 8)
            lens_metrics.append({
                "cx_direct_abs_difference_pixels": abs(x[2] - y[2]),
                "cx_horizontal_reflection_abs_difference_pixels": abs(a["width"] - 1 - x[2] - y[2]),
                "cy_abs_difference_pixels": abs(x[3] - y[3]),
                "focal_max_relative_difference": max(abs(x[i] - y[i]) / max(abs(y[i]), 1e-30) for i in (0, 1)),
                "radial_coefficient_max_abs_difference": max(abs(ca[i] - cb[i]) for i in range(6)),
                "p1_abs_difference": abs(ca[6] - cb[6]),
                "p2_direct_abs_difference": abs(ca[7] - cb[7]),
                "p2_horizontal_reflection_abs_difference": abs(-ca[7] - cb[7]),
            })
        records.append({
            "record_index": index, "label": record["label"], "lens_slot_differences": lens_metrics,
            "rotation_direct_max_abs_difference": matrix_difference(rotation, named_rotation),
            "rotation_S_R_S_max_abs_difference": matrix_difference(reflected, named_rotation),
            "translation_direct_max_abs_difference_raw_units": max(abs(translation[i] - pose[i][3]) for i in range(3)),
            "translation_S_t_max_abs_difference_raw_units": max(abs(sign[i] * translation[i] - pose[i][3]) for i in range(3)),
        })
    return {"scope": "numerical relation of private decoded blocks; no physical calibration/mode binding",
            "reflection": "S=diag(-1,1,1), R'=S R S, t'=S t, cx'=width-1-cx, p2'=-p2",
            "reflection_is_a_proper_rotation": False, "raw_factory_values_distributed": False,
            "input_cli_crc_claims_checked_not_factory_bytes_reverified": True,
            "image_orientation_verified": False, "camera_assignment_verified": False,
            "translation_units_verified": False, "physical_accuracy_verified": False,
            "matching_full_size_records": records}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("named_fields", type=Path)
    parser.add_argument("laser_fields", type=Path)
    args = parser.parse_args()
    print(json.dumps(compare(json.loads(args.named_fields.read_text()), json.loads(args.laser_fields.read_text())), indent=2))


if __name__ == "__main__":
    main()
