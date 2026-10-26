#!/usr/bin/env python3
"""Offline fixed feature-pair orientation hypotheses, no scanner access."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

import cv2
import numpy as np

from check_factory_epipolar import check


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("native_cli", type=Path)
    parser.add_argument("named_fields", type=Path)
    parser.add_argument("laser_fields", type=Path)
    parser.add_argument("left_frame_directory", type=Path)
    parser.add_argument("right_frame_directory", type=Path)
    args = parser.parse_args()
    # Re-export from integrity-checked single-frame wrappers, not just manifests.
    for directory in [args.left_frame_directory, args.right_frame_directory]:
        with tempfile.TemporaryDirectory(prefix="scancraft-feature-replay-") as temporary:
            image_path = Path(temporary) / "replayed.png"
            output = subprocess.check_output([str(args.native_cli.resolve()), "export-image",
                                              str(directory / "frame.raptor"), str(image_path)], timeout=60)
            replay = json.loads(output)
            manifest = json.loads((directory / "manifest.json").read_text())
            if replay["integrity"] != "PASS" or replay["frames"] != 1 or replay["profile"] != manifest["profile"]:
                raise RuntimeError("unexpected saved-frame wrapper integrity or profile")
            saved = cv2.imread(str(directory / "image.png"), cv2.IMREAD_GRAYSCALE)
            decoded = cv2.imread(str(image_path), cv2.IMREAD_GRAYSCALE)
            if saved is None or decoded is None or not np.array_equal(saved, decoded):
                raise RuntimeError("feature image differs from native decoded recording")
    results = []
    identity = None
    for source, path, index in [("named", args.named_fields, None),
                                ("laser_record_0", args.laser_fields, 0)]:
        for swap in [False, True]:
            for mirror in [False, True]:
                report = check(path, args.left_frame_directory, args.right_frame_directory,
                               index, swap, mirror)
                current = [report[k] for k in ["image_sha256", "keypoints", "mutual_matches"]]
                if identity is None:
                    identity = current
                if current != identity:
                    raise RuntimeError("image or feature count changed across geometry hypotheses")
                results.append({"source": source, "report": report})
    print(json.dumps({"scope": "image-only feature matching; fixed camera-order/reflection hypotheses",
                      "runtime_conventions_verified": False, "physical_accuracy_verified": False,
                      "source_single_frame_wrappers_integrity_replayed": True,
                      "feature_images_equal_native_decoded_wrappers": True,
                      "original_multiframe_source_reverified_by_this_tool": False,
                      "images_and_feature_counts_equal_across_hypotheses": True,
                      "calibration_activated": False, "results": results}, indent=2))


if __name__ == "__main__":
    main()
