# Scoped line capture with explicit gain

## Confirmed implementation

`scancraft-cli stereo-line-capture FRAME_COUNT NEW_DIRECTORY [--fps FPS] [--gain RAW_GAIN]`
now exposes the existing combined gain/line leases. Without `--gain`, behavior remains
unchanged, including default 50 FPS. Both flag orders are accepted; duplicate,
unknown, missing and malformed flags are rejected. Raw gain is not a physical gain
unit, laser energy setting or projector-mode selection.

Sources: `crates/scancraft-cli/src/capture_options.rs::parse`, the command dispatch in
`main.rs`, and `stereo_probe.rs::record_line_gain`/`record_trial`. The wrapper passes
`Some(gain)` with `IlluminationTrial::Line` to the existing capture implementation.
`scancraft-device::ir_gain::ScopedIrGain` and `control::line_trial::ScopedLine`
retain bounds checking, original-state restoration and readbacks. Recording formats,
reconstruction algorithms, dependencies and device-control implementations are unchanged.
The prior CLI exposed line capture and gain capture separately; this small addition
allows useful surface observations with both existing controls enabled.

## Actual hardware observation, 2026-11-04

A yellow box was placed on the marker turntable. The owner supplied exterior dimensions
79.70 × 59.78 × 39.80, interpreted as millimeters from the ongoing measurement context.
Instrument zero, cooling and uncertainty remain unconfirmed. These are held-out
comparison values; no model, correspondence, endpoint, scale or acceptance threshold
was fitted to those dimensions. The box is visible in both scanning cameras.

The combined trial saved 10 GREY frames per camera at 30 FPS, raw gain 8000,
with supported property 151 enabled. It used the original installed driver, no
module/firmware modification, normal scanner locks, an idle-media check and a
scoped WirePlumber pause/restoration. Both native full recording replays passed;
three subsequent color frames also passed full validation/decoding. Visible stripes
cross the box, but physical wavelength was not measured. Color capture occurred
after the line lease restored, so it is not a color observation of laser emission.

**Startup rejected buffers: left 0, right 97. Ongoing rejected buffers: 0/0.**
These are buffer counts, not independently measured lost exposures. This short
baseline-driver trial does not qualify acquisition reliability and does not negate
earlier retained failures. All five saved formats were restored, seven before/after
native-state comparisons matched, and WirePlumber returned to its original active
state. Gain readbacks report 1000 → 8000 → 1000 on both cameras; line state reports
0 → 1 → 0. Collector and replay commands exited successfully.

Selected left frame 5 and right frame 3 share sensor PTS and differ by 25 microseconds
in host timestamps. Equal preview indices 5/5 are not a matched pair here. Equal PTS
does not establish physical exposure synchronization.

## Offline reconstruction evidence and limitations

A private Rust probe calls existing `StereoRectification::build`,
`StereoImageMaps::build/apply`, `matching::match_rectified` and
`StereoCalibration::prepare`/`PreparedStereo::triangulate`. It uses the unchanged
native board fit discussed in [caliper evidence](CALIPER_SCALE_EVIDENCE.md), preserving
both source lens validity domains. No factory fit, synthetic surface, scale correction,
active calibration record or asserted metric export was substituted.

Processing plans and source/input/binary hashes were recorded before each run.
The first centered ideal grid clipped part of the box at the output boundary.
A second pass translates the virtual image principal point by (-600,-200) pixels,
leaving physical camera models unchanged, and changes regular sampling from 4 to
6 pixels to bound seed count. A third pass uses 13×13 rather than 7×7 patches to
test whether more image context distinguishes stripe bends. It retains all other
thresholds and source geometry. No acceptance gate was relaxed.

| Diagnostic | Seeds | Accepted points | Main rejection counts |
| --- | ---: | ---: | --- |
| Initial virtual grid, 7×7 | 9,515 | 8 | 8,732 invalid/low texture; 434 low correlation; 302 ambiguous |
| Shifted grid, 7×7 | 6,996 | 5 | 6,631 invalid/low texture; 212 low correlation; 137 ambiguous |
| Shifted grid, 13×13 | 7,063 | 5 | 6,412 invalid/low texture; 449 low correlation; 173 ambiguous |

All rejection reasons are retained privately. Accepted points are diagnostic RAW
model units; small ray gaps/reprojection errors after rectification do not prove
correct stripe identity. Rectangular source ROIs may include background. These
sparse observations are **not a recognizable or dimensionally validated box scan**.
No physical size/error, mesh accuracy, three-trial repeatability or completed native
GUI workflow is claimed. Immediate engineering priority is reliable correspondence
and useful surface coverage, before measuring the supplied dimensions.

## Verification and reproducibility

The final source gate passes formatting, strict all-target Clippy, release build,
and **895 tests, 0 failed, 1 ignored**. New regressions cover default/combined flags,
option order, incomplete/duplicate/unknown/overflow input and actual CLI refusal
before capture for malformed options or unsupported FPS. Initial Clippy failure
for `chunks_exact` was corrected with `as_chunks`, without suppressing the lint.
The private probes separately pass Clippy and release build after a retained
collapsible-if diagnostic was corrected; they are not shipped production features.

Private artifact roots: `measured-yellow-box-20261104`,
`measured-yellow-box-line-20261104`, `measured-yellow-box-gain8000-20261104`,
`measured-yellow-box-line-gain8000-20261104`, and
`yellow-box-native-reconstruction-20261104`. The developer artifact
`line-gain-integration-20261104` contains gate logs, `verify.py`, `verification.json`,
and `private-evidence-manifest.json`. The manifest binds 427 regular files totaling
234,824,088 bytes; Cargo targets/caches are excluded and symlinks listed separately.
Manifest SHA256: `bad53693e72916a3b64c8653107fd7672ffd25737518a238a353797f4d2f019f`.
Reconciliation checks command outcomes, native replay receipts, source hashes,
restoration, pair identity and diagnostic counts; it is not an independent raw parser
or physical validation. Original images, recordings, models, binaries and owner
measurement notes remain outside public Git.

Publication parent is `876c3af5e49214d466534cbcca25c24de67c7956`; exact implementation
commit is recorded in Git history. CodeRabbit usage lookup returned unavailable
for included capacity/billing/credits, so no review was uploaded or paid usage
requested. Substantive CodeRabbit review and independent examination of this new
change remain outstanding; tests do not replace them. No main merge is authorized
by these results. Independent reviewer records remain untouched.

## Follow-up: full-trace correspondence ambiguity

A subsequent offline diagnostic uses the existing native `trace-stripes` detector
and continuity implementation on the same matched left5/right3 recordings. Initial
contrast gates5/10 exceeded the unchanged100,000 observation cap; the failure is
retained. A second, explicitly recorded12/24 contrast plan reduces noise admissions
rather than increasing the cap. It retains6,309/7,022 whole-image observations.
Within the image-defined box rectangles, three trace portions per camera each
contain at least20 observations. This selection does not assign projector identities.

A separate private Rust probe calls `epipolar_trace::PreparedTrace::intersections`
for every left observation against every retained right trace, then the existing
stereo triangulator for every intersection. Source camera models and original
triangulation limits are unchanged. All alternatives/errors are retained; no
winner is selected by depth, box dimensions or a desired surface shape.

**396 observations yield709 geometrically accepted candidates:** eight have none,
67 have one and321 have two. There are no trace/triangulation errors in these
selected inputs. A unique candidate within a cropped trace set still does not
prove physical identity, since weak/missing traces are not included. No candidate
collection is presented as a qualified object cloud. This experiment demonstrates
why triangulation checks alone are insufficient to choose between repeated stripes;
it does not prove that every previous patch match was wrong.

Private artifact `yellow-box-trace-correspondence-20261104` retains plans, both
native detector outputs, the initial failure, request/probe source, build/Clippy
logs, all candidate outcomes and verification. Its manifest binds20 files,
4,987,339 bytes, excluding Cargo targets/caches. SHA256:
`f1b1772b2363e45421d147718f43340cfdc5cda6b89249c2dce2cca992925d1c`.
The initial wrapper aborted before writing command-event receipts; its stderr is
preserved, and no completed initial detector output is claimed. The later detector
invocations and pre-execution intersection bindings are retained and verified.

Next investigation: determine whether retained calibration-board recordings can
establish projector-plane/stripe identity with the frozen native camera model.
Existing stationary board line-on and line-off captures are available, but one
historical line-on association fails the exactly-four-anchor requirement. Do not
reuse a line-off pose without testing inter-capture stability, reuse the historical
factory-coordinate plane as native geometry, or fit the known box to choose sheets.
No new projector-plane fit or physical accuracy result has yet been established.

Source implementation remains `ebb77a2954f8563d15e28f2d0b02928da0835cd2`, published
as draft PR37. Both exact-head CI runs41787075489 (PR) and41786941673 (push) passed.
This follow-up changes documentation only; it does not imply completed independent
or CodeRabbit review.
