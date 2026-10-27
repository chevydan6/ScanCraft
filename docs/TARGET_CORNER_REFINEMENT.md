# Offline target-corner measurements

Experimental seeded measurement preparation for future calibration. This does
not perform physical calibration, detect or index a calibration board, establish
scanner/mode binding, or install a capture calibration. Physical calibration
remains deferred by owner instruction.

`scancraft_image::corners::refine_corner` refines one supplied pixel seed in an
immutable packed 8-bit grayscale image. `scancraft_workflow::target_corners::run`
connects those measurements to the existing planar-observation data model, with
bounded input reading, cooperative cancellation and no-overwrite publication.
The CLI uses the shared workflow:

```sh
scancraft-cli refine-target-corners image.png seeds.json NEW_OUTPUT_DIRECTORY
```

Example seed request (target positions are declared millimeters, never inferred):

```json
{
  "schema_version": 1,
  "provenance": "user_supplied_unverified",
  "image_size": [640, 480],
  "view": {"observations": [
    {"target_xy_mm": [0.0, 0.0], "pixel": [120.0, 100.0]},
    {"target_xy_mm": [20.0, 0.0], "pixel": [180.0, 100.0]}
  ]},
  "settings": {
    "radius": 5, "max_iterations": 40, "epsilon_px": 0.0001,
    "max_shift_px": 4.0, "min_eigenvalue": 0.00001,
    "min_eigenvalue_ratio": 0.01
  }
}
```

The PNG must be L8 with exactly the declared dimensions and at most 4,194,304
pixels; compressed input is capped at 16 MiB. Requests are capped at 4 MiB and
1–4096 observations, rejecting unknown fields and duplicate/nonfinite/excessive
target coordinates. Every corner must succeed. Failure identifies its zero-based
index and does not publish a final directory or silently discard associations.
Partial output, if writing has started, is retained with an explicit diagnostic.
Input files remain unchanged. Output retains:

- `image.png` and `request.json`: exact supplied bytes.
- `refined_view.json`: `PlanarView`, preserving target associations and order.
- `diagnostics.json`: seed/refined pixel, iterations and gradient diagnostics.
- `manifest.json`: artifact lengths/CRC32 and false binding/accuracy/association
  qualification flags. CRC32 detects accidental corruption, not authenticity.

A caller may explicitly assemble multiple `refined_view.json` values into the
existing `calibrate-planar` or `calibrate-stereo-extrinsics` observation request.
No automatic fit, activation or hardware write occurs. Those workflows still
require their own minimum points/views, known dimensions and quality checks.

## Method, origin and design rationale

Independent Rust implementation of the gradient-orthogonality least-squares
method described for OpenCV
[`cornerSubPix`](https://docs.opencv.org/4.1.2/dd/d1a/group__imgproc__feature.html).
No OpenCV source or assets are copied, translated or adapted. There is no imported
source revision; the identified mathematical reference is OpenCV documentation
4.1.2. New Rust/Python code follows ScanCraft's MIT license. Existing OpenCV and
NumPy test-only environment requirements are unchanged; no new production
libraries, native dependencies or Rust manifest changes are introduced.

At window sample p with normalized image gradient g, minimize
`sum(w * (g dot (q-p))^2)`. Form `G = sum(w*g*g^T)` and the relative-coordinate
right-hand side, then solve the symmetric 2×2 normal system for displacement.
The window is Gaussian weighted, recentered each iteration, and bilinearly
sampled with one-pixel central differences. Raw intensities are normalized by
255 before gradient products. No border extrapolation is allowed.

The implementation rejects insufficient contrast/two-dimensional rank, windows
crossing the image, excessive total movement from the seed, nonfinite results,
and iteration exhaustion. Its minimum eigenvalue and ratio describe the last
solved gradient window; they are not position covariance or physical uncertainty.
Cancellation is checked per iteration, with at most 961 window samples. New
window storage is at most 961 tuples; no full-frame gradient arrays are allocated.
No wall-clock latency, SIMD, GPU or throughput advantage has been benchmarked.

This Rust implementation follows the existing native-Rust mission and reuses
ScanCraft's image/data/publication boundaries. An OpenCV integration would offer
mature full-board detection and indexing; it remains a useful offline numerical
reference. This seeded primitive does not replace that functionality or prove a
performance/accuracy advantage over it. The narrow implementation adds explicit
failure/immutability/cancellation policies suitable for shared offline jobs.

## Reproducible validation

```sh
cargo run --frozen -p scancraft-image --example corner_reference > fixture.json
python tools/compare_target_corners_opencv.py fixture.json reference-report.json
cargo test --frozen -p scancraft-image corners
cargo test --frozen -p scancraft-workflow target_corners
PATH="$HOME/.cargo/bin:$PATH" tools/check.sh
```

Use the existing isolated Python reference environment from
`tools/reference-requirements.txt`, not an unreviewed system package install.
The generator produces 12 synthetic 64×64 saddle-corner images covering two
subpixel positions, three rotation/noise combinations and both polarities.
The independent NumPy oracle uses vectorized sampling and a general linear solve;
OpenCV `cornerSubPix` provides a separate mature reference. These successful
interior cases do not compare differing border/rank failure policies.

Measured with OpenCV 5.0.0/NumPy 2.5.4: largest Euclidean differences were
5.27e-8 px from NumPy, 1.06e-6 px from OpenCV, and 0.0284 px from synthetic truth.
Comparison tolerances declared in the script are 1e-4, 1e-3 and 0.08 px,
respectively. These measurements describe only this quantized, smoothly blurred
synthetic family. They are not a calibrated Raptor pixel error, metric accuracy,
registration result, hardware test, statistical confidence interval or benchmark.

Unit tests also exercise flat/single-edge rejection, invalid configuration/seeds,
window borders, displacement, cancellation, iteration exhaustion, input
immutability, exact retained bytes/target association, artifact CRCs and
no-overwrite publication. Remaining work includes robust full-board detection,
identity/order validation, real blur/glare/occlusion tests, multi-view coverage,
uncertainty estimates and explicit source/mode/optical binding. No hardware or
mission acceptance gate is closed by these synthetic measurements.
