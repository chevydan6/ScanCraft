# Local geometric cylinder fitting

Expand **Cylinder measurement** in the point-cloud workbench, select 6..20000
cylindrical wall points across multiple heights and angles, set a maximum
geometric RMS in millimeters and click **Fit cylinder to selected**. The worker
reports axis point/direction, diameter, radial RMS/max residual, sampled axial
range, Jacobian conditioning and convergence diagnostics. Endcaps/outliers
influence this unweighted fit; select the intended wall evidence explicitly.
Read-only fit results are transient status reports. Saved cylinder measurement
records and historical outlines are now supported; see CYLINDER_SNAPSHOTS.md.

For a whole-file offline measurement without camera acquisition:

```
scancraft-app --fit-cylinder PROJECT_OR_PLY_FILE MAX_RMS_MM
```

This opens the file, selects its points and runs the read-only fit. Geometry,
project dirty state, measurement snapshots and geometry undo/redo are unchanged;
startup selection changes are not geometry edits. Imports still require declared
units or the existing explicit fallback. A zero RMS gate requires exactly zero
computed residual, rather than hiding floating-point error with an implicit
physical tolerance. The default interactive gate is an editable fit threshold,
not a scanner accuracy claim.

The independent Rust implementation in `scancraft_geometry::cylinder` uses the
orthogonal radial-distance objective: distance from each point to the infinite
axis minus radius. The mathematical objective and axis-gauge normalization are
consistent with Shakarji's NIST discussion, section 3.6:
[Least-Squares Fitting Algorithms of the NIST Algorithm Testing System](https://pmc.ncbi.nlm.nih.gov/articles/PMC4890955/).
No NIST implementation was copied and no ATS equivalence is claimed. This is a
local algorithm, not automatic global primitive recognition or a CAD model.

Implementation details and acceptance:

- Center/scale input to normalized coordinates; reject nonfinite or overflowing
  data and input covariance eigenvalue ratio <=1e-10 (insufficient 3D support).
- Try all three PCA axes. Initialize each axis's projected radius/offset using
  the existing algebraic circle fit, then optimize all five independent cylinder
  parameters with damped Gauss-Newton steps: two transverse offsets, two tangent
  axis changes and radius. Axis direction is not fixed during optimization.
- Use a positive radius and unit axis, keeping the axis point closest to the
  centered data origin. Limit angular steps to 0.35 radians and candidate radius
  and offset to bounded normalized extents (radius >1e-12 and <=1e6; offset <=1e6).
- Bound each start to 80 iterations and each iteration to 12 damping trials.
  Small steps alone do not establish convergence: the normalized gradient must
  be small, the undamped five-column Jacobian must have singular-value ratio
  >1e-8, and its independent correction norm must be <=1e-7. Rejected/stalled
  starts do not silently become successful results.
- Choose the lowest residual among converged, full-rank starts and apply the
  caller's finite nonnegative world-mm RMS gate. Report radial statistics from
  all selected samples; do not rename algebraic error as geometric distance.

The axis point is the point on the line nearest the data's centered origin, not
an endcap. The dominant axis component is made positive as a numerical sign
convention. Axial range describes projected samples, not verified object length.
The reported condition is for the normalized tangent parameterization. Normal
attributes are not used as axis constraints. Planar/collinear data, very poorly
conditioned partial arcs or extreme aspect ratios can be rejected. Three starts
are not a proof of global optimality. Cancellation is checked through bounded
loops and between bounded SVD calls. No physical qualification is implied.

Evidence: `evidence/cylinder-{tests,clippy,release}.log`,
`cylinder-example-report.json`, redistributable `cylinder-preview.ply` and
`cylinder-render/`. Reproduce the synthetic wall/report with Rust example
`cylinder_fixture NEW_PLY`. Tests include central-difference Jacobian checks,
nonlinear recovery from a wrong tilted/offset initial model, long/squat tubes,
rigid poses, distant origins, scales 1e-80..1e80, known radial noise/RMS rejection,
asymmetric partial-wall axis refinement, invalid data, degeneracy/cancellation,
and read-only worker preservation of existing redo history. Native rendering is
a synthetic startup measurement check, not complete mouse interaction, reference
algorithm comparison, performance qualification or physical scanner validation.

An optional independent SciPy comparison now covers eight synthetic cases; see
CYLINDER_REFERENCE_COMPARISON.md. This does not establish NIST or physical
qualification.
