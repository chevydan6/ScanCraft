# Temporary native IR gain capture

```sh
scancraft-cli stereo-gain-capture 60 3000 NEW_DIRECTORY --fps 30
```

This diagnostic changes only standard V4L2 gain on discovered IR interfaces
0 and 2 of exactly one Raptor. It holds the exclusive session lock, checks
both original settings and reported range/step/flags before writing, and
limits trials to raw values 1000..15000. Raw gain units are not physically
interpreted. It does not change exposure, laser/projector, firmware or calibration.

Each write requires matching G_CTRL readback. After stream startup each
worker checks gain again; that observation is stored in its versioned,
CRC-protected recording header. Other calibration/control state stays
explicitly unknown. This observation does not prove a volatile control stays
constant for every exposure.

The capture joins both workers and restores both original gains before
publishing a successful directory. Ambiguous write failures are treated as
potentially applied and rolled back. Restore attempts run for both cameras
even if one fails. An explicit restoration error prevents successful output
publication; Drop provides a fallback attempt. A crash, kill or physical
disconnection cannot guarantee successful device restoration. Errors include
the original raw values where restoration is unresolved. Partial files remain
available for diagnostics and existing output directories are never overwritten.

Actual trials at 1000, 3000, 8000 and 15000 raw gain each saved 60 accepted
frames per camera with integrity replay and restored the original 1000 gains.
All standard-control snapshot values match before/after. A final hardware
regression after rollback refactoring captured 30 frames each and restored.
Failure-injection tests cover an error after the second write takes effect,
and one-camera restoration failure without skipping the other camera.

Intensity increases with gain, but the current view remains blurred and
provides no adequate common feature set. The highest trial saturates parts
of the left image. These trials do not qualify synchronization, reliability,
optical decoding, camera calibration or geometric accuracy. See
`evidence/ir-gain-trial-summary.json` and per-gain epipolar diagnostic summaries.
Raw records and images remain private. A controlled stationary textured
subject and verified illumination/exposure setup are still required.
