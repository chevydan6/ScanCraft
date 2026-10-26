# Explicit disconnected vertex-fan splitting

```
scancraft-cli mesh-topology MESH_OR_PROJECT
scancraft-cli split-mesh-vertex-fans MESH_OR_PROJECT 0,12 NEW_RESULT_FOLDER
```

In the mesh workbench, enter explicit IDs under **Vertex IDs to split** and
choose **Preview disconnected-fan split**. Inspect the unapplied counts and
surface, then apply/save to a new folder or discard. The example IDs above are
placeholders; get current IDs from topology diagnostics. The GUI clears split
selections whenever the current mesh version changes. IDs are not persistent
selection identities across arbitrary edits.

The operation addresses one particular indexed topology defect: disconnected
triangle fans sharing a vertex. The first incident fan in face order keeps the
original index. Each other fan receives an appended vertex with a bit-exact copy
of the same coordinates; only corners belonging to that fan change index.
Explicit vertex selections are processed deterministically, independent of the
selection's order. The report lists each original ID, resulting fan vertex IDs
and first triangles, together with the number of copies added.

Original vertex values, per-corner face coordinates, triangle count, face order,
winding, boundary-edge count, winding-conflict count and isolated-vertex count
stay unchanged. Unselected defects must remain unchanged. The selected original
and copied vertices must pass vertex-link checks afterward. No triangle is
deleted, and no smoothing, positional separation, gap creation or welding occurs.
The surfaces may still touch geometrically; indexed manifold classification
does not establish embedding validity, watertightness or physical scan accuracy.

## Supported inputs and persistence

Every selected ID must be in range and have multiple disconnected fans. Empty,
duplicate and regular/isolated vertex selections fail. Nonmanifold edges and
duplicate faces require separate repair first. At most 100,000 explicit IDs
are accepted, subject to the existing one-million-vertex/two-million-triangle
mesh bounds. Copies are counted before cloning the source; exceeding the vertex
budget fails without a partial result. Cancellation checks cover indexing,
root extraction, remapping and final validation. Other unused vertices are
preserved rather than removed to make room silently.

CLI/GUI share immutable preparation and atomic no-overwrite publication. Source
bytes are retained, the GUI checks that file-backed source geometry still
matches the loaded model, and result bundles include the checksummed mesh
history. Exact undo/redo and project reopening restore the original mesh.
Native project/mesh JSON and OBJ carry the separate vertex indices. STL stores
triangle geometry without shared-vertex IDs: in the regression fixture its
bytes are identical before/after the split. An importer that welds coincident
coordinates may reintroduce the index contact, even with an indexed format.
Use the native history/project to retain this edit unambiguously.

## Evidence and remaining work

The synthetic pinched tetrahedra gain one coordinate copy (seven vertices become
eight), retain eight faces at exactly the same positions, and change from one
nonmanifold vertex to zero. Original vertex 0 retains its first fan, while the
other uses vertex 7. The result is an indexed closed-manifold candidate; it is
not qualified as watertight or geometrically separated. Release CLI outputs:
`evidence/mesh-fan-split-report.json`, `mesh-fan-split-result.json`,
`mesh-fan-split-topology.json` and `mesh-fan-split-publication.json`.

Geometry tests check per-corner f64 bits including negative zero, deterministic
selection ordering, unchanged unselected defects, open bowtie contours, duplicate/
nonmanifold-edge refusal, the million-vertex limit, invalid IDs and cancellation.
They also check identical STL geometry and a distinct indexed OBJ face. A
composition test repairs a pinched component and then permits a separately
selected ring fill that was refused before the repair. GUI and CLI tests cover
preview/current separation, source preservation, apply, exact undo/redo,
checksummed history reopening, failed publication and no overwrites.

![Indexed fan split with unchanged touching geometry](../evidence/mesh-fan-split-render.png)

The private authenticated Xvfb render was visually inspected: eight vertices,
eight faces, no red topology marker and unchanged touching surfaces. No scanner
device descriptor was open. This is release-result startup/render evidence;
interactive edit/history behavior is covered by tests, not inferred from the
screenshot. Capture scope/hash: `mesh-fan-split-render-summary.json`.
All 261 tests across targets, warning-denying Clippy, formatting and the release
build pass (`mesh-fan-split-*` logs). Nonmanifold-edge repair, geometric contact/
intersection repair and genuine calibrated scanning remain unfinished.
