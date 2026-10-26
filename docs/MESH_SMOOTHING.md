# Constrained native mesh smoothing

The mesh workbench provides **Mesh smoothing** with iterations, relaxation,
maximum source-vertex displacement (mm), maximum face-normal change (degrees),
and source crease protection threshold (degrees). Use **Preview smoothing**, then
inspect the unapplied geometry before **Apply and save edit** or **Discard preview**.
Publication uses a new destination; originals and saved geometry undo/redo remain
available, including after reopening the editable project.

## Algorithm and constraints

The independent Rust CPU implementation performs uniform Laplacian Jacobi steps.
For each unlocked vertex, compute the average of its current indexed neighbor
position differences, multiply by relaxation, and propose simultaneous updates.
Neighbors are unique by vertex index; coincident indexed parts are not welded.
Boundary vertices and both endpoints of source crease edges above the selected
dihedral threshold stay fixed. Unused vertices remain unchanged.

Each whole-mesh proposal must keep every vertex within the selected distance of
its original position and every face nondegenerate, with its normal within the
selected angle of its original normal. Normal dot comparisons allow 1e-12 rounding.
Bounds are cumulative against the source, rather than independent per-step limits.
If a proposal violates a constraint, halve the global step and retry up to 20 times.
Accept a valid smaller step or stop explicitly with `constraints`. No changed
floating-point positions yields `stationary`; completing all requested changing
steps yields `iterations_completed`. Reports record accepted iterations,
backtracking steps, locked vertices, maximum actual displacement and stop reason.
A constrained partial result can be published successfully; inspect the report.

Triangle indices, counts, winding order and vertex identities stay unchanged.
Duplicate faces, inconsistent winding, nonmanifold edges and invalid vertex fans
must be repaired explicitly first. Input/result coordinate and triangle validity
are checked. Work limits are 100k vertices, 200k faces and 100 iterations, with
cooperative cancellation during indexing, neighborhood work, trials and output.

Uniform Laplacian smoothing can shrink surfaces and attenuate small details.
These constraints do not certify volume preservation, global self-intersection
absence, Hausdorff distance, reconstruction quality or physical dimensional
accuracy. Locking uses source creases, not dynamically reclassified edges.
This is a whole-mesh operation; selected-region smoothing and attribute-aware
smoothing remain future work. Large-data speed/memory qualification is open.

## CLI and saved files

```
scancraft-cli smooth-mesh MESH_OR_PROJECT ITERATIONS RELAXATION MAX_DISPLACEMENT_MM MAX_NORMAL_DEGREES CREASE_DEGREES NEW_OUTPUT_DIRECTORY
scancraft-app --mesh-smooth-preview MESH_OR_PROJECT ITERATIONS RELAXATION MAX_MOVE_MM MAX_NORMAL_DEG CREASE_DEG
```

For example, the redistributable synthetic bump fixture uses `5 0.5 0.1 30 180`.
A 180-degree crease threshold disables crease locking for this demonstration;
boundary locking still applies. These parameters are not an approved scanner
accuracy tolerance. The app startup command only prepares an unapplied preview.
The CLI publishes an original-preserving bundle containing mesh JSON, STL/OBJ,
checksummed mesh project, parameters/diagnostics and manifest. Existing destinations
are rejected. Geometry history is saved, not a parametric operation stack.

## Evidence

`evidence/mesh-smooth-tests.log` records 390 passing Rust tests. New regressions
cover bump reduction, exact fixed boundary/index preservation, source displacement
and normal bounds, zero movement, crease protection, unused vertices, invalid
input, cancellation, an analytic one-step Jacobi result and translation behavior.
Workflow tests verify byte-exact original preservation, no overwrite, cancelled
publication and exact reopened undo/redo. The GUI worker test covers unapplied
preview, apply/save, undo/redo and project reopening without source changes.

`mesh-smooth-example/` retains a release CLI example: 25 vertices / 32 triangles,
16 locked boundary vertices, 5 accepted iterations, 19 backtracking steps and
0.09888668434 mm maximum source movement under a 0.1 mm limit.
`mesh-smooth-cli-checks.json` independently checks saved indices, boundary positions,
movement, source bytes and overwrite rejection. Lint/build logs and native
preview/reopen screenshots are alongside them. Screenshot inspection verifies
startup display, not a full mouse workflow or physical scanner qualification.
