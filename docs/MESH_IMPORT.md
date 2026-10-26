# Native triangle mesh import

**Meshes → Import STL / OBJ** imports external triangle geometry into a new,
original-preserving bundle and opens its editable mesh project. Choose the source
format and units (mm, inches, or meters) explicitly. Units have no default and are
converted to internal millimeters. Supply a source path and new output folder.
Existing destinations are rejected. Source files are never rewritten.

Choose whether to weld exactly equal coordinates. The default keeps separate
vertices. For STL this retains three independent vertices per facet; use exact
welding to recover shared indices when appropriate. Welding can also join
coincident parts, create duplicate faces or nonmanifold connectivity. It is a
user-selected import policy, not automatic mesh repair. There is no proximity
threshold. Positive/negative zero compare equal for welding; other coordinates
must match exactly after unit conversion. Topology defects are reported.

## Supported input and deliberate limits

- OBJ: UTF-8 position-only `v x y z` and triangular `f i j k` records, blank lines
  and comments. Positive one-based and relative negative indices are supported.
  References must point to existing vertices. Original indices and unused vertices
  are preserved unless explicit exact welding is enabled. Polygons, texture or
  normal face indices, colors, homogeneous vertices, materials, groups, objects,
  smoothing declarations and other records are rejected with line-specific errors.
  They are not silently discarded or triangulated. Broader OBJ support remains open.
- STL: strict ASCII facet/loop/vertex records and little-endian binary STL with
  exact triangle-count/length agreement. A valid binary file is recognized even
  when its header starts with `solid`. Nonzero binary attribute/color fields are
  rejected. ASCII solid names are retained in the original bytes, not modeled as
  scene objects. Multiple ASCII solids and trailing records are rejected.
- Supplied STL facet normals must be finite but are recomputed from vertex winding.
  OBJ normals are unsupported rather than silently lost. Meshes currently model
  indexed positions and triangles; textured/colored mesh import remains unfinished.

Input is bounded to 128 MiB, one million result vertices, two million triangles,
and 4,096 bytes per text line. OBJ also limits source vertex records to one million.
STL can supply up to six million vertex records under the triangle/byte limits
when exact welding keeps the output within its vertex budget. Input coordinates,
scaled coordinates, indices and triangle conditioning are validated. Zero-area,
repeated-index or ill-conditioned faces and nonfinite/overflowing values fail with
controlled errors. Empty surfaces fail explicitly. Parsing/validation/publication
check cooperative cancellation; failed import retains the previous GUI model.

## Saved data and CLI

```
scancraft-cli import-mesh FILE obj|stl mm|in|m separate|weld-exact NEW_OUTPUT_DIRECTORY
scancraft-app --import-mesh FILE obj|stl mm|in|m separate|weld-exact NEW_OUTPUT_DIRECTORY
```

Both publish a new bundle. The app then opens the saved project without starting
camera acquisition. Raw bytes are retained as `original.obj` or `original.stl`.
The bundle contains converted `mesh.json`, checksummed `project.meshproj`, native
STL/OBJ exports, parameter/topology report and integrity manifest. An imported
project starts with no undo steps; subsequent edits create normal reversible
history. Units/policy/import diagnostics live in `report.json`; the mesh project
stores converted geometry, not an external scene or parametric import stack.

ASCII numeric coordinates parse as f64. Binary STL coordinates arrive as f32 and
are converted to f64; this does not recover precision already lost in the source.
Unit multiplication introduces normal floating-point rounding. Native position-only
OBJ exports round-trip f64 coordinates/indices in mm; STL facet coordinates can
round-trip exactly in the tested native ASCII fixtures, with explicit welding
needed to recover connectivity. External-tool precision comparisons remain open.
No physical accuracy, global intersection or watertightness qualification follows
from importing a file or from a clean indexed topology report.

## Verified evidence

`evidence/mesh-import-tests.log` records 399 passing Rust tests. New geometry tests
cover OBJ round-trip/scaling and relative indices, explicit exact welding versus
separate STL facets, signed zero and distinct near coordinates, binary `solid`
headers, all truncated prefixes of a binary fixture, unsupported records/attributes,
invalid indices/polygons, nonfinite/overflowing/degenerate input, malformed ASCII
and cancellation. Workflow tests verify raw bytes, saved unit policy, reopening,
further trim/undo and overwrite/failure/cancellation safeguards. The GUI worker
test requires explicit units and exercises import/open/trim/save/undo.

Release CLI imports of synthetic native OBJ/STL exports preserve all ordered face
coordinates exactly and retain original bytes. Evidence is in
`mesh-import-{obj,stl}-example/`, summaries and `mesh-import-cli-checks.json`.
Native app startup import is rendered on a private display with no camera file
descriptors, and the resulting saved mesh project is opened for editing. Screenshots
and visual inspection describe that narrow scope, not a full mouse workflow,
independent external importer comparison or physical scanner qualification.

Earlier import captures were classified as partial while experimenting with
wider path fields; standard field sizing was retained. A later byte/pixel audit
found the saved lower mesh viewport present, so the earlier failure interpretation
and width-causation inference are withdrawn. Original inspection files remain;
`inspection-review.json` beside those captures records the correction. This does
not establish a rendering bug or rendering fix. See the recording recovery render
review for the repeated-image/hash evidence. Broad interaction qualification remains open.
