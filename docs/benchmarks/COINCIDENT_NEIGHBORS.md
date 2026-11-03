# Coincident-point neighbor regression

Developer-identified issue DEV-PERF-001; independently unverified. Before revision:
`ba5ec7ca187fe9da6aa94e0dbc706e96c3ee15ae`. Source/binary fingerprints and all three
samples per size are in coincident-neighbors-2026-10-26.json.

`SpatialIndex::search_k` previously explored both branches for every zero-distance
split because `abs(delta) <= bound` remained true. An all-coincident statistical
filter therefore did a full tree traversal for every point. This is an existing
processing defect, not a new editing feature.

Each tree node now retains its subtree's minimum original index. Once the heap
contains k zero-distance neighbors, a subtree whose minimum index is at least the
worst retained index cannot improve either distance or tie order. Excluding one
point only makes this minimum conservative. No epsilon or positive-radius shortcut
is introduced. Cancellation checks and exact ordering remain intact. The change is
independently implemented project code; no external algorithm source is copied.

Tradeoff: one additional usize of metadata per tree node, affecting every consumer
of SpatialIndex. This is eight bytes of field payload per node on 64-bit targets;
resident memory/allocator effects were not benchmarked. No general positive-radius
search speedup or mature-library comparison is claimed.

| Coincident points | Before median (ms) | After median (ms) |
| --- | ---: | ---: |
| 500 | 15.309 | 8.561 |
| 1,000 | 42.102 | 9.936 |
| 2,000 | 143.606 | 21.650 |
| 4,000 | 534.388 | 39.297 |

Three sequential runs per size, same machine/session, release CLI, ASCII PLY with
`comment units millimeters`, double x/y/z properties, N identical `0 0 0` rows.
For each input run `scancraft-cli filter-statistical INPUT 4 1 NEW_DIRECTORY` and
measure subprocess duration with Python time.perf_counter. Each run needs a new
output directory. Timings include process startup, parsing, tree construction,
analysis, copying, project validation, serialization and filesystem sync. Baseline
was measured before rebuilding the modified CLI. They are synthetic local timings,
not isolated algorithm benchmarks, hardware qualification, or a million-point gate.

Validation: two new Rust tests check exact brute-force results across mixed repeated
positions/positive queries/exclusions/k values and a structural bound (<128 visited
nodes on 4096 coincident points, formerly all 4096). All 499 workspace tests,
formatting, strict Clippy and release build pass. Six NumPy reference cases pass.
Genuine calibrated reconstruction and transport qualification remain higher-priority
hardware goals; no scanner is currently enumerated and physical calibration remains
deferred. Pathological general distributions may still require linear query work.
