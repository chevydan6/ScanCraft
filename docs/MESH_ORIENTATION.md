# Consistent face winding and normal inspection

```
scancraft-cli orient-mesh MESH_OR_PROJECT none NEW_RESULT_FOLDER
scancraft-cli orient-mesh MESH_OR_PROJECT 0,2 NEW_RESULT_FOLDER
scancraft-app --mesh-normals MESH_OR_PROJECT
```

The first command makes adjacent indexed triangles wind consistently while
preserving each connected component's first triangle. The second also reverses
the selected components after establishing consistency. IDs match the existing
indexed-edge component analysis, in first-triangle order. `none` means no explicit
component reversal; it does not disable consistency repair.

In the mesh workbench, leave **Reverse component IDs** empty to preserve each
component's first-face sign, or enter explicit IDs from component analysis.
Choose **Preview consistent winding**, inspect the unapplied result and normal
arrows, then apply/save to a new folder or discard. Normal arrows turn on for
the preview. Outward direction is not inferred: a component can be internally
consistent and still face inward, and open surfaces have no unique volume-based
outward convention. The report explicitly leaves outward direction unverified.
Reversal selections and cached topology IDs clear when the mesh version changes.

## Native geometry and persistence

`mesh_orientation::orient` validates the source and builds adjacency from edges
with at most two incident faces. Equal original edge directions require opposite
flip parity; opposite directions require equal parity. Breadth-first propagation
anchors the first face of each component. Conflicting parity constraints cause
refusal instead of a partial repair. Duplicate faces and nonmanifold edges also
require separate repair first. Vertex-only contacts remain separate components.

The operation preserves vertex values, vertex IDs, triangle count, face order,
openings and connectivity. Only a triangle's second/third indices are swapped.
STL and viewport facet normals are recomputed from the resulting winding.
There is no proximity welding, surface smoothing, coordinate approximation,
automatic hole filling or global sign choice. This does not repair nonmanifold
vertices, self-intersections, overlapping shells or volumetric validity, and
does not establish watertightness or physical scan accuracy.

Each triangle has at most three manifold neighbors, stored in fixed-size
adjacency arrays. Traversal is bounded by the existing one-million-vertex,
two-million-triangle mesh limits and 100,000 components. Invalid/repeated
selection IDs, nonorientable constraints and cancellation fail without changing
the source. The parity solution is deterministic for fixed face ordering and
component seeds; reordering the source's first faces can change the chosen
global component signs.

GUI and CLI share immutable preparation and atomic no-overwrite publication.
Preview/current separation, unchanged original source bytes, exact undo/redo and
self-contained checksummed history reopening use the existing mesh-edit path.
Reports retain per-component seed/flip counts and the original/result topology.

## Normal arrows and evidence

The optional **Face normals** control draws orange arrows at sampled triangle
centers. At most 2,000 normals are drawn from the existing sampled mesh preview;
arrows show through surfaces and are a view aid, not a complete topology check.
The checkbox tooltip states that limit. Two-sided shading alone is unchanged by
an entire-component reversal, so arrows make that sign choice visible.

Analytical tests cover a conflicting two-triangle patch, idempotent repair,
explicit/disconnected component reversal, duplicate/nonmanifold refusal,
invalid IDs and cancellation. A triangulated Möbius strip is refused as
nonorientable while preserving the source. The GUI worker test verifies an
unapplied preview, emitted normal-arrow shapes, unchanged coordinates/current
mesh, apply, exact undo/redo and history reopening. CLI tests verify reports,
source preservation, explicit reversal, no overwrites and no output on bad IDs.

`evidence/mesh-orientation-input.json` is a synthetic ring with one reversed
face. The actual release CLI changes one face and removes its two winding
conflicts, preserving all eight vertices/triangles and both openings. Explicit
component reversal instead changes seven original faces; outward direction
remains the user's choice. Publication, report and result are retained in the
`mesh-orientation-*` evidence files.

![Synthetic consistent winding with face-normal arrows](../evidence/mesh-orientation-render.png)

The released result was opened on a separate authenticated Xvfb display. The
inspected image shows the ring's eight sampled normal arrows, unchanged openings
and zero reported winding conflicts. No scanner device descriptor was open.
This is startup/render evidence, not full interactive or hardware qualification;
scope/hash are in `evidence/mesh-orientation-render-summary.json`.
All 247 tests across targets, warning-denying Clippy, formatting and the release
build pass (`mesh-orientation-*` logs). General mesh repair and genuine calibrated
scanning remain unfinished.
