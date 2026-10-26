# Constrained native mesh simplification

The mesh workbench now provides **Mesh simplification**. Set a target maximum
triangle count, maximum source-vertex movement (mm), maximum surviving face-normal
change (degrees), source-crease protection threshold (degrees) and collapse budget.
Click **Preview simplification**, inspect the unapplied result, then use **Apply
and save edit** or **Discard preview** in Mesh edits. Geometry changes only after
successful publication of a new result bundle. Existing undo/redo and saved mesh
history restore exact source versions, including after reopening.

The preview reports original/result counts, accepted collapses, maximum observed
source-vertex movement and whether the requested count was reached. Constraints
can prevent the target; the algorithm returns a valid constrained intermediate
result with an explicit stop reason. It never forces additional collapses just
to meet the requested count. Render sampling is separate: simplification uses the
full indexed input within its current limits, while the viewport can sample at
most 50,000 triangles.

## Geometry and topology policy

The independent Rust implementation uses the general
[edge-collapse simplification model](https://doc.cgal.org/latest/Surface_mesh_simplification/)
with a shortest-current-edge priority and deterministic indexed tie breaking.
Endpoint/midpoint placements are considered under these constraints:

- All source boundary vertices and vertices of source edges whose dihedral angle
  exceeds the crease threshold are locked. Protected edges must remain present.
  Original boundary edge graphs are checked after remapping, not just counted.
- Every original vertex in either merged cluster must remain within the chosen
  displacement of the proposed replacement. This bounds cumulative movement
  against source coordinates, rather than allowing repeated per-step drift.
- Surviving faces must stay nondegenerate and remain within the chosen angle of
  their original face normals. Dot comparisons include a 1e-12 rounding allowance.
- Interior edge incidence, common-neighbor/link constraints and duplicate-face
  rejection protect indexed connectivity. Tetrahedral component collapses that
  would create duplicate faces are rejected.
- Result manifold/winding/duplicate checks, component count, used-vertex Euler
  characteristic, protected boundary graph and all source-face images are checked
  before return. Every source face must map to a result face, edge or used vertex.
  Unused original vertices are retained unchanged; there is no implicit cleanup.

Input duplicate faces, inconsistent winding or nonmanifold edges/vertex fans must
be repaired explicitly first. Coincident positions with separate indices stay
separate; simplification does not proximity-weld components or fill openings.
The source remains immutable and a complete source-to-result vertex map is
returned for auditing the displacement checks.

These guards do not establish global absence of self-intersections, a certified
closest-surface/Hausdorff error, volume preservation, thin-feature accuracy or
physical dimensional qualification. Crease locking uses the original source
edges; it does not dynamically classify every newly formed edge as a feature.
The current mesh representation is indexed positions/triangles, with face normals
recomputed for display/export. Independent decimator comparisons, richer attribute
handling and wider geometric qualification remain open.

## Bounds and cancellation

Current CPU limits are 100,000 vertices, 200,000 triangles, 100,000 accepted
collapses, 1,000,000 candidate/stale events and 1,000,000 queued entries. Local
candidate neighborhoods exceeding 256 incident faces are skipped before building
larger candidate sets. The returned stop reason distinguishes target, constraints
and collapse/candidate/queue budgets. Cancellation is checked throughout input,
indexing, candidate work, cluster checks, output validation and publication.
Cancelled/failed preparation never replaces the current source. Large-data
performance and memory qualification remain unfinished.

## Files and CLI

```
scancraft-cli simplify-mesh MESH_OR_PROJECT TARGET_TRIANGLES MAX_DISPLACEMENT_MM MAX_NORMAL_DEGREES CREASE_DEGREES MAX_COLLAPSES NEW_OUTPUT_DIRECTORY
scancraft-app --mesh-simplify-preview MESH_OR_PROJECT TARGET_TRIANGLES MAX_DISPLACEMENT_MM MAX_NORMAL_DEGREES CREASE_DEGREES MAX_COLLAPSES
```

The app startup option creates an unapplied preview, with no acquisition or
publication. The CLI publishes a new bundle; successful publication can still
have `simplification.target_reached=false`, so inspect that field and its stop
reason. Existing destinations are never overwritten. The bundle retains exact
original input bytes, configuration, stop/rejection diagnostics, source-to-output
map, mesh JSON, available STL/OBJ exports and a checksummed editable mesh project.
Parameters and lineage live in `report.json`; the mesh project retains geometry
undo/redo, not a parametric operation stack. Physical/watertightness qualification
flags remain false. Full GUI history can be saved separately as before.

## Verified scope

Evidence: `evidence/mesh-simplify-{tests,clippy,release}.log`, synthetic
`mesh-simplify-grid-input.json`, `mesh-simplify-example/`,
`mesh-simplify-cli-checks.json`, and native final-preview/reopen render directories.
The 8x8 planar grid decreases from 98 to 70 triangles after 14 collapses, retains
all 28 boundary edges and has maximum source-vertex movement 0.7071067812 mm under
an explicitly selected 1.5 mm bound. That loose demonstration setting is not an
approved scanner tolerance; the planar source surface itself stays planar.

Nine geometry regressions cover exact boundaries/cumulative movement, protected
creases/zero movement, closed topology and work budgets, tetrahedral rejection,
invalid/cancelled input, cumulative source-normal limits, an inner opening plus
unused vertex, coincident components, and cancellation after processing begins.
Two workflow tests cover byte-exact original preservation, no overwrite,
publication cancellation and reopened exact undo/redo. A GUI worker test exercises
preview, apply/save, undo/redo and project reopening while the source file stays
unchanged. All 382 Rust tests, strict lint, formatting and release build pass.
Native screenshots check preview and reopened saved geometry, not a full mouse
workflow or physical scanner qualification.
