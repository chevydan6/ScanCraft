# Per-pixel epipolar constraint reuse

Status: implemented locally; automated independent review unavailable. This is
an optimization of existing diagnostic laser matching, not calibrated scanning.

## Implementation and rationale

`crates/scancraft-calibration/src/laser_trace.rs::PreparedMatcher::match_impl`
previously recomputed the same first-camera inverse-distortion ray and essential
line for every second-camera trace. The model, first pixel and intersection
configuration are fixed within that call. A measured private prototype justified
removing this repeated work before changing production code.

`epipolar_trace.rs::PixelQuery` borrows one immutable `PreparedTrace`, owns a
copy of that pixel/configuration and lazily stores the line or its error in a
query-local `std::cell::OnceCell`. `for_pixel` does no numerical work. The first
cached-trace query evaluates `compute_line`; later traces reuse that exact result.
There is no session history, cross-pixel cache, synchronization, global state,
new dependency, unsafe code or persisted-format change. Existing prepared models
and matchers remain `Send + Sync`; the cell lives only inside one matching call.
Storage is constant per query, apart from an existing bounded error string.

The numerical expressions extracted into `compute_line` are unchanged from
`6fc557db5c1b6a46ec5531b7c1590a1cfbbe5563`. Every trace still calls the cancellation
callback before accessing the cell; cancellation precedes a cached geometry error,
which precedes a cached trace-preparation error. Indexed intersection, scalar
fallback, candidate order, ambiguity, triangulation, sheet support, work admission
and configured tolerances are unchanged. Uncached public queries retain their
original scalar execution. This cache does not reduce the number of numerical
queries or silently discard competing results.

This is independently written reuse of ScanCraft's own computation, using Rust's
standard cell abstraction. No implementation was copied from OpenCV, Open3D, PCL
or another project; the existing project license applies.

## Correctness evidence

Three permanent tests in `epipolar_trace/query_tests.rs` compare complete scalar,
fresh and repeated-query results for distorted/pinhole, long/coincident/oscillating
and invalid traces; cancellation callback counts and pulse cancellation; geometry
before trace error; degenerate geometry; pixel isolation; and shared matcher traits.
The full offline gate passes 812 tests with one existing ignored test, formatting,
strict all-target Clippy and release builds. No required check was suppressed.

The final production processor was compared with a separate namespace containing
the exact baseline calibration sources and the baseline pair processor with only
its calibration import redirected. All 24 pairs in one retained Raptor capture
preserve all 63,436 query results, including 185,094,875 serialized detail bytes,
complete compact reports and previously saved results. All 624 timed executions
also match complete reports outside the timers. This tests saved hardware data;
it is same-method equivalence, not independent geometric truth.

Build and run the independent analytic synthetic reference with:

```sh
cargo build --frozen --release -p scancraft-calibration --example match_laser_traces
python3 tools/check_laser_trace.py --prepared-pixel --dense-traces
```

All six combinations of sparse/dense and scalar/prepared/prepared-pixel execution
pass 36 cases each. Maximum position error is 9.043e-9 synthetic units sparse and
1.303e-8 dense; maximum pixel error is 1.240e-10 and 3.222e-10 respectively. These
do not establish Raptor dimensional accuracy.

## Numerical processing measurements

One release binary compares the final production processor with that copied
baseline on identical decoded frames, using real `WorkControl` callbacks. Thirteen
iterations per implementation/pair alternate execution order; the first is warmup.
Timing includes detection, preparation, matching, allocation and report construction;
it excludes raw/model I/O, JSON serialization, display and source verification.
Detailed result buffers remain resident. Rust 1.99.1, shared warm workstation,
no CPU affinity or frequency isolation. Namespace/build-context differences can
affect compiler optimization; these are scoped observations, not universal ratios.

| Saved pair ordinal | Queries / traces | Baseline median ms | Production median ms |
| --- | ---: | ---: | ---: |
| 0 | 2657 / 9 | 35.176 | 29.553 |
| 4 | 2636 / 8 | 32.421 | 27.455 |
| 9 | 2669 / 9 | 35.381 | 29.478 |
| 14 | 2638 / 8 | 32.288 | 27.307 |
| 18 | 2612 / 8 | 32.153 | 27.319 |

The private preliminary prototype is retained separately and is not substituted
for these final production measurements. A single-trace workload has no repeated
line to save; no advantage there is claimed.

## Native GUI checks and remaining problems

Four finite native trials used the actual preserved baseline executable and final
release executable in order baseline/current/current/baseline, with the same
explicit geometry/request. Each saved 64 raw frames per camera. Authenticated
private X11 display readiness was verified by `xdpyinfo`; software rendering used
`LP_NUM_THREADS=2`. No competing compilation/benchmark ran during these trials.
The geometry remains unqualified and no physical calibration was performed.

| Trial | Processed pairs | Queue drops, cameras 1 / 2 | Pending, cameras 1 / 2 | Startup rejects | Ongoing rejects |
| --- | ---: | --- | --- | --- | --- |
| Baseline 1 | 49 / 64 | 14 / 14 | 1 / 1 | 0 / 1 | 1 / 2 |
| Current 1 | 54 / 64 | 10 / 8 | 0 / 1 | 1 / 0 | 2 / 2 |
| Current 2 | 54 / 64 | 10 / 10 | 0 / 0 | 89 / 88 | 1 / 1 |
| Baseline 2 | 47 / 64 | 17 / 17 | 0 / 0 | 0 / 2 | 0 / 0 |

Current 84.375% remains below the mission's 90% processing target. These short,
sequential trials do not isolate every source of variation or establish sustained
FPS, latency, renderer-default behavior, memory stability or accuracy. Reject
counters measure V4L2 buffers, not necessarily physical exposures; no startup
samples were removed to claim reliability. The high startup counts need further
investigation and are not attributed to the cache.

A separate active-window-close trial saved 8 raw frames/camera and 8 pairs, with
zero queue drops/pending, startup rejects 95/92, ongoing rejects 1/1 and one stopped
incomplete buffer/camera. Close-to-exit was 0.313477 s, one observation without a
hard deadline guarantee. All five trials exit successfully, preserve raw integrity
and publication, and restore all measured stream/illumination GET readbacks.
Helpers own device descriptors; the GUI owns none. All observed children exit.
Saved and active GUI screenshots were inspected.

The preserved earlier engine/serializer independently replays all five resulting
bundles: 212 complete records and 403,462 candidate values, every raw-frame
reference and complete serialized old record bytes match exactly. Artifact lengths
and CRCs also pass. This is compatibility evidence, not calibrated truth or fusion.

Private input, raw data, probe source, hashes, timing rows, screenshots and failed
attempts are retained in developer recovery artifacts `epipolar-query-cache-pykdu4us`
and excluded from publication. CodeRabbit CLI 0.9.2 remains authenticated, but a
fresh repository-scoped quota/billing lookup fails. No review upload, paid usage,
push, PR or merge is authorized by that unavailable result. Independent findings
005/006/007/021 remain unchanged; calibrated reconstruction, tracking/fusion,
transport reliability, long sessions and the complete model workflow remain open.
