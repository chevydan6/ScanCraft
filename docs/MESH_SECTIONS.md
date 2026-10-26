# Indexed mesh cross sections

The native geometry engine and CLI now extract triangle/plane section graphs:

```
scancraft-cli mesh-section MESH_OR_PROJECT OX OY OZ NX NY NZ TOLERANCE_MM
```

The origin and tolerance are millimeters; the nonzero normal is normalized
internally. Input is the existing strict mesh JSON envelope or mesh project.
Output is a schema-1 millimeter JSON report on stdout. The command is read-only;
redirect stdout to retain the report. It does not open cameras or invoke Wine.
The native workbench also supports section controls and overlays; see
GUI_MESH_SECTIONS.md. Reproducible one-section source bundles are described in
SECTION_SNAPSHOTS.md; integrated multi-section project entities remain open.

`scancraft_geometry::mesh_section::intersect` preserves shared-edge topology. Each
node names either an original vertex or an original indexed edge. Existing mesh
indices determine connectivity; coincident coordinates with separate indices are
not welded. Segments are undirected and duplicate indexed segments are counted
and emitted once. The source mesh is not changed.

Connected components with degree two everywhere become closed contours. A
component with exactly two degree-one endpoints and all other degrees two becomes
an open contour. Closed contour lists omit the repeated closing node. Other
components are reported as ambiguous graphs with their node and segment IDs;
branching paths are never arbitrarily joined. Lengths sum Euclidean segment
lengths, including one closing segment for a closed contour. Total segment length
also includes ambiguous components, each indexed segment once. These are not
cross-sectional area, hole classification, self-intersection validation or a
physical perimeter qualification. Traversal IDs are deterministic for the exact
indexed input and need not survive mesh edits or reindexing.

A vertex within the explicit tolerance band is treated as on-plane without moving
its stored coordinates. Consequently, a positive tolerance can produce contours
whose vertices are not exactly coplanar. Entirely coplanar triangles have no
unique line intersection: they are counted and omitted, while edges shared with
noncoplanar triangles can still appear. Single-vertex tangent triangle contacts
are counted but do not emit zero-length edges. Review these counters before
interpreting a section as complete.

Finite validation, source mesh budgets and cancellation apply. The section is
bounded to 1,000,000 segments, 2,000,000 nodes and 100,000 components. Nonfinite
planes, negative tolerances, unrepresentable coordinate differences/intersections
or lengths, and sections collapsed by coordinate rounding are rejected. Normal
normalization and edge interpolation use scaled arithmetic. This is a bounded CPU
implementation; large-dataset performance is not qualified.

Evidence: `evidence/mesh-section-{tests,clippy,release}.log`, the redistributable
unit cube `mesh-section-cube-input.json`, native CLI report
`mesh-section-cube-report.json` and `mesh-section-cli-checks.json`. Six analytical
tests cover a closed unit-square cut with 4 mm length, open sheets, duplicate
faces, unwelded coincident sheets, coplanar faces, tangent vertices, tolerance
classification, branching, invalid/cancelled input, scales 1e-100..1e150 and
distant origins. Release CLI checks verify the cube result, rejection of missing
parameters/zero normal/negative tolerance, and unchanged source SHA-256.
This is synthetic numerical evidence, not physical scanner qualification.
