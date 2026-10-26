# Indexed vertex-link topology checks

```
scancraft-cli mesh-topology MESH_JSON_OR_PROJECT
```

Mesh topology now checks the triangle fans around each indexed vertex, in
addition to edge incidence/winding, duplicate faces and unused vertices. A
surface can have two faces per edge and still be pinched at a vertex: two closed
tetrahedra sharing one indexed vertex are the regression fixture here.

For every triangle corner, a bounded disjoint-set structure records which
incident faces connect through an edge at that vertex. Shared-edge endpoint
corners are joined independently. The number of surviving roots per vertex is
the number of connected triangle fans. Coordinates are never welded or moved.
Used vertices are flagged when they have disconnected fans, incident edges with
more than two faces, or a boundary-edge degree other than zero or two.
Isolated vertices remain separately reported rather than included in the used
vertex problem list.

The additive topology fields are:

- `nonmanifold_vertices` and sorted `nonmanifold_vertex_ids`;
- `vertices_with_disconnected_fans`, `vertices_with_invalid_boundary_degree`
  and `maximum_vertex_fans`;
- `closed_manifold_candidate`, combining existing closure/winding/duplicate/
  vertex-usage conditions with the new vertex-link checks.

The legacy `edge_closed` field retains its prior meaning. On the pinched-shell
fixture it is true while `closed_manifold_candidate` is false. Neither field is
a watertightness proof: self-intersections, coincident but separately indexed
geometry, shell nesting/orientation and physical accuracy remain unchecked.
Two geometrically coincident vertices with different indices stay separate;
the operation describes indexed topology, not embedding validity.

The viewer reports the new counts and shows red problem-vertex markers through
surfaces. The hover diagnostic lists the first 32 IDs; CLI output contains the
complete bounded list. Up to 10,000 markers are sampled for display, with that
limit stated in the UI. Analysis does not split or weld vertices. A separate
explicit disconnected-fan split is documented in `MESH_FAN_SPLITTING.md`. Explicit
boundary filling now refuses nonmanifold vertices in the source and checks
their absence in its result, in addition to its existing edge conditions.

## Bounds, correctness and evidence

Mesh limits remain one million vertices/two million triangles. There are at most
six million corners; parent/rank storage is linear in corner count, and fan/
boundary-degree/problem arrays are linear in vertices. The existing ordered
edge map retains endpoint-corner IDs. Cancellation checks cover the additional
passes. This increases topology work/memory and does not establish qualification
at the maximum dataset size or for a 60-minute workload.

Tests distinguish a pinched pair of closed tetrahedra from separately indexed
shells, classify an open boundary bowtie, flag nonmanifold-edge endpoints, retain
duplicate-face refusal conditions, and cover regular boundary patches, empty
meshes, isolated vertices and cancellation. A fill test refuses an otherwise
valid ring boundary when another component contains a pinched vertex, preserving
the source. The CLI test verifies pinched IDs/closure classification and unchanged
input bytes. The GUI test verifies the red marker and unchanged geometry.

`evidence/mesh-vertex-pinch-input.json` is a synthetic seven-vertex/eight-face
fixture. The actual release report (`mesh-vertex-pinch-report.json`) shows zero
boundary/nonmanifold edges, legacy edge closure true, bad vertex ID 0 with two
fans, and closed-manifold candidate false. A separately authenticated Xvfb render
was inspected: the counts and red marker are visible; no scanner descriptor was
open. This is startup/render evidence, not a repair or full interactive test.

![Pinched vertex diagnostic](../evidence/mesh-vertex-links-render.png)

`cargo run --release -p scancraft-geometry --example mesh_vertex_audit` builds a
257×257 synthetic grid (66,049 vertices/131,072 triangles), checks its expected
1,024 boundary edges and single fan at every used vertex, and times three full
validation/edge/vertex-analysis calls. On this machine they took 65.87, 60.83 and
59.94 ms. A fresh Linux Python process spawning the same native audit and reading
`resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss` measured 24,224 KiB peak
RSS for the entire audit process. This is not incremental algorithm memory, a
timing speedup, large-dataset qualification or evidence of long-term memory
stability. Raw audit data: `mesh-vertex-links-grid-audit.json` and
`mesh-vertex-links-memory-audit.json`.

All 254 tests across targets, warning-denying Clippy, formatting and the release
build pass (`mesh-vertex-links-*` logs). Automatic vertex repair, broader mesh
repair and genuine calibrated scanning remain unfinished.
