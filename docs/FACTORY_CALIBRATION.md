# Existing factory-data investigation

The native commands read or inspect existing bytes only. The user's physical
calibration deferral remains in effect. No firmware or factory-data write API
is provided.

- `scancraft-cli factory-read 20a000 NEW_DIRECTORY` reads the observed fixed region.
- `scancraft-cli factory-verify FILE` authenticates either supported envelope.
- `scancraft-cli factory-inspect FILE` indexes the supported laser-record schema.
- `scancraft-cli factory-lenses FILE` extracts the supported named-camera lens blocks.

Raw files should stay under `evidence/private/`, which Git ignores. The lens
command emits device-specific values; redirect its output there as well.

## Supported named-camera decoder

Only top/body signatures 0x46447312/0x4644610f, declared length 6646, count/size
words 1/6586, payload flag 1, version field `3.0.13`, count byte 3, and labels
IR_L/IR_R/COLOR in observed order are accepted. Each camera must have a Brown
lens label, bounded positive dimensions, finite lens values and positive first
two intrinsic values. Four intrinsic and eight Brown floats stay in file order.
Unused historical pointer/padding bytes are not dereferenced. No CameraModel,
Brown5 or stereo calibration is constructed from this diagnostic result.

Unknown coefficient formulas/directions, pose association/units, active mode
binding and independent physical accuracy must be resolved before reconstruction.
The parser reference establishes extraction, not these optical properties.

## Offline reference experiment

`tools/factory_parser_oracle.c` is independently written temporary diagnostic C
code. It loads the locally installed file-parser DLL, calls only
`LoadCalibrationParamsMulti(input, private_zeroed_buffer, 0)`, and saves status
plus a private buffer dump. It does not load the scanner SDK or expose device
operations. The production implementation remains Rust and has no DLL/Wine
dependency. Proprietary DLLs and memory dumps are not redistributed.

The helper was compiled with Clang/LLD in the existing kernel compiler image,
using the installed Wine kernel32 import library. It ran inside bubblewrap with
a read-only root, a writable private evidence directory, a private `/dev`, no
display and an unshared network. A private copy of initialized Wine system files
avoided first-run initialization; the original scanner prefix was not modified.
The initial empty-prefix attempt timed out before parsing and its own processes
were terminated. The initialized-prefix attempt returned status zero.

The reference printed a warning that it has no size check for this lens-model
combination. Its success is therefore not a structural validation oracle.
The Rust decoder applies explicit bounds, version, count and schema checks.

To compare an existing private dump with native output:

```sh
scancraft-cli factory-lenses evidence/private/factory-region-20a000/factory-region.bin > evidence/private/factory-lenses-native.json
python3 tools/compare_factory_lens_oracle.py evidence/private/factory-lenses-native.json evidence/private/parser-oracle-result.bin
```

The comparator checks three labels, six dimension integers and all 36 float32
bit patterns. It emits only counts, equality status and hashes. The reference
output offsets are specific to the hashed installed DLL; they are not a stable
public ABI. The recorded result and DLL hash are in
`evidence/factory-lens-oracle-comparison.json`. Physical qualification is pending.

## Named pose fields

The native diagnostic also returns `rotation_fields` and `translation_fields`,
three floats each in file order. The reference text exporter calls them
Rotate.RX/RY/RZ and Translate.TX/TY/TZ. All 18 pose float bits match the
reference object. Field names alone do not establish angle representation,
order, units, transform direction, active mode or physical accuracy. No
rotation matrix or calibrated stereo record is created. Nonfinite pose values
are rejected under the same strict authenticated schema.

The temporary helper optionally accepts `PARSER_TEXT`, refuses an existing
text path, and calls `WriteCalibrationParamsMulti(path, buffer, 3, 0)` to
create a local YAML file. This is a file export, not a hardware calibration
write; run it only with scanner nodes unavailable. It was invoked in the
same read-only-root/private-output/device-inaccessible sandbox. Production
Rust does not depend on either vendor API.

Pass `--named-export PRIVATE_YAML` to the comparison tool to check the simple
scalar subset emitted by that exact exporter. This is not a general YAML
reader. Current binary comparison covers 36 lens and 18 pose floats; named
comparison covers 48 floats and six dimensions. Lens names established by
this export are Fx/Fy/Cx/Cy and Brown slots 0/1/2/3/6/7 as K1/K2/K3/K4/P1/P2.
The remaining two Brown slots and the distortion formula remain unverified.
See `evidence/factory-pose-oracle-comparison.json` for the recorded result.

## Observed pose-matrix convention

`factory_pose::matrix_raw_units` independently computes a row-major homogeneous
matrix for column vectors using Rz(RZ)*Rx(RX)*Ry(RY), with radians and unchanged
raw translation. `factory-lenses` includes these diagnostic matrices together
with explicit direction/unit uncertainty flags. These are not calibrated
StereoRig extrinsics and are never automatically used for reconstruction.

The convention was tested against six synthetic numerical-prefix captures
from the exact DLL hash recorded in evidence/factory-pose-math-summary.json.
The temporary `tools/factory_pose_prefix_oracle.c` loads only the local depth-
engine DLL, supplies synthetic fields, and sets a breakpoint before the
remainder of the routine executes. Its handler saves the known scratch
matrices and exits. It requires that exact local DLL, checks the stop-site
instruction prefix, and must run only in the device-inaccessible, network-
isolated sandbox. It is research instrumentation, not a production dependency.
No scanner data is accepted by this helper. No vendor instructions are copied
into the repository; public fixtures contain only synthetic numerical results.

Identity, each axis, asymmetric mixed angles and mixed translation match Rust
within 1e-12. The mixed-angle case differs from all five other Euler orders.
This does not establish relative-camera composition/inversion, physical units,
active-mode applicability or physical accuracy. Those must be resolved before
creating an application calibration record or claiming genuine 3D geometry.

## Observed relative pose operation

`factory_pose::relative_matrix_raw_units` implements the observed ordered
operation T(second)*inverse(T(first)), using rigid rotation transpose and
unscaled translation. `factory-lenses` emits this as
`second_times_inverse_first_raw_units`, alongside explicit unresolved unit
and physical-direction flags. It does not create StereoRig or RAPCAL data.

`tools/factory_relative_pose_oracle.c` is a second synthetic-only prefix probe
for the same hashed local depth-engine DLL. Its breakpoint at RVA 0x12edf
occurs after both pose matrices and their ordered relative product are built,
before the rest of the engine executes. It captures four scratch matrices
and exits; all six synthetic cases reached that breakpoint. Run only with
hardware nodes unavailable, network unshared and original files read-only.
The helper never accepts scanner calibration bytes. The public fixture stores
only independently chosen synthetic inputs and numerical outputs.

Both pose slots use Rz*Rx*Ry radians. Reference inversion and multiplication
match independent numerical calculations; reversed multiplication differs on
noncommuting cases. Rust matches all six relative references within 1e-11.
See evidence/factory-relative-pose-summary.json for scope, offsets and hash.
Physical translation units, active capture-mode binding, actual camera/world
interpretation and image correspondence checks remain necessary before a
calibration record can be used for genuine scanner reconstruction.

## Forward Brown coefficient mathematics

`factory_brown::forward` evaluates the observed rational model, accepting
[k1,k2,k3,k4,k5,k6,p1,p2]. The radial factor is
(1+k1*r2+k2*r4+k3*r6)/(1+k4*r2+k5*r4+k6*r6); standard tangential terms follow.
`brown5_if_supported` refuses any nonzero denominator coefficient. It reduces
only coefficient representation, without validating coverage/invertibility.
The actual private factory data supports this reduction for all three lenses.

`tools/factory_brown_lut_oracle.c` is optional synthetic-only offline research
instrumentation for the hashed installed LUT DLL. It calls object constructor,
image/intrinsic/coefficient/config setters and the forward LUT generator. It
writes private tables using CREATE_NEW and never accepts scanner data or
calls scanner APIs. Run with hardware unavailable, network unshared, original
files read-only and only private evidence writable. The two output planes are
separate f64 normalized displacements. Expanded-grid border polynomial/clamp
behavior is outside the comparison; only original-image sample positions count.

Six coefficient sets exercise zero, radial, tangential, full rational and
independent denominator effects. Across 4608 in-image points the normalized
component error is <1e-7; the retained public Rust fixture has 210 points and
2e-7 tolerance. The DLL's mixed float/double arithmetic differs slightly from
Rust f64 evaluation. This comparison establishes forward coefficient math;
it does not establish a physically accurate or inverse-valid camera model.

`tools/check_factory_epipolar.py` is an offline OpenCV reference diagnostic.
It requires saved frame manifests with equal PTS and expected interface/order,
then matches SIFT features after CLAHE without factory-geometry guidance.
Only a nonempty match set receives hypothetical epipolar scores; no score is
a qualification result. The saved 30 FPS pair produced zero mutual matches under that method.
The cause is not established, and this does not reject the factory geometry. Better
exposure/illumination and independently checked correspondences are needed.

## Numerical inverse-domain audit of the actual lens fields

The native offline example `factory_domain_audit` constructs temporary lens
models from authenticated named fields, supported Brown5 reduction, zero skew
and an **explicit test radius**. It never serializes a RAPCAL record or changes
scanner state. Example (input remains private):

```
cargo run --release -p scancraft-calibration --example factory_domain_audit -- PRIVATE_FACTORY_REGION 0.8
```

The radius is an experiment parameter, not measured optical/calibration
coverage. The existing analytic strong-monotonicity bound validates the Brown
Jacobian over that disk. Every integer image pixel center is then inverted
and reprojected, counting failures and residuals. Continuous rectangle coverage
is not asserted. Reports use aggregate numerical findings without publishing
factory coefficients. An audit can exit successfully with reported pixel
failures; inspect `failed_pixel_centers`, not just the process status. Invalid
schemas, models and files fail before a successful report. Inputs are bounded
regular files; at most 4,194,304 pixels per lens are tested.

For the actual three decoded lenses at radius 0.8, all 5,632,000 pixel centers
inverted and reprojected within 1e-7 pixels. Worst native residual was below
2.1e-9 pixels. Maximum inverse normalized radii were approximately 0.5555,
0.5599 and 0.6649. These observations establish a numerically usable test
domain under the decoded lens interpretation; they do not establish physical
lens accuracy, active-mode image binding, translation units or pose direction.

`tools/check_factory_domain.py PRIVATE_LENSES_JSON NATIVE_AUDIT_JSON` runs an
independent OpenCV reference across the same complete integer pixel grids in
bounded row batches. OpenCV 5.0.0 maximum round-trip residual was below 1e-12
pixels; aggregate maximum inverse radii differed from Rust by <2.1e-13. This
is an aggregate comparison, not pointwise equality of every reference ray.
As a negative control, a deliberately insufficient radius 0.3 reported failed
pixels for every lens (approximately 1.10M, 1.09M and 0.66M respectively).

Evidence: `factory-lens-domain-audit.json`, `factory-lens-domain-reference.json`
and `factory-lens-domain-negative.json` under `evidence/`. All 166 workspace
tests, formatting and warning-denying Clippy pass; the release example built
and executed. The reference Python tool is research-only; production remains
Rust without OpenCV dependencies. Factory coefficients and source bytes remain
private. Physical calibration is still deferred.

## Synthetic rectification-prefix relationship

`tools/factory_rectification_prefix_oracle.c` extends the synthetic-only probe
for the same hashed depth-engine DLL to stop at RVA 0x131ce, before lens-field
copying or downstream image processing. It captures the earlier matrices and
two 3×3 stack matrices at 0x2a0 and 0x90 (column-major ABI). The helper accepts
only its six compiled-in synthetic poses; it never reads factory bytes. As
with earlier helpers, run only with root read-only, private output/prefix
writable, a fresh device namespace without scanner nodes, and network unshared.
The private Wine prefix also needs WINEPATH set to the read-only plugin folder
for dependent DLL loading. The initial missing-dependency attempt exited 22
before entering the routine; adding that search path allowed all six cases to
reach the stop site. Original Wine configuration and installed files stay intact.

For the five finite mixed-pose cases, with Q_first from 0x2a0 and Q_second from
0x90, the numerical identities are:

```
Q_second * R_relative = Q_first
Q_second * t_relative = [norm(t_relative), 0, 0]
```

Maximum rotation relation residual is <6e-16. This strengthens the ordered
numerical interpretation of T(second)*inverse(T(first)) in the reference
rectification path. It does not assign those slots to active physical sensor
interfaces or establish translation units. In particular it must not silently
convert factory poses into an application's `right_from_left` calibration.

The zero-angle synthetic case with a nonzero baseline reached the breakpoint
but both captured rotations were nonfinite. This is a reference-prefix
numerical degeneracy, not proof of a scanner failure: this isolated invocation
may lack higher-level preconditions. The independently implemented native
rectifier already tests identity, vertical and reversed baselines and must not
adopt this NaN behavior. The public synthetic fixture explicitly flags this
case and omits nonfinite matrices; Rust checks that its raw relative transform
remains finite for that input.

Only synthetic numerical inputs and finite outputs are retained in
`crates/scancraft-calibration/tests/fixtures/factory-rectification-prefix.json`.
No vendor instructions, DLLs, raw factory bytes or heap dumps are distributed.
Aggregate scope/hash/offset evidence:
`evidence/factory-rectification-prefix-summary.json`. One new Rust relation
regression passes; all 167 workspace tests, warning-denying Clippy and formatting
pass (`evidence/factory-rectification-tests.log`). No native production formula
was changed or replaced by the reference implementation.


## Laser-record serializer layout verified with synthetic markers

The native `factory-laser-fields FACTORY_REGION_FILE` command now inspects the
observed 1680-byte records, separately from the named-camera decoder. CRC and
record bounds are checked first. It rejects unknown record labels, nonzero
reserved fields, invalid dimensions/focal lengths and nonfinite interpreted
floats. It never produces an application calibration or writes the device.

Record-relative layout established for this observed schema:

| Offset | Bytes | Fields |
| --- | --- | --- |
| 0x00 | 16 | Opaque, not dereferenced |
| 0x10 | 16 | Label (post-NUL bytes ignored) |
| 0x20 | 8 | Two width/height pairs, u16 |
| 0x28 | 4 | Observed reserved zero |
| 0x2c / 0x60 / 0x94 | 52 each | Four intrinsic floats, u16 width/height, eight coefficient floats |
| 0xc8 / 0xf8 | 48 each | Nine matrix-like floats, three translation-like floats |
| 0x128 | 1280 | 320 coefficient floats, exposed as 32 groups of ten |
| 0x628 / 0x64c | 36 each | Row-major essential / fundamental matrices (verified in next section) |
| 0x670 | 32 | Opaque tail |

`tools/factory_laser_layout_oracle.c` is a synthetic copy-region probe for
`line_laser_param_preprocessing.dll`, SHA256
`1eeafa48017e8b18deb3bd01fe49e9652f3a810a2a64a7eb01bb26eccdc3402a`.
It enters RVA 0x44b4 with its own stack/scratch memory, substitutes the first
load in its exception handler, and stops at RVA 0x4635 before subsequent calls.
No higher-level flash generator is invoked. Every source float is a unique
synthetic marker; no factory bytes enter the reference process. The six copy
ranges contain 383 words, all bit-exact against the independently calculated
marker sequence. This verifies binary movement, not optical formulas.
`tools/check_factory_laser_layout.py DLL MARKERS` checks the DLL hash, exact
output length and every marker. Aggregate evidence is
`evidence/factory-laser-layout-summary.json`.

Compile using the same private Wine headers/container recipe as the other
prefix probes. The actual run used bwrap with root read-only, only
`evidence/private` writable, `/tmp` temporary, a fresh `/dev` without scanner
nodes, network unshared, DISPLAY/WAYLAND_DISPLAY unset, the private
`parser-reference-prefix`, and WINEPATH pointing to the read-only plugin folder.
Original user Wine prefixes and hardware were not accessed. The copied output
is private; only source for the independent helper and aggregate results are
published. Verify the hash before running the helper, not just afterward.

Both previously read, CRC-valid factory regions decode: four records each.
Their first-record intrinsic arrays differ from the corresponding named-camera
slots in all three positions. The matrix-like fields have orthogonality
residual below 1e-7 and determinant near +1. These numerical checks do not
establish row/column interpretation, physical direction, sensor assignment,
translation units, or which record corresponds to the active capture mode.
The ten-value setter ABI supports grouping the coefficient section, but does
not establish a plane equation, projector-line identity or polynomial model.
Do not automatically apply the named-camera calibration to laser captures.
`evidence/factory-laser-record-audit.json` records dimensions and aggregate
checks without distributing the private factory values.

Two new Rust tests cover all interpreted synthetic fields, every truncation,
every single-byte corruption and CRC-valid invalid numeric/reserved fields.
All 216 workspace tests pass; see `evidence/factory-laser-layout-tests.log`.
Accurate genuine 3D reconstruction and physical calibration remain open.


## Laser-record essential and fundamental matrices

The previously opaque record fields at 0x628 and 0x64c are now decoded as two
row-major 3×3 matrices. The independent native formula uses the first two lens
slots and the first transform slot:

```
E = [t]cross * R
F = inverse(K_second).transpose() * E * inverse(K_first)
```

`tools/factory_laser_epipolar_oracle.c` verifies this against the same hashed
preprocessing DLL used for the layout probe. It supplies only compiled-in
synthetic lens/pose values to RVA 0x75f0, stopping at 0x7ba1 after numerical
outputs and before the routine returns. Six cases cover zero baseline,
horizontal and mixed translations, a quarter-turn, a mixed-axis cycle, and a
0.6/0.8 rotation with unequal focal lengths/principal points. Inputs and output
matrices are redistributable synthetic values only, in
`crates/scancraft-calibration/tests/fixtures/factory-laser-epipolar.json`.
`tools/check_factory_laser_epipolar.py DLL PRIVATE_DIRECTORY` verifies the hash,
all output lengths/finiteness and independent scalar matrix algebra.
Maximum essential difference is <2e-6 and fundamental difference <3e-6 from the
f32 reference. Row-major interpretation agrees; transposed alternatives differ
in the nondegenerate rotated cases. This establishes numerical convention,
not physical camera assignment or translation units.

Run conditions match the preceding probe: read-only root/vendor installation,
private writable prefix/output, fresh device namespace without scanner nodes,
network unshared, no desktop display. All six helper processes exited zero and
produced exactly 152 bytes. One external process printed a `safe_close` assertion
during these Wine runs; the source of that diagnostic was not established.
Successful numerical output is not a claim that Wine is crash-free or that the
scanner passed any crash qualification. Summary:
`evidence/factory-laser-epipolar-summary.json`.

The native CLI now includes computed matrices and absolute residuals against
the stored fields. Across all eight genuine records, maximum absolute E
residual is <4.2e-6 and F residual <5.5e-6. These are internally consistent with
f32 arithmetic; they are not physical pixel or millimeter accuracy errors.
Aggregate evidence: `evidence/factory-laser-epipolar-record-consistency.json`.

Production Rust `epipolar::EpipolarMatrices::from_calibration` validates the
application's full stereo model and SO(3), requires zero skew, and normalizes
the baseline. Its Sampson-distance function accepts undistorted pinhole pixel
coordinates, normalizes matrix scale, and rejects nonfinite, degenerate or
overflowing calculations. Sampson distance is a first-order correspondence
gate, not triangulation reprojection error. The separate `raw_field_matrices`
entry point is explicitly a numerical diagnostic and does not assign physical
units or convert factory records into validated application calibration.
Three new tests cover six reference cases, projected correspondences, displaced
outliers, scale/sign invariance, invalid pose/numeric inputs and zero-baseline
constraints. Factory decoding also checks the added tail floats for finiteness
and preserves every synthetic field in tests.

A repeat of the earlier SIFT/CLAHE mutual-ratio check on the saved line-on pair
has only six tentative matches. Assuming the observed eight-float Brown order
and either transform direction for first-record fields yields no match below
two approximate pixels for either saved region. This sparse result neither
validates nor rejects physical calibration: correspondence reliability,
coefficient order and capture-mode association remain unverified. Report:
`evidence/laser-record-image-epipolar-hypotheses.json`. The reported distances
use the older normalized-coordinate diagnostic with average-focal scaling;
they are explicitly approximate and do not claim the exact pixel Sampson
metric of the new native module. The laser coefficient formula, line identity,
physical sensor mapping and accurate geometry remain open.

Reproduce the image diagnostic with `tools/check_factory_epipolar.py
FACTORY_LASER_FIELDS_JSON LEFT_FRAME_DIRECTORY RIGHT_FRAME_DIRECTORY
--laser-record-index 0` in the private OpenCV reference environment. It requires
full-size IR slots and equal saved PTS; the report explicitly retains the
coefficient/camera hypotheses. Saved timestamp equality alone does not prove
exposure synchronization. All 219 workspace tests, warning-denying Clippy,
formatting and the release workspace build pass (the `laser-epipolar-*` logs).


## Laser surface formula research

`docs/LASER_SURFACES.md` documents a verified synthetic numerical fragment from
previously captured compiler inputs and an independent native curved-sheet
solver. The older path uses X=aY²+bY+cZ+d; a separate cone path also exists.
This narrows the possible model semantics but does not map the ten-value flash
groups to the seven-value GPU arrays or establish current mode/line identity.
Factory-to-surface conversion and accurate genuine reconstruction remain open.
