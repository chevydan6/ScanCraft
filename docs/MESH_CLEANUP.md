# Selective reversible mesh cleanup

The Meshes pane offers separate **Remove duplicate faces** and **Remove unused
vertices** options under Mesh edits. Both start disabled. Select the desired
operations, choose **Preview cleanup**, inspect proposed topology/counts and
then apply/save to a new result folder or discard. Existing source-change
checks, cancellation, immutable preview, original preservation, undo/redo and
full-history save/reopen apply to cleanup too. A saved version is committed only
after result publication succeeds.

The CLI uses the same preparation/publication pipeline:

```
scancraft-cli cleanup-mesh MESH_OR_PROJECT duplicates NEW_RESULT
scancraft-cli cleanup-mesh MESH_OR_PROJECT unused NEW_RESULT
scancraft-cli cleanup-mesh MESH_OR_PROJECT both NEW_RESULT
```

Input can be strict mesh JSON or MSHPRJ01 history. Project input continues the
stored edit chain. Output retains original bytes, current mesh JSON, applicable
STL/OBJ, a checksummed editable history project, topology/operation diagnostics
and integrity manifest. Existing targets are never overwritten.

Duplicate identity is the same set of three **vertex indices**, irrespective
of winding or cyclic order. The first source face and its original winding are
retained; later duplicates are removed in source order. This can remove an
intentional oppositely wound face, so inspect the explicit preview before
applying. Coincident faces using different vertex indices are not duplicates
under this operation. No positional tolerance, welding or seam inference occurs.
Unused-vertex removal compacts only vertices unreferenced by retained triangles,
preserving surviving coordinate values and order while remapping indices.
These position-only meshes do not currently carry per-vertex color/material
attributes. No positions are moved, normals smoothed, components removed,
nonmanifold edges split, holes filled or watertightness certified.

At least one operation must be selected. Existing invalid/nonfinite/degenerate
mesh inputs fail validation; cleanup is not a repair path for arbitrary invalid
geometry. Empty retained surfaces remain valid explicit results. Counts report
exact removed faces/vertices, even when zero. Existing one-million-vertex/two-
million-triangle and history/payload limits remain. Dense topology/hash-map
memory and large-model performance are not qualified by these tests.

Two geometry regressions cover first-winding/coordinate preservation, independent
options, exact undo, invalid/empty-option/cancelled cases. A CLI regression covers
option-specific output, original bytes, chained project history, exact restoration,
unknown options and no overwrite. A GUI controller regression covers unapplied
preview, topology changes, durable apply, exact undo/redo and persisted original
history. All 208 workspace tests, Clippy, formatting and release build pass.

The release example adds two duplicate faces and one unused vertex to the
synthetic stereo surface: 94 vertices/148 triangles become 93/146. The cleaned
mesh JSON is byte-identical to the original clean stereo mesh; the deliberately
defective input remains byte-identical in the new bundle and its exact geometry
is in persisted undo. Artifact CRCs and lengths were verified. Actual native
result rendering was visually inspected on a private display with no scanner
handles (`evidence/mesh-cleanup-render-summary.json`). This is synthetic cleanup,
controller and render evidence, not full pointer workflow, general repair or
physical geometry qualification.

Disconnected faces can now also be analyzed and selected explicitly by shared-
edge component ID. This is a separate operation with no automatic size threshold;
see `MESH_COMPONENTS.md`. Duplicate/unused cleanup semantics remain unchanged.
