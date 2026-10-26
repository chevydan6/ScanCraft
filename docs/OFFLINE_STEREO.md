
## Automatic textured sampling

Instead of supplying `seeds`, a schema-1 request can specify:

```json
"automatic_sampling": { "spacing": 8, "maximum_seeds": 100 }
```

Omit `seeds` or set it to `[]`; supplying nonempty explicit seeds together with
this option is an error. Without this option the existing manual seed behavior
remains. The same settings file works in CLI and GUI reconstruction workflows.

Sampling occurs on the rectified left image, after remapping. It visits a
row-major grid starting at the patch radius and retains patches with complete
valid support inside the calibrated lens domain and sufficient population
standard deviation, using the same patch rules as matching. Spacing is in
rectified pixels. Sampling stops at `maximum_seeds`; this deterministic cap
can favor earlier rows, so choose spacing and limits for the desired coverage.
This is textured grid sampling, not a corner detector or laser-line identifier.
A high-contrast edge can be sampled but still fail correspondence uniqueness.

Maximum seeds must be 1..100000 and must fit the matching budget of 100 million
bidirectional sample visits for the supplied disparity range and patch radius.
Candidate extraction has its own 100 million patch-sample bound. Invalid
settings fail rather than being silently reduced. Cancellation remains
cooperative. No valid textured samples yields an explicit report with empty
seeds and no reconstruction; it never manufactures points.

Reports now retain `seeds` in exact matching-index order and
`automatic_sampling` (null for manual inputs), so all mapped/rejected seed
indices remain interpretable. Existing explicit-seed callers keep their
behavior. Synthetic integration tests reconstruct the known 400 mm plane
without manual pixels and verify blank input, conflicts, resource validation
and lineage. Mask/domain and texture checks are algorithmic safeguards, not
physical calibration, exposure synchronization or real scanner qualification.

## Optional surface export from seed-grid lineage

A reconstruction request can additionally include:

```json
"meshing": { "spacing": 4, "maximum_edge_mm": 35.0 }
```

This must describe the actual rectified seed lattice, anchored at
`[patch_radius, patch_radius]`. Automatic sampling spacing must match exactly.
Manual seeds must also lie on that explicit lattice; irregular inputs fail
rather than being reordered into an invented surface. At least two rows and
columns and at most one million lattice sample slots are supported. Missing,
unsampled, rejected or unmapped observations remain holes.

After triangulation, original left-camera-frame positions are placed at their
known seed-grid slots. Mesh vertices remain in that original frame, not the
rectified optical frame. Every mesh vertex has a `vertex_seed_indices` entry
in `mesh-report.json`, linking to the stereo report's exact seed/match lineage.
The mesher requires four valid corners per cell and enforces the explicit
3D edge limit/conditioning checks documented in `ORGANIZED_MESHING.md`.

The bundle adds `mesh-report.json` and, only when triangles exist, `mesh.json`,
`mesh.stl`, and `mesh.obj`. Summary adds `mesh_triangles` (zero when disabled or
empty). All artifacts use the same atomic publication and integrity manifest.
A blank pair retains diagnostics without mesh files. Existing requests without
`meshing` keep their reconstruction behavior. The GUI accepts the same settings,
reports exported triangle count, and still opens a point-cloud project: mesh
viewing/editing and mesh project persistence are not yet implemented.

The synthetic dataset generator plus automatic spacing 4/max seeds 100 and
meshing spacing 4/max edge 35 mm produced 93 points and 146 triangles in the
release CLI and native workbench. CLI/GUI artifacts and manifest match
byte-for-byte; the private-display screenshot was visually inspected with no
camera handles. Automated tests check exact source-frame vertex lineage,
known plane depth, blank inputs, explicit sparse lattice and invalid layout.
All 189 workspace tests, Clippy, formatting and release build pass. Evidence:
`stereo-surface-example-summary.json`, `stereo-surface-render-summary.json` and
stereo-surface test/build logs. These are synthetic pipeline checks; no physical
scanner geometry, exposure timing or surface accuracy is qualified.
