# Persistent recorded-sequence references

New fused recorded-sequence projects retain initial sequence metadata in ORPROJ07:
session ID, declared source kind, calibration record ID, ordered selected indices,
per-frame integration decisions and fingerprints of the retained artifacts. Editing,
measurements, undo/redo and standalone save/reopen retain this metadata separately
from geometry history. It describes initial inputs and decisions, not current
per-point correspondence or newly verified geometry.

The selected raw frame recordings, full calibration/request contents and complete
reports remain external. Keep the complete sequence result folder. References alone
are not disk verification: project loading performs no external filesystem access,
and works even when the folder is missing. The stored directory is descriptive;
copying or moving a project does not silently relocate or verify its sources.

Use the existing explicit CLI command, or open **Recorded sequence sources** in
the Point clouds GUI and choose a folder to verify:

```
scancraft-cli verify-source-bundle PROJECT SEQUENCE_FOLDER
scancraft --verify-source-bundle PROJECT SEQUENCE_FOLDER
```

Verification compares saved byte counts and CRC32 fingerprints against the chosen
folder. It includes sequence-request.json, report.json and the initial points.ply,
plus eight files in each frame-NNNN directory: report, selected left/right recordings,
calibration, reconstruction request, recording-sources sidecar, points and manifest.
Three frames therefore have 27 references. The project itself and the root manifest
are excluded to avoid self-referential project/manifest checksums. The root manifest
still fingerprints the saved project in the normal publication workflow.

This is retained-artifact identity checking, not full replay, semantic revalidation
of all report fields, cryptographic authentication, historical capture completeness,
whole-original-recording fingerprinting or physical calibration. CRC32 protects
against accidental corruption, not deliberate collision/forgery. A matching file
is compared to a producer-recorded reference, not independent physical truth.
Missing, size-changed or CRC-changed files fail verification. Nonregular files and
symlinks are rejected; nested frame parents must be real directories, with their
identity/timestamps checked again after streaming verification. Files are guarded
around reads and at completion using the existing verifier. These are point-in-time
filesystem checks, not an adversarial atomic snapshot guarantee. Cancellation is
cooperative between bounded reads. Verification never mutates references or geometry.

ORPROJ02..06 remain readable and keep their prior save version when sequence metadata
is absent. Older binaries reject ORPROJ07 rather than silently losing its references.
Version/magic mismatches, older-format headers containing sequence metadata, missing
v7 metadata, duplicate/unsafe/incomplete artifact names, invalid order/budgets and
qualified flags are rejected. Single-frame origin/complete-recording references and
sequence references cannot coexist as contradictory initial provenance.

Metadata is bounded to 1000 strictly ordered frame selections and 3+8N fixed safe
artifact names, with an 8MiB serialized metadata ceiling and the existing 64MiB
project ceiling. Stored indices are below the million-frame recording bound.
The producer derives fingerprints directly from written artifacts. All binding,
synchronization and physical-accuracy fields remain false, including after a
successful explicit fingerprint check. Root report, selected frames and calibration
remain in the external bundle rather than copied into the project metadata.

Implementation: geometry sequence_references.rs, Editor sequence attachment/history
validation, formats.rs v7 reader/writer; workflow recorded_sequence.rs artifact
collection and reference-bearing editor; shared source_verification.rs; GUI
source display/verification dispatch. This is project-local serialization and
orchestration using existing serde/CRC implementations, not an imported algorithm
or new dependency. No production scanner control or calibration activation changes.

Tests cover version7/history round trips without sources, legacy save behavior,
CRC-valid downgrade/missing/qualified metadata rejection, unsafe/incomplete/duplicate
references, moved-folder verification, modified artifact detection, symlink-parent
refusal, and actual CLI/GUI reference preservation and verification. Earlier v2..v6
and reconstruction/sequence regressions continue to run in the workspace gate.
The previous GUI/CLI 29-file byte equivalence was for projects without these new
references. New project files legitimately differ between output locations in the
descriptive directory string, and root-manifest project CRCs reflect that change;
geometry and initial references should otherwise agree after that normalization.
