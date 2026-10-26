# Exporting an edited project with retained sources

In Point clouds → Initial reconstruction provenance, select the source bundle,
expand **Export project with retained sources**, enter a new folder and choose
**Export new project bundle**. The background job copies the current editable
state, including unsaved edits, geometry undo/redo and measurement history, with
its five retained initial reconstruction artifacts. It verifies the source,
checks copied bytes, reloads/re-serializes the saved project to compare the complete
canonical state, and publishes the new directory without replacing existing data.

The active document, dirty state, current path and original source references
remain unchanged. Export is a copy; open the exported project to continue there.
The exported copy's descriptive source directory points to its new location.
No source/project content is overwritten. Source access may update filesystem
access times in the normal way.

Commands:

```
scancraft-cli export-project-bundle PROJECT SOURCE_BUNDLE NEW_DIRECTORY
scancraft-app --export-project-bundle PROJECT SOURCE_BUNDLE NEW_DIRECTORY
```

The app explicitly loads the supplied project, then exports it. The CLI exports
the project's saved state; the GUI button exports the current in-memory state.
Both use the same Rust workflow. Ordinary project load still does not read external
source artifacts. On a moved bundle, **Use project folder** fills the directory
containing the current project for an explicit verification/export action.

The output contains project.orproj (v5, or v6 when complete recording references are present), calibration.rapcal, request.json,
recording-sources.json, left-frame.raptor, right-frame.raptor, bundle-report.json
and manifest.json. This is self-contained for the selected initial frame pair
and serialized editable geometry state. GUI selection, viewport and unsaved
parameter widgets are not serialized by the existing Editor project format.
Full original multi-frame recordings, unrelated
files, historical reconstruction diagnostics, meshes and additional scan sessions
are not copied by this exporter. Initial provenance remains historical and does
not become current per-point or merge lineage.

Moving the complete directory keeps these data together. The stored directory
is a descriptive absolute reference from export time and can become stale after
a move. Choose **Use project folder** or enter the moved directory to verify its
artifacts; references are not automatically resolved or rewritten on load. Raw
frame/source paths retained inside historical provenance still describe the
original selection. Each retained raw recording contains one selected frame;
its local replay index is zero, while historical source indices remain recorded.

Bounds and failure handling:

- The current project must have validated recorded-source provenance. Its 64 MiB
  serialized limit is checked before history cloning or staging allocation.
- Copying uses a reusable 64 KiB buffer, bounded expected sizes plus one byte to
  detect growth, cancellation checks between reads/writes, and no final-component
  source symlink following. The existing source-verification guards apply.
- Source and copied artifact CRC/length references must match. The source is checked
  again before publication; the copied project must reload and reproduce the same
  complete serialized-state CRC/length. These are noncryptographic accidental-
  corruption checks, not authenticity or physical-calibration attestation.
- Files and the staging directory are synced before no-overwrite atomic rename;
  the parent directory is synced afterward. Cancellation/failure before publication
  leaves the final target absent and reports retained staging. A destination race
  cannot replace an existing target. A parent-sync error after rename reports that
  publication already occurred; inspect that output before retrying.
- Cancellation cannot preempt an in-progress filesystem read or a serializer's
  current CPU pass. Publication commits the copy; later cancellation does not
  retract it. No source edits, calibration activation or scanner handles are used.
- Export temporarily clones the editable state and loads its saved representation
  for verification; the clone is dropped before reload. This is bounded by the
  project envelope and is not a 60-minute memory or large-project performance gate.

Verification:

Six workflow tests cover edited geometry/measurement histories and exact source
retention, immutable originals, missing old source directory after export, existing
outputs, invalid input, source corruption, deterministic source changes and output
races at publication, cancellation after copying with retained staging, and the
64 MiB preflight before source access. The GUI worker regression now exports a
trimmed document and confirms the active document is unchanged. Startup/UI/parser
tests verify explicit load-then-export and required paths.

453 workspace Rust tests / 17 reference-tooling tests pass, with strict all-target
lint, formatting and release binaries/examples. Logs: evidence/project-bundle-*.log.
The reproducible synthetic fixture has one current point, an undo state with three
points, a saved distance and measurement undo history. Independent Python auditing
in evidence/project-bundle-cli-proof verifies complete project JSON equality except
the exported directory, exact source copies, unchanged source SHA256s, overwrite
refusal and successful verification after moving the archive. Native export and
fresh-app reopen/verification are captured in project-bundle-{gui,reopen}-render,
without camera descriptors. These are startup workflows, not full mouse interaction
or calibrated scanner qualification.

Reproduce:

```
cargo run --release -p scancraft-workflow --example project_bundle_fixture -- NEW_FIXTURE
python3 tools/check_workbench_render.py NEW_FIXTURE NEW_EVIDENCE --export-project-bundle --captures 3
```

The generator accepts only the bundled synthetic provenance and refuses existing
directories. Full raw/session project packaging, persistent standalone relocation,
multi-source lineage, calibrated scanner geometry, live tracking and the master
mission remain unfinished. Physical calibration stays deferred.

The edited fixture also exposed a default-window layout defect: expanded source
and saved-measurement controls placed status/viewport below 850 pixels. The saved
baseline PNG has zero viewport-fill pixels inside the 1150x850 app window; see
project-bundle-layout-baseline-render. Controls and long status text now use bounded
vertical scrolling; progress/cancellation and geometry stay outside the controls
scroller. A headless native-UI regression requires a viewport of at least 180 pixels
inside the default window for the expanded fixture. Final native screenshots show
the edited point and completed status. This fixes the demonstrated layout case,
not all GUI sizes or interaction qualification. Saved PNG pixels distinguish this
from earlier tool-preview partial-image interpretations.

Complete supplied recording files now have a separate validated raw archive
workflow (FULL_RECORDING_ARCHIVES.md). Selected-frame editable project export still
has the scope above; association with full recording archives remains work.


Complete-recording references in v6 projects survive this export unchanged. This
selected-frame bundle does not copy full recordings or retarget their directory.
Use scan-project export for a container with the complete supplied recordings.
See PROJECT_ORIGIN.md for explicit full-recording verification and schema details.
