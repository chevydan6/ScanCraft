# Native board preparation panel

The **Calibration** tab runs the existing recorded-board observation workflow on
a background worker and displays both source camera views with marker decisions.
Start it from the app, or use:

```sh
scancraft --observe-calibration-pair REQUEST_JSON NEW_OUTPUT_DIRECTORY
```

Supply an [observation request](RECORDED_TARGET_OBSERVATIONS.md) and a new output
folder, then select **Measure recorded board**. The request retains explicit raw
recording paths, frame indices, target coordinates, unit declaration and detector
settings. This panel does not infer those settings or acquire a new camera pose.

The result shows selected camera images, target indices, per-camera retained
counts and the stereo intersection. Green marks are ordinary observations retained
by the shape gate; yellow marks are other detected components, including anchors;
red marks failed the shape gate. The optional contrast stretch changes only display
pixels. Thumbnails preserve camera orientation; neither reflection nor image
normalization changes fit inputs. These counts are image observations, not 3D
points or a physical-accuracy score.

## Implementation and lifecycle

`board_ui` owns one cancellable worker and its presentation state. The workflow's
new `run_with_frames` returns the published manifest/measurements and original
selected frames; existing `run` still returns the unchanged manifest. Preview
construction uses these same in-memory frames, avoiding a reread of files that
could change after publication. Existing `FrameView` computes full-frame statistics
and bounded 320-by-240 thumbnails. Raw frames are released after preparing display.

The texture and overlays share an explicitly allocated, aspect-correct rectangle.
Using the widget response rectangle was incorrect inside justified columns and
was caught during actual rendered-image inspection before publication.

Cancellation before publication follows the existing workflow. If display
preparation fails or is cancelled after successful publication, the app still
reports the saved directory and preserves the previous view with its original
path. Root window-close confirmation and cancellation include the board worker.
Thread cleanup has no claimed hard wall-clock deadline. This is independent of
scanner capture ownership and never automatically installs calibration.

## Evidence and remaining work

Source commit `e7679be6384e720cd686b207a4b00279725bfa44`:

- Full gate: **842 tests passed**, one existing ignored test, formatting, strict
  all-target Clippy and optimized workspace build passed.
- Three new app tests cover argument/path handling, close confirmation, and saved
  status/previous-view handling after preview failure or a subsequent worker error.
  The workflow regression additionally verifies returned frame/report identity.
- Four genuine saved board cases retain all preceding CLI artifact bytes exactly.
- The native app processes the held-out real board request and saves all bundle
  files byte-for-byte equal to the previous CLI result. The final inspected render
  shows aligned overlays, 38/36 retained marks and 36 paired observations. A
  missing-source request displays an error and publishes no output.
- Both final private-display runs remain alive at capture, with no video/USB
  descriptors found at that snapshot. Final executable hashes match the tested
  build. These are software-rendered startup tests, not complete interaction,
  GPU throughput or live FPS/latency qualification.

Private evidence is retained under developer artifacts `native-board-panel-mhuq9fbw`:
source/binary hashes, gate logs, exact outputs, reproduction scripts and initial/
corrected render images. No private captures or target coordinates are published.
CodeRabbit quota lookup remains `lookup_failed`; no review approval or paid request.

Multi-pose fit assembly, saved observation-bundle reopening, guided target setup,
calibration activation, physical scale/accuracy qualification and the first real
object surface remain unfinished. The existing request format remains explicit;
this is not an automatic calibration wizard. See [real-board evidence](REAL_BOARD_CALIBRATION.md).
