# Historical fit outlines

The point-cloud workbench has an optional “Show historical measurement outlines
(purple)” checkbox, initially disabled. It draws the stored analytic circle or
three sphere great circles in the snapshot's original coordinates and labels
the center as a historical fit. These complete analytic rings extrapolate beyond
the selected evidence; they are not acquired surface points or a qualification
of physical accuracy. Geometry edits do not move or refit them.

The overlay uses the current cloud's view bounds, with 64 segments per ring and
at most 16 records (3,072 segments). Off-screen lines are clipped. No cloud means
no projection; fits outside current bounds may be invisible until the view is
adjusted. Outlines are not selectable and do not alter point picking, project
metadata, undo history or exports. They can temporarily disappear while a worker
owns the document. Labels now use bounded non-overlapping placement, wrapping and leader lines.
Crowded viewport labels can be omitted; the panel retains their values.

Projection accepts a separate model offset, avoiding premature addition of a
small radius to a distant world origin. Nonfinite projected segments are skipped.
The model values come from validated project snapshots. No tessellated sphere
surface, fit residual overlay or hardware accuracy claim is provided.

Run `scancraft-app --measurement-outlines PROJECT_OR_PLY_FILE` to open this view
without camera acquisition. The private-display render tool supports the same
flag. Evidence: `evidence/measurement-outlines-{tests,clippy,release}.log` and
`evidence/measurement-outlines-render/`. Analytical tests check circle planarity
and radius, three closed sphere rings, ordinary projection equivalence and small
offsets around a distant origin. The synthetic sphere render was visually
inspected; this does not qualify a full interactive workflow or GPU performance.

Saved distance segments and angle arms are also supported; see
DIRECT_MEASUREMENT_SNAPSHOTS.md for their evidence and validation semantics.

Historical cylinders draw two sample-span rings and four generators (132
segments), with explicit extrapolation/qualification limits; see
CYLINDER_SNAPSHOTS.md. The existing global 3,072-segment bound still applies.
