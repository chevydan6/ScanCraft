# Bounded completion-copy hardware trial

**Confirmed:** the opt-in candidate executes without camera-copy worker dispatch
in the enabled sample, but does not solve acquisition loss. In a same-binary
600-frame/camera disabled → enabled → disabled comparison, ongoing rejects per
camera are **2 → 9 → 3**. All arms fail the total rejected-buffer target. Keep the
option experimental and disabled; no persistent driver installation is justified.

## Reproduction and scope

Source checkout `08adf1be077be172538131d3accf42f317bb7421` contains the
[GPL experiment](../experiments/kernel-inline-copy/README.md). The unchanged native
CLI is SHA-256 `be02dd6a39ecf18d451377517a19b3d3a03a1fe57fffd59a7e188ef958fecda0`.
All five trials temporarily use the same candidate SHA-256
`bcce45bd2752db746373be96e216d866f3393ad74a4a9fccc01462f1e076bbbf`, srcversion
`12034367675945449357638`, with `scancraft_inline_copy=0` or `1`, five URBs per
video endpoint and otherwise unchanged parameters. Each arm restores the installed
baseline `579c7633261b27cfd76af16f4b1ad57c1306ce6c7d75ca914e15693e3ec127e5`,
srcversion `9D1275CDD5E013864E4C564`. There are no firmware or affinity writes.

The collector retains ordinary scanner locks, refuses active media/device
consumers, saves all five idle camera formats, and temporarily stops idle
WirePlumber. Capture uses genuine 1920 × 1200 GREY images at requested 30 FPS with
temporary flood illumination and complete native replay. USB monitoring explicitly
uses a 64 MiB ring and 32 payload-prefix bytes per transfer. The private monotonic
ftrace instance retains scanner xHCI/IRQ, filtered camera-copy workqueue events,
all scheduler switches/wakeups and local timers, with 128 MiB per CPU on 16 CPUs.
Effective controller IRQ affinity is CPU8 in every trial. Instrumentation is
substantial; this is not an isolated performance benchmark.

## Short capture and cancellation checks

Disabled and enabled trials each save 30 frames per camera. All 120 frames replay
and match actual PTS/FID/EOF/length associations. USB event counts are 24,252 and
24,246; complete trace counts are 215,782 and 107,631. No reported collector loss.
Ongoing rejects are zero, but startup rejects L/R are 106/0 and 0/100. These short
samples do not qualify reliability.

Each candidate mode then runs the existing `tools/check_stereo_cancel.py`:
SIGTERM and SIGINT are sent to owned native probes after observing their active
format/rate. Both modes report cooperative cancellation with exit 1, restore
reported formats/rates, and reopen for ten valid frames per camera. Signal-to-exit
measurements are 0.214175/0.215575 seconds disabled and 0.215469/0.215298 seconds
enabled. These are four finite observations, not hard cleanup deadlines,
disconnection tests, suspend tests or proofs of reference/race safety.

## Longer comparison

All three arms save and replay 1,200 frames each. Startup and ongoing rejects are
retained separately; the total proxy includes both. A rejected V4L2 buffer is not
necessarily one physical exposure.

| Arm | Option | Startup rejects L/R | Ongoing rejects L/R | Total proxy | Ongoing-only proxy |
| --- | --- | ---: | ---: | ---: | ---: |
| A1 | Disabled | 0 / 86 | 2 / 2 | 90/1290 = 6.976744% | 4/1204 = 0.332226% |
| B1 | Enabled | 1 / 1 | 9 / 9 | 20/1220 = 1.639344% | 18/1218 = 1.477833% |
| A2 | Disabled | 1 / 0 | 3 / 3 | 7/1207 = 0.579950% | 6/1206 = 0.497512% |

All fail the below-0.1% target. This finite comparison does not establish a general
slowdown or improvement; host conditions and tracing cost are not isolated.

| Observation | A1 | B1 | A2 |
| --- | ---: | ---: | ---: |
| Complete USB events | 454,290 | 458,248 | 454,210 |
| Complete ftrace events | 3,797,648 | 1,984,825 | 3,917,821 |
| Camera-copy worker cycles | 226,995 | 0 | 227,015 |
| IRQ39 entry/exit pairs | 226,539 | 228,157 | 226,565 |
| IRQ elapsed median / p99 | 3 / 5 µs | 3 / 5 µs | 3 / 5 µs |
| Maximum observed IRQ elapsed | 1,135 µs | 81 µs | 1,811 µs |

IRQ intervals include interruptions and are not isolated memcpy CPU times or
latency bounds. The absence of filtered worker events supports execution of the
intended path in B1, not complete kernel concurrency qualification. USB identities
balance, per-CPU trace entry counts agree, and reported drop/overrun counters are
zero in every arm. All accepted frame associations match; rejected-frame exceptions
below remain explicit.

Whole-CLI `/usr/bin/time -v` measurements for A1/B1/A2 are respectively
24.64/25.04/25.27 seconds wall time, 1.39/1.36/1.37 seconds user time,
3.67/3.37/3.70 seconds system time, and 31,720/35,060/35,608 KiB peak RSS.
These exclude whole-system collector memory and independently executing kernel
work; they do not demonstrate whole-application CPU or memory improvement.

## Framing exceptions and byte accounting

A1's 1,204 and A2's 1,206 accepted/ongoing-rejected associations satisfy the existing
single-final-EOF and byte-length checks. B1 has 1,218 associations, including
**three strict header-semantic exceptions and two PTS-group length mismatches**:

- Both cameras' sequence 147 has 183 payloads with one FID and EOF at zero-based
  indices 180 and 182. Summing every same-PTS payload gives 2,242,620 image bytes;
  native rejected buffers contain 2,221,956 bytes. The prefix through the first
  EOF is exactly 2,221,956; the two following same-FID payloads total **20,664**.
- Left sequence 449 has no observed EOF. Its 2,242,620 bytes match the native
  rejected buffer; the next observed header toggles FID.

**Engineering inference supported by source:** unchanged Linux
`uvc_video_decode_end` closes at EOF; `uvc_video_decode_start` skips same-FID data
while the next buffer is inactive and also closes an active frame on a FID toggle.
A separate first-EOF/subsequent-FID accounting pass matches native byte counts for
all 1,218 B1 associations while retaining the three semantic exceptions. This
explains the finite B1 discrepancy without relaxing validation or fabricating
missing pixels. It is consistent with the earlier 20,664-byte exception but does
not retrospectively prove that earlier incident's header sequence. The source
reference is the exact private candidate/baseline `uvc_video.c`; the public patch
leaves decoding unchanged. Header prefixes cannot reconstruct complete images or
establish where the anomalous headers originated.

The original strict analyzers remain unchanged. Their JSON reports mismatches but
their exit codes were zero; the orchestration therefore started A2 before those
fields were inspected. The later recycling analyzer correctly refuses B1 with an
assertion, and **no B1 recycling result is claimed**. This diagnostic orchestration
oversight is retained in the evidence. Future trial admission must explicitly
check semantic result fields, not just process exit codes. No further hardware
trial followed this discovery; no native validation rule or counter was weakened.

## Remaining host gaps

B1 has no camera-copy worker cycles, yet paired truncations remain. Eight of its
nine left rejected-frame within-PTS gaps overlap 1.616–11.338 ms without selected
host events across the traced CPUs. Sequence 329 instead has a small within-group
gap, with USB-monitor gaps of 7.768/7.771 ms before the left/right groups; a
within-frame-only maximum misses such boundary intervals. These observations do
not prove CPU stoppage, an SMI, a scanner fault, physical bus inactivity, or that
the candidate caused the timing pattern. Independent timing and observer-cost
checks remain necessary. Do not extend this experiment into a production fix.

## Restoration, validation and evidence

Across five trials, 119 native state-field comparisons match. The installed
module/parameters, five saved camera formats per trial, WirePlumber and global
tracing state restore; private trace instances are removed. Kernel-log deltas
contain ordinary UVC unload/load/enumeration and the existing nodrop deprecation
message, with no observed crash/refcount/lockdep report. Absence of a warning in
these short runs does not establish race freedom or disconnect safety.

The enabled long trial's final image pair was extracted and viewed: recognizable
calibration board, clipped at the right image edge, without a separately measured
object. This is acquisition evidence, not calibrated 3D reconstruction. Physical
scale, diverse calibration poses, a measurable object and three genuine repeat
scans remain outstanding.

Fresh `tools/check.sh`: **890 passed, zero failed, one existing ignored**,
formatting, strict Clippy and release build pass. Application/kernel source is
unchanged from `08adf1b`; this publication changes evidence documentation only.
Independent assignments 461/464 verified the earlier offline proposal and both
08adf1b CI events, not these runtime results. PR33 remains draft and unmerged.
CodeRabbit quota/billing lookup again returns unavailable/lookup_failed; no new
completed or paid review is claimed. Independent review of this evidence is pending.

Private `inline-smoke-20261104` and `inline-aba-20261104` retain plans/input hashes,
collectors, all original recordings, replay, header and complete host traces,
state/service/module restoration, kernel logs, resource measurements, extracted
images, analyzers, strict failures, separate framing accounting and file manifests.
Reproduce analysis with `analyze_headers.py`, `correlate.py`, `verify_trace.py`,
`analyze_host.py`, `analyze_irq.py`; `analyze_recycling.py` is valid for the strict
matching arms and intentionally refuses B1. `analyze_framing.py B1` explains
finite boundary accounting while keeping original failure results. Raw/private
data and module binaries are not published.
