# Native capture repeatability — 2026-11-02

Application source: `9e72156970e75df96be9f57f0bff791edd8a5222`.
The exact executable hash, UTC interval, kernel and sanitized measurements are in
[capture-soak-2026-11-02.json](benchmarks/capture-soak-2026-11-02.json).
Private raw recordings, device context and detailed rejection samples remain local.
No firmware, calibration, illumination mode or transport implementation was changed.

Three successive `stereo-capture 30 NEW_DIRECTORY --fps 30` trials each produced
30 valid saved frames per camera and 30 equal-PTS pairs. Full recording replay
and reported stream-settings restoration passed in every trial. Trials one and
three had zero rejected buffers. Trial two had one startup and one ongoing
rejection per camera, with one sequence gap in each. Both ongoing rejections
had the same sensor PTS and 2,242,620 video bytes instead of 2,304,000, with
video/metadata error flags. This is evidence of a correlated fault, not proof
of whether the device, USB transport or host driver caused it.

A subsequent `stereo-probe 18010 --fps 30` completed with exit 0 and reported
settings restoration PASS. This diagnostic validates incoming payloads/metadata
but does not save raw frames. It uses `Capture::open`; recording trials use
`Capture::open_recording`, which additionally gathers setup observations. They
are not interchangeable recording-throughput benchmarks.

| Measurement | Left camera | Right camera |
| --- | ---: | ---: |
| Accepted frames | 18,010 | 18,010 |
| Accepted host-timestamp span | 603.642 s | 603.776 s |
| Accepted-buffer rate | 29.834 Hz | 29.827 Hz |
| Rejections before first accepted frame | 1 | 87 |
| Rejections after first accepted frame | 98 | 102 |
| Accepted-sequence gaps | 98 | 102 |
| Missing metadata / adjacent duplicate PTS reported | 0 / 0 | 0 / 0 |
| Ongoing rejected-buffer proxy | 0.5412% | 0.5632% |

The ongoing proxy is `ongoing_rejections / (accepted + ongoing_rejections)`;
startup errors are separately retained, not hidden. The JSON also reports the
all-buffer proxy including startup. Both interpretations exceed 0.1%. Counts
refer to V4L2 buffers and may not equal physical exposure counts. The overall
device reliability gate therefore remains unqualified; the ten-minute structural
duration criterion alone has new supporting evidence.

There were 18,007 equal raw sensor PTS values across streams. Equality does not
prove shared clock epochs or simultaneous exposures. The production live
association already requires equal PTS and an explicit host window, and still
labels pairs as candidates (`live_stereo::Association::push`). Do not pair by
equal frame index: earlier short recordings independently demonstrate offsets.
The existing `recording-timing` command revalidated six short recordings and
confirmed 9/9/10 pairs in the earlier board sessions and 30 in each new trial,
using a 1,000-microsecond host diagnostic limit. Those are diagnostic associations,
not new calibration or exposure-timing qualification.

The probe's child-process resource accounting reported 10.146 user CPU seconds,
0.717 system CPU seconds and 51,204 KiB peak resident memory over 604.468 wall
seconds. Offline analysis also ran on the workstation during the probe. This
is a single acquisition-only observation, not a dedicated CPU benchmark, full
pipeline latency result, 60-minute memory-growth test or comparison with CrealityScan.

Implementation references: `scancraft-cli/src/stereo_probe.rs` (`capture`,
`run_inner`, `run_workers`, `summary`), `scancraft-device/src/lib.rs`
(`Capture::next_frame_until`, `reject`, `close`), and
`scancraft-workflow/src/recording_timing.rs` / `recording_timing/raw_pts.rs`.
The probe retains error flags, rejection samples and sequence counters; no guard
was relaxed to make the results appear successful. Further work should isolate
the recurring buffer faults and qualify recovery while preserving this baseline.
