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
