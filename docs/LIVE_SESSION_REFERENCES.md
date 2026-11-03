# Live-session project references

Live scans with geometry now save an ORPROJ08 editable project carrying initial
session lineage. This closes the prior gap where standalone edited projects lost
the relationship to the raw scan bundle. Files without live-session references
retain their existing ORPROJ02–07 versions and compatibility.

The metadata records session ID, synthetic versus declared physical input, full
raw frame counts, candidate pair IDs and left/right raw indices, and each pair’s
integration decision. It fingerprints the two complete raw recordings, calibration,
reconstruction settings, scan settings, root report, pair reports and original
points PLY by byte count and CRC32. Paused/no-geometry/lost pairs remain represented
as not integrated. These are initial inputs, not current per-point correspondence.

The publisher verifies raw recordings and restores settings before publication as
before. It writes the report, fingerprints the fixed artifact set, attaches metadata
and saves the project, then writes the outer manifest including the project.
Project and manifest are excluded from project-owned references to avoid checksum
cycles. Blank scans still publish diagnostics/raw data without a fabricated project.

Edits, undo/redo and standalone saves retain references without opening external
files. Loading does not automatically access the descriptive directory path. To
check retained bytes explicitly, choose a folder in the GUI’s Live session sources
panel, or run:

```sh
scancraft-cli verify-source-bundle PROJECT.orproj RETAINED_LIVE_SCAN_FOLDER
```

A moved folder can be verified by explicit choice; verification does not rewrite
stored locations or silently relocate references. Missing, altered or symlinked
artifacts fail identity verification. This reuses existing bounded, cancellable,
regular-file and mutation-aware source verification. No raw files are embedded in
a standalone project, and no resumable fusion accumulator is saved.

CRC32 detects accidental corruption, not malicious replacement or authenticity.
Source-device binding, synchronization and physical accuracy flags remain false.
Declared physical input is not hardware attestation or qualified reconstruction.

Implementation: geometry/live_references.rs, Editor::attach_live_session,
formats::save_project/load_project, workflow/live_stereo/runner.rs,
source_verification::verify_live/verify_project, app cloud_jobs/ cloud_ui.
Tests cover unsafe/incomplete/qualified metadata, format downgrade rejection,
edit/reopen persistence without source I/O, moved-directory verification, modified
raw/calibration/report bytes and symlink rejection. All current datasets are
synthetic; actual Raptor qualification remains outstanding.

Release GUI startup verification was also inspected on a separate authenticated
X display: the intact synthetic bundle reported 8/8 matches, while a copied bundle
with modified left raw bytes reported 7/8 and left.raptor:crc_mismatch. The
three-point cloud and reference panel stayed loaded; no camera/USB descriptor
was open. This is synthetic GUI verification evidence, not scanning qualification.

## Export an edited live project with its raw session

The Live session sources panel can export to an explicit new folder. The CLI
uses the existing command with a live project:

```sh
scancraft-cli export-project-bundle EDITED.orproj SOURCE_LIVE_FOLDER NEW_FOLDER
scancraft-cli verify-source-bundle NEW_FOLDER/project.orproj NEW_FOLDER
```

The exporter verifies saved source fingerprints, copies all referenced artifacts
including both complete raw recordings, checks copied bytes, saves/reopens the
complete edited state and rechecks original sources before atomic publication.
Cancellation, changed sources or existing outputs do not publish over an existing
folder. Incomplete staging remains available for diagnosis. The original project
and its stored location are unchanged; only the exported copy refers to the new
folder. Undo/redo and measurement history use the existing full-state serializer.
Open the exported `project.orproj` explicitly. This is a project/source archive,
not a resumable acquisition session or the older scan-container folder schema.
If moved later, select the new folder explicitly for verification; stored paths
are descriptive. Source PLY and reports describe initial reconstruction, while
the project retains subsequent edits. No physical qualification is inferred.

Tests include a synthetic byte-identity fixture for relocation and edit-history
retention, cancellation and late source mutation, alongside unchanged single-frame
export regressions. This fixture’s dummy raw bytes are not sensor replay evidence.
