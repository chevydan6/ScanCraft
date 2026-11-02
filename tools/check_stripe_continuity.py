#!/usr/bin/env python3
"""Optional NumPy adjacency-matrix reference for within-image stripe traces.

Synthetic recordings only. Can also compare an existing native JSON report;
this checks algorithm equivalence, not laser identity or physical accuracy.
"""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

import numpy as np

from check_components_reference import recording


def reference(pixels, config):
    traces, active, ambiguous, comparisons = [], [], 0, 0
    for y in sorted(set(p[1] for p in pixels)):
        indices = [i for i, p in enumerate(pixels) if p[1] == y]
        active = [t for t in active if y-pixels[traces[t][-1]][1]
                  <= config["maximum_row_gap"]]
        endpoint = np.array([pixels[traces[t][-1]] for t in active]).reshape(-1, 2)
        current = np.array([pixels[i] for i in indices])
        matrix = np.abs(endpoint[:, 0, None]-current[None, :, 0]) <= (
            config["maximum_horizontal_step"]
            + config["maximum_slope"]*(y-endpoint[:, 1, None]))
        comparisons += matrix.size
        degree_old, degree_new = matrix.sum(axis=1), matrix.sum(axis=0)
        retained = {active[i] for i in range(len(active)) if degree_old[i] == 0}
        for local, index in enumerate(indices):
            unique = np.flatnonzero(matrix[:, local])
            if degree_new[local] == 1 and degree_old[unique[0]] == 1:
                t = active[int(unique[0])]
                traces[t].append(index)
                retained.add(t)
            else:
                ambiguous += int(degree_new[local] > 0)
                retained.add(len(traces))
                traces.append([index])
        active = sorted(retained)
    return [{"observation_indices": t,
             "span_rows": int(pixels[t[-1]][1]-pixels[t[0]][1]),
             "continuity_supported": len(t) >= config["minimum_observations"]
             and pixels[t[-1]][1]-pixels[t[0]][1] >= config["minimum_span_rows"]}
            for t in traces], ambiguous, comparisons


def compare(report):
    native = report["continuity"]
    pixels = [o["pixel"] for o in report["observations"]]
    expected, ambiguous, comparisons = reference(pixels, native["parameters"])
    assert expected == native["traces"]
    assert ambiguous == native["ambiguous_observations"]
    assert comparisons == native["comparisons"]
    assert native["raw_observations_preserved"] is True
    assert report["projector_line_identity_verified"] is False
    assert report["physical_accuracy_qualified"] is False
    partition = sorted(i for t in expected for i in t["observation_indices"])
    assert partition == list(range(len(pixels)))
    return {"observations": len(pixels), "traces": len(expected),
            "supported_traces": sum(t["continuity_supported"] for t in expected)}


def check(native_cli):
    results = []
    random = np.random.default_rng(73129)
    with tempfile.TemporaryDirectory(prefix="scancraft-stripe-continuity-") as directory:
        path = Path(directory)/"synthetic.raptor"
        for case in range(32):
            image = np.zeros((96, 128), dtype=np.uint8)
            for y in range(96):
                for x in [20+y//8, 100-y//6]:
                    if random.random() > case/40:
                        image[y, x] = 80
            for _ in range(case*8):
                image[random.integers(96), random.integers(1, 127)] = 80
            path.write_bytes(recording(image))
            run = subprocess.run([str(native_cli), "trace-stripes", str(path), "0",
                                  "10", "30", "3", "255", "10000", "4", "1",
                                  "0.25", "20", "6"],
                                 capture_output=True, check=True, timeout=60)
            results.append(compare(json.loads(run.stdout)))
    return {"synthetic_only": True, "cases_passed": len(results),
            "observations_checked": sum(r["observations"] for r in results),
            "numpy": np.__version__, "physical_accuracy_qualified": False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("native_cli", type=Path)
    args = parser.parse_args()
    print(json.dumps(check(args.native_cli.resolve()), indent=2))
