# Reusing corrected trace vertices

Profiling the recorded laser workflow identified repeated inverse distortion as
a useful optimization target. The original native matcher corrected every vertex
of every second-camera trace for each first-camera observation. Those vertices
and their camera model do not change within a frame pair.

`PreparedMatcher::prepare_traces` now returns `PreparedQueries`, which owns the
normalized vertices and immutably borrows the exact matcher, raw traces, sheets
and limits used to prepare them. Its public API cannot substitute another camera
or mutate source geometry while the cache is alive. `match_pixel` reuses the
vertices; it still evaluates the epipolar line, every crossing, sheet support and
triangulation normally. The uncached method remains available.

`epipolar_trace::NormalizedTrace` is crate-private. Both paths share the numerical
intersection routine; raw endpoint coordinates and interpolation provenance remain
unchanged. The cache adds O(total trace vertices) storage. It does not approximate
distortion, change precision, downsample traces or relax acceptance limits.

Preparation retains individual trace errors and reports them in the same query
phase as the uncached path, following first-ray/sheet checks. Cancellation during
preparation is latched and fails the whole preparation, including a one-shot
cancellation callback. No partial cache is returned. Per-query cancellation and
work budgets remain active. Work counters deliberately retain their previous
conservative meaning, including vertices, although inverse distortion is reused.

`recorded_traces::run_automatic` prepares once per selected frame pair. The bundle
schema, individual results, failure counts, original observations and candidate
coordinates are preserved. This is an optimization of provisional numerical
matching, not a calibration or identity qualification.

## Correctness evidence

- Three new tests compare complete cached/uncached results across distorted/skewed
  geometry, distinct models, missing/multiple/shared/coincident crossings, bad
  traces/sheets, limits and cancellation. Separate prepared models produce their
  own results; cached state is not global.
- Full formatting, strict all-target Clippy, 703 tests and release build pass.
- The 36 independent known-3D synthetic reference cases pass in both modes, with
  identical maximum position error 9.05e-9 synthetic units and pixel error 1.24e-10.
- Every complete query result on three private Raptor pairs equals the earlier
  uncached output exactly: 473/480/473 candidate points. Raw observations, exact
  request/geometry and artifact lengths/CRCs pass verification. The older seeded
  workflow still reproduces 79/81/74 candidates exactly.
- A deliberately insufficient per-query budget retains all 592 error records
  byte-for-byte and zero candidate points.

```sh
cargo build --frozen --release -p scancraft-calibration --example match_laser_traces
python3 tools/check_laser_trace.py
python3 tools/check_laser_trace.py --prepared
```

## Performance evidence and scope

Measurements used the local Ryzen 9 6900HS with 16 logical CPUs and a private
1920×1200 recorded stereo pair. It has 592 supported first observations, seven
second traces and 473 selected candidates. The optimized private Rust probe links
the release libraries, uses `Instant`, excludes one warm-up and measures ten
iterations. Cached/uncached execution order alternates, five iterations each way.
Complete result equality is checked outside the matching timers. Raw timing rows,
probe source, build command and source/library/binary hashes are retained locally.

| Matching phase | Minimum | Median | Maximum |
| --- | ---: | ---: | ---: |
| Uncached matching | 186.85 ms | 187.26 ms | 188.04 ms |
| Cached matching | 7.80 ms | 7.87 ms | 7.92 ms |
| Cache preparation | 0.307 ms | 0.310 ms | 0.315 ms |

The median paired speedup including preparation is **22.9× for this matching
workload**. Both benchmark paths retain results in memory rather than immediately
streaming them as the CLI does. This is not a full scanner speedup or a claim about
different scenes, trace sizes or hardware. The shared machine is not an isolated
benchmark host. Cache lifetime is one frame pair, not reuse across changing frames.

Subsequent [block-based opening](BLOCK_LASER_OPENING.md) addresses the detection
cost measured below; these figures retain this prepared-trace checkpoint's scope.

In the same phase probe, medians for two-image stripe detection, full recording
validation/selection and continuity were about 71.5, 56.0 and 0.56 ms. A separate
repeated right-ray conversion loop took 164.5 ms; this is comparison work, not an
instrumented subspan of matching. The result supports targeting repeated inverse
distortion, while identifying detection as the largest remaining compute phase.

Three subsequent complete CLI runs took 0.19 seconds each, user CPU 0.11–0.12
seconds, system CPU 0.05–0.06 seconds and peak RSS 18164–18528 KiB. Earlier runs
were 0.38 seconds. These reused-file observations include validation of two
ten-frame recordings and publication/fsync, and were not a controlled paired
end-to-end benchmark. They do not establish live FPS, preview latency, sustained
memory bounds, acquisition reliability or physical accuracy. No hardware settings
or calibration changed.

## Conservative block culling

Prepared queries additionally cache 64-segment normalized bounds for long traces.
The scalar solver remains the reference and fallback. Selection, ambiguity, work
budgets and numerical values are unchanged. See [indexed trace contracts and
measurements](INDEXED_EPIPOLAR_TRACES.md) for rounding, cancellation and evidence.
