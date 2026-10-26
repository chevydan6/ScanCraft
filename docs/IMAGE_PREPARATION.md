# Native image preparation

`scancraft-image` provides borrowed packed GREY image views and checked reusable
undistortion/rotation maps, independently implemented in Rust. These are
processing components, not a claim that Raptor lens calibration or a scanning
mode is validated. The desktop app now uses this stage for offline stereo reconstruction; live camera preview remains separate.

Integer coordinates are pixel centers. For each ideal output pixel, invert
its intrinsics (including skew), rotate the ray with `source_from_output`,
project through the calibrated source Brown-5 model and sample the original
image with bilinear interpolation. This destination-to-source convention
avoids forward-warp holes. Output rays and source projections must remain
inside their declared calibrated disks. Output distortion must be zero.
The source-from-output rotation must belong to SO(3). Translation is absent:
this operation changes viewing direction, not camera position or stereo scale.

The mathematical reference is the official OpenCV documentation for
[image undistortion and rectification](https://docs.opencv.org/4.8.0/d9/d0c/group__calib3d.html)
and [geometric transforms](https://docs.opencv.org/5.0/main_modules/imgproc_transform.html).
OpenCV's rectification R maps source to output; our explicitly named rotation
is its inverse. No OpenCV code or production dependency is included.

Pixels outside source sample-center bounds are invalid, without extrapolation
or guessed padding. Only roundoff within `64 * f64::EPSILON * max_dimension`
pixels at a border is clamped. A zero stored intensity is not a valid sample
unless its mask is true. `sample(x,y)` returns `None` for invalid coordinates;
raw intensity consumers must also use `valid()`. Floating-point intensities
retain fractional 8-bit values. This step performs no denoising, thresholding,
normalization, correspondence detection or confidence estimation.

Map construction validates models once through an immutable `PreparedCamera`.
The private map holds at most 4,194,304 entries, reuses its coordinates across
frames, and supports cancellation every 1024 pixels. Applying a map checks
image size, allocates a fresh result, and exposes no partial result on failure.
Source and destination image sizes are bounded; pixels are never silently
resized to make a calibration fit. Source images borrow raw payload storage.
`GrayImage::from_frame` preserves core frame checks including error flags,
byte lengths and genuine sensor PTS. GREY is the only supported input here;
Y14 packing, RGB conversion and actual optical image quality remain separate.

Ten unit tests cover identity including full-resolution nonbinary intrinsics,
analytic bilinear interpolation, singleton/border cases, masks, lens coverage,
rotation direction, distorted/skewed inverse rays, malformed/oversized inputs,
cancellation and the zero-copy raw-frame adapter.

Reproduce the independent synthetic reference with the pinned test-only Python
environment from `tools/reference-requirements.txt`:

```sh
cargo run --release -p scancraft-image --example undistort_reference > evidence/undistort-reference-fixture.json
.venv-reference/bin/python tools/compare_undistort_opencv.py evidence/undistort-reference-fixture.json evidence/undistort-opencv-comparison.json
cargo run --release -p scancraft-image --example undistort_benchmark
```

The reference compares source coordinates, full validity masks and intensities.
It distinguishes CV_32F map rounding from interpolation coefficient quantization
permitted by some OpenCV backends. The reported version and observed residuals
are recorded, rather than assuming all versions quantize identically. This
fixture has zero skew for the OpenCV comparison; separate analytic Rust tests
cover skew. No real scanner calibration parameters or private images are used.
