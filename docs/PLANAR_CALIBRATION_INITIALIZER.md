# Native planar calibration initializer

Status: experimental, software-only prerequisite for future calibration. No
scanner access, physical calibration, automatic factory conversion or active
capture-calibration change occurs. The production scanning path is unchanged.

`scancraft_calibration::planar::initialize_planar(image_size, views, max_error_px)`
accepts already associated metric target-plane/pixel observations from one fixed
camera. It estimates five pinhole intrinsics (including skew) and one
`camera_from_target` rigid pose per view. Target coordinates are `[X,Y,0]` in
millimeters; camera axes are +X right, +Y down, +Z forward. The caller supplies a
maximum **individual Euclidean pixel error**, not a physical-accuracy threshold.
Per-view/global RMS and maximum errors are returned.

The result deliberately uses `InitialIntrinsics`/`PlanarInitialization`, rather
than `CameraModel` or `CalibrationRecord`. It has no distortion model, qualified
lens domain, device/mode binding or capture installation API. Supplied target
coordinates determine translation scale: incorrect board dimensions produce
incorrect metric poses even with a small image residual. A good fit alone does
not establish correct correspondences, lens coverage or physical accuracy.

## Method and provenance

Independent Rust implementation based on Zhengyou Zhang,
[A Flexible New Technique for Camera Calibration, MSR-TR-98-71](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/tr98-71.pdf),
sections 2.3/3.1 and appendices A/C (report updated August 13, 2008). No upstream
source code or paper assets are copied. The new implementation uses ScanCraft's
MIT license and the existing Apache-2.0 nalgebra 0.35.0 dependency; no new
production dependencies or notice changes are needed.

1. Normalize target and pixel coordinates independently per view, then estimate
   a homography by homogeneous DLT with bounded-iteration SVD. Pad the four-point
   8×9 system so thin SVD includes its null vector.
2. Apply a common image-center/maximum-dimension similarity to the homographies.
   Stack the two orthogonality/equal-length constraints per view, normalize their
   rows, and solve the six-coefficient image-of-absolute-conic system.
3. Require a positive-definite conic. Cholesky recovers the upper-triangular
   intrinsic matrix from `B = K^-T K^-1`; invert the image similarity afterward.
4. Recover each target pose, choose homography sign at an observed point, project
   rotation to SO(3) with determinant correction, and check all observed depths
   and reprojection errors. The target-coordinate origin may lie outside view.

This is the linear initialization stage, not Zhang's full nonlinear/distortion
calibration procedure. It has no checkerboard detector, bundle adjustment,
robust correspondence/outlier selection, uncertainty estimate or stereo joint
optimization. Those remain necessary development work. It is not claimed faster
or more accurate than a mature calibration library.

## Input and numerical policies

The Rust API bounds inputs to 3–64 views, 4–4096 observations per view, positive
image dimensions at most one million each, and finite target coordinates within
±1e9 mm. Pixels must lie in `[-0.5,width-0.5) × [-0.5,height-0.5)`; duplicate
target coordinates are rejected. Views need diverse plane orientations, not
merely different translations. Exact repeated views and collinear targets fail.

After normalization, required singular directions must have singular value at
least `1e-8` times the largest; the smallest/next singular-value ratio must be at
most `0.25`. These conservative numerical policies reject weak/inconsistent
systems; they are not uncertainty, coverage or physical calibration guarantees.
Large-angle lens distortion can bias this pinhole initializer or make it reject
an observation set. It must not be installed as a qualified Raptor model.

## Reproduction and evidence

```sh
cargo test --frozen -p scancraft-calibration planar::tests
cargo run --frozen --release -p scancraft-calibration --example planar_reference > /tmp/planar-cases.json
python3 -m venv /tmp/planar-reference-venv
/tmp/planar-reference-venv/bin/python -m pip install -r tools/reference-requirements.txt
/tmp/planar-reference-venv/bin/python tools/compare_planar_opencv.py /tmp/planar-cases.json /tmp/planar-comparison.json
tools/check.sh
```

Six Rust tests cover known asymmetric intrinsics and positive/negative skew,
metric poses, minimum three views/four corners, noisy fits/strict rejection,
degeneracy/duplicates/outliers, input/resource bounds and unit/origin invariance.
The independent example supplies three synthetic cases (eight views ×63 points):
exact zero skew, exact nonzero skew, and deterministic 0.02-pixel noise.

`tools/compare_planar_opencv.py` recomputes all reprojection metrics with NumPy,
checks known intrinsics/poses, and compares zero-skew cases with OpenCV's refined,
fixed-zero-distortion `calibrateCamera` fit. OpenCV uses float32 observations and
zero skew; Rust estimates skew in f64 without nonlinear refinement. They are not
identical algorithms or equivalent optimization objectives. The skew case is
checked against analytical truth, not unsupported OpenCV skew estimation.
See the [OpenCV reference](https://docs.opencv.org/4.13.0/d9/d0c/group__calib3d.html).
Pinned OpenCV 5.0.0 and NumPy 2.5.4 are isolated test-only dependencies.

Observed maximum intrinsic discrepancies: exact Rust versus truth below
6e-12 pixels, exact Rust versus refined OpenCV about 0.000212 pixels; noisy Rust
versus truth about 0.109 pixels and versus OpenCV about 0.085 pixels. These are
synthetic numerical results only. The JSON report records exact limits and
fixture/source hashes. No actual target capture, Raptor optical binding,
synchronization, dimensional accuracy, throughput or hardware qualification is
established. SCAN-REV-006 remains under independent review.

The combined workspace gate passed 589 Rust tests, formatting, strict all-target
Clippy and release build. Deliberately changing an exact-case focal length by one
pixel makes the reference checker fail with exit 1; this negative control is
recorded with the reference report. Automated review is a separate gate.
