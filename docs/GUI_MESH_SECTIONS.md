# Mesh cross sections in the native workbench

Expand **Mesh cross section**, set an origin (mm), a nonzero normal and a
nonnegative tolerance (mm), then click **Compute section**. The worker uses the
full current saved mesh. Its existing progress and cancellation controls apply.
The analysis does not edit geometry or create undo entries.

The panel reports segment count, closed/open contour counts, ambiguous graphs,
full segment length and coplanar/tangent/duplicate counters. A contour index
control shows that contour's full node count, open/closed status and length.
It does not select geometry. **Show section graph** overlays up to 10,000 sampled
segments in purple, through surfaces; omitted display segments remain included
in all counts and lengths. Branching graph segments are included. See
MESH_SECTIONS.md for the numerical and topology interpretation, including the
limitations of tolerance-band and coplanar contacts.

Changing section parameters invalidates the cached analysis. Recomputing clears
the previous result; errors/cancellation preserve the mesh. Opening/replacing a
model, applying an edit, undo and redo clear the analysis. It is hidden while an
unapplied edit preview is displayed, and refers to the saved source during that
preview. Saving mesh history does not store sections. Use **Save section snapshot** to
retain one section and its source in a separate reproducible bundle; see
SECTION_SNAPSHOTS.md. Integrated multi-section project entities remain open.

Startup without camera acquisition:

```
scancraft-app --mesh-section MESH_OR_PROJECT OX OY OZ NX NY NZ TOLERANCE_MM
```

The private-display render checker accepts `--mesh-section`, which uses a z=0.5
plane for its synthetic fixture. Evidence:
`evidence/gui-mesh-section-{tests,clippy,release}.log` and
`evidence/gui-mesh-section-render/`. Worker tests verify immutable source model,
unchanged undo state, cancellation/error handling, invalidation on undo/redo and
reload, and startup cube analysis. The native screenshot shows a closed 4 mm
cube section, eight segments and its overlay. This is synthetic startup/render
evidence, not a complete mouse-driven workflow, GPU performance qualification or
physical scanner accuracy qualification.
