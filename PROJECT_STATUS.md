# Project status

Stage: experimental. Specification: docs/MISSION.txt.

Latest robust-registration checkpoint: 489 workspace Rust tests and 19 reference/tooling
tests pass, with strict all-target lint, formatting and release binaries/examples.
Native manual recording playback is implemented; calibration remains deferred.

Latest calibration investigation: an explicit horizontal-reflection/reversed
camera hypothesis gives 165–171 distinct baseline stripe pairs on saved frames
10/50, versus at most four in shifted controls. This is temporal holdout evidence
from one scene, not physical calibration or verified geometry. Production
calibration remains unchanged. See docs/CAMERA_REFLECTION_HYPOTHESIS.md.
The subsequent image-only feature audit does not corroborate this hypothesis:
six line-on matches fail every tested convention; the saved line-off pair has
no matches. Native re-export verifies the diagnostic image pixels. This scene
cannot resolve physical camera binding; no calibration is activated.
Native `mirror-status` now queries three fixed mirror properties without setters.
Color reports disabled; left-IR range data is inconsistent and right IR errors,
so neither IR orientation is inferred. Static host paths corroborate property
IDs but not runtime application. See docs/MIRROR_STATUS.md.

Current checkpoint: 413 workspace tests plus 4 diagnostic example tests pass. Native acquisition/recording,
synthetic stereo-to-surface reconstruction, offline cloud tracking/fusion,
mesh viewing/plane trimming and checksummed CLI/GUI mesh history are implemented.
Genuine Raptor geometry remains unverified. Native line-pattern activation is
now observed in both IR streams; scene blur, calibration/mode binding and
physical calibration (deferred) remain unresolved. Acquisition reliability is not qualified;
a short clean 60-frame interval does not resolve earlier soak rejections.
See the latest sections and ACCEPTANCE_RESULTS.md for scope and evidence.

## Verified

- Rust 1.99.1 installed locally; modular workspace compiles.
- Rust discovers three image nodes and their three metadata nodes on Raptor.
- No Creality Scan or Wine process was running for first hardware captures.
- Left and right IR: 100 genuine 1920x1200 GREY frames each recorded with
  exact-sequence metadata and increasing sensor PTS; integrity replay passes.
- Startup rejected frames and sequence gaps are reported, not hidden.
- Offline format tests cover truncation, payload corruption, length limits and
  100 replay iterations. Full qualification remains incomplete.

## In progress

- Rust desktop camera workbench with live native color preview, bounded preview
  queue, recording and snapshot decoding passed an initial hardware smoke test.
- Color padding fix passed: 30 captured and decoded frames, no rejects/gaps
  in that short run. The first implementation rejected valid zero padding.
- Point-cloud workbench imports ASCII PLY, exports PLY, performs box selections,
  exact-index trim/keep/invert, voxel downsampling, undo/redo and project save/load.
  Synthetic tests pass; GUI workflows remain under verification.
- Known-correspondence rigid fitting, plane fitting and a capped CPU reference
  ICP implementation pass synthetic cases; real scan alignment is unqualified.

## Deferred and unverified

- User explicitly deferred calibration. No calibration writes are performed.
- Synchronization between cameras, hardware controls, IMU, initialization after
  a cold unplug/replug, and recovery have not been qualified in the Rust app.
- Native genuine point cloud, tracking/fusion, mesh reconstruction, advanced editing,
  scanner-derived measurements, mesh export and all accuracy gates remain unverified.
- No stable release or public publication has occurred.

Working Wine bridge is preserved in /home/developer/scancraft-driver-research. Native
project changes do not replace its installed launcher or kernel support.

## Current reliability defect

Extended left-IR capture reports occasional incomplete V4L2 payloads with the
error flag set, above the provisional 0.1% corruption limit. These are rejected
and separately counted; no known-bad frames are recorded. Cause not established.

## Checkpoint

Local git checkpoint ff556d5 preserves the first native implementation. An
optimized Rust build is installed behind the ScanCraft menu launcher.
Desktop became locked during later GUI checks; no attempt was made to unlock
it. Further manual GUI interaction checks remain pending.

## Hardware recovery needed

A separate nusb/usbfs diagnostic stalled on a UVC control request. Rebinding
the right-IR camera failed; the kernel subsequently logged setup-address and
USB enumeration errors -71. At that checkpoint the scanner was absent from USB. The user subsequently
reconnected USB/power and native camera recovery was verified below.
No firmware, calibration, installed kernel module or working Wine launcher
was modified. The direct USB experiment is feature-gated off by default.
No successful direct USB capture or restoration is claimed.

## Responsive editing checkpoint

- Point-cloud load, selection, trim, downsampling, filtering, plane fitting,
  undo/redo and save/export run in one bounded background job. The viewport
  keeps a cached preview of at most 50,000 points and can still orbit/pan.
- Cooperative cancellation is checked during long geometry loops and I/O.
  Proposals are committed only after validation and a final cancellation check.
  Project JSON parsing/hashing and OS file sync are bounded phases rather than
  immediately interruptible operations; cancellation-latency qualification is pending.
- Worker ownership survives an injected unexpected thread termination. Ordinary
  validation failures and cancellation preserve geometry, selection and redo.
- Save/export publishes a completed filename only after successful write and
  sync; failed/cancelled writes clean up their own partial, and never remove
  a preexisting partial or overwrite an existing completed file.
- Point picking returns original indices and prefers the frontmost overlapping
  rendered point. Ctrl-click toggles selection. Unit tests cover those paths
  and extreme finite coordinates. Unsaved edits are guarded on close/import.
- At the responsive-editing checkpoint, 49 offline tests, warning-free Clippy,
  formatting and optimized build passed.
  Manual desktop interaction and physical scanner recovery remain unverified.

## Hardware recovery

The user reconnected scanner USB/power. Raptor enumerates again and all six
video/metadata nodes are present. V4L2 right-IR returned 100 valid frames
(with 15 incomplete frames rejected) and color returned 20 valid frames.
The direct USB experiment remains disabled. Reliability remains unqualified.

## Screen selection checkpoint

Rectangle, clicked polygon, freehand lasso and brush selection use the same
orthographic projection as rendering, but evaluate the complete cloud in a
cancellable worker. Replace/add/subtract/intersect modes are supported.
Frontmost-per-pixel mode uses a one-logical-pixel point depth buffer; it is
an approximation to point visibility, not reconstructed-surface visibility.
UI selection includes a 0.5-pixel boundary tolerance.

Mouse/key gesture tests cover press/release coordinates, polygon completion,
freehand/brush endpoints and Escape/resize cancellation. Rapid clicks at
different polygon vertices initially triggered premature double-click
completion; completion now also requires spatial proximity and three vertices.

One million analytically labeled synthetic points selected in 12.745299 ms
(release); zero classification errors in that single rectangle case. This
is not broad selection or interactive latency qualification.

Current selection checkpoint: 62 offline tests pass; Clippy has no warnings,
formatting passes, and the optimized workspace build succeeds. The scanner
is enumerated as USB device 004:017 after recovery.

## Calibration mathematics checkpoint

A new Rust calibration crate implements pinhole/Brown-5 projection, bounded
inverse distortion, stereo extrinsics and checked ray triangulation. Its
integrity-checked record format carries explicit units, axes, provenance and
declared device/mode binding. Fifteen tests pass, including independent
analytical cases, corruption handling, and rejection of invalid models.
OpenCV 5.0.0 agrees on 60 synthetic cases (maximum 3D difference ~1.08e-8 mm).
These are numerical tests, not actual Raptor calibration or M2 completion.

A simultaneous Rust V4L2 probe acquires 100 structurally validated buffers
from each IR camera. Some accepted frames share exactly equal sensor PTS.
Optical exposure synchronization and the sensor clock model remain unverified.
Additional rejected buffers include short spurious fragments, so legacy
`rejected_frames` counters must be interpreted as returned V4L2 buffers, not
a direct count of lost physical exposures. Strict rejection remains enabled.

Current geometry/timing checkpoint: 79 offline tests pass; warning-free Clippy,
formatting and optimized workspace build pass. M2 genuine scanner point-cloud
reconstruction remains unimplemented, and IR framing/reliability is unresolved.

## Sparse reconstruction integration

A Rust `scancraft-depth` stage now connects checked calibration/triangulation to
the editable point-cloud representation. Four new tests cover analytic depth,
source-index and color preservation, per-match invalidity reasons, PLY roundtrip,
source/binding rejection, cancellation and all-invalid results. These use
synthetic observations only. Actual Raptor image matching, calibrated geometry,
GUI reconstruction and the M2 genuine point-cloud milestone remain incomplete.

Current workspace checkpoint: 83 offline tests pass, warning-free Clippy and
optimized workspace build pass. Physical scanner qualification remains open.

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

Image checkpoint: 93 workspace offline tests pass; warning-free Clippy and
optimized workspace build pass. The full scanning goal remains incomplete.

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

Current checkpoint: 103 offline workspace tests pass, Clippy has no warnings,
formatting passes and the optimized workspace build succeeds.

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

Current checkpoint: 110 offline tests pass, warning-free Clippy and optimized
workspace build pass. Full native scanner qualification remains incomplete.

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

Pending hardware run: acquisition-only `stereo-probe 30000`, live exec session
70408, PID 415505, last verified running at elapsed 05:07. Output paths:
`evidence/private/native-stereo-soak-01.json` and `.log`. Do not restart on an
observation timeout; poll the same handle or inspect the PID. No soak gate has
passed yet. Recording/capture controls must not be changed during this run.

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

Soak continuation: session 70408 / PID 415505 remains live after 11:46. At the
latest thread inspection only `scancraft-probe-0` remains with the joining main
thread; no combined result/error is available yet. Do not infer right-worker
success or restart the process. Raw result/log remain in the same private
paths. Poll the existing handle or inspect process state before querying controls.

## Completed dual acquisition soak and control snapshot

Session 70408 / PID 415505 finished successfully; it is terminal and must not
be restarted. Each camera accepted 30,000 buffers. Left host span 821.845 s at
36.502 Hz, 375 rejected buffers and 375 sequence gaps; right 606.139 s at
49.492 Hz, 301 rejected buffers and 300 gaps. No duplicate PTS were reported.
Raw-buffer rejection proxies are 1.2346% and 0.9934%, above the 0.1% proxy
threshold. Physical corruption rate and optical synchronization remain unknown.
The duration experiment passed; overall reliability/optical mode qualification
did not. Full samples are private, summary is native-stereo-soak-01-summary.json.

A subsequent native read-only controls snapshot succeeded. All image cameras
report absolute exposure 3000; IR gain is 1000. Left auto-exposure query returns
EINVAL while right/color report raw auto mode 2. The standard exposure-unit
interpretation gives 0.3 s, which is not a measured exposure and conflicts with
a simple 50 Hz frame-interval interpretation. Resolve advertised versus actual
sensor units before designing writes. No controls were changed. Snapshot:
evidence/native-standard-controls-01.json. Next work: diagnose left rate/banding
and exposure/format semantics using controlled reference experiments.

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

Current check: 123 Rust offline tests pass, warning-free Clippy and release
workspace build pass. Kernel candidate compile/policy checks are separate from
hardware qualification and do not establish a transport fix.

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

## Native factory-reader checkpoint

Native Rust now reads allowlisted factory headers and bounded 8 KiB regions
without Wine or proprietary runtime dependencies. Region 200000 was read
successfully; its header matches the saved reference. Private raw bytes are
excluded from Git; evidence/factory-read-summary.json records structural findings.
Optical calibration decoding remains incomplete and no calibration/firmware
was modified. Full workspace: 136 tests pass, formatting and warning-free
Clippy pass. Fixed-packet tests cover all 768 allowed region/chunk combinations,
invalid offsets and opaque header handling.

After reconnection, USB discovery, protocol-version and short dual-camera
capture succeeded. Both cameras supplied ten valid frames; right startup
rejections persist. This is recovery evidence, not reliability qualification.
Next: verify the factory checksum and field semantics against read-only vendor
behavior before any conversion into the application's calibrated stereo model.

## Factory integrity/index checkpoint

`scancraft-cli factory-inspect FACTORY_REGION_FILE` validates the observed laser
container's CRC and bounded record layout offline. Native reads from regions
200000 and 202000 both pass; each has four 1680-byte records starting at byte
60, ending at declared length 6780. Region 200000 includes LAS_L_SCAN and
CROSS_LAS_L_SCAN labels; 202000 labels are all LAS_L_SCAN. Checksums differ
and both verify independently. No assumption about active/default calibration
is made. Public evidence contains indexes only; raw bytes stay private.

139 offline tests pass, including every truncation and single-byte corruption
of a synthetic factory container, structural errors with recomputed CRC, and
explicit exclusion of flash padding from the authenticated range. Clippy
with warnings denied, formatting and release CLI build pass. Optical field
decoding, distortion conventions and physical geometry remain unverified.
Next: trace record consumers to determine camera mapping and extrinsic units;
retain the user's deferral of physical calibration.

## Second factory envelope checkpoint

A bounded native read of allowlisted region 20a000 succeeds without calibration
or firmware writes. Its declared length is 6646 bytes; CRC matches using the
same zeroed-checksum-field rule. Top/body signatures differ from laser records
(0x46447312 / 0x4644610f). The payload contains explicit IR_L, IR_R, COLOR
and Brown tokens. Their positions are research evidence, not a decoded optical
model. See evidence/factory-envelope-20a000.json; raw bytes remain private.

`factory-verify` now checks either observed envelope independently of payload
indexing. `factory-inspect` remains restricted to the verified laser record
layout and refuses the second schema. Tests cover this distinction and every
single-byte corruption of both synthetic envelope types. 140 offline tests,
warning-free Clippy, formatting and release CLI build pass.

Static investigation found the laser prefix resembles public Orbbec intrinsic/
distortion/transform structures, but numerical plausibility alone does not
establish camera order, units, distortion direction or active calibration.
The bundled ob_pro file parser has multiple modes and binary versions; using
it as an oracle requires establishing the correct input schema first.
Next: trace consumers of the explicitly named-camera second payload and
verify field offsets against an isolated read-only parsing reference.

## Named-camera lens decoder checkpoint

Native Rust `factory-lenses FILE` decodes three named lens blocks in the
verified 3.0.13 / 6646-byte factory envelope. It returns labels, dimensions,
four intrinsic floats and eight Brown-model floats per camera, preserving
file order. It does not construct CameraModel/Brown5, bind a capture mode,
interpret poses, or produce a stereo CalibrationRecord. Unknown versions,
models, labels/counts, dimensions and nonfinite lens values are rejected.
Historical serialized pointer/padding bytes are ignored, never dereferenced.

An isolated offline run of the installed vendor LoadCalibrationParamsMulti
file parser accepts the private payload. Independent comparison proves
bit-for-bit agreement for all 36 lens floats, all six dimension integers and
three labels. Both IR dimensions are 1920x1200; COLOR is 1280x800. The vendor
parser warns it lacks a size check for this model combination; our strict
bounds/version checks remain mandatory. Raw calibration, proprietary binary
and oracle memory dump are private. No hardware nodes or network were
available to the oracle; its own processes have ended.

143 workspace tests, warning-free Clippy, formatting and optimized CLI build
pass. See docs/FACTORY_CALIBRATION.md and
evidence/factory-lens-oracle-comparison.json. Transform associations, units,
coefficient formula/direction, image-mode binding and physical accuracy
remain unverified. M2 first genuine calibrated point cloud remains open.

## Named pose fields and text reference checkpoint

The existing named-camera decoder now extracts six additional raw pose fields
per camera. All 18 values match the isolated vendor parser bit-for-bit,
including signed zero. A separate private YAML file export labels them
Rotate.RX/RY/RZ and Translate.TX/TY/TZ. It also confirms Fx/Fy/Cx/Cy and
K1/K2/K3/K4/P1/P2 positions. The other two Brown array slots are not named
by this export and remain uninterpreted.

The vendor file-export function ran only inside the existing private offline
sandbox: no scanner nodes or network, original filesystem read-only except
private evidence. It wrote a new local YAML file, not device calibration.
No proprietary data or memory dumps were added to Git. Production decoding
remains independent Rust. Binary comparison checks 54 float bit patterns;
named scalar comparison checks 48 floats and six dimensions. Changed rotation
is rejected by both comparisons. Evidence: factory-pose-oracle-comparison.json.

144 offline tests, warning-free Clippy, formatting and release CLI build pass.
Tests cover file order, signed zero and each nonfinite pose component under
a valid recomputed CRC. The decoded pose values are not converted to a
rotation matrix or stereo rig: angle convention/order, direction, units,
active capture-mode binding and physical accuracy remain unverified.
Next: trace the scan-library pose consumer to resolve those conventions.

## Observed pose mathematics checkpoint

A synthetic-only offline prefix probe identifies the depth-engine pose
calculation as Rz(RZ)*Rx(RX)*Ry(RY), radians, with translation copied without
scaling into a column-major homogeneous matrix. The independent Rust helper
returns row-major matrices for column vectors and agrees within 1e-12 on
six reference cases (identity, individual axes, mixed angles and translation).
The mixed-angle case distinguishes all other Euler permutations by >0.05
maximum element error. This is numerical convention evidence, not physical
calibration or proof of the active scan-mode path.

The reference uses only synthetic fields. A breakpoint exits before the
remainder of the depth engine executes, inside the hardware-inaccessible,
network-isolated private Wine sandbox. No vendor code is redistributed.
The public fixture contains only synthetic inputs and numerical outputs.
See evidence/factory-pose-prefix-cases.json and factory-pose-math-summary.json.

`factory-lenses` includes diagnostic matrices in raw translation units,
explicitly marking transform direction and units unverified. No StereoRig
or CalibrationRecord is constructed. 146 workspace tests, warning-free Clippy,
formatting and release CLI build pass. Next: establish the relative-camera
composition/inversion and translation units, then validate image-mode binding
and distortion against actual image correspondences before genuine geometry.

## Relative pose composition checkpoint

A later breakpoint in the same isolated numerical routine captures both
synthetic camera matrices, inverse(first), and the ordered relative matrix.
All six probes identify T(second)*inverse(T(first)); the reversed order differs
by up to 45.858 in the chosen synthetic raw units. Both camera slots follow
the observed Rz*Rx*Ry convention. Native Rust rigid composition matches each
reference within 1e-11, rejects nonfinite fields in either pose and reduces
identical poses to identity. See evidence/factory-relative-pose-summary.json
and the public synthetic cases. No device data or vendor instructions are
in the public fixture.

`factory-lenses` includes second_times_inverse_first_raw_units as a diagnostic,
with translation units and physical camera direction still marked unverified.
The observed numerical operation is established; active-mode applicability,
physical frame interpretation, units, distortion conventions and real image
correspondence validation remain unresolved. No camera calibration is activated
and no genuine point cloud is claimed. 148 offline tests, warning-free Clippy,
formatting and release CLI build pass.

## Rational Brown formula and real-image check

Synthetic offline calls to the installed Brown LUT generator establish the
coefficient order k1,k2,k3,k4,k5,k6,p1,p2 and the forward rational radial
formula. Two separate f64 LUT arrays contain normalized output-minus-input
displacements. In-image comparison covers 4608 points across six coefficient
sets, including independently nonzero denominator and tangential terms,
with maximum component error <1e-7. The public Rust regression fixture has
210 representative points and tolerance 2e-7. No extrapolated/border-polynomial
behavior is claimed. Reference DLL/hash and scope: factory-brown-summary.json.

Native Rust now evaluates the eight-coefficient forward formula and guards
Brown5 reduction against nonfinite or any nonzero denominator coefficients.
All three actual named factory lenses permit lossless Brown5 reduction. This
verifies coefficient math, not calibration coverage, camera mode or physical
accuracy. No inverse-validity domain or StereoRig is inferred automatically.
150 workspace tests, Clippy with warnings denied, formatting and release CLI
build pass.

The actual saved 30 FPS stereo pair at equal sensor PTS (8 us host difference)
was checked with geometry-independent SIFT/CLAHE mutual matching. It yields
8/19 keypoints and zero mutual matches. Intensities remain low. This cannot
validate or reject either transform hypothesis. The image-only OpenCV helper
is offline reference tooling; production remains Rust. See
evidence/factory-epipolar-30fps-reference.json. Next: establish exposure and
illumination using controlled native/reference acquisition before image-
geometry validation. Physical calibration remains deferred by the user.

## Native temporary IR gain trials

`stereo-gain-capture COUNT RAW_GAIN NEW_DIRECTORY [--fps FPS]` performs a
standard V4L2 gain-only trial on the two discovered Raptor IR interfaces.
Reported raw gain bounds/step and availability flags are checked before any
write; the fixed trial envelope is 1000..15000. The gain lease borrows the
exclusive SessionLock, records both original values and verifies all writes
and restorations by readback. Capture verifies requested gain again after
stream startup and stores that observation in each CRC-protected recording
header. Both originals are restored before successful output publication;
failures keep partial output and explicit restoration errors. Drop provides
a fallback restoration attempt. Exposure, projector, firmware and calibration
settings are not written.

Actual 30 FPS trials at 1000/3000/8000/15000 each saved 60 accepted frames
per IR camera with full integrity replay. Original gain 1000 was restored
on both after each trial. All captured standard-control snapshot values on
interfaces 0/2/4 match before/after. A final 30-frame-per-camera regression
after rollback refactoring also passes restoration readback. Raw records
and image content remain private; evidence/ir-gain-trial-summary.json records
results and exact qualification limits.

Gain changes intensity, but even the highest settings provide only 2/3 mutual
feature matches and no under-two-pixel hypothetical epipolar inliers. At 15000
the left 99th percentile saturates. The 8000 left image visibly shows a blurred
scene; gain alone does not solve optical acquisition. A stationary textured
subject at normal scanning distance was requested for the next image check.
No camera mapping, geometry, exposure synchronization or reliability pass is
claimed. Next: controlled scene/illumination setup and image correspondences.

153 workspace tests, formatting, warning-free Clippy and release CLI build
pass. New failure-injection tests cover ambiguous second-camera write failure
(restoring both) and restoration failure without skipping the other camera
or falsely reporting success.

## Native illumination status and scoped flood trial

`illumination-status` queries 17 fixed SDK illumination-related property IDs
with native opcode-1 GET_PROPERTY packets and exact five-word replies. SDK
metadata/read-path references are in illumination-property-mapping.json.
Unimplemented properties may return status 0x2; several others return status
zero with inconsistent range data. The snapshot preserves raw replies but
explicitly marks inconsistent integer ranges unusable for setting. No status
number alone establishes physical emission.

Observed baseline: generic laser boolean 1, flood boolean 0, line-laser
boolean 0, line mode switch 0 in reported range 0..8. Flood returns a
consistent 0..1 boolean range. Native `stereo-flood-capture` writes only that
fixed boolean, checks readback, records its pre-stream observation in each
CRC-protected header, verifies enabled after capture, and restores the
original value before publication. The scope borrows SessionLock; Drop
provides fallback restoration with explicit error reporting. No generic
property setter, laser, mode, power/current, calibration or firmware writer
is exposed.

A real 30 FPS trial saved/replayed 60 valid frames per camera. Flood reported
0 -> 1 -> 0 with successful readbacks; all raw property replies and all
standard V4L2 control values match before/after. Images brighten and yield
245/669 keypoints but zero mutual SIFT matches in the current blurred view.
No geometry, synchronization, reliability or physical-emission qualification
is claimed. Raw records/images stay private. See native-flood-trial-summary.json
and flood-trial-epipolar.json.

157 workspace tests, warning-free Clippy, formatting and release CLI build
pass. Tests constrain GET packets to the fixed snapshot, check payload
length/order, reject inconsistent integer ranges, and constrain setter
encoding to property 6 with boolean values. Next: a controlled scene, known
illumination/mode behavior and independent image correspondence validation.

## Native IR display diagnostics checkpoint

The Cameras workbench already selects left/right IR. It now shows exact
raw grayscale p01/median/p99, mean and zero/255 clipping percentages. An
optional display-only p01..p99 stretch updates the texture immediately even
when paused. Flat percentile ranges use identity mapping. Changing camera
clears stale image/statistics. Raw recordings and snapshots retain original
pixel levels; no exposure/gain/projector setting is changed by the display
option. These intensity summaries are not focus or optical-validity scores.

A native left-IR smoke run on a private authenticated X display rendered
the new controls and visible stretched image, captured 21 accepted frames,
and stopped cleanly. The saved snapshot exactly equals recording frame 20
pixel-for-pixel after whole-recording CRC/footer validation. Raw p01/median/
p99 are 16/17/39, mean 18.4394. Screenshot/image content remains private;
summary: evidence/ir-preview-render-summary.json. User desktop/app instances
were not accessed or restarted. Startup rejects (119) and sequence gaps (12)
remain, so this does not qualify acquisition reliability or preview latency.

159 offline tests, formatting, warning-free Clippy and optimized workspace
build pass. Exact quantile tests use an independent sorted nearest-rank
reference; display tests cover monotonic mapping, clipping endpoints, flat
ranges and unchanged source pixels. M2 genuine geometry remains incomplete.
Next: controlled scene positioning, verified camera-mode/calibration binding
and genuine correspondence/reconstruction while retaining physical calibration
deferral.

## Experimental native stripe observation primitive

Added `scancraft-image::laser::detect_rows`: bounded raw-GREY row-run extraction,
background-subtracted subpixel centroids and second moments, explicit peak,
width, clipping and border rejection, cancellation and hard output limits.
Source intensity data is immutable. Three analytic/rejection/error regression
tests pass; all 162 workspace tests and warning-denying Clippy pass (see
`evidence/laser-detector-tests.log`, `docs/LASER_DETECTION.md`). No projector
line identity, real laser observation, geometry or physical accuracy is claimed.
This production Rust primitive still needs a recording diagnostic entry point,
controlled real laser validation and mode-specific calibrated integration.
M2 remains open; physical calibration remains deferred.

## Recording-backed stripe diagnostics

`scancraft-cli detect-stripes` now runs the native detector on a selected raw GREY
frame only after whole-recording integrity validation. JSON retains acquisition
context and source lineage, settings and rejection counts. A CLI regression
covers analytic results and late source corruption; all 163 workspace tests,
warning-denying Clippy, formatting and release CLI build pass. Actual existing
1920×1200 Raptor frame 20/21 produced no accepted runs at explicit example
thresholds (aggregate `evidence/stripe-recording-summary.json`); this scene has
no verified projected laser. No geometry or laser-mode qualification claimed.
Next requires genuine controlled laser observations and mode-specific calibration
binding; physical calibration remains deferred. Full report stays private.

## Automatic textured stereo sampling

Native stereo reconstruction can now choose bounded textured grid samples in
rectified images through optional `automatic_sampling` settings. The shared
workflow exposes this in both CLI and GUI job processing. Manual seeds remain
supported; conflicting inputs fail. Reports retain actual seeds and sampling
configuration, including the empty-texture case. Matching, reverse uniqueness
and original-frame triangulation checks remain mandatory. The deterministic
row-major cap can bias spatial coverage; no corner/laser identity is inferred.
Three new tests cover synthetic plane reconstruction without manual pixels,
blank/no-geometry, seed lineage, conflicts/resource limits, masked support,
texture, deterministic caps and cancellation. All 166 workspace tests, Clippy,
formatting and release workspace build pass (automatic-stereo evidence logs).
Real Raptor geometry remains unverified, M2 open and physical calibration deferred.

## Actual factory lens inverse-domain evidence

Added a bounded offline Rust numerical audit example and independent OpenCV
reference tool. With explicit experimental radius 0.8, the actual decoded lens
fields pass the existing analytic monotonicity bound; every one of 5,632,000
integer pixel centers inverts/reprojects, worst Rust residual <2.1e-9 pixels.
OpenCV aggregate maximum inverse radii agree within 2.1e-13. Deliberately
insufficient radius 0.3 reports domain failures for all lenses. Aggregate-only
public evidence is in `factory-lens-domain-*.json`; raw factory data stays
private. This resolves a numerical-domain prerequisite for these fields, not
physical coverage, units, transform direction or active-mode binding. No
calibration record activated, no hardware writes; physical calibration deferred.
166 workspace tests, Clippy and formatting pass; release audit executed.

## Further synthetic factory pose interpretation

A hardware-inaccessible offline depth-engine prefix probe now reaches the two
rectification rotations before lens/image processing. Five finite synthetic
cases satisfy Q_second*R_relative=Q_first and Q_second*t_relative=positive
horizontal baseline, strengthening ordered numerical transform interpretation.
The zero-angle case produces nonfinite reference rotations; native relative
math stays finite and native rectifier retains its existing identity-baseline
tests. This is isolated reference behavior, not a scanner defect claim.
Synthetic fixture and one regression committed; all 167 workspace tests,
Clippy and formatting pass. Details/limitations in `docs/FACTORY_CALIBRATION.md`.
Physical sensor assignment, active-mode binding and translation units remain
unverified, M2 remains open, physical calibration deferred. No hardware writes.

## Bounded native tracking state machine

Added a production Rust tracker prototype over small calibrated-cloud inputs,
with explicit Empty/Tracking/Lost state, frame-to-world pose composition,
convergence/RMS/motion gates and final bidirectional coverage verification.
Rejected frames yield no new pose and preserve the reference for recovery;
reset deliberately re-anchors a new session. Invalid input/cancellation leaves
state unchanged. Existing reference ICP now offers cooperative cancellation.
Three synthetic regressions cover a 20-frame known trajectory, loss/recovery,
quality rejection and lifecycle errors. All 170 workspace tests, Clippy,
formatting and release workspace build pass; evidence in tracking logs.
This is an unqualified bounded CPU prototype, not live scanning: no camera
connection, fusion, global relocalization or real-data qualification yet.
See `docs/TRACKING.md`. M2/M3 remain open; physical calibration remains deferred.

## Transactional bounded voxel fusion

Added native deterministic centroid fusion with explicit world poses, signed
floor voxel keys, capped persistent/staging maps, running observation means,
independent optional color/normal counts and normalized rotated normals.
Frames commit atomically after validation and cancellation checks; capacity or
invalid-data failures leave model/frame ID unchanged. A tracking-aware entry
point rejects lost/unaccepted updates. Shared Rigid::validate also checks poses
for reference ICP. Three regressions include analytic attributes and rollback,
a ten-frame tracker/fusion sequence, lost-frame exclusion and exact PLY output.
173 workspace tests, Clippy, formatting and release build pass (fusion logs).
This is a backend prototype: no session-level tracker/fusion transaction,
accumulator persistence, camera/GUI integration or physical/live qualification.
See `docs/FUSION.md`; M2/M3 remain open, physical calibration deferred.

## Coupled sessions and runnable cloud-sequence workflow

Added ScanSession with explicit Ready/Running/Paused/Stopped lifecycle and
bounded trajectory history. It tentatively copies only the small tracker,
committing its reference/history after successful atomic fusion; capacity or
invalid/cancelled integration leaves all session state unchanged. Lost frames
record explicit rejection without fusion and can recover without re-anchoring.
Shared offline `fuse-sequence` workflow processes bounded unit-aware PLY files
with explicit tracking indices, source declarations and no-overwrite output.
Reports retain poses/configuration/index lineage/input CRCs; bundles include
fused PLY and existing editable project. A reproducible synthetic Rust dataset
generator exercises anchor/translation/loss/recovery in the release CLI: three
integrated, one rejected, six points. Five new tests; all 178 workspace tests,
Clippy, formatting and release build pass (session-sequence evidence logs).
No camera/live integration, resumable fusion persistence or genuine calibrated
cloud validation yet. M2/M3 remain open; physical calibration deferred.

## Native GUI cloud-sequence processing

The point-cloud workbench now exposes Track and fuse cloud sequence, with
background progress/cancellation, existing unsaved-document guard and integrated/
rejected status. `--fuse-sequence SETTINGS NEW_RESULT_FOLDER` starts it without
camera acquisition. Shared immutable prepare/publication separation lets the
GUI validate its proposed editor/preview before durable bundle publication.
One new worker regression plus extended lifecycle/startup tests pass; all 179
workspace tests, Clippy, formatting and release build pass. An actual private-X
software render was visually inspected: six synthetic points and 3-integrated/
1-rejected status, no scanner fds. GUI and CLI report/PLY/project/manifest bytes
match exactly for the same fixture (gui-sequence evidence). Full interactive
workflow and genuine live scanner data remain unqualified; M2/M3 still open.

## Reconnected hardware scene check and indexed registration

A short new dual-IR capture after reconnection accepted 60 frames/camera at
about 30 FPS, with zero rejected buffers, gaps or missing metadata and 60 exact
PTS matches. Complete recordings/footer validated during extraction. The
visually inspected scene remains dark/blurred; SIFT yielded only one mutual
match, with no validated geometry. This short clean interval does not explain
prior intermittent errors or pass reliability/sync qualification. No exposure,
gain, illumination or firmware changed. Aggregate evidence is in
reconnected-scene summary/epipolar JSON; raw images remain private.

Independent work added a bounded exact balanced spatial index and connected
it to ICP and tracker final verification. Deterministic ties and robust-distance
handling preserve exact query semantics; two tests compare against independent
brute force and cover invalid/cancelled cases. A reproducible synthetic 10k-point/
1k-query microbenchmark agrees exactly and measures ~3.3–3.7 ms including build
versus ~349 ms scalar brute force. This is not whole-ICP/live qualification.
181 workspace tests, Clippy, formatting and release build pass; details in
`docs/SPATIAL_INDEX.md`. Genuine geometry still needs physical scene suitability,
sensor/mode binding and scale/direction evidence; calibration remains deferred.

## First organized surface meshing and native STL/OBJ export

Added a bounded native mesh representation, validation and edge-topology
analysis plus organized-grid triangulation preserving missing cells and rejecting
long edges/ill-conditioned triangles. Shared `mesh-grid` workflow publishes
exact mesh JSON, ASCII STL and OBJ with diagnostics and integrity manifest;
no surface produces diagnostics without fake exports. Edge closure is not
labeled qualified watertightness: vertex links/self-intersections remain unchecked.
Six new tests cover analytic plane geometry, holes/discontinuities, topology,
export coordinate/index preservation and CLI error/publication behavior.
All 187 workspace tests, Clippy, formatting and release build pass. Release
synthetic example produced 4 vertices/2 triangles (organized-mesh evidence).
This does not complete M5: genuine calibrated source-grid connection, general
unordered cloud reconstruction, mesh GUI/persistence, repair/decimation and
physical accuracy remain unfinished. See `docs/ORGANIZED_MESHING.md`.

## Connected stereo-to-surface export

Optional stereo `meshing` configuration now connects accepted triangulation
results to explicit rectified seed-lattice adjacency. It validates layout,
preserves rejected/missing samples as holes, retains original left-camera-frame
coordinates and records mesh-vertex-to-seed lineage. Shared CLI/GUI bundles add
surface diagnostics and mesh JSON/STL/OBJ only when triangles exist; existing
point-cloud projects stay unchanged. GUI status reports exported triangle count.
Two new regressions cover exact source lineage, analytic plane depth, empty
surfaces and explicit/conflicting layouts. All 189 workspace tests, Clippy,
formatting and release build pass. Actual synthetic release execution produced
93 points/146 triangles; native private-X render was inspected and all GUI/CLI
bundle bytes match. See stereo-surface evidence and `docs/OFFLINE_STEREO.md`.
Real Raptor geometry remains unverified; mesh viewport/editing/repair and general
surface reconstruction are unfinished. Physical calibration remains deferred.

## Native mesh inspection pane

Added an independent Meshes tab and `--mesh MESH_JSON` startup. Strict bounded
background loading validates schema/units/geometry, computes full edge-topology
diagnostics and prepares sampled render data before replacing an old model.
Shading/wireframe, orthographic orbit/pan/zoom and fit controls inspect source
without changing the point-cloud document. Three new regressions plus startup
coverage pass; all 192 workspace tests, Clippy, formatting and release build
pass. Actual private-X release render was inspected: 93 vertices/146 triangles,
38 boundaries, shaded/wireframe surface visible; no scanner fds or user desktop.
The preview samples at most 50k triangles and depth sorting has occlusion limits;
this is not a qualified depth-buffer GPU mesh engine. Mesh editing/persistence,
repair/measurement and real-scanner qualification remain unfinished. See
`docs/MESH_VIEWER.md` and mesh-viewer evidence.

## Reversible native mesh plane trimming

Added immutable plane clipping with shared edge intersections, preserved winding,
compaction and explicit open cuts/sliver counts. MeshEditor provides exact bounded
undo/redo with retained-payload accounting and reported history evictions.
`trim-mesh` publishes a new bundle, retaining original source bytes and edited
JSON/STL/OBJ plus diagnostics/integrity. Five new regressions cover analytic and
oblique cuts, shared boundaries, edge cases, 100 exact undo/redo cycles and CLI
preservation/error behavior. All 197 workspace tests, Clippy, formatting and
release build pass. Actual synthetic release cut: 146→90 triangles; original
bytes retained and result inspected in the native mesh viewer with no scanner
access. See `docs/MESH_TRIMMING.md` and mesh-trim evidence. GUI editing controls,
preview/apply flow, mesh project/history persistence and physical qualification
remain unfinished. M5/M6 are not complete.

## Native GUI plane preview/apply and saved-version history

The mesh pane now prepares an explicit unapplied plane-cut preview off-thread,
checks source-file geometry consistency and builds render state before output.
Apply publishes a new original-preserving bundle before committing current model;
failed/cancelled output keeps the current version. Exact immutable saved-version
undo/redo has 32-step/128-MiB payload bounds with explicit evictions. Parameter
changes/new opening are blocked while preview awaits apply/discard. No unsaved
mesh edits are committed: every applied version already has a durable bundle.
`--mesh-plane-preview` opens a review-only proposal. Two new controller tests
plus extended startup/render coverage pass; all 199 workspace tests, Clippy,
formatting and release build pass. Actual private-X preview was inspected:
62 vertices/90 triangles, clear unapplied label, source bytes unchanged, no
result publication and no scanner fds. Full pointer/control qualification,
serialized mesh-history projects, repair and physical accuracy remain unfinished.

## Checksummed backend mesh-history projects

Added MSHPRJ01 current/undo/redo persistence with strict units/schema, length/CRC,
trailing-byte and full historical geometry validation. Streaming bounded two-pass
save preserves f64 geometry exactly; 32-step/128-MiB bounds remain explicit.
Trim accepts a project to continue its history; mesh-history CLI reopens exact
undo/redo into new original-preserving bundles. Four new regressions; all 203
workspace tests, Clippy, formatting and release build pass. Actual synthetic
release run: 90→54 triangle trims, reopened undo/redo mesh JSON byte-identical
to earlier versions and artifact CRCs verified. Details in MESH_PROJECTS.md.
GUI saved-version navigation is not yet hydrated/saved through this format;
complete scan/calibration/session/measurement project coverage remains unfinished.
No genuine scanner/physical qualification claimed.

## Full GUI mesh-history save/reopen

The Meshes pane now saves its current plus complete undo/redo snapshot history
into a new checksummed bundle and hydrates all states/previews from project
content. Reopened snapshots no longer depend on source files; further trimming
uses immutable validated mesh data. Original JSON loading retains source-change
checks. Stored history limits and stricter viewport-payload bounds are enforced
without silent load truncation. The new regression deletes original inputs and
trim folders after saving, then restores all meshes/current undo+redo exactly in
a fresh controller and continues trimming. Failed saves/corrupt opens retain state.
All 204 workspace tests, Clippy, formatting and release build pass. Actual release
private-X startup opens a synthetic project with 54 triangles and 2 undo steps;
render inspected, no scanner handles. This completes this scoped GUI history
connection, not broader raw/calibration/session/measurement project coverage or
physical/interaction qualification. See MESH_PROJECTS.md and gui-mesh-history evidence.

## Selective duplicate/unused mesh cleanup

Added explicit immutable duplicate-face and unused-vertex cleanup to backend,
CLI and GUI preview/apply. Duplicate identity uses vertex indices independent
of winding and retains the first face; positions are not welded or moved.
Options are independent and disabled by default in GUI. Shared edit publication
preserves originals, persisted history and exact undo/redo; topology/removal
counts are shown before applying. Four new regressions; all 208 workspace tests,
Clippy, formatting and release build pass. Actual synthetic release cleanup
94v/148t→93v/146t removed exactly two added faces/one vertex; cleaned mesh JSON
matches the original clean stereo surface byte-for-byte. Input bytes and undo
geometry retained, CRCs checked and native result render inspected with no
scanner handles. General nonmanifold repair, hole filling, simplification and
physical qualification remain unfinished. See MESH_CLEANUP.md and evidence.

## Explicit edge-connected mesh components

Added deterministic shared-index-edge component analysis and explicit whole-
component keep/remove, with stable first-face IDs, bounds and counts. Vertex-only
contact and coincident unshared geometry remain separate; no size heuristic
silently removes parts. CLI and background GUI analysis/preview share existing
original-preserving apply/history. Model changes invalidate stale analysis.
Three core plus CLI/controller regressions pass; all 213 workspace tests,
Clippy, formatting and release build pass. Actual synthetic release components
146/1 triangles: keeping the small ID produced one face, removing it restored
the clean stereo mesh JSON byte-for-byte. Original/undo retained, CRCs verified,
small-result native render inspected without scanner handles. The 100k component
cap and edge-index memory remain experimental, not large-mesh qualification.
Full face selection, broader repair and physical geometry remain unfinished.

## Genuine native projected-line capture

Added a scoped fixed property-151 boolean trial using stock SDK mapping and
actual supported range. Readback/restoration, exclusive lock, frame context and
full replay precede result publication; no mode/energy/calibration/firmware writes.
Actual trial 0→1→0 saved 60 frames per camera near 30 FPS (one startup reject
per stream, no accepted gaps/missing metadata), with all sampled vendor/V4L2
controls unchanged afterward. Both on images visibly show a multiline pattern;
native detection accepted 294/606 observations. Restored-off verification yielded
0/0 with the same detector settings and no visible pattern in the inspected image.
This is genuine native optical-control progress, not just USB/readback success.
Monochrome data does not verify blue wavelength. Six SIFT matches do not validate
factory geometry; line identity, mode/image binding, scale and calibrated 3D
remain unresolved. User calibration remains deferred. 214 tests, Clippy,
formatting and release build pass; raw data private, summary/protocol docs updated.


## Native laser factory-record field decoder

The native CLI now exposes CRC-verified raw laser record fields separately from
named-camera calibration. A sandboxed synthetic serializer probe verified all
383 copied marker words across three lens blocks, two transform blocks and the
1280-byte coefficient section. Both saved factory regions decode (eight records).
All three first-record lens intrinsic arrays differ from corresponding named
slots; named calibration cannot be silently assigned to the captured line mode.
No physical assignment, transform units, laser formula or accurate geometry is
claimed. Physical calibration remains deferred. Details and caveats:
`docs/FACTORY_CALIBRATION.md`; aggregate evidence:
`evidence/factory-laser-layout-summary.json` and
`evidence/factory-laser-record-audit.json`. All 216 workspace tests pass.


## Laser epipolar numerical convention

Six isolated synthetic reference cases establish row-major E=[t]cross*R and
F=inverse(K_second).transpose()*E*inverse(K_first) for the laser record slots.
The stored E/F tail fields agree with the native formula across all eight saved
factory records (max absolute residual <5.5e-6, internal consistency only).
Native calibrated Sampson-distance gating now rejects degenerate/nonfinite
constraints; raw factory diagnostics remain explicitly unbound to sensors and
units. The genuine line-on image pair still has only six tentative mutual SIFT
matches and provides no physical calibration confirmation. Plane/coefficient
formula, physical assignment and mode binding remain open. See
`docs/FACTORY_CALIBRATION.md` and `evidence/factory-laser-epipolar-summary.json`.


## Independent curved laser-sheet intersection

Located distinct old four-coefficient and newer cone-model paths in previously
captured private OpenCL compiler inputs. An isolated synthetic fragment run on
the local AMD GPU verifies the older numerical equation X=aY²+bY+cZ+d; no full
matching kernel or current mode execution is claimed. Independent Rust f64
intersection handles exact linear sheets, two-root ambiguity and numerical
failures without vendor runtime dependencies or added curvature. Seven
synthetic reference cases and analytical edge cases pass. The seven-float GPU
stride versus ten-float factory groups remains unbound; no physical factory
conversion or genuine depth is claimed. Details: `docs/LASER_SURFACES.md`;
reference: `evidence/laser-surface-reference.json`. All 222 workspace tests pass.


## Genuine stripe support for explicit factory-sheet hypotheses

Added a bounded native offline diagnostic that projects hypothesized sheet
intersections into the other camera, checks three-pixel detection support and
compares +200/+400/+600 horizontal-shift controls. Both camera assignments are
tested; no mapping is selected. Original full recordings reproduce the saved
294/606 stripe observations exactly at source frame 30 (extracted wrapper frame
0). Corrected counts yield only 6 distinct greedy pairs in the original order
and 10–13 in the swapped order, versus 0–2 in shifted controls. Support remains
sparse/clustered and does not establish physical calibration or accurate depth.
The analytical example test caught and corrected unit-ray versus Z-normalized
ray confusion before commit. Full details/caveats: `docs/LASER_SURFACES.md`;
aggregate `laser-sheet-hypothesis-*` evidence. Production calibration, device
state and geometry output are unchanged. 222 workspace + 3 example tests pass.


## Optional native local-background stripe detection

Added row morphological-opening subtraction with O(pixels) monotone queues,
reused bounded buffers, contrast-unit metadata and original-raw clipping checks.
Raw input/default detection remain unchanged. A fixed six-profile on/off replay
recovers 2764/3763 observations with local-strict settings, with 130/0 off-capture
detections; these are not all verified laser centers. The denser sheet-hypothesis
check has shifted controls equal to or stronger than unshifted support, so the
mapping remains unvalidated. Exact row indexing preserves every baseline result
while reducing distance comparisons; it does not relax geometric thresholds.
Details/evidence: `docs/LOCAL_LASER_DETECTION.md`, `laser-opening-*` reports/logs.
No hardware, calibration or production geometry activation was performed.
226 workspace tests plus 4 example tests pass.


## Native mesh boundary inspection and selected-contour overlay

Added deterministic directed boundary contours, perimeter/bounds and separate
ambiguous-graph diagnostics. Outer borders and intentional openings remain
unclassified and unchanged; no hole filling is claimed. CLI `mesh-boundaries`
accepts strict JSON or history projects; `scancraft-app --mesh-boundaries` opens
and analyzes either. GUI analysis is cancellable/background, lists are
virtualized, selected contours highlight through surfaces with a 10k-edge
preview cap, and cached IDs clear on geometry/history changes. Synthetic ring
CLI/project parity and private-display visual inspection pass. See
`docs/MESH_BOUNDARIES.md`, the ring fixture/report and render summary.
All 236 tests (232 workspace + 4 diagnostic example tests), warning-denying
Clippy, formatting and release build pass. Accurate genuine scanning, selective
hole filling and full mesh repair remain open. Calibration remains deferred.


## Explicit reversible planar boundary filling

Added native scaled-projection ear clipping for one explicitly chosen contour,
including concave contours, planarity bounds, overlap/intersection refusal,
consistent winding and unchanged original vertices. CLI `fill-mesh-boundary` and
GUI selected-contour preview share preparation and original-preserving atomic
publication. Exact undo/redo and checksummed history reopening pass. A synthetic
ring fills only its selected inner contour; its overlapping outer cap is refused.
Concave prism, tetrahedron, crossing geometry and numeric/cancellation cases are
tested. The release-rendered filled ring is visually verified on a private
X display. Limits and numerical caveats: `docs/MESH_FILLING.md`. All 242 tests
(238 workspace + 4 diagnostic example tests), Clippy, formatting and release
build pass. General mesh repair and physical scanning qualification remain open;
physical calibration remains deferred.


## Reversible face orientation and sampled normal display

Native parity propagation repairs shared-edge winding while preserving the
first face of each edge-connected component; explicit component IDs can reverse
that sign. Duplicate faces, nonmanifold edges and nonorientable constraints are refused.
Coordinates, connectivity, counts and openings stay fixed. CLI `orient-mesh` and
GUI preview/apply share exact history/persistence; normal arrows make sign changes
visible despite two-sided shading. Geometry/CLI/GUI tests include a Möbius strip,
source preservation, explicit reversal and undo/history reopening. Release ring
result/render inspected on a private display: one face changed, two conflicts
removed, openings preserved. Details: `docs/MESH_ORIENTATION.md`. All 247 tests
(243 workspace + 4 example tests), Clippy, formatting and release build pass.
Outward orientation, general repair and physical scan qualification remain open.


## Indexed vertex-link topology and pinched-surface diagnostics

Topology now counts connected incident-face fans per vertex and flags disconnected
fans, invalid boundary degree and nonmanifold-edge endpoints. Sorted bad vertex
IDs and a closed-manifold candidate flag supplement the unchanged legacy edge
check; neither is watertightness/embedding qualification. CLI `mesh-topology`
and GUI counts/red markers expose a synthetic pair of closed tetrahedra pinched
at vertex 0 despite zero bad edges. Filling now refuses bad source/result vertex
links. No automatic splitting or welding occurs. Analytical geometry, CLI and
GUI tests and a private-display render pass. A 131,072-triangle synthetic grid
checks correct fans/boundaries in about 60–66 ms; the separate whole-process
three-call audit peaks at 24,224 KiB RSS, not long-workload qualification.
Details: `docs/MESH_VERTEX_TOPOLOGY.md`. All 254 tests (250 workspace + 4 example
tests), Clippy, formatting and release build pass. Vertex repair, general mesh
repair and genuine calibrated scanning remain open; calibration stays deferred.


## Explicit disconnected vertex-fan repair

Added selected-vertex fan splitting through shared CLI/GUI preparation and exact
history publication. Each extra disconnected fan gets an identical-coordinate
vertex index; face positions, winding, boundaries and unselected defects stay
unchanged. Source nonmanifold edges/duplicates, invalid/regular selections,
vertex-budget overflow and cancellation are refused. Tests cover bit-exact
coordinates/negative zero, deterministic selection order, selected-only repair,
open bowties, the million-vertex limit and repair followed by explicit filling.
GUI/CLI preview/apply/source preservation/undo/reopen/no-overwrite checks pass.
Release pinched-shell result gains one vertex and loses the indexed defect,
without a physical gap. STL is byte-identical; native/JSON/OBJ retain indices.
Details: `docs/MESH_FAN_SPLITTING.md` and `mesh-fan-split-*` evidence. All 261 tests
(257 workspace + 4 example tests), Clippy, formatting and release build pass.
Geometric contact/intersection repair, nonmanifold-edge repair and genuine
calibrated scanning remain unfinished; physical calibration remains deferred.


## Native reflection conversion and SDK accessor clarification

Added checked pixel/camera horizontal reflection with distortion/skew and unit-ray
roundtrip tests. The stripe diagnostic now uses the shared native pixel helper;
its saved baseline report reproduces exactly. Local reference SDK source shows
Gemini2 mirror properties use host frame processors with inverted IR semantics;
raw numeric firmware queries are not equivalent to SDK mirror-state reads.
The native command now explicitly marks that equivalence unverified. There is
no new wire format established, so strict range decoding remains unchanged.
These reference paths are not yet proven for the customized Raptor SDK. See
MIRROR_STATUS.md and CAMERA_REFLECTION_HYPOTHESIS.md in docs. All 265 Rust tests,
Clippy, formatting and workspace/example release builds pass. No production
calibration, firmware, mirror state or physical accuracy claim is activated.


## Installed SDK mirror dispatch and complete stereo basis conversion

A hash-guarded read-only binary audit confirms the installed stock SDK base
FrameProcessor getter routes IDs 18/112 through a host FrameMirror block with
conditional boolean inversion. Its runtime flag and Raptor pipeline dispatch
remain unverified; raw firmware values still cannot be interpreted as SDK
mirror state. The native reflection helper now consistently converts both
camera models and stereo R/T, with synthetic projection/triangulation and
involution checks. This reflects scene handedness; no active calibration or
mesh winding is changed. Details: docs/MIRROR_STATUS.md and
CAMERA_REFLECTION_HYPOTHESIS.md. All 266 Rust tests, Clippy, formatting and
workspace release build pass. Calibrated real scanning remains unfinished.

## Installed inversion-flag initialization evidence

The read-only SDK audit now traces six direct calls that enable the host
mirror inversion flag, including PioneerDevice IR creation. RTTI, its primary
vtable slot and constructor initialization independently identify that class
path. The connected Raptor PID-to-constructor dispatch and actual frame
processing remain unverified; no factory binding is selected. Evidence:
sdk-mirror-inversion-flag-audit.json; details in docs/MIRROR_STATUS.md. The
updated Python audit passes against the hashed SDK and rejects an unrelated
synthetic binary before disassembly. No native Rust behavior or hardware state
changed in this investigation; the previous 266-test Rust checkpoint stands.

## Conditional Raptor PID dispatch located

The hash-guarded installed-SDK audit now finds PID 0x06a8 in USB and network
lists whose membership-match branches call the verified PioneerDevice
constructor. The list initializers, u16 comparison field, branch targets and
constructor call sites are checked; the exported PID getter confirms the field
offset. This resolves the static conditional link missing from the previous
checkpoint. Runtime gate/branch execution, earlier-list exclusivity, actual
frame processing and physical camera conventions remain unverified. Evidence:
sdk-scancraft-pid-dispatch-audit.json; details in docs/MIRROR_STATUS.md. No device
state or native reconstruction behavior changed; physical calibration remains
deferred and the overall mission is incomplete.


## Native pixel reflection and Pioneer configuration intent

Added explicit allocation-free GREY row reflection into reusable scratch space,
with exact source preservation and fail-before-mutation size validation. Three
new tests cover row/domain cases and consistency with raw/local-opening laser
centroids, run endpoints, signal and rejection behavior. Static SDK analysis
now checks both Pioneer IR branches create FrameMirror entries with flag 1 and
that the processor forwards this to the mirror enable setter. An additional
internal getter-inversion flag and later overrides remain unverified; no native
orientation default or calibration binding is changed. Details in
CAMERA_REFLECTION_HYPOTHESIS.md and MIRROR_STATUS.md under docs; binary audit:
sdk-pioneer-mirror-configuration-audit.json. All 269 Rust tests, Clippy,
formatting and release workspace build pass. Genuine calibrated reconstruction
and overall mission qualification remain incomplete.


## Complete-integrity image publication and mirror constructor state

Fixed export-image publishing the first PNG before discovering corruption in a
later frame/footer. It now validates/decode-checks the entire recording before
staging and publishing PNG with shared atomic no-overwrite semantics, then
syncs the destination parent. Three CLI regressions cover exact first pixels,
source preservation, later corruption/footer/truncation with no artifact, and
existing file/dangling-symlink preservation. Genuine saved-frame re-export is
pixel-identical and original 60-frame recording replay/export succeeds. See
docs/VERIFIED_IMAGE_EXPORT.md and verified-image-export-* evidence. All 272
Rust tests, Clippy, formatting and release workspace build pass.

The static SDK audit also locates the mirror creator calling a base constructor
that initializes its internal inversion flag to zero. This resolves its initial
value for that path; later writes and the Raptor's actual runtime state remain
unverified. No orientation/calibration default is activated. Physical calibration
remains deferred and real accurate reconstruction remains unfinished.


## Validate recording profiles at the header boundary

Readers for both recording versions now reject invalid profile semantics at
construction, including otherwise CRC-valid empty recordings. Writers reject
before emitting bytes. Checks require nonzero dimensions/FPS, existing supported
formats and <=16,777,216 declared pixels; this is a resource bound, not downstream
algorithm or physical-mode qualification. Independent header tests cover invalid
and exact-boundary cases, and twelve release CLI cases behave as expected.
Original saved 60-frame Raptor replay/decode still passes. Details:
docs/RECORDING_PROFILE_VALIDATION.md and recording-profile-* evidence. All 274
Rust tests, Clippy, formatting and release workspace build pass. Calibration,
genuine accurate geometry, live scanning and overall qualification remain open.


## Terminal recording errors

Recording writers now reject further pushes/finalization after output I/O
failure, rather than append records/footer at an uncertain boundary. Pre-output
validation/order rejections remain correctable. Readers reject all subsequent
reads after a stream error instead of consuming more bytes from an unknown
record position. Four integration tests inject recoverable I/O faults across
length/CRC/payload boundaries, verify no subsequent API stream calls, preserve
correctable write/order state, and keep CRC/footer/truncation errors terminal.
Original saved 60-frame replay/decode still passes. Details:
docs/RECORDING_STREAM_ERRORS.md and recording-fail-stop-* evidence. All 278 Rust
tests, Clippy, formatting and release workspace build pass. This improves
recording integrity handling; it does not qualify acquisition reliability or
resolve calibration, genuine reconstruction, live scanning or the full mission.


## Native point-to-plane registration engine

Added cancellable CPU point-to-plane ICP with supplied target normals, exact
indexed nearest correspondences/distance gate, centered/scaled bounded SVD,
six-axis rank/conditioning rejection, proper rotation updates and exact-residual
checks. Local step limits and at least 50% source overlap are required;
convergence needs stable residual and small matched-point displacement.
Exhaustion is reported honestly. Six synthetic tests include known three-plane
pose recovery, normal sign/scaling, single-plane rank rejection, invalid input/
limits/cancellation, iteration exhaustion and a distant coordinate origin.
Details: docs/POINT_TO_PLANE_ICP.md and point-plane-* evidence. All 284 Rust
tests, optimized new-algorithm tests, Clippy, formatting and release workspace
build pass. This is an engine API, not yet wired into GUI/tracking/fusion.
Normal estimation, robust correspondence filtering, genuine Raptor alignment
and physical accuracy remain unverified; calibration stays deferred.


## Point-to-plane sequence tracking/fusion integration

Offline sequence schema 1 now accepts explicit registration_method point_to_plane;
omission preserves point_to_point. The report records the chosen method. The
tracker retains target normals in each accepted local frame and requires valid
normals on selected samples for the new method. Post-fit acceptance/report RMS
remains independently recomputed Euclidean distance with existing coverage and
motion gates. No solver fallback or automatic normal estimation is introduced.
CLI tests exercise loss/recovery and world poses, fused normals/source retention,
missing-normal refusal without publication and the omitted-method default.
Tracker tests check invalid-normal retry/reset and successive rotated local
normal bases. All 288 Rust tests, optimized CLI sequence tests, Clippy, formatting
and release workspace build pass. Details: docs/POINT_TO_PLANE_ICP.md and
point-plane-sequence-* evidence. Genuine Raptor tracking/accuracy, GUI interaction
and live acquisition integration remain unverified; calibration stays deferred.


## Explicit surface-normal reconstruction output

Added angle-weighted normals on validated explicit mesh adjacency, with a
per-incident-face deviation gate, unsupported isolated/invalid-link vertices,
and optional declared viewpoint orientation. Geometry is not changed and sharp
conflicts do not become invented smooth normals. Optional surface_normals in
reconstruction requests requires meshing; source vertex/cloud correspondence
is checked and a separate surface-normal-points.ply exports supported points
with complete normals. The full cloud/project remain intact; mesh-report.json
records supported-point cloud indices, configuration and counts. Three geometry
tests include unequal-area/equal-angle weighting and two CLI tests verify exact
lineage/attributes and missing-meshing refusal. All 293 Rust tests, optimized
normal/reconstruction tests, Clippy, formatting and release workspace build
pass. Details: docs/SURFACE_NORMALS.md and surface-normals-* evidence. Unstructured
normal estimation, genuine Raptor normal/geometry accuracy, live tracking and
GUI interaction remain unqualified; physical calibration stays deferred.


## Cloud normal direction inspection

The native point-cloud workbench now displays optional sampled normal arrows
with pixel-length control and a full-cloud supported-normal count. Display
normalization does not alter source attributes. Rendering caps arrows at 2,000
from the bounded point preview, preserves source-index picking and projects
directions without adding small vectors to huge coordinates. The new
--cloud-normals startup path opens geometry without camera acquisition. Tests
cover normalization/zero vectors, source preservation, sampling/foreshortening,
extreme origins and picking with the overlay enabled. An authenticated private
Xvfb render was inspected: 231 synthetic plane normals and controls are visible;
no camera device descriptors were opened. Details: docs/CLOUD_NORMAL_VIEW.md,
cloud-normal-* evidence and cloud-normal-render/window.png. All 295 Rust tests,
Clippy, formatting and release builds pass. This is a startup/render check, not
full interactive/GPU performance or genuine scanner accuracy qualification.
Calibrated reconstruction and live scanning remain unfinished.


## Reversible selected point-normal correction

Added explicit Flip selected normals to the native cloud workbench and geometry
API. It reverses selected vectors without moving points, normalizing magnitudes
or modifying other attributes. Empty/duplicate/out-of-range selections and
missing/zero normals reject the whole proposal. A prepared worker preview
commits through Editor history and keeps selection; undo/redo and saved/reopened
history restore the original attributes. Tests cover bit-exact double reversal
(including signed zeros), source preservation, failure/cancellation and complete
worker/history roundtrip. The separate-display release screenshot confirms the
command is present/disabled with no selection and no camera descriptors opened.
Details: docs/NORMAL_CORRECTION.md and normal-flip-* evidence. All 298 Rust tests,
Clippy, formatting and release workspace build pass. This attribute correction
does not establish physical orientation, repair mesh winding or resolve the
remaining calibrated reconstruction/live scanning/qualification requirements.


## Native selected-point sphere measurement

Added centered/scaled algebraic sphere fitting with bounded SVD, rank/conditioning
rejection and independently evaluated geometric RMS/maximum residual. The native
workbench reports center/radius/diameter for selected points without modifying
geometry, selection, dirty state or history. The command requires four selected
points; planar/collinear/repeated/nonfinite or unrepresentable inputs fail.
Three analytical tests include tiny scales, distant origins and known noisy
radial residuals. A worker test checks read-only behavior and retained redo after
success/failure/cancellation. A private-display startup render verifies the new
control is visible and disabled without selection; no camera descriptors opened.
Details: docs/SPHERE_MEASUREMENT.md and sphere-fit-* evidence. All 302 Rust tests,
Clippy, formatting and release builds pass. This is not robust/nonlinear fitting,
saved measurement annotations, parametric CAD or physical sphere/scanner accuracy
qualification. Calibrated reconstruction, live scanning and full mission remain
unfinished; physical calibration stays deferred.


## Native 3D circle/diameter measurement

Added centered/scaled PCA-plane plus algebraic circle fitting with explicit
planarity RMS gate, conditioned bounded SVD and independently evaluated 3D
curve residuals. The read-only native workbench command reports center/diameter
and plane/geometric RMS; it requires at least three selected points and exposes
the planarity tolerance. Three analytical tests cover tilted geometry, tiny
scales/distant origins, nonplanarity and known residuals, invalid/degenerate data
and cancellation. A worker test verifies unchanged selection/document/redo
history. The private-display release render confirms control presence and
empty-selection disabling, with no camera descriptors opened. All 306 Rust
tests, Clippy, formatting and release build pass. Details:
docs/CIRCLE_MEASUREMENT.md and circle-fit-* evidence. Automatic hole recognition,
robust/nonlinear fitting, saved measurements, cylinder fitting and physical
accuracy qualification remain incomplete; calibrated scanning is unresolved.


## Persistent sphere/circle measurement snapshots

The native workbench now explicitly records selected measurement snapshots with
fitted values, source coordinates and circle settings. Recording marks the
project dirty but preserves geometry and selection; fit-only commands stay
read-only. Snapshot records remain historical through later geometry undo/edits.
The project format upgrades to ORPROJ03/version 3 when records exist, while
annotation-free projects retain version 2 and old projects remain readable.
Validation enforces typed finite metadata/results, source counts, unique labels,
16-record/100000-source-point bounds and the existing 64 MiB project cap. A
roundoff allowance corrects an initially overstrict RMS-versus-maximum check.
Three persistence tests plus a worker roundtrip cover compatibility, source
retention, malformed/versioned data, caps and failed/cancelled commits. All 310
Rust tests, Clippy, formatting and workspace/example release builds pass. A
native-generated analytical version-3 project reopens visibly with its label,
10 mm diameter and six source points in a private-display render; no camera
descriptors opened. Details: docs/MEASUREMENT_SNAPSHOTS.md and
measurement-snapshot-* evidence. Annotation renaming/deletion/undo, overlays,
physical measurement accuracy and calibrated live scanning remain unfinished.


## Reversible saved-measurement management

Added rename/removal and separate snapshot undo/redo to the native workbench.
Compact inverse changes retain deleted source evidence, preserve geometry and
selection, enforce a combined 16-change history bound, and clear redo on a new
branch. Rename forms close before other worker operations to avoid stale indices.
Version-4 projects preserve metadata history even with an empty current list;
versions 2/3 remain readable and migrate only when needed. Loading validates
history bounds and both reachable label/index chains. Three new persistence
tests and a worker test cover deletion/reopen/source restoration, rename/redo,
branching, invalid changes, cancellation, history cap, migration and CRC-valid
inconsistency. All 314 Rust tests, Clippy, formatting and release builds pass.
A native-generated version-4 project reopens visibly with management/undo controls
in a private-display render, with no camera descriptors opened. Details:
docs/MEASUREMENT_HISTORY.md and measurement-history-* evidence. Measurement
viewport overlays, physical accuracy, calibrated reconstruction/live scanning
and broader mission qualification remain unfinished.

## Historical measurement fit outlines

Added optional purple saved circle/sphere outlines and historical labels in the
point-cloud viewport, bounded to 3,072 segments. Snapshot source coordinates stay
independent of current cloud edits. Projection preserves separate local offsets
around distant origins. See docs/MEASUREMENT_OUTLINES.md.

Verified 317 Rust tests across all targets, strict Clippy, formatting and release
workspace build. Private-display startup screenshot inspected: the saved synthetic
10 mm sphere renders its ring, great-circle diameters and historical label; no
camera descriptors opened. Physical accuracy, calibrated live reconstruction and
full interactive/performance qualification remain incomplete.

## Direct point distance and angle measurements

The point-cloud workbench now measures two selected points in mm/inches and a
three-point angle with an explicit selected source vertex index. Selected source
indices are visible for small selections. Robust finite-angle math handles
independently scaled arms and overflowing endpoint differences. Read-only worker
operations preserve geometry, selections, serialized projects and redo history.
Results are transient status reports, not saved snapshots.

All 321 Rust tests, strict Clippy, formatting and the release workspace build pass.
See docs/DIRECT_MEASUREMENTS.md and evidence/direct-measurements-*. Physical
calibration remains deferred; live reconstruction and physical qualification
remain incomplete.

## Persistent direct measurement evidence

Distance and angle records now retain their own coordinates and explicit local
angle vertex, with rename/removal and persisted snapshot undo/redo. Direct values
are recomputed during validation to detect inconsistent stored results, including
CRC-valid corruptions. Cloud edits and geometry undo do not alter these records.
Optional historical outlines show distance segments and angle arms with labels.
See docs/DIRECT_MEASUREMENT_SNAPSHOTS.md.

325 Rust tests, strict Clippy, formatting and release workspace build pass. The
3-4-5 triangle fixture provides synthetic native render evidence. Calibration,
genuine dimensionally verified reconstruction and physical qualification remain
incomplete.

## Readable historical measurement labels

Historical labels are drawn after the model segments, wrapped to viewport width
and placed in non-overlapping rectangles inside the viewport. Backgrounds and
leader lines associate displaced labels with their source locations. A bounded
greedy search omits labels that cannot fit; full values remain in the panel.
Tests cover sixteen coincident labels, all four viewport corners, deterministic
placement, insufficient room and invalid anchors.

328 Rust tests, strict Clippy, formatting and release workspace build pass. The
previous triangle screenshot's overlapping labels are replaced by separately
readable historical distance/angle labels in evidence/measurement-labels-render/.
This is synthetic render evidence, not full interactive or physical qualification.

## Point-to-fitted-plane measurement

Added an explicit selected probe excluded from centered/scaled PCA of the other
selected points, with a user plane-RMS gate, rank/normal-ambiguity rejection,
distance/sign/foot reporting and historical project evidence. Optional viewport
outlines draw the retained probe-to-foot segment. Read-only and saved worker
paths preserve geometry and selection; snapshot undo restores saved evidence
after reopening. See docs/POINT_PLANE_MEASUREMENT.md.

333 Rust tests, strict Clippy, formatting and the release workspace build pass.
Analytical cases cover tilted planes, extreme scales, distant origins, RMS gates
and degenerate inputs. Synthetic native render evidence is separate from physical
qualification. Calibrated live reconstruction and physical accuracy remain open.

## Indexed mesh cross sections

The native mesh section API and `mesh-section` CLI extract shared-edge section
graphs, ordered open chains and closed contours with lengths and source lineage.
Branching components, coplanar faces, tangent contacts and duplicate segments are
reported explicitly. Separate coincident indices are not welded; positive
tolerance classifies vertices without moving them. Source geometry is immutable.
GUI integration, project section entities and area measurements remain open.
See docs/MESH_SECTIONS.md.

339 Rust tests, strict Clippy, formatting and the release workspace build pass.
The release CLI produces a closed 4 mm unit-cube section, rejects invalid planes
and leaves the input SHA-256 unchanged. Extreme-scale and distant-origin cases
are analytical evidence only; genuine calibrated reconstruction, performance and
physical qualification remain incomplete.

## Native mesh cross-section workbench

The mesh workbench now exposes cancellable plane section analysis, full contour
lengths/counters and a bounded purple through-surface section graph overlay.
Parameter changes, model changes, apply, undo and redo invalidate cached results.
Edit previews hide the saved mesh's section. Source geometry/history remain
unchanged. Startup `--mesh-section` opens and analyzes without cameras. Sections
remain transient; project section entities and GUI section export are unfinished.
See docs/GUI_MESH_SECTIONS.md.

341 Rust tests, strict Clippy, formatting and release workspace build pass. The
private-display cube render was visually checked: 8 segments, one closed contour,
4 mm length and a visible purple section graph; no camera descriptors opened.
Full interaction/performance and physical scanning qualification remain open.

## Reproducible section snapshot bundles

The mesh workbench and CLI can save one section's exact source mesh, original
plane settings and expected totals in a self-contained checked bundle. Reopening
verifies both files, recomputes the graph and checks totals before replacing the
view. New destinations are atomically published without overwriting originals.
Original input normals survive reopening/resaving without normalization drift.
Current mesh/history are unchanged by save. See docs/SECTION_SNAPSHOTS.md.

346 Rust tests, strict Clippy, formatting and release workspace build pass. A
release-generated unit-cube snapshot reopens in the native app with z=0.5, eight
segments, one closed contour and 4 mm length. Corruption/missing-source/CRC-valid
inconsistency/cancellation/no-overwrite paths are tested. Multi-section project
history, calibrated live reconstruction and physical qualification remain open.

## Manual coordinate alignment and origin repositioning

Added explicit selected-source origin/X/XY-side references to transform the whole
point cloud into a right-handed frame. Normals rotate without renormalization;
colors, order and selection persist. Preparation/preview are cancellable and
commit atomically. Source geometry and transformed geometry survive undo/redo and
project save/reopen. Historical measurement snapshots retain their old coordinates.
Named reusable transform records and mesh frame editing remain open. See
docs/COORDINATE_FRAMES.md.

352 Rust tests, strict Clippy, formatting and release workspace build pass.
Analytical cases cover oblique axes, extreme reference scales, distant origins,
normal attributes, invalid references, cancellation and overflow. The synthetic
aligned project provides native render evidence; physical accuracy and calibrated
live reconstruction remain incomplete.

The private-display render was inspected: frame controls open, four aligned
points with local bounds, available geometry Undo, retained 2 mm historical
measurement and no camera descriptors.

## Named reusable coordinate-frame documents

Frames can now be defined without editing the cloud, named, saved, loaded and
explicitly applied to another compatible cloud. Standalone RAPFRM01 documents
validate units/mapping, labels, proper frame axes and CRC within 16 KiB. Atomic
no-overwrite publication preserves existing data. Load/save/definition preserve
geometry/project state; application supports project undo after reopening.
Explicit finite matrix-invariant checks also close an overflow/NaN validation
gap in frame and rigid-registration pose validation. See docs/REUSABLE_FRAMES.md.

357 Rust tests, strict Clippy, formatting and release workspace build pass.
The synthetic saved frame provides native loading-screen evidence without any
application to a cloud. Embedded named-frame project records, calibrated live
reconstruction and physical qualification remain unfinished.

Native frame-document render inspected: saved name/origin/axes visible, zero
cloud points, Apply disabled and status confirms no cloud change; camera FDs [].

## Nonlinear geometric cylinder measurement

Added independent Rust cylinder fitting with three PCA initial axes, full
five-parameter nonlinear radial-distance refinement, explicit stationarity/rank
checks and caller RMS gates. Read-only worker/GUI measurement reports axis,
diameter, radial residuals, sampled axial range and conditioning without consuming
geometry history. See docs/CYLINDER_FITTING.md for bounds and limitations. Saved
cylinder records and model outlines are unfinished.

365 Rust tests, strict Clippy, formatting and release workspace build pass.
Analytical tests cover derivatives, nonlinear axis recovery, rotated/asymmetric
wall sampling, noisy residuals, extreme scales, degeneracy and cancellation. The
synthetic native fit fixture reports a 6 mm diameter; physical accuracy, global
recognition, independent reference comparison and calibrated live scans remain open.

Native cylinder render inspected with controls open, 96 selected synthetic wall
samples, 6 mm diameter, axial range -6..6 mm and conditioning diagnostics; camera
FDs []. Raw residuals remain in cylinder-example-report.json at full precision.

## Historical cylinder records and outlines

Cylinder measurements now persist selected source coordinates, fit parameters,
RMS gate and convergence/conditioning diagnostics using snapshot schema 4.
Rename/removal and persistent snapshot undo restore deleted evidence after
reopening. Geometry, selection and geometry history stay unchanged on recording.
Historical sample-span outlines use 132 segments and separate projected local
offsets, with labels and no point-picking changes. See docs/CYLINDER_SNAPSHOTS.md.

370 Rust tests, strict Clippy, formatting and release workspace build pass.
Tests cover geometry/metadata undo independence, reopen and malformed/CRC-valid
invalid records, cancellation, source retention and tilted model geometry.
Source validation is structural rather than an automatic nonlinear refit.
Calibrated live reconstruction, independent algorithm comparison and physical
qualification remain incomplete.

Native tilted-cylinder project render inspected: saved 6 mm diameter and original
RMS limit, 96 source points, snapshot controls and historical outline/label all
visible; no camera descriptors opened.

## Independent cylinder numerical comparison

Eight deterministic synthetic cases pass comparison to a separate SciPy optimizer
with numerical derivatives and a different axis parameterization. The native
fitter and production runtime remain Rust. Cases include partial noisy walls,
asymmetric sampling, elliptical form and taper. Recorded gates were not relaxed;
wrong-diameter and nonunit-axis sensitivity checks fail as intended. Input/native
hashes match the initial run. See docs/CYLINDER_REFERENCE_COMPARISON.md.

370 Rust tests, strict Clippy, formatting and release workspace/example builds
pass. Maximum diameter disagreement is 2.49e-9 mm on these synthetic inputs, not
a physical accuracy/uncertainty claim. Official NIST practice archive retrieval
returned HTTP 403 in the execution runtime; zero NIST cases ran. NIST coverage,
broader robustness, calibrated live reconstruction and physical qualification
remain unfinished.

## Native constrained mesh simplification

Added shortest-edge collapse with fixed source boundary/crease vertices, cumulative
source-cluster movement limits, original-face normal bounds, indexed link/duplicate
checks and final topology/boundary/component/Euler/source-face-image validation.
Targets may be unreachable; stop reasons and rejection/work diagnostics are
explicit. No proximity welding, hole filling or implicit unused-vertex cleanup.
GUI preview/apply/discard and CLI publication preserve original bytes, complete
source lineage and editable project undo/redo. See docs/MESH_SIMPLIFICATION.md.

382 Rust tests, strict Clippy, formatting and release workspace build pass. The
synthetic planar fixture reduces 98 -> 70 faces with 28 boundary edges unchanged
and 0.7071067812 mm maximum source-vertex movement under its explicit 1.5 mm
demonstration bound. Tests include closed/holed/coincident-component topology,
normal/displacement bounds, cancellation and exact saved history restoration.
Current limits are 100k vertices/200k faces; global intersection, independent
decimator, large-data, physical accuracy and calibrated live-scan qualification
remain open.

Final native preview/reopen screenshots were visually inspected on a private
display; the preview stays unapplied and the saved project exposes its undo step.
Startup parameter display preserves supplied values without silent range clamping.

## Native constrained mesh smoothing

Added uniform Laplacian Jacobi smoothing with fixed source boundaries/creases,
cumulative source displacement/normal limits and global step backtracking.
Triangle indices and unused vertices stay unchanged. CLI and GUI unapplied
preview use original-preserving publication and saved exact geometry undo/redo.
See docs/MESH_SMOOTHING.md for constraints and limitations.

390 Rust tests pass, including analytic one-step/translation checks, bounded bump
reduction, fixed boundaries, protected creases, cancellation, publication guards
and GUI save/reopen/history. Strict lint, formatting and release build pass.
Release synthetic example: 25 vertices / 32 faces; 5 iterations; 19 backtracking
steps; maximum movement 0.09888668434 mm under a 0.1 mm bound. Native preview/reopen
screenshots are inspected separately from worker tests. Volume, global
intersection, large-data and physical accuracy qualification remain unfinished.

## Native explicit-unit triangle mesh import

Added bounded ASCII/binary STL and position-only triangular OBJ import in Rust,
with explicit mm/in/m units and optional exact coordinate welding. Unsupported
attributes/polygons are rejected, source bytes retained, and topology reported.
CLI and native GUI publish a new bundle and open a checksummed editable mesh
project; further trim/save/undo is verified. See docs/MESH_IMPORT.md for limits.

399 Rust tests pass; strict lint, formatting and release build pass. Release CLI
native-export fixture imports preserve ordered face coordinates exactly, original
bytes and overwrite protection. Native startup import/render is inspected on a
private display without camera access. Broader OBJ/mesh attributes, independent
external-format comparisons, large-data and physical qualification remain open.

Import render review caught partial rendering with experimental wider path fields.
Reverting to standard text field sizing restored the full editor/viewport in the
layout recheck capture. Both failed captures are retained and labeled as failures.

## Dual-IR ten-minute structural acquisition trial

Completed one uninterrupted native dual-IR 30 FPS trial: each camera accepted
18,100 valid buffers across 606.4426 seconds; native exit 0. Duration PASS,
rejection proxy FAIL: 92 / 18,192 observed accepted+rejected buffers per camera
(0.505717%), with 92 accepted-sequence gaps, no missing metadata and no duplicate
PTS counter. First 32 retained rejects per camera are short/error-flagged buffers;
30/32 deficits are multiples of 61,380 bytes. Shared raw PTS supports a common
truncation investigation, not a verified cause or exposure synchronization.

Short warm reopen acquires valid buffers but rejects 111 left / 1 right buffers,
so reopening is not qualified. Evidence, module/binary hashes, exact runner source,
independent sample checks and diagnostics are in evidence/stereo-ir-soak-20261026/.
Ten new offline tool tests cover assessments, capped diagnostic samples and owned
child cleanup; 399 Rust tests remain at the prior source checkpoint. Full device,
physical geometry, 60-minute memory and end-to-end qualification remain open.
See docs/STEREO_ACQUISITION_QUALIFICATION.md.

## Rate counterbalance and independent rejection indicators

A new 15/30/30/15 equal-duration short comparison still has rejects at both rates;
large startup bursts occur on either camera. No lower-rate fix is claimed. The
first-sample cap obscured later diagnostics after startup bursts, so native Rust
capture now retains exact before/after-first-valid counts and a separate bounded
ongoing sample, with independent video/metadata flags, image length and PTS parse
indicators. All acceptance checks and the total rejection denominator stay intact.

A fresh 1,500-buffer 30 FPS run records five ongoing rejects per camera and zero
startup rejects. All retained metadata PTS structures parse, while video is short
and both V4L2 error flags are set. Retained first UVC flags lack the device ERR bit.
Local kernel source can propagate short-video error onto metadata; root cause is
still unverified. No kernel candidate was loaded/installed. See
docs/REJECTION_DIAGNOSTICS.md and the referenced public diagnostic evidence.

404 Rust tests, 12 offline tool tests, strict lint, formatting and release build
pass. Hardware reliability and physical/calibrated reconstruction remain open;
calibration stays deferred.

## Cooperative stereo process interruption

Native stereo probe/recording commands now convert SIGINT/SIGTERM to cooperative
cancellation with normal worker/resource unwinding. Prior handlers are restored;
control leases remain scoped through cleanup. New GET-only stream-state snapshots
verify reported formats/rates before, during and after capture. Separate single
camera CLI cancellation remains future work; no broad restoration claim is made.

Real hardware SIGTERM/SIGINT cases return explicit cancellation errors in
0.16450/0.16587 s and restore all reported state fields on both IR cameras, then
reopen with ten valid buffers each. No illumination/firmware/calibration writes.
406 Rust and 12 tool tests pass, with strict lint/format/release checks. An unrelated
parallel frame-document test temp-name collision was fixed with an atomic suffix;
its initial failure log is preserved. See docs/STEREO_CANCELLATION.md.
Noninteractive sudo is unavailable; the kernel candidate remains untouched.
The transport rejection and full mission qualification gates remain open.

## Single-camera interruption and partial recording publication

Extended scoped SIGINT/SIGTERM cancellation to native validate/capture. Interrupted
recordings report their retained staging path; cancellation checks precede final
publication. Twelve real cases across both IR cameras and color restore all
reported state fields, return explicit cancellation and keep six interrupted
recordings unpublished. Three completed recordings replay all frames with integrity
PASS; raw test images are removed from private temporary storage, not committed.
Two stereo signal cases also pass on the same current release binary.

406 Rust tests / 12 tooling tests, strict lint/format/release checks pass. This
qualifies the documented steady-acquisition interruption cases, not all failure
or illumination timing. See docs/SINGLE_CAMERA_CANCELLATION.md. Truncation and
calibrated geometry remain unresolved; calibration stays deferred.

## Native interrupted-recording prefix recovery

Added streaming validated-prefix recovery in Rust, original-preserving workflow/
CLI publication, and a native GUI background panel with progress/cancellation and
close handling. Only complete valid frames before the first damaged/incomplete
suffix are retained; no resynchronization or interpolation. Existing metadata is
preserved where supported, legacy header protection remains explicitly unknown,
and a full replay precedes publication. Keep the bundle's provenance report.

417 Rust tests / 12 tool tests pass, with strict lint, formatting and release
workspace/examples. Three genuine interrupted IR/color captures recover and decode
three frames each on the final binary, with byte-exact native-v2 prefixes and
unchanged sources. Raw test images remain private and are removed. See
docs/RECORDING_RECOVERY.md and referenced evidence. Transport truncation and
calibrated geometry remain unresolved; calibration stays deferred.

The GUI's unnecessary idle redraw timer was made conditional on active work.
A short private-display observation drops from 52 CPU ticks to zero measured ticks
at 100 ticks/s. Apparent partial screenshot interpretations were disproved by
identical PNG hashes/full pixel data; no rendering defect fix is claimed. Earlier
mesh-import partial-image/width-causation classifications are withdrawn by a saved
pixel audit; original inspection files remain for history.

## Indexed offline recording access

Added a bounded, cancellable full-validation index followed by direct frame
seeks. Every source frame/order/footer is checked before indexing succeeds;
selected records are checked again against original fingerprints. Four new
regressions cover corruption, budget/cancellation, mutation and exact seek read
sizes. Workspace tests and strict all-target lint pass. The offline diagnostic
and evidence are documented in docs/INDEXED_RECORDING_PLAYBACK.md. Native GUI
playback and filesystem mutation guards remain the next layer; this is not a
claim that the playback GUI or full scanner mission is complete.

Genuine validation: eight frames from each IR camera and color passed decoded
full replay and last/first/middle/last indexed reads. Source SHA256 and reported
camera settings were unchanged; private raw recordings were removed. Evidence:
evidence/recording-index-hardware.json. These short captures do not qualify live
throughput, transport reliability, synchronization or physical geometry.

## Native recording image playback

The Cameras panel now opens complete/recovered recordings through a background
worker, with indexed Previous/Next/frame-number navigation, cancellation and
source close. GREY, bounded JPEG RGB and explicitly labeled Y14 container display
are supported. Whole-file validation precedes the first image; read-only source
identity/timestamp checks surround indexing and every selected decode. Failure
or cancellation clears stale previews. Recovery provenance limitations remain
visible. Startup: `--play-recording RECORDING`.

Seven new workflow/UI/parser regressions pass. The same workflow/decoder checks
eight genuine frames per camera in nonsequential order with matching repeated
RGB checksums, unchanged sources and restored camera settings. Raw recordings
remain private and are removed. An isolated-display synthetic pattern render
has no camera descriptors. See docs/RECORDING_PLAYBACK.md for evidence and limits.
Manual image navigation is implemented; timed/synchronized reconstruction replay,
calibrated geometry and full end-to-end qualification remain unfinished.

## Selected recorded stereo frames → native reconstruction

Added direct recording/index inputs to the shared calibrated image pipeline,
CLI and native Point clouds panel. Whole-recording validation and read-only
source guards precede selected packed GREY inputs. The published bundle retains
exact selected raw frames, calibration/request and explicit source lineage;
context cross-checks do not invent missing device binding or synchronization.
Unsaved-document protection, cancellation and atomic no-overwrite publication
remain in the common GUI workflow.

Six workflow tests and a parser test pass, including exact synthetic report
identity with PNG reconstruction and raw/metadata/context retention, corruption,
source declarations, embedded calibration, legacy state and cancellation. Native
isolated-display startup reconstructs/opens the same three synthetic points with
no camera descriptors. CLI proof verifies equivalent reports and unchanged input
SHA256s. See docs/RECORDED_STEREO_RECONSTRUCTION.md. Actual scanner calibration,
genuine calibrated geometry, tracking and end-to-end qualification remain open;
physical calibration stays deferred.

## Persistent initial reconstruction provenance

Recorded reconstruction now embeds bounded initial source/calibration/request
metadata and five retained-artifact length/CRC references into both the live and
saved editable document. Origin-bearing projects use ORPROJ05/schema 5; ordinary
projects retain v2–4 behavior. Trimming, undo/redo, measurements, save and reopening
preserve initial provenance. The GUI shows descriptive source references without
automatically opening them. Missing/moved raw bundles do not prevent geometry
load; retain the bundle for reproducibility.

437 workspace Rust tests / 17 reference-tooling tests, strict lint, formatting and
release builds pass. Native isolated-display reconstruction and fresh-app reopen
plus an independent artifact/envelope audit are documented in docs/PROJECT_ORIGIN.md.
Evidence is synthetic. Full raw/session project packaging, multi-source merge
lineage, genuine calibrated geometry and master-mission qualification remain open.
Physical calibration stays deferred.

## Explicit retained-source bundle verification

Added read-only background verification of the five retained source artifacts
against v5 project length/CRC references. The native provenance panel accepts an
explicit directory (including a moved bundle), reports mismatches and supports
cancellation without changing document state or saved references. CLI and explicit
startup paths share the same verifier. No automatic source reads occur on ordinary
project load. Per-file and final metadata checks catch changes during verification.

444 workspace Rust tests / 17 tooling tests, strict lint, format and release
binaries/examples pass. CLI synthetic relocation/corrupt/restore proof and native
5/5 verification are recorded in evidence/source-verification-*; documentation:
docs/SOURCE_BUNDLE_VERIFICATION.md. This checks retained byte identity, not semantic
calibration quality, physical accuracy, synchronization or authenticity. Persistent
relocation/self-contained raw-session packaging and genuine calibrated geometry
remain unfinished; calibration stays deferred.

## Export edited projects with retained source artifacts

Added a native GUI/CLI export that copies the current editable project and its
five verified retained initial frame-input artifacts into a new folder. Full
geometry/measurement histories are retained; the copy's source location is updated
while the active document and originals stay unchanged. Copied sources and full
canonical project replay are verified before synced no-overwrite publication.
Cancellation/source changes/output races retain explicit staging without clobbering
existing data; a 64 MiB preflight precedes cloning/staging.

453 workspace Rust tests / 17 tooling tests, strict lint, format and release
builds pass. An edited synthetic fixture independently proves full project-state
equality except the exported reference, exact source copies, unchanged source
SHA256s and verification after moving the archive. Native export and fresh-app
reopen are captured without cameras. See docs/PROJECT_BUNDLES.md. This contains
the selected initial frame pair, not full original scan sessions, meshes or
multi-source lineage. Calibration and genuine scanner geometry remain deferred/
unqualified; the full mission is not complete.

The edited fixture exposed genuine control overflow at the default window size:
expanded provenance and measurement panels pushed the viewport/status offscreen.
Bounded controls/status scrolling now keeps the geometry and cancellation/progress
accessible. The saved failing baseline is retained, and a viewport-bounds regression
plus final native renders verify this specific layout case. Earlier partial-tool-
preview interpretations remain distinct and withdrawn as previously documented.

## Complete supplied recording archives

Native GUI/CLI now stream-copy full left/right recordings through strict header,
frame/order/CRC/footer validation, verify complete saved replay and publish a new
no-overwrite archive. Every supplied frame and original v1/v2 byte is retained;
legacy unknown header/context status is preserved. Cancellation, mutation and
publication races keep incomplete staging explicit. This is standalone raw-file
preservation, complementing selected-frame editable bundles.

461 workspace Rust tests / 17 tooling tests, strict lint, format and release builds
pass. Eight genuine saved frames per IR camera archive byte-identically and decode
successfully, with original SHA256s and camera settings unchanged; overwrite is
refused. Raw test images remain private and are removed. Native synthetic startup
archives all three frames per file with no camera descriptors. See
FULL_RECORDING_ARCHIVES.md under docs and full-recording-archive evidence.
Full-recording/project association, multi-session persistence, calibrated genuine
geometry and complete hardware/live/release qualification remain unfinished.
Physical calibration stays deferred.

## Capture setup metadata and stereo group restoration

Real GUI/CLI recordings now save bounded GET-only setup observations on the selected
video descriptor before video STREAMON: reported format/rate, allowlisted standard
controls, reported USB attributes and explicit errors/unknowns. Existing trial metadata
is retained. Descriptor serials are not promoted into bound identity, and no firmware,
projector mode, per-frame exposure or calibration verification is inferred.

A short comparison exposed a preexisting stereo reported-rate restoration defect
in both probe (no observation collection) and recording. A group guard now snapshots
both cameras before workers, restores all formats then all rates after workers stop,
and verifies reported fields. Controlled before/after evidence preserves the failing
baseline and passing fix; the underlying kernel/device coupling remains unproven.

468 workspace Rust tests / 19 tooling tests, strict lint, format and release builds
pass. Seven genuine metadata/decoded-replay entry cases (CLI single/stereo and native
GUI color/IR), 12 single and two stereo interruptions, and three recovered-prefix
checks pass with reported camera states restored. Raw media/context values remain
private; public replay evidence is redacted. See docs/CAPTURE_OBSERVATIONS.md and
capture-observations evidence. Calibration, physical accuracy, transport truncation,
live tracking and full master-mission qualification remain unfinished.

## Complete recording/project association checkpoint

Native GUI and CLI can export a saved edited project, its retained reconstruction
inputs and both complete supplied recordings into one atomically published folder.
Selected-frame lineage and canonical retained-frame fingerprints are checked against
the full recordings; originals and project histories are preserved. Relocation and
undo regressions pass. Historical identity of unselected frames is explicitly
unverified because earlier origins lack whole-recording fingerprints. Root-manifest
loading, scan resumption, recovery report embedding, meshes and multi-session lineage
remain unfinished. See docs/SCAN_PROJECT_EXPORT.md. This checkpoint passes 474 Rust
and 19 reference/tool tests, lint, formatting and release build, with synthetic
CLI/native-GUI worker/render evidence. No genuine geometry gate is advanced;
calibration stays deferred and transport reliability remains unresolved.


## Verified scan-folder reopening checkpoint

Exported scan folders now load through native background import and CLI verification,
including nested manifest/inventory checks, complete artifact fingerprints, full
recording replay and selected-frame association. Moved folders relocate the editor's
source location in memory without writing packaged inputs. Failed verification keeps
the current document/selection. Saved edit undo/redo survives a moved-folder load.
478 Rust tests, 19 reference/tool tests, lint, formatting and release build pass;
three inspected private-display captures show a moved synthetic project reopened,
with all 12 packaged files unchanged. See docs/SCAN_PROJECT_EXPORT.md and
scan-project-load evidence. Verification is point-in-time integrity, not historical
or cryptographic authenticity. Live resumption, recovery-report inclusion,
multi-session/mesh packaging, transport qualification and genuine calibrated
geometry remain unfinished. Calibration stays deferred.


## Persistent complete-recording reference checkpoint

ORPROJ06 stores both full-recording fingerprints/counts/versions/selected indices,
validated against initial lineage. New scan containers write these; verified v5
container loads add them in memory. Standalone edit/save/reopen retains references,
geometry/measurement history and undo/redo without opening external recordings.
Native UI and CLI provide explicit read-only complete-recording verification.
483 Rust tests and 19 reference/tool tests, lint, formatting and release build pass,
with synthetic native render while recordings are absent and legacy v5 container
verification. Evidence: recording-references-*; details in docs/PROJECT_ORIGIN.md.
Recovery-report embedding, multi-session/mesh provenance, resumed/live operation,
transport qualification and genuine calibrated geometry remain unfinished.
Calibration stays deferred; no full milestone/release qualification is claimed.


## Optional robust point-to-plane tracking checkpoint

Huber-weighted local registration now integrates into offline tracker/session/CLI
and native sequence fusion, with explicit millimeter scale and unchanged independent
Euclidean/coverage/motion gates. Raw plane RMS remains separate from robust objective.
489 Rust tests, 19 reference/tool tests, strict lint, formatting and release build
pass. Six independent SciPy fixed-correspondence comparisons agree below 9.6e-9 mm
in transformed points on synthetic data. Native synthetic fusion records loss and
recovery; CLI/GUI output artifacts match. Bias and an initial failed unsupported
test expectation are retained in evidence. See docs/POINT_TO_PLANE_ICP.md and
huber-plane-* evidence. No genuine geometry, global alignment, live scanning,
transport or physical accuracy gate is advanced. Calibration remains deferred.


Development checkpoint 2026-10-26: exact coincident-point k-neighbor pruning added
with brute-force/order and structural work-bound regression tests. All 499 Rust
tests and six NumPy statistical-filter comparisons pass. The 4000-coincident-point
CLI workload median improved from 534.388 ms to 39.297 ms (three runs each); this
is not general scanner or million-point qualification. See docs/benchmarks/COINCIDENT_NEIGHBORS.md
and developer-owned docs/reviews/REVIEW_RESPONSES.md. Independent shared findings
were absent and were not created by development. Hardware discovery remains empty.

SCAN-REV-004 development correction: capture panic/disconnection now reaches an
explicit stopped/error state after thread completion. Five hardware-free lifecycle
regressions pass, including retained incomplete recording and replacement-worker
availability; full workspace has 504 passing Rust tests. Await independent reviewer
verification. SCAN-REV-005 shutdown-bound and physical scanning qualifications
remain unresolved. No hardware, firmware or calibration writes performed.

Final correction checkpoint: 505 Rust tests pass with six hardware-free capture
regressions, including deterministic late-publication and repeated immediate-success
checks. CodeRabbit completed a committed review of 231695c against the workflow
branch with zero findings and a passed major gate. Draft PR #3 is available for
independent verification; no reviewer finding was closed by development and no merge
or protection change was performed. Genuine reconstruction and bounded driver
shutdown remain unqualified. See developer responses for exact commit scope.

SCAN-REV-005 ordinary-close mitigation: window close requests stop and keeps the UI
open while capture cleanup remains active. Completion explicitly rechecks unsaved/
busy project guards; errors keep diagnostics visible. Replacement capture cannot
start during pending close. Six additional hardware-free tests pass; full frozen
Rust gate now 511 tests. No measured whole-app shutdown bound or real driver-stall
qualification is claimed. Reviewer's finding is not closed by development.

## Optional fractional stereo checkpoint

Opt-in three-sample quadratic ZNCC peak refinement now propagates fractional
observations through original-camera triangulation. Integer matching remains the
default and existing confidence gates remain prerequisites. Independent NumPy
polynomial fitting agrees within 7.4e-15 across four synthetic signed shifts; native
CLI persistence and no-overwrite checks pass. A further regression verifies two
rotated-camera configurations, including fractional rectified x becoming original
y, against independently computed pixel rotations and analytic plane depth.

519 Rust tests, formatting, strict Clippy and release build pass. Earlier 518-test
implementation received completed CodeRabbit CLI review with zero findings; later
test/docs work is tracked separately in developer responses. Private-display GUI
startup/render passed; this is not live scanning qualification. See
docs/SUBPIXEL_STEREO.md and docs/reviews/evidence/2026-10-26-subpixel-validation.json.
Fresh discovery returned no scanner. Genuine calibrated Raptor reconstruction,
transport reliability, physical accuracy, live workflow and bounded forced teardown
remain unqualified. No calibration or firmware operation was performed.

## Read-only recorded timing checkpoint

`recording-timing` validates complete stereo recordings then reports nearest-host
value candidates, ties, reuse and raw PTS equality without automatically pairing
frames for geometry. 524 Rust tests, formatting, strict Clippy and release build
pass, including an exhaustive-distance oracle and native CLI source preservation.
Source: indexed.rs, recording_playback.rs, recording_timing.rs; method and memory
tradeoffs: docs/RECORDING_TIMING.md.

Read-only replay of the preserved genuine 100+100-frame native stereo recording,
at a caller-selected 1000us diagnostic limit, found six unique nearest candidates
in range, 94 outside, no ties/reuse, and three raw-PTS-equal in-range candidates.
Both original file SHA256 values were unchanged. Only aggregate evidence is
published in docs/reviews/evidence/2026-10-26-genuine-recording-timing.json; raw
images/timestamps remain private. This limit is not a qualified exposure tolerance.
These observations do not verify clock domains, exposure synchronization, optical
content, calibrated geometry or hardware accuracy. No new acquisition, physical
calibration or firmware operation was performed.

## Host/PTS comparison checkpoint

The read-only native timing report now independently compares unique raw PTS
values across complete recordings, withholding repeated values as ambiguous.
529 Rust tests and frozen format/Clippy/release pass. Genuine historical replay
finds72 equal-PTS pairs, only3 within1000us of host time; host differences range
8..20026us. Independent parser agrees and inputSHA256s are unchanged. This narrows
investigation but does not establish a cause or synchronized exposures. Details,
standard flag interpretation and qualification limits: docs/RECORDED_CLOCK_OBSERVATIONS.md.
No genuine calibrated geometry, new acquisition, physical calibration or firmware
action. Independent reviewer rejected the alleged earlier PTS compatibility defect;
raw CodeRabbit gate remains recorded as blocked, not silently relabeled passed.

## Offline recorded-stereo sequence checkpoint

A reusable recording processor now keeps validated indexes and rectification maps
across explicitly selected pairs. The new native CLI sequence workflow connects
reconstruction to the existing point-to-point tracking/fusion session, retains each
raw pair and diagnostics, and atomically publishes fused PLY/editable project.
Tracking rejection preserves diagnostics and is not fused; a subsequent valid pair
can recover against the retained reference. No-geometry failure retains current
raw inputs/rejection evidence in incomplete staging rather than inventing poses.

535 Rust tests, formatting, strict Clippy and release build pass. New regressions
cover cached report equivalence/source mutation, full retained inputs, project
reopening/overwrite/input preservation, cancellation/index errors, synthetic
wrong-depth rejection/recovery, no-geometry diagnostics and actual CLI execution.
Details: docs/RECORDED_STEREO_SEQUENCE.md. Per-frame provenance remains in the full
external bundle; fused standalone projects do not yet embed multi-frame lineage.
Point-to-plane selection, meshing in this path, blank-frame recovery/resumption,
dedicated GUI orchestration and genuine live/calibrated Raptor evidence remain
unfinished. No scanner, firmware or physical calibration action was performed.

## Native recorded-sequence GUI checkpoint

The Point clouds panel now submits recorded stereo sequences to the existing
bounded geometry worker, with stage progress/cancellation and the save/discard
guard before replacing unsaved work. Preparation stages the complete sequence;
Editor/Preview construction precedes source revalidation and atomic publication.
Only successful publication replaces the live document. CLI run composes the
same phases, and startup arguments support repeatable offline operation.

538 Rust tests, formatting, strict Clippy and release build pass. Regressions
cover late cancellation/source mutation between phases, GUI preservation on
errors, unsaved guards and saved/live geometry equality. Actual private-display
GUI operation produced all29files byte-identical to CLI output, integrated three
synthetic frames, preserved inputSHA256s and opened no camera descriptors.
See docs/RECORDED_STEREO_SEQUENCE.md and sequence-gui-equivalence evidence.
This is not manual dialog/hardware qualification. Multi-frame project provenance
remains external, live integration and physical accuracy remain unfinished.

## Persistent sequence-project references checkpoint

ORPROJ07 preserves initial sequence selections, integration decisions, declared
source/calibration ID and retained-artifact fingerprints across edit history and
standalone save/reopen. Both saved and live GUI editors retain the same metadata.
Older02..06 projects keep their old schema when sequence metadata is absent.
Project loading never opens external sources. Explicit source verification now
supports sequence folders and guards nested parents against symlinks/changes.

541 Rust tests, formatting, strict Clippy and release pass. Runtime native CLI/GUI
proof creates v7, filters/saves/reopens without losing refs and explicitly verifies
27retained artifacts after moving the bundle; inputSHA256s unchanged. GUI/CLI27
non-location-dependent files are identical, projects agree after descriptive-path
normalization and root-manifest differences are only project size/CRC. Independent
reviewer passed30targeted tests on a stable snapshot, not the full541suite.
See docs/SEQUENCE_PROJECT_REFERENCES.md and sequence-references-runtime evidence.
Raw sources/full reports stay external; fingerprints do not qualify original
recording completeness, semantic replay, binding, synchronization or physical
accuracy. No hardware/calibration/firmware change or new dependency.

## Native live stereo pipeline implementation

Added finite scan-live-stereo CLI acquisition -> raw recordings -> bounded
association -> existing reconstruction/tracking/fusion -> project/PLY bundle.
Synthetic injected sources exercise complete publication and cleanup; physical
entry rejects synthetic calibration. Configured source binding, equal rawPTS and
host-window association do not establish calibration/exposure accuracy. Actual
Raptor operation remains unverified. Live GUI integration is now implemented
and synthetically validated as described in the following milestone;
no physical calibration, firmware or illumination operation was performed.
See docs/LIVE_STEREO_SCAN.md and developer review responses for evidence scope.

## Experimental live GUI milestone

The live GUI now shares native device/session ownership through the local
`scancraft-acquisition` crate. It offers reconstruction pause/resume, graceful
stop/save, abort, bounded camera/cloud previews and guarded editor handover.
See [behavior and validation](docs/LIVE_STEREO_GUI.md). The frozen Rust gate
passed 571 tests; actual release-GUI interaction and busy-window close passed
with synthetic sources only. Real calibrated reconstruction, source binding,
synchronization, accuracy and bounded kernel shutdown remain unqualified.
