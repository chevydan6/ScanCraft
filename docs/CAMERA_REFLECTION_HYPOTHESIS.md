# Camera reflection hypothesis

Status: offline diagnostic evidence, no production calibration activated.
Physical calibration remains deferred. Neither dimensions nor accuracy of a
genuine reconstructed object have been verified.

The named and laser factory blocks have different apparent horizontal
conventions. `tools/compare_factory_coordinate_hypotheses.py` compares decoded,
CRC-verified CLI reports without distributing their raw values. It tests
`S=diag(-1,1,1)`, `R'=S R S`, `t'=S t`, `cx'=width-1-cx`, and `p2'=-p2`.
Only matching full-size lens slots are compared. S is a reflection, not a proper
rotation and cannot be silently installed as a rigid camera pose.

For the first full-size record, maximum rotation-entry differences decrease
from about 0.694 to 0.000863 (region 200000), and from 0.693 to 0.00139
(202000). Translation maximum differences decrease from about 258 raw units
to 0.0435 and 0.133 respectively. The fields are not identical. Their age,
temperature dependence, runtime purpose, physical axes and units remain unknown.
Aggregate evidence: `evidence/factory-horizontal-reflection-{200000,202000}.json`.

The Rust `laser_sheet_hypotheses` diagnostic accepts optional `--mirror-x`.
It reflects both sets of detected pixel centers in memory before ray/surface
calculations. Original recording bytes, metadata and CRCs remain unchanged.
It tests both interface assignments, with the previous depth interval, surface
coefficient interpretation, Brown order, numeric radius and support thresholds.
The default diagnostic remains unmirrored; no CameraModel/CalibrationRecord
activation or point-cloud export is added.

Frame 30 suggested a stronger mirrored hypothesis. The fixed audit
`tools/audit_laser_mirror_pairs.py` then examined source frames 10 and 50 of the
same 60-frame native line-on recording, using unchanged baseline and local
opening profiles. Each native replay validates all frames/footer, metadata,
profile interface and selected frame; cross-profile source identities and
equal pair sensor PTS are checked. Full observations remain private.

For the first full-size record with factory camera 1 assigned to interface 2:

| Region | Frame | Profile | Unmirrored pairs | Mirrored pairs | Mirrored controls +200/+400/+600 px |
| --- | --- | --- | --- | --- | --- |
| 200000 | 10 | baseline | 10 | 170 | 3 / 4 / 0 |
| 200000 | 50 | baseline | 8 | 171 | 2 / 4 / 0 |
| 202000 | 10 | baseline | 13 | 165 | 4 / 4 / 0 |
| 202000 | 50 | baseline | 12 | 167 | 3 / 3 / 0 |
| 200000 | 10 | local strict | 186 | 1114 | 152 / 187 / 34 |
| 200000 | 50 | local strict | 194 | 1123 | 151 / 192 / 29 |
| 202000 | 10 | local strict | 195 | 1081 | 157 / 206 / 33 |
| 202000 | 50 | local strict | 197 | 1069 | 157 / 206 / 27 |

These are distance-sorted greedy pairs without observation reuse, not maximum
matching or verified correspondences. Reports include both camera assignments
and all eligible records: `evidence/laser-mirror-holdout-{200000,202000}.json`.
Duplicate full-size CROSS records do not constitute independent replicates.
Temporal holdouts of one scene are not independent ground truth. Periodic
stripes, model-guided selection, blurred detections and arbitrary shifted
controls limit interpretation; these counts are not a significance test.

Synthetic Rust assertions check the reflection against an analytically known
camera/surface pair, involution and the pixel-center domain. Three Python tests
check an exact reflected quarter-turn/translation/lens fixture, cropped-mode
exclusion, invalid inputs and unverified flags. Production geometry remains
unchanged. Next validation must establish runtime camera/image conventions,
line identity and mode binding independently, then physical units and accuracy
against calibrated ground truth before this can become usable calibration.

## Independent image-feature check

`tools/audit_factory_feature_orientation.py` runs the existing image-only SIFT
matching (CLAHE, mutual ratio 0.7) for all four combinations of camera order and
horizontal observation reflection, using named fields and laser record 0 from
202000. Matching is performed before coordinate reassignment; no surface model
or factory geometry selects correspondences. Both stored and inverse poses
are evaluated with each assignment, including the appropriate lens intrinsics.
The earlier inverse-only check did not swap intrinsic slots.

Native replay checks the saved single-frame wrappers, and native re-exported
image pixels must equal the PNGs used by SIFT. This does not reverify the
original multi-frame recording or independently establish exposure timing.
Image hashes/keypoint counts/match counts must remain equal across hypotheses.

The saved line-on pair has six mutual matches. All 16 source/order/reflection/
pose combinations have zero matches under two approximate Sampson pixels;
median errors range from about 52 to 70 pixels. The saved line-off pair has no
mutual matches. Thus the present feature evidence does **not corroborate** the
stripe-derived mapping. This weak/blurred scene cannot distinguish incorrect
feature correspondences from incorrect calibration conventions; neither is
declared resolved. No thresholds were tuned to obtain agreement.

Reports: `evidence/feature-orientation-line-{on,off}.json`. Two synthetic tests
check correspondence-preserving lens-slot reassignment, reflection/involution,
source immutability, endpoints and invalid domains/shapes. The reference tool
requires the private OpenCV environment; production remains Rust and acquires
no OpenCV runtime dependency. Further progress needs independent runtime
convention evidence and/or usable textured physical observations, followed by
physical calibration/accuracy checks. No production mapping is selected.

## Native numerical conversion

`scancraft_calibration::orientation` now provides explicit checked pixel reflection
and camera-model reflection. The camera conversion changes `cx` to
`width-1-cx`, negates skew and Brown tangential `p2`, and preserves the other
fields. Both models must validate. It changes both the image-X and camera-X
bases; the reflecting matrix has determinant -1 and is not a physical rotation.
No source image, calibration record, device setting or active camera assignment
is changed. The offline stripe example uses the shared pixel-domain helper.

Analytical tests check that projection commutes with reflection at 126 points
with nonzero skew/radial/tangential distortion and two depths, that inverse unit
rays reflect consistently, and that double conversion restores the model within
floating-point precision. Pixel tests include endpoints, singleton images and
invalid/nonfinite domains. These prove numerical conversion behavior, not the
Raptor's runtime convention. See also the SDK-accessor clarification in
[mirror diagnostics](MIRROR_STATUS.md).

`reflect_stereo_basis_x` extends the explicit numerical conversion to both
camera models and the right-from-left pose: `R'=S R S`, `t'=S t`. Input and
output must validate; a reflection matrix is never installed as the rotation.
This also reflects the left-camera scene coordinates and changes their
handedness relative to the original scene. It must not be treated as an
orientation-preserving physical movement or silently applied to mesh winding.
It neither converts raw factory translation into millimeters nor creates a
qualified calibration record.

A synthetic stereo test uses a rotated baseline, nonzero three-axis translation,
skew and Brown distortion. At twelve known 3D positions, pose application,
reflected pixel projections and reconstructed coordinates agree within their
specified numerical tolerances. Double conversion restores the fixture exactly;
an invalid rotation is rejected. Production scanning selects no reflection or
camera assignment automatically.

## Explicit raw pixel conversion

`GrayImage::reflect_x_into` copies packed GREY rows into caller-owned reusable
scratch space with exact horizontal reversal. It allocates no image buffer,
preserves row order/dimensions, and rejects an incorrect output size before
changing that buffer. Source recording bytes and metadata are untouched. The
caller must deliberately choose it; acquisition and reconstruction defaults do
not select an orientation from the current factory hypotheses.

Three image tests cover odd/even widths, nonsquare and singleton images,
involution, source preservation and wrong-size rejection. The integrated
laser test compares detection before/after image reflection with the checked
pixel-coordinate transform for both raw and local-opening detectors, including
border and saturation rejection, run endpoints, signal, centroid and variance.
This verifies consistency between pixel processing and the existing coordinate
conversion; it does not identify physical projector lines or qualify geometry.
