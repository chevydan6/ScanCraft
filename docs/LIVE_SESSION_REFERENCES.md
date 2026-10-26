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
X display: the intact synthetic bundle reported8/8matches, while a copied bundle
with modified left raw bytes reported7/8 and left.raptor:crc_mismatch. The
three-point cloud and reference panel stayed loaded; no camera/USB descriptor
was open. This is synthetic GUI verification evidence, not scanning qualification.
