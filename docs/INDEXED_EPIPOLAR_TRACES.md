# Conservative epipolar trace indexing

Status: implemented locally; independent automated review pending. This improves
matching computation, not calibration, optical coverage, tracking or scan accuracy.

## Implementation and provenance

`scancraft-calibration/src/epipolar_trace.rs::prepare_indexed_trace` first uses
the existing complete raw validation and normalized camera-ray preparation. For
traces with at least 128 vertices, it stores axis-aligned bounds over blocks of
64 consecutive segments, including both endpoints. Bounds belong to the immutable
prepared trace; they cannot be reused with a different camera or trace set.
`laser_trace.rs::prepare_traces` selects this preparation. The public uncached
`PreparedTrace::intersections` retains the full scalar path.

`epipolar_trace/index.rs::Block::may_cross` computes the extrema of the linear
functional `a*x + b*y + c` over each normalized bounding rectangle. It rejects a
block only when every distance is provably on the same side beyond the configured
tolerance, with a conservative floating-point margin. The bound uses
`32 * f64::EPSILON * (|a|*max|x| + |b|*max|y| + |c| + 1)`. This covers rounding
in the absolute bound, extrema and scalar dot product, including subnormal error;
`f64::MAX/16` headroom excludes intermediate overflow. Nonfinite or risky bounds
force the whole trace back to its original scalar distance prepass. All blocks
are classified before interpolation so this fallback preserves numerical error
ordering. This assumes normal Rust round-to-nearest floating-point semantics;
no fast-math or unsafe SIMD is introduced.

Retained blocks use the original interpolation, raw-endpoint handling, projection,
residual and ambiguity arithmetic in shared `append_span`. Segment/vertex indices
stay global, including shared-endpoint deduplication across block boundaries.
Competing crossings, tolerance-qualified endpoints and coincident segments remain
explicit. Work admission/counters still count all supplied vertices and support
comparisons as before; culling never raises a configured cap or changes selection.

This is independently written Rust using standard interval bounds for a linear
functional, adapting ScanCraft's own scalar solver from `72ea37c5d07bf09ca54eb5ee26b23b14e4e3364a`.
No upstream implementation, third-party source or new dependency was copied.
Existing project licensing applies. Generic local cancellation helpers also avoid
unnecessary dynamic dispatch without removing their check sites.

## Bounds and cancellation

At most 64 blocks describe a 4096-vertex trace. Each block stores two `usize`
indices and four `f64` extrema (48 bytes on this 64-bit host), up to 3072 bytes of
block entries per maximum-length trace, plus vector metadata/allocator overhead.
The existing global 65536-vertex/128-trace limits remain. The candidate mask is
64 bytes on the stack; indexed distance scratch holds at most 65 `f64` values
(520 bytes), versus the scalar path's up-to-4096 distances. Output vectors and
shared-vertex set are still bounded by crossings; no worst-case RSS claim follows.

Preparation checks cancellation per existing vertex and per block. Queries check
per classification, per skipped/retained block, every retained distance/segment,
and final completion. Checks decrease when numerical work is skipped. Callback
counts are deliberately not identical to the scalar path. These are bounded
operation intervals, not a wall-clock or whole-application shutdown guarantee.
Long/coincident traces can retain every block and approach scalar work plus index
overhead; no universal acceleration is promised.

## Validation

Six new regressions in `epipolar_trace/index.rs` and `index_tests.rs` cover bounds
across extreme exponent scales, cancellation and tolerance margins, distorted and
reversed traces (127/128/129/257/4096 vertices), global shared endpoints,
coincident segments, and pre-emission scalar fallback/error ordering. The existing
full `laser_trace` cached-equivalence test also includes long, competing and
oscillating traces. An initial adversarial test fixture incorrectly expected a
projection error for a valid center pixel; corrected its out-of-domain setup and
retained the failed log. The first full gate caught a test-only Clippy expression;
corrected without suppressing checks. Final gate: 738 tests, formatting, strict
all-target Clippy and release build pass.

`tools/check_laser_trace.py --dense-traces` adds independent analytic known-point
checks with 192 vertices per trace. Run both sparse and dense cases in uncached,
`--prepared` and `--prepared-pixel` modes (36 cases per mode/density). These are
synthetic numerical references, not millimeter-accuracy qualification. Dense modes
all achieve maximum position error 1.303e-8 synthetic units and pixel error
3.222e-10; sparse modes retain 9.043e-9 and 1.240e-10 respectively.

All eleven saved raw-pair computations reproduce every complete match record,
observation and candidate value. Every artifact and manifest in all eleven
recording-backed bundles and all three seeded bundles is byte-identical to the
previous implementation, with input SHA and artifact CRC/length checks. Native
app renders of the expensive pose and box retain 2532/1764 diagnostic points,
source camera panels and unverified-unit labels; no scanner descriptors are held.
Screenshots were inspected. This is saved-data validation, not a new hardware scan.

## Matching-only paired measurements

Final production library versus the exact original scalar/prepared source from
72ea37c in separate namespaces of one release binary. Same immutable saved inputs,
real `WorkControl` cancellation in both, 13 iterations with first warmup and three
rotating modes (original real check, indexed real check, original constant false).
Complete results are compared outside timers; callback counts are measured in a
separate correctness pass. Allocation/results accumulation is included; image
processing, JSON conversion, model/input I/O and trace preparation are excluded.
Original/candidate result storage and warm files remain resident. Shared host,
no CPU affinity or frequency isolation. A repeated first-camera ray loop is a
comparison workload, not an instrumented matcher subspan.

| Saved workload | Queries / traces | Original median ms | Indexed median ms | Median paired ratio |
| --- | ---: | ---: | ---: | ---: |
| Small front pose | 809 / 1 | 1.502 | 1.524 | 0.985 |
| Expensive pose | 3145 / 9 | 86.610 | 32.485 | 2.672 |
| Box | 2604 / 8 | 49.997 | 23.592 | 2.117 |

The small scalar-fallback workload is slightly slower in this run; do not report
an improvement there. Cancellation calls per pair are unchanged at 80403 for the
small pose, reduced from 22861050 to 2599939 for the expensive pose, and 12169710
to 2310841 for the box. An earlier isolated helper-only trial yielded only about
1.07x on the larger cases, motivating block culling. Prototype timings are not
substituted for these final production-library measurements.

Private raw inputs, source/build hashes, complete results, timing rows and failed
attempts are retained in the developer recovery evidence. The public report
contains no scan payload or calibration model. These figures do not establish
live FPS, camera synchronization, physical accuracy or sustained-session capacity.

A separate final full-engine probe (same decoded frames, real WorkControl,
11 iterations/first warmup, alternating no-sink/query-JSON order) measured median
51.968/41.272 ms without full query JSON and 101.541/74.404 ms with it for the
expensive pose/box. Reports remain equal after timers. This measures detection,
continuity, preparation and matching, but excludes source integrity/model/file
I/O and display. It is not a paired before/after index benchmark. At these times,
serially processing all incoming roughly 30 Hz pairs is not yet established.
A bounded acquisition handoff, explicit omissions/backpressure, tracking/fusion,
long-session capacity and qualification still remain to be implemented/verified.
