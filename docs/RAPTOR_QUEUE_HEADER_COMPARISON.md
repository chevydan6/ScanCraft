# Raptor queue-depth comparison with complete USB headers

**Confirmed:** a controlled 5 → 32 → 5 request-depth experiment saved and replayed
3,600 genuine camera frames. All three trials retained complete USB/header and
IRQ/request traces. **Depth 32 still produced a paired short frame**, so the
earlier favorable finite comparison does not establish a complete fix. Every
trial fails the total rejected-buffer target of less than 0.1%.

## Source and procedure

The application checkout is `a55eedb1e8ea340ea6d81aa79a6279418eda2102`;
production source is unchanged from `2ce5903`. The acquisition CLI SHA-256 is
`be02dd6a39ecf18d451377517a19b3d3a03a1fe57fffd59a7e188ef958fecda0`.
It uses the existing `stereo-flood-capture` path, 600 accepted frames per camera,
1920 × 1200 GREY, requested 30 FPS, temporary flood illumination, unchanged
validation, and complete native recording replay.

All arms use the same previously verified experimental kernel module; only its
`raptor_urbs` load parameter changes. The [original queue experiment](RAPTOR_QUEUE_DEPTH_TRIAL.md)
documents its GPL-2.0-only source, reproducible build, historical/public parameter
name mapping, and policy tests. Candidate SHA-256 remains
`46248a3c5e050db16182fdf0f034cd844f4e3dc503c037ffc8656b398c202700`,
srcversion `10A4B6BC7346B07ADC2D755`. No new kernel patch was made.

Each trial independently snapshots all five idle camera formats and seven native
state groups, acquires the ordinary session locks, checks for active media and
device consumers, temporarily stops idle WirePlumber, and performs ordinary
module removal/loading. It verifies the candidate hash, srcversion and every
parameter, restores saved formats before acquisition, and checks all seven state
groups again. No forced unload, persistent install, IRQ-affinity write or firmware
change occurs. Cleanup restores the installed module after **each** arm, then
the service, saved formats and native state. Cooperative locks do not exclude
arbitrary external clients.

The [complete-header collector](RAPTOR_IRQ_COMPLETION_TRACE.md#follow-up-explicit-monitor-ring-and-complete-header-correlation)
uses a read-back-verified 64 MiB usbmon ring and 32-byte payload prefixes, plus
64 MiB requested ftrace capacity per CPU. Native outputs and traces are saved
before validity assertions and during cleanup. The same collector is used in
all arms. Effective IRQ affinity was CPU12 for A1 and CPU8 for B1/A2; it remained
unchanged within each arm. This is a finite, instrumented comparison, not an
otherwise isolated performance benchmark.

## Results

Counts are V4L2 buffers, not independently verified physical exposures. Startup
rejects remain in the complete rejection proxy.

| Trial | Queue depth | Accepted L/R | Startup rejects L/R | Ongoing rejects L/R | Sequence gaps L/R | All-rejection proxy |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A1 | 5 | 600 / 600 | 0 / 97 | 2 / 2 | 2 / 2 | 101 / 1301 = 7.763259% |
| B1 | 32 | 600 / 600 | 1 / 1 | 1 / 1 | 1 / 1 | 4 / 1204 = 0.332226% |
| A2 | 5 | 600 / 600 | 94 / 0 | 2 / 2 | 2 / 2 | 98 / 1298 = 7.550077% |

Accepted host spans are approximately 20.036, 20.002 and 20.036 seconds per
camera. All six recordings pass complete native CRC/footer replay. All 63
before/after/final native state-group comparisons match. Five formats per trial,
WirePlumber activity, global trace configuration and private trace-instance removal
are checked. Installed module SHA-256 remains
`579c7633261b27cfd76af16f4b1ad57c1306ce6c7d75ca914e15693e3ec127e5`,
loaded srcversion `9D1275CDD5E013864E4C564`, with baseline parameters.
These observations do not establish a hard cleanup deadline.

| Trial | Selected USB events | Matched bulk completions | Total kernel trace events | Accepted + ongoing-rejected frame associations |
| --- | ---: | ---: | ---: | ---: |
| A1 | 454,250 | 227,048 | 907,370 | 1,204 |
| B1 | 452,986 | 226,416 | 904,346 | 1,202 |
| A2 | 454,248 | 227,047 | 907,264 | 1,204 |

All monitor queued/drop/I/O counters and ftrace loss counters are zero. All
request identities balance; per-CPU parsed event totals equal retained entries.
Bulk completions match across USB and kernel traces by order, length and stable
identity mapping. Every associated frame matches actual PTS, one FID, a final
EOF and its native image-byte count. This does not independently reconstruct
full images from USB: only the first 32 payload bytes per transfer are retained.

## Short frames and request recycling

Each incident below is paired by actual sensor PTS. All missing-block counts use
the observed 12,276-byte image payload in an ordinary full transfer.

| Trial, rejected sequences L/R | Missing image blocks L/R | Largest internal completion gap L/R, µs |
| --- | ---: | ---: |
| A1, 154 / 249 | 20 / 20 | 3,941 / 3,902 |
| A1, 214 / 309 | 15 / 15 | 3,532 / 3,516 |
| B1, 105 / 105 | 5 / 5 | 6,081 / 6,059 |
| A2, 416 / 324 | 10 / 10 | 2,386 / 2,363 |
| A2, 537 / 445 | 25 / 25 | 4,902 / 4,879 |

B1's largest gap ends with 32 software-pending requests on the left and 31 on
the right. The request returned at the left gap's start is resubmitted after
8 µs; the corresponding right request waits 6,121 µs, while 31 remain pending
throughout that gap. The maximum accepted-frame internal gaps are 1,084, 2,518
and 835 µs in A1/B1/A2 respectively. No universal timing classifier follows.

**Engineering inference:** the larger queue may provide buffering against some
host-service delays, consistent with earlier finite results, but this sample
directly disproves an assertion that depth 32 eliminates ongoing truncation.
It does not identify why the 6 ms interval occurred or quantify an improvement.
Software-pending requests can already have completed in hardware while awaiting
callbacks. Their count does not establish controller ring availability, physical
bus timing, interrupt causation or which device/controller component lost bytes.

## Startup and physical limitations

A1 right and A2 left each have 188 successful USB completion prefixes outside
the normal 12-byte UVC header pattern. Every such prefix precedes that camera's
first accepted frame; none occurs at or after it. B1 has none. The native
startup rejection counts remain 97 and 94: prefix counts and rejected-buffer
counts are different units. Startup samples in the native report are capped,
so this is not a complete per-rejected-buffer startup reconstruction or a
framing fix. All unrecognized prefixes remain in private evidence.

The final A2 frame pair was exported and visually inspected: the calibration
board remains visible with its right edge clipped in the right camera. No
measured printed object is present. No new physical scale, calibration
qualification, genuine object reconstruction or GUI end-to-end scan is claimed.
The earlier 20,664-byte exception remains unresolved.

## Reproduction and next work

Private `queue-header-aba-20261104` retains the plan, per-arm module transitions,
collector/input bindings, complete recordings, native replays, monitor/trace
statistics, actual traces, state restoration, images and failure accounting.
`analyze_headers.py`, `verify_trace.py` and `correlate.py` reproduce transport
association; `startup_context.py` locates every unrecognized prefix relative to
the first accepted PTS, and `analyze_recycling.py` accounts for software request
lifecycles. Its synthetic lifecycle self-tests are separate from hardware data.
The copied trace verifier was adapted to write this trial's newly derived result
instead of expecting the previous trial's result file; its identity and event
count checks remain intact, with the analyzer revision bound separately.

Public changes are documentation only. Existing 890 tests/one ignored,
formatting, strict Clippy and release build remain applicable to unchanged code;
both `a55eedb` CI workflows passed. CodeRabbit availability is recorded separately
and is not substituted by these checks. Independent reviewer assignment450
accepted the preceding complete-header evidence within its stated limits; this
new comparison awaits independent review.

Next: localize long host-service intervals and startup framing before proposing
a production driver fix. Do not simply increase queue limits or suppress rejects.
Advance physical reconstruction when measured board scale and an independently
measured object are available; transport experiments do not replace that goal.
