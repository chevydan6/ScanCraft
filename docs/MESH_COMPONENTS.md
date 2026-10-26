# Explicit disconnected-component selection

Mesh component analysis groups triangles sharing an **indexed edge**. Two faces
must share the same pair of vertex indices to connect; winding is irrelevant.
Vertex-only contact, coincident geometry with different indices and physical
proximity do not connect components. Nonmanifold shared edges connect all their
incident faces, without repairing them. Isolated vertices have no face component.
This is topology analysis, not semantic part recognition or positional welding.

IDs are assigned in first-source-triangle order, independent of component size.
Each summary gives ID, first triangle, triangle count and exact coordinate bounds
in the input's millimeter frame. Reanalyze after geometry/index changes; IDs are
not persistent semantic identifiers. No largest-component choice or size-based
fragment removal is automatic.

In **Meshes → Disconnected mesh components**, choose **Analyze components**,
review the virtualized summary list, enter explicit comma-separated IDs and
choose **Keep IDs** or **Remove IDs**. **Preview component edit** creates an
unapplied proposed surface. Use the shared Apply and save edit / Discard preview
controls in Mesh edits. A new model invalidates old analysis; previews do not
change the current model. Job/cancellation, source-change checks, original-
preserving publication, undo/redo and full history save/reopen remain shared
with other mesh edits. Component analysis and preparation run off the UI thread.
Long summary rows truncate visually with full bounds on hover.

CLI equivalents:

```
scancraft-cli mesh-components MESH_OR_PROJECT
scancraft-cli select-mesh-components MESH_OR_PROJECT keep 0,2 NEW_RESULT
scancraft-cli select-mesh-components MESH_OR_PROJECT remove 1 NEW_RESULT
```

Read-only analysis emits all component summaries. Selection accepts mesh JSON
or MSHPRJ01 history, preserves source bytes and persists the new exact edit.
Duplicate, unavailable, empty or malformed ID selections fail rather than being
silently reduced. Selected faces retain source order; surviving vertex values
and order are preserved while unused vertices are compacted. Keep/remove acts
on whole components, not arbitrary faces; an attached small feature sharing an
edge cannot be selected separately by this tool. Removing all components yields
an explicit empty mesh. No caps, smoothing, welding or inferred repair occurs.

Analysis uses a deterministic union/find over indexed edges. Existing mesh
vertex/triangle limits remain; a separate 100,000-component cap fails explicitly
without discarding components. Edge indexing can use substantial memory on large
meshes; memory/runtime and million-triangle interaction are not qualified. UI
summaries use virtual rows rather than silently omitting later IDs. Results do
not prove watertightness, vertex manifoldness or physical accuracy.

Three geometry regressions verify shared-edge versus vertex-only/coincident
connectivity, stable IDs/bounds, exact keep/remove/compaction, retained small
components, empty selection results, undo and invalid/cancelled requests. CLI
coverage checks read-only summaries, explicit keep/remove, original bytes,
persisted undo, invalid IDs and no overwrite. A GUI-controller regression checks
analysis, keeping a one-face component independent of a larger component,
unapplied preview, durable apply, exact undo/redo and analysis invalidation.
All 213 workspace tests, Clippy, formatting and release build pass.

Actual release commands analyzed the synthetic stereo surface plus a separate
one-triangle fragment: IDs 0/1 contain 146/1 triangles. Keeping ID 1 produced
exactly that three-vertex/one-triangle surface. Removing ID 1 restored the clean
stereo mesh JSON byte-for-byte. Original bytes, persisted undo and artifact CRCs
were checked. The kept small component rendered in the native app on a private
display, with no scanner handles. Evidence: mesh-components analysis/example/
render summary JSON and test/build logs. This verifies synthetic topology,
controller and result rendering, not all pointer interactions, general repair,
large-data performance or genuine Raptor geometry.
