# Normals from explicit surface adjacency

`scancraft_geometry::mesh_normals::estimate` computes angle-weighted vertex normals
from an indexed mesh without moving vertices or changing triangles. It uses
only declared triangle adjacency, not inferred cloud neighborhoods. Input mesh
validation/caps and cancellation apply. Duplicate faces, nonmanifold edges and
inconsistent shared-edge winding are rejected; vertices with invalid/disconnected
links remain unsupported. Isolated vertices likewise receive no normal.

Each corner contributes its unit face normal weighted by its corner angle.
The normalized sum is accepted only if every incident face agrees within the
explicit `maximum_deviation_rad` gate, which must lie in `(0,pi/2]`. Conflicting
or cancelling normals stay absent. This protects against silently averaging
across a sharp crease; it does not split that crease or repair topology.

With no viewpoint, orientation follows face winding. Optional `viewpoint_mm`
orients each supported normal toward that point in the mesh coordinate frame.
Coincident/tangential or numerically unrepresentable viewpoint directions remain
unsupported. Sign selection is a declared policy, not evidence of physical
camera placement, outward shell orientation or sensor calibration.

## Reconstruction workflow

Request schema 1 accepts optional `surface_normals`, for example:

```json
"surface_normals": {
  "maximum_deviation_rad": 0.3,
  "viewpoint_mm": [0.0, 0.0, 0.0]
}
```

This requires explicit `meshing` settings. The existing seed lattice, missing
samples and maximum-edge gate determine adjacency. Normals are mapped back
through an explicit vertex-to-accepted-cloud-index list; each position is checked
against its source before attribute assignment.

The complete `points.ply` and editable project keep their existing points and
attributes. When at least one normal is supported, an additional
`surface-normal-points.ply` contains only those points, with unchanged positions
and colors and a finite unit normal on every point. This avoids inventing zero
normals or dropping unsupported samples from the full cloud, and fits the
current PLY exporter's uniform-attribute requirement. `mesh-report.json` records
the configuration, estimated/unsupported counts, artifact name (null when none)
and `cloud_indices`: each normal-artifact point's index in the full cloud.
All artifacts participate in existing atomic publication/digests.

The normal-bearing PLY can be supplied to explicitly selected point-to-plane
sequence tracking. Normals alone do not guarantee six-axis observability: a
single plane remains rank deficient and is rejected by that solver. There is
no automatic live tracking activation or normal estimation for unstructured
clouds in this change. Physical reconstruction/calibration remain unqualified.

Three geometry tests cover planar winding/viewpoint orientation, isolated
vertices, source preservation, sharp-corner refusal, topology errors, invalid
configuration and cancellation. Two CLI tests cover reconstruction-to-normal
PLY lineage/exact coordinates and missing-meshing refusal before publication.
Debug workspace and optimized reconstruction tests use `surface-normals-`
evidence. General noisy-scene quality, large-workload performance, real Raptor
normal accuracy and GUI interaction are not qualified by those fixtures.
