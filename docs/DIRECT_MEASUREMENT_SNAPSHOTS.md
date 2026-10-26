# Saved point distances and angles

The saved measurement panel can record a **Distance** with exactly two selected
points or an **Angle** with exactly three. For angles, the vertex source index
must be selected, just as for the read-only measurement control. Records support
rename, removal and the separate snapshot undo/redo history, including after
project save/reopen. Recording marks the project dirty without changing geometry
or geometry undo history.

Each record retains its own finite source coordinates. The stored angle vertex
indexes this retained list; it is not an index into the subsequently edited cloud.
Distances report mm/inches and angles report degrees. Cloud deletions, including
removal of all source points, do not delete the measurement evidence. These are
historical values and are not automatically recomputed after geometry edits.

Snapshot schema 2 introduces `distance` and `angle` result tags. Existing sphere
and circle snapshots remain schema 1 and continue to load. Project envelope and
metadata history rules remain versions 3/4. Older applications without these new
tags reject these records instead of silently discarding them. Schema 1 cannot
claim a direct result. A direct snapshot is checked against its retained source
coordinates on load and throughout history validation: distance tolerance is
1e-12 relative (with a minimum-positive floor); angle tolerance is 1e-10 degrees.
This is a numerical consistency check, not provenance authentication or physical
accuracy qualification. Sphere/circle validation retains its existing structural
checks rather than promising a refit.

“Show historical measurement outlines (purple)” draws the retained distance
segment or both angle arms with historical labels, alongside sphere/circle fits.
They use the current cloud's view bounds, do not participate in picking and do
not alter exports. An empty current cloud has no projection, so its records still
appear in the panel but no outlines are drawn. Labels are wrapped and placed in bounded, non-overlapping viewport rectangles,
with dark backgrounds and leaders to their source locations. If no placement
fits, a viewport label is omitted; the panel retains the complete value.
Placement is greedy, not an optimal packing guarantee. The existing
`--measurement-outlines PROJECT` startup option enables these outlines without
starting acquisition.

Verification: `evidence/direct-snapshots-{tests,clippy,release}.log`, the synthetic
`direct-snapshots-preview.orproj` (reproduce with the
`direct_measurement_fixture` Rust example), and `direct-snapshots-render/`.
Tests cover record/save/reopen, restoration of removed source evidence, geometry
and metadata undo independence, invalid local vertices, numerical inconsistency,
CRC-valid tampering, worker cancellation and explicit source-to-retained vertex
mapping. Native screenshot inspection is limited to startup/rendering; it is not
full mouse interaction, scanning or physical accuracy qualification.
