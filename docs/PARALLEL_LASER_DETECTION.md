# Concurrent camera detection

`scancraft-depth::laser_pair::Processor` can detect the two camera images
concurrently before the existing serial trace-matching/triangulation stages. This
scheduling change reuses the original detectors, continuity rules and complete observations;
it introduces no numerical approximation, sampling, calibration activation or
new dependency. Native laser capture still produces diagnostic candidates with
unverified units and physical accuracy, rather than a qualified fused model.

## Execution and ownership

`Processor::new` caches whether `std::thread::available_parallelism` reports more
than one CPU. An unavailable or single-CPU result selects serial detection.
`laser_pair/detection.rs::pair` also keeps images smaller than 256 * 1024 pixels
on either side serial. This threshold is a conservative scheduling heuristic from
sampled workloads, not a universal optimum or an accuracy parameter.

For eligible images, a single scoped standard-library worker runs first-camera
detection while the caller detects the second camera. Both borrow immutable images,
settings and the existing shared WorkControl. There is no raw-pixel copy, detached
worker, new frame queue, per-observation thread or parallel sink publication.
Thread-creation failure falls back to the original serial calls. Its actual resource
exhaustion path has been inspected but is not forced by a regression test.

The caller joins the worker before returning either result, preserves first-camera
error precedence when both cameras fail, and propagates worker panics. Scope exit
also joins on parent unwinding. Cancellation remains cooperative inside the
existing detector/continuity algorithms; this establishes no hard shutdown bound.
Shared progress stages/percentages may interleave while both cameras run and remain
approximate diagnostics. CPU availability is a construction-time hint; later quota
or affinity changes are not tracked.

Matching, candidate/outcome order, all query/work limits, source ownership, required
raw/pair journals, optional previews and saved formats remain unchanged. No new
SIMD/GPU path or mathematical algorithm is introduced. Code is independently
written orchestration around the project's existing Rust implementations; their
recorded algorithm provenance and dependency licenses remain applicable.

Two detector invocations can now hold scratch allocations concurrently. Existing
per-image pixel/observation/comparison bounds still apply, but peak scratch memory
can increase: top-hat contrast buffers, row-extrema buffers, trace bookkeeping and
outputs can overlap. This is a bounded concurrency claim, not a measured whole-app
RSS limit or a 60-minute memory qualification. A thread is created/joined for each
eligible pair; a persistent worker pool remains a possible future comparison.

## Reproducible numerical and component checks

The private benchmark uses the exact preceding depth processor source from
`fcd8be8ecf4bcd508ba75037fa79fcdf3375d035` as the serial baseline. Both implementations
are compiled into one release executable against the same dependency builds. Five
retained native frame pairs are loaded outside timing; thirteen iterations alternate
execution order, with the first iteration per pair excluded from timing summaries.
All 130 complete report results match the saved reports. Timing includes processing
and scoped worker startup/join, and excludes input I/O and output JSON encoding.
The no-op query sink is the same numerical mode used by native pair processing.

| Saved pair | Serial median ms | Concurrent median ms | Median paired serial/concurrent ratio |
| --- | ---: | ---: | ---: |
| 0 | 40.650 | 32.775 | 1.246 |
| 4 | 40.519 | 33.079 | 1.231 |
| 9 | 40.757 | 32.693 | 1.242 |
| 14 | 40.941 | 33.165 | 1.237 |
| 18 | 40.090 | 32.346 | 1.237 |

A separate detector-only comparison uses fifteen alternating iterations for each
of five native pairs and five synthetic image sizes, with first iterations excluded.
All observations, rejected-run counters, traces, continuity counters and membership
entries match. Native serial medians are 16.463–16.531 ms versus 8.598–8.783 ms
concurrent. Synthetic 640x480 is 2.397 versus 1.292 ms; 1024x1024 is 8.223 versus
4.306 ms. At 64x48, concurrent execution is slower (0.110 versus 0.046 ms), which
supports retaining a serial path for small inputs. These measurements use top-hat
opening radius 12 and this host's observed 16-CPU availability. They do not promise
similar benefits for every image/configuration/machine or establish preview FPS,
p95 latency, sustained throughput or independent geometric accuracy.

The 794-test frozen workspace gate passes formatting, strict all-target Clippy,
automated tests and release build; the existing large storage test remains explicitly
ignored in the ordinary gate and its preceding separate evidence is unchanged.
Three new regressions verify a complete 480-candidate large pair is byte-identical
under serial/concurrent execution with unchanged raw bytes; two distinct camera
errors retain first-camera precedence in both orders; and pre-cancellation rejects
before starting borrowed work. Existing small-pair, sink, ambiguity, work-limit,
process IPC and publication regressions remain passing.

## Hardware and environment evidence

Six finite native GUI captures alternate implementation order as serial/concurrent,
concurrent/serial, serial/concurrent. They use the same private camera model/request,
64 accepted raw frames per camera, explicit schema-2 storage quotas, existing scoped
line lease and `LP_NUM_THREADS=2` private software-renderer condition. The old release
executable is preserved with its exact hash; the new executable matches the gate.

| Trial | Processed pairs / raw frames per camera | Processing drops first/second | Unmatched first/second | Startup buffer rejects first/second | Ongoing rejects first/second |
| --- | --- | --- | --- | --- | --- |
| Serial 0 | 41/64 (64.0625%) | 23/23 | 0/0 | 1/1 | 1/1 |
| Concurrent 0 | 50/64 (78.125%) | 14/14 | 0/0 | 1/1 | 1/1 |
| Concurrent 1 | 47/64 (73.4375%) | 16/17 | 1/0 | 1/87 | 0/1 |
| Serial 1 | 41/64 (64.0625%) | 23/22 | 0/1 | 1/0 | 1/1 |
| Serial 2 | 42/64 (65.625%) | 22/22 | 0/0 | 103/92 | 1/1 |
| Concurrent 2 | 49/64 (76.5625%) | 15/15 | 0/0 | 1/0 | 2/2 |

All six trials publish verified raw/pair artifacts, pass reported restoration and
exact full before/after camera-stream/illumination GET comparisons, and observe
helper camera descriptors with none in the GUI. All observed helpers/apps exit.
Buffer rejects are V4L2-buffer observations, not necessarily physical exposure
counts. Trials retain warm-start variability and explicit processing drops; they
are short observations on changing sensor samples, not a controlled universal
speedup, rate qualification or a substitute for the 90% sustained target.

The first separate close-test attempt fails before any staging bundle: the GUI
reports helper IPC ended without a complete final result, and its advertised log
is absent at inspection. The helper is already absent; later app/server exits have
no established cause. Full before/after stream/illumination readbacks still match.
A subsequent display start fails to write its `/tmp` PID lock, and a direct small
write reproduces EDQUOT (122). Moving this mission's cached Gitleaks executable to
the home cache with verified identical hash and an old-path compatibility symlink
restores `/tmp` write/fsync. No unrelated files or application code are removed.
This does not establish the original helper-exit cause.

The private display initially waited only for a socket pathname, which did not
prove readiness. After explicit successful `xdpyinfo` readiness (19 retries in one
observation), an authenticated normal-display close trial passes with home-directory
temporary diagnostics. It saves 10/9 raw frames, 8 pairs in 8 segments, processing
drops 1/1, unmatched 1/0, startup rejects 1/96, ongoing 0/0 and one stopped
incomplete buffer/camera. Source/publication integrity and full stream/illumination
readbacks pass. One observed close-to-exit time is 0.293 s, not a hard bound.
Earlier failures remain failures; neither cache relocation nor display readiness
establishes the original helper-exit cause.

Across seven new successful native bundles and three preceding saved bundles,
310 complete records / 580,698 candidate values match the unchanged original
serializer, numerical engine and every raw CRC/sequence/flags/time/PTS reference.
Actual unpaced replay of a new 64-frame/camera capture processes four pairs
(7707 candidate values), reports 59 processing drops/camera and one unmatched input/camera, and emits four preview
packets plus completion. Its records also match; combined totals are 314 records /
588,405 values. All eight new successful native/replay manifests have verified
lengths/CRCs. This is integrity evidence, not independent geometry truth.
All observed successful apps/helpers exit; a subsequent accessible-FD inventory
finds no camera/USB holders. Saved screenshots show both raw views, diagnostic
candidate geometry and the unverified-unit warning.

Private display retry failures, logs and final close/replay evidence remain in
the developer response and recovery checkpoint. No failed attempt is silently
counted as a pass. Raw frames, camera models, screenshots and probes remain private.

## Remaining priorities

The finite native processed fraction still fails 90%; startup/ongoing transport
rejects and renderer robustness remain unresolved. Profile the remaining serial
matching/dispatch costs and whole-pipeline scheduling before increasing queue
sizes or dropping observations. Calibrated dimensions, optical coverage, actual
tracking/fusion, complete editable models, sustained storage/resource/reliability
qualification and the end-to-end release workflow remain incomplete. Physical
calibration is still deferred. Independent reviewer findings remain under reviewer
control. Assignment354 independently resolves 024's specific unavoidable single-file
capacity defect in preceding commit `234c2b6af2309c0bce0bc6e87813276a509b1230`;
this performance change does not close 005/006/007/021 or authorize a merge.
