# Raptor host-service trace

**Confirmed:** one baseline-driver capture distinguishes two host-event patterns
around four paired short frames. Three incidents overlap 3.869–5.318 ms without
any selected events across the traced CPUs. A fourth includes **923/895 µs with
zero software-pending video requests**, while camera-copy work waits to execute.
This localizes an observed scheduling/queue problem; it does not establish a
firmware fault or explain every truncation.

## Capture and validation

Checkout `4102fd4ad9a15116d4d9e445b722657547c525c3` uses unchanged acquisition
code and CLI SHA-256
`be02dd6a39ecf18d451377517a19b3d3a03a1fe57fffd59a7e188ef958fecda0`.
The installed baseline module remains loaded throughout: srcversion
`9D1275CDD5E013864E4C564`, five requests per video endpoint, unchanged parameters.
There is no module replacement, firmware write, affinity write or production edit.

The established private collector records 600 accepted frames per camera at
1920 × 1200 GREY/requested 30 FPS with temporary flood illumination. It retains
the explicit 64 MiB USB monitor ring and adds these events to its private ftrace
instance:

- Existing scanner request enqueue/giveback and controller IRQ entry/exit.
- Camera-copy queue, activation, execution start and execution end, filtered by
  the unambiguous resident `uvc_video_copy_data_work` function address.
- All scheduler switch/wakeup and local timer entry/exit events.

Ftrace uses a monotonic clock and requests 128 MiB per CPU on 16 CPUs, a 2 GiB
diagnostic allocation. Effective controller IRQ affinity stays on CPU8. The
609,311,391-byte text trace contains **4,014,236 events**, with exact per-CPU
entry-count agreement and zero reported losses. USB monitoring saves 455,590
selected-device events with zero queued/drop/I/O counters; identities balance.
All 1,208 accepted/ongoing-rejected frame associations match actual PTS, FID,
EOF and image length. Only 32 payload bytes per USB transfer were retained;
this is not independent full-image reconstruction from USB.

Native results: 600/600 accepted, 94/0 startup rejects, 4/4 ongoing rejects and
4/4 sequence gaps. All 1,200 stored frames pass complete native replay. Total
rejected-buffer proxy is **102 / 1302 = 7.834101%**, failing the <0.1% target.
The left stream also has 188 unrecognized successful transfer prefixes; these
remain in the evidence rather than being declared valid startup frames.

All 14 post-capture/final native state-group comparisons match. Five idle formats,
WirePlumber service state and global tracing configuration are restored; the
private trace instance is removed. The final frame pair was exported and viewed:
calibration board, right edge clipped in the right camera, no measured object.
No physical scale, dimensional accuracy or end-to-end object scan is established.

## Host-event results

The analyzer balances **227,666 complete camera-copy work cycles** and **90,486
local timer entry/exit intervals**. All 1,334,445 recorded scheduler switches are
consistent with the preceding next-task identity on their respective CPUs.

| Interval | Median | Empirical p99 order statistic | Maximum |
| --- | ---: | ---: | ---: |
| Camera work queued → execution starts | 4 µs | 26 µs | 3,950 µs |
| Camera callback starts → ends | 3 µs | 12 µs | 752 µs |

These instrumented intervals include interruptions and scheduling effects; they
are not measurements of uninterrupted memcpy execution or uninstrumented speed.
The largest local-timer entry/exit interval in this sample is 61 µs.

| Rejected sequences L/R | Missing image blocks per camera | Largest completion gap L/R | Longest interval without selected events across all CPUs |
| --- | ---: | ---: | ---: |
| 134 / 42 | 20 | 3,956 / 3,934 µs | 3,869 µs |
| 227 / 135 | 5 | 1,010 / 993 µs | 225 µs |
| 315 / 223 | 30 | 5,360 / 5,345 µs | 5,318 µs |
| 496 / 404 | 20 | 3,954 / 4,061 µs | 3,925 µs |

The three longer gaps end near local-timer activity on multiple CPUs. Their
absence of selected events is compatible with a wider host-service interruption,
but it is not proof that no CPU executed instructions. The trace lacks independent
physical bus timing and cannot distinguish platform pauses, clock effects, idle
behavior, interrupt masking or other unrecorded activity. Do not label them SMI,
BIOS or scanner firmware failures from this trace alone.

The shorter incident has a different observable pattern. Ten camera-copy jobs
queue and activate, then wait 909–1,506 µs before callback execution while other
host events continue. One worker wakes with target CPU1 and first switches in on
CPU0 1,500 µs later. Each endpoint's five requests are eventually all returned;
software-pending counts stay at zero for 923 µs on the left and 895 µs on the
right within the respective largest frame gap. Copy execution and resubmission
then replenish the queues. This is observed software request starvation alongside
the paired truncation, not merely a count of requests awaiting callbacks.
The reason for the scheduling delay and its sensitivity to instrumentation remain
unresolved.

The inspected local Linux reference explains the relevant boundary:
`drivers/media/usb/uvc/uvc_video.c::{uvc_video_complete,uvc_video_copy_data_work}`
defers copy operations to a workqueue and resubmits after copying/releasing buffer
references. `uvc_driver.c::uvc_stream_new` allocates an unbound high-priority
workqueue. This reference is not a retrospective resident-kernel attestation.
The recorded queue/execution events independently establish the observed path.

## Limits and next action

Additional tracing generated 1,334,445 switch and 678,191 wakeup events. Its cost
has not been isolated against an uninstrumented control. No performance advantage,
rejection-rate improvement or universal root cause is claimed. The earlier depth-32
counterexample, startup framing and 20,664-byte exception remain separate.

Private `host-service-trace-20261104` retains the collector, runtime event formats
and callback filter, raw recordings, replay, USB/trace counters, source bindings,
complete trace, state restoration, final images and analyzers. `analyze_host.py`
accounts for work lifecycles (including requeue before a prior execution ends),
per-CPU scheduler identities, timer pairs and each rejected gap. Its lifecycle
self-test is separate from hardware evidence. `analyze_recycling.py` measures
software pending intervals; existing header/correlation analyzers bind the
camera observations. Raw data remain private.

**Recommendation:** evaluate a narrowly bounded experiment at the copy/resubmit
boundary against these observed scheduling delays, preserving buffer references,
cancellation, validation and IRQ-latency accounting. Do not install a fix based
only on this trace or assume it solves the longer host-wide event gaps. Continue
investigating those gaps with an independent timing check. Physical calibration
scale and measured-object reconstruction remain the main qualification needs.

Public changes are evidence documentation only. Existing 890 tests/one ignored,
formatting, strict Clippy and release build apply to unchanged application code.
Both `4102fd4` CI runs passed, independently corroborated by reviewer assignment457;
assignment454 accepted the preceding finite queue comparison within its limits.
This new host trace awaits independent review and is not a completed CodeRabbit
review.
