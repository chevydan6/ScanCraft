# Scoped flood capture with explicit gain

## Purpose and implementation

The measured yellow box had weak ambient monochrome contrast. The earlier flood
trial used raw gain 1,000; later ambient trials used 15,000. Those captures did not
isolate illumination at equal gain. The native CLI now supports:

```sh
scancraft-cli stereo-flood-capture 10 NEW_DIRECTORY --fps 30 --gain 15000
```

Gain is explicit and remains subject to both cameras' actual descriptors; 15,000
is an observed limit for this device, not a portable constant. Without `--gain`,
the original flood behavior and default 50 FPS remain. The optional flags work in
either order; duplicate, incomplete, malformed and unknown flags are rejected.

`crates/scancraft-cli/src/capture_options.rs::parse` is the existing line-option
parser, renamed for sharing by line and flood capture. Its missing-value error now
says “capture” rather than “line capture”. `main.rs` dispatches to the additive
`stereo_probe.rs::record_flood_gain`, which calls the existing
`record_trial(..., Some(gain), IlluminationTrial::Flood)`.

The existing `ScopedIrGain` and `control::flood_trial::ScopedFlood` retain descriptor
checks, readbacks, scoped restoration and session locking. Both restore before
successful publication. Gain is checked after stream startup in each recording
header; flood enablement is reported before setup and checked after capture.
The manifest includes both trial/restoration reports. No new device-control
implementation, exposure/current/mode/firmware command, dependency, recording format
or reconstruction algorithm is introduced. Raw gain is not a physical gain unit.

## Validation

The shared parser's existing unit tests retain defaults and flag-order coverage.
`native_laser_preflight.rs::illumination_gain_cli_rejects_ambiguous_options_and_invalid_rate_before_capture`
now exercises both commands against duplicate gain/FPS, unsupported energy, missing
value, signed-integer overflow and invalid FPS in both flag orders. All cases fail
before hardware access or output creation. The focused test passes.

Full `tools/check.sh`: **904 passed, zero failed, one ignored**, formatting, strict
all-target Clippy and optimized workspace build pass. The test count is unchanged
because existing regressions were extended. All hardware evidence below uses the
exact archived CLI built from this source; build and later commit bindings are
retained privately. Older no-gain control code and line capture are preserved;
the live scanner workflow is not changed by this CLI addition.

## Actual equal-gain comparison

A frozen plan captures ambient A, flood, then ambient B, ten frames per camera at
raw gain 15,000 and requested 30 FPS. The scanner and box remain in the same setup.
The baseline line/flood properties are checked off; the original installed kernel
is unchanged. Existing scanner leases, idle-media/consumer checks, format saving
and seven GET-state comparisons remain. All three native commands exit successfully,
all 60 saved frames pass complete replay, both gains and flood restore, all saved
formats/readbacks agree, and WirePlumber returns to its original active state.

| Capture | Startup rejects L/R | Ongoing rejects L/R | Sequence gaps L/R | Equal-PTS pairs |
| --- | --- | --- | --- | --- |
| Ambient A | 1 / 1 | 0 / 0 | 0 / 0 | 10 |
| Flood | 0 / 103 | 0 / 0 | 0 / 0 | 8 |
| Ambient B | 93 / 1 | 1 / 1 | 1 / 1 | 10 |

These are short samples, not a reliability qualification or evidence that flood
causes/reduces rejection. Native records retain every reported failure. Equal PTS
and the existing host-window test do not establish synchronized exposure.

All ten frames are averaged per camera for photometric inspection, without frame
selection or alignment. Original frames remain intact. Fixed interior patches
were chosen from the earlier face image before inspecting this capture:

| Camera / patch | Ambient A mean | Flood mean | Ambient B mean |
| --- | ---: | ---: | ---: |
| Left / top | 23.782 | 32.069 | 23.765 |
| Left / right face | 21.959 | 27.885 | 21.958 |
| Right / top | 22.812 | 30.652 | 22.766 |
| Right / right face | 21.977 | 27.948 | 21.944 |

Values are raw 8-bit pixel intensities. The fixed-range display shows clearer box
faces/edges with flood enabled and brightness returning near baseline afterward.
This supports a useful visibility improvement in this setup, not a measured
geometric-accuracy improvement. Noise also increases: left top temporal RMS standard
deviation is 2.611 / 3.451 / 2.651 intensity units. Background patches and all other
statistics remain in the private report, including saturation counts. Eighteen
patch means/temporal variances/counts are cross-checked with scalar integer sums
and sample-moment formulas; all native gain/flood restoration reports pass.

## Corner-measurement failure retained

Four physical corner hypotheses are chosen visually from the flood images before
triangulation, with the native camera model and quality limits unchanged. The
numeric input is the rounded ten-frame mean, without display smoothing. Existing
native `corners::refine_corner` runs unrefined, radius-5 and radius-10 cases.

The unrefined front-top/bottom seeds yield 41.305892 raw units; both top-side edges
fail the stereo ray-gap gate. Neither refinement case yields a complete usable
edge: seed-displacement failures remain. Radius 10 accepts only the front-top
corner. No value is chosen for closeness to the owner's measurement, no acceptance
threshold is widened, and the rough seed distance is not presented as measured
box height. All three accepted point pairs agree with independent numerical
triangulation within 1.60e-12 raw units; that does not validate feature identity.
The owner's dimensions remain held out.

This rules out claiming precise corner measurements from the current procedure.
A useful next investigation is the distance between observed top and supporting
turntable planes, with correspondence, marker thickness and calibration uncertainty
reported explicitly. It must use genuine points and cannot impose the known height.

## Private evidence and limits

Roots `flood-gain-capture-20261104`, `flood-gain-integration-20261104` and
`box-ambient-edge-measurement-20261104` preserve plans, raw recordings, native
extractions, source/binary bindings, tests, scalar checks, scientific images and
original failures. Manifest `flood-gain-milestone-20261104/evidence-manifest.json`
binds 449 files / 513,526,284 bytes, SHA256
`f91c5b550484951e0ea45f270702fa491bd294e7a59cbb7a1dec8a2f6d9ee081`.
Private images/models and executables are not published.

Preparation initially used the wrong application binary filename and failed before
capture; the corrected path is `target/release/scancraft`. The initial private
photometry script name shadowed Python's `inspect` module and failed before
processing; renaming the identical script fixed it. A display-only plotting syntax
error was corrected separately. These failures/receipts remain; none required a
production-code change or repeated hardware acquisition.

Reproduction requires authorized private evidence: verify manifests/bindings, replay
all six recordings, use `photometry.py` with a fresh output root, independently run
`verify_photometry.py`, and reproduce the frozen corner request using the bound
native probe. Never overwrite original sealed roots. The source is ready for
independent review, not merged. CodeRabbit availability is recorded separately.
Physical units, complete reconstruction, dimensional accuracy, three complete
measured scans and native edit/save/reopen/export qualification remain outstanding.
