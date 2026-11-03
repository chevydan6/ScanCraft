# Recorded board calibration sessions

`scancraft-cli prepare-board-session SESSION_JSON NEW_OUTPUT_DIRECTORY` assembles
multiple [recorded board observation requests](RECORDED_TARGET_OBSERVATIONS.md),
retains their raw frames and separates training from validation observations.
When explicitly requested and target units are declared millimeters, it runs the
existing independent left/right planar Brown5 fitters. It does not activate a
capture calibration or choose a stereo lens domain.

Example request structure (numerical settings are explicit, not automatic):

```json
{
  "schema_version": 1,
  "training": ["pose-0.json", "pose-1.json", "pose-2.json"],
  "validation": ["pose-3.json"],
  "fit_intrinsics": false,
  "initial_max_reprojection_error_px": 20.0,
  "refinement": {
    "max_iterations": 80,
    "max_reprojection_error_px": 2.0,
    "estimate_skew": false
  }
}
```

Paths are relative to the session request. Each names a single-pair request, not
an already published result folder. Its own recording paths remain relative to
that single-pair request. Settings and raw source selection are retained exactly.
The session request accepts at most 1 MiB, with one possible overflow-detection
byte. Unknown fields are rejected. Limits are 3–32 training and 0–32 validation
poses. Three poses are a numerical minimum, not sufficient calibration coverage;
follow the broader [physical calibration plan](FIRST_SCAN_CALIBRATION_PLAN.md).

## Consistency and provenance

Every pose uses complete-recording integrity/selection and publication checks from
the existing workflow. The session requires identical target layout, exact target
coordinates, unit declaration and per-camera stream profiles (including frame
rate). This does not prove physical camera identity, unchanged optics or settings.
Detector thresholds may differ; each exact request is retained.

Canonical request-path duplicates are rejected. Per-camera fingerprints also
reject reuse of the same recorded frame across either split, including request
aliases. Fingerprints include profile, sequence, timestamps and payload/metadata
CRCs. They detect this reuse, not cryptographic identity, optical pose diversity,
independent exposures or malicious input forgery. Separate frames may still show
an unchanged board pose; numerical conditioning and physical coverage need their
own validation. Source validation occurs per pose; raw snapshots preserve what
was actually processed rather than claiming a simultaneous whole-session source
snapshot.

## Published artifacts

- `session-request.json`: exact supplied session bytes.
- `training-00/`, etc., and `validation-00/`, etc.: complete existing single-pair
  bundles with exact requests, measurements, original frame snapshots and digests.
- `dataset.json`: per-camera and stereo observations separated by split, pose
  labels, source lineage, profiles and explicit unit/qualification flags.
- `left-intrinsics-request.json` and `right-intrinsics-request.json`: emitted only
  for declared millimeters with at least four retained marks in every training
  camera view. Validation observations never enter these requests.
- Optional `left-fit/` and `right-fit/`: existing planar fitting result bundles
  when `fit_intrinsics` is true and the original fitters accept the inputs.
- `manifest.json`: top-level digests, nested bundle-manifest digests, counts and
  actual fitting/export status. Nested manifests bind their own artifacts.

With unverified units and `fit_intrinsics: false`, collection succeeds without
millimeter-typed requests. Requesting fitting with unknown units fails and retains
the incomplete session, including observations. No unit conversion is guessed.
A millimeter declaration still leaves `units_verified` and physical qualification
false. Fit failures retain the partial session and any already completed camera
fit; they do not publish a successful final session or weaken solver thresholds.

No existing destination is overwritten. Publication uses staging, sync and the
shared no-replace rename. Cancellation remains cooperative; I/O and initializer
linear algebra have no hard interruption deadline. Sequential per-pose processing
releases raw frames after each pose and retains observation arrays and bounded
indexes while needed. The pose limit bounds count, not a measured peak-RSS claim
or a reservation of disk space.

## Verification

Source `10049dd46548a73cc27053c615653d5cea01b600`, 2026-11-03:

- Full Rust gate: **845 passed**, one existing ignored test; formatting, strict
  all-target Clippy and optimized workspace builds pass.
- Three new synthetic-image workflow tests cover six training poses plus one
  held-out pose, both camera fits, training-only fit requests, exact retained
  validation data, unit boundaries, copied-frame rejection, target mismatch,
  cancellation, existing output preservation and manifest digests. These are
  synthetic numerical/integration tests, not hardware accuracy evidence.
- A real saved-data session reproduces all artifact bytes of the three previous
  training poses and separate held-out pose exactly. Per-camera counts are 37/38,
  31/30, 30/28 and 38/36; stereo intersections are 37, 29, 27 and 36. All recursive
  artifact CRCs/lengths pass and original recording SHA-256 hashes stay unchanged.
- Those actual observations retain unverified units and are not refitted as
  millimeters. A separate real-input refusal check confirms requested fitting
  fails, leaves no final session, and preserves the partial observation dataset.

Evidence and reproduction script remain private under developer artifacts
`board-session-jiuj9ub1`. Initial focused tests hit the existing `/tmp` quota;
rerunning with the established home-directory TMPDIR passed. No solver failure
or hardware diagnosis is inferred from that filesystem failure. CodeRabbit quota
lookup remains unavailable; no paid review or independent approval is claimed.

The native app currently runs single-pair preparation. Session UI integration,
validation-pose scoring, physically qualified stereo geometry, independent scale
and a first measured object reconstruction remain outstanding. This session
workflow supplies traceable inputs; it is not a qualified calibration wizard.

## Separate fixed-camera validation

The native `validate-planar-pose` command can score explicit held-out observations
against a supplied fixed camera, preserving arbitrary target units. See
[contract, real evidence and domain limitations](FIXED_CAMERA_POSE_VALIDATION.md).
Session assembly does not automatically run this command or attest independent
lens provenance; validation views remain separate from fitting.
