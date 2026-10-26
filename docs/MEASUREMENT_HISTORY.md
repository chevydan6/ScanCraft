# Reversible measurement management

The saved-snapshot panel now provides **Rename**, **Remove**, **Undo snapshot
change** and **Redo snapshot change**. Recording a new snapshot also joins this
history. These controls affect the historical measurement list, not cloud
geometry, selection or the separate geometry Undo/Redo controls. Changes mark
the project dirty and persist when the project is saved.

Renames require a nonempty, unique label of at most 128 bytes without control
characters. Invalid names/indices and cancelled worker jobs do not commit.
Starting another worker operation closes an unfinished rename form so it cannot
later rename a different row after the document changes.

The history stores at most 16 inverse changes across undo and redo, rather than
cloning whole clouds or measurement lists. A deletion retains the removed typed
snapshot and all its source coordinates for restoration. New changes discard
the redo branch; oldest undo steps expire at the stated bound. Current records
are not removed to satisfy history limits, and the existing 16-record/100,000-
source-point and 64 MiB project limits remain. Save can still fail if retained
evidence plus geometry/history exceeds that file limit; data is not silently
discarded to make it fit.

Loading checks change counts, snapshot content, insertion/removal indices,
expected labels and both reachable undo/redo label sequences. It does not infer
physical provenance or rerun the fit. Geometry edits remain independent because
measurements intentionally retain historical source coordinates.

## Format compatibility

Measurement history uses ORPROJ04/schema 4, including when the current list is
empty but undo/redo retains source evidence. Existing ORPROJ02 projects remain
readable, and annotation-free projects without measurement history still save
as version 2. ORPROJ03 snapshots load without inventing prior history and may
resave as version 3 unchanged; editing their measurement list upgrades to
version 4. Older readers reject new envelopes rather than losing history.
History inserted into version 2/3 or inconsistent with schema 4 is rejected.

Three new persistence tests cover deleted-source restoration after save/reopen,
rename/redo/branching, failed-change atomicity, the 16-change bound, version-3
migration and CRC-valid inconsistent history. A worker test covers management
commands, dirty-state reporting, geometry/selection retention and cancellation.
The existing snapshot tests now cover version 4 output as well. Evidence uses
`measurement-history-`. A native-generated version-4 fixture is separately
rendered to inspect its restored controls/history; this is startup visibility,
not a complete mouse-driven editing qualification.

Projects that also carry initial reconstruction provenance now use ORPROJ05 /
version 5 and retain the same measurement-history semantics. Without provenance,
version 4 behavior is unchanged. See PROJECT_ORIGIN.md.
