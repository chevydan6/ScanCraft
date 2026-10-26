# Checksummed editable mesh history

`MSHPRJ01` projects store exact current mesh geometry, bounded undo/redo snapshots
and the history limit. Units are explicitly millimeters. This is mesh edit
history persistence, not the mission's complete combined scan/calibration/session/
measurement project. Plane parameters, source capture provenance and viewport
state remain in their separate result reports or are not yet persisted.

`trim-mesh` now accepts either mesh JSON or a mesh project and writes
`project.meshproj` alongside its edited exports. JSON input starts a new history;
project input preserves the previous history and appends the new trim. The input
source is copied byte-for-byte as `original.mesh.json` or `original.meshproj`.
A new edit clears redo according to the editor contract. Oldest undo snapshots
can be evicted by step/payload limits, with eviction count in the report.

```
scancraft-cli trim-mesh SOURCE/mesh.json 1 0 0 0 FIRST_RESULT
scancraft-cli trim-mesh FIRST_RESULT/project.meshproj 1 0 0 50 SECOND_RESULT
scancraft-cli mesh-history SECOND_RESULT/project.meshproj undo UNDONE_RESULT
scancraft-cli mesh-history UNDONE_RESULT/project.meshproj redo REDONE_RESULT
```

These example planes retain x<=0 and then x<=-50 mm. Choose planes appropriate
to actual geometry; the commands never silently cap the cuts. Undo/redo commands
reopen the stored history, apply exactly one available step and save a new
bundle with previous project bytes, the new project, current mesh JSON, applicable
STL/OBJ exports, diagnostics and integrity manifest. An unavailable step is an
error and publishes no target. All writes preserve existing targets.

The native Meshes pane now opens both mesh JSON and `project.meshproj` using
content-based detection. Enter a **New history bundle** folder and select
**Save full mesh history** to persist current, undo and redo states together.
Save is disabled while a trim preview is pending. Reopen its `project.meshproj`
with **Open mesh** or `scancraft-app --mesh FILE`. Reopened historical snapshots
are self-contained: source JSON and earlier result folders are not required,
and further trim previews use the validated in-memory geometry.

All states and viewport previews are prepared on a worker before replacement.
Corrupt/unsupported projects, excessive retained viewport-history payload,
invalid geometry and cancelled loading preserve the current model/history.
The GUI's 128-MiB retained coordinate/index/preview payload bound can reject a
core-valid project whose additional cached draw data would exceed that bound;
no history is silently dropped on load. The stored history limit is preserved.
Saving creates a new bundle without changing current geometry or overwriting
an existing target. Save the full history before closing to preserve the current
undo/redo navigation state; applied trim versions remain separately saved too.
A trim operation's own backend project may contain only that operation's
backend chain, so use **Save full mesh history** for the complete GUI navigation
state. This does not yet persist plane-tool fields, viewport orientation,
measurements or combined raw/calibration/session project data.

Binary layout is 8-byte magic `MSHPRJ01`, little-endian u32 JSON payload length,
little-endian u32 CRC-32/ISO-HDLC payload checksum, then compact UTF-8 JSON.
Payload contains schema version 1, explicit units and the serialized editor.
Length and serialized payload are capped at 128 MiB. Current plus undo/redo
coordinate/index payload is also capped at 128 MiB, with at most 32 historical
steps. Text expansion may exceed the file cap even when geometry fits; saving
then fails explicitly. Source geometry/history is never approximated to fit.

Save validates every snapshot, serializes through a bounded checksumming sink,
then streams a second pass with cooperative cancellation and consistency checks.
Load checks magic/length, exact payload availability, CRC and no trailing bytes,
then strict schema/units/history count/geometry. Invalid indices, nonfinite or
degenerate faces in historical snapshots are rejected too. Direct generic serde
construction of a MeshEditor is not validation; production readers must use the
project loader or call `validate_history` before treating such data as valid.
Existing mesh JSON remains unchanged and is not mistaken for this binary format.

Three geometry regressions check exact f64/current/undo/redo round trips,
corruption/all truncations/trailing/oversize, and CRC-valid invalid histories,
schema/units and cancellation. A CLI regression chains two trims, reopens and
undoes both, redoes both, checks exact restored meshes and source project bytes,
and rejects unavailable/corrupted history. All 203 workspace tests, Clippy,
formatting and release workspace build pass.

An actual release CLI run on the synthetic stereo surface produced 90 triangles
on the first trim and 54 on the second. Reopened undo's mesh JSON matches the
first trim byte-for-byte; reopened redo matches the second. All artifact lengths
and CRCs were verified (`evidence/mesh-history-example-summary.json`). This is
synthetic persistence evidence, not physical accuracy, full mission project
coverage or GUI project-workflow qualification.

## GUI round-trip verification

An added native-controller regression applies two trims, undoes one, saves the
full GUI state, destroys that controller and deletes the original JSON and both
trim result folders. A fresh controller opens the project with 1 undo/1 redo,
restores all three meshes exactly and prepares another trim from its saved
snapshot. Existing-target save and corrupt-project load retain the current
state. All 204 workspace tests, Clippy, formatting and release build pass.

An actual separate software-rendered native release process reopened a two-trim
synthetic project: 41 vertices/54 triangles with 2 undo/0 redo displayed and Undo
enabled. Its screenshot was visually inspected, no scanner handles were opened,
and the user desktop was not accessed (`evidence/gui-mesh-history-render-summary.json`).
This proves startup/rendering and controller persistence, not all mouse/control
interactions, large-history performance or physical geometry accuracy.
