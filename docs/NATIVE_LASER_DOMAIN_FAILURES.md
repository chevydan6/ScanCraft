# Held-out board evidence and native full-image integration failure

Status: **experimental calibration evidence; the full native reconstruction fails**.
Investigation starts at255241173db92620e421760898e670a0d50579eb. Production code remains
ebb77a2954f8563d15e28f2d0b02928da0835cd2. No model activation, owner-dimension fitting,
metric export or complete object scan is claimed. Raw observations remain private.

## Additional board pose, with original-pixel measurements

Three front-facing samples0/1/8 were selected by image inspection from the retained
900-frame rotating-board sequence before this analysis. Existing native
`inspect-components` validates the complete900-frame source for each selected view
and camera; six calls cover two unique900-frame recordings, not5,400 independent
frames. Selected source indices0/112/899 have equal stereo sensorPTS and6/21/20us
host differences. Neither equality nor apparent sharpness proves exposure synchronization.
PNG payload CRCs match the selected recordings; the private native PNG shape probe
reproduces every original CLI observation and rejection exactly.

The unchanged original threshold100, area201..10000, component cap10000 and ellipse
RMS<=0.5pixel leave only2–3 accepted anchors. Every original four-anchor association
fails explicitly. A separately planned11-pixel horizontal grayscale opening removes
thin lines for **coarse identity only**. The initial SciPy import fails before any
capture/analysis; the retained replacement uses NumPy min11 then max11, edge padding.
No packages or production dependencies change.3,024 direct scalar-window checks agree.

The same native components/shape gates and exact-four-anchor association recover
sample0 in both cameras and sample1 in both. Sample8 left still fails with3anchors.
Native `target::associate` retains the existing10pixel seed/3pixel refined radii,
20match minimum and3support margin. Final marker measurements are mutually unique
within3pixels of the coarse matched centers and come exclusively from original
small, unsaturated, shape-passing components. Opened centers never become geometry.

| Sample | Fine original markers, left/right | Joint pairs | Triangulated points | Plane result |
| --- | ---: | ---: | ---: | --- |
|0|26/13|13|13|Fails unchanged20point minimum|
|1|27/29|25|24|RMS0.049139627RAW, max0.098582183RAW|
|8|0/26|0|0|Fails association/point support|

One sample1 pair fails the frozen native inverse-distortion domain. Domain limits are
not enlarged. Existing `PreparedStereo::triangulate` and `registration::fit_plane`
use actual original marker pixels, without sheet or box fitting. The pose was not
used in the previous two-pose sheet fit. It shares the same camera model and moving
capture limitations, so it is not independent metrological validation.

## Frozen sheets, original failure and stricter marker masking

Native stripe detection uses the previous board settings unchanged. A3pixel-eroded
hull of accepted original marker pairs, excluding all detected opened-component
rectangles+3pixels, retains1,165centroids. Existing native rays/board intersections
and25pixel x-gap grouping yield five groups. All groups are tested against all four
previously frozen curved sheets; three corresponding groups pass RMS_X<=0.2RAW and
maximum<=0.5RAW without refitting. Other hypotheses remain explicit failures.

After inspecting source pixels, an additional rectangle x1110..1160,y200..850 inside
the visible board face adds517centroids. This extrapolates the marker-derived board
plane to visible face area outside its marker hull; it is image-informed, not a
pre-registered physical qualification. The new group tests the sheet used for the
box fragment. Its RMS_X0.148926RAW passes, but maximum3.060867RAW **fails**.

The largest error is at original pixel[1127.299145,639], visibly on a marker boundary.
The initial hypothesis that a small marker was omitted by the minimum-area rule is
**disproved**: including every opened component down to area1 removes zero points,
and the marker already had area284. Its opened bounds[1119,643,1136,660] are narrower
than original component31's bounds[1119,640,1136,662], area315. Opening changed the
outline; opened ellipse RMS0.51865 fails while original RMS0.28317 passes. The original
failed experiment and the unsuccessful smaller-area experiment are both retained.

A subsequent globally stronger mask unions all opened-component rectangles with
**every original shape-passing component's rectangle**, keeping the same3pixel margin.
It removes two observations without changing measurements, camera domains, sheets,
fit coefficients or acceptance limits. The four frozen sheet checks then report:

| Original near/far sheet | Additional-pose support | RMS_X, RAW | Maximum absolute X residual, RAW |
| --- | ---: | ---: | ---: |
|2/0|136|0.041095|0.128963|
|3/1|426|0.042587|0.124073|
|4/2|420|0.045616|0.147870|
|5/3|515|0.063494|0.437988|

All24 group/sheet combinations remain recorded, including unsupported groups0/1.
The last result follows inspection-driven mask correction and must not be presented
as untouched held-out accuracy. Numerical SVD/ray/forward-projection checks agree;
these are developer checks, not independent reviewer or physical-accuracy evidence.
No sheets are refitted in this investigation. Missing outer-sheet coverage remains.

## Actual native full-image workflow: zero candidates

The existing `scancraft-cli match-recorded-laser-traces` receives the frozen native
camera model, four curved sheets, original box recordings and selected indices5/3.
Interface order is[0,2] with no reflection, unlike historical factory-coordinate
experiments. The prior box detector/continuity/epipolar/triangulation settings remain.
All supported full-image traces participate; no ROI/partner seeding is supplied.
The existing3pixel projected-sheet support tolerance is used. That test differs
from the private0.75RAW X-residual check and is not an equivalent quality claim.

The command exits0 and publishes an intact diagnostic bundle:6,309/7,022 detections,
2,206queries,20right traces/2,130vertices, **zero candidates and2,206query errors**.
Every observation equals the earlier unfiltered detector output, and all artifact
lengths/CRCs and exact model/request bytes reconcile. CLI success means diagnostic
publication, not successful reconstruction. No GUI scan success is claimed.

A private audit uses the same native camera/ray/sheet APIs, preserves every result,
and reconciles each actual query error in evaluation order:

| First failing stage | Queries | Evidence |
| --- | ---: | --- |
|First-camera inverse ray|268|Outside usable inverse-distortion domain|
|Second-camera projection of a sheet root|546|Projection outside calibrated lens domain|
|Second-trace preparation/query|1,392|Otherwise successful first/sheet stages encounter an invalid trace vertex|

The complete errors are1,660 inverse-distortion failures and546 projection-domain
failures. Right traces0/2/49 have212/46/23 invalid vertices respectively; the other
17traces have none. Trace0 also contains170valid vertices, which the current whole-
trace contract does not use when another vertex fails.

Relevant production boundaries:

- `scancraft-image::laser_continuity::trace_rows` constructs image traces without a
  camera-domain mask; image continuity is not calibrated-ray validity.
- `scancraft-calibration::epipolar_trace::PreparedTrace::prepare_trace` rejects the
  whole supplied trace when a vertex cannot produce a valid camera ray.
- `laser_trace::PreparedMatcher::prepare_traces` retains per-trace preparation errors;
  `match_impl` propagates an encountered error and returns no partial selection.
- `PreparedMatcher::project_sheets` propagates an out-of-domain second-camera root
  projection instead of retaining other roots as a usable partial query.
- `scancraft-depth::laser_pair::Processor::process` supplies all supported traces and
  preserves these query errors; `recorded_traces::run_automatic` publishes diagnostics.

This is an integration gap with intentionally strict existing error contracts, not
permission to remove error reporting or expand the calibration domain. A failed
unrelated trace prevents the otherwise usable box fragment reaching the native GUI.
The earlier201-point private fragment remains valid only within its documented
cropped hypotheses and does not establish that the full native workflow works.

One shared-machine failed-run observation:0.17s elapsed,0.09s user CPU,0.08s system,
38,832KiB peakRSS (`/usr/bin/time -v`). This includes validation/diagnostic publication,
mostly failed queries, warm data and concurrent work. It is **not** successful
reconstruction throughput, rendering FPS, isolated processing or live latency.

## Immediate implementation priority

Develop explicit domain rejection and contiguous valid-segment handling, with full
source-vertex/segment/sheet-root provenance. Never bridge across rejected vertices,
ignore ambiguous valid alternatives, silently choose a nearest candidate, expand
lens limits or count rejected observations as recovered geometry. Invalid first rays
remain errors. Out-of-domain roots and vertices must remain visible in diagnostics.
Malformed inputs, cancellation and resource-limit failures must remain fail closed.
Existing strict APIs and persisted bundles need compatibility protection.

Required regression evidence includes mixed valid/invalid traces, no bridge across
holes, preserved root ambiguity, explicit rejection accounting, cancellation and
bounds, unchanged valid-only results, then this exact genuine full-image recording.
Native GUI inspection follows successful processing; physical scale, additional
sheets, recognizable surface, three trials and measured export remain unmet.
No production change has been made as part of this investigation.

## Evidence and review

Private roots `native-heldout-board-20261104` and `native-box-workflow-20261104` retain
all sources, plans, original failures, native outputs, numerical checks and timings.
The milestone manifest `heldout-integration-milestone-20261104/evidence-manifest.json`
binds223files/25,966,302bytes, excluding Cargo targets; executable hashes are bound
in execution plans. SHA256:
`155dc264989674bae47c90add270b20e89a8a7099732a39ce3c8047f457f368a`.
Original900-frame source recordings and image hashes are bound externally. Existing
sealed evidence is unchanged. None of these private files are published.

Private Rust probes pass formatting, strict Clippy and release builds. Application
source/dependencies are unchanged; the independently verified895-test/fmt/Clippy/
release gate remains applicable, without a redundant full gate for documentation.
Independent522 reproduces the previous2552411 curvature/box-fragment experiment and
confirms both of its CI runs passed. It does not review this new held-out/native-
integration investigation. Substantive CodeRabbit coverage remains outstanding.
No hardware/service/module/firmware, paid-service, protection or merge action occurred.
