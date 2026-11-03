# Sustained acquisition after metadata-order correction — 2026-11-03

**Confirmed:** the corrected native binary recorded 18,000 accepted frames per
camera over 600.275 seconds. All 36,000 saved frames replay. No ongoing empty
metadata buffer occurred, and every traced matching metadata return preceded
video-buffer reuse. Seven paired truncations and 96 right-camera startup rejects
remain. The all-rejection proxy is **0.304625%, failing the <0.1% target**;
excluding startup gives 0.038874%. Neither denominator is a physical exposure count.
This is acquisition evidence, not a calibrated object reconstruction or release.

## Source, configuration and reproducibility

Application implementation: `907484e4fa2967e45ca10840fc48f0eed523f124`.
Tested repository HEAD: `5943cd1893ed4ca510d6a53da655d78cf1a6ade1`.
Corrected release CLI SHA-256:
`3a46c4995b134912482af20a303e76c024ea6cea38e699935a6a4eef430cae55`.
No production source changes were made for this experiment. Relevant functions
and controlled causal evidence are in [metadata return ordering](METADATA_REQUEUE_ORDER.md).
The finite CLI uses the [bounded recording worker](BOUNDED_RECORDING.md), eight
owned frames per camera, unchanged validation and version-2 recordings.

Both trials request 1920 × 1200 GREY, 30 FPS, IR interfaces 0/2 and temporary flood
illumination. Command: `scancraft-cli stereo-flood-capture N OUTPUT --fps 30`.
A 300-frame/camera preflight precedes the 18,000-frame/camera sustained trial.
The existing temporary 32-URB candidate is reloaded before each trial; four video
and four metadata V4L2 buffers remain unchanged. Candidate source, reproducible
binary and the local `raptor_urbs`/published `scancraft_urbs` parameter-name mapping
are documented in [queue-depth trials](RAPTOR_QUEUE_DEPTH_TRIAL.md). No firmware
write or permanent driver installation occurs. This does not establish the same
rates on the installed baseline driver.

Private evidence directory: `sustained-ordered-s2y99uc1` under the developer
artifact directory. It retains `run.py`, `capture.py`, format snapshot helpers,
`source-binding.json`, diagnostic sources/build commands, raw native outputs,
complete recordings/replays, traces, `analyze.py`, resource samples/analysis,
image selector/source and restoration events. SHA-256 manifests bind retained
files. Private captures, device identifiers and raw observations are not in Git.
The native report and trace independently reconcile video-dequeue counts with
accepted plus rejected buffers; sample sequences independently reproduce gaps.

## Observer and resource limits

The original private buffered tracer is enlarged from 20,000 to **400,000 rows**
for this run. Fixed row/length storage is **308,800,000 bytes**. It is prefaulted
before acquisition; trace-file writes occur only at process teardown. A strict
warnings-as-errors build and a four-thread 400,000-event boundary self-test pass,
including a zero-byte trace file until teardown and zero lost rows afterward.
There is no injected delay, changed ioctl result, substituted image or weakened
validation. Bounds and final trace-error counts are checked explicitly.

CPU, memory, path lookup, formatting, syscall and scheduling observer costs remain.
Prefaulting does not prohibit later faults. The resource sampler also performs
small independent log writes every five seconds. This is normal-system-activity
measurement, not an isolated benchmark or proof that observation has no effect.
Do not compare whole-process RSS directly with the preceding smaller tracer or
subtract the static allocation and call the remainder measured application RSS.

## Capture and integrity results

| Observation | Preflight | Sustained |
| --- | ---: | ---: |
| Accepted frames L/R | 300 / 300 | 18,000 / 18,000 |
| Startup rejects L/R | 0 / 91 | 0 / 96 |
| Ongoing rejects L/R | 1 / 1 | 7 / 7 |
| Accepted sequence gaps L/R | 1 / 1 | 7 / 7 |
| Accepted missing metadata / duplicate PTS | 0 / 0 on both | 0 / 0 on both |
| Ongoing rejection proxy | 2 / 602 = 0.332226% | 14 / 36,014 = 0.038874% |
| All-rejection proxy | 93 / 693 = 13.419913% | 110 / 36,110 = 0.304625% |
| Complete trace rows, including footer | 5,405 | 288,707 |
| Trace errors / lost rows | 0 | 0 |
| Verified metadata-before-video returns | 693 | 36,110 |
| Unmatched video returns in trace analysis | 0 | 0 |
| Empty metadata dequeues, all / ongoing | 22 / 0 | 28 / 0 |

Both trials exit zero. All **36,600 accepted frames in four recordings** pass
complete native CRC/order/footer replay. Sustained raw recordings total
82,945,950,762 logical bytes; preflight recordings total 1,382,439,162 bytes.
The sustained accepted-host spans are 600.275244 s left and 600.275148 s right.
Sequence gaps equal ongoing rejects; no extra accepted-sequence gaps are observed.
This does not count losses invisible to these counters or establish losslessness.

The seven sustained truncations have matching raw PTS and payload sizes across
cameras. These are preserved rejects, not repaired frames:

| Left / right sequence | Video bytes per camera | Missing 12,276-byte payload blocks |
| --- | ---: | ---: |
| 5,778 / 5,872 | 2,242,620 | 5 |
| 8,916 / 9,010 | 2,119,860 | 15 |
| 9,278 / 9,372 | 1,874,340 | 35 |
| 12,599 / 12,693 | 2,242,620 | 5 |
| 15,564 / 15,658 | 1,997,100 | 25 |
| 16,771 / 16,865 | 2,242,620 | 5 |
| 17,977 / 18,071 | 2,058,480 | 20 |

Expected video length is 2,304,000 bytes. Both video and metadata error flags are
set; the native rejection reason is `metadata buffer error flag`. Block counts
are arithmetic deficits, not a new USB trace identifying the ultimate source of
loss. Startup rejects and their 28 empty metadata buffers are separate from the
ongoing queue-order reproduction fixed by this source change.

There are 17,998 exact sensor-PTS matches. Nearest-host diagnostic associations
have 2–162 µs separation, median 19 µs. Equal PTS and close host timestamps do not
verify exposure synchronization, calibrated camera identity or physical accuracy.

## Image sampling

An existing private selector linked to the native recording Reader checks each
complete recording before exporting selected frames as PNG. First, middle and
last exact-PTS pairs have left/right indices 2/0, 9,001/8,999 and 17,999/17,997.
All six images were inspected: the board is recognizable, with no prominent
horizontal corruption in these samples; its right edge remains clipped in all
three right views. This is visual sampling, not exhaustive pixel validation,
quantified image quality, new pose coverage or a measured object scan.

## Recording concurrency and performance

Of 72,002 traced recording writes, **58 exceed 33.333333 ms**; the longest is
136.921779 ms. All 58 contain a total of **91 complete successful same-camera
video dequeues on different threads**. Both dequeue start and end must lie inside
the write interval. `long-write-concurrency.json` retains every counted write and
dequeue; no start-only overlap is counted. These observations support continued
camera servicing during these storage waits, not a universal latency bound.

Whole finite CLI, including setup, recording, its internal validation and trace
flush: 652.71 s wall time, 39.95 s user CPU, 55.65 s system CPU, reported 14% CPU,
maximum RSS **357,472 KiB**, zero swaps. External replays and later image extraction
are outside that measurement. There were two whole-process major faults.

A partial acquisition-only sample contains 90 observations spanning 445.064368 s:
35.98 CPU seconds, **8.084224% of one core**, RSS **333,732–338,644 KiB**, zero sampled
process swap and zero major-fault delta. Samples begin after recording starts and
exclude points where either file has reached its final image-payload size; this
excludes teardown/replay. It is not a full-duration peak, a 60-minute memory
qualification, kernel memory measurement, GUI FPS or reconstruction throughput.
The fixed 308.8 MB trace allocation is included in these process measurements.

## Restoration, review and qualification

The administrative harness completed successfully. The original installed module
and its parameters were restored; WirePlumber is active. Saved raw format/frame-
interval structures were restored and compared for all five idle capture nodes.
Native left/right/color stream-state and illumination readbacks match their
initial snapshots, both per trial and after global restoration. Cooperative
session locks and ordinary no-consumer/no-forced-unload checks remain in use.
No unsnapshotted optical-state guarantee or hard cleanup deadline is claimed.

The existing source gate remains **863 passing tests, one existing ignored test**,
formatting, strict all-target Clippy and release builds; exact tested HEAD passed
CI41642834028. This evidence-only milestone does not require rerunning unchanged
Rust tests. Diagnostic builds/self-test, full raw replay, trace/counter analysis,
resource analysis and restoration are new evidence. CodeRabbit quota lookup again
returns `lookup_failed`; no completed new automated review or paid request is
claimed. PR32 remains draft; an empty inline-thread snapshot is not approval.
Independent implementation review remains outstanding.

**Next priorities:** investigate the retained startup framing and paired transport
truncations without weakening validation; obtain full board coverage and independent
physical dimensions; freeze a justified calibration and reconstruct the measured
object in three trials. Finite ten-minute capture does not satisfy 30-minute live
processing, GUI capacity, cleanup deadlines, tracking/fusion or complete native
scan/edit/save/reopen/export qualification. SCAN-REV-005, -006 and -021 remain
separate; this developer evidence does not close independent findings.
