# Component shape diagnostics

`inspect-components` extends raw recording component observations with an ellipse
fit and an explicit residual threshold:

```sh
scancraft-cli inspect-components input.raptor 5 100 201 10000 10000 0.5
```

Arguments match `detect-components`, followed by the maximum RMS first-order
Sampson residual in pixels. There is no default quality threshold. The command
verifies the complete recording and footer before processing the selected frame.
It retains every raw component and its original binary centroid, adding either
fit diagnostics or a specific error and `rms_gate_passed=false`. It never replaces
the centroid with the fitted center or silently deletes failed observations.
The existing `detect-components` output remains unchanged.

`scancraft_image::shape::measure` revalidates the component against the image and
threshold in a padded region. Bounds, area, centroid, intensity peak and saturation
must agree, and the region must contain exactly one isolated component. Its
boundary consists of foreground pixel centers adjacent to background in one of
the four cardinal directions. Interior hole boundaries participate. This is
integer-pixel boundary extraction, not subpixel contour localization or hole filling.

`scancraft_calibration::ellipse::fit` independently implements general-conic
least squares. It centers samples and scales by their RMS radius, constructs
rows `[x², xy, y², x, y, 1]`, and takes the smallest right singular vector under
unit coefficient norm. It rejects insufficient rank, poorly separated smallest
singular values and nonelliptical quadratic forms. For a valid ellipse it reports
center, semiaxes, major-axis direction and residual statistics. Major-axis sign
is arbitrary; circle orientation is undefined. The residual is `|F|/||gradient F||`,
a first-order approximation rather than exact orthogonal distance.

The conic formulation and its limitations are discussed in section 2 of
[Fitzgibbon, Pilu and Fisher, PAMI 1999](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/ellipse-pami.pdf).
This implementation uses the general-conic approach, not that paper's proposed
ellipse-constrained method or OpenCV's `fitEllipseDirect`. No upstream code is
copied or translated. Existing nalgebra SVD/eigendecomposition supplies numerical
linear algebra; no dependency is added. OpenCV and NumPy remain optional reference
tools, with their algorithms described in the
[OpenCV shape documentation](https://docs.opencv.org/5.0/main_modules/geometry_shape.html).

Bounds: at most 512 components in the CLI shape pass, 65,536 pixels per padded
region, and 6–4,096 boundary samples per fit. Coordinates must be finite and
within +/-1e6; unrepresentable ellipses return errors. Source/image processing
checks the existing cancellation token; the fitter checks its callback around
bounded SVD and every 128 samples. Individual SVD calls are not interruptible.
This does not establish a measured latency or whole-application shutdown bound.

Tests cover exact rotated ellipses/circles, similarity transforms, input ordering,
cut shapes, nonelliptical/degenerate/nonfinite inputs, cancellation, source mismatch,
extra components, region limits, holes, recording corruption and retention of
raw observations on failed fits. Optional reference commands are:

```sh
python tools/check_component_shapes.py target/release/scancraft-cli
python tools/compare_component_shapes.py image.png native-inspection.json
```

The reference uses the existing OpenCV component/PNG-payload-CRC check, then
independent NumPy SVD on the same cardinal boundary. It validates successful fits;
native error cases are counted but not numerically certified by that tool.
It does not independently reverify the source recording. Development checks on
2026-11-02 passed 32 synthetic images and 227 real-image fits from six saved
Raptor frames. Maximum center differences were below 2e-13 pixels. Numerical
agreement does not establish physical accuracy or target identity.

The experimental 0.5-pixel threshold flags the visually obscured marker that
previously passed stereo reprojection checks. It also flags other irregular or
hole-containing regions. These are the same development images used to explore
the method; no held-out sensitivity/specificity or universal threshold is claimed.
Fitting an ellipse to an obscured mark barely changed its biased center, so a
successful fit cannot be treated as an occlusion repair. Lens distortion,
perspective circle-center bias, threshold/blur effects and partial visibility
remain relevant. Outputs explicitly leave `physical_center_verified=false`.
No factory calibration is activated, no device controls are touched, and private
images/board coordinates are not included in source or review uploads.

Final Rust gate: 675 tests passed, formatting/strict all-target Clippy/release
build passed. The first attempt failed with `/tmp` quota errors. Rerunning with
a dedicated home-filesystem `TMPDIR` passed; no tests or assertions were disabled.
