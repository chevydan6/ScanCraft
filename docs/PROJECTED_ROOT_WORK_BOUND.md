# Projected-root admission for recorded laser matching

The automatic recorded workflow now retries an overly pessimistic work bound
using the actual projected sheet roots. The configured total limit remains at
most 100 million units, with the same per-query limits, geometry, thresholds and
correspondence rules. This admits one previously rejected rotating-board pose;
it does not qualify that pose's geometry or make the pipeline real-time.

## Bound and implementation

`laser_trace::Matches::work` counts supplied second-trace vertices and
crossing-to-projection comparisons. It is not a CPU-time, complete operation-count
or memory limit. Let `V` be the total vertices, `S` the total segments, `L` the
number of supplied sheets, and `Q` the supported first observations.

The existing cheap bound is `Q * (V + S * 2 * L)`. It allows two roots per sheet
for every observation, whether those roots exist within the requested depth range
or not. When it fits, the workflow keeps its original path and manifest fields.

When it exceeds the request's limit, the new fallback computes every numerical
sheet projection for each first pixel. If pixel `i` has `R_i` projections, its bound
is `V + S * R_i`. `epipolar_trace::intersect_line` emits at most one crossing per
segment; shared endpoints only reduce the count and coincident segments stay
ambiguous. Consequently, `Matches::work <= V + S * R_i`, including duplicate
sheet hypotheses and both roots of a quadratic. The checked sum must fit the
existing total limit **before trace intersection/support matching or staging**.
No supported/accepted-match count is substituted for the full comparison bound.

`laser_trace::PreparedQueries::prepare_pixel` returns a `PreparedPixel` borrowing
its exact immutable matcher, traces, sheets and settings. Its private projected
roots cannot be supplied to another model. `work_bound` exposes the conservative
count, and consuming `execute` moves the same roots into the unchanged match
routine without recomputing or cloning them. The former projection loop is shared
by cached and uncached calls through `PreparedMatcher::project_sheets`.

First-ray/projection errors occur before counted matching work. The fallback
retains each as the original per-query error with zero counted work; it does not
silently drop a pixel or return a partial correspondence. Cancellation is checked
and propagated separately by the workflow. Errors later in matching and all
existing per-query work guards remain unchanged.

`recorded_traces/automatic.rs::run` uses this fallback only after coarse admission
fails. Fallback manifests retain `conservative_total_work_bound` and additionally
record `coarse_total_work_bound`, `work_bound_method: per_pixel_projected_roots`,
and `projected_root_preparation_failures`. Existing requests and the seeded
workflow remain compatible; the ordinary admitted path is unchanged. Previously
rejected requests can now publish when this stronger bound proves they fit.

This is original project code derived from the existing bounded loops, without
copied upstream code, new dependencies or license changes. It reuses the existing
ray/sheet solver, projection and trace matcher rather than changing their math.

## Bounds and limitations

All previous caps remain: 4096 first queries, 128 second traces, 4096 vertices per
trace, 65536 total vertices, 128 sheets, 10 million units per query, 100 million
total units and the existing report-byte limit. Source/PTS/CRC/geometry checks and
no-overwrite staging remain active. Qualification flags remain false.

Fallback preparation retains at most 4096 × 256 `SheetProjection` structs plus
per-query metadata before execution; existing trace-cache and report memory are
additional. The ordinary path avoids this new first-pixel storage. Root preparation,
image processing, serialization and filesystem work are outside the logical match
counter and retain their separate bounded inputs. No worst-case RSS, hard timeout
or whole-application shutdown qualification is claimed.

## Tests and actual recorded evidence

Four added regressions cover complete-result equivalence and a valid work bound
with zero/one/two roots, duplicate hypotheses, zigzags and coincident segments;
invalid pixels, cancellation and per-query exhaustion; exact batch admission and
one-unit-below rejection; and retained early geometry errors. The synthetic
workflow's bound tightens from 3008 to 2016: 2016 admits identical output and 2015
rejects before creating a partial folder. Its previous 3007 rejection assertion
therefore becomes the new 2015 boundary assertion, with explicit equality coverage.
This changes the estimate, not the configured work cap or numerical acceptance.

The complete 725-test/fmt/strict-Clippy/release gate passes. The independent
known-point projection reference runs 36 synthetic cases in all three modes:
uncached, prepared traces and prepared first pixels. Each gives maximum position
error 9.05e-9 synthetic units and pixel error 1.24e-10. These checks validate
numerical behavior, not physical Raptor accuracy.

Nine private saved pairs span the previous rotating-board recording. A pre-change
release-linked probe calls the original public numerical components directly,
without the workflow's coarse admission guard. Its eight previously admitted
complete match files equal the historical workflow files byte-for-byte. New
workflow results equal all nine probe files; the eight previous candidate files
also remain byte-identical. These are shared-method equivalence checks, distinct
from the independent known-point reference.

| Pose sample | Previous candidate outcome | Current candidates |
| --- | --- | --- |
| 0 | 0 | 0 |
| 1 | 779 | 779 |
| 2 | admission rejected | 2532 |
| 3 | 953 | 953 |
| 4 | 1275 | 1275 |
| 5 | 1449 | 1449 |
| 6 | 648 | 648 |
| 7 | 1276 | 1276 |
| 8 | 0 | 0 |

Sample 2 has 3145 queries and 3608 second-trace vertices. Its coarse bound is
169811130, projected-root bound 64691538, and the baseline results count 11414357
matching units with zero query errors. Every new candidate position and support
selection agrees with the baseline. All source hashes and output artifact
lengths/CRCs pass. A real-data boundary control rejects 64691537 before staging
and admits 64691538 with identical matches. No detector/model setting was adjusted.

The native app processes this pair, opens 2532 candidates and displays both source
frames with unverified-unit labels. Its private-display screenshot was inspected
and the process held no scanner descriptors. This is diagnostic publication and
visualization, not a calibrated surface, tracking result or reconstruction-quality
pass. The two front-facing zero-result poses remain failures of useful coverage.

## Processing evidence and next bottleneck

The pre-change probe uses six iterations per pose, excludes the first as warm-up,
and measures public component calls over normalized one-frame recordings. Results
are accumulated in memory, unlike the streaming workflow. Only iteration zero
writes a match file; measured serialization iterations write to memory. On the
same shared Ryzen 9 6900HS workstation, sample 2 medians are:

| Phase | Median ms |
| --- | --- |
| One-frame recording validation/selection | 2.99 |
| Two-image detection | 16.07 |
| Continuity | 0.75 |
| Second-trace preparation | 0.76 |
| Matching | 60.58 |
| JSON serialization to memory | 53.30 |

That pair's full diagnostic stream is 11131727 bytes. The new complete offline
CLI run observed 0.29 seconds elapsed, 0.23 seconds user CPU, 0.04 seconds system
CPU and 29216 KiB peak RSS, including publication/fsync. Files were reused and
other validation/build activity shared the machine. These observations are not
a controlled speedup comparison, cold-cache benchmark or live frame-rate proof.
They also do not measure indexing the original complete 900-frame recordings.

Engineering inference: continuously writing full diagnostic JSON for every pair
would add substantial work to an already slow worst-pose compute path. Next work
should separate in-memory pair processing from diagnostic publication, preserving
raw replay and explicit rejection counters, then profile and optimize matching
before continuous integration. Reliable signal, calibrated model/units, stripe
identity, tracking/fusion and sustained transport remain higher-level open issues.
Private scans, models and raw benchmark artifacts are not distributed.
