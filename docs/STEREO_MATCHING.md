# Sparse rectified stereo matching

`scancraft-depth::matching` implements integer-pixel sparse ZNCC matching in Rust.
It is an image-processing component, not a validated Raptor scanning mode. It
accepts caller-supplied seed coordinates and explicitly calibrated rectified
masked images. It does not find features, estimate calibration, establish
exposure synchronization, control illumination, or rectify an arbitrary rig.
The desktop app exposes offline matching/reconstruction through the shared file workflow. Live scanning remains unfinished.

## Geometry and matching

Camera axes remain +X right / +Y down / +Z forward. The required ideal camera
pair has identical intrinsics, zero skew and distortion, identity relative
rotation, and nonzero horizontal translation. These metadata conditions are
checked exactly. A source camera with small residual rotation/distortion is
not silently treated as rectified. Matching uses signed integer disparity
`x_left - x_right`, so either horizontal baseline direction is supported.
The result carries the exact rectified geometry used for processing. Its pixel
coordinates belong to that ideal geometry, not the original distorted images.
Never triangulate them with a different original camera calibration.

Each patch must fit wholly inside the image, validity mask and declared lens
coverage disk. Intensity values use the 0..255 scale. Subtract the patch mean,
normalize by its Euclidean norm, and compute the dot product with a target
patch. This zero-mean normalized correlation is invariant to a positive affine
brightness change when values have not clipped. Texture is checked using
population standard deviation before normalization. Low texture, mask gaps or
unsupported lens coverage reject a patch.

Search every integer disparity in the requested range. An accepted forward
winner must exceed both an absolute correlation threshold and the runner-up
by an explicit absolute ZNCC margin. Every other valid candidate counts in the
runner-up comparison, including adjacent pixels; one candidate is insufficient
to establish uniqueness. Search independently in reverse, require the same
thresholds and require its winner to be the original seed. Repeated texture,
occlusions and weak matches can still defeat local matching; passing these
checks is a heuristic confidence result, not proof of a correct physical match.
The default remains integer-only. Optional bounded subpixel refinement is described
in [SUBPIXEL_STEREO.md](SUBPIXEL_STEREO.md). No dense cost aggregation or learned
inference is present.

The reference concepts are documented by OpenCV's official
[stereo correspondence API](https://docs.opencv.org/5.0.0-alpha/dd/d86/group__stereo.html)
and [template matching API](https://docs.opencv.org/4.x/df/dfb/group__imgproc__object.html).
This implementation uses ZNCC and an absolute score gap; it does not claim to
reimplement OpenCV StereoBM/SGBM or their percentage uniqueness parameter.
No OpenCV production runtime or source code is included.

## Failure handling and bounds

Results retain original seed indices, matched pixel positions, forward and
reverse scores and runner-up scores. Rejected seeds retain explicit reasons:
invalid/low-texture seed, insufficient candidates, weak score, ambiguity,
reverse rejection or reverse mismatch. Configuration errors return no result.
Duplicate seeds, incompatible dimensions, nonrectified geometry, invalid
parameters or requests exceeding the work budget fail before matching.
Cancellation checks each seed and each disparity and returns no partial result.

Limits are 100,000 seeds, patch radius 1..15, search span 2..1024 disparities,
and at most 100 million bidirectional patch-sample visits per request. The
budget includes template reads and is not a runtime guarantee. This sparse
reference implementation allocates candidate patches and has not been
qualified for full-resolution dense or live performance. Split large requests
into explicit batches rather than silently discarding seeds.

## Evidence

Nine matcher tests cover an affine-brightness shifted image pair reconstructed
into an analytic plane, repetitive and flat texture, masks/borders, weak scores,
occlusion/reverse mismatch, reverse ambiguity, single-candidate rejection,
signed disparity, lens coverage, invalid configurations and cancellation.
The masked-image constructor separately tests finite intensities, sizes and
invalid-sample semantics.

An independent OpenCV comparison uses a synthetic two-depth image pair with
known disparities 4 and 8 pixels (400 mm and 200 mm in the synthetic rig).
It compares every winner and runner-up correlation using
`cv2.matchTemplate(TM_CCOEFF_NORMED)` in both directions, then triangulates
with `cv2.triangulatePoints` and compares against analytic coordinates.

```sh
cargo run --release -p scancraft-depth --example stereo_reference > evidence/stereo-reference-fixture.json
.venv-reference/bin/python tools/compare_stereo_opencv.py evidence/stereo-reference-fixture.json evidence/stereo-opencv-comparison.json
```

Use the pinned test-only environment in tools/reference-requirements.txt.
The fixture, test thresholds and resulting residuals are recorded. This
synthetic numerical evidence does not qualify real scanner dimensional error,
mode support, tracking, exposure timing, laser correspondence or reconstruction.
