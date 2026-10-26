# Acquisition cadence and optical artifacts

Current evidence narrows a real native transport problem but does not identify
its exact cause or qualify a scanning mode. All rates below count accepted
V4L2 buffers over their host timestamp span, excluding initial startup time.
They are not independently measured physical exposure rates.

| Configuration | Accepted buffers | Left rate | Right rate | Returned-buffer rejects |
| --- | --- | --- | --- | --- |
| Initial dual 50 FPS soak | 30000 each | 36.502 Hz | 49.492 Hz | 375 / 301 |
| Isolated left 50 FPS | 1000 | 49.688 Hz | — | 99, including 93 before first accepted buffer |
| Isolated right 50 FPS | 1000 | — | 49.788 Hz | 6 |
| Dual 30 FPS | 1000 each | 29.907 Hz | 29.907 Hz | 5 / 3 |
| Later dual 50 FPS repeat | 100 each | 48.517 Hz | 49.987 Hz | 212 / 2 |

The 30 FPS short trial is not a replacement qualification target. Default
stereo requests remain 50 FPS; the CLI now accepts explicit `--fps` for
controlled comparisons. Its allowed rates match the measured IR profile
allowlist: 5, 10, 15, 20, 25, 30, 40, 50. Unsupported requests fail validation.
Single-stream summaries now retain first/last host timestamps and accepted rates.

## What the evidence says

In the initial dual soak, left sensor PTS intervals are commonly about 20005 or
40010 ticks. There are 10646 intervals exceeding 30000 ticks with V4L2 sequence
step one. On the right there are zero such intervals without a sequence gap.
Both aggregate sensor/host clock slopes are near one tick per host microsecond,
but exposure synchronization and clock interpretation are not independently
qualified. Sequence-gap counters alone do not capture the left cadence deficit.

Selected initial dual-50 left frames show pronounced horizontal banding. The
selected dual-30, isolated-50 and later dual-50 left frames lack that pronounced
banding, but remain dark. No contrast/geometry edits were applied to these PNGs.
The later 50 FPS repeat has many rejected startup buffers, followed by a higher
accepted rate and an apparently cleaner selected frame. This prevents attributing
the problem solely to a fixed bandwidth ceiling. Startup ordering, firmware
stream state, host/kernel queuing and payload assembly remain hypotheses.
These observations are not a quantitative image-quality qualification.

Exposure/gain were not written. A post-comparison standard-control snapshot
matches the earlier reported values. This checks reported values at rest, not
actual sensor exposure during each frame. Absolute exposure reports 3000; its
standard 100 us interpretation is not established physical timing.

## Reference research

The locally inspected OrbbecSDK_v2 revision
`85e94c4ee4032817df2e33228dbd7f51517d3d4e` maps IR exposure to the generic UVC
exposure property, then passes Linux control values without unit conversion.
See the official [UVC property accessor](https://github.com/orbbec/OrbbecSDK_v2/blob/85e94c4ee4032817df2e33228dbd7f51517d3d4e/src/device/component/property/UvcPropertyAccessor.cpp)
and [V4L2 backend](https://github.com/orbbec/OrbbecSDK_v2/blob/85e94c4ee4032817df2e33228dbd7f51517d3d4e/src/platform/usb/uvc/ObV4lUvcDevicePort.cpp).
This public source does not establish Raptor exposure units: no Raptor device
implementation was identified in that inspected source tree. Other-model
microsecond conventions must not silently replace the standard interpretation.

A search also found the independent [Logic Renewed Raptor Linux project](https://logicrenewed.com/scancraft-linux/),
whose primary page announces December 26, 2026 or sooner. The reviewed overview
and development-log pages provide claims/demonstrations, not usable source or
protocol details for this implementation. Its capabilities have not been verified
here; no code or assets were imported. It is related work, not evidence that
ScanCraft meets any requirement.

## Next controlled experiment

A separate Raptor-only queue-depth kernel candidate compiles and passes policy
checks, but remains uninstalled and untested on hardware. See
experiments/kernel-queue/README.md. It preserves the existing parsing/error rules
and defaults to five URBs, with an explicit 5..32 diagnostic override restricted
to the original Raptor IR interfaces. A deeper queue may help tolerate host
completion latency; only a controlled runtime comparison can establish that.

Raw recordings and PNGs remain private. Public summary evidence includes
native-rate-comparison.json, native-stereo-soak-01-intervals.json and the individual
trial JSON/log files. Firmware, calibration and projector state were not changed.

## 2026-10-26 counterbalanced lower-rate comparison

After the dual-30 ten-minute rejection-proxy failure, a new previously unmeasured
15 FPS condition was tested in 15/30/30/15 order. Each trial targets approximately
40 s of accepted samples: 600 per camera at 15 FPS, 1,200 at 30 FPS. All four
complete with one unchanged native binary, no concurrent compilation, and the
same frame validation. Source editing and lightweight diagnostic inspection ran
concurrently. Exposure/gain/projector/firmware/calibration were not changed.

| Trial | FPS | Left rejects / accepted gaps | Right rejects / accepted gaps | First accepted sequence L / R |
| --- | --- | --- | --- | --- |
| 15a | 15 | 1 / 1 | 99 / 1 | 0 / 98 |
| 30a | 30 | 9 / 6 | 7 / 6 | 3 / 1 |
| 30b | 30 | 100 / 9 | 9 / 8 | 91 / 1 |
| 15b | 15 | 3 / 3 | 5 / 3 | 0 / 2 |

Accepted rates range 14.924–14.973 Hz at 15 FPS and 29.749–29.847 Hz at 30 FPS.
Startup fragments dominate some total counts. The legacy first-32 sample cap can
hide later rejection details after those bursts; it cannot support a complete
phase classification. Accepted sequence gaps are not automatically equated with
explicit rejects or physical lost exposures. These short runs do not establish
causality or statistical superiority of one rate. Lower rate still rejects
buffers, and the startup burst can occur on the right as well as the left.

Evidence: evidence/stereo-rate-counterbalance-20261026/{design,comparison}.json
and each trial's native/assessment/run/progress files. This is a changed-condition
investigation, not a replacement ten-minute qualification. Native capture now
adds exact rejection phase counters and an independent ongoing sample to remove
the diagnostic blind spot. See REJECTION_DIAGNOSTICS.md for the subsequent new
instrumented run and the distinction between metadata flags and malformed PTS.
