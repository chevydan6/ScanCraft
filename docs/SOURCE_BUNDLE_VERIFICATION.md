# Explicit source-bundle verification

Open an origin-bearing project, expand **Initial reconstruction provenance**,
enter a bundle directory and choose **Verify source bundle**. **Use recorded
location** fills the historical directory; a moved bundle can be selected by
entering its new directory. Verification runs in the existing background worker,
with progress/cancellation. Geometry, edit history, measurements, dirty state and
saved references remain unchanged. The result reports matching artifact count
and per-file failures. Successful verification does not retarget or save the
project. Loading a project alone still does not inspect external source files.

Explicit startup and CLI forms:

```
scancraft-app --verify-source-bundle PROJECT BUNDLE_DIRECTORY
scancraft-cli verify-source-bundle PROJECT BUNDLE_DIRECTORY
```

The app loads the selected project, then verifies the explicitly supplied bundle.
CLI prints JSON; mismatched bundles return nonzero after the report. Invalid
projects/directories and cancellation return controlled errors. No scanners,
firmware, illumination controls or calibration activation are involved.

The verifier checks the five artifact references from the project's validated
origin: names, expected byte lengths and CRC32. It uses a reusable 64 KiB buffer,
checks cancellation between reads and bounds each read to its declared length
plus one byte (at most 128 MiB + 1 per artifact, five references total). A size
mismatch is rejected before payload reading. Missing/unreadable files, unexpected
file types and checksum mismatches produce individual statuses rather than hiding
other failures. Final-component symlinks and non-regular artifacts are rejected;
the chosen directory itself must be a directory rather than a symlink.

Open-handle and pathname identity/length/mtime/ctime are compared around hashing.
A final pathname check catches changes to previously hashed files, and directory
identity/timestamps are checked across the operation. These are point-in-time,
accidental-change guards, not file locks or an adversarial immutable snapshot.
Parent directories can contain symlinks; this is not a filesystem sandbox.
Unrelated directory-entry changes may conservatively fail the directory check.
Cancellation does not interrupt an in-progress filesystem read.

A pass means the retained artifact bytes match the saved noncryptographic
references under these checks. It does not authenticate the origin, validate all
artifact formats, qualify physical calibration/device binding, establish exposure
synchronization, or continuously monitor later modifications. JSON flags for
binding, synchronization and physical accuracy stay false. Normal reconstruction
still independently validates calibration and raw recording semantics.

Verification evidence:

- Five workflow tests cover moved-directory matches and unchanged bytes/references,
  same-size CRC corruption, wrong sizes, missing files, symlink/FIFO refusal,
  cancellation before/after reads, deterministic modification of an already hashed
  file before final checking, directory moves, project framing/origin errors and
  unchanged project bytes.
- The existing GUI worker round-trip regression now checks verification before
  and after moving a bundle, a failed check after damage, cancellation and exact
  document preservation. A new UI test checks startup load-then-verify and retention
  of an already dirty state; a parser test requires both explicit paths.
- 444 workspace Rust tests / 17 reference-tooling tests pass, with strict all-target
  Clippy, format and release binaries/examples. Logs:
  evidence/source-verification-*.log.
- evidence/source-verification-proof copies a synthetic bundle to a different
  directory: CLI verification passes, a same-size request corruption returns a
  checksum failure/nonzero exit, and restored bytes pass. Original/restored SHA256s
  include the unchanged project. evidence/source-verification-gui-render shows the
  expanded native panel and completed 5/5 result on this relocated bundle without
  camera descriptors. Render scope is startup verification, not full click coverage.

Reproduce native inspection using the committed synthetic bundle:

```
python3 tools/check_workbench_render.py evidence/source-verification-proof/relocated-bundle NEW_EVIDENCE --verify-source-bundle --captures 3
```

Persistent reference relocation, self-contained raw/session packaging, multi-source
lineage and calibrated scanner qualification remain work. Physical calibration
remains deferred. This feature does not complete the master mission.

Verified retained inputs can now be exported with an edited project to a new
selected-frame bundle. **Use project folder** selects a moved project's directory
for an explicit check. The active project's references remain unchanged; the
exported copy gets its new directory. See PROJECT_BUNDLES.md.
