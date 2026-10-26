# Archiving complete supplied recordings

Cameras → **Archive complete supplied recordings** accepts left/right native
recording files and a new output directory. **Validate and archive full recordings**
runs in a background worker with cancellation and close handling, without opening
camera streams. It copies each entire supplied file, including every saved frame,
original header/context, timestamps, flags, UVC metadata and footer. Originals are
read-only; output is never a normalized/re-encoded replacement.

Commands:

```
scancraft-cli archive-recordings LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY
scancraft-app --archive-recordings LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY
```

Output is `left.raptor`, `right.raptor` and `manifest.json`. Open either archived
recording with native recording playback for indexed inspection. The manifest
records source display paths, full-file byte lengths/CRC32, versions/header
protection, profiles/contexts, frame counts, first/last sequence/host/PTS values,
and internal accepted-sequence gaps. Sequence gaps are sequence increments,
not independently measured lost physical exposures.

The streaming reader validates profile/header protection where supported, every
frame's lengths/CRC/metadata/order, footer count and EOF while a tee copies bytes.
After output sync, copied byte counts/CRC must agree and the complete saved file
is replayed again with identical metadata/statistics. Legacy RAPTOR01 is copied
unchanged; its original header remains explicitly unprotected and context unknown.
JPEG image envelopes have the reader's existing structural checks; codecs are not
decoded by the archive operation (`images_decoded: false`). Separate decoded replay
and native playback provide image decoding checks.

Each input must be a nonempty recording with at least one frame, at most 1 TiB
(and the platform-size bound) and at most 1,000,000 frames. Left must declare
interface 0 and right interface 2. Different counts/rates/timestamps are retained;
no exposure pairing, clock alignment, device binding or calibration is inferred.
A supplied recovered prefix can have a valid footer: keep its recovery report.
`original_capture_completeness_verified`, synchronization and physical-accuracy
flags remain false. These are complete *supplied files*, not proof that an original
scan contained every incoming exposure.

Copy I/O is limited to 64 KiB per step, with cooperative cancellation checks.
Memory retains the current parsed frame/body and bounded header/statistics rather
than all frames. Existing reader limits are 64 MiB per frame and 1 MiB per header;
this is not a large-scan memory qualification. Final source symlinks/special files
are rejected before opening; handle/path identity, length, mtime and ctime are
checked after copying and before publication. These are accidental-change guards,
not locks or cryptographic authentication. Cancellation cannot preempt a filesystem
read already in progress.

Publication syncs files/staging and uses no-overwrite atomic rename plus parent
sync. Errors/cancellation before publication report retained incomplete staging;
no validated prefix is silently published as a full archive. Destination races
cannot overwrite an existing directory. Parent-sync errors after rename explicitly
identify that the archive was already published. Successful publication commits
the output even if cancellation arrives afterward.

Verification:

- Six workflow regressions cover all-frame/context/byte preservation, legacy bytes
  and missing protection, truncated/corrupted suffixes, budget/role/empty inputs,
  cancellation before/after copying, source mutation, output races, overwrite refusal,
  symlinks and FIFOs. GUI worker and startup-parser tests cover native entry.
- 461 workspace Rust tests / 17 reference-tooling tests, strict all-target lint,
  formatting and release binaries/examples pass. Logs:
  evidence/full-recording-archive-*.log.
- evidence/full-recording-archive-hardware.json captures eight genuine frames from
  each IR camera sequentially, archives every supplied frame with matching original/
  output SHA256s, decodes all archived frames successfully, refuses existing output
  and restores all reported camera settings. Raw test images are private and removed.
  Acquisition rejection counters are reported separately; no synchronization,
  live throughput or geometry acceptance gate follows from this short test.
- evidence/full-recording-archive-gui-render shows native startup archiving of the
  complete three-frame synthetic inputs, exact output equality and no camera
  descriptors. This is startup workflow evidence, not full mouse interaction.

Reproduce native inspection:

```
python3 tools/check_workbench_render.py evidence/recorded-stereo-dataset NEW_EVIDENCE --archive-recordings --captures 3
```

This standalone archive complements selected-frame editable project bundles.
Associating the complete recordings with editable projects, recovery reports,
meshes and multiple scan sessions remains work. Calibrated genuine reconstruction,
tracking and full master-mission qualification remain unfinished; calibration is
deferred by user instruction.
