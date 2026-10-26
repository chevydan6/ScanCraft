# Native line-pattern activation trial

```
scancraft-cli illumination-status
scancraft-cli stereo-line-capture 60 NEW_DIRECTORY --fps 30
```

This operational experiment uses only fixed SDK property 151,
`OB_PROP_LASER_LINE_BOOL`, established by stock SDK metadata and an actual
consistent 0..1 boolean response. It does not expose arbitrary property writes.
Unsupported/inconsistent ranges fail before mutation. It saves the reported
original switch state, enables that boolean with immediate readback, captures
both native IR streams, checks the switch again, restores the original state
and checks restoration before publishing. If originally on, restoration leaves
it on; this trial started and ended off. The scoped guard holds the exclusive
session lock and attempts fallback restoration on Drop. Disconnect, process
kill or crash can prevent restoration and require a subsequent status check.

No numeric line mode, energy/current, protection override, exposure, gain,
calibration or firmware is changed by this command. The shared capture path now
uses a typed None/Flood/Line choice; gain and existing flood capture interfaces
retain their previous behavior. Recording headers state reported enable before
stream setup, with physical emission/wavelength unverified. This is operational
line control, not a qualified scanning mode. No GUI continuous-laser control is
introduced yet.

Actual connected Raptor trial:

- Reported property 151 readbacks were 0→1→0, including enabled-at-end and
  explicit restoration. All 17 vendor snapshots and all sampled V4L2 standard
  controls match before/after; a final later status still matches.
- Both IR cameras saved 60 valid 1920×1200 GREY frames near 30 FPS. Each rejected
  one startup buffer; accepted sequences had no gaps or missing metadata.
  Recordings passed complete CRC/footer replay. All 60 selected sensor PTS values
  match between cameras; this still does not prove exposure synchronization.
- Exact-PTS pair 30/30 (host timestamp difference 11 µs) was exported only after
  whole-file integrity validation. Both raw images visibly contain a projected
  multiline pattern. Raw images, calibration and per-observation data stay private.
- Native `detect-stripes` with background 30, minimum peak 50, maximum width 12,
  saturation 255 and cap 100,000 accepted 294 left / 606 right observations.
  Saturated/broad/weak candidates remain explicit rejections. These are intensity
  runs, not identified projector lines or validated correspondences.
- A subsequent restored-off 60-frame dual capture was also integrity validated.
  The same detector parameters accepted zero stripe observations on each chosen
  image, and the inspected off image lacks the pattern. This supports actual
  optical line-pattern activation associated with the SDK switch, beyond merely
  USB communication or successful readback.

The cameras are monochrome: this does not independently verify blue wavelength,
manufacturer mode name or physical emitter specification. The scene remains
blurred. Image-only SIFT produced six mutual matches, with none under two
approximate pixels for either tested factory epipolar hypothesis; these matches
do not validate/reject the factory calibration or establish a point cloud.
Translation scale, active-mode/image binding, line identities, correspondences,
physical coverage and geometric qualification remain unresolved. Physical
calibration is still deferred by the user.

One new pure encoder/range regression constrains this module to property 151 and
boolean payloads. All 214 workspace tests, Clippy, formatting and release build
pass. Evidence: `native-line-trial-summary.json`, `native-line-trial-epipolar.json`
and native-line test/build logs. Raw control snapshots and on/off recordings
remain private. The runtime's physical-emission fields deliberately stay false:
its readback cannot inspect light; later offline image evidence is recorded
separately as an observed pattern, not spectral or metrological qualification.


Later offline detector work is documented in `LOCAL_LASER_DETECTION.md`.
Local background removal recovers more observations but also off-capture noise;
denser shifted-control comparisons do not validate the factory sheet mapping.
The original operational control/restoration evidence above is unchanged.
