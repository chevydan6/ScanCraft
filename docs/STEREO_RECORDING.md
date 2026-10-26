# Native dual-stream recording and frame extraction

```sh
scancraft-cli stereo-capture FRAME_COUNT NEW_OUTPUT_DIRECTORY [--fps FPS]
scancraft-cli extract-frame RECORDING ZERO_BASED_INDEX NEW_OUTPUT_DIRECTORY
```

`stereo-capture` uses the established native V4L2 path for interfaces 0 and 2,
1920x1200 packed GREY at 50 FPS by default; an explicit --fps selects another
enumerated IR rate for controlled experiments. One shared scanner/bridge session lock prevents
competing supported clients. Close other scanner software first. Each camera
runs in its own worker and writes every accepted frame directly to a version-2
raw recording. Recording backpressure can cause upstream gaps; counters are
retained and are not silently converted into saved frames. Preview is absent.

The count is 10..30000 accepted buffers per camera. At the current image size,
100 per camera requires approximately 461 MB; 30000 per camera requires about
138 GB. Choose storage deliberately. Raw pixels, kernel metadata, sequence and
host timestamps are preserved. Calibration and control settings remain explicitly
unknown in recording context; capture does not guess them or initialize a scan
mode. Firmware, calibration and projector commands are not written by this path.

A worker failure cancels the other worker, joins both and retains incomplete
staging output. The existing capture cleanup restores camera formats/rates.
Successful files must pass complete frame CRC/order/footer replay and expected
frame count before publication. A JSON manifest records all diagnostic counters
and timestamp samples. Same-parent staging and Linux atomic no-replace rename
prevent overwriting an existing output directory. The result holds `left.raptor`,
`right.raptor` and `manifest.json`.

Timestamp intersections and nearest-host associations remain diagnostics.
Equal sensor PTS and close host timestamps do not prove exposure synchronization
or optical calibration. Rejection counters count returned V4L2 buffers, which
can include malformed fragments; they are not a physical exposure-loss count.

## Selecting a recorded frame

`extract-frame` consumes and validates the complete recording, including frames
after the requested index and its footer, before returning the selected frame.
It retains only the selected and current frame in memory. Missing indices,
truncation or later corruption fail explicitly. Packed GREY and existing image
pixel-count bounds are enforced before export.

The new output directory contains:

- `image.png`: the selected original 8-bit image, without contrast or geometry changes.
- `frame.raptor`: the exact selected raw payload and metadata in a one-frame v2 recording.
- `manifest.json`: original index/count, profile/context, timestamps, flags,
  payload and metadata CRC-32, source recording version and explicit unverified
  synchronization/accuracy state.

Legacy source headers lack v2 CRC protection; the manifest distinguishes this.
A legacy source's absent context remains null in diagnostics, while the new
one-frame recording uses explicit unknown context. No missing context is invented.
The output is published atomically with overwrite refusal, like dual recording.

A selected PNG pair can feed `reconstruct-stereo` with a genuine corresponding
calibration and request. Retain both raw selections/manifests and source files;
PNG alone loses frame timing and context. Selection by index or timestamp is
not a verification that two physical exposures match.

## Current hardware evidence

A native trial saved 100 accepted frames per camera and passed complete raw-file
CRC/footer replay. Left rejected 3 returned buffers and reported 2 sequence gaps;
right rejected 2 and reported 1 gap. Neither reported duplicate sensor PTS.
There were 72 exact sensor-PTS intersections. The first saved pair had equal PTS
and an 8 us host-timestamp difference. These are transport observations, not
exposure or geometry qualification.

Raw data and selected image samples are private under `evidence/private` and
excluded from git. Public manifests and numeric image statistics are saved.
Visual inspection of indices 0 and 50 shows dark images, with pronounced left
horizontal banding. The left sample intensity range is 14..53, while the right
sample ranges are 14..27 and 15..27. The cause is unresolved; exposure,
illumination, format decoding and transport/content artifacts require controlled
investigation. Accepted frame structure does not establish usable optical data.
No real point cloud or scanning-mode qualification is claimed.

An acquisition-only 30000-buffer-per-camera soak was started separately with
`stereo-probe`, without saving image payloads. It has now finished with 30,000 accepted buffers per camera. See
evidence/native-stereo-soak-01-summary.json: duration exceeds ten minutes for
both cameras, but rejection proxies exceed threshold and the left accepted
rate is 36.5 Hz versus 49.5 Hz on the right. This is not full qualification.
