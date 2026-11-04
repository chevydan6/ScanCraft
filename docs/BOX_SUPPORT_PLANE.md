# Supporting-plane diagnostic from genuine Raptor images

Status: **experimental; every supporting-plane qualification failed**. This is
additional validation of the flood/gain capture in `FLOOD_CAPTURE_GAIN.md`, using
application source `56563c1039a8b27c88e08a7e897132c093936461`. No application code,
calibration coefficients, acceptance limits or original sensor samples changed.

## Method and provenance

**Confirmed:** selected flood frames are left index 5 and right index 3, equal
sensor PTS with 13 us host timestamp separation. This is not independent exposure
synchronization. Native extraction previously validated the full recordings; PNG
payload CRCs were checked again. Thresholds 80/100/120, area 5..1000 and ellipse
Sampson RMS <=0.5 px were specified before evaluating geometry. The native
`scancraft_image::components::detect` and `shape::measure` implementations supply
binary-area centroids and shape diagnostics. Saturated samples remain reported;
centroids are not asserted to be exact physical sticker centers.

Initial full-image attempts failed the 512 examined-component bound. Explicitly
allowing up to the existing supported 100,000 examined components still failed
the CLI's separate 512 accepted-component shape limit. Both sets of failures are
retained. A private Rust adapter then limits shape evaluation to the already
specified turntable region, y>=550; it retains the 512 shape-work bound. It reads
the original full PNG without cropping, masking, resampling or changing pixel
coordinates. All area-accepted observations, including those outside the ROI,
remain in its output. This is an exploratory adaptation, not pristine held-out
validation or an application feature.

Every shape-passing left/right combination is passed to the existing
`StereoCalibration::prepare` / `PreparedStereo::triangulate` with the unchanged
recorded camera model and prior limits: depth 100..1000 RAW, minimum ray angle
0.0001 rad, maximum ray gap 1 RAW and reprojection error 2 px. All accepted
alternatives and errors are retained. A pair enters the plane hypothesis only
when each endpoint has exactly one accepted partner. Original image-derived box
silhouette exclusion and y>=550 tests are applied; no match is selected using its
plane residual. No RANSAC, outlier removal, nearest-to-truth selection, alignment,
scale fit, plane flattening or orthogonality constraint is applied.

Native `scancraft_geometry::registration::fit_plane` fits all retained table
hypotheses. A separate NumPy SVD reproduces the fits. Preset diagnostic acceptance
requires >=6 points, RMS <=0.2 RAW and maximum absolute residual <=0.5 RAW.
Library fields ending `_mm` do **not** establish metric units for this model.

## Results

| Threshold | Shape candidates L/R | Cartesian pairs | Stereo accepted | Mutual unique table pairs | Plane RMS / maximum (RAW) | Gate |
|---|---:|---:|---:|---:|---:|---|
| 80 | 44 / 46 | 2024 | 44 | 19 | 19.244006 / 76.274575 | Fail |
| 100 | 35 / 38 | 1330 | 32 | 18 | 0.315736 / 0.830816 | Fail |
| 120 | 32 / 28 | 896 | 25 | 18 | 0.466436 / 1.477708 | Fail |

**Confirmed:** all 101 accepted stereo pairs numerically reproduce using a separate
inverse-Brown, least-squares closest-ray and forward-projection reference. Maximum
position difference is 2.378e-9 RAW; reprojection difference is 2.321e-10 px.
Native and reference plane residuals agree within 1.422e-13 RAW. Numerical agreement
does not verify physical correspondence, calibration accuracy or scale.

**Engineering inference:** the large threshold-80 failure and inspected labeled
stereo overlay demonstrate that mutual uniqueness alone is inadequate evidence
of sticker identity. The tighter other fits do not justify discarding the failed
threshold or relaxing the plane gate. Calibration error, threshold-dependent
centroid bias, wrong correspondences and actual support-surface nonplanarity have
not been independently separated.

## Conditional top-to-plane distances — not a qualified box height

As an additional diagnostic, all original top points from each of the three
stationary line trials in `BOX_FACE_REPEATABILITY.md` were evaluated against each
of the three unconstrained planes. There is no fitted transform between captures.
No owner-measured dimensions enter the computation or selection.

| Support threshold | Signed mean distance, trial 1 / 2 / 3 (RAW) | Top/support normal angle range |
|---|---|---|
| 80 | 34.524373 / 34.609792 / 34.602565 | 5.260..5.336 degrees |
| 100 | 39.359415 / 39.375995 / 39.368904 | 0.495..0.616 degrees |
| 120 | 39.268363 / 39.285593 / 39.278549 | 0.521..0.645 degrees |

Every row has a **failed support plane**. These are conditional signed distances,
not millimeters, a validated height, three qualified scans, or independent scale
validation. The full distance distributions are retained. A no-warp proximity
check between the prior line trial and flood image finds 20 left / 18 right
mutual unique neighbors within 3 px (median 0.216 / 0.256 px; maximum 1.395 /
1.025 px). This supports image consistency but does not prove object immobility
or physical feature identity. Sticker thickness and support contact are also
unmeasured.

## Reproducibility and limitations

Private artifact root: `box-support-plane-20261104` under the developer artifact
directory. It contains the original plan, explicit work/ROI amendments, native
probe source and lockfile, strict release Clippy/build logs, archived binaries,
source/input hashes, all component/pair/error outputs, plane requests/results,
independent verification, labeled images, conditional distances and alignment
checks. The initial adapter dependency manifest/lock was expanded for the ROI
shape adapter before any stereo result; its earlier development binding is
historical. `roi-execution-binding.json` binds the final source before geometry
execution and is the binding checked by `verify.py`. `execution-notes.json` retains the initial work-limit failures and a
NumPy-bool JSON serialization failure. `summarize.py` recovers only the summary
from saved complete outputs; original `analyze_roi.py` and results remain intact.

Run the archived component/stereo probes with the recorded arguments in
`roi-analysis-receipts.json`; `verify.py`, `summarize.py`, `check_alignment.py` and
`display_and_height.py` document the reference computations. Use a separate
output copy when reproducing; do not overwrite the original evidence.

The external milestone manifest `box-support-plane-milestone-20261104/evidence-manifest.json`
binds 90 files / 8,210,478 bytes; SHA256:
`e533bcd1e005f68bf2145227c4e5c2ead2d221fd275ea72ffa161c63b84d158c`.
Raw recordings remain in the prior private capture archive. Nothing here requires
new hardware acquisition, firmware changes, public raw-data uploads or a fresh
full application build; the last 904-test gate applies to unchanged application
source. Private adapters passed formatting, strict release Clippy and release
build checks. Independent review remains outstanding for this evidence.

**Next priority:** establish sticker correspondence using additional image/geometric
consistency and evaluate centroid uncertainty before using marker planes for
metrology or rotation tracking. Then validate calibration scale independently and
expand actual surface coverage. Do not turn these rejected plane hypotheses into
an accepted metric scan or postpone the complete pipeline for unrelated editing
features.
