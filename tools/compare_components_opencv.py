#!/usr/bin/env python3
"""Compare native component observations with OpenCV on the identical GREY payload.

Offline reference only. Input report must come from a complete native recording
replay; this tool binds PNG pixels to its payload CRC but does not reverify the
recording or establish physical provenance. Never upload private input images.
"""
import argparse
import hashlib
import json
from pathlib import Path
import zlib

import cv2
import numpy as np


def compare(image_path, report_path):
    report = json.loads(report_path.read_text())
    assert report["schema_version"] == 1
    assert report["source_integrity"] == "all frames and footer validated"
    assert report["source_header_crc_protected"] is True
    profile, options = report["profile"], report["parameters"]
    assert profile["fourcc"] == list(b"GREY")
    assert 0 < profile["width"] * profile["height"] <= 4_194_304
    image = cv2.imread(str(image_path), cv2.IMREAD_GRAYSCALE)
    assert image is not None and image.shape == (profile["height"], profile["width"])
    assert zlib.crc32(image.tobytes()) == report["payload_crc32"], "PNG differs from source payload"
    foreground = (image > options["threshold_exclusive"]).astype(np.uint8)
    count, labels, stats, centers = cv2.connectedComponentsWithStats(foreground, connectivity=8)
    assert count - 1 == report["components_examined"] <= options["maximum_components"]
    assert int(foreground.sum()) == report["foreground_pixels"]
    peaks = np.zeros(count, dtype=np.uint8)
    np.maximum.at(peaks, labels.ravel(), image.ravel())
    saturated = np.bincount(labels.ravel(), weights=(image == 255).ravel(), minlength=count)
    expected, rejected_area, rejected_border = [], 0, 0
    for i in range(1, count):
        x, y, width, height, area = map(int, stats[i])
        if not options["minimum_area"] <= area <= options["maximum_area"]:
            rejected_area += 1
        elif x == 0 or y == 0 or x + width == image.shape[1] or y + height == image.shape[0]:
            rejected_border += 1
        else:
            expected.append({"pixel": centers[i].tolist(), "area": area,
                             "bounds_inclusive": [x, y, x + width - 1, y + height - 1],
                             "peak": int(peaks[i]), "saturated_pixels": int(saturated[i])})
    assert report["rejections"] == {"area": rejected_area, "border": rejected_border}
    assert report["accepted_observations"] == len(report["observations"]) == len(expected)
    def key(value):
        return (*value["bounds_inclusive"], value["area"], *value["pixel"])
    maximum_error = 0.
    for actual, reference in zip(sorted(report["observations"], key=key), sorted(expected, key=key)):
        for field in ["area", "bounds_inclusive", "peak", "saturated_pixels"]:
            assert actual[field] == reference[field], field
        error = float(np.max(np.abs(np.array(actual["pixel"]) - reference["pixel"])))
        assert np.isfinite(error) and error <= 1e-10
        maximum_error = max(maximum_error, error)
    return {"comparison": "PASS", "opencv_version": cv2.__version__,
            "image_sha256": hashlib.sha256(image_path.read_bytes()).hexdigest(),
            "report_sha256": hashlib.sha256(report_path.read_bytes()).hexdigest(),
            "payload_crc32_matches": True, "accepted_observations": len(expected),
            "components_examined": count - 1, "maximum_centroid_difference_pixels": maximum_error,
            "recording_reverified_by_this_tool": False, "physical_accuracy_qualified": False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image", type=Path)
    parser.add_argument("native_report", type=Path)
    args = parser.parse_args()
    print(json.dumps(compare(args.image, args.native_report), indent=2))
