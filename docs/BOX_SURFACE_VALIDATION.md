# Measured-box surface investigation — 2026-11-04

## Outcome and scope

Confirmed: fresh color images show the yellow box on the turntable. Three short
line-mode stereo captures produce 612, 1,030 and 593 diagnostic candidates through
the native recorded pipeline. The native GUI displays the 1,030-point result.
These are sparse fragments and additional unqualified scene hypotheses, not a
complete box or a dimensionally qualified scan.

Application revision: `37b8dd6cb3f5e26eeaa6aeeeca48af34289e8bdb`, containing correction
`8e2826052a8f323e49a8008b9d02ee822807b56c`. No production code, dependency, calibration
coefficient or reconstruction threshold changed in this investigation. Independent
review assignment 528 resolves SCAN-REV-033 only for its opt-in recorded-domain
contracts; it does not qualify physical reconstruction.

Owner-reported dimensions, 79.70 × 59.78 × 39.80 mm, remain held-out truth. They were
not used to fit scale, calibration, a cuboid, orthogonality, correspondence or pixel
selection. Physical unit provenance and measurement uncertainty remain unresolved.
All distances below are **raw reconstruction units**, not established millimeters.

## Capture and state restoration

Private capture plans bind the executed CLI and commands. Existing scoped capture
commands preserve original recordings and timing metadata, validate every saved
frame, and restore seven queried device states, camera formats and WirePlumber.
All four capture wrappers pass those restoration checks. The original kernel
module remains unchanged; there were no firmware writes.

Each stereo arm saves ten frames per camera. Pairs for processing were selected by
equal recorded sensor PTS, not by assuming equal recording indices. Equal PTS is
not independent proof of simultaneous exposure. Initial preserve/flood GREY images
had poor object contrast. An initial interpretation that the box was absent was
incorrect: a fresh three-frame color capture confirms its presence. The earlier
request to reposition it was superseded. The view differs from an older recording;
the cause is unverified, and old pixel windows were not reused on the new view.

A requested gain of 16,000 was rejected by existing bounds; the failed arm and
wrapper exit 1 are retained. A separate trial uses the descriptor maximum 15,000.
The successful preserve/flood arms and failed gain arm must not be represented as
one wholly successful experiment. Subsequent scoped line captures use gains 8,000,
15,000, then 8,000. The control enables the existing line-light mode; this experiment
does not independently identify projector wavelength.

| Arm | Startup rejects L/R | Ongoing rejects L/R | Sequence gaps L/R | Equal-PTS pairs |
| --- | --- | --- | --- | --- |
| Preserve | 84 / 1 | 0 / 0 | 0 / 0 | 9 |
| Flood | 1 / 0 | 0 / 0 | 0 / 0 | 9 |
| Gain 15,000, no added light | 1 / 0 | 0 / 0 | 0 / 0 | 10 |
| Line 8,000 A | 0 / 0 | 0 / 0 | 0 / 0 | 10 |
| Line 15,000 | 1 / 0 | 1 / 1 | 1 / 1 | 9 |
| Line 8,000 B | 101 / 0 | 0 / 0 | 0 / 0 | 9 |

These short captures neither qualify the rejection-rate target nor prove that gain
causes rejection. Startup failures, ongoing failures and gaps remain distinct.

## Frozen native reconstruction

The three line captures use the earlier domain-aware request unchanged except for
actual frame CRC bindings. Camera geometry, four provisional curved sheets,
detection, continuity, triangulation, ambiguity and work limits are unchanged.
Full-image processing uses no surface ROI. Implementation boundaries are
`scancraft-depth/src/laser_pair.rs`, `scancraft-calibration/src/laser_trace.rs` and
`laser_trace/domain.rs`, and `scancraft-workflow/src/recorded_traces/automatic.rs`
(all under `crates/`). The candidate loader is `scancraft-workflow/src/candidate_bundle.rs`.

| Arm | Queries | Candidates | First-query errors | No unique match | Excluded vertices / segments / roots |
| --- | ---: | ---: | ---: | ---: | --- |
| Line 8,000 A | 1,507 | 612 | 131 | 764 | 161 / 161 / 513 |
| Line 15,000 | 2,770 | 1,030 | 417 | 1,323 | 533 / 522 / 617 |
| Line 8,000 B | 1,497 | 593 | 131 | 773 | 162 / 162 / 513 |

Developer NumPy checks independently recompute all 2,235 accepted candidate
triangulations and forward projections using the frozen model. Maximum coordinate
difference is 1.331e-9 raw units; maximum forward-pixel difference is 9.417e-10 pixels.
All query outcomes, source/support indices, exclusions, artifact lengths/CRCs and
selected image payload CRCs reconcile. Numerical agreement does not establish
physical correspondence, calibration accuracy or units.

The real native GUI opens the 1,030-point bundle without scanner descriptors;
its screenshot shows sparse fragments and explicit unverified-unit/domain warnings.
It is not evidence of metric editing, complete-object reconstruction or mesh export.

## Surface consistency: one provisional pass, five failures

Before fitting planes, an image-only plan fixed rectangles inside putative upper
and lower stripe branches in both cameras. The same windows apply to all three
captures. All candidates within each region are retained without outlier deletion.
Surface labels are hypotheses: the region called “front” may combine different
physical faces or incorrect correspondences.

The private Rust probe calls existing `scancraft-geometry/src/registration.rs`
`fit_plane`; a separate NumPy SVD verifies every residual within 1e-8 raw units.
Predeclared diagnostic gates require at least 40 points, at least two sheet IDs,
RMS ≤0.2 and maximum residual ≤0.5 raw units. These are provisional consistency
gates, not the millimeter acceptance specification.

| Arm / putative surface | Points / sheets | RMS raw | Maximum raw | Diagnostic gate |
| --- | --- | ---: | ---: | --- |
| 8,000 A / top | 39 / 1 | 0.029826 | 0.081847 | Fail: support |
| 8,000 A / front | 270 / 3 | 0.880953 | 1.631074 | Fail: residuals |
| 15,000 / top | 199 / 3 | 0.138950 | 0.396761 | Pass |
| 15,000 / front | 279 / 3 | 0.878526 | 1.782089 | Fail: residuals |
| 8,000 B / top | 25 / 1 | 0.029746 | 0.062274 | Fail: support |
| 8,000 B / front | 272 / 3 | 0.885499 | 1.680114 | Fail: residuals |

The high-gain plane-angle diagnostic is 89.238966 degrees, but the front plane
fails its gate, so this is not angular qualification. Low-gain angles likewise
lack adequate top support. High-gain front residuals differ by sheet: sheet means
approximately −0.339, +1.252 and −0.333 raw units; sheet 0 also has large within-sheet
spread (standard deviation 0.867). These observations do not isolate calibration
as the cause. Surface assignment, correspondence and the provisional model must
be distinguished before changing calibration. No flattening or right-angle
constraint was applied.

## Earlier image-only corner and silhouette failures

On the older frozen image pair, three visually selected silhouette seeds were
chosen before numeric results. Existing `scancraft-image/src/corners.rs`
`refine_corner` and `scancraft-calibration/src/stereo.rs` triangulation were used
without new calibration. Raw seed distances were 81.717845 and 38.614149 raw units.
Radius-5 refinement yields only a 38.800862 raw-unit height; both far-corner
refinements exceed the permitted shift. Radius-10 refinement yields no complete
edge because of convergence/shift failures. These are measurement hypotheses,
not validated dimensions; no favorable value is selected against owner truth.

A second frozen image-gradient plan samples silhouette interiors, with native
camera rays and normalized total-least-squares edge fits. Predeclared trims 0–3,
1.5-pixel residual, minimum 30 points/70% retention, and original contrast gates
are preserved. One required edge has zero support in both cameras; bottom-edge
support fails after trimming. Only the untrimmed derived height exists
(40.464808 raw units). No accepted trimmed height or complete width results.
Five accepted corner pairs and five derived-edge corner cases pass separate NumPy
inverse-distortion/closest-ray numerical checks. Rounded corners, weak contrast
and unverified physical feature identity prevent metrology claims.

## Evidence and reproduction

Private evidence is retained outside Git under developer state `artifacts/`:

- `native-box-corner-diagnostic-20261104`: frozen corner/edge plans, requests,
  private Rust probes, native rays, original failures, execution bindings and results.
- `box-edge-visibility-20261104`, `box-edge-gain15000-20261104`,
  `box-current-color-20261104`: raw recordings, capture plans, device-state receipts,
  replay results and the color correction evidence.
- `box-current-lines-20261104`: three raw stereo captures, selected-pair metadata,
  frozen requests, matching plans/results, overlays and native GUI receipt/image.
- `box-surface-consistency-20261104`: exact image rectangles in `plan.json`, selected
  original candidate indices in `request.json`, Rust probe, `evaluate.py`, native
  results, per-sheet residuals, source/binary bindings and execution receipts.
- `box-visibility-validation-20261104`: `verify.py`, `verification.json`, execution
  summary and unavailable CodeRabbit usage response.

Manifest `box-surface-evidence-milestone-20261104/evidence-manifest.json` binds
524 files / 400,718,594 bytes, SHA256
`978c6ec4da60bc661c2a532d1a796319f6f65557b323153e8a4ac4e3ea1bf82b`.
Disposable Cargo target/cache directories are excluded; retained executables and
source/input bindings accompany the probes. Original referenced recordings and
model inputs are separately hash-bound in the plans. Hashes prove consistency,
not historical execution time or physical truth. Private images and models are
not published; independent reproduction requires authorized access to that evidence.

To reproduce offline, verify the manifest and external bindings, rebuild private
probes against the exact application source with isolated target directories,
replay the selected equal-PTS pairs using recorded commands, run surface evaluation
and master verification, and compare all outcomes including failures. Hardware
recapture is a separate experiment and must not overwrite these original roots.
Private probes pass formatting, Clippy and release builds. The unchanged application
revision has 904 passing tests, zero failures, one ignored, plus formatting, strict
Clippy and release success; assignment 528 independently confirms that gate.
CodeRabbit quota/billing lookup remains unavailable, not a completed review.

## Immediate priority

Keep the measured object stationary while resolving physical face assignment and
correspondence versus model error. Obtain adequate independent calibration/scale
evidence; do not fit to held-out box dimensions. Then demonstrate three genuine
measured reconstructions and native save/reopen/edit/export. Increased candidate
counts and a single flat patch are useful diagnostics, not completion of those gates.
