# Raptor startup sequencing investigation — 2026-11-03

Confirmed result: startup rejection remains reproducible. Completing both format/
rate setups before starting video is not sufficient, and serializing STREAMON
alone is not sufficient. These trials do not identify the root cause or justify
changing production acquisition ordering.

## Source and scope

All captures used optimized CLI source `cf59693664e40507408ce44f10b91c565819ffd0`
(numerical source `68cf31fc4e6f4871c364c706aad22019903d9a3e`). The acquisition code
was unchanged. Relevant functions are `stereo_probe::{capture,run_workers,
record_trial}`, `scancraft_device::Capture::{open_inner,next_frame_until,reject}`,
`Stream::{new,dequeue,close}` in device `lib.rs`, and `stream_group::SettingsLease`.
Existing frame and metadata validation remained enabled throughout.

Private artifact `scene-readiness-gipig_qh` contains the exact executable/source
binding, raw recordings, readbacks, native reports, trace source/binary, scripts,
analysis and SHA-256 inventory. `trace_trials.py`, `start_order_trials.py` and
`analyze_start_order.py` reproduce the retained experiment and analysis. Do not
publish these device captures or target coordinates.

## Fresh scene and acquisition

A native ten-frame-per-camera flood capture showed the full calibration pattern
in both images. The selected pair has equal sensor PTS and a 23-microsecond host
timestamp difference; neither proves exposure simultaneity. Native observation
preparation retained 38 ordinary marks per camera with existing settings. Against
the preceding held-out image, common marks moved on average 5.25/5.19 pixels
(left/right), with maxima 5.89/6.12. **Engineering inference:** this is effectively
another observation of the same setup, not the diverse tilts/depths needed for
calibration coverage. It is not counted as an additional calibration training pose.

Initial startup rejections were 0/86 (left/right); ongoing rejections were 0/0.
Both recordings replayed with integrity checks and reported format/rate and
illumination readbacks matched before/after. This is a short acquisition check,
not sustained-rate qualification or a completed object scan.

## Instrumented observation and controlled comparison

A private Linux x86_64 `LD_PRELOAD` diagnostic intercepted ioctl calls, forwarded
the original requests through the syscall, and logged selected format/rate,
STREAMON/OFF and DQBUF boundaries. It retained request/result, thread/node,
monotonic timestamps and buffer counters, not video payloads. Diagnostic overhead
changes scheduling; these measurements are not performance benchmarks or USB-wire
observations. No kernel, firmware or production source change was made.

Three ten-frame-per-camera observation runs reproduced startup rejected-buffer
counts of 0/101, 0/99 and 102/91. In the second and third, all format/rate ioctls
completed before either video STREAMON began. Thus setup overlapping stream start
cannot by itself explain those two failures. This does not exclude effects of
concurrent earlier configuration or unknown device state.

A second diagnostic variant optionally held a process-local mutex around only
STREAMON. The declared sequence was A/B/B/A, 60 accepted frames per camera per
trial: A forwards the original concurrency; B serializes STREAMON. No requests,
frame acceptance rules or firmware contents changed.

| Trial | STREAMON serialized | Startup rejects L/R | Ongoing rejects L/R |
| --- | --- | ---: | ---: |
| A1 | No | 0 / 0 | 1 / 1 |
| B1 | Yes | 0 / 1 | 1 / 1 |
| B2 | Yes | 1 / 104 | 0 / 0 |
| A2 | No | 0 / 91 | 0 / 0 |

Trace analysis confirms no overlapping video STREAMON intervals in either B
trial. B2's 104 right startup rejections demonstrate that serialization alone is
insufficient. This sample cannot establish comparative probabilities or causal
attribution. Rejected buffers are not necessarily individual physical exposures.

An initial diagnostic build failed its strict format-type check. The shell then
ran one capture with the missing preload library ignored by the loader. That
extra uninstrumented capture retained 10 frames/camera and startup rejects85/86;
its loader warning, successful replays and readbacks are preserved as
`trace-0`, explicitly excluded from the instrumented experiment. The cast was
corrected, the library precondition added and subsequent build/run used `set -e`.
No trace evidence is claimed for the failed-instrumentation run.

Across the initial scene capture, that extra capture, three traced observations
and four controlled trials: **580 accepted raw frames** were retained in18
recordings. All18 native replay checks and all nine pairs of stream/illumination
before/after readbacks passed. Missing-metadata counters were zero in the seven
instrumented trials. Successful cleanup readbacks do not prove a hard cleanup
deadline, no unreported controls changed, or sustained reliability.

## Remaining interpretation and next experiment

Startup samples include tiny video buffers with kernel error flags and implausible
metadata/PTS values. These facts establish rejected driver-delivered data, not
whether device output, USB transport, UVC parsing or host configuration caused it.
The earlier [finite-completion investigation](FINITE_STEREO_COMPLETION.md) concerns
teardown and continuing rejection; it does not explain this startup failure.

A useful next experiment is a synchronized USB-monitor and native-startup trace
that retains monitor drop counters and enough transfer bytes to distinguish actual
UVC headers from payload interpreted as metadata. Read-only usbmon access currently
requires root on this machine; the noninteractive sudo probe required a password.
No privilege, permission or kernel changes were made during these trials.
Do not change frame validators or add arbitrary warm-up exclusions to improve the
reported rejection rate. All startup errors remain part of qualification evidence.

Production source remains at the previously verified850-test gate. Public CI
41620572099 completed successfully for cf59693. Hardware experiments do not replace
that source gate, independent review or the physical accuracy requirements. No
new CodeRabbit review is claimed for this evidence-only documentation milestone.

The subsequent [complete USB startup trace](RAPTOR_USB_STARTUP_FRAMING.md) obtained
privileged monitor access using existing owner authorization and retained zero
reported monitor drops. It identifies a concrete shifted-boundary incident without
changing production acquisition or attributing the ultimate cause.
