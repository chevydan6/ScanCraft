# Native recording playback

In Cameras, expand **Recording playback**, enter a complete `.raptor` path and
choose **Open recording**. The background worker validates every frame, global
order, footer and EOF before displaying the first image. Previous, Next and the
zero-based frame number navigate the indexed recording without rereading every
image. Close recording releases the source. Cancel playback stops loading and
clears the image; reopen to continue. Closing the app during a job offers the
existing operation cancellation flow. Camera acquisition is independent.

Startup shortcut:

```
scancraft-app --play-recording recording.raptor
```

The view displays dimensions/format, recording version/header protection,
calibration status, frame sequence, host time and raw sensor PTS. GREY displays
raw grayscale; MJPG decodes to RGB; Y14 uses the full little-endian 16-bit
container range for display only. No orientation, exposure normalization,
calibration or physical scale is inferred. Navigation is manual; timed playback,
audio, synchronized multi-camera replay and reconstructed geometry are not part
of this panel.

Recovered files can be opened after recovery, using `recovered.raptor` inside
the new bundle. Keep `report.json` with it: a complete recovered footer means
the retained prefix is structurally complete, not that the original scan was
complete. The panel warns about this; it does not authenticate recovery reports
or infer provenance from a filename.

Implementation: `scancraft-workflow::recording_playback` owns a read-only regular
file, rejects final-component symlinks/FIFOs and files over 1 TiB, and builds the
bounded index described in INDEXED_RECORDING_PLAYBACK.md. File and path device,
inode, length, mtime and ctime are compared after indexing and before/after each
selection, including decoding. Replacement/modification aborts the operation;
a selected frame also has its original CRC fingerprint checked. These are
accidental-change guards, not adversarial immutable snapshots or file locks.
Changes after a completed read are detected on the next navigation, rather
than continuously monitoring an idle displayed frame.

JPEG decoding has profile width/height limits, exact dimension agreement and
a 128 MiB decoder allocation limit; recording dimensions are already limited
to 16,777,216 pixels. Images and index entries are bounded; construction still
reads the full source once. There is one worker job and at most one selected
image/result, with cancellation checks around frame reads and decoding.
Cancellation is cooperative, not an interrupt inside a filesystem read or
codec call. Job failure/cancellation clears the old texture. Idle playback does
not schedule periodic repaint. This does not establish the hour-long memory or
live latency gates.

Verification:

- Five workflow tests exercise exact GREY pixels/timestamps, nonsequential
  navigation, source preservation, replaced/modified sources, symlink/FIFO
  refusal, cancellation, damaged footer, valid/invalid JPEGs and dimension
  disagreement, and explicit Y14 display mapping.
- The GUI worker regression opens, navigates, cancels, rejects a source change
  and corrupt reopening, clears stale images and successfully reopens a repaired
  source. A separate parser test covers the explicit startup flag.
- Workspace tests, strict all-target Clippy, formatting and release binaries/
  examples pass. See evidence/recording-playback-*.log. Python tool tests use
  `.venv-reference/bin/python` (system Python lacks the reference OpenCV module).
- evidence/recording-playback-hardware.json exercises the exact same workflow
  and decoder as the GUI on eight genuine frames from each IR camera and color,
  reading last/first/middle/last. Repeated selected RGB checksums match, sources
  remain unchanged and reported camera settings are restored. Raw scanner data
  is private and removed after testing. This is not GUI button automation,
  stereo synchronization, live throughput or physical geometry qualification.
- evidence/recording-playback-gradient-render contains three isolated-display
  startup screenshots of a generated synthetic gradient/checker pattern, no
  camera descriptors, and capture metadata. Generator:
  `cargo run --release -p scancraft-recording --example playback_fixture -- NEW_FILE`.
  Then `python3 tools/check_workbench_render.py NEW_FILE NEW_EVIDENCE_DIRECTORY
  --play-recording --captures 3`. The generator refuses to overwrite files.

Calibration remains deferred; this is offline image inspection, not completion
of calibrated scanning, tracking, reconstruction or the master mission.
