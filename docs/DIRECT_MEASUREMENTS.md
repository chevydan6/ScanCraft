# Point distance and three-point angle

In the point-cloud workbench, select exactly two points and click **Measure
point distance**. The status reports the Euclidean distance in millimeters and
inches (25.4 mm per inch), and the source point indices. Coincident positions
have zero distance; the selected indices must still be distinct.

For an angle, select exactly three points, enter the vertex's **source index**
and click **Measure angle**. The selected source indices are displayed when one
to three points are selected. The vertex must be selected; the other two points
form the arms. The result is the unsigned angle from 0 through 180 degrees.
Reversing arm order does not change it. Coincident arm/vertex positions are
rejected. This does not measure a signed rotation or an angle between fitted
planes.

Both operations run on the document worker and leave geometry, selected indices,
undo/redo, saved measurements and project dirty state unchanged. Their status
results are transient. To retain or visualize them, use the saved measurement
panel described in DIRECT_MEASUREMENT_SNAPSHOTS.md. Source indices
refer to the current cloud and can change after edits. Coordinate units follow
the imported/declared units and conversion; these measurements do not qualify
scanner scale or physical accuracy.

The Rust API `measurement::angle_degrees(a, vertex, b)` checks finite coordinates,
normalizes each arm independently and evaluates atan2(cross magnitude, dot).
Overflowing endpoint subtraction is replaced with half-coordinate differences;
subnormal finite differences use ordinary subtraction without premature halving.
Zero arms are rejected. Existing `distance_mm` rejects nonfinite endpoints and
nonrepresentable distances. No hardware or vendor runtime is involved.

Evidence: `evidence/direct-measurements-tests.log` (321 passing tests across all
targets), strict Clippy and release logs, and `direct-measurements-render/`.
Analytical tests cover known, parallel, opposite and translated angles, arm
exchange, independent scales, subnormal arms, overflowing differences, invalid
coordinates and zero arms. A worker test verifies a 3-4-5 distance, a right angle,
invalid vertex/count/indices, cancellation and preserved serialized project plus
redo history. The render checks control layout only; it is not a mouse-driven
workflow or physical qualification.
