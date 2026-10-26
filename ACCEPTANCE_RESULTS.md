# Acceptance results

PASS means the stated narrow experiment passed, not the full release gate.

| Experiment | Result | Evidence |
| --- | --- | --- |
| Native discovery | PASS, one observation | CLI discovery, six Raptor nodes |
| Left IR structural acquisition | PASS, 100 payloads; optical usability unqualified | evidence/native-left-ir.json |
| Right IR structural acquisition | PASS, 100 payloads; optical usability unqualified | evidence/native-right-ir.json |
| Left/right raw replay | PASS, payload CRC and metadata validation | evidence/native-*-ir-replay.json |
| Color capture initial | FAIL, JPEG padding validation | evidence/native-color-diagnostic.log |
| Color capture after fix | PASS, 30 frames and full JPEG decoding | evidence/native-color-padded.json; native-color-decoded.json |
| Rust GUI preview and recording | PASS, first color smoke test | evidence/gui-smoke.log |
| Discovery repetition | PASS, 20/20 same-session trials | evidence/discovery-20-trials.json |
| Million-point trim | PASS, 26.1 ms release, count restored by undo | evidence/trim-million-points.json |
| Geometry offline | PASS, 28 tests, narrow synthetic scope | evidence/geometry-tests.log |
| Warm stream start | PASS, 20/20 | evidence/warm-stream-start-20-trials.json; not full initialization |
| Historical single-IR ten-minute duration | PASS, 30,050 valid frames / 604.66 s | evidence/native-ir-ten-minute.json |
| Raw buffer rejection proxy | FAIL, 171 rejects / 30,221 buffers (~0.566%) | Same run; threshold <0.1% |
| Dual-IR >=10-minute structural duration | PASS, one connected-session run, 18,100 accepted per camera over 606.44 s | evidence/stereo-ir-soak-20261026/assessment-reviewed.json |
| Dual-IR buffer rejection proxy | FAIL, 92 / 18,192 per camera = 0.505717% | Same trial; no warmup exclusion; not physical exposure counts |
| Post-soak warm reopen quality | INCOMPLETE / ISSUE OBSERVED, valid buffers but 111 left / 1 right rejects | evidence/stereo-ir-soak-20261026/after-soak-probe.json |
| Full device reliability gate | INCOMPLETE / FAIL | Cold reconnection unqualified; corruption threshold fails |
| Calibration and geometry accuracy | DEFERRED | User request; no calibrated ground truth |
| Full native live-scan-to-qualified-mesh release | INCOMPLETE | Offline mesh tools exist; calibrated live pipeline and release qualification remain open |
| Editing full mission gate | INCOMPLETE | Basic offline tools implemented; advanced operations pending |

The first left IR run rejected three startup frames and observed one sequence
gap. The right IR run rejected two startup frames and observed one sequence
gap. Neither short run establishes the <0.1% corruption threshold. Frame
validation currently covers payload framing/length, V4L2 error flags, UVC PTS
and monotonic sequence/timestamps; it does not establish geometric correctness.

- Real version-2 color recording: 100/100 release-build replays, each decoding
  all 30 JPEG frames, passed. Evidence: real-color-replay-100-trials.json.
- Read-only native Rust GET_PROTOCOL_VERSION passed; raw payload preserved in
  native-protocol-version.json. It is not a firmware version decoder.
- Radius-filter tests match a brute-force numerical reference on a deterministic
  dataset. Geometry tests do not establish accuracy on physical scanner data.
- Direct USB diagnostic: FAIL; no frames captured, driver restoration failed,
  physical recovery later verified via V4L2. Hardware path is not enabled by default.

Responsive editing: 11 desktop worker/render tests pass, including injected
worker termination, mid-filter cancellation with exact original-document
restoration, non-overwriting atomic save, project/history reload, unsampled
bounds, original-index picking and large finite coordinate rendering. Evidence:
evidence/tests.log and cloud-worker-tests.log. At that checkpoint the workspace had 49 passing tests.

The observed cancellation-to-result latency in one debug test was about 1 ms;
this excludes the subsequent equality verification. It is not a p95 UI
latency measurement or whole-application cancellation qualification. Current
release trim trial: 26.060871 ms; initial 73.73181 ms result is preserved as
trim-million-points-initial.json. One trial each, not a stability distribution.

Screen selection evidence: screen-selection-million.json reports one million
analytic grid points, 250000 selected, zero false positives/negatives and
12.745299 ms release processing time with 1e-9-pixel test tolerance. GUI uses
0.5-pixel tolerance; this numerical case does not qualify every GUI boundary.
Additional tests cover concave/self-crossing even-odd polygons, capsule brush
strokes, point-depth ties, selection modes and cancellation.

Viewport mouse/key tests pass (viewport-gesture-tests.log). An integration
test selects and trims an unsampled original point, then verifies undo restores
its geometry and attributes. Manual native desktop workflows remain pending.

Recovery after physical reconnection: all six nodes return. Right IR 100 valid
frames / 15 rejects; left IR 100 / 9; color 20 / 0. Evidence: recovery-*.json.
These short runs confirm recovery, not reliability qualification.

Current selection checkpoint: 62 workspace tests pass, Clippy with warnings
denied passes, formatting passes and the release workspace build passes.
Hardware qualification and manual GUI workflow qualification remain incomplete.

Calibration math: 15 unit tests plus 60 independent OpenCV numerical cases
pass. Evidence: calibration-tests.log and calibration-opencv-comparison.json.
The parameters and points are synthetic; no Raptor dimensional gate is passed.

Simultaneous IR acquisition: native-stereo-probe*.json records actual metadata
and bounded rejection samples. Timestamp agreement is observed, while exposure
synchronization remains unverified. Rejection counts include spurious kernel
fragments and must not be equated to physical frame loss without more evidence.

Current geometry/timing checkpoint: 79 offline tests pass. OpenCV comparison
passes on 60 synthetic cases. Native stereo diagnostics returned accepted
buffers from both cameras, with no optical synchronization or real geometric
qualification claim. The updated rejection samples preserve strict flags and
make the unresolved kernel framing behavior observable.

## Sparse reconstruction integration

A Rust `scancraft-depth` stage now connects checked calibration/triangulation to
the editable point-cloud representation. Four new tests cover analytic depth,
source-index and color preservation, per-match invalidity reasons, PLY roundtrip,
source/binding rejection, cancellation and all-invalid results. These use
synthetic observations only. Actual Raptor image matching, calibrated geometry,
GUI reconstruction and the M2 genuine point-cloud milestone remain incomplete.

## Image preparation checkpoint

Native Rust GREY-frame views and reusable undistortion/rotation maps are now
implemented, with per-pixel validity masks and checked source/calibration sizes.
Ten new unit tests pass. OpenCV 5.0.0 agrees on the synthetic rotated Brown-5
fixture: 2,887 valid / 185 invalid pixels, identical masks, maximum coordinate
error 1.907e-6 pixels and intensity error 1.145e-5 against bilinear reference.
The full-resolution border regression found and fixed floating-point roundoff
incorrectly masking zero-distortion image edges.

These are image-processing component tests. Physical Raptor calibration, image
matching, exposure synchronization and M2 genuine reconstruction remain open.
The GUI does not yet expose this stage. See docs/IMAGE_PREPARATION.md.

## Sparse image correspondence checkpoint

A native Rust rectified-image matcher now provides zero-mean normalized patch
correlation, full-support validity/texture checks, explicit uniqueness thresholds
and reverse-match consistency. It retains seed indices and score diagnostics,
and rejects nonrectified calibration metadata rather than assuming raw frames
are rectified. Work and memory are bounded; cancellation returns no partial
result. Nine tests cover correspondence, ambiguity/occlusion rejection, invalid
inputs, signed disparities and the link to checked point-cloud reconstruction.

OpenCV 5.0.0 comparison agrees on all 28 synthetic two-depth matches, including
forward/reverse winners and runner-up scores. Maximum ZNCC residual is 2.98e-8;
triangulation/analytic geometry residual is 6.04e-11 mm on this synthetic fixture.
These residuals do not measure physical Raptor accuracy. Actual stereo
rectification, calibrated capture association, feature selection and native GUI
reconstruction remain incomplete. No M2 or physical acquisition gate is closed.
See docs/STEREO_MATCHING.md and evidence/stereo-opencv-comparison.json.

## Rectification and image-to-cloud integration

A calibrated Rust rectification stage now constructs a common ideal horizontal
rig from original camera geometry and an explicit output grid, retaining both
source ray rotations. Paired maps feed the sparse matcher. Accepted matches map
back to original distorted pixels before triangulation, preserving original
axes and original-camera depth/reprojection limits. Reports retain full original
calibration, seed lineage, rejections and explicit unverified timing/accuracy.
No accepted observations produces diagnostics with no reconstructed cloud.

Four new mathematical tests and three integrated pipeline tests pass, including
vertical baselines and rotated camera images. The independent OpenCV comparison
on 60 synthetic cases has maximum ideal-pixel difference 8.26e-10 pixels and
original-frame geometry difference 1.194e-8 mm. This is numerical evidence only.
Actual Raptor calibration, synchronized inputs, GUI reconstruction and M2 remain
unverified/incomplete. See docs/STEREO_RECTIFICATION.md.

## Usable offline reconstruction command

The native CLI now exposes calibration + source PNG pair + explicit request
processing through `reconstruct-stereo`. Results publish atomically into a new
directory with provenance/rejection report, input/artifact CRCs, PLY and editable
project when geometry exists. Empty/rejected pairs retain diagnostics and no
fake cloud. Process-level tests exercise the actual binary and project reload.
A generated rotated-camera fixture produces three synthetic analytic-plane
points with one rejected border seed; it is not real Raptor calibration or M2.

An exact PLY/project comparison found one-ULP JSON parsing changes. Enabling
serde_json float_roundtrip fixes this; a difficult-finite-bit-pattern regression
also checks original coordinate restoration through project undo history.
Current checkpoint: 116 offline tests, warning-free Clippy and release build
pass. Physical calibration, capture pairing, GUI scan reconstruction and real
scanner qualification remain incomplete. See docs/OFFLINE_RECONSTRUCTION.md.

## Native dual recording and integrity-checked selection

`stereo-capture` now records both IR streams concurrently and publishes only
after complete CRC/footer replay and count checks. `extract-frame` validates
entire recordings before selecting a frame, preserving raw metadata alongside
a lossless PNG and diagnostics. Selection tests reject corruption after the
chosen frame. Native CLI process tests verify payload/metadata preservation.

A real trial saved 100 structurally accepted frames per camera, with left/right
rejected-buffer counts 3/2, sequence gaps 2/1 and no duplicate PTS. Both files
pass complete replay; 72 sensor-PTS values intersect. This does not verify
exposure synchronization. Raw files/images remain private. Selected images are
dark and left samples show pronounced banding; optical usability and cause
remain unresolved. Structural acceptance alone is not complete M1 optical or
mode qualification, and no M2 point cloud is claimed.

119 offline tests, warning-free Clippy and optimized workspace build pass.
See docs/STEREO_RECORDING.md and evidence/native-stereo-recording-01.json.

## Read-only camera settings investigation

Native `controls` diagnostics now query a fixed standard V4L2 allowlist and
retain driver ranges/flags/current values or explicit errors. No setting or
vendor write is exposed. Absolute exposure has a separate standard 100 us-unit
interpretation; gain and other raw units are not guessed. Two UAPI/unit tests
pass. Hardware snapshot is pending until the existing soak releases its lock.

Future stereo probes/captures now emit stderr progress every 500 accepted
buffers and include accepted host-span rates in summaries. A rate regression
checks that device sequence gaps are not mistaken for accepted-frame counts.
The already-running binary has no new progress logging and was not restarted.
122 offline tests, warning-free Clippy and release workspace build pass.

Completed dual-stream duration experiment: 30,000 accepted buffers each; left
821.845 s / 36.502 Hz, right 606.139 s / 49.492 Hz. Duration >=10 minutes passes
for each stream; full reliability does not. Rejected-returned-buffer proxies
1.2346% / 0.9934% exceed 0.1%; these do not directly measure physical exposure
corruption. Left rate is below requested 50 Hz; actual valid incoming exposure
rate is not established, so this is not a verified live-processing ratio.
Evidence: native-stereo-soak-01-summary.json. Controls snapshot is read-only
and passed, with unresolved actual exposure-unit interpretation.

## Rate comparison and kernel candidate

Isolated left/right 50 FPS runs reach 49.688/49.788 accepted Hz. A dual 30 FPS
1000-buffer trial reaches 29.907 Hz on both cameras. Selected left images lack
the pronounced earlier banding. A later dual 50 FPS repeat reaches 48.517/49.987
Hz after 212/2 rejected buffers; its selected later left image also lacks that
banding. The failure is intermittent/startup/reconfiguration/load-sensitive,
not a proven fixed bandwidth limit. Images remain dark and optics unqualified.

Stereo CLI commands now accept explicit --fps while retaining 50 as default.
Unsupported requests fail validation; single-stream diagnostics include host-span
rates. An isolated Raptor-IR-only 5..32 URB queue candidate compiles for the
current kernel and passes 79 compiled policy cases, including a hard freeze
against runtime parameter changes. Clean patch application reproduces it. It
is not loaded/installed or hardware-tested; installed module hash is unchanged.
Cached noninteractive sudo authentication is unavailable. No hardware gate is
closed by this candidate. See docs/ACQUISITION_RATE_INVESTIGATION.md.

## Workbench reconstruction-output integration

Found and fixed a real integration mismatch: CLI writes `.orproj`, while GUI
import previously recognized only `.orproject`. Import now detects project
content and retains CRC/version/history checks. Startup accepts a project/PLY
path, selects the point-cloud tab and loads in the background without starting
a camera. Unitless startup PLY requires an explicit GUI conversion choice.

Regression tests use the actual public CLI fixture to verify geometry import,
exact delete/undo restoration, renamed project history and corruption/units
handling. A release app on a private authenticated virtual display visibly
loads three synthetic points with correct bounds and no open scanner device
descriptors. This is not full interactive or physical scanner qualification.
127 offline tests, warning-free Clippy and release workspace build pass.
See docs/WORKBENCH_PROJECT_OPEN.md and evidence/workbench-project-startup.

## Native offline GUI reconstruction

The workbench now runs stereo image reconstruction in its background document
worker, using a shared Rust scancraft-workflow module also used by the CLI. The
panel accepts explicit calibration/images/settings/output, displays progress and
rejection counts, supports cancellation, and applies the unsaved-document guard.
A completed bundle publishes before replacing the editor. Failed/cancelled work
and all-rejected pairs preserve the current geometry/history/selection.

Bounded regular-file reading rejects special files without blocking; cancellation
checks extend through I/O and artifact writes. The publication commit boundary
is explicit. A real private-display release run reconstructs and shows the
three-point synthetic fixture; saved artifacts match the CLI bundle byte-for-byte.
No scanner was opened. This does not qualify actual Raptor geometry, timing,
live scanning or a complete interactive workflow.

134 offline tests, warning-free Clippy and release workspace build pass.
See docs/GUI_RECONSTRUCTION.md and evidence/workbench-reconstruction-final.

## Native factory-read verification

136 offline workspace tests pass; formatting and Clippy with warnings denied
pass. Native opcode-25 header and fixed 8 KiB region reads succeed on Raptor.
The byte-count/hash and candidate record boundaries are summarized in
evidence/factory-read-summary.json; source bytes remain private. This advances
protocol investigation only. M2 genuine calibrated geometry and physical
calibration/accuracy gates remain unverified.

## Factory checksum/index checkpoint

139 offline tests pass; warning-free Clippy and release CLI build pass.
`factory-inspect` verifies CRC and indexes two independently read real factory
regions (evidence/factory-index-200000.json and factory-index-202000.json).
Every truncation and single-byte corruption of the synthetic container is
rejected. Checksummed malformed counts/strides/labels are also rejected.
This proves only the supported container integrity/layout checks; genuine
calibrated geometry, live tracking and physical accuracy remain incomplete.

## Distinct factory-envelope verification

140 offline tests pass, Clippy with warnings denied passes and release CLI
build passes. The native verifier authenticates actual 20a000 data; laser
indexing refuses that distinct payload. Synthetic tests verify corruption
rejection and refusal of mixed signature pairs even with a recomputed CRC.
Evidence: evidence/factory-envelope-20a000.json. This advances format research;
M2 genuine calibrated point cloud and all physical accuracy gates remain open.

## Named-camera lens field verification

143 offline workspace tests pass; warning-free Clippy, formatting and release
CLI build pass. Native decoding matches the isolated vendor file-parser
reference for 36 float bit patterns, six dimension integers and three labels.
Evidence: evidence/factory-lens-oracle-comparison.json. Tests reject all
truncations and checksummed unsupported versions/models/labels, invalid
dimensions and nonfinite lens values. This is a field-extraction comparison;
it does not qualify a camera model, stereo transform, genuine point cloud
or physical accuracy. No firmware or calibration was changed.

## Raw pose field verification

144 offline tests pass; warning-free Clippy, formatting and release CLI build
pass. All 54 lens/pose floats match the isolated vendor parser bit-for-bit.
A separate named file export matches 48 named floats and six dimension
integers. Both comparisons reject an injected changed rotation. Synthetic
tests reject each nonfinite pose component even with valid CRC and preserve
signed-zero bit patterns. Evidence: evidence/factory-pose-oracle-comparison.json.
This verifies extraction and field names only; transform conventions, units,
image-mode binding and physical accuracy remain open. M2 is incomplete.

## Pose numerical-prefix comparison

146 workspace tests pass; warning-free Clippy, formatting and release CLI
build pass. Six synthetic matrix cases captured from an isolated numerical
prefix match independent Rust within 1e-12. The mixed-angle case rejects all
five alternative Euler permutations. Nonfinite pose fields are rejected and
translation is preserved without inferred unit conversion. Public fixture:
evidence/factory-pose-prefix-cases.json; scope/hash:
evidence/factory-pose-math-summary.json. This is numerical convention evidence
only. Relative camera direction, physical units, active mode binding and
M2 genuine calibrated geometry remain unverified.

## Relative pose numerical comparison

148 workspace tests pass; formatting, warning-free Clippy and release CLI
build pass. Six isolated synthetic references validate both pose matrices,
first inverse and the T(second)*inverse(T(first)) composition. Rust relative
matrices match within 1e-11. Noncommuting cases reject reversed multiplication;
nonfinite inputs are rejected for either pose. Public scope/results:
evidence/factory-relative-pose-summary.json and factory-relative-pose-cases.json.
No physical units, active-mode applicability, camera-frame direction or
accuracy qualification follows from this comparison. M2 remains open.

## Brown optical formula comparison and insufficient real-image evidence

150 workspace tests, formatting, warning-free Clippy and release CLI build
pass. 4608 in-image synthetic Brown-generator samples agree with independent
rational Brown math within 1e-7 normalized component error; 210 retained
reference points exercise Rust at tolerance 2e-7. Guard tests reject poles,
nonfinite/overflow inputs and lossy Brown5 reduction. Existing Brown5 forward
math agrees for supported zero-denominator cases.

Reference evidence: factory-brown-summary.json and factory-brown-cases.json.
Real-image evidence: factory-epipolar-30fps-reference.json. The actual dark
equal-PTS pair has zero mutual image-feature matches, so epipolar model
validation is unavailable. No geometry or accuracy pass follows. M2 remains
incomplete; physical calibration and firmware remain unchanged.

## Temporary standard gain acquisition trials

153 offline tests, formatting, warning-free Clippy and release CLI build pass.
Real 1000/3000/8000/15000 raw gain trials each capture/replay 60 valid frames
per IR camera; originals restore by readback. A post-refactor 30-frame trial
also restores correctly. Before/after standard-control values agree on all
three camera interfaces. Unit failure injection verifies rollback of an
ambiguous second write and continued first-camera restore when the second
restore fails. Evidence: evidence/ir-gain-trial-summary.json and per-gain
epipolar diagnostics. Startup rejections remain and this is not a reliability
qualification. High gain saturates portions of the left image without a
usable correspondence set. No M2 geometry/accuracy pass is established.

## Illumination status and temporary flood acquisition

157 workspace tests, formatting, warning-free Clippy and release CLI build
pass. Native fixed GET_PROPERTY snapshots succeed with controlled errors
for unsupported/inconsistent properties. Exact-length parsing and fixed
opcode/ID tests pass; only the consistent flood boolean has a scoped setter.
A real trial captures/replays 60 valid frames per camera with enabled and
restore readbacks, preserving all sampled property and standard-control
values afterward. Evidence: native-flood-trial-summary.json,
illumination-status-native.json and illumination-status-after-flood.json.
Images brighten but produce zero mutual SIFT matches, so no camera-direction,
geometry or physical accuracy pass is established. M2 remains incomplete.

## IR display and raw-data preservation

159 workspace tests, formatting, warning-free Clippy and optimized workspace
build pass. Exact histogram quantiles agree with independently sorted input;
display LUT tests preserve source bytes, handle flat data and remain monotonic.
Private-display native IR smoke renders the actual camera, statistics and
contrast option. Its saved snapshot equals a genuine raw recording frame
exactly after complete integrity validation. Evidence:
evidence/ir-preview-render-summary.json. This covers rendering and raw snapshot
preservation only. Startup rejects/gaps persist; sustained performance, optical
focus/correspondences, M2 geometry and physical accuracy remain unqualified.

### Native stripe extraction (synthetic scope only)

PASS: analytic weighted centroid/variance, separated runs, raw preservation,
explicit rejection classes, invalid parameters, cancellation and output overflow.
162 workspace tests pass; Clippy with warnings denied passes. See
`docs/LASER_DETECTION.md` and `evidence/laser-detector-tests.log`.
Real Raptor blue-laser extraction and dimensional reconstruction: NOT VERIFIED.

### Recorded stripe diagnostic integration

PASS (limited scope): complete-source integrity before analysis, retained raw
frame lineage, analytic CLI output, late corruption/truncation rejection, and
execution against an actual CRC-valid Raptor IR recording. All 163 workspace
tests, Clippy and release CLI build pass. Real projected laser identity and
geometry remain NOT VERIFIED. See `evidence/stripe-recording-summary.json`.

### Automatic stereo sampling (synthetic)

PASS: shared file workflow reconstructs known 400 mm synthetic plane without
manual seeds; report preserves actual seed-index lineage. Blank input yields no
geometry; conflicting/manual inputs, bounds and cancellation are controlled.
All 166 workspace tests, warning-denying Clippy, formatting and release build
pass. Evidence: `evidence/automatic-stereo-tests.log` and
`evidence/automatic-stereo-build.log`. Physical geometry remains NOT VERIFIED.

### Actual decoded lens numerical domain (limited evidence)

PASS: all 5,632,000 integer pixel-center inverse/reprojection checks at explicit
experimental radius 0.8; native worst residual <2.1e-9 pixels. Independent
OpenCV aggregate inverse-radius agreement <2.1e-13; insufficient-radius negative
control reports failures. See `evidence/factory-lens-domain-*.json` and
`docs/FACTORY_CALIBRATION.md`. This is mathematical evidence, not physical lens
accuracy, measured coverage, pose direction, unit or scanning-mode qualification.
166 existing workspace tests and static checks pass.

### Synthetic factory rectification-prefix relation

PASS (five finite numerical cases): ordered relative rotation and translation
satisfy captured rectification identities within 1e-12. Sixth zero-angle case
is explicitly NONFINITE in the isolated reference; native relative transform
remains finite. No physical sensor/direction/scale qualification follows.
167 workspace tests, warning-denying Clippy and formatting pass; evidence in
`evidence/factory-rectification-prefix-summary.json` and associated test log.

### Bounded frame tracking (synthetic prototype)

PASS within synthetic scope: 20-frame known-pose composition, rejection of
poor coverage/excess motion/high residual, loss retaining reference, recovery
without origin reset, frame-order validation, cancellation and explicit reset.
170 workspace tests, Clippy and release build pass; see tracking evidence logs
and `docs/TRACKING.md`. Genuine tracking/fusion/live throughput: NOT VERIFIED.

### Bounded voxel fusion (synthetic)

PASS: analytic centroid/color/normal aggregation and transform handling,
transactional input/capacity/cancellation rejection, frame ordering, synthetic
tracker-to-world fusion, rejected-frame exclusion and exact PLY round trip.
173 workspace tests, Clippy, formatting and release build pass; see fusion logs
and `docs/FUSION.md`. Genuine scanning/fusion accuracy and throughput remain
NOT VERIFIED; no live scanning milestone is complete.

### Coupled session and offline cloud sequence

PASS (synthetic): lifecycle, atomic tracker/fusion/history rollback, bounded
history, explicit loss/recovery, CLI pose lineage, artifact CRCs, exact derived
PLY/project equality, no overwrite and controlled invalid/corrupt inputs.
178 workspace tests, Clippy, formatting and release build pass. Release demo:
3 integrated frames / 1 lost frame / 6 fused points; see session-sequence logs
and `docs/SCAN_SESSIONS.md`. Genuine native scanning remains NOT VERIFIED.

### GUI sequence job and output parity

PASS (synthetic/startup scope): immutable preparation before publication,
original-document preservation, unsaved replacement guard, startup argument
validation, rendered fused cloud and status. GUI/CLI bundle artifacts match
byte-for-byte. All 179 workspace tests, Clippy, formatting and release build
pass. Evidence: `evidence/gui-sequence-render-summary.json` and test/build logs.
No hardware, full interactive workflow or live accuracy qualification claimed.

### Short reacquisition and exact indexed registration

Short hardware check: 60/60 accepted per IR camera near 30 FPS, no rejected
buffers/metadata gaps in this interval. Geometry NOT VERIFIED: dark/blurred
scene and only one mutual feature match. This is not a reliability soak or
exposure-sync qualification (`reconnected-scene-*.json`).

PASS (synthetic search scope): exact indexed/brute-force nearest results,
deterministic ties, large finite distances, controlled invalid/cancelled inputs;
existing ICP/tracking/fusion/session regressions pass. 181 workspace tests,
Clippy, formatting and release build pass. Benchmark evidence is specific to
synthetic nearest queries, not whole ICP or live performance.

### Organized-grid surface and mesh export (synthetic)

PASS: analytic sloped-plane area, missing-cell preservation, edge rejection,
orientation, degeneracy and edge/duplicate/winding/isolation diagnostics;
STL/OBJ coordinate/index text preservation, integrity manifests, no overwrite,
empty-surface and invalid/cancelled input handling. 187 workspace tests, Clippy,
formatting and release build pass. Release example: four vertices/two triangles.
M5, general watertightness, real Raptor meshing, repair and physical surface
accuracy are NOT COMPLETE/NOT VERIFIED. See organized-mesh evidence and docs.

### Connected stereo surface export (synthetic)

PASS: accepted-point/seed/mesh lineage, original-camera coordinates, known
plane depth, missing-sample holes, explicit sparse-grid meshing and conflicting
layout rejection. Release synthetic CLI/GUI: 93 points and 146 triangles,
byte-identical bundles, inspected point-cloud render and mesh status. All 189
workspace tests, Clippy, formatting and release build pass. Evidence in
stereo-surface logs/summary JSON. Genuine scanner surface accuracy, exposure
sync, mesh editing and M5 qualification remain NOT VERIFIED/INCOMPLETE.

### Native mesh inspection (limited render scope)

PASS: strict background mesh-envelope loading, invalid/cancelled load retention,
shaded/wireframe shape emission, source preservation, startup syntax, and actual
inspected synthetic surface render with full topology counts. 192 workspace
tests, Clippy, formatting and release build pass. Mesh-viewer evidence records
93 vertices/146 triangles with no camera access. Full control interaction,
large/intersecting mesh rendering, editing and physical accuracy remain unverified.

### Mesh plane trimming and exact undo (synthetic/backend scope)

PASS: analytic retained area, shared intersections, oblique uncapped cut,
controlled boundary/empty/invalid/cancelled cases, exact restoration over 100
undo/redo cycles, history bounds/redo invalidation, original-byte preservation,
CLI artifact integrity and no overwrite. 197 tests, Clippy, formatting and
release build pass. Actual synthetic 146→90-triangle cut rendered in native
viewer. Interactive GUI editing, mesh project persistence and physical precision
remain INCOMPLETE/NOT VERIFIED. See mesh-trim evidence and docs.

### GUI plane preview/apply and saved-version undo

PASS within synthetic controller/render scope: unapplied preview leaves model
and files unchanged; save precedes version commit; existing-target failure and
changed source are controlled; 100 exact GUI undo/redo cycles and bounded history.
199 workspace tests, Clippy, formatting and release build pass. Actual private-
display preview inspected with source bytes unchanged/no trim publication.
See gui-mesh-trim evidence. Full pointer interaction, serialized mesh project
history and physical surface accuracy remain NOT VERIFIED/INCOMPLETE.

### Persisted backend mesh undo/redo

PASS (synthetic): exact current/history/f64 round trip; corruption/all truncations,
trailing/oversize and CRC-valid bad history/schema/units rejection; reopened CLI
trim chaining, undo/redo restoration and prior-byte preservation. 203 workspace
tests, Clippy, formatting and release build pass. Actual release history example
has byte-identical restored mesh JSON; see mesh-history evidence. GUI full-history
project integration and complete mission project coverage remain INCOMPLETE.

### Full GUI mesh-history project round trip

PASS (synthetic controller scope): current/undo/redo saved, reopened in a fresh
controller after original sources were deleted, exact geometry restoration,
further snapshot-based trimming and current-state preservation on failed save/
corrupt load. 204 tests, Clippy, formatting and release build pass. Actual release
project startup render inspected with 54 triangles/2 undo steps; no camera access.
Full pointer interactions, large-history performance and full combined scanning
project/physical accuracy qualification remain INCOMPLETE/NOT VERIFIED.

### Selective mesh duplicate/unused cleanup

PASS (synthetic/backend/controller scope): exact first-face winding and retained
coordinates, independent options, original preservation, preview/apply, exact
undo/redo and persisted chaining. 208 tests, Clippy, formatting and release build
pass. Release cleanup restores deliberately augmented stereo surface to a
byte-identical clean mesh JSON; actual result render inspected. General repair,
full pointer workflow and physical geometry qualification remain NOT VERIFIED.

### Explicit mesh component analysis and keep/remove

PASS (synthetic/core/controller scope): exact indexed-edge connectivity, stable
IDs/bounds, vertex-touch/coincident separation, explicit small-component retention,
keep/remove order/coordinate preservation, compaction, original/undo/history,
invalid/cancelled requests and no overwrite. 213 tests, Clippy, formatting and
release build pass. Release 146/1-face example restores clean mesh bytes after
small-fragment removal; kept fragment render inspected. General repair,
pointer/large-data workflows and physical qualification remain unverified.

### Native line-pattern control/capture

PASS (limited operational/optical scope): supported fixed SDK line boolean
0→1→0 with restoration; all sampled vendor/standard controls unchanged; complete
60-frame-per-camera integrity and genuine projected pattern in both on images.
Native stripe counts 294/606 on versus 0/0 after restore with identical settings.
214 tests, Clippy, formatting and release build pass. See native-line evidence
and `docs/NATIVE_LINE_TRIALS.md`. Blue wavelength, exposure sync, mode binding,
line correspondence, dimensional geometry and scan reliability remain unverified.

- Native triangle STL/OBJ import: PASS for the explicit subset documented in
  docs/MESH_IMPORT.md. Tests cover units, indices, welding policy, malformed input,
  original preservation and further saved edits. Native fixture CLI round trips
  retain ordered face coordinates exactly; 399 total Rust tests pass. Evidence:
  mesh-import-tests.log, mesh-import-cli-checks.json, native GUI import render.
  This does not qualify broad OBJ attributes, external-tool precision or physical
  scanning accuracy.

- Independent rejection indicators and sample retention: PASS in the stated
  diagnostic scope. 404 Rust tests / 12 tool tests pass. A new short native run
  retains all five ongoing short/error buffers per camera despite valid retained
  PTS structure. New phase counters are exact explicit-reject counts; no startup
  exclusion or relaxed validation is used. The hardware rejection gate still
  fails. See docs/REJECTION_DIAGNOSTICS.md.

- Cooperative stereo SIGTERM/SIGINT cleanup: PASS for two real interruption cases,
  with reported format/rate restoration on both IR cameras and successful short
  reopen. Native errors are explicit and return exit 1 rather than signal death.
  Evidence: stereo-cancel-hardware-20261026/result.json. No claim for other signals,
  single-camera CLI paths, illumination cancellation or full recovery qualification.

- Single-camera SIGINT/SIGTERM interruption: PASS in twelve left/right IR/color
  validate/capture cases. Reported states restore; all six aborted recordings stay
  unpublished and fail incomplete replay as expected; three subsequent completed
  recordings replay with CRC/footer integrity PASS. No new image decode claim.
  Stereo cancellation also passes two regression cases on the same binary.
  See evidence/single-cancel-hardware-20261026/ and
  docs/SINGLE_CAMERA_CANCELLATION.md. Full recovery qualification remains open.

- Interrupted-recording prefix recovery: PASS for the stated offline/workflow and
  three genuine IR/color cases. Sources unchanged, validated frames retained,
  decoded replay succeeds, no overwrite, source mutation/cancellation/I/O failures
  prevent publication. Raw evidence images are not distributed. 417 Rust tests and
  twelve tool tests pass. See docs/RECORDING_RECOVERY.md; no physical accuracy gate.

## Indexed recording access — narrow offline checks

PASS: bounded/cancellable index construction with full frame/order/footer
validation; nonsequential single-record reads; changed selected-frame rejection
including recomputed on-disk CRC. Four new regression tests and workspace/static
checks recorded in evidence/recording-index-*.log. This is an access-layer check,
not GUI playback or whole recording/live/reconstruction qualification. See
docs/INDEXED_RECORDING_PLAYBACK.md for source immutability limits.

## Native recording image playback — narrow PASS

Seven added workflow/UI/parser regressions verify decoded frame navigation,
source changes/replacement, cancellation, stale-image clearing, format bounds
and corrupt-file errors. Release build, workspace tests and strict lint pass.
The same workflow decodes last/first/middle/last from eight genuine frames on
each IR/color camera with repeated RGB checksum equality, unchanged sources and
restored settings (evidence/recording-playback-hardware.json). Isolated-display
synthetic renders show the native panel; no camera descriptor is opened. This
is not full GUI interaction, timed replay, stereo timing, live performance or
physical accuracy qualification. See docs/RECORDING_PLAYBACK.md.

## Recorded stereo reconstruction adapter — narrow synthetic PASS

Explicit fully validated recorded GREY frames now reach the native shared image
reconstruction pipeline. Synthetic reports equal the PNG path; selected frame
payload/metadata/context and supplied calibration/request bytes are retained.
Six workflow tests cover rejection and cancellation; parser/unsaved-document
checks cover GUI entry. CLI and isolated-display native startup produce/open the
same three synthetic points, preserve sources and open no cameras. Evidence:
evidence/recorded-stereo-{cli-proof,gui-render} and tests/static/build logs.
Synchronization, device binding and physical accuracy are not independently
verified. This does not pass M2 or calibrated geometry/end-to-end gates.

## Initial project provenance — narrow persistence PASS

ORPROJ05 retains bounded initial recorded-reconstruction provenance through
geometry and measurement histories. Five geometry regressions cover version
compatibility, validation/bounds, every small-fixture truncation/byte corruption
and history round trips. Workflow checks compare saved/live origin and all five
artifact CRC/length references. A GUI worker reconstruct/edit/save/reopen/undo/
redo regression passes with the original bundle path missing. Native synthetic
reconstruction and fresh-app reopening are captured without camera access;
independent envelope/artifact checks are in evidence/project-origin-proof.json.
This is initial-source persistence, not full raw-session packaging, current
per-point lineage, physical accuracy or end-to-end scanner qualification.

## Explicit source-bundle identity checks — narrow PASS

Five new workflow tests cover matching/moved bundles, file failures and types,
project errors, cancellation and deterministic changes after an artifact was
hashed. GUI worker/UI tests preserve document/dirty state through verification,
and startup parsing requires explicit paths. Synthetic CLI proof passes at a new
location, fails same-size corruption with nonzero exit and passes restored bytes;
source/project SHA256s remain unchanged. Native expanded-panel verification reports
5/5 without camera descriptors. See docs/SOURCE_BUNDLE_VERIFICATION.md and
source-verification evidence. This does not qualify authenticity, semantic
calibration, exposure timing, physical geometry or full raw/session persistence.

## Selected-frame editable project bundles — narrow PASS

Six new workflow tests cover exact edited/history preservation, raw-source copies,
original preservation, rejection and preflight bounds, cancellation, changing
sources and no-overwrite publication races. GUI worker/UI/parser checks preserve
the active document and support explicit startup. Independent edited synthetic
CLI proof checks complete state equality except the copy's source directory,
source SHA256 preservation, exact artifacts, overwrite refusal and moved-bundle
verification. Native export and reopen use no cameras. See docs/PROJECT_BUNDLES.md
and project-bundle evidence. This does not qualify full recording/session projects,
mesh packaging, relocation automation, current per-point lineage, scanner geometry,
physical accuracy, live tracking or end-to-end operation.

Default-window expanded-panel layout: the saved native baseline has no viewport
fill inside the app window. Bounded controls/status scrolling passes a new viewport
bounds regression and final edited-project renders. This is a demonstrated layout
fix, not full GUI interaction or arbitrary-window qualification.

## Complete supplied raw recording archives — narrow PASS

Six workflow tests plus GUI worker/parser tests verify full original-byte/frame/
context preservation, v1 status, corruption/budget rejection, cancellation,
changing sources and no-overwrite races/special-file handling. Genuine sequential
8+8 IR recordings copy with matching source/output SHA256s and decoded replay;
reported camera settings restore and existing output is refused. Native synthetic
startup preserves complete three-frame files without opening cameras. Evidence:
full-recording-archive-*. This does not qualify original exposure completeness,
synchronization, physical geometry, full session/project integration or live gates.

## Recorded setup observations / stereo restoration — narrow PASS

GET-only descriptor observations now persist in real recording headers alongside
existing trial metadata and explicit unknowns. Seven added Rust tests cover failure/
bounds/merge/roundtrip and group restoration ordering/errors. Seven genuine CLI/
stereo/native-GUI entry files pass metadata/profile and decoded-replay checks.
A controlled old stereo probe and recording both leave a reported test rate behind;
a pre-worker group snapshot plus post-worker format/rate/readback guard passes the
same comparison. 12 single and two stereo signal cases and three prefix-recovery
checks pass on the final binary. Failure harness attempts remain documented and
excluded. Source: docs/CAPTURE_OBSERVATIONS.md and capture-observations evidence.
No physical calibration, per-frame setting attestation, optical quality, stable
reconnect/startup/throughput or full end-to-end qualification follows from this.

## Saved project plus full recordings — narrow PASS

- Six new regressions: four backend export/association/failure/relocation cases,
  one native background worker and one startup-argument case. 474 workspace Rust
  tests and 19 reference/tool tests pass, with lint, formatting and release build.
- Synthetic full recordings and five selected-frame artifacts are byte-identical
  across CLI/native-GUI exports and source files; six frames decode. Exported
  project source verification passes. Three private-display native captures were
  inspected; no scanner descriptors were open. Evidence: scan-project-*.log and
  evidence/scan-project-gui-render/{capture,inspection}.json and window PNGs.
- This is one saved project with two supplied recordings. Historical identity of
  unselected frames, recovery report embedding, complete session lifecycle,
  multi-session/mesh packaging, genuine geometry and all release gates remain open.

## Exported scan-folder load — narrow PASS

478 workspace Rust tests / 19 reference-tool tests, strict all-target lint,
formatting and release binaries/examples pass. Three new backend cases cover
verified moved-folder load with undo/redo, corruption/manifest mismatch and
path/symlink/change/cancellation failures. One native worker case checks source
relocation and preservation of the previous document/selection on failed import.
CLI verification passes on a moved synthetic folder; three inspected native render
captures show its three points, no camera descriptors, and 12 unchanged input files.
See evidence/scan-project-load-* and docs/SCAN_PROJECT_EXPORT.md. CRC-based artifact
integrity is not authenticity or historical full-capture identity. No genuine
geometry, live scanning, large-workload or release gate is advanced.

## Persistent complete-recording references — narrow PASS

Five new tests plus an extended native worker check cover ORPROJ06 metadata,
geometry/measurement undo/redo, schema/role/size/index validation, forbidden format
downgrades, saved edited reference verification and CRC-valid container mismatch.
483 workspace Rust tests, 19 reference/tool tests, lint, formatting and release
build pass. Native captures show both references and three synthetic points with
recordings absent. After restoration all nine archive/project artifact fingerprints
match. Explicit new-reference CLI verification and legacy v5 container verification
pass. See evidence/recording-references-* and docs/PROJECT_ORIGIN.md. No genuine
geometry, live scanning, accuracy/performance or stable-release gate is advanced.

## Huber point-to-plane registration — narrow PASS

Six new tests plus six independent fixed-correspondence numerical cases verify
optional Huber weighting, exact objective descent checks, raw-RMS visibility,
parameter/rank/cancellation handling, unchanged tracker Euclidean rejection and
CLI tracking loss/recovery/fusion. 489 Rust tests / 19 reference-tool tests, lint,
formatting and release build pass. Native and SciPy transformed points differ by
less than 9.6e-9 mm on the six synthetic cases; bias is explicitly retained, including
the initially failed 0.04 mm expectation. Native private-display synthetic fusion
integrates 3/4 frames, recovers after loss and produces 27 points; CLI/GUI artifacts
match byte-for-byte. Evidence: huber-plane-* and docs/POINT_TO_PLANE_ICP.md. These
checks do not qualify global registration, real scanner geometry or physical gates.
