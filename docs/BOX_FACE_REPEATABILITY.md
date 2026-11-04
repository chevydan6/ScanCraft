# Box face assignment and fresh surface repeatability

Date: 2026-11-04. Application source remains unchanged from
`37b8dd6cb3f5e26eeaa6aeeeca48af34289e8bdb`; this investigation starts at documentation
head `f6397039c7e30127ef65d86e601e87cab8835c68`. These are genuine Raptor observations
and diagnostic geometry, not a qualified complete scan.

## Correction to the previous surface interpretation

The earlier [surface experiment](BOX_SURFACE_VALIDATION.md) selected lower stripe
groups 0, 1 and 2 as one putative “front” face. That grouping mixed faces. Native
extraction of all ten saved ambient frames per camera, followed by a display-only
mean and 9×9 box smoothing, reveals the box's vertical edge. Overlaying the
original candidate pixels without a warp shows group 0 on the left face and groups
1/2 on the right. Original pixel values, selected points and failed fits are retained.
Averaged/smoothed images are used for visual interpretation, not triangulation.

Native `component_diagnostics::run` / image component and shape routines compare
ambient and line images using threshold 120, area 5–1,000 and ellipse RMS ≤0.5 pixels.
All mutual unique neighbors within three pixels are retained, without fitting a
warp: 25 left / 22 right pairs have median separations 0.08347 / 0.10738 pixels,
maximum 1.28633 / 0.64928. Saturated marker centroids can shift. This supports image
alignment, not physical marker identity or independent proof of object immobility.

An exploratory comparison retains all 18 pairwise plane cases and 18 single-stripe
line cases, including missing-support failures. Using the original lower groups
1/2 alone gives RMS 0.137100 / 0.108317 / 0.130156 and maximum 0.398819 / 0.296711 /
0.445669 raw units in the original three captures. Group-0 combinations fail the
prior 0.2/0.5 residual gates. This analysis followed failure inspection; its results
are not pristine held-out qualification. Image evidence provides the face-label
correction, not a preference for whichever numerical subset fits best.

The earlier approximately 0.88 raw-unit mixed-face RMS must not be interpreted as
a measured planar calibration error. Calibration may still be inaccurate; the
mixed-face experiment cannot isolate that question.

## Three new acquisitions with a frozen plan

Before collecting new data, the plan keeps all original processing parameters,
geometry, four sheets, raw-image region rectangles and numerical gates. Only face
grouping changes: top uses its original three groups, right face uses the original
lower groups 1/2, and lower group 0 is reported separately as a single left stripe.
All full-image candidates and failures remain in the native output; points are not
flattened, snapped, culled from the full result or fitted to owner dimensions.

An ambient gain-15,000 capture is followed by three separate scoped line-mode
startups, each ten frames per camera, gain 15,000 and requested 30 FPS. The same
stationary object and scanner pose are used. The baseline line/flood properties
must be off before capture. Existing native leases, consumer checks, saved formats,
seven GET-state comparisons and media-service restoration are retained. All 80
saved frames pass native replay; all reported states/formats and WirePlumber are
restored. No kernel-module or firmware change occurs.

Left index 5 is paired with its unique equal-PTS right counterpart within the
unchanged 1,000-microsecond host window. Original frame CRCs bind each request.
Equal PTS does not establish simultaneous exposure.

| Capture | Startup rejects L/R | Ongoing rejects L/R | Sequence gaps L/R | Equal-PTS pairs |
| --- | --- | --- | --- | --- |
| Ambient | 83 / 0 | 0 / 0 | 0 / 0 | 8 |
| Trial 1 | 1 / 1 | 0 / 0 | 0 / 0 | 10 |
| Trial 2 | 0 / 95 | 0 / 0 | 0 / 0 | 8 |
| Trial 3 | 1 / 106 | 0 / 0 | 0 / 0 | 9 |

Short-capture zero ongoing rejects is not sustained transport qualification. Startup
rejections remain substantial. Fresh ambient/first-trial image comparison has 24/23
mutual marker neighbors, median 0.09460/0.11109 pixels and maximum 1.22832/1.10377.
The fresh unwarped overlay again supports the two-face interpretation.

## Results and numerical checks

The existing native full-image pipeline produces 1,051 / 1,046 / 1,059 candidates,
377 / 460 / 327 query errors and 1,344 / 1,332 / 1,233 queries without a unique match.
These counts include unqualified scene hypotheses, not only the box.

The predeclared surface gates remain ≥40 points, ≥2 distinct sheets, RMS ≤0.2 and
maximum residual ≤0.5 **raw units**. They are not millimeter acceptance gates.
The private probe calls existing `scancraft_geometry::registration::fit_plane`;
independent NumPy SVD reproduces every residual within 1e-8 raw units.

| Trial / surface | Points / sheets | RMS raw | Maximum raw | Diagnostic gate |
| --- | --- | ---: | ---: | --- |
| 1 / top | 192 / 3 | 0.135642 | 0.420134 | Pass |
| 1 / right face | 152 / 2 | 0.115326 | 0.346545 | Pass |
| 1 / left stripe | 127 / 1 | 0.029967 | 0.107202 | Fail: coverage |
| 2 / top | 201 / 3 | 0.139488 | 0.386533 | Pass |
| 2 / right face | 152 / 2 | 0.111361 | 0.303886 | Pass |
| 2 / left stripe | 127 / 1 | 0.030523 | 0.093501 | Fail: coverage |
| 3 / top | 200 / 3 | 0.147610 | 0.428685 | Pass |
| 3 / right face | 150 / 2 | 0.117285 | 0.366012 | Pass |
| 3 / left stripe | 127 / 1 | 0.030223 | 0.079961 | Fail: coverage |

Unconstrained acute top/right-plane angles are 89.651946°, 89.328872° and 89.394782°.
No independently measured angle/uncertainty exists; these are diagnostics, not
angular accuracy claims. No right-angle or cuboid constraint was imposed.

A separately declared repeatability calculation compares all unique sheet/left-image-
row keys common to the three captures, without rigid alignment or outlier removal.
For top/right/left, respectively, 191/150/127 keys are common. Excluded noncommon
counts are 1/10/9 for the top, 2/2/0 for the right, and zero for the left. Pooled RMS
Euclidean distances to each key's three-trial mean are 0.169824 / 0.114786 / 0.069733
raw units; maximum distances are 0.591395 / 0.545344 / 0.211464. Corresponding left-x
ranges reach 1.176923 / 0.874554 / 0.534329 pixels. Fixed image rows are a stationary-
scene proxy, not verified correspondence to the same material point. These results
therefore do not qualify dimensional repeatability or the mission's millimeter gate.

Independent inverse-distortion/closest-ray/forward-projection formulas check all
3,156 native candidates, source/support selection and artifact CRCs/lengths. Maximum
coordinate difference is 1.334e-9 raw units, forward-pixel difference 9.457e-10 pixels.
This verifies numerical implementation consistency, not physical accuracy. The
native GUI opens trial 1's full 1,051-point bundle, with explicit unverified-unit/
identity warnings and no camera descriptors. The inspected screenshot remains
sparse fragments, not a complete object or verified editing/export workflow.

## Historical corner provenance correction (review assignment 530)

Accepted: the old corner `execution-binding.json` names source hash
`c25b74ff712fb483d396700c05fb3811eab6760b73a228afcf1472dce7080f89`, whereas the
retained `probe/src/main.rs` hashes to
`a26ce8459345bf470b145da4fccd916542da004ebc0e4c972ad662a2f6ba5d77`.
The historical source transition has not been established. It must not be described
as formatting-only or treated as a valid original execution binding. Original
source, result and inconsistent receipt are preserved.

A new isolated frozen-dependency Clippy/release build uses the retained source and
records exact source, manifest, lock, request, image and executable hashes before
execution. The fresh execution exits 0 and reproduces the original corner result
byte-for-byte; all bound inputs remain unchanged afterward. This establishes
current-source reproducibility, not historical identity or chronology. Independent
assignment 530 had separately reproduced those results and reported the discrepancy.
No independent finding is closed or edited by the developer.

## Evidence and next work

Three private roots preserve scripts, original inputs, receipts, failures, source/
executable bindings, raw recordings, native bundles, plots and GUI evidence:
`box-face-assignment-20261104`, `box-face-repeatability-20261104`, and
`corner-provenance-reconciliation-20261104`. The new corner executable is archived;
other native executables are bound to prior retained artifacts. Disposable Cargo
targets/caches are excluded. Manifest
`box-face-repeatability-milestone-20261104/evidence-manifest.json` binds 467 files /
519,799,426 bytes, SHA256
`629169018b3e46b4a384bc8b994ebf03a2ebccb7752a01cc7a22efb577887762`.
Private imagery/models remain outside Git.

Reproduce by checking the manifest/external bindings, replaying the four saved
stereo recordings per camera, running `process.py`, `verify.py`, `ambient.py`,
`alignment.py` and `repeatability.py` into separate fresh output directories, and
checking the native plane outputs against their SVD residuals. Source paths/output
roots must be redirected; never rerun scripts over immutable original evidence.
The preceding assignment root retains all pairwise failed hypotheses. The corner
reconciliation root supplies a separate fresh execution receipt and exact inputs.

The unchanged production source retains the recent 904-test/fmt/strict-Clippy/release
pass, independently verified in assignment 528; assignment 530 reproduces the prior
capture diagnostics. This follow-up requires independent review. CodeRabbit's last
quota lookup remains unavailable and the PR is draft-skipped; no substantive
CodeRabbit pass is claimed. No repeated full Rust gate is needed for prose-only
changes; the native probes and new real-data checks above are the relevant new tests.

Next: establish corresponding physical edges and measurement uncertainty using the
now-visible stationary object, and extend independently supported stripe coverage
where needed. Physical scale, complete geometry, three complete measured scans,
tracking/fusion and native edit/save/reopen/export qualification remain outstanding.
The owner's 79.70 × 59.78 × 39.80 mm values remain independent truth, never fit inputs.
