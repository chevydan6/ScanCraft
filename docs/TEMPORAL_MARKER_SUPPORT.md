# Experimental temporal support for recorded marker matching

## Problem and implementation

The real-board investigation in [rotation coverage limits](ROTATION_COVERAGE_LIMITS.md)
found that individually plausible stereo hypotheses can connect different markers.
The previous requirement for mutual uniqueness discarded the correct alternative
alongside incorrect alternatives, leaving too little reference coverage to track.
Lowering the coverage or residual gates would conceal this problem.

`fuse-recorded-candidates` now accepts an optional top-level request field:

```json
"temporal_marker_support": true
```

Omitting it preserves the original behavior. The option is experimental and does
not identify physical markers, certify geometry, enable relocalization or declare
metric units. It uses the existing `tracking.max_correspondence_model_units` as
the support distance; the verified experiments use 3 unverified model units.

After the first accepted frame, an accepted stereo hypothesis must lie within
that distance of a point in the **last accepted raw-camera marker cloud**. Mutual
uniqueness is then evaluated among supported hypotheses. No points are moved,
averaged or substituted during this step. All stereo errors, unsupported
hypotheses, support distances and uniqueness decisions remain in `markers.ndjson`.
New `temporal_reference` rows identify the reference frame, point count and bound;
`temporal_support` rows record each accepted stereo hypothesis's nearest reference
index, distance and support decision.

The prior advances only after `ScanSession::process_with_tracking_cloud` succeeds
with accepted tracking. Tracking-only frames advance it; rejected or insufficient
frames retain it. Fusion failure aborts publication before advancing the prior.
There is no automatic reset, motion prediction or coordinate-system change.
The original triangulation, ICP residual, bidirectional coverage, motion, voxel
and resource gates still apply. Markers never enter surface fusion.

Implementation: `tracked_candidates.rs::Request`/`run` and
`tracked_candidates/markers.rs::Prior`/`select`/`reconstruct`. It reuses the native
`SpatialIndex`, `Tracker` and `ScanSession`; no dependency was added. The reference
contains at most 512 original points. Additional distance queries use the existing
cancellable exact spatial index and remain inside the existing stereo-attempt
budget. This is independently written integration code motivated by retained
Raptor experiments, not imported upstream algorithm code.

## Evidence for the design choice

The unmodified four-anchor board association initially refused all 20 diagnostic
views because laser-connected components confused the area-based anchor class.
A private segmentation experiment applied horizontal binary openings of 9, 13 and 17
pixels at the same threshold 100. All 120 masks matched independent NumPy erosion
then dilation. Existing four-anchor association and its 10/3-pixel radii were
unchanged. Pattern labels were mapped back to **original shape-accepted centroids**
within 3 pixels, requiring agreement across all three widths. Mask centers never
entered stereo reconstruction or the new temporal support algorithm.

At the original loss frame 70, this diagnostic labels 31 accepted stereo hypotheses
as the same pattern marker and 17 as different markers. The original uniqueness
rule retains only 16. Frame 72 has 31 same-marker and 33 different-marker hypotheses,
but only 11 original unique pairs. Pattern association still fails in the right
camera at frame 80 for all widths; that frame has no label-based verification.
These labels provide image-pattern evidence, not independently certified physical
marker identity. The segmentation aid is private analysis, not a new production
preprocessor, and no known board coordinates or labels are inputs to temporal
support.

## Genuine recorded-data results

All tests below preserve the original camera model, raw observations and numerical
quality gates. No physical dimensions or desired geometry were used to choose
matches, fit scale or accept results.

| Dataset and processing | Original rule | Temporal support |
| --- | ---: | ---: |
| First 101 consecutive board frames, accepted tracking | 70 | 101 |
| Same 101 frames, integrated / tracking-only / rejected | 28 / 42 / 31 | 51 / 50 / 0 |
| Same 101 frames, fused laser candidates | 544 | 1,176 |
| 225 uniformly sampled board views, private marker-only diagnostic | 18 accepted | 39 accepted |
| Four saved stationary yellow-box samples, native accepted tracking |4 |4 |
| Same stationary box samples, fused laser candidates |2,704 |2,698 |

The longer sampled sequence still loses tracking: the temporal experiment has 186
subsequent insufficient-point inputs and no full-rotation recovery. Changing the
fused point count is not itself evidence of better accuracy. The stationary box
pose estimates change slightly when temporal support excludes previously admitted
observations; this is not a dimensional improvement claim.

For the 101-frame native replay, every marker cloud, pose, support decision and
tracking outcome agrees with the independently checked prototype. It processes
all 1,325 original surface candidates; only candidates from accepted nonempty
frames enter fusion. Independent camera mathematics recomputes all 1,325 surface
triangulations within 3.19e-12 model units, and independent transformed voxel means
agree within 1e-8. The 21 files from the three earlier default-mode results remain
byte-for-byte unchanged.

Separate NumPy nearest-neighbor/Kabsch-SVD checks reproduce all 652 baseline and
experimental outcomes across the 101-frame and 225-sample tests, including rejected
and insufficient inputs. Maximum numeric differences are 6.74e-12 for the new
consecutive-frame tracking and 3.05e-10 for the new sampled tracking. Where the
three-width pattern diagnostic supplies labels, no selected temporal pair has
conflicting pattern IDs. Unlabelled pairs remain unverified.

The actual native GUI opens the new 101-frame result, displays all 1,176 candidates,
reports 51 integrated/50 tracking-only frames and shows original camera pair 101.
The inspected view remains sparse fragments. This private-display software-render
check establishes startup/loading/display, not GPU performance, editing, metric
export or a complete recognizable object scan.

## Validation and limitations

Two added regression tests cover ambiguity retention, original coordinates,
distance diagnostics, cancellation, tracking-only prior advancement, rejected and
missing frames, subsequent recovery, source-reference IDs and native bundle
loading. Full source gate: **923 passed, zero failed, one ignored**, formatting,
strict all-target Clippy and optimized build pass. An initial missing bracket and
missing test import failed compilation; both were corrected and failure logs kept.

Private roots `rotation-marker-identity-20261104`,
`temporal-marker-candidates-20261104` and `temporal-marker-integration-20261104`
retain plans, code/binary/input bindings, all failures, full outcomes, numerical
checks and the GUI receipt. Original scans and calibration data remain outside Git.
No firmware, scanner control or calibration coefficient changed in this work.

**Limitations:** an incorrect accepted reference can reinforce incorrect matches.
Fast motion, occlusion, new surface visibility and sparse observations can remove
valid hypotheses. The support distance is in unverified model units and is not a
measured physical motion envelope. This local filter does not search older keyframes
or establish a globally correct pose. Physical marker/laser identity, synchronization,
calibrated scale, dimensional accuracy, full object coverage and end-to-end native
scan/edit/save/reopen/export remain unqualified. Keep the option explicit while
those questions are tested; do not relax existing gates to hide lost tracking.

The sealed evidence manifest binds 1,391 files totaling 549,331,833 bytes; SHA256
`977cb817ddf80c43478040c24fa6425583fe50ae5e7c06406449f1c63f986c61`.
Four-box-sample numerical checks additionally cover 83 accepted marker hypotheses
and all 4,161 surface candidates: camera-coordinate differences <=2.44e-9 model
units, tracking <=1.17e-12 and voxel means <=1.14e-13. CodeRabbit usage lookup
remains unavailable; no substantive or paid review is claimed.
