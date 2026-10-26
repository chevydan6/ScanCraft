# Saved historical cylinder measurements

Select 6..20000 cylindrical wall points, set the cylinder panel's geometric RMS
limit, then expand **Saved measurement snapshots** and click **Save selected
Cylinder measurement**. This recomputes the fit from the selected evidence and
retains source coordinates, axis, radius/diameter, radial residual statistics,
sampled axial range, conditioning, iteration/converged-start diagnostics and the
requested RMS gate. It marks the project dirty without changing geometry,
selection or geometry undo/redo. Save the project to persist the record.

The row shows diameter, radial RMS, saved limit and source count. Hover it for
axis, sampled range and diagnostics. Existing rename/removal and separate snapshot
undo/redo apply, including restoration of removed source evidence after reopening.
Cloud edits, coordinate alignment and geometry undo do not move or refit the
historical measurement.

Snapshot schema 4 adds `cylinder`. The existing project envelope remains version
3/4 under its original measurement/history rules; the two version numbers are
independent. Prior snapshot schemas 1..3 continue to load. Older applications
without the new tag reject it instead of silently dropping source evidence.
Cylinder records are capped to 20000 source points within the existing 16-record,
compact metadata-history and 64 MiB project bounds.

Load/history validation checks finite fields, source/result counts, a unit axis,
positive radius and matching diameter, radial RMS/max ordering (with relative
rounding allowance), the finite nonnegative RMS gate, ordered finite axial range,
normalized Jacobian condition in [1,1e8), iterations 1..80 and converged starts
1..3. Physical qualification must remain false. This is structural fit validation;
it does not rerun the nonlinear optimizer, authenticate provenance or certify
that arbitrary rewritten model parameters describe the source. Retained source
coordinates allow explicit later refitting. See CYLINDER_FITTING.md for the actual
solver's stronger numerical convergence gates and its limitations.

**Show historical measurement outlines** draws two 64-segment rings at the
sampled axial endpoints and four connecting generators (132 segments per saved
cylinder). These complete analytic circles extrapolate beyond partial-wall
samples. They are not verified endcaps, surface meshes, object-length estimates
or newly acquired scan points. Labels identify a historical cylinder over its
sample span. Outlines use bounded, clipped rendering with the existing non-overlap
label placement; crowded labels can be omitted while panel values remain visible.
They do not participate in point picking or alter exports.

Offsets are projected separately from the saved axis point to avoid prematurely
adding small radii to distant world origins. Nonfinite projected segments are
skipped. The current cloud still supplies view bounds; empty clouds have no
projection and historical fits can lie outside the current view after edits.

Evidence: `evidence/cylinder-snapshots-{tests,clippy,release}.log`, synthetic
`cylinder-snapshot-preview.orproj` (reproduce with Rust example
`cylinder_snapshot_fixture NEW_PROJECT`), and `cylinder-snapshots-render/`.
Tests cover saved/deleted record restoration, geometry/metadata undo independence,
project reopening, schema/parameter/gate rejection, CRC-valid invalid diagnostics,
worker cancellation/duplicate labels/preserved source and 132-segment model
geometry across tilted axes and distant origins. Native rendering checks the
reopened tilted 6 mm cylinder's record and historical outline. This does not
qualify full mouse interaction, reference algorithms or physical scanner accuracy.
