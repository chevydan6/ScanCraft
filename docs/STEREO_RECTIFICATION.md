# Calibrated rectification and sparse image reconstruction

The native Rust components now connect an explicit source stereo calibration to
paired image preparation, sparse matching, source-coordinate recovery and checked
triangulation. This pipeline is used by the CLI and offline GUI. It is not a
validated live Raptor scanning mode. Physical calibration remains deferred. No firmware,
projector or calibration data is written to the scanner by these components.

## Basis and output policy

For source coordinates `X_right = R * X_left + t`, compute the right camera
center `C = -transpose(R) * t` in original left-camera axes. New X follows
`C / |C|`. Project the mean forward optical directions onto the plane normal
to X to choose forward Z; construct orthonormal Y and Z using cross products.
The basis is right-handed. Both output cameras share those axes and the same
explicit ideal pinhole grid. Their relative pose is exactly identity rotation
with translation `[-|C|, 0, 0]` millimeters.

The output grid is caller supplied: width, height, focal lengths, principal
point and supported normalized output radius. Skew and distortion must be zero.
There is no automatic crop, focal-length estimate or source calibration resize.
The output radius limits requested virtual rays; it does not expand the source
lens coverage. Each image map independently preserves the original calibrated
coverage and sample-center bounds through its validity mask.

`left_from_rectified` maps rectified-left rays into original left-camera axes;
`right_from_rectified` maps rectified-right rays into original right-camera
axes. These are ray rotations about the respective camera centers. They do not
translate a point from the left camera origin to the right camera origin.
`point_in_source_left` transforms a rectified-left 3D point into original left
axes. A vertical or reversed baseline can rotate the virtual image substantially;
this policy always places the right camera center on positive virtual X.

Degenerate/forward-looking baselines, unstable common optical directions,
nonfinite or invalid poses, and invalid output grids fail explicitly. Valid
matrices alone cannot establish common visible scene content: masks may still
contain no useful overlap. `StereoRectification` has private validated fields
and serializes diagnostics; it cannot deserialize unchecked inconsistent bases.

OpenCV's official [stereo correspondence documentation](https://docs.opencv.org/5.0/main_modules/stereo.html)
describes calibrated rectification rotations and projection matrices. Our
baseline/mean-optical-axis basis and explicit output grid need not equal the
particular basis/crop selected by `cv2.stereoRectify`. Reference comparisons
use our explicit rotations with OpenCV's independently implemented sparse
undistortion/projection and triangulation operations.

## Processing and provenance

`StereoImageMaps` owns both reusable maps and their exact source/ideal geometry.
`StereoProcessor::build` additionally retains the complete validated original
calibration record. `process(StereoRequest, WorkControl)` performs:

1. Check declared source/provenance binding, triangulation limits and cancellation.
2. Apply source-calibrated maps to the borrowed packed GREY images.
3. Match supplied seeds in rectified coordinates with texture, uniqueness and
   reverse-match checks; retain rejected seed diagnostics.
4. Map each accepted pair of ideal pixels back to original distorted observations.
5. Triangulate with the original calibration, preserving original left-camera
   axes, original Z-depth bounds and original-image reprojection tolerances.

A synthetic calibration cannot be declared physical observations. Actual
observation bindings must match the record's declared serial, firmware, mode
and image sizes. These are consistency checks, not attestations of physical
origin. Input `GrayImage` can be created from validated recorded frames through
`from_frame`; direct views also support offline images. Pair/exposure timing
must be supplied and verified upstream. The processor cannot infer it.

A schema-version-1 serializable report retains original calibration/provenance,
rectification, matching configuration/scores, mapping failures and triangulation
results. `synchronization_verified` and `physical_accuracy_qualified` remain
false because the processing API cannot establish either. It does not contain
raw images or replace recording/project persistence. No reader or report
roundtrip qualification is implied.

Index lineage is explicit: matching accepted entries carry original seed
indices; mapped entries carry both seed and rectified-match indices; a
triangulation result's `source_index` indexes the mapped-entry vector. This
preserves associations after individual failures. A fully rejected image pair
returns a diagnostic report with `reconstruction: None`, never fabricated
vertices. Cancellation or invalid input returns no partial report. Processing
retains all existing bounded image/matcher limits. Output buffers are currently
allocated per pair and full-pipeline live performance is unqualified.

## Evidence and reproduction

Four rectification tests check identity, reversed/vertical/oblique baselines,
asymmetric rotated/distorted/skewed cameras, epipolar alignment, inverse pixel
mapping, restoration of original 3D axes and rejection of degenerate input.
Three integrated tests rectify synthetic 180-degree camera rotation and a
vertical stereo pair, match them and recover the analytic plane in original
axes. They also check seed lineage, blank-pair diagnostics, provenance,
dimensions and cancellation. These tests do not use actual Raptor calibration.

A 60-case independent OpenCV comparison includes differing intrinsics,
Brown-5 distortion and a rotation about all three axes. It checks ideal pixels,
source-pixel roundtrip, epipolar rows, pose composition, original-frame
triangulation and analytic ground truth.

```sh
cargo run --release -p scancraft-calibration --example rectification_reference > evidence/rectification-reference-fixture.json
.venv-reference/bin/python tools/compare_rectification_opencv.py evidence/rectification-reference-fixture.json evidence/rectification-opencv-comparison.json
cargo test -p scancraft-calibration -p scancraft-depth
```

The test-only Python requirements are pinned in tools/reference-requirements.txt.
Reference thresholds and observed errors are saved with library versions.
Synthetic numerical residuals cannot be interpreted as physical dimensional
accuracy, imaging quality, matching success rate, verified synchronization,
laser decoding or Raptor mode qualification. Native controls, physical
calibration, validated paired recordings, feature selection, subpixel matching,
GUI scanning and the genuine M2 scanner point cloud remain incomplete.
