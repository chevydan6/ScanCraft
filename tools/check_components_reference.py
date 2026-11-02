#!/usr/bin/env python3
"""Device-free seeded comparison against OpenCV; no scanner access.

Generates synthetic recording-v2 bytes independently of the Rust writer.
Requires the optional OpenCV/NumPy reference environment.
"""
import argparse
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import zlib

import cv2
import numpy as np

from compare_components_opencv import compare


def recording(image):
    height, width = image.shape
    profile = dict(interface=0, width=width, height=height, fourcc=list(b"GREY"), fps=30)
    context = dict(device_identity=None, calibration_bytes=None,
                   calibration_status="synthetic_reference_fixture_not_hardware", controls=None)
    header = json.dumps(dict(profile=profile, context=context)).encode()
    metadata = bytearray(16)
    metadata[10:12] = bytes([6, 4])
    metadata[12:] = struct.pack("<I", 100)
    body = struct.pack("<IQIII", 0, 1000, 0, 16, width * height) + metadata + image.tobytes()
    return (b"RAPTOR02" + struct.pack("<II", len(header), zlib.crc32(header)) + header
            + struct.pack("<II", len(body), zlib.crc32(body)) + body + struct.pack("<IQ", 0, 1))


def check(native_cli):
    rng = np.random.default_rng(20261102)
    results = []
    with tempfile.TemporaryDirectory(prefix="scancraft-components-reference-") as directory:
        root = Path(directory)
        for case in range(60):
            if case < 4:
                width, height = [(1, 1), (1, 32), (32, 1), (2, 2)][case]
            else:
                width, height = map(int, rng.integers(3, 65, 2))
            image = rng.integers(101, 256, (height, width), dtype=np.uint8)
            density = [.05, .15, .4, .9][case % 4]
            image[rng.random((height, width)) > density] = 0
            threshold = int(rng.integers(0, 255))
            maximum = width * height
            minimum = min(1 + case % 3, maximum)
            (root / "case.raptor").write_bytes(recording(image))
            assert cv2.imwrite(str(root / "case.png"), image)
            with (root / "report.json").open("w") as output:
                subprocess.run([str(native_cli), "detect-components", str(root / "case.raptor"),
                                "0", str(threshold), str(minimum), str(maximum), "10000"],
                               check=True, stdout=output, timeout=60)
            results.append(compare(root / "case.png", root / "report.json"))
        report = json.loads((root / "report.json").read_text())
        report["payload_crc32"] ^= 1
        (root / "bad.json").write_text(json.dumps(report))
        try:
            compare(root / "case.png", root / "bad.json")
        except AssertionError:
            pass
        else:
            raise RuntimeError("binding mismatch was accepted")
    return {"synthetic_only": True, "seed": 20261102, "cases_passed": len(results),
            "maximum_centroid_difference_pixels": max(r["maximum_centroid_difference_pixels"]
                                                      for r in results),
            "payload_crc_negative_control_rejected": True, "opencv": cv2.__version__}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("native_cli", type=Path)
    args = parser.parse_args()
    print(json.dumps(check(args.native_cli.resolve()), indent=2))
