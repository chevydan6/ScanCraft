# Manual point-cloud coordinate frames

Expand **Coordinate frame** in the point-cloud workbench. Select the reference
points and enter their current source indices. **Move origin to reference**
translates the whole cloud so the chosen origin becomes zero. **Align coordinate
frame** also uses two distinct selected references: +X follows origin to the X
point, and the third point defines the positive XY side. The resulting frame is
orthonormal and right-handed. All points transform, not just the references.

The operation rotates normals without normalizing their magnitudes, preserves
colors, missing attributes, point order and selected indices, and leaves scale
unchanged within floating-point rounding. It is a rigid coordinate change, not
scanner calibration, scale correction, registration or a best-fit reference
plane. Noisy reference points affect orientation. Mesh coordinate editing and
interactive frame gizmos remain separate work.

Preparation runs on the document worker with cancellation. The next cloud and
viewport preview must both succeed before commit. Invalid/unselected/out-of-range
references, repeated reference indices, coincident/near-collinear directions
(sine of reference angle <=1e-8), invalid frames and nonrepresentable transformed
coordinates/normals fail without replacing the source. New transformations mark
the project dirty, reset the view and create a geometry undo step. Undo/redo and
project save/reopen retain the source and transformed clouds. Origin-only
translation preserves normal component bits, including signed zero/subnormals.

Saved measurement snapshots retain their original historical coordinates. They
do not move with the cloud and may fall outside its current view. Their labels
and source evidence remain historical; recompute a new measurement in the new
frame when that is the intended evidence. Geometry undo remains separate from
measurement metadata undo. Frame origin/axes are reported in the operation status and can now be saved as
separate named reusable documents; see REUSABLE_FRAMES.md. Embedding these records
in ordinary cloud projects remains unfinished.

API: `scancraft_geometry::coordinate_frame::Frame::{at_origin,from_points,apply}`.
Positions map as R*(p-origin), normals as R*n. R contains the world-coordinate
frame axes as rows. Direction vectors are independently scaled before
normalization; anchored subtraction preserves small local offsets around a
distant origin. Output rotation uses scaled arithmetic and rejects overflow.
Public frame values are validated as finite proper orthonormal frames before use.

Startup convenience, without acquisition:

```
scancraft-app --coordinate-frame PROJECT_OR_PLY_FILE
```

This opens the controls and file, without selecting references or changing data.
The private-display render checker accepts the same flag. The synthetic
`coordinate_frame_fixture` Rust example generates an aligned editable project
with source undo history and a retained historical distance measurement.

Evidence: `evidence/coordinate-frame-{tests,clippy,release}.log`,
`coordinate-frame-preview.orproj` and `coordinate-frame-render/`. Five numerical
tests cover known right-handed axes, oblique rotation, distance/normal magnitude
preservation, independent reference scales, distant origins, normal bits/missing
attributes, degenerate frames, cancellation and overflow. A worker regression
checks invalid/cancelled atomicity, selection and historical evidence retention,
save/reopen and source undo/redo. Synthetic native rendering checks control layout
and reopened aligned geometry; full mouse workflow, performance and physical
accuracy qualification remain incomplete.
