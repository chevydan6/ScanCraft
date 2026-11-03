# Real-board calibration candidate and solver correction

Status on 2026-11-03: physical calibration is authorized. Native camera and
stereo fitting have now processed genuine Raptor observations. These are
experimental candidates, not an activated calibration or a qualified object scan.
Physical units, measurement uncertainty, coverage, exposure synchronization and
three independent object trials remain unverified.

## Captures and observation lineage

The owner returned the board. The first new capture retained five color frames
and ten frames per scanning camera; the left image clipped the top marker. After
repositioning, a second ten-frame-per-camera capture showed the full pattern in
both cameras. Native recording replay passed and reported stream/illumination
GET values matched before and after both acquisitions. These short captures do
not qualify sustained acquisition or the ongoing rejected-buffer rate.

For the complete new pose, `inspect-components` and
`scancraft_calibration::target::associate` matched all 42 markers per camera,
including four orientation anchors. Maximum homography residuals were 1.4524 and
1.3236 pixels. The existing 0.5-pixel component-shape gate rejected two right
observations; their raw measurements were retained. Association used the existing
nominal pattern and unchanged 10/3-pixel search/refinement radii. Numerical fits
used the exact retained QR coordinates, not rounded grid coordinates.

The three older board poses supply training observations. The fresh complete
pose was excluded before fitting. An initial experiment used index 5 in both
recordings, which had different sensor timestamps in two older captures. Its
results remain diagnostic evidence, superseded by timestamp-matched training:

| Training pose | Left/right frame index | Host timestamp separation | Retained small markers, left/right |
|---|---:|---:|---:|
| Board front | 5 / 6 | 16 microseconds | 37 / 38 |
| Board with 30 mm caliper | 5 / 4 | 19 microseconds | 31 / 30 |
| Board with 50 mm caliper | 5 / 5 | 22 microseconds | 30 / 28 |

Each pair has equal recorded sensor PTS; this is not proof of simultaneous
exposures. All six original recordings passed full native replay. Caliper jaws
are not training points or independent ground truth. Source hashes, selected
indices, exact inputs, marker rejections and executable hashes remain private.

## Native fitting and independent numerical checks

`scancraft-cli calibrate-planar` runs `planar::initialize_planar` followed by
`refinement::refine_planar`. Zero-skew Brown5 fits use the existing 80-iteration,
20-pixel initializer and 2-pixel final individual-error limits. They fit only
three poses per camera; this is below the planned physical-qualification coverage.

| Timestamp-matched fit | RMS image error | Maximum image error |
|---|---:|---:|
| Left camera | 0.096407 px | 0.231246 px |
| Right camera | 0.116744 px | 0.364299 px |
| Fixed-intrinsic stereo, interior observations | 0.248642 px | 1.049905 px |

The refinement's maximum observed normalized radius is **not** a qualified lens
domain. Using that radius (with only a 1e-8 relative numerical margin) for the
full stereo request rejected a noisy boundary observation during inverse
projection. The original failure remains recorded. A separate, explicitly
restricted experiment kept only observations within 98% of both observed radii:
two training pairs excluded, 36/28/27 pairs retained. No radius or error threshold
was enlarged. This limited-domain fit does not establish full-image operation.

Independent NumPy projection recomputed the corrected stereo RMS within
5.9e-15 pixels. OpenCV 5.0.0 `stereoCalibrate(CALIB_FIX_INTRINSIC)` on the same
interior observations returned 0.248641577-pixel RMS, versus native
0.248641663. Rotation-matrix Frobenius difference was 6.71e-8 and translation
difference 1.66e-5 **QR raw units**. OpenCV uses float32 observation arrays in this
comparison. These are numerical agreement checks, not measured physical accuracy.

For the complete held-out pose, fixed-lens OpenCV pose-only fitting gave
0.135030/0.169558-pixel RMS. Pose parameters were fitted on those observations;
lens parameters were frozen. Two right-camera observations exceeded the training
radius. Native triangulation of all 38 small-marker pairs accepted 35; three
failed the lens-domain check. Applying the unchanged image-shape gate retained
34. Rigid alignment to exact QR coordinates, without fitting scale, yielded
0.188872 raw-unit RMS and 0.340726 maximum error. Maximum absolute error over all
pairwise target distances was 0.193632 raw units. These are sparse board markers,
not a reconstructed object surface or an independent millimeter accuracy trial.

The request schema names coordinates `target_xy_mm`; the private experiments
explicitly supplied the QR-unit hypothesis as `user_supplied_unverified` and
recorded that units remain unverified. No RAPCAL01 record, device binding,
qualified millimeter export or active capture calibration was created.

## Corrected stopping condition

Developer finding `DEV-STEREO-STATIONARY-ROUNDOFF`, fixed in
`b417ebd6903772fc1465be107d09c4e1a0522719`, affects
`crates/scancraft-calibration/src/stereo_fit.rs::fit_stereo_extrinsics`.
The timestamp-matched interior fit reached scaled gradient 5.09e-8, satisfying
the existing final 1e-6 gate. Objective rounding prevented another strict cost
decrease, but the loop required either gradient below 1e-8 or an accepted small
step, and therefore returned a failure.

When all damping trials fail to decrease cost, the solver now permits the current
state to proceed only if it satisfies the existing final stationarity gate.
Final individual residual, information, lens-domain and rig checks remain.
No worsening candidate is adopted and no gate value, iteration cap, dependency,
record format or hardware behavior changes.

The entirely synthetic regression
`stationary_noisy_fit_survives_objective_roundoff_and_keeps_final_gates` uses three
views of the existing test rig with deterministic noise. The old implementation
reproduced the plateau failure. The corrected test checks the known optimum RMS,
stationarity and information, plus rejection under a stricter residual limit and
a one-iteration budget. No hardware observations or vendor coefficients enter
this test. The corrected native CLI then passed the retained real failing input.

## Validation and remaining work

The full `tools/check.sh` gate passed: formatting, strict all-target Clippy,
832 tests (one existing ignored test), and optimized workspace build. No Rust
source changed after the gate. CodeRabbit usage lookup still returned unavailable
for included quota, billing period and credits; no review or paid request was
made, and no CodeRabbit approval is claimed.

Private evidence directory `board-fit-evaluation-q_17v0is` contains exact raw
source hashes, requests, fit bundles, failure logs, isolated solver instrumentation,
reference scripts, test/build logs and corrected results. Its `matched-training`
subdirectory is the authoritative timestamp-matched evaluation. The initial
OpenCV helper used a removed API name; it was corrected for the installed 5.0.0
API and retained separately. An initial isolated test build needed corrected
fixture paths. These harness failures did not modify capture data or production
source. Only aggregate results belong in the public repository.

Next: collect diverse stationary board tilts/depths with image-edge coverage,
corroborate physical target scale with independent measurements, rerun calibration
and reserved-pose tests, then bind the validated model to reconstruction and
measure a separate object in three trials. Continue investigating transport loss.
The first independently validated object scan remains unfinished.
