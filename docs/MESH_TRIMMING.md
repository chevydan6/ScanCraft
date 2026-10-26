# Native reversible plane trimming

```
scancraft-cli trim-mesh INPUT_MESH_JSON NX NY NZ OFFSET NEW_RESULT_DIRECTORY
```

The half-space `normal dot position + offset <= 0` is retained. For example,
normal 1,0,0 and offset -0.5 keeps x<=0.5 mm. The plane coefficients are
normalized together internally; offset is a geometric signed distance only
when the supplied normal is unit length. Coordinates use the input's explicit
millimeter frame. Invalid/nonfinite or near-zero normals and numerical overflow
fail before output publication.

The native `mesh_edit::trim_plane` primitive clips each source triangle,
introducing intersection vertices rather than dropping an entire crossing face.
An undirected source-edge cache shares each crossing vertex between incident
triangles. Exact on-plane source vertices are retained without creating duplicate
crossings. Triangle winding is preserved; unused vertices are compacted after
clipping. The input is immutable. The cut stays open: no cap or automatic hole
filling is performed, and discarded isolated vertices are recoverable from the
retained original. Intersections use scaled signed-distance ratios to avoid
subtraction overflow. Ill-conditioned new slivers are rejected and counted using
the existing triangle-normal criterion; inspect that count when precision or
small features matter. Floating-point intersection coordinates have numerical
rounding and are not physically qualified.

Mesh vertex/triangle and intermediate budgets remain one million/two million.
An intermediate intersection allocation can exceed the vertex budget before
compaction even when a final compact mesh would fit; that fails explicitly.
Cancellation or an invalid proposal returns no partially accepted geometry.

`MeshEditor` provides exact whole-mesh undo/redo, bounded to 1..32 historical
states. Total retained coordinate/index payload, including current state and
undo/redo, is capped at 128 MiB; capacity overhead/allocator behavior is outside
that accounting. New edits clear redo and evict oldest undo states as necessary;
the commit result explicitly reports evictions. Undo/redo swaps existing exact
snapshots, so their retained payload stays constant. Invalid/cancelled proposals
do not change mesh or history. The subsequent MSHPRJ01 format now persists this backend history; GUI navigation integration remains unfinished.

The command writes a new no-overwrite bundle with:

- `original.mesh.json`: byte-for-byte input source, for exact restoration.
- `mesh.json`: edited mesh, including a valid empty result when fully removed.
- `project.meshproj`: current mesh and backend undo/redo snapshots.
- `mesh.stl` / `mesh.obj`: edited exports when surface triangles remain.
- `report.json`: plane, counts, slivers, original/result topology and limitations.
- `manifest.json`: output sizes and CRCs.

Original files are never overwritten. Invalid input and precommit cancellation
leave no published target; write failures report a retained staging directory.
CLI bundles now preserve the original and serialize backend history. Project input retains prior backend edits; see `MESH_PROJECTS.md`.
Open either mesh JSON in the native Meshes pane to inspect it. The current GUI
pane remains read-only; interactive plane preview, applying edits, undo/redo
buttons and saving editable mesh history are not implemented yet.

Four geometry regressions cover analytic square half-area, shared cut edges,
oblique tetrahedron cuts with an open boundary, on-plane/empty/invalid/cancelled
cases, and exact restoration across 100 undo/redo cycles plus redo invalidation
and bounded history. A CLI regression checks split geometry, original-source
bytes, topology, output integrity, no overwrite and failed/cancelled publication.
All 197 workspace tests, Clippy, formatting and release workspace build pass.

A native release command cut the synthetic stereo surface at x<=0: 93 vertices/
146 triangles became 62 vertices/90 triangles, with 13 cut source edges. Original
bytes and artifact CRCs were checked. The result was visually inspected in the
native mesh viewer on a separate authenticated display without scanner handles.
Evidence: mesh-trim test/build logs and example/render summary JSON. This is
synthetic backend/CLI/render validation, not interactive GUI editing or physical
surface accuracy qualification.

## Native GUI preview, apply and saved-version undo

The Meshes pane now provides a Plane trim panel. Enter the normal/offset,
choose **Preview trim**, inspect the highlighted UNAPPLIED preview and topology,
then either **Discard preview** or supply a new result folder and select
**Apply and save trim**. Plane inputs/opening another mesh are disabled while a
preview is pending, so Apply cannot accidentally use parameters different from
the displayed proposal. Current model/history stay unchanged during preview.

Preparation and publication are now separate shared workflow stages. The worker
checks that disk geometry still equals the loaded source, builds the proposal
and render data, and retains them immutably. Apply publishes the original-
preserving bundle before committing the displayed version. No post-publication
geometry loading is needed. Failed/cancelled precommit publication retains the
current model; the failed proposal is discarded and can be previewed again.
A sync error after rename can leave a published bundle; its path is explicitly
reported and the current model is retained. No files are overwritten.

**Undo mesh trim** and **Redo mesh trim** swap exact immutable saved-version
models without rereading mutable files. New edits clear redo. History is bounded
to 32 steps and approximately 128 MiB retained coordinate/index/preview payload;
evicted step counts are reported. Saved bundles remain on disk after history
eviction. Opening another mesh resets this in-memory navigation history.
The separate GUI navigation stack is not yet serialized into the new backend mesh-project format: restart preserves saved files, but not that GUI navigation stack. Closing an unapplied preview changes no saved
geometry; closing during work requests cancellation. Point-cloud unsaved-work
protection remains independent and unchanged.

A fresh app can review a plane without writing a result:

```
scancraft-app --mesh-plane-preview MESH_JSON NX NY NZ OFFSET
```

Two new controller regressions exercise asynchronous preview/apply, exact saved
output, 100 GUI undo/redo cycles, existing-target failure retention, source-change
rejection and bounded history. Existing render/startup/CLI tests still pass.
All 199 workspace tests, Clippy, formatting and release build pass. An actual
private-display release run shows an unapplied x<=0 preview of the synthetic
stereo surface (62 vertices/90 triangles), with source bytes unchanged and no
trim result published. This verifies preview rendering and controller behavior,
not all pointer interactions or physical accuracy. See gui-mesh-trim evidence.

The GUI now also provides **Save full mesh history** and opens `.meshproj`
files with all saved current/undo/redo snapshots. This supersedes the earlier
GUI-navigation persistence limitation above. Use that explicit full-history
save before restart; see `MESH_PROJECTS.md` for self-contained snapshots,
load bounds, continuation and verification scope.

The shared preview/apply/history path also supports separately selected duplicate-
face and unused-vertex cleanup. See `MESH_CLEANUP.md` for exact identity rules,
retained winding, limitations and verification. General cleanup does not alter
plane clipping semantics or automatically cap openings.
