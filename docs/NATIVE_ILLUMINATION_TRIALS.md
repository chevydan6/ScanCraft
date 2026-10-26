# Native illumination status and temporary flood capture

```sh
scancraft-cli illumination-status
scancraft-cli stereo-flood-capture 60 NEW_DIRECTORY --fps 30
```

The snapshot sends GET_PROPERTY for 17 fixed SDK illumination-related IDs.
It preserves exact raw integer words and unsupported errors. Some device
replies report success while containing inconsistent ranges; these are
explicitly marked unusable for setting. A reported switch state is not proof
of physical emission, scan readiness or correct camera-mode binding.

The capture trial can write only flood boolean property 6. It requires an
exact consistent 0..1 range, saves the original state, sends a fixed boolean
setter and checks readback. It holds the exclusive session lock for capture
and restoration. No laser, mode, power/current, calibration or firmware
setting is writable through this interface.

Each recording header stores that flood was reported enabled before stream
setup, with physical emission explicitly unverified. The capture joins both
workers, verifies flood still reported enabled, restores the original boolean
and checks restoration before publishing a completed directory. Other
control/calibration context remains unknown. An unsuccessful restore returns
an error and prevents publication; Drop attempts fallback restoration. A
crash, kill or disconnection can prevent restoration and requires checking
the device before proceeding. Existing output directories are never replaced.

The first real 30 FPS trial saved 60 valid frames per camera with complete
CRC/footer replay and successful 0->1->0 readbacks. Every sampled vendor and
standard V4L2 control value matches before/after. Brightness increases, but
the current blurred view yields no mutual SIFT matches. This does not qualify
physical emission, synchronization, reliability, calibration or geometry.
Raw captures/images remain private. See `evidence/native-flood-trial-summary.json`.

`evidence/illumination-property-mapping.json` records the stock SDK hash,
metadata ID/name references and protocol read-path references. Raw snapshot
results are in `illumination-status-native.json`; post-trial results are in
`illumination-status-after-flood.json`. Proper scene positioning, supported
mode behavior and independently verified correspondences remain necessary.

A separate later `stereo-line-capture` interface now tests only the supported
fixed line-laser boolean 151, with its own restoration guard. On/off raw imagery
shows genuine line-pattern activation; see `NATIVE_LINE_TRIALS.md`. This does
not expand the flood setter into arbitrary mode/power control or qualify a
complete scanner mode.
