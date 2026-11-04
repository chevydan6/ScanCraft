# Stationary marker consistency and native tracking

**Experimental real-data evidence; not rotation or physical-accuracy qualification.**
Application source remains `56563c1039a8b27c88e08a7e897132c093936461`
(documentation parent `4ba4314d24f528df00a63a802471abc74f5123d4`).

## What this resolves

`BOX_SUPPORT_PLANE.md` showed that a unique stereo hypothesis can still be wrong.
This experiment evaluates a proposed cross-threshold consistency filter and the
**existing** `scancraft_geometry::tracking::Tracker`. It does not add a production
filter, change calibration or substitute marker points for object geometry.

**Confirmed:** all eight unique equal-PTS pairs within 1000 us from the saved flood
capture were processed, left indices2..9/right0..7, rather than selecting successful
frames. These are a short stationary sequence, not independent exposure-sync proof.
Original PNGs and full-recording provenance are bound and payload CRCs checked.
Native component, ellipse-shape and stereo methods/limits are unchanged from the
support-plane experiment, including shape evaluation in the original y>=550 ROI.

## Proposed consistency check and its failure

The private `run.py` specifies thresholds80/100/120 and a3px image-association
radius before processing the other seven pairs. It associates observations only
when proximity is mutually unique across all three threshold combinations in each
camera. It intersects the **complete** stereo acceptance graphs through those
tracks and then requires a mutually unique surviving stereo partner. Competing
edges, missing observations and rejections remain in the outputs. Original
triangulated coordinates from each threshold are retained separately: no averaging,
plane-residual trimming, scale fit or owner-dimension input.

This is exploratory follow-up to a known failure, not an untouched held-out test.

| Pair index | Left/right frame indices | Consensus table pairs | Plane RMS at80 /100 /120 (RAW) |
|---|---|---:|---|
| 0 | 2 /0 | 16 | .4922 /.4717 /.4961 |
| 1 | 3 /1 | 14 | .3442 /.3392 /.3095 |
| 2 | 4 /2 | 15 | 16.9320 /16.9339 /16.9279 |
| 3 | 5 /3 | 15 | .3381 /.3183 /.2825 |
| 4 | 6 /4 | 14 | .4750 /.4616 /.4624 |
| 5 | 7 /5 | 17 | .4757 /.4579 /.4785 |
| 6 | 8 /6 | 14 | .3329 /.3104 /.3232 |
| 7 | 9 /7 | 15 | .3485 /.3450 /.3452 |

**Confirmed:** every plane fails the existing0.2RAW RMS gate. The earlier bad
threshold80 hypothesis disappears, but pair2 retains a different inconsistent
stereo hypothesis across **all three** thresholds. Therefore threshold persistence
alone is insufficient to establish physical identity. No threshold is promoted
because its result happens to resemble the measured box.

## Existing native tracker

A private Rust adapter calls the existing `Tracker::track` with three consensus
sequences and three baseline mutually-unique sequences. Each has all eight frames.
Configuration, fixed before tracking: point-to-point ICP, maximum correspondence
3RAW, RMS0.25RAW, source **and distinct target** overlap>=0.7, translation step<=2RAW,
rotation step<=0.02rad,30iterations, convergence tolerance1e-8RAW. The API's `_mm`
field names do not establish metric units here.

All six sequences establish an initial anchor and accept their seven later updates.
The anchor's zero residual is an initialization convention, not a measured error.

| Sequence | Largest later alignment RMS (RAW) | Largest accumulated translation norm (RAW) | Largest accumulated rotation (degrees) |
|---|---:|---:|---:|
| Consensus80 | .190581 | .268408 | .031177 |
| Consensus100 | .124306 | .264011 | .036175 |
| Consensus120 | .140559 | .248283 | .031179 |
| Baseline80 | .191965 | .196715 | .027926 |
| Baseline100 | .160048 | .321150 | .044671 |
| Baseline120 | .128031 | .284497 | .036144 |

These are fitted stationary motions, not calibrated pose errors. The distance gate
excludes unmatched source points from the ICP fit; exclusion lists, complete nearest
distances and both coverage fractions are retained. In consensus pair2, four of15
source points lie outside the distance gate. Such exclusions can also reflect a
marker absent from the preceding cloud, not necessarily a wrong stereo identity.
The existing tracker retains the complete accepted cloud as its next reference;
this experiment does not certify or remove every bad point. Marker clouds were
**not fused into an object scan**.

**Engineering conclusion:** the current tracker merits a real-motion trial. The
new threshold filter has no demonstrated necessity or general accuracy advantage
and should not be integrated merely because it was investigated. Neither tracker
acceptance nor low inlier RMS certifies the support plane or all input geometry.

## Verification and reproduction

Separate scalar graph enumeration reproduces every threshold association and all
consensus decisions. Separate inverse-Brown/least-squares/forward-projection code
checks all802 accepted stereo pairs: maximum position difference2.400e-9RAW and
reprojection difference2.340e-10px. All24 native plane fits agree with NumPy SVD;
maximum absolute-residual difference1.339e-13RAW.

A separate NumPy nearest-neighbor/SVD rigid-fit ICP implementation reproduces all42
non-anchor tracking decisions, RMS, coverage and accumulated poses. Maximum numeric
difference is1.365e-12. This verifies numerical behavior, not physical ground truth.
The private adapter passed formatting, strict release Clippy and release build.
The application is unchanged; the independently reproduced904-test gate still
applies. No fresh full application test run is claimed for this documentation.

Private root `marker-consistency-20261104` contains the plan, source/input hashes,
all raw component/pair graphs, selected clouds, rejected alternatives, archived
native tracker, private Rust source/lockfile/build logs, complete tracking outputs,
independent verifier scripts and inspected original-image overlays. Reproduce in
an output copy using `run.py`, `verify.py`, the argv-equivalent archived tracker
with `tracking-request.json`, and `verify_tracking.py`; do not overwrite originals.

Manifest `marker-consistency-milestone-20261104/evidence-manifest.json` binds
218files /32,654,687bytes; SHA256:
`df5900cc7690213e0c00aed2c39c84ed0b92e3a3907841d4621e797d912c7357`.
No raw data is included in the public repository. No hardware or service control
was performed in this experiment. Independent review of this new evidence remains
outstanding; previous review is not a blanket approval.

**Next:** process consecutive frames from the existing900-frame rotating-board
recording with full-file validation and bounded playback, then evaluate actual
motion tracking before asking for another box rotation. Preserve pose rejection,
recovery, exposure/scale uncertainty and all source geometry. Only after that
connect verified poses to actual laser-surface fusion and native viewing/export.
