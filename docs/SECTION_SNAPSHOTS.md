# Reproducible section snapshots

After computing a section in the mesh workbench, enter a **New section snapshot
folder** and click **Save section snapshot**. The destination must not already
exist. Saving preserves the current mesh, undo/redo state and current analysis.
Open the bundle's `section.secproj` with **Open mesh** to restore its measured
source and plane and recompute the graph. Keep both bundle files together.

A bundle contains:

- `source.meshproj`: an exact checksummed snapshot of the indexed source mesh,
  without its previous undo/redo history.
- `section.secproj`: schema-1 plane settings, source byte count/CRC, algorithm
  version and expected node/segment/contour/ambiguity counts and total length.

The descriptor begins with `SECPLN01`, a little-endian u32 payload length and a
CRC32 of its JSON payload. It is bounded to 64 KiB. The source uses the existing
128 MiB mesh-project payload limit and must contain one mesh state with no undo
or redo steps. Source filename is fixed by the format; the descriptor cannot
redirect loading to an arbitrary path. Schema, millimeter units, algorithm tag,
qualification flag and unknown fields are checked. Original input normal values
are retained, avoiding extra normalization rounding on reopen/resave.

Reopening checks descriptor integrity and source size/CRC, validates the mesh,
recomputes the section and compares the recorded counts and length (1e-12
relative length tolerance with a minimum-positive floor). Geometry/plane data
remain the authoritative evidence; a derived graph is not trusted from serialized
input. CRC detects accidental corruption, not maliciously rewritten provenance.
Algorithm changes that invalidate this reproduction contract need explicit format
compatibility handling. Physical accuracy always remains unqualified.

Writes use a sibling staging directory, sync artifacts/directories and publish
with Linux no-replace rename. Cancellation before publication leaves no final
bundle. A failed write can retain an incomplete staging directory identified in
the error. If parent-directory sync fails after publication, the error explicitly
identifies the published bundle. The source and existing destinations are never
overwritten. Normal worker progress/cancellation applies to preparation and save.

CLI equivalent:

```
scancraft-cli save-mesh-section MESH_OR_PROJECT OX OY OZ NX NY NZ TOLERANCE_MM NEW_OUTPUT_DIRECTORY
scancraft-app --mesh NEW_OUTPUT_DIRECTORY/section.secproj
```

The app recognizes section documents by `.secproj` extension. Ordinary section
recomputation uses restored settings; explicit `--mesh-section` arguments can
request a different section after reopening its source. Failed/corrupt opens
preserve the currently displayed source and analysis.

This is one immutable measurement/source snapshot per bundle. It does not embed
multiple sections in mesh undo/redo projects, save the previous mesh history,
freeze all derived graph coordinates across algorithm versions, or provide CAD
cross sections/area. Integrated multi-section project/history management remains
unfinished. Users can preserve original history separately using **Save full mesh
history**. See MESH_SECTIONS.md for geometric interpretation limits.

Evidence: `evidence/section-document-{tests,clippy,release}.log`,
`section-document-cli.json`, synthetic `section-document-cube/` and the inspected
`section-document-render/`. Tests cover round trips, source retention, no overwrite,
cancellation, missing/corrupt sources, descriptor CRC/trailing-data errors,
CRC-valid inconsistent totals/unknown metadata, preservation of original oblique
normal values, byte-identical descriptor resaving and GUI save/reopen/failure
behavior. The native render restores the source cube, plane, 8 segments, one
closed contour and 4 mm length without camera descriptors. This is synthetic
startup/reopen evidence, not full interaction or physical qualification.
