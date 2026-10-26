# Native mesh inspection

Open the **Meshes** tab, enter an exported `mesh.json` path and choose **Open
mesh**. A fresh application can open one directly:

```
./target/release/scancraft --mesh RESULT/mesh.json
```

The independent mesh pane does not replace or edit the point-cloud document.
Its background loader reads a bounded regular file (128 MiB), requires schema
version 1 and explicit millimeter units, validates finite/indexed/nondegenerate
geometry and computes edge-topology diagnostics. It prepares the render data
before replacing the previous mesh. Missing/malformed files, unsupported units,
invalid geometry, cancellation or worker failure retain the previous model.
Only the app's mesh JSON envelope is supported; this is not STL/OBJ import.
Standalone JSON has no embedded integrity checksum, so schema/geometry validation
is distinct from checking the export bundle's external CRC manifest.

Shaded and wireframe controls are independent. Drag to orbit, shift-drag to
pan, scroll to zoom and Fit view to reset zoom/pan. The viewport is orthographic
with two-sided directional shading. Preview triangles are depth-sorted CPU
shapes rendered through egui; this is not a dedicated depth-buffer mesh engine.
Intersecting surfaces or triangles with overlapping depth ranges can display
incorrect occlusion. Large meshes sample at most 50,000 triangles for drawing;
that preview can show holes absent from the source. The UI displays total and
preview counts; source geometry is never changed. This is not large-mesh
performance, GPU throughput or visualization-accuracy qualification.

Boundary, nonmanifold-edge, winding-conflict, duplicate-face and isolated-vertex
counts are shown for the full loaded mesh, not its draw sample. These do not
certify full watertightness, vertex manifoldness, self-intersection freedom or
physical accuracy. Editing, mesh undo/history, mesh projects, measurements,
repair, hole filling and simplification remain unfinished.

Two GUI regressions verify shaded triangle and wireframe shapes, exact source
preservation, failed asynchronous-load retention and cancelled preview building.
A workflow regression checks schema/units/index validation, malformed files and
cancellation. Startup tests include `--mesh`. All 192 workspace tests, Clippy,
formatting and release workspace build pass.

An actual software-rendered release startup on a separate authenticated X
display was visually inspected: the synthetic stereo mesh shows 93 vertices,
146 triangles, 38 boundary edges and zero reported nonmanifold/winding/duplicate/
isolated conditions. Shading and wireframe are visible. It opened no scanner
handles and did not access the user's desktop. Evidence:
`evidence/mesh-viewer-render-summary.json` and mesh-viewer test/build logs.
This checks startup/rendering of synthetic geometry, not every interactive
control, general occlusion, real Raptor geometry or physical qualification.

The pane subsequently gained original-preserving plane preview/apply and saved-
version undo/redo. It is no longer limited to read-only inspection. See
`MESH_TRIMMING.md` for the workflow, publication semantics, history bounds and
verification scope. General mesh projects, repair/simplification and physical
qualification are still unfinished.
