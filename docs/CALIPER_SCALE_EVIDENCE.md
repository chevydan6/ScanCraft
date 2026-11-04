# Caliper observations and physical-scale readiness

## Result

**Confirmed:** the saved 50 mm caliper observations do not establish physical
scale or dimensional accuracy. They do pass the existing reprojection limits.
Earlier calibration-plan wording grouped these observations with rejected
correspondences and was too broad. Acceptance by a geometric consistency check
cannot identify which reflective jaw surfaces were actually selected.

The corrected native candidate reconstructs the two frozen refined observations
at a separation of **49.492247 raw model units**. This is not a measured millimeter
error. Neither equality of a raw unit to a millimeter nor correspondence to the
caliper's contact surfaces has been independently established. The display/owner
setting was 50 mm; it was not used to select endpoints, fit scale or accept results.

## Inputs, method and provenance

Private evidence `caliper-native-evaluation-20261104` contains the frozen plan,
source bindings, exact historical seeds/refinements, corrected native rig,
probe source, all results and interpretation. The source capture is the retained
`hardware-20261102/caliper-50mm-b5bMho` recording. Frame 5 in each camera has equal
sensor PTS and a 22-microsecond host timestamp separation; simultaneous exposure
is not proven. Current native extraction validates all ten frames in each recording
and reproduces the prior selected manifest and image bytes exactly. Both complete
recording hashes match the retained calibration-training source bindings.

The private Rust probe calls
`scancraft_calibration::stereo::StereoCalibration::prepare` and
`PreparedStereo::triangulate` using the existing native Brown/stereo model.
It introduces no new reconstruction algorithm. The public API's mm-named fields
contain explicitly unverified raw model units only in this private diagnostic;
no active calibration, metric project or export is created. Camera ordering is
native for the native rig; the unchanged historical factory case is a reference
reproduction, not a replacement native calibration.

Limits are frozen: depth 100–1000 raw units, minimum ray angle 0.0001 radians,
maximum ray gap 1 raw unit and maximum reprojection error 2 pixels. The two old
endpoint hypotheses are retained exactly. There is no pixel reselection, new
calibration fitting, scale adjustment or selection of a result closest to 50.
The helper uses the existing calibration library; its dependencies are a subset
of versions already in the workspace lockfile.

## Numerical evidence

| Frozen observation/model combination | Reconstructed separation, raw units |
| --- | ---: |
| Original refined points, factory reference | 49.477877532 |
| Original seed points, native candidate | 49.582751617 |
| Original refined points, native candidate | 49.492247260 |

The factory reference reproduces the retained numerical output exactly, with
maximum absolute numeric difference 0. Native refined reprojection residuals are
0.4012/0.3765 pixels for the first point and 0.4681/0.4596 for the second. These are
image consistency results, not dimensional uncertainty or known point identity.

The frozen sensitivity experiment perturbs one of the eight image coordinates at
a time, independently in both directions. It retains every rejected case:

| Perturbation | Accepted / total | Accepted separation range, raw units |
| --- | ---: | ---: |
| ±1 pixel | 16 / 16 | 49.251389–49.738640 |
| ±5 pixels | 8 / 16 | 48.343855–50.778950 |

These deterministic perturbations are not measured observation noise or confidence
intervals. The eight rejected 5-pixel cases remain failures. No perturbation is
chosen to improve agreement with the display.

**Engineering inference:** reflective facets and silhouette edges can produce
plausible geometric intersections while representing different physical points.
A distance between guessed tips also need not equal the normal separation between
the caliper's contact planes. Neither low reprojection error nor closeness to 50
resolves these ambiguities. Instrument accuracy/zero uncertainty is also unknown.

Board markers from this same caliper scene contributed to the three-pose native
calibration fit. Jaw endpoints were not fitted, but this is not a completely
independent capture/pose holdout. Existing limited lens coverage and unverified
physical units remain documented in [REAL_BOARD_CALIBRATION.md](REAL_BOARD_CALIBRATION.md).

## Current physical scene and acquisition

A fresh inspection is retained in private `reconstruction-scene-idle-media-20261104`.
It uses the original baseline driver, not the temporary sustained-trial candidate.
An initial attempt refused WirePlumber-held media devices before camera capture;
that refusal is retained in `reconstruction-scene-20261104`. The subsequent attempt
verified idle PipeWire nodes, paused WirePlumber, and restored its original active
state in cleanup. No module, firmware or permanent service change was made.

Ten frames per scanning camera pass native replay. Startup rejected buffers are
**98 left / 1 right**, with zero ongoing rejects or sequence gaps in this very short
sample. Nine equal-PTS associations are available; left frame 5 matches right frame 6
with 22-microsecond host separation. Initial frame 5/5 images are scene previews,
not a claimed synchronized reconstruction pair. These observations do not extend
the ten-minute zero-rejection result or qualify normal-driver reliability.

Three color frames were captured. The orchestration process exits 1 because
`extract-frame` supports packed GREY only and refused the MJPG color recording.
The existing `export-image` path subsequently validates and decodes all three color
frames and exports the first image successfully. This is an invocation correction,
not a new decoder implementation. The original failure remains recorded.

All seven native state comparisons agree after cleanup; saved camera formats
restore and WirePlumber is active again. Both scanning-camera images show the
calibration board, cropped at the right edge in the right camera. The color image
also shows a yellow rectangular object beside the turntable, absent from the two
scanning-camera images. No measured object reconstruction results from this capture.

## Next physical evidence

Use independently measured, identifiable surfaces on the printed test object.
Measure the cooled print with recorded instrument/zero information; nominal CAD
size is not ground truth. Record endpoints or contact faces and retain dimensions
that were not used for calibration/scale fitting as validation measurements.
The object must be fully visible to both scanning cameras. The board still needs
independent scale confirmation and sufficient diverse, uncropped calibration views.

The owner has been asked whether the printed object is ready and to provide its
measured dimensions. While that physical input is pending, preserve the existing
calibration and capture evidence. Do not force agreement with 50 mm or treat sparse
board markers as an independently accurate reconstructed object.
