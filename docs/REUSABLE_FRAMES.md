# Reusable coordinate frames

The point-cloud **Coordinate frame** panel now separates frame definition from
application. **Define reusable frame** uses the selected origin/X/XY-side source
indices without changing the cloud. Manual alignment also retains its frame.
The loaded/defined frame shows its origin and axes, accepts a name, and can be
saved to a new folder with **Save reusable frame**.

Load that folder's `frame.rapframe` with **Load frame**. Loading and saving leave
cloud geometry, selection, project dirty state and geometry history unchanged.
**Apply reusable frame** explicitly maps the current cloud from the frame's world
millimeter coordinates into local millimeter coordinates. It rotates normals,
preserves other attributes and selection, resets the view and creates a geometry
undo step. The input cloud must use the frame's original coordinate system;
applying it again to an already aligned cloud repeats the transformation.
Historical measurement snapshots remain in their historical coordinates.

The saved document retains a label, world origin, axes, schema version, millimeter
units and `world_to_local` mapping. It contains no scanner calibration, source
reference-point evidence or physical provenance. Frames are separate reusable
files; they are not yet embedded as named records in ordinary cloud projects.
Project geometry undo/redo does retain applied results and exact originals.

`RAPFRM01` files have an eight-byte magic, little-endian u32 payload length and
CRC32 followed by strict JSON. Payloads are capped at 16 KiB. Schema/mapping/units,
label bounds/control characters, unknown fields and a false qualification flag
are checked. Frame axes must be finite, orthonormal and right-handed. Explicit
finite checks on orthogonality and determinant prevent very large finite matrix
entries from bypassing validation through overflow/NaN. The same correction was
applied to rigid-registration pose validation. CRC is an accidental-corruption
check, not provenance authentication.

Saving stages and syncs a new one-file bundle before Linux no-replace publication.
Existing destinations are never overwritten. Cancellation before publication
leaves no final destination; failed staging can be retained with its path in the
error. A post-publication parent-sync failure explicitly identifies the published
location. Save/load/definition run through the normal cancellable document worker.
A malformed load preserves the existing cloud and retained frame.

To inspect a saved frame without acquiring cameras or applying it:

```
scancraft-app --frame-document PATH/TO/frame.rapframe
```

The app opens the frame controls with no cloud; import a compatible cloud before
applying. The private-display render tool supports `--frame-document` as well.
The `scancraft-workflow` Rust example `reusable_frame_fixture NEW_FOLDER` creates the
redistributable synthetic reference document used for evidence.

Evidence: `evidence/reusable-frame-{tests,clippy,release}.log`,
`reusable-frame-fixture/frame.rapframe` and `reusable-frame-render/`.
Tests cover definition/save/load without edits, reuse on a different cloud,
cancellation, project undo after reopen, no overwrite, CRC/header/trailing errors,
CRC-valid invalid metadata/reflections/overflowing matrices and preserved source
on corrupt load. Native startup rendering checks the loaded parameter controls;
it does not qualify complete mouse interaction, reference accuracy, scanner scale
or physical calibration. See COORDINATE_FRAMES.md for the transformation math.
