# Benchmarks

[Bounded preview handoff](docs/ASYNC_LASER_PREVIEW.md): same-binary old/new complete
Snapshot bytes match in 65 comparisons. Producer work changes from 6.665–6.710 ms
to 0.186–0.224 ms by copying raw inputs; worker preparation still costs about
6.7 ms and sequential total work increases slightly. No application-wide speedup,
preview FPS/p95 or memory-soak qualification is claimed.

[Laser stream profiling](docs/LASER_STREAM_PROFILE.md): same-binary alternating
journal comparison on five saved pairs reduces encoding/flush medians from
6.875–7.076 ms to 1.894–2.029 ms with exact output equality. Engine time remains
42.788–46.298 ms; preview preparation adds 6.70–6.77 ms. These component timings
are not an application-wide speedup or live-rate qualification.

Native laser GUI diagnostics (2026-11-03) save all 32 raw frames/camera in one
private renderer-limited trial but process only 19 pairs (59.375%, below 90%).
Two default private software-renderer starts accept zero frames; rendering-load
causality is unproven. No preview FPS/p95 or physical qualification is claimed.
See [GUI observations and limits](docs/LIVE_LASER_GUI.md).

Current acquisition-only repeatability evidence (2026-11-02): a source/binary-bound
ten-minute stereo probe accepted 18,010 frames per camera at about 29.83 Hz,
but ongoing rejected-buffer proxies were 0.54–0.56%. Resource accounting and
limits are documented in [NATIVE_CAPTURE_REPEATABILITY.md](docs/NATIVE_CAPTURE_REPEATABILITY.md).
This is not a full scanning throughput or accuracy benchmark.

Initial experiments: AMD Ryzen 9 6900HS / Radeon 680M, Linux
7.2.11-6900hs-lto, Raptor USB 3 (5000 Mbps), firmware 1.5.7.

Rust debug build, sequential single-camera acquisition to local disk:
left IR 100 frames / 2.161 s; right IR 100 frames / 2.157 s. Startup is
included. These short experiments are not sustained throughput benchmarks.
Raw sensor setting is 1920x1200 GREY at 50 fps. See evidence JSON for counters.

Release compilation and ten-minute structural acquisition duration are now
verified in the stated experiments; the buffer-rejection reliability gate still
fails. Pending: thirty-minute live pipeline; sixty-minute memory stability;
timing distribution and p95 latency; comparison
against matched Creality Scan modes; calibrated geometry ground truth. No
manufacturer accuracy or processing-throughput claims are made.

One-million-point exact-index trim of 500,000 selected indices: 73.73181 ms
in a release build; undo restored the original point count. Single trial,
not a latency-distribution or full editing-qualification result.

Extended Rust IR validation: 30,050 accepted frames over 604.6597 s,
171 incomplete frames rejected (0.5658% of received frames), 171 sequence
gaps, zero missing metadata pairs. No recording in this duration test.
Other builds and offline tasks ran concurrently; this is not an idle-system
isolation experiment. Cause of incomplete frames has not been established.

After replacing BTreeSet membership checks with a validated selection mask,
the same one-million-point / 500,000-selected release trim trial took
26.060871 ms. The original benchmark is retained separately. This still
does not establish the complete editing acceptance gate.

Full-cloud rectangle selection on an analytic million-point orthographic grid:
12.745299 ms release, exactly 250000 selected, zero false positives/negatives.
Single measured run, strict 1e-9-pixel test tolerance; no whole-GUI or general
selection accuracy claim. See screen-selection-million.json.

Synthetic stereo mathematics: OpenCV 5.0.0 comparison across 60 exact cases
observed maximum projection difference 1.14e-12 px, ray difference 9.33e-13
and 3D difference 1.08e-8 mm. These measure numerical agreement only.
Native stereo diagnostics preserve raw metadata/counters in native-stereo-
probe*.json; rejected buffers include fragments and are not a physical-frame
loss denominator. Exposure timing remains unqualified.

## Synthetic undistortion benchmark

Same Ryzen 6900HS development workstation, release build, 1920x1200 packed GREY
synthetic input, Brown-5 synthetic lens. One map build: 149.589 ms;
20 applies after one warmup: median 40.359 ms, nearest-rank
p95 40.971 ms, maximum 45.892 ms. Fresh output
allocation is included; map construction is excluded from per-frame results.
Map coordinate storage is 55,296,000 bytes. No RSS or
long-run growth measurement is implied. This is one CPU component, without
matching, capture, reconstruction or rendering; it does not qualify live FPS.
Evidence: evidence/undistort-benchmark.json; runnable example in scancraft-image.

## Sparse stereo numerical reference

The OpenCV 5.0.0 / NumPy 2.5.4 comparison on 28 synthetic two-depth seeds
agrees on forward/reverse winners and runner-up correlations; max ZNCC score
residual 2.9744e-8, max triangulation deviation 6.0335e-11 mm. The fixture uses
known 4/8 pixel disparities at 400/200 mm with a positive affine intensity
change. This is numerical correctness evidence, not physical accuracy, broad
matching success-rate or live performance qualification. See the reproducible
commands and limitations in docs/STEREO_MATCHING.md.

## Calibrated rectification numerical comparison

OpenCV 5.0.0 / NumPy 2.5.4, 60 synthetic rotated/asymmetric Brown-5 cases:
maximum ideal-pixel residual 8.2538e-10 px; source roundtrip 1.6078e-13 px;
epipolar Y residual 8.1753e-10 px; original-frame triangulation difference
1.1932e-8 mm. This compares our explicitly chosen basis with OpenCV sparse
projection, not identical crop/basis output from OpenCV stereoRectify. Results
and thresholds: evidence/rectification-opencv-comparison.json. No physical
accuracy or full-pipeline throughput claim is made.

## Exact spatial-query microbenchmark

`cargo run --release -p scancraft-geometry --example spatial_benchmark` executes
three deterministic 10,000-point / 1,000-query rounds and checks every indexed
answer against a scalar brute-force hypot-distance loop. Current Ryzen 6900HS
x86_64: tree build 1.15–1.48 ms, indexed queries 2.15–2.18 ms, brute queries
~349 ms; 95–105× ratio including build. Raw results in
`evidence/spatial-index-benchmark.json`. This is not total ICP speed, scanner
throughput, latency or a CrealityScan comparison. All answers exactly agree.

## 2026-10-26 dual-IR structural soak

One native 1920x1200 GREY / requested 30 FPS run accepts 18,100 buffers per camera
over 606.4426 s (29.84454 accepted buffers/s). Each camera rejects 92 buffers
(0.505717% proxy; FAIL <0.1% gate). Native elapsed 606.958 s, no qualification
crash in this trial. Largest two-second sampled RSS: 26,932 KiB; no claim of
true allocation peak or 60-minute memory qualification. Ryzen 9 6900HS / Fedora
44 / USB reported 5000 Mbit/s; no concurrent compilation or vendor scan ran.
Lightweight diagnostic inspection and offline Python tool tests ran during capture.
This is acquisition structural throughput, not reconstruction or preview speed.
Evidence/method: docs/STEREO_ACQUISITION_QUALIFICATION.md and
evidence/stereo-ir-soak-20261026/.

## Idle redraw observation during recovery UI validation

On the private display with software rendering requested, the unchanged completed
recovery panel previously used 52 app CPU ticks over 2.350 s (100 ticks/s; about
22% of one CPU). Conditional active-work redraw polling used zero measured ticks
over 2.349 s. Tick resolution and screenshot requests limit this short comparison;
no universal zero-CPU, live throughput or renderer qualification follows.
Evidence: recording-recovery-idle-{baseline,controlled}-render/capture.json and
recording-recovery-render-review.json. Active camera/worker/smoke polling remains.

## Robust registration numerical comparison (not a throughput benchmark)

Six synthetic fixed-correspondence Huber fits compare native Rust against independent
SciPy nonlinear least squares. Maximum transformed-point difference is below
9.6e-9 mm; scale-dependent systematic bias remains (about 0.0161/0.0644/0.322 mm for
three deltas on one contaminated geometry). This is numerical equivalence, not
scanner accuracy, global convergence, robustness prevalence or speed qualification.
Native example/runner hashes and Python/NumPy/SciPy versions are retained in
evidence/huber-plane-reference/result.json. Method and scope: docs/POINT_TO_PLANE_ICP.md.
No live/throughput/physical acceptance gate is advanced by these comparisons.

## Repeated single-image undistortion baseline (2026-10-27 local date)

Unchanged release undistort_benchmark on the Ryzen 6900HS: 1920×1200 synthetic
GREY, 20 runs after 1 warmup, median 40.109 ms, p95 46.174 ms, maximum 49.208 ms; one map
build 202.117 ms. Allocation included; mapbuilding excluded from apply timing.
Coordinate storage 55,296,000 bytes, not processRSS. All 2,304,000 map pixels valid.
Evidence docs/benchmarks/undistort-baseline-2026-10-27.json records source/example
hashes and original output. This is a repeated component baseline, not a speedup,
paired-camera measurement, full-pipeline FPS or hardware accuracy/latency gate.
No optimized implementation is selected from this result alone.

## Measured paired image preparation

Large stereo maps now use one scoped helper when at least two CPUs are reported;
small/one-CPU/resource-limited cases retain sequential processing. Sampling math
is unchanged. On this workstation, the production API’s synthetic 1920×1200 pair
median changed from 89.930 to 43.795 ms with bit-identical outputs/masks. The 583-test gate
passes; single-CPU and thread-denied child fallbacks passed. This is not full
scan throughput, accuracy or RSS qualification. See docs/PAIRED_REMAP_CONCURRENCY.md
and docs/benchmarks/paired-remap-2026-10-27.json. Automated review remains pending.

## Conservative epipolar block indexing

Final same-binary production-library versus 72ea37c scalar/prepared comparison,
real WorkControl callbacks, 13 iterations/first warmup/rotating order: expensive
saved pose median86.610→32.485ms (median paired ratio2.672), box49.997→23.592ms
(ratio2.117), small scalar-fallback pose1.502→1.524ms (ratio0.985). Every complete
result equal outside timers. Matching only, excluding preparation/image/JSON/I/O;
shared warm host, no live/accuracy/general speedup inference.
See docs/INDEXED_EPIPOLAR_TRACES.md for exact scope, cancellation counts and limits.
