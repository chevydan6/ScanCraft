# Combined acquisition mitigation: one-minute diagnostic pass

**Confirmed finite result:** temporarily enabling the existing bounded-copy kernel
candidate while pausing the optional session logger saves and replays 1,800 frames
per camera with zero ongoing rejected buffers and zero sequence gaps. Strict
accepted-frame framing and complete transport/host trace checks pass. Each camera
still rejects one startup buffer. This is one instrumented sample, not a permanent
fix, a stable release, or a measured 3D reconstruction.

## Method and provenance

Acquisition checkout: `dbfc2fa4b966fa6a49cc7688dc8bb451521cebf9`; unchanged native
CLI SHA-256 `be02dd6a39ecf18d451377517a19b3d3a03a1fe57fffd59a7e188ef958fecda0`.
Candidate source: `08adf1be077be172538131d3accf42f317bb7421`,
[bounded-copy experiment](../experiments/kernel-inline-copy/README.md).
Candidate binary SHA-256:
`bcce45bd2752db746373be96e216d866f3393ad74a4a9fccc01462f1e076bbbf`;
loaded srcversion `12034367675945449357638`, `scancraft_inline_copy=1`.
Original baseline SHA-256:
`579c7633261b27cfd76af16f4b1ad57c1306ce6c7d75ca914e15693e3ec127e5`,
srcversion `9D1275CDD5E013864E4C564`, restored afterward. No permanent installation,
firmware, affinity, power-policy or application-source changes.

This is a separately declared combined-intervention experiment, following failures
in [inline-copy trials](RAPTOR_INLINE_COPY_TRIAL.md) and
[baseline captures with the logger paused](RAPTOR_NO_LID_SUSTAINED.md).
Their failed admissions remain unchanged. Combining interventions cannot isolate
each intervention's effect or prove the cause of earlier failures.

Sixteen inputs are hash-bound before execution, including the plan, runner,
unchanged strict analyzers, CLI and corrected monotonic USB monitor. Collector
checks source/module hashes, ordinary scanner locks, idle media/device consumers,
10 GB available memory and 20 GB free disk. It saves five camera formats, pauses
idle WirePlumber and the optional logger, and temporarily loads the candidate with
original parameters. It checks native state before/after capture and after restore.

Native `stereo-flood-capture` requests 30 FPS, 1920 × 1200 GREY, temporary flood
illumination and 1,800 accepted frames/camera. The monitor uses a 64 MiB ring,
32 payload-prefix bytes and an explicit 180-second deadline. Private host tracing
requests 384 MiB/CPU across 16 CPUs (393,217 KiB/CPU readback), covering selected
xHCI/IRQ/workqueue events plus scheduler and local timers. Instrumentation cost
is not an uninstrumented application benchmark.

## Results

| Observation | Result |
| --- | ---: |
| Saved and natively replayed frames L/R | 1,800 / 1,800 |
| Startup rejected buffers L/R | 1 / 1 |
| Ongoing rejected buffers L/R | 0 / 0 |
| Sequence gaps L/R | 0 / 0 |
| Total rejection proxy | 2/3,602 = 0.055525% |
| Ongoing-only rejection proxy | 0/3,600 = 0% |
| Accepted host span | about 59.974 seconds per camera |
| Accepted buffer rate | about 29.996 Hz per camera |
| Exact equal sensor-PTS associations | 1,800 |
| Saved selected USB events | 1,354,534 |
| Complete parsed host trace events | 7,414,651 |
| Balanced host xHCI identity pairs | 677,267 |
| Ordered matched bulk completions L/R | 338,595 / 338,595 |
| Stable USB/host identity mappings | 10 |
| Actual IRQ39 placement | CPU10: 675,471 intervals |
| IRQ affinity readbacks | CPU10 → CPU10 |
| IRQ elapsed median / p99 / maximum | 3 / 5 / 180 microseconds |
| Largest accepted-frame completion gap | 929 microseconds |

The total proxy is below 0.1% in this finite sample. It counts rejected V4L2
buffers, not independently observed physical exposures. No statistical long-run
bound or reproducibility claim follows from one run. No independent exposure
synchronization is established by equal PTS or nearest-host associations.

The USB monitor stops by signal with exit0, zero queued/dropped events and no I/O
failure. All selected events balance; no unmatched/pending identity remains. All
3,600 accepted frames have matching bytes, PTS grouping, a single FID and exactly
one terminal EOF. There are no unrecognized successful boundaries in this trace.
Ten canceled bulk completions at cleanup retain status -2; they are not silently
converted to successful payloads. The full per-CPU trace counts match saved stats.
The unchanged strict admission returns true, authorizing only another diagnostic.

Complete host analysis finds zero camera-copy worker cycles and 350,653 balanced
local-timer intervals, consistent with the enabled candidate's intended execution
path. Software giveback-to-resubmission accounting covers 338,590 recycling cycles
per camera: median 7 microseconds both, p99 27/24 and maximum 299/343 microseconds
left/right. This is software chronology, not physical controller-ring availability,
isolated copying CPU time or a causal proof. There are no rejected ongoing gaps
to analyze in this sample.

## Restoration, images and limits

All 21 native field comparisons pass across candidate-before, trace-after and
final snapshots. Saved formats, original module/parameters, WirePlumber, logger
and global/private tracing restore. The logger source is unchanged. Kernel logs
are not identical: they add the two ordinary module unload/load discovery sequences
and the existing deprecated-nodrop warning; older ring-buffer lines roll out.
No additional warning/crash signature appears in the observed additions.

Both final images were extracted from the recordings and viewed. They show the
recognizable calibration board; its right edge is clipped in the right camera.
There is no independently measured object, verified metric scale, complete surface,
tracking/fusion demonstration or GUI/export qualification in this experiment.

Whole native CLI time-v: 71.38 seconds wall, 4.27 user + 11.23 system seconds,
44,724 KiB maximum RSS, exit0. This includes recording/replay and excludes the
monitor, large kernel trace buffers and offline analyzers. It does not measure
rendering speed, isolated copying time or whole-application resource consumption.

## Review and next step

The preceding test-only correction at dbfc2fa passes 892 tests, formatting, strict
Clippy and release; independent assignment485 resolves SCAN-REV-031 in that scope.
Exact-head CI push41706905583 and PR41706960114 both pass. The historical a4d2de1
push failure remains recorded. No completed CodeRabbit review is established by
those checks; the read-only review diagnostic reports no known current-head review.

**Recommendation:** retain the candidate as temporary/default-off. Repeat the
combined configuration under a predeclared longer, bounded-storage plan, retain all
startup/ongoing failures, and validate cleanup/disconnection independently. Establish
repeatable acquisition before any production integration. Continue physical scale
verification and real reconstruction; authorization alone is not metric evidence.

Fresh verification of the unchanged implementation also completes: 892 tests pass,
one ignored, formatting, strict Clippy and optimized release pass. Offline host and
recycling self-tests pass. Two operator invocation mistakes are retained: the IRQ
analyzer initially received no directory and produced no analysis; the recycling
analyzer initially failed for the missing required directory argument. Corrected
explicit-directory invocations complete successfully. Neither changed collected data
or validation. CodeRabbit usage lookup again returns unavailable/lookup_failed;
no review upload, paid usage or completed automated review is claimed.

Private evidence directory `inline-no-observer-20261104` contains the plan, bound
inputs, recordings, replay logs, headers, host traces, strict admission, state
restoration and image extracts. Raw scans and binaries remain outside public Git.
