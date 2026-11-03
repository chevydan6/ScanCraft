# First measurable Raptor scan: calibration procedure

Status: **physical calibration authorized on 2026-11-03**. The owner clarified
“i mean remove all explicit restrictions”, superseding the previous deferral.
Authorization is recorded in the developer mission and recovery checkpoint.
No new calibration fit or independently validated physical scale is established
by that authorization. This plan uses local observations and numerical fitting;
firmware modification is not needed to execute it.

## Existing evidence and missing evidence

The retained board QR was decoded consistently in eight color frames. It contains
38 coordinate pairs plus an identifier. Keep its exact coordinates and raw bytes
private; do not replace them with a rounded regular grid. Board identity, coordinate
units and physical scale still require independent confirmation. Existing fixed
factory-model experiments produced plausible board correspondences across several
views, but this is neither calibrated application state nor accuracy qualification.

The earlier 30 mm and 50 mm caliper images do not establish a measured opening:
the two camera views did not reliably identify the same jaw surfaces, and existing
reprojection checks rejected candidate correspondences. Do not relax those checks,
fit scale to force the displayed distance, or count the display itself as a 3D
measurement. Occluded board circles also produced biased centroids with low
reprojection residuals; preserve that failure evidence.

## Calibration procedure

1. Verify the physical board identity against its retained QR and obtain an
   independent measurement of two widely separated board distances, preferably
   along different axes. Record the instrument, zero check, stated accuracy and
   measurement uncertainty. Treat QR units as unknown until corroborated.
2. Use the existing native scanner acquisition and restoration paths. Preserve
   exact firmware/device/mode/image orientation, exposure readbacks, camera
   interfaces, timestamps, raw frames, rejection counters and capture checksums.
   Do not change exposure or projector modes silently. Keep the target stationary
   during each capture; physical repositioning is performed by the owner.
3. Capture at least 12 usable static board poses spanning image center/edges,
   multiple depths and tilts. Keep all four orientation marks visible and reject
   cropped, saturated, motion-blurred or partially occluded marks using recorded
   image-quality criteria. Preselect at least three poses as holdouts before
   fitting. Preserve rejected observations and reasons. A single front-facing
   view is insufficient to qualify lens distortion and stereo pose.
4. Associate exact board points using the existing bounded Rust target routines.
   Validate image correspondences independently of factory epipolar geometry.
   Retain raw pixel/target indices and image orientation; do not let a model choose
   its own supporting correspondences without an independent check.
5. Fit local camera intrinsics/distortion using the existing planar initializer
   and Brown refinement; estimate stereo extrinsics with the existing fixed-
   intrinsic fitter. Compare residual distributions and coverage with the fixed
   factory hypothesis and an isolated numerical reference. Brown5 is an explicit
   candidate model, not an assumed complete Raptor lens model. If it cannot explain
   held-out observations, stop and report the model mismatch.
6. Persist the candidate calibration only in a new private local record, with
   raw observation hashes, target identity, units, image dimensions/orientation,
   model convention, device/mode binding and fit/holdout evidence. No installation
   into the scanner or factory-data modification. Mark accuracy unqualified until
   the independent object trials pass; matching metadata is not physical proof.
7. Reconstruct a separate stationary, matte object with visible geometry and at
   least two independently measured dimensions. Record measurement endpoints,
   instrument uncertainty and acquisition conditions. Prefer an easily imaged
   block/step or other feature whose corresponding surfaces can be identified in
   both cameras. Do not use calibration-board fit residuals as independent object
   accuracy evidence.
8. Repeat acquisition and reconstruction three times using frozen calibration and
   processing settings. Preserve original captures, calibration lineage, rejected
   data and missing surfaces. Report dimensional errors, plane residuals where
   applicable, density, noise and repeatability, including failed trials. The
   existing mission thresholds remain provisional and are not silently weakened.
9. Inspect the actual result in the native GUI, edit reversibly, save/reopen and
   export PLY. Verify geometry and provenance round trips. Generate STL/OBJ only
   if measured surface coverage supports a meaningful mesh; do not fabricate
   missing surfaces to create a recognizable model. Tracking/fusion must preserve
   rejected alignments and cannot turn unverified units into millimeters.

## Authorization and physical readiness are separate

The authorization gate is lifted. The owner returned and repositioned the board;
the complete pattern is now visible in both scanning cameras. Two new stereo
captures passed native replay with reported stream/illumination states restored.
Native fitting and held-out diagnostics are documented in
[real-board calibration](REAL_BOARD_CALIBRATION.md). Diverse poses, independent
dimensions and measurement uncertainty remain required.

Continue existing-recording analysis and integration work while physical setup is
pending. Do not claim a first independently validated scan until the physical
measurements and all three trials exist. Raw capture evidence remains in the
private developer artifact directory `authorized-calibration-2t4qng11`.

Code references: `scancraft-calibration::{target, planar, refinement, stereo_fit}`
(see the individual calibration documents for exact module interfaces),
`scancraft-workflow::recorded_sequence`, `scancraft-geometry::{tracking, session}`.
Related evidence: [target association](TARGET_ASSOCIATION.md),
[camera model](CALIBRATION.md), [rotating-board diagnostics](ROTATING_TARGET_EVIDENCE.md),
and [recorded sequences](RECORDED_STEREO_SEQUENCE.md).
