# Persistent measurement snapshots

The cloud workbench's **Saved measurement snapshots** panel explicitly records
selected sphere/circle measurements in the editable project. Fit-only commands
remain read-only. Recording a snapshot refits the current selection, retains
the fitted result and source positions, and marks the document unsaved; use
**Save project** to persist it. The panel lists saved labels, diameters and source
point counts after reopen. Generated labels are unique; duplicate API labels
are rejected. Circle snapshots retain the chosen plane-RMS limit.

Snapshots are historical evidence in their original millimeter coordinates.
They do not update when the cloud is trimmed, transformed or undone. Their
source positions are retained separately so a later cloud version cannot make
them appear to measure its current geometry. Geometry undo/redo leaves the
snapshot list intact. [Renaming/deletion and separate snapshot undo](MEASUREMENT_HISTORY.md) are now
available. On-viewport measurement overlays remain unfinished. No live link to source
point indices or parametric CAD model is inferred.

`measurement_snapshot::Snapshot::{sphere,circle}` computes typed results through
the native fitting APIs. `Editor::add_measurement` validates metadata/results,
unique labels and a 16-record cap. Each record permits at most 100,000 source
positions. The existing total project limit of 64 MiB still applies; no history
or source evidence is silently discarded to fit the limit.

Snapshot schema 1 records label, source positions, a typed sphere/circle result,
circle fit settings where applicable, and physical qualification false. Validation
checks source/result counts, finite values, positive radius and matching diameter,
residual/conditioning bounds, unit circle normals, planarity settings and schema.
A relative rounding allowance permits RMS to fractionally exceed its maximum
residual in floating arithmetic. Loading does not rerun a fit or authenticate
the claimed object/model: structural validity and CRC are not physical accuracy
or scientific provenance qualification.

## Project compatibility

Projects without snapshots or measurement history retain the existing
ORPROJ02/version-2 envelope and omit empty new fields. New loaders continue to read these files. Projects
with snapshots and no retained change history use ORPROJ03/version 3;
measurement-history projects use ORPROJ04/version 4. Both preserve length/CRC framing
and strict millimeter schema. Older apps reject that new format instead of
silently losing measurements. Magic/schema mismatches and snapshots inserted
into a version-2 envelope are rejected. The native startup loader already
recognizes the project family and opens both versions.

Three persistence tests cover legacy compatibility, both snapshot types,
geometry edits/undo/reopen, retained source values, duplicate/cap rejection and
CRC-valid malformed metadata/results/version mismatches. A worker test covers
recording without geometry/selection changes, duplicate/cancel failure before
commit, dirty-state reporting and saved-project reopen. The initial strict
RMS/maximum comparison exposed a rounding edge case, corrected before this
checkpoint. All final checks use `measurement-snapshot-` evidence.

The redistributable `measurement_snapshot_fixture` Rust example creates a new
analytical sphere project without overwriting an existing path. Its version-3
output is rendered in a separate authenticated display to inspect restored
measurement text. This is startup/render evidence, not a complete mouse-driven
measurement workflow or physical qualification.

Initial reconstruction provenance uses project version 5 alongside the existing
measurement snapshots/history. Projects without provenance keep versions 2–4.
See PROJECT_ORIGIN.md.
