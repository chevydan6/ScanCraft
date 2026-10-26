# Explicit planar boundary filling

```
scancraft-cli mesh-boundaries MESH_OR_PROJECT
scancraft-cli fill-mesh-boundary MESH_OR_PROJECT BOUNDARY_ID MAX_PLANARITY_MM NEW_RESULT_FOLDER
```

In the mesh workbench, analyze boundaries, select the intended contour, set the
planarity tolerance and choose **Preview fill of selected contour**. Review the
unapplied surface, then use **Apply and save edit** with a new result folder.
Discard leaves the current mesh unchanged. Apply creates an original-preserving
bundle and checksummed history project; undo/redo and history reopening preserve
the exact previous vertices and triangles. Existing output folders are refused.
CLI and GUI use the same preparation/publication implementation, including
source-change checks for GUI file-backed edits.

Contour selection is explicit. Outer borders and intentional openings are not
automatically classified as holes. This operation fills one contour at a time;
it never moves, flattens, welds, deletes or creates vertices. It adds triangles
using only the selected contour's existing vertices, with reversed boundary-edge
orientation to match neighboring faces. The report records the added-face count,
selected ID, requested planarity tolerance and measured maximum plane deviation.
Boundary IDs must be reanalyzed after edits.

## Supported geometry and refusals

An oriented-area contour plane supplies a local orthonormal projection, scaled
by the contour's coordinate extent. Maximum deviation from that plane must fit
the caller's finite nonnegative tolerance (GUI default 0.01 mm). This is not a
least-squares plane fit or physical accuracy qualification. Nonzero tolerance
permits correspondingly nonplanar input; original coordinates stay fixed.

The projected contour must be simple and numerically well conditioned. The
native ear-clipping triangulator supports concave contours, checks inclusive
point containment and refuses unstable/degenerate ears. Coplanar positive-area
overlap with existing triangles is rejected. For noncoplanar triangles, plane
sections entering a cap triangle's interior are rejected; zero-area boundary
contacts remain permitted. Comparisons use a normalized 1e-12 tolerance, with
finite-arithmetic checks. These are floating-point predicates, not a general
exact-predicate mesh Boolean or self-intersection repair engine. Interactions
below numerical tolerance, nonmanifold vertex contacts and volumetric validity
are not qualified by this operation.

Duplicate faces, nonmanifold edges/vertices, conflicting edge winding or ambiguous
boundary graphs must be resolved first. Contours are limited to 3..512 edges,
geometric comparisons to 10 million, and result triangles to the existing
two-million limit. Cancellation/limits/refusal return no partial mesh or output
bundle. Final validation checks triangle validity, consistent edge winding,
duplicate/nonmanifold edges and the expected reduction in boundary edges.
New internal cap edges pair consistently; other openings remain unchanged.

## Verification and limits

The synthetic ring's inner contour (ID 1) adds two triangles, reducing boundary
edges from eight to four while preserving all eight vertices. Its outer contour
is refused because a cap would overlap existing faces. Additional analytical
tests cover a concave L-shaped prism cap (four triangles, area 3 mm²), a missing
tetrahedron face, nonplanarity, invalid tolerance, crossing geometry, segment
contacts and cancellation. CLI tests verify unchanged source bytes, exact undo
after project reopening, overlap refusal without publication and no overwrites.
The GUI worker test covers preview/current separation, apply, cache invalidation,
exact undo/redo and history reopening. A test-only mutable-constant warning was
also corrected while running all targets.

Actual release CLI output for the ring is retained in
`evidence/mesh-fill-ring-report.json`, `mesh-fill-ring-result.json` and
`mesh-fill-ring-publication.json`. The released app was opened on that result
using a separate authenticated Xvfb display. The inspected image shows the
center filled, the outer boundary preserved, 8 vertices/10 triangles and the
explicit fill control. No scanner device descriptor was open. This screenshot
is startup/render evidence; interactive preview/apply/history behavior is
covered by the GUI test rather than claimed from the screenshot.

![Explicit synthetic ring fill](../evidence/mesh-fill-render.png)

Capture scope/hash: `evidence/mesh-fill-render-summary.json`.
All 242 tests across workspace targets, warning-denying Clippy, formatting and
the release build pass (`mesh-fill-*` logs). General nonplanar filling, broader
mesh repair, genuine calibrated scanning and physical qualification remain open.
