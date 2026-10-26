#!/usr/bin/env python3
"""Fixed held-out-frame mirror audit using native replay and hypothesis tools.
Per-observation/complete hypothesis outputs stay private; prints aggregates.
No capture, setting changes, calibration activation or geometry export.
"""
import argparse
import json
from pathlib import Path
import subprocess

PROFILES = [("baseline", 30, 50, 12, None), ("local_strict", 3, 10, 48, 32)]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("cli", type=Path)
    p.add_argument("diagnostic", type=Path)
    p.add_argument("factory", type=Path)
    p.add_argument("recording_directory", type=Path)
    p.add_argument("new_private_directory", type=Path)
    args = p.parse_args()
    root = Path(__file__).resolve().parents[1]
    destination = args.new_private_directory.resolve()
    if not destination.is_relative_to(root / "evidence/private"):
        raise SystemExit("observation output must remain under evidence/private")
    destination.mkdir(parents=True, exist_ok=False)
    results = []
    identities = {}
    for frame in [10, 50]:
        for name, background, peak, width, opening in PROFILES:
            paths, observations = [], []
            for side, interface in [("left", 0), ("right", 2)]:
                command = [str(args.cli.resolve()), "detect-stripes", str(args.recording_directory / f"{side}.raptor"),
                           str(frame), str(background), str(peak), str(width), "255", "100000"]
                if opening is not None:
                    command += ["--opening-radius", str(opening)]
                output = subprocess.check_output(command, timeout=60)
                data = json.loads(output)
                if (data["source_integrity"] != "all frames and footer validated"
                        or not data["source_header_crc_protected"] or data["source_total_frames"] != 60
                        or data["selected_frame_index"] != frame or data["profile"]["interface"] != interface):
                    raise RuntimeError("unexpected source/integrity metadata")
                identity = [data["payload_crc32"], data["metadata_crc32"], data["sensor_pts"]]
                key = (frame, side)
                if identities.setdefault(key, identity) != identity:
                    raise RuntimeError("source frame changed across detector profiles")
                path = destination / f"{frame}-{name}-{side}.json"
                with path.open("xb") as handle:
                    handle.write(output)
                paths.append(path)
                observations.append(data)
            if observations[0]["sensor_pts"] != observations[1]["sensor_pts"]:
                raise RuntimeError("saved camera PTS differ")
            for mirrored in [False, True]:
                command = [str(args.diagnostic.resolve()), str(args.factory), *map(str, paths)]
                if mirrored:
                    command += ["--mirror-x"]
                output = subprocess.check_output(command, timeout=60)
                data = json.loads(output)
                if data["source_payload_crc32"] != [d["payload_crc32"] for d in observations]:
                    raise RuntimeError("hypothesis/source lineage mismatch")
                with (destination / f"{frame}-{name}-mirror-{mirrored}.json").open("xb") as handle:
                    handle.write(output)
                results.append({
                    "source_frame_index": frame, "detector_profile": name, "both_inputs_mirrored_x": mirrored,
                    "observations": [d["accepted_observations"] for d in observations],
                    "source_payload_crc32": data["source_payload_crc32"],
                    "comparisons": data["comparisons"],
                    "records": [{"record_offset": r["record_offset"], "label": r["label"],
                                 "first_camera_input_interface": r["first_camera_input_interface"],
                                 "predictions": r["predictions_in_second_camera_image"],
                                 "greedy_pairs_by_shift": [s["greedy_one_to_one_pairs"] for s in r["supports_by_shift"]],
                                 "supported_predictions_by_shift": [s["supported_predictions"] for s in r["supports_by_shift"]]}
                                for r in data["records"]],
                })
    print(json.dumps({"scope": "fixed mirror hypotheses on held-out saved frames 10/50; frame 30 was previously inspected",
                      "detector_profiles_retuned": False, "orientation_applied_to_observations_only": True,
                      "shift_controls_pixels": [0, 200, 400, 600], "source_integrity_verified_each_replay": True,
                      "source_frame_identity_checked_across_profiles": True,
                      "scene_is_independent_physical_ground_truth": False,
                      "exposure_synchronization_verified": False, "camera_orientation_mode_units_verified": False,
                      "physical_accuracy_verified": False, "calibration_activated": False,
                      "results": results}, indent=2))


if __name__ == "__main__":
    main()
