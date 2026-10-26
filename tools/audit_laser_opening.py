#!/usr/bin/env python3
"""Replay fixed stripe-detector profiles on saved on/off dual recordings.

Prints aggregates only. Per-observation outputs must stay under evidence/private.
No new capture, device control, calibration conversion or geometry export.
"""
import argparse
import json
from pathlib import Path
import subprocess

PROFILES = [
    ("baseline", 30, 50, 12, None), ("wide", 30, 50, 48, None),
    ("low_global", 15, 30, 48, None), ("faint_global", 8, 16, 48, None),
    ("local_strict", 3, 10, 48, 32), ("local_faint", 1, 5, 48, 32),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cli", type=Path)
    parser.add_argument("on_directory", type=Path)
    parser.add_argument("off_directory", type=Path)
    parser.add_argument("new_private_directory", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    destination = args.new_private_directory.resolve()
    if not destination.is_relative_to(root / "evidence/private"):
        raise SystemExit("observation output must be under this repository's evidence/private")
    destination.mkdir(parents=True, exist_ok=False)
    reports = []
    for name, background, peak, width, opening in PROFILES:
        row = {"profile": name, "parameters": {"background": background,
               "minimum_peak": peak, "maximum_width": width, "saturation": 255,
               "maximum_observations": 100000, "opening_radius": opening}, "captures": {}}
        for mode, source in [("on", args.on_directory), ("off", args.off_directory)]:
            saved_pts = []
            for side, interface in [("left", 0), ("right", 2)]:
                command = [str(args.cli.resolve()), "detect-stripes", str(source / f"{side}.raptor"),
                           "30", str(background), str(peak), str(width), "255", "100000"]
                if opening is not None:
                    command += ["--opening-radius", str(opening)]
                output = subprocess.check_output(command, timeout=60)
                data = json.loads(output)
                if (data["source_integrity"] != "all frames and footer validated"
                        or not data["source_header_crc_protected"]
                        or data["profile"]["interface"] != interface
                        or data["selected_frame_index"] != 30
                        or data["source_total_frames"] != 60):
                    raise RuntimeError("unexpected source recording or integrity metadata")
                if opening is not None and data["saturation_units"] != "original raw 8-bit intensity":
                    raise RuntimeError("clipping convention missing")
                with (destination / f"{name}-{mode}-{side}.json").open("xb") as handle:
                    handle.write(output)
                saved_pts.append(data["sensor_pts"])
                row["captures"][f"{mode}_{side}"] = {
                    "accepted_observations": data["accepted_observations"],
                    "rejections": data["rejections"], "source_frames_verified": data["source_total_frames"],
                    "payload_crc32": data["payload_crc32"], "metadata_crc32": data["metadata_crc32"],
                }
            if saved_pts[0] != saved_pts[1]:
                raise RuntimeError("saved sensor PTS differ between cameras")
        reports.append(row)
    print(json.dumps({"scope": "offline detector comparison on saved source frame 30",
                      "whole_source_integrity_verified_each_run": True,
                      "equal_saved_sensor_pts_per_pair": True, "exposure_synchronization_verified": False,
                      "physical_accuracy_verified": False, "raw_samples_unchanged": True,
                      "source_capture_state_unchanged": True, "profiles": reports}, indent=2))


if __name__ == "__main__":
    main()
