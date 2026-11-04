# Retained-reference recovery experiment

## Scope and status

Update: the [native session core](SESSION_REFERENCE_RECOVERY.md) now implements
this policy and reproduces the recovery experiment. The [recorded workflow/GUI
integration](RECORDED_REFERENCE_RECOVERY.md) also supports it explicitly. The
experiment and baseline details below describe PR47.

**Confirmed:** a private Rust adapter recovers tracking against the original
reference when the marked face returns in the existing recorded-board sequence.
**Historical PR47 baseline:** the recorded workflow retained only its latest
accepted marker reference. This report supplies experiment evidence
and requirements for native integration; it does not announce a working recovery
feature, complete object scan or general global registration.

Production baseline: `f75685cfa9b5b267821a83a1fb25791fb58a879b` / PR46, independently
verified in reviewer assignment558, including 925 passing tests and completed CI.
Relevant existing code: `scancraft-geometry::tracking::{Tracker, TrackingUpdate}`,
`registration::{icp_with, rigid_fit}`, `session::ScanSession`, `fusion::VoxelFusion`,
and `scancraft-workflow::tracked_candidates` / `tracked_candidates::markers`.
No production source, dependency, physical calibration or hardware changed here.

## Experiment fixed before execution

Use all 225 previously selected original indices 0,4,...896 and their unchanged
geometrically accepted stereo hypotheses. Preserve the same raw centroids, supplied
camera model, correspondence distance, RMS, overlap, motion and convergence gates.
Neither board-pattern labels nor physical dimensions enter tracking.

Compare the existing temporal-support strategy with this bounded proposal:

1. Keep a clone of the accepted native tracker and original raw marker positions
   at the initial anchor and every eighth accepted update, including tracking-only.
2. Retain at most 16 snapshots: the anchor plus the latest 15 periodic references.
3. Attempt ordinary tracking first. Only on failure, select hypotheses against each
   retained reference using the existing distance/support/mutual-uniqueness rules,
   then run a clone of that reference's native tracker with unchanged gates.
4. Evaluate every retained reference. If several trials pass, require their world
   transforms to agree within the existing RMS allowance when applied to the union
   of their selected current points. Otherwise preserve failure. If consistent,
   select the oldest passing reference deterministically.
5. Continue from the chosen tracker, which already carries the reference's transform
   into the original scan coordinates. Never call reset or create a new world origin.

The cadence and capacity are fixed experimental choices, not tuned optima or
validated production defaults. This searches existing local alignment basins; it
does not infer arbitrary large rotations or reconstruct the unobserved interval.
The existing motion gate applies between current observations and the selected
stored reference, not to an invented continuous path through the lost interval.

## Observed results

| Result over all225 sampled frames | Existing prior only | Retained references |
| --- | ---: | ---: |
| Accepted tracking updates | 39 | 44 |
| Insufficient-observation outcomes | 186 | 181 |
| Surface-integrated frames | 26 | 31 |
| Tracking-only accepted frames | 13 | 13 |
| Integrated original surface observations | 6,357 | 6,471 |
| Fused candidates | 5,785 | 5,884 |
| Last accepted/integrated original frame | 152 | 896 |

There is one recovery event: **original frame880 against reference0**, with19
selected markers, RMS0.0530437255 in supplied model units, source coverage1.0 and
target coverage19/23=0.8260869565. The remaining four sampled frames track locally.
Only one recovery trial passes, so pose-conflict arbitration is **not exercised by
this recording**. At most six references are retained; cache eviction is likewise
not exercised. All181 intervening insufficient outcomes remain failures; no pose
interpolation, missing surface fabrication or complete-rotation claim is made.

The private fusion adapter feeds unchanged genuine laser candidates to the native
`VoxelFusion` at the existing0.25-model-unit voxel setting. Markers are never surface
points. Both strategies see all277,704 source candidates; only accepted frames
integrate. Recovery contributes114 additional surface observations. The baseline
fused positions exactly match the prior native application result. This adapter
is not yet an atomic `ScanSession` recovery implementation or GUI workflow.

## Numerical and image-pattern checks

Separate NumPy nearest-neighbor/Kabsch-SVD calculations check **all1,360 local and
retained-reference attempts**, including support graphs, rejection reasons, poses,
reference history and selection decisions. Maximum tracking difference is
1.180e-10; support-distance difference is1.137e-13. The original prior-only results
match the previously verified prototype exactly. Independent surface-only voxel
means agree within1.137e-13 model units for both configurations.

A separate image diagnostic checks frames0/880/884/888/892/896 using all three
previously specified horizontal opening widths9/13/17. All36 binary masks match
independent NumPy erosion/dilation; homography residual recomputation differs by
at most2.274e-13 pixels. Pattern labels are transferred only to original raw
centroids with unanimous three-width agreement and unique labels. Opened-mask
centers never enter geometry or tracking.

Across the five recovered frames,25 selected stereo pairs have matching pattern
labels and59 are unlabelled. **All selected pairs at the first recovery frame880
remain unlabelled**, as do frames884/892; those identities are not independently
verified. No labelled selected pair has conflicting IDs, but missing labels must
not be counted as positive validation.

At888,14 uniquely labelled stereo points share pattern IDs with the original
reference; transforming them with the tracked world pose gives RMS0.07968 and
maximum0.25067 model units. At896,15 common IDs give RMS0.10766 and maximum0.29428.
These include available labelled stereo hypotheses beyond the tracker's selected
subset. A separate correspondence-based rigid fit gives RMS0.06009/0.06974;
its transform differs from the tracked pose by0.000454/0.000626 radians and
0.08511/0.18457 translation model units. This supports consistency of the returning
pattern under the supplied model. It neither proves physical dimensions nor certifies
the earlier unlabelled recovery event.

## Native integration requirements

**Engineering recommendation:** reuse the existing tracker/fusion mathematics and
introduce a bounded, session-owned reference bank, rather than accepting arbitrary
external world transforms. Expose immutable reference information for marker
selection and perform trials without consuming a frame or mutating fusion/history.
Prepare original stereo hypotheses once; separate reference trials must not hide
or double-count physical stereo attempts.

Selection, tracker advancement, reference insertion/eviction, fusion and history
must commit together only after successful fusion. Cancellation, publication/fusion
failure, incompatible reference IDs and ambiguous passing poses must retain the
previous complete state. Check monotonic current frame IDs against the live session,
not just an older snapshot. Keep the anchor convention explicit and retain original
failure rows. An opt-in request must preserve byte-identical legacy behavior when
disabled and journal trial outcomes and chosen reference IDs.

Required regressions before promotion: contradictory passing references, no passing
reference, wrong/periodic marker configurations, retention/eviction capacity,
tracking-only cadence, monotonic IDs across old references, cancelled trials,
failed-fusion rollback and persistent-result reopening. Actual native replay must
reproduce this experiment before claiming integration. Real moving-object identity,
complete coverage, calibration/units, three measured trials and edit/save/export
qualification remain outstanding. This experiment does not establish those gates.

Private helper formatting/strict Clippy/optimized builds and numerical verifiers
completed. No production source changed, so the independently verified925-test
baseline remains applicable; it is not a newly executed recovery test suite.
CodeRabbit usage lookup failed; no substantive CLI review or paid usage is claimed.
Publication bindings are recorded with the milestone.
Private evidence root: `retained-reference-recovery-20261104`; external manifest:
`retained-reference-recovery-publication-20261104`. Original captures, board target
data, images and derived coordinates remain outside public Git.
