# Initial reconstruction provenance in editable projects

Recorded stereo reconstruction now embeds its initial source provenance in the
editable project and the live GUI document. Saving after trimming/filtering,
undo/redo, measurement changes or reopening retains this record. The Point clouds
panel has an **Initial reconstruction provenance** section showing the source
bundle reference, calibration ID, selected frame indices and artifact checksums.

This record describes the initial reconstruction inputs. It does not claim that
current edited point indices still map directly to original frames, or record
additional merged sources. Raw recordings remain outside the project. Reopening
a project does not open, execute, activate or validate external references. A
missing or moved source bundle does not prevent geometry/history reopening.
Keep the complete reconstruction bundle for reproducibility; the stored absolute
directory is a display reference and can become stale after moving files.

The stored record contains supplied calibration/request JSON, original recording
source/context/index/timestamp details and the byte length/CRC32 of five retained
artifacts: calibration.rapcal, request.json, recording-sources.json,
left-frame.raptor and right-frame.raptor. Checksums refer to the normalized retained
artifacts, not whole original recordings. They are accidental-corruption checks,
not cryptographic authentication. Metadata can contain user declarations; loading
it is not independent evidence of device identity or calibration quality.

`scancraft-geometry::project_origin` bounds this metadata to 8 MiB and requires the
current schema/kind, five unique supported artifact names, bounded references and
explicit false device-binding/synchronization/accuracy qualification flags.
Raw image payloads are not embedded. Origin attachment is once-only, before
geometry edits or measurements; it is retained separately from reversible history.
The existing overall project envelope remains limited to 64 MiB.

Projects with an origin and no complete-recording references use ORPROJ05/schema 5, with the existing length/CRC
framing. Projects without an origin continue using versions 2–4 as appropriate.
Versions 2–4 cannot carry an origin, and version 5 requires one; inconsistent
magic/schema and invalid metadata are rejected even with a recomputed checksum.
Older readers reject v5 explicitly rather than silently discarding provenance.
Schema 5 also carries the existing measurement and geometry history.

Verification:

- Five new geometry tests cover v5 save/reopen with trim/undo/redo, measurement
  history, late/duplicate origin attachment, v2 compatibility, forbidden downgrade,
  missing/invalid provenance, duplicate/unsafe artifact names, the 8 MiB bound,
  every truncation and every single-byte corruption of a small v5 fixture.
- The recorded reconstruction workflow regression checks saved/live origin
  equality and all five artifact byte lengths/CRC32s against published files.
- A GUI worker regression reconstructs, trims, moves the bundle, saves, reopens
  and undoes/redoes, preserving initial provenance while the referenced original
  bundle path no longer exists.
- Initial persistence checkpoint: 437 workspace Rust tests and 17 reference/tooling tests pass, along with strict
  all-target Clippy, formatting and release binaries/examples. Logs:
  evidence/project-origin-*.log.
- evidence/project-origin-gui-render reconstructs the bundled synthetic pair;
  evidence/project-origin-reopen-render opens its v5 project in a fresh native
  app. Both isolated displays open no camera descriptors. The independent Python
  envelope/artifact audit is evidence/project-origin-proof.json.

All render/reconstruction evidence here is synthetic. No genuine calibrated
scanner cloud, physical accuracy or full project/session milestone is qualified.
Full self-contained scan/session projects, merge lineage and persistent source
reference relocation remain work. Calibration remains deferred by user instruction.

Explicit retained-artifact verification is now available in the provenance panel
and CLI/startup commands, including checking a moved bundle at a selected directory.
Project loading remains independent; verification does not retarget references.
See SOURCE_BUNDLE_VERIFICATION.md for scope and evidence.

Native project-bundle export now copies edited state and the five retained input
artifacts to a new folder, updating the exported copy's descriptive source location.
The active document stays unchanged. See PROJECT_BUNDLES.md for scope and evidence.

## Persistent complete-recording references (schema 6)

Projects can now carry separate `recordings` metadata alongside the initial origin.
`scancraft-geometry::recording_references::Recordings` stores schema 1, a descriptive
folder, and exactly two ordered entries (`left.raptor`, `right.raptor`). Each entry
has byte length, CRC32, total frames, recording version and selected index. Names,
1 TiB/one-million-frame bounds, versions 1/2 and indices are validated against the
initial source lineage. A reference requires an origin. This is a fingerprint of
the supplied complete recording at packaging/verification time; it does not
retroactively attest every byte of the historical original capture.

These documents use `ORPROJ06`/schema 6 with the existing 64 MiB length/CRC envelope.
Versions 2–5 remain readable and retain their existing save versions when no complete
recording references are present. Versions below 6 cannot hide this field, and
version 6 requires both references and origin. Metadata validation performs no
filesystem I/O. Geometry and measurement histories retain the references through
editing, undo/redo, ordinary save and standalone reopen, even with recordings absent.

Scan-folder export writes references for its verified complete copies and final
recordings location. Explicit scan-folder loading adds references to legacy v5
projects in memory, or cross-checks existing references against container bytes,
then relocates both descriptive folders in memory. Packaged files remain unchanged.
Selected-frame-only project bundles retain complete-recording references but do not
copy the full recordings; these references remain external.

The native **Complete recording references** panel shows the saved fingerprints.
Choose a folder and request **Verify complete recordings** for bounded read-only
hashing, full replay and selected-frame membership checks. CLI equivalent:

```
scancraft-cli verify-project-recordings PROJECT RECORDINGS_DIRECTORY
```

Verification neither changes the project/references nor silently reads the saved
path. It shares regular-file/directory identity checks and cancellation with the
scan-folder loader. Physical binding, synchronization, accuracy, original capture
completeness and cryptographic authenticity are not established by CRC checks.
Automatic timed replay, resumed scanning, recovery provenance, multi-session and
mesh project integration remain unfinished.

Checkpoint evidence: 483 workspace Rust tests, 19 reference/tool tests, strict
all-target lint, formatting and release workspace binaries/examples pass. Three
new geometry tests cover exact v6 geometry/measurement history, version gates and
invalid metadata atomicity. Two new workflow cases cover saved edited references,
explicit verification failure and a CRC-valid project/container disagreement. The
native worker regression now saves/reopens v6 and explicitly verifies recordings.
See `evidence/recording-references-*.log` and CLI JSON reports. Three inspected
private-display captures show a standalone v6 project with three synthetic points
and both references while the recordings directory is temporarily absent. All nine
manifest artifacts still match their fingerprints after restoration. Legacy v5
container verification also passes. No real scanner geometry or release gate is
advanced; calibration stays deferred.
