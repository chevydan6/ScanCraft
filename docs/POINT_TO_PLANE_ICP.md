# Native point-to-plane registration

`scancraft_geometry::point_plane` adds a CPU reference solver for supplied target
surface normals, complementing existing point-to-point ICP. Positions and
translations are millimeters. It does not infer normals, calibrated units or
the physical provenance of an imported cloud. The offline sequence workflow
now offers explicit point-to-plane tracking/fusion as described below. Existing
registration defaults are unchanged; live acquisition integration is unfinished.

`step` computes one linearized rigid update for fixed correspondences. Each
normalized target normal contributes the row `[(p-center)/scale cross n, n]`
and right-hand side `n dot (q-p)`. The six unknowns are scaled angular motion
and translation about the source centroid. A bounded-iteration SVD solves the
system; all six singular values must exceed `1e-8` of the largest. This rejects
unobservable poses, including sliding/rotation ambiguities on a single plane.
The rotation update uses the exponential-map rotation, not a first-order matrix
masquerading as SO(3); translation accounts for rotation about the centroid.

The exact fixed-correspondence point-to-plane RMS must not increase beyond
the documented numerical allowance in the implementation. Excessive local
rotation, nonfinite calculations and invalid normals fail with no update.
Normal direction and positive/negative magnitude scaling have no effect on the
squared residual because each normal is normalized. This does not correct
inconsistent surface normals for subsequent meshing or rendering.

`icp` uses the existing exact KD-tree nearest-neighbor search and an explicit
Euclidean correspondence-distance gate. Every iteration requires at least six
matches and at least half of source points. Centered/scaled updates are composed
with the supplied initial pose. Convergence requires both residual stability
and small RMS displacement of the matched source points, using `tolerance_mm`.
Iteration exhaustion returns `converged: false`. The report explicitly labels
its metric `point_to_plane_rms_mm`; it is not Euclidean cloud distance, physical
accuracy or proof of global alignment.

Both clouds are limited to 10,000 points, with at most 1,000 ICP iterations.
Rotation-step limits must be in `(0,0.5]` radians. WorkControl cancellation is
checked during matching/system construction and between bounded SVD solves;
no partial pose is returned on cancellation. Inputs remain immutable. Bad
initial guesses can fail or reach a wrong local solution. The original quadratic path has no robust loss; the opt-in extension below adds
Huber weighting. There is still no reciprocal/normal-angle correspondence gate, global registration,
automatic normal estimation or scanner qualification in this implementation.

Six synthetic tests cover known three-plane alignment, normal sign/magnitude
invariance, rank-deficient single-plane rejection, invalid normals/overlap/step
limits/cancellation, honest iteration exhaustion, and conditioning with a
coordinate origin millions of millimeters away. The distant-origin test checks
transformed points directly as well as rotation and translation, since pose
parameter errors are coupled far from the origin. Debug and optimized test logs
and workspace checks use the `point-plane-` evidence prefix. Genuine Raptor
registration/accuracy and integration into live scanning remain unverified.

## Offline sequence tracking and fusion

`fuse-sequence` request schema 1 accepts optional
`"registration_method": "point_to_plane"`. Omission selects `point_to_point`;
unknown methods are rejected by deserialization. The chosen method is written
to the report. Point-to-plane mode requires 6..512 explicitly indexed tracking
samples with supplied finite, nonzero PLY normals on every input frame.
Positions/normals must use each input cloud's local coordinate system. Normals
outside the tracking selection remain governed by the existing fusion policy.

The tracker keeps normals alongside its last accepted local reference. It uses
the selected solver, then independently recomputes Euclidean nearest-neighbor
RMS and distinct target/source coverage for the existing acceptance gates.
Reported frame `rms_mm` therefore remains Euclidean, not the solver's plane
residual. Total translation/rotation gates and convergence checks still apply.
The per-iteration angular bound is the smaller of the tracking rotation gate
and 0.5 radians. No fallback to a different solver occurs when constraints fail.

Missing/invalid normals are caller errors and do not advance tracker state.
Rejected alignments mark tracking lost and keep the last accepted points,
normals and pose for possible recovery. Reset clears the normal reference too.
The session still publishes a candidate tracker/history update only after
fusion succeeds; preparation failures publish no result bundle. The source
clouds stay unchanged. The initial accepted frame deliberately defines the
world anchor; it is not an independently measured six-axis pose.

CLI integration tests cover explicit selection, three accepted frames separated
by one loss, recovery/world pose, preserved fused normals and input data,
missing-normal failure without publication, and the omitted-method default.
Tracker tests cover invalid-normal retry/reset and three rotated frames whose
normals change local basis. Debug workspace and optimized CLI test evidence
uses `point-plane-sequence-`. GUI interaction, genuine sensor tracking, robust
correspondences and physical accuracy remain unqualified.

For reconstruction outputs with a declared surface lattice, optional
[surface-normal estimation](SURFACE_NORMALS.md) now exports a supported-point
PLY suitable as input to this explicit workflow. It does not supply a global
registration guarantee or remove the rank/coverage gates above.


## Optional Huber loss

`step_with_loss` and `icp_with_loss` accept `Loss::Huber { delta_mm }`; existing
`step`/`icp` wrappers select quadratic loss. Delta must be finite and positive and
is explicitly in millimeters. No noise scale is inferred from a scanner or cloud.
The implementation independently follows the standard
[Huber loss and weights documented by Open3D](https://www.open3d.org/docs/release/python_api/open3d.pipelines.registration.HuberLoss.html)
and its [IRLS formulation](https://www.open3d.org/docs/release/tutorial/pipelines/robust_kernels.html).
For normalized signed plane residual r, rho is r²/2 inside delta and
`delta*(abs(r)-delta/2)` outside. Weight is 1 inside and delta/abs(r) outside.
The square root of the current residual weight multiplies each SVD row and RHS.
This reduces influence; points are not silently deleted or declared correct.

Huber updates require the exact fixed-correspondence mean rho to be nonincreasing
within `1e-10*before + MIN_POSITIVE`. The quadratic wrapper retains its previous
exact-RMS update allowance. The finite rigid pose, rank, rotation, overlap,
iteration and cancellation checks remain active. ICP convergence in Huber mode
uses both the change in sqrt(2*mean rho) and the existing RMS point-displacement
threshold. It is local convergence, not global pose correctness. Nearest-neighbor
correspondences can change between iterations; the monotonic check concerns each
iteration's fixed correspondence set, not a global multi-frame objective.

The backend report keeps unweighted `point_to_plane_rms_mm` over all current
correspondences, separately from `objective_mean_mm2` and
`downweighted_correspondences` (post-update residuals in the last correspondence
set). A smaller robust objective must not be presented as raw residual or physical
accuracy. Tracking still independently recomputes Euclidean RMS and distinct
source/target coverage, and applies unchanged convergence and total motion gates.
The robust fit can therefore be rejected by the Euclidean gate. No fallback or
quality-gate relaxation occurs. Normals remain required.

Offline CLI/native GUI sequence settings opt in using:

```json
"registration_method": {"point_to_plane_huber": {"delta_mm": 0.05}}
```

This is an example parameter, not a qualified Raptor setting. Existing string
methods and the omitted point-to-point default retain their behavior. Invalid
scales, unknown method parameters and missing normals prevent publication. The
chosen method/scale is retained in the sequence report. The GUI uses the existing
sequence-settings-file workflow; there is no new interactive robust-scale editor.

Evidence:

- Six new regressions cover fixed-correspondence bias, quadratic equivalence below
  delta, invalid scales/rank/cancellation, actual nearest-neighbor ICP downweighting
  with honest raw RMS, unchanged tracker Euclidean rejection and retained reference,
  and CLI loss/recovery/fusion/configuration validation. 489 workspace Rust tests and
  19 reference/tool tests pass with lint, formatting and release binaries/examples.
- `tools/check_robust_plane_reference.py` compares the Rust diagnostic with an
  independent nonlinear [SciPy least_squares Huber solve](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.least_squares.html).
  Six synthetic fixed-correspondence cases cover clean data, three deltas,
  reversed/scaled normals and opposite outlier sign. Maximum transformed-point
  disagreement is below 9.6e-9 mm, and exact costs agree within 1e-8 mm². These are
  numerical comparison results, not physical scanner errors. Reference trace
  monotonicity allows the implementation's relative tolerance plus a 1e-25 mm²
  numerical floor for near-zero clean residuals. Results: evidence/huber-plane-reference.
- An initial arbitrary 0.04 mm unit-test expectation failed: Rust's biased synthetic
  fit is 0.064351713 mm, independently matched by SciPy. The unsupported expectation
  was replaced by reference agreement; see huber-plane-initial-expectation.json.
  The six-case reference deliberately retains bias: delta 0.005 gives about
  0.0161 mm, delta 0.02 about 0.0644 mm, and delta 0.1 about 0.322 mm on this one
  contaminated geometry. This does not revise any physical mission acceptance gate.
- Native private-display fusion of a synthetic four-frame sequence integrates three
  frames, records one loss and recovers, producing 27 points. CLI and GUI points,
  editable project, report and manifest are byte-identical. Three captures were
  inspected, with no camera descriptors open. Bright fixture colors aid inspection;
  the initial dark-color render is retained privately. Evidence: huber-plane-gui-render
  and huber-plane-sequence-dataset. No complete manual or hardware workflow is qualified.

Reproduce the numerical comparison with the optional pinned NumPy/SciPy environment
from tools/cylinder-reference-requirements.txt (diagnostics only):

```
cargo build --release -p scancraft-geometry --example robust_plane_reference
REFERENCE_PYTHON tools/check_robust_plane_reference.py NEW_EVIDENCE_DIRECTORY
```

Global alignment, reciprocal/normal-angle matching, automatic noise-scale selection,
genuine calibrated sequences, performance and live qualification remain unfinished.
Physical calibration remains deferred. This adds no proprietary or C++ application
runtime dependency, and no device controls are written.
