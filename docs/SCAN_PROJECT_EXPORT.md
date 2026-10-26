# Project and complete recording export

The native camera pane's **Archive complete supplied recordings** panel can now
include a saved editable project and its retained reconstruction inputs. Save edits
first, enable the project option, and supply the saved project, its source folder,
both full native recordings and a new destination folder. This background job has
progress/cancellation and does not start the camera or change another open document.

CLI:

```
scancraft-cli export-scan-project PROJECT SOURCE_BUNDLE LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY
scancraft-app --export-scan-project PROJECT SOURCE_BUNDLE LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY
```

The container has a root `manifest.json`, `project/project.orproj` with the five
retained reconstruction inputs, and `recordings/left.raptor`, `right.raptor` and an
archive manifest. Existing formats are preserved. Complete supplied recordings are
copied byte-for-byte with strict frame/order/CRC/footer/EOF replay and bounded I/O.
The edited project retains geometry, measurements, undo/redo and initial provenance.
Its source-directory display points at the final project subfolder, rather than a
staging location. Originals are never written. Publication uses no-replace rename
of the whole parent folder only after both child exports and association pass.
Failure/cancellation leaves an identified partial folder for inspection.

For each camera, the selected index is read from the project origin. Full recording
validation precedes selection. Version, total count, profile, context, sequence,
host/sensor timestamps and payload/metadata fingerprints must match retained
lineage; source path strings may differ after relocation. The selected frame is
then serialized canonically as the retained v2 single-frame recording and its
length/CRC must match the project's artifact reference. Retained source files are
separately verified by the project-bundle exporter. Swapped roles, changed selected
lineage/artifacts, corrupt/truncated files, missing origin and existing destinations
cannot publish a successful container.

This establishes selected-frame membership in the supplied recordings. Earlier
projects did not store a fingerprint of every byte in the original full recording,
so historical identity of unselected frames cannot be established retroactively.
The root manifest explicitly records this as unverified. Its relative entry paths
remain usable when the folder moves. Import the **scan folder** in the point-cloud
workbench or start `scancraft-app SCAN_FOLDER` to verify the packaged inputs before
opening geometry. `scancraft-cli verify-scan-project SCAN_FOLDER` performs the same
verification without starting a GUI. Source-directory relocation changes only the
in-memory editor; save a new project to retain that location. Opening the inner
`project/project.orproj` alone retains ordinary standalone-project behavior and does
not verify the whole container. Raw recordings open through existing playback.
Continuous scan resumption, multi-session provenance and mesh packaging remain
unfinished. Recovery
reports are not embedded: keep original recovery reports separately. Capture
completeness, exposure synchronization, physical device/calibration binding and
optical accuracy remain unverified. Physical calibration stays deferred.

Evidence is synthetic, not a genuine geometry qualification:

- Four backend regressions cover full recording bytes, edited-project undo after
  relocation, original preservation, wrong lineage/artifact, invalid inputs,
  late cancellation and a publication race. Two native tests cover startup argument
  validation and background export.
- 474 workspace Rust tests, 19 reference/tool tests, strict all-target Clippy,
  formatting and release workspace binaries/examples pass. Logs are under
  `evidence/scan-project-*.log`. The first system-Python tool run lacked OpenCV;
  the reference virtual environment passes. That failed attempt is retained.
- `evidence/scan-project-gui-render` records three inspected private-display captures
  of the native panel after export, no camera/USB device descriptors open, and
  binary hashes. `inspection.json` verifies CLI/GUI full recordings equal the
  synthetic originals, all six frames decode, five retained artifacts match, and
  exported project source verification passes. This is startup/worker/render
  evidence, not a complete manual interaction or hardware qualification.

## Explicit scan-folder verification and loading

`scancraft-workflow::scan_project_load` checks schema/kind, fixed relative entry paths
and unqualified-status flags. Nested manifests must match the root copies. Artifact
inventories must contain exactly the seven known project artifacts and two complete
recordings, with valid bounded lengths and CRC32 values. Unknown/duplicate/missing
listed artifacts are rejected; unrelated unlisted files are not verified. Root and
known child directories/files reject final symlinks and non-regular files. All
opened files and directories retain identity/size/mtime/ctime checks through the
operation, including a final readback check before returning the editor.

Manifests are bounded to 32 MiB each, the project to 64 MiB + 16 bytes, retained
artifacts to 128 MiB each, and complete recordings to 1 TiB each. Hashing uses 64 KiB
chunks with cancellation checks. Complete recording replay/indexing retains its
existing frame/index budgets. Load verifies retained project references and repeats
full recording validation/selected-frame association, then relocates the source
folder in the returned editor. Corruption in unselected recording bytes also fails
the stored archive fingerprint. These are point-in-time integrity checks against
manifests, not cryptographic authenticity or historical original-capture attestation.
Later changes require another explicit verification.

The GUI reuses its existing background import, cancellation and unsaved-document
guard. No document/selection change commits until verification and preview succeed.
Stored undo/redo survives loading and relocation. No packaged files are rewritten.
No new physical calibration or synchronization claim is made.

The load checkpoint passes 478 Rust tests, 19 reference/tool tests, strict all-target
Clippy, formatting and release workspace binaries/examples. Three backend tests
exercise relocation with exact saved undo/redo, whole-file corruption, manifest
mismatch, unexpected root paths, symlinks, late mutation and cancellation. A native
worker regression checks successful relocation and document/selection preservation
on failed reopening. Evidence: `evidence/scan-project-load-*.log`, CLI report and
`evidence/scan-project-load-gui-render`. Three private-display captures show the
verified moved synthetic project with three points; all 12 packaged files remain
byte-identical after CLI and GUI opening. This does not qualify real scanning,
manual interaction coverage, large-data performance or the complete mission.


Scan-folder export now writes an ORPROJ06 project with persistent complete-recording
fingerprints. Loading older v5 containers attaches these in memory from the verified
archive inventory; loading v6 cross-checks existing references before relocation.
A later standalone save retains the full-recording link without requiring external
files on ordinary project load. See PROJECT_ORIGIN.md for schema and explicit
`verify-project-recordings`/native verification. References do not replace recovery
reports or establish historical original-capture identity.
