# Mesh boundary inspection

```
scancraft-cli mesh-boundaries MESH_JSON_OR_PROJECT
scancraft-app --mesh-boundaries MESH_JSON_OR_PROJECT
```

The native mesh workbench also offers **Boundary contours → Analyze boundaries**.
Analysis runs on a background worker with cancellation and progress. Select a
contour to highlight its indexed edges in yellow. Highlighting shows through
surfaces and is limited to 10,000 sampled edges for large contours. The full
analysis is retained; the contour list is virtualized. During an unapplied edit
preview the saved-mesh overlay is hidden. Opening/applying a different version
or undoing/redoing clears analysis and selection, preventing stale vertex IDs.

This is inspection, not hole filling. An outer border, an intentional opening,
and a missing surface patch can all yield a boundary contour. No opening is
automatically classified, capped, welded or otherwise changed. Explicit
planarity-bounded filling is now documented in `MESH_FILLING.md`; nonplanar
filling and general repair remain unfinished.

## Definition and guarantees

The independent Rust geometry module validates the mesh and reports existing
topology warnings. Edges incident to exactly one indexed triangle form a
boundary graph. Coordinates are never welded by proximity. A connected graph
becomes a contour only when each vertex has undirected degree two and exactly
one incoming/outgoing boundary edge. Branching graphs and direction conflicts
are returned separately, preserving their counts and first vertex ID.

Contours start at their lowest vertex ID and follow the existing face-edge
orientation, with the final-to-first closing edge implicit. IDs follow ordered
indexed-edge traversal and are deterministic under triangle-order changes for
the same oriented geometry. IDs are derived data for that mesh, not persistent
identities across edits. Results include ordered vertices, 3D polyline perimeter
in millimeters and coordinate bounds. Nonfinite perimeter calculations fail.

A simple indexed cycle need not be planar or spatially simple. This operation
does not test contour self-intersection, distinguish inner/outer loops,
prove volumetric watertightness or establish physical
measurement accuracy. Existing duplicate-face, nonmanifold-edge and winding
warnings remain visible in the result. They are not silently repaired.
Indexed vertex-link diagnostics are now also included; see `MESH_VERTEX_TOPOLOGY.md`.

Input uses the existing mesh bounds (one million vertices/two million triangles)
and strict versioned mesh/project loaders. Boundary output is limited to one
million edges and 100,000 connected boundary graphs, with explicit failure on
excess. Edge construction is ordered-map based; connected-graph traversal
expands each vertex once, avoiding quadratic behavior at high-degree branches.
No geometry or history is modified, and CLI output is a single JSON document.

## Evidence

The redistributable synthetic ring in `evidence/mesh-boundary-ring-input.json`
has eight triangles, an outer 40 mm contour and an inner 16 mm contour. The
CLI report is `evidence/mesh-boundary-ring-report.json`. The CLI integration
test checks both mesh JSON and a checksummed history project, exact agreement
of their reports, byte-preserved inputs and rejected unknown units.

Geometry tests cover ring orientations/perimeters/bounds, deterministic results
after face reordering, a closed tetrahedron, a removed tetrahedron face,
vertex-touching contours, inconsistent winding, cancellation, invalid numeric
input and overflowing perimeters. A 2,000-patch vertex branch remains one
ambiguous graph, not 2,000 arbitrarily chosen holes. The GUI test runs analysis
through its worker, checks emitted yellow edges and unchanged geometry/history,
then verifies invalidation on undo, redo and reload. Startup argument parsing
also covers the new command.

`tools/check_workbench_render.py ... --mesh-boundaries` captured the released
app on a separate authenticated Xvfb display, requesting software rendering.
The image was visually inspected: both contours and expected perimeters are
listed, the outer contour is highlighted, and the inner opening is preserved.
No scanner device file was open in the test process. This is startup/render
evidence, not an interactive workflow or hardware qualification.

![Synthetic ring boundary inspection](../evidence/mesh-boundary-render.png)

Capture scope/hash: `evidence/mesh-boundary-render-summary.json`.
All 236 tests across workspace targets pass; warning-denying Clippy, formatting
and the release workspace build pass (`mesh-boundary-*` logs). Physical scan
geometry/calibration qualification and general mesh repair remain unfinished.
