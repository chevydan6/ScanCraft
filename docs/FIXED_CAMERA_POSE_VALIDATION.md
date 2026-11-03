# Fixed-camera planar pose validation

`scancraft-cli validate-planar-pose OBSERVATIONS_JSON NEW_OUTPUT_DIRECTORY`
fits only the six target-pose parameters against a supplied fixed camera. It
provides a native numerical check for held-out board images. It does not fit
intrinsics/distortion, activate calibration, or open scanner hardware.

## Contract and artifacts

Schema 1 request fields are `schema_version`, `provenance` (the existing
`synthetic_fixture` or `user_supplied_unverified` declaration), `fixed_camera`,
`observations`, and `fitting`. Each observation has `target_xy: [x,y]` and
`pixel: [u,v]`. `fitting` specifies `max_iterations` (1..100) and positive finite
`max_reprojection_error_px`. The full CameraModel includes its supplied
`valid_radius`; this command never enlarges it. Input accepts at most 1 MiB;
the shared bounded reader may consume one additional byte to detect overflow.
Unknown fields and schemas are rejected.

Coordinates are in the caller's supplied target units. Translation is serialized
as `translation_in_supplied_target_units`, with `rotation` defining
`p_camera = R * [target_x,target_y,0] + t`. No unit conversion or millimeter
assumption is made. Existing numerical helpers use historical internal `*_mm`
fields, but their homogeneous calculations do not establish physical scale;
these names are not exposed by this new interface.

The new directory contains the exact `observations.json`, diagnostic `fit.json`
and CRC/length `manifest.json`, using the existing staged, synced, no-replace
publisher. Existing paths are refused. Cancellation and errors preserve the
publisher's existing partial-output semantics. `fit.json` reports the unchanged
camera, every predicted-minus-observed 2D residual in input order, initial/final
RMS, maximum residual, accepted steps, scaled gradient and information diagnostic.
No observations are silently removed. RMS is sqrt(sum of squared Euclidean pixel
errors / number of points), not RMS per scalar coordinate.

Unit verification, physical accuracy, device binding and held-out independence
are explicitly false. The numerical request alone cannot establish that lens
training excluded these images. Keep the source session and its training/validation
lineage alongside the output; see [board sessions](BOARD_CALIBRATION_SESSION.md).
Session assembly and the GUI do not yet invoke this check automatically.

## Method, provenance and limits

`scancraft-calibration/src/planar_pose.rs::fit` is independently written Rust
reprojection least squares. It reuses the existing validated inverse rays,
`stereo_fit::pose_seed` homography initialization, `planar::validate_view`, native
Brown projection/local SO(3) derivatives and `normal_blocks` solver. The six pose
unknowns occupy the shared solver's global block with no subordinate views.
Damped normal steps must decrease the objective; numerical stationarity,
positive depth, lens domain, reprojection threshold and undamped scaled
information checks are required before returning a fit. No new dependency.

This has comparable *problem scope* to OpenCV's iterative planar solvePnP, whose
[official method description](https://docs.opencv.org/4.13.0/d5/d1f/calib3d_solvePnP.html)
describes homography initialization and reprojection optimization. No OpenCV code
was copied or translated. ScanCraft uses its existing exponential-map rotation
increments; algorithmic similarity is not an attribution to an upstream source
revision. New original source remains under the repository MIT license; existing
nalgebra and other dependency notices continue to apply.

Requires 4..4096 distinct finite target locations, in-bounds observed pixels and
nondegenerate homography/information. Target coordinates are bounded to magnitude
1e9 in supplied units. Both observed inverse rays and trial projected points must
remain within the camera's domain. Boundary noise can therefore reject a view;
the caller must retain the failure and document any separate subset experiment.
There is no robust loss, outlier removal, nonplanar fitting, alternate-pose search,
pose-uniqueness proof, uncertainty estimate or physical calibration qualification.
Small residuals can coexist with wrong units or a poorly identified camera.

## Verification

Five new tests cover exact distorted poses with zero/nonzero skew and three
coordinate scales; noisy fits and every reported residual; duplicate/collinear/
invalid/domain/iteration/cancellation rejection; objective gradients against
finite differences; and workflow exact-request/unit/digest/no-overwrite behavior.
The complete gate passes 850 tests, with one existing ignored test, formatting,
strict all-target Clippy and optimized workspace builds. Run `bash tools/check.sh`
with a writable TMPDIR. Focused tests: `cargo test --frozen -p scancraft-calibration
-p scancraft-workflow planar_pose`.

Real saved Raptor board evidence is private under developer artifact
`fixed-camera-pose-d7tddglu`. `verify_real.py` reads the preceding three-training/
one-validation session dataset and frozen camera models, invokes the optimized
CLI, verifies exact request copies, CRCs and source SHA-256, and independently
fits/reprojects the same observations with OpenCV 5.0.0. Initial failure and script
are retained separately from final comparison output. No new capture occurred.

| Genuine held-out case | Points | Native RMS (px) | OpenCV RMS (px) | Native maximum (px) |
| --- | ---: | ---: | ---: | ---: |
| Left full retained view | 38 | 0.135029585575 | 0.135029585575 | 0.308040453850 |
| Right full retained view | 36 | Rejected | Previous unconstrained result 0.169558284717 | Not accepted |
| Right explicitly separate inner-98%-domain subset | 33 | 0.168662045384 | 0.168662045384 | 0.372131442665 |

Two right observations exceed training lens coverage. The separate 98% subset
excludes those plus one near-boundary observation; the original camera is
unchanged and exclusions/identities are retained. It is not a pass for the full
right view. Native/OpenCV rotation Frobenius differences are below 1.01e-10;
translation differences below 1.11e-8 **supplied target units**. Independently
reprojected native residuals differ by at most 1.37e-12 px. These are numerical
agreement measurements, not millimeter accuracy, throughput or reconstruction
qualification. Six pose parameters were fitted on each validation view.

The QR coordinate scale still lacks independent physical verification. A complete
object surface, repeated dimensional trials, diverse calibration coverage and
end-to-end scan/edit/save/export qualification remain outstanding. CodeRabbit
quota lookup is unavailable (`lookup_failed`); this milestone has no new completed
CodeRabbit review. Independent reviewer findings remain untouched.
