#!/usr/bin/env python3
"""Device-free varied ellipse/cut-shape reference comparison; no scanner access."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

import cv2
import numpy as np

from check_components_reference import recording
from compare_component_shapes import compare


def check(native_cli):
    results = []
    with tempfile.TemporaryDirectory(prefix="scancraft-shapes-reference-") as directory:
        root = Path(directory)
        y, x = np.mgrid[:96, :128]
        for case in range(32):
            angle = case*.31
            u = (x-64.3)*np.cos(angle)+(y-48.1)*np.sin(angle)
            v = -(x-64.3)*np.sin(angle)+(y-48.1)*np.cos(angle)
            mask = (u/(15+case % 9))**2+(v/(9+case % 7))**2 <= 1
            if case % 4 == 0:
                mask &= x >= 60
            image = np.where(mask, 200, 0).astype(np.uint8)
            (root/"input.raptor").write_bytes(recording(image))
            assert cv2.imwrite(str(root/"image.png"), image)
            with (root/"report.json").open("w") as output:
                subprocess.run([str(native_cli), "inspect-components", str(root/"input.raptor"),
                                "0", "100", "20", "10000", "10", "0.5"],
                               stdout=output, check=True, timeout=60)
            results.append(compare(root/"image.png", root/"report.json"))
            assert results[-1]["successful_fits_checked"] == 1
    return {"synthetic_only": True, "cases_passed": len(results),
            "maximum_center_difference_pixels": max(r["maximum_center_difference_pixels"] for r in results)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("native_cli", type=Path)
    args = parser.parse_args()
    print(json.dumps(check(args.native_cli.resolve()), indent=2))
