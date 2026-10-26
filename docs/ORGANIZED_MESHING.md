# Experimental organized-grid meshing

The native mesh module and `mesh-grid` command build a triangle surface from
an explicitly organized grid of 3D millimeter samples. This is not general
surface reconstruction from unordered clouds and does not complete M5.
It neither fabricates missing depth nor interprets calibration/sensor modes.

```
./target/release/scancraft-cli mesh-grid examples/organized-grid.json NEW_MESH_RESULT
```

Input is a bounded regular JSON file (64 MiB) with schema version 1, explicit
`units: "millimeters"`, `source` (`synthetic_fixture` or `unverified_imports`),
width/height, row-major `samples` (3D arrays or null), and `maximum_edge_mm`.
Dimensions must be at least 2×2 with at most one million samples. Nonfinite
positions and invalid settings fail. Grid adjacency must come from the source
sampling layout; arbitrarily reshaping an unordered cloud is invalid.

Each cell requires all four valid samples. An incomplete cell stays open;
there is no hole filling or triangulation across a missing sample. The shorter
3D diagonal determines the two candidate triangles. Every triangle edge must
satisfy the explicit maximum length. Coincident/collinear or numerically
ill-conditioned triangles are rejected and counted. The normal calculation
rescales edge differences to avoid cross-product overflow and rejects normalized
cross-product length <=1e-12. Edge subtraction overflow also rejects a triangle.
These choices can remove thin legitimate details; configure and inspect results
rather than treating them as physical acceptance thresholds.

Winding is [top-left, bottom-left, top-right] and the corresponding second
triangle, giving -Z normals for a regular XY grid. No automatic sensor-facing
orientation, global consistency repair or inferred camera pose is applied.
Vertices are compacted from all valid samples but isolated vertices are retained
and reported. Colors and input normals are outside this position-only format.

The mesh type supports up to one million finite vertices and two million
valid indexed triangles. Export methods revalidate it. Topology diagnostics
count boundary edges, edges with more than two incident faces, two-face winding
conflicts, duplicate faces (irrespective of winding), and isolated vertices.
`edge_closed` reports only these edge/duplication/isolation conditions. It is
not a full watertightness test: self-intersections and nonmanifold vertex links
are not checked. Reports explicitly keep watertightness and physical accuracy
unqualified, even for an edge-closed mesh. Topology analysis can allocate maps
proportional to triangle edges; this is bounded but not yet memory-qualified.

A successful no-overwrite bundle contains report and CRC/length manifest, plus
`mesh.json`, `mesh.stl` and `mesh.obj` when triangles exist. A `no_surface` result
retains diagnostics and omits mesh exports. Staging/publication/cancellation use
the shared workflow. Invalid input fails before publication; output-write errors
retain an explicitly reported partial directory. Existing targets are preserved.

Mesh JSON stores schema, explicit units, vertices and triangles with f64
round-trip serialization. Deserialized Mesh values still require validation.
ASCII STL emits independently computed unit face normals and shortest round-trip
f64 coordinate text. STL has no standard unit metadata: `open_scancraft_mm` is only
an informational solid name, and consumers must interpret coordinates as mm.
OBJ uses 1-based triangle indices and an informational millimeter comment.
No normals/materials/colors are exported to OBJ. Native coordinate text preserves
f64 values; downstream application precision and unit behavior remain unqualified.
This adds export, not general STL/OBJ import or editable mesh project persistence.
The current point-cloud GUI does not render these mesh bundles.

Four geometry regressions cover holes, discontinuities, orientation, degeneracy,
edge/duplicate/winding/isolation diagnostics, cancellation/invalid data, and
independent text parsing of exact STL coordinates/OBJ vertices and indices.
Two CLI tests verify a sloped plane's analytic area, artifact integrity,
no-overwrite behavior, empty surfaces and controlled malformed/unit/cancelled
inputs. All 187 workspace tests, Clippy, formatting and release build pass.
The release command ran the committed synthetic 2×2 example, producing four
vertices/two triangles with verified artifact hashes
(`evidence/organized-mesh-example-summary.json`). No real Raptor surface or
physical dimensional accuracy is claimed.

Next connect source-grid lineage from calibrated stereo reconstruction, add
native mesh viewing/editable persistence, and implement unordered surface
reconstruction, repair, simplification and physical/reference qualification.

Stereo reconstruction now optionally feeds this primitive using an explicit
seed lattice and accepted-point lineage, preserving original camera-frame
coordinates and reporting each mesh vertex's seed index. See the `meshing`
request option in `OFFLINE_STEREO.md`. This connects synthetic image-to-surface
processing in CLI/GUI export; it does not add real calibration qualification,
unordered cloud meshing, a mesh viewport, editing or repair.

The later native Meshes pane can now inspect exported `mesh.json` surfaces
without modifying the point-cloud document. See `MESH_VIEWER.md` for loading,
controls, sampled-render/occlusion limits and verification scope. Mesh editing,
general format import and editable mesh persistence are still unfinished.
