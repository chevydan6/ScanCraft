# Rotation coverage, marker ambiguity and tracking loss

## Confirmed result

The earlier successful first31-frame replay does not establish full-rotation
tracking. A longer replay of the same genuine900-frame board recording exposes
both laser-processing capacity failures and unrecovered marker tracking loss.
No acquisition, camera-model fitting, scale fitting or gate relaxation was used
for this investigation. This is stored Raptor data, not synthetic geometry.

Source revision: `037d67e1dfc65fcf088cbc8b2462ac7dbb4fd3a5`. See
[separate tracking/fusion](SEPARATE_TRACKING_FUSION.md) and
[recorded marker fusion](RECORDED_MARKER_FUSION.md) for the preceding short replay.

## Sampling and retained failures

Both original900-frame recordings were fully validated using native
`recording_playback::Playback::open`. Before examining results, the experiment
selected original left indices0,4,...896, spanning30.003703seconds. Each has one
unique equal-PTS right counterpart within1000microseconds of host timestamp;
all225 pairs were retained. Exposure synchronization remains unverified.
`select_raw` preserved original frame payloads, metadata and capture contexts in
single-frame derivative recordings. PNGs are inspection outputs, not inputs to
native reconstruction. Selecting every fourth frame bounds this diagnostic to
the existing256-pair result format; it is not an incoming-rate processing test.

The unchanged box detector/matcher preset processed every selected pair through
`scancraft-cli match-recorded-laser-traces`:

| Outcome | Number of pairs |
| --- | ---: |
| Successful matching operation | 154 |
| Automatic trace query budget exceeded | 71 |
| Successful operation with zero candidate points | 13 of154 |

Successful operations produced140,497 **unfused candidates**, including scene
and background observations. They are not a reconstructed object or a quality
measurement. All71 errors remain in the receipts. The runner refused to publish
a complete fused sequence from only the successful subset; it did not fabricate
empty surface bundles for failed operations. Numerical gates, domain restrictions
and capacity bounds remained unchanged. The earlier board-specific detector has
not been evaluated over this full interval; preset dependence remains a limit.

Relevant implementation: `scancraft-workflow::recorded_traces`, the matching CLI
dispatch, and `tracked_candidates::run`. A budget refusal is a bounded-processing
failure, not proof that every observation in that frame is geometrically invalid.

## Marker-only motion test

To distinguish surface-processing failure from tracking failure, a private
adapter processed all225 original pairs independently of laser success. It calls
the existing native component/shape/stereo/Tracker APIs with the same threshold100,
area5..1000, shape RMS0.5pixels, lens domains and stereo limits used previously.
All Cartesian hypotheses and mutual-uniqueness outcomes are retained. Markers
are tracking observations only, never substituted surface geometry.

The first18 sampled frames, original indices0..68, were accepted. At original
index72 tracking rejected for insufficient bidirectional coverage. The remaining
interval contains96 insufficient-ICP-overlap rejections and110 inputs with fewer
than three usable marker points. The last accepted reference remains index68;
there is no automatic reset and no recovery by the final sampled image.
Inspected original images show the board's marked face, edge/back views, and its
marked face returning near the end. This does not establish an externally
measured rotation angle or exact full revolution.

## Consecutive-frame control

A second, separately planned test processed **all first101 consecutive frames**,
including frames omitted by the uniform sampling. It accepted frames0..69, then
rejected frames70..100: eight coverage failures followed by23 ICP-overlap failures.
Thus the initial loss also occurs without temporal subsampling. Recovery over
all900 consecutive frames has not been tested.

At frame69 there are35/37 shape-accepted left/right observations,41 geometrically
accepted stereo hypotheses and24 mutually unique pairs. At frame70 there are34/37
shape-accepted observations,48 accepted hypotheses but only16 mutually unique pairs.
All16 points align within the correspondence distance, but distinct reference
coverage is16/24=0.6667, below the unchanged0.7 gate. RMS is0.0432583 in unverified
model units. Low residual alone therefore does not justify accepting the pose.

**Engineering inference:** increasing stereo ambiguity removes enough individually
unique marker pairs to trigger the coverage gate; subsequent motion moves beyond
the last accepted local ICP reference. This explains the observed initial failure
mechanism, not every later missing observation. The test does not independently
identify each physical marker or prove that every accepted stereo hypothesis is
correct. Relevant code: `tracked_candidates/markers.rs::reconstruct`,
`tracking.rs::Tracker::track_inner`, `registration.rs::icp_with`.

## Numerical verification and evidence

Separate NumPy nearest-neighbor/Kabsch-SVD calculations reproduce every accepted,
rejected and insufficient-point outcome, including preservation of the last
accepted reference. Maximum numeric differences are1.34e-12 over225 sampled frames
and3.50e-12 over101 consecutive frames. Independent inverse-Brown/closest-ray and
forward-projection calculations check2,276 and4,029 accepted stereo hypotheses,
respectively: coordinate differences<=2.36e-9 model units and reprojection
differences<=1.71e-9 pixels. These are numerical checks, not physical accuracy.

All450 sampled and202 consecutive-frame PNG payload CRCs match native source
bindings. Successful laser bundles' artifact lengths/CRCs and candidate counts
are reconciled. This investigation does not independently recompute every one
of the140,497 laser candidates. Original source recording hashes match the earlier
bound capture. Private evidence roots: `rotation-coverage-replay-20261104` and
`rotation-loss-fullrate-20261104`; plans, source/binary bindings, original failures,
command receipts, full outcomes and reference scripts remain outside public Git.
The initial consecutive-frame orchestration failed before processing because its
copied binding list named a nonexistent plan file. Correcting that private path
allowed execution; the original script/error/receipt remain retained.

No production algorithm changed. The unchanged workspace's full formatting,
strict Clippy, test and optimized-build gate passes: 921 passed, zero failed,
one ignored. Independent
assignment550 already verified the parent viewer and921-test gate. This new
motion investigation still needs its own independent review.

The external evidence manifest binds 5,910 files totaling 4,638,158,074 bytes;
SHA256 `7d01052b56035a9000e3739db9305995bb516ae5983a74d354d49e5705b848c2`.
CodeRabbit's usage lookup remains unavailable; no review upload or paid review
was performed. Passing local checks is not substantive CodeRabbit review.

## Immediate priorities

1. Capture the measured box rotating slowly, keeping the scanner fixed; distinguish
   markers attached to the moving turntable from stationary background features.
2. Investigate temporal marker identity and explicit relocalization against retained
   references. Preserve ambiguity, coverage and residual checks; do not silently
   re-anchor a lost scan into a different coordinate system.
3. Profile the failed laser queries and examine bounded batching/diagnostics before
   changing resource admission. A capacity increase alone does not resolve marker
   ambiguity, stripe identity, calibration or accuracy.
4. Continue physical calibration/scale and held-out dimensional validation. Neither
   a numerically accepted pose nor an attractive fused preview qualifies a scan.

Current capacity must not be described as universally1000 frames: legacy live
schema1 retains that bound, while `live_stereo/storage.rs` admits schema2 up to
100,000 frames with explicit storage quotas; `stereo_probe::validate_request`
admits up to30,000 raw capture frames. These software limits are not sustained
hardware or memory qualification results.
