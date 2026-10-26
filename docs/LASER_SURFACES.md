# Laser surface numerical models

The native `scancraft_calibration::laser_surface` module intersects an undistorted
camera ray with a curved sheet. It does not decode a factory record into a
physically calibrated projector. Camera assignment, coefficient-to-model
binding, line identity, translation units and capture-mode binding remain open.

Previously captured private OpenCL compiler inputs contain distinct older
four-coefficient and newer seven-coefficient numerical paths. Compiler input
existence does not establish which path ran for the current scanner mode.
The older path's ray equation corresponds to this sheet:

```
X = a Y² + b Y + c Z + d
```

For an undistorted normalized ray `(nx Z, ny Z, Z)`, substitution gives:

```
(a ny²) Z² + (b ny + c - nx) Z + d = 0
```

All coordinates share the caller's length unit. `a` has inverse-length units,
`d` has length units, and `b/c` are dimensionless. The native struct names each
coefficient and requires a finite positive closed depth interval. It reports
zero, one, or two intersections. Two positive roots remain explicitly ambiguous;
the caller must obtain independent correspondence evidence. A ray contained in
the sheet is an error rather than an arbitrary depth.

The implementation uses f64, scaled coefficients, a cancellation-resistant
quadratic solution, exact linear handling, fixed storage for at most two
results, positive-depth filtering, finite checks and polynomial-residual checks.
It adds no artificial curvature. The reference adds `1e-10` to its quadratic
coefficient, which can introduce an enormous spurious root for an exactly
linear sheet. That behavior is not adopted. The production module has no
OpenCL, Wine or vendor runtime dependency.

## Synthetic numerical reference

`tools/laser_surface_reference.py PRIVATE_CAPTURED_SOURCE` checks the complete
private source SHA256 before extracting only its numerical root expressions
in memory. The independently written OpenCL harness supplies synthetic rays
and coefficients, executes that fragment on the local GPU, and publishes only
synthetic inputs/roots and hashes. No original or modified source is printed,
saved in this repository or distributed. It does not invoke a full matching
kernel or access scanner devices. It releases its OpenCL resources afterward.

Reference source SHA256:
`6163b6d8448a01317e7336e761bbc9bce8bc4f65dc665deaa4e733cbba1a1f49`.
The separate cone-model compiler input has SHA256
`ccb7718cd2d411527ae109883e3b0d69e4b3574769482668483a293e69b02d43`;
no cone model or factory conversion is implemented by this checkpoint.

The actual fragment run used AMD Radeon 680M through Rusticl, CL1.2 and
single-precision constants. Rusticl printed its existing warning that a patched
Mesa libclc was not detected; these narrow numerical observations do not qualify
the OpenCL runtime generally. The seven cases exercise linear sheets, mixed
coefficients, two positive intersections, tangency, a miss and negative depths.
The miss's nonfinite reference roots are explicitly represented as nulls in JSON.
Results: `evidence/laser-surface-reference.json`. An identical synthetic fixture
is kept under `crates/scancraft-calibration/tests/fixtures/` for native regression.

Within the synthetic depth interval [1,1000], all finite in-range reference roots
agree with the native solver within 0.001 coordinate units. The reference is
f32 with added curvature; native results additionally satisfy the original
sheet equation within 1e-9 for these cases. Independent analytical tests cover
an exact linear intersection, a tangent, both possible roots, a miss, a
contained ray, and a small root in a badly separated root pair. Invalid numeric
fields, invalid depth intervals and coefficient overflow are rejected. These
are synthetic numerical tolerances, not scanner accuracy measurements.

## Binding still required

The captured older/newer GPU parameter arrays use seven-float strides. The
factory flash records expose ten-float groups; current genuine records have
plausible leading coefficients and many zero trailing fields, but this is not
proof of the host's conversion or model selection. No automatic factory-to-sheet
conversion is supplied. Likewise, projector line identity must be established
before a detected image stripe is associated with a sheet. The accurate genuine
point-cloud milestone remains open, and physical calibration remains deferred.

## Factory sheet hypothesis against genuine observations

The offline Rust example `laser_sheet_hypotheses` tests the first four floats of
each nonzero factory group as the equation above. It requires the trailing six
floats to be zero and tests only full-size lens slots. It tries both assignments
of native interfaces 0/2 to factory lens slots 0/1, preserving the stored R/T.
The Brown order, radius 0.8 and depth interval [100,1000] in raw units are
explicit numerical hypotheses, not established physical calibration coverage.
The example never saves a point cloud or activates a device calibration.

```
cargo run --release -p scancraft-calibration --example laser_sheet_hypotheses -- \
  FACTORY_BIN LEFT_STRIPES_JSON RIGHT_STRIPES_JSON
```

Inputs are bounded regular files, limited to 5000 observations per side, equal
saved sensor PTS, and known profile/schema fields. Factory CRC is verified.
Observation JSON integrity statements are checked but are not cryptographic
proof of source recordings; the actual experiment independently reran the
native detector on frame 30 of both original 60-frame recordings. All stripe
coordinates, payload/metadata CRCs and non-wrapper fields exactly matched the
previously extracted single-frame results. The extracted wrapper's frame 0 is
original source frame 30. Evidence:
`evidence/laser-sheet-observation-revalidation.json`.

For each sheet/ray intersection, the example predicts a pixel in the second
camera and seeks a detection within three pixels. It repeats this check after
wrapping second-camera x coordinates by +200/+400/+600 pixels, keeping their
y distribution. These are simple accidental-support controls, not a statistical
significance test. Counts include prediction reuse, distinct observation counts
and a distance-sorted greedy pairing with no observation reuse. Greedy pairing
is not maximum bipartite matching or a verified correspondence assignment.
All comparisons share a hard 100-million-comparison budget; exceeding it fails
without a partial report. Camera rays are unit vectors, so the example converts
them to `(ray.x/ray.z, ray.y/ray.z)` before sheet intersection.

On the saved 294/606-detection pair, seven nonzero groups are present per
eligible factory record. Final corrected results are:

| Region | Interface for first factory lens | Supported predictions | Greedy distinct pairs | Shifted-control distinct pairs (+200/+400/+600) |
| --- | --- | --- | --- | --- |
| 200000, first LAS_L_SCAN | 0 | 18 | 6 | 2 / 0 / 1 |
| 200000, first LAS_L_SCAN | 2 | 15 | 10 | 0 / 1 / 0 |
| 202000, first LAS_L_SCAN | 0 | 18 | 6 | 2 / 1 / 1 |
| 202000, first LAS_L_SCAN | 2 | 20 | 13 | 0 / 1 / 0 |

The full-size CROSS_LAS_L_SCAN record in region 200000 produces identical counts;
this does not constitute an independent dataset or replicate. Raw aggregate
reports are `evidence/laser-sheet-hypothesis-200000.json` and
`evidence/laser-sheet-hypothesis-202000.json`. The initial diagnostic accidentally
used unit-ray x/y directly; an analytical synthetic camera/plane test caught
that convention error before commit. The reports above were regenerated after
the correction; preliminary 23-prediction counts are superseded.

Support exceeds these shifted controls but remains sparse and can reuse
clustered detections. Both camera orders have support. This does not establish
the camera assignment, sheet identity, model binding, physical units or accurate
geometry. The source scene's blur/limited texture and lack of independent
physical ground truth remain material limitations. No winning mapping is
selected. Three example tests check wrapped controls, observation reuse,
analytically known projection/order, budget failure and unsupported extra
coefficients. Production calibration and geometry are unchanged.


## Subsequent horizontal-reflection investigation

The unmirrored results above remain reproducible, but an explicit reflected
observation hypothesis has substantially stronger support on temporal holdouts.
See [camera reflection evidence](CAMERA_REFLECTION_HYPOTHESIS.md) for the fixed
profiles, controls, source lineage and remaining validation limits. This does
not activate a factory calibration or establish genuine accurate geometry.
