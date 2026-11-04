# Native board planes and provisional sheet-transfer experiment

Status: **experimental offline evidence; no projector calibration activated and no
validated object scan**. Application source and dependencies remain unchanged from
`ebb77a2954f8563d15e28f2d0b02928da0835cd2`; analysis started at documentation head
`56431057280c3e2740c6c7f3e3c811566f78fce7`. The supplied box dimensions are not fitting,
selection or validation-gate inputs in this experiment.

## Confirmed observations and methods

Two retained stationary board scenes each contain line-off and line-on stereo
recordings. Existing native `inspect-components` fully validates all eight recordings
(80 GREY frames), then measures selected frames. Selected indices are near-off4/5,
near-on5/4, far-off5/5 and far-on5/5; each pair has equal sensorPTS, with host differences
3,6,6 and16 microseconds respectively. These are timestamp associations, not proof
of simultaneous physical exposure or unchanged device geometry across captures.

Both off scenes have exactly four detected anchors in each camera. Existing
`scancraft-calibration::target::associate` succeeds with unchanged settings.
Area-only classification of line-on components fails in near-left and both far
cameras because laser-connected components add apparent anchors. The original
exactly-four-anchor rule is not relaxed. A private exploratory transfer instead
requires mutually unique off/on marker proximity within3pixels and both existing
ellipse RMS gates; unmatched and ambiguous measurements remain recorded.

The declared stability criteria require at least20 small markers per camera,
maximum shift<=1pixel, RMS<=0.5pixel and coverage of at least half the off-marker
extent on both axes. Initial near-left maximum shift1.180451284pixels fails, although
its RMS0.261354604 passes. The displaced marker contains114 saturated pixels; a
second0.833809439pixel shift has29 clipped pixels. The initial failure is preserved.
A subsequent **stricter, data-informed experiment** excludes every marker containing
any saturated pixel in either exposure. No shift/count/coverage threshold changes.
It retains32/33 near markers and32/32 far markers per camera, with32 jointly observed
markers in each pose. Maximum shifts are0.180727/0.219626pixels near and0.166441/
0.168766pixels far. This finite agreement is not independent physical pose proof.

`StereoCalibration::prepare`/`PreparedStereo::triangulate` use the unchanged native
board-fit rig and original source lens domains/limits. The plane inputs are actual
line-on marker pixels; the historical factory-coordinate plane is not reused.
`scancraft-geometry::registration::fit_plane` gives:

| Pose, stricter marker set | Used points | Plane RMS, RAW units | Maximum absolute residual, RAW units |
| --- | ---: | ---: | ---: |
| Near line-on |32|0.066123853|0.149102612|
| Far line-on |27|0.140693763|0.297233780|

Five far pairs fail inverse distortion within the original calibrated domain;
those rejections remain, without enlarging the domain. All original/off/on cases
and their outcomes are retained. Independent NumPy SVD reproduces the native plane
fits numerically; agreement is not physical scale or accuracy validation.

## Stripe observations and all cross-pose hypotheses

Native `trace-stripes` uses background30, peak50, maximum width12, saturation255,
opening radius12 and unchanged100,000-observation cap. It retains1,988/1,837 near/far
observations, while rejecting4,316/3,865 saturated runs. Saturation splits the traces;
only four near portions have20 observations and no far portion reaches20.

The private preparation selects pixels within a3pixel-eroded convex hull of the
accepted marker positions, excluding every known off-marker rectangle enlarged by
3pixels. All exclusions are retained. Native `PreparedCamera::ray` and the existing
linear case of `QuadraticSheet::intersect` project944/703 observed stripe centroids
onto their own marker-derived board planes. The board equation is represented in
that API only to reuse its ray intersection; it is not a fitted projector sheet.
Independent NumPy forward projection has maximum discrepancy2.07e-9/9.90e-10pixels;
plane residuals are below1.14e-13RAW. These are numerical consistency checks only.

After inspecting the image-coordinate distribution, a private exploratory grouping
sorts source x and splits gaps greater than25pixels. It imposes no group count and
fits no object dimensions. Groups qualify with at least20samples and200rows of
span; every group/sample remains recorded. Five groups per pose qualify. This
simple gap rule depends on these particular non-overlapping image-x ranges and is
**not a general detector, line-identity algorithm or production feature**.

The existing native plane fitter evaluates all25 cross-pose group pairings, with
all measured samples used unweighted. Unequal support is reported per pose. Before
looking at box consistency, a provisional selection retains mutual row/column
minimum RMS with margin>=0.25RAW, total RMS<=0.2RAW and each-pose maximum<=0.5RAW:

| Near group / far group | Plane RMS, RAW units |
| --- | ---: |
|2 /0|0.142597246|
|3 /1|0.051947501|
|4 /2|0.166515434|

These selection thresholds were chosen after board-fit inspection, before box
transfer. They are exploratory checks, not qualified uncertainty limits. All25fits
and excluded hypotheses remain available; no physical projector identities inferred.

## Failed transfer to the measured box

A private Rust consistency probe tests every one of the prior709 stereo candidates
for396 box observations against all three provisional planes. It preserves every
distance and accepts only a single candidate/plane combination within0.75RAW.
That gate is an exploratory consistency allowance, not a millimeter accuracy claim.

**Zero candidates pass; no ambiguous survivors.** The closest plane distance is
24.129303209RAW, far beyond the unchanged0.75gate. No best-fitting candidate is
substituted, no threshold widened and no physical box dimensions used to refit
sheets. This experiment does not resolve box stripe identity or qualify reconstruction.

Engineering inference: limited sheet coverage is one lead. The conservative near
marker hull ends before a visible outer stripe; other projected stripes lie outside
that board face. This alone does not establish the cause of transfer failure.
Additional saved board views, physical sheet identity, board-mask validity,
projector-mode/device continuity and the planar-sheet approximation require testing.
Do not assume a common projector origin or extrapolate missing planes from these
three hypotheses. Further board views and independent camera checks precede any
activation or accuracy claim.

## Reproducibility and current review scope

Private artifacts `native-board-sheet-preparation-20261104` and
`yellow-box-board-sheet-test-20261104` retain native inputs/outputs, plans, original
failures, Python preparation sources, private Rust probes and build/Clippy logs.
`prepare_markers.py` reproduces all four original/stricter stability and request
JSON values exactly. Final bindings for input recordings, source, requests and
executables match. Native source modules are unchanged; all private binaries pass
Clippy and release builds. NumPy checks validate plane fitting and ray projection.
No synthetic test or numerical agreement is counted as hardware accuracy.

`native-board-sheet-milestone-20261104/evidence-manifest.json` binds100 regular
files,6,565,378bytes, excluding Cargo targets/caches. SHA256:
`b6444c8f9d1281022d24712a50ade7c4159360542f2e2de7cb4fac2403faaecb`.
Original recordings/models and all private observations remain outside public Git.
The frozen native model still has unverified QR-unit scale and limited training
poses/coverage. No RAPCAL activation, scanner command, firmware/module operation,
new capture, paid review or main merge occurred during this offline investigation.

Independent516 reproduced the preceding396-query/709-candidate experiment, and518
confirmed both5643105 CI runs passed. They do **not** validate this new board/sheet
experiment. The prior independently verified895-test/fmt/Clippy/release gate remains
applicable to unchanged production source. New private evidence requires independent
review; substantive CodeRabbit coverage remains outstanding.


## Wider board coverage and curved-sheet follow-up

**Confirmed, exploratory:** independent assignment519 reproduced the preceding
board preparation, all25 fits and failed planar transfer at87ce1f4. The following
new experiment starts from that revision and is not covered by519. Production
source remains unchanged. The original failed inputs/results are retained.

A source-image inspection identifies an additional near-board rectangle
x1128..1160, y200..850 inside the visible board face but outside the conservative
marker hull. Adding its native detections, with unchanged off-marker exclusions,
adds426 original observations. This assumes the marker-derived plane continues
across that visible face; it does not enlarge camera lens validity. The patch was
chosen from the board image, without using box dimensions or transfer residuals.
This is an image-informed mask experiment, not independent physical qualification.

All1,370 near and703 far samples feed the unchanged native ray/plane intersection.
The unchanged gap/span/count rules yield six near and five far groups. All30
native plane fits are retained. The original selection still admits only2/0,3/1
and4/2. The new near5/far3 fit fails: total RMS0.245556814RAW and far maximum
0.818292948RAW exceed0.2/0.5. No threshold is widened. Repeating the same three
planes against the box would add no evidence, so that gate is not rerun.

### Curvature diagnostic, before box transfer

The existing native model in
[laser_surface.rs](../crates/scancraft-calibration/src/laser_surface.rs),
`QuadraticSheet`, represents X=aY²+bY+cZ+d. A private NumPy least-squares diagnostic
compares this equation against X=bY+cZ+d for every one of the30 group pairs.
Columns are centered/scaled before fitting; coefficients are transformed back.
All points remain unweighted, with no outlier removal or box input. This is private
analysis code using NumPy, not a newly integrated Rust calibration fitter.

Before fitting, the plan declares twofold validation: alternate contiguous100-row
image bands, train on one parity in both poses and test on the other, then reverse.
Full-fit and each held-out fold require each pose's X-residual RMS<=0.2RAW and
maximum<=0.5RAW, full rank, and mutual row/column minimum held-out RMS with margin
>=0.25RAW. Every model, coefficient, fold, sample residual and rejection is saved.
X residual is an upper bound on shortest distance to this graph; it differs from
the earlier orthogonal plane-fit statistic. No physical uncertainty bound follows.

| Near/far pair | Linear held-out RMS_X, RAW | Quadratic held-out RMS_X, RAW | Quadratic worst held-out residual_X, RAW |
| --- | ---: | ---: | ---: |
|2/0|0.160498206|0.047761604|0.162566728|
|3/1|0.053404322|0.053303692|0.209638118|
|4/2|0.173563321|0.053252688|0.180570726|
|5/3|0.253399768|0.060876645|0.230127827|

These four quadratic hypotheses pass all declared residual/margin checks. Linear
3/1 and4/2 pass; linear2/0 fails a per-pose/fold maximum despite the lower aggregate
RMS. Improvement for3/1 is negligible, not evidence of necessary curvature there.
The added5/3 data show a systematic curved residual under the linear model.

**Engineering inference:** curvature explains these observations better than the
straight-sheet approximation. Two-image spatial validation does not validate an
independent board pose, camera calibration, physical units, device-mode continuity
or projector identity. Shared distortion/board-plane errors can mimic curvature.
The inferred sheets remain provisional; no RAPCAL is activated.

### Box consistency: one supported stripe fragment

The four board-selected sheets are frozen before a new box test. A private Rust
probe calls existing `PreparedCamera::ray` and `QuadraticSheet::intersect`, retaining
all results for the previous709 stereo candidates/396 queries. It uses the original
camera domains and depth interval. A candidate must have absolute X residual
<=0.75RAW and a unique native ray intersection; a query survives only when exactly
one candidate/sheet combination passes. This residual gate is conservatively
stricter than a shortest-surface-distance gate with the same numeric limit.
All roots, failures, alternatives and residuals remain recorded.

**201 queries survive,195 have no supported candidate, none have multiple survivors.**
All survivors connect left trace368/right662 to provisional near5/far3. Their
coordinates remain the original stereo triangulations; they are not snapped onto
the sheet. Median/maximum sheet X residuals are0.052112162/0.177723464RAW. The
stereo points and separate ray-sheet points differ by median0.343708677 and maximum
1.199378606RAW, retained as uncertainty evidence rather than suppressed.

This is one narrow stripe fragment, not a recognizable complete object surface or
verified physical correspondence. Missing/weak/out-of-ROI stripes can invalidate
uniqueness among the tested alternatives. No owner dimension is used to select,
scale or fit these points, and no dimensional error, metric PLY, mesh, three-trial
repeatability or complete native-GUI scan is claimed.

### Reproduction and limitations

Private roots `native-board-edge-coverage-20261104` and
`native-board-curvature-diagnostic-20261104` contain preparation/fit/verification
sources, full requests/results, native-probe source/lock, fmt/Clippy/release evidence
and a diagnostic plot. The edge experiment's original exit receipts were not
retained; later recorded native reruns exit0 and reproduce both result files
byte-for-byte. Exact source-index/mask reconciliation confirms426 added samples,
all original near samples and unchanged far inputs. All30 plane fits agree with
NumPy SVD to3.11e-15RAW RMS.

Developer numerical crosschecks reconcile all2,836 candidate/sheet outcomes and
query counts. Native roots satisfy the sheet equation within2.85e-14RAW, project
back within3.22e-13pixels and agree with NumPy polynomial roots within1.71e-13RAW
depth. These checks are not independent reviewer or physical-accuracy validation.
Private Rust fmt, strict Clippy and release build pass. The previously independently
verified895-test/fmt/Clippy/release gate applies to unchanged application source;
no redundant full application gate is run for this documentation-only milestone.

`native-board-curvature-milestone-20261104/evidence-manifest.json` binds36 files,
10,486,104bytes, excluding Cargo targets/caches; the native executable is separately
bound before execution. Manifest SHA256:
`d2299ce54e80e3704e9018266b41de59038036a1f4ddaabd49440a228f7db341`.
These are local private artifacts, not public repository contents. Existing sealed
evidence remains unchanged. No new capture or device operation occurred.

**Next:** obtain additional actual board-sheet coverage and independently held-out
poses, verify camera/physical-scale and sheet identity, then reconstruct broader
object geometry. Do not extrapolate absent sheets from a presumed projector origin,
fit the supplied box dimensions, or treat this201-point fragment as mission success.
