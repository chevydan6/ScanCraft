# Baseline acquisition without optional lid polling

**Result:** pausing the optional session logger does not eliminate ongoing Raptor
rejections. Two 1,800-frame/camera captures save and replay **7,200 real frames**,
but still produce two and four rejected pairs. The first USB diagnostic expires
early; the corrected diagnostic is complete and identifies a missing-EOF pair
plus software-request starvation. Both strict admissions remain **denied**.

This extends the [direct lid-read investigation](RAPTOR_LID_READ_CONTROL.md).
It is a separate declared mitigation test, not a reclassification of that failed
challenge. No application, driver, firmware, affinity or permanent observer change
is made. Physical scale and a first independently measured object remain absent.

## Common method and preserved failure

Both captures use acquisition checkout a4d2de17f77fb3cb0d4baef9af467f00fd779d06,
CLI SHA-256 `be02dd6a39ecf18d451377517a19b3d3a03a1fe57fffd59a7e188ef958fecda0`,
and installed baseline module srcversion `9D1275CDD5E013864E4C564`. Each new plan
binds 16 inputs before execution. The optional logging service is temporarily
stopped and restored; its source is unchanged. No deliberate lid/cached-read probe
runs. Native capture requests 30 FPS, 1920 × 1200 GREY, temporary flood illumination
and 1,800 accepted frames per camera. USB monitoring requests a 64 MiB ring. Host
tracing requests 384 MiB per CPU on 16 CPUs; actual readback is 393,217 KiB per CPU.
Memory/disk admission requires 10 GB available memory and 20 GB free storage.

**Developer diagnostic mistake:** the first copied USB monitor still used its
fixed 60-second wall-clock deadline. It expired before the longer capture finished,
returning exit2 with `diagnostic deadline`, `io_failed: true` and one queued event.
The native CLI completed, but the collector returned exit1. The original strict
header analyzer refuses that input and admission remains denied. Zero reported
USB drops does not mean a complete trace. No missing tail is counted as passing.

The private helper correction replaces the fixed wall-clock cutoff with an explicit
bounded 1–900-second monotonic deadline. The separately declared corrected trial
uses 180 seconds. Strict GCC compilation, 19 actual-helper deadline cases, six
invalid-argument cases that create no output, and two serialized PCAP records pass
offline. The initial indentation-warning compile failure is retained. Helper hash:
`4689b7d5c1afd854970a15a56ab9d764c3abeace39561c9f8985b5a464454673`.
The new helper does not alter camera validation or classify incomplete data as good.

## Native counts and diagnostic coverage

| Observation | Original deadline trial | Corrected monitor trial |
| --- | ---: | ---: |
| Accepted/replayed L/R | 1,800 / 1,800 | 1,800 / 1,800 |
| Startup rejected buffers L/R | 107 / 102 | 84 / 0 |
| Ongoing rejects and sequence gaps L/R | 2 / 2 | 4 / 4 |
| Total rejection proxy | 213/3813 = 5.586153% | 92/3692 = 2.491874% |
| Ongoing-only proxy | 4/3604 = 0.110988% | 8/3608 = 0.221729% |
| Saved USB events | 1,338,010, incomplete | 1,358,194, complete |
| Complete host trace events | 12,389,145 | 12,610,199 |
| Balanced host xHCI identities | 678,382 | 679,097 |
| IRQ affinity readback | CPU15 → CPU15 | CPU10 → CPU10 |
| Actual IRQ39 entries | CPU15: 677,285 | CPU10: 677,489 |

All rejection proxies fail the below-0.1% target. They count rejected V4L2 buffers,
not independently counted physical exposures. Different IRQ CPUs and capture
conditions preclude an isolated rate/performance comparison between trials.

The first trace's separate **partial-prefix** analysis matches 668,971 observed
bulk completions in order, length and ten stable identities to the complete host
trace. USB completions missing from the tail are 4,665 left and 4,669 right.
Of 3,604 native accepted/ongoing-rejected associations, 47 have no sufficient prefix
and two accepted frames have partial observed bytes. Those two mismatches and all
missing associations remain explicit; the original full-capture gate is unchanged.
All four ongoing rejections lie within the retained prefix and match their native
2,242,620-byte lengths. Their two incidents overlap copy-worker waits up to
1,413/1,836 µs. Selected-event absences are 407/688 µs, not proof of CPU stoppage or
firmware causality. No complete USB/header qualification follows from this salvage.

## Corrected trace: strict framing failure and software starvation

The corrected monitor exits0 after SIGINT with zero reported dropped/queued events
and no I/O failure. All 1,358,194 selected USB events and per-CPU host counts balance;
339,512/339,508 camera completions match ordered identity/length mappings. All
3,600 accepted frame headers pass strict association checks. Every one of the
3,608 native lengths matches the USB image-byte sum, but **two rejected frames lack
EOF**, so strict admission still returns exit1. There is no subsequent hardware arm.

Left sequence321/right239 share PTS3572033025 and each contain183 same-FID payloads,
2,242,620 image bytes, no EOF, and an observed following FID toggle. Separate FID
boundary accounting agrees with the native length for all3,608 associations.
That explains a decoder boundary; it does not recover missing image data or accept
malformed frames. The original failure is retained. Byte agreement alone is not
semantic success.

Software-request accounting is valid over the complete host trace and its matched
native byte associations, while the missing-EOF failure remains separate. It does
not require accepting the malformed frames. Time with **zero software-pending
requests inside each rejected completion gap** is:

| Rejected sequence L/R | Zero-pending interval L/R, µs |
| --- | ---: |
| 321 / 239 | 1,435 / 1,419 |
| 324 / 242 | 1,244 / 1,234 |
| 479 / 397 | 2,185 / 2,173 |
| 844 / 762 | 1,109 / 1,096 |

These are software enqueue/giveback observations, not controller-ring occupancy
or physical bus measurements. They identify remaining request-recycling delays;
the cause and a safe effective correction still require evidence.

Full host analysis accounts for678,957 camera-copy worker cycles. At those four
incidents the longest overlapping queue-to-start waits are2,013,1,847,2,764 and
1,703 µs. The respective selected-event absences are975,341,964 and555 µs. These
elapsed intervals include scheduling; they do not establish uninterrupted CPU
execution or prove why the workers waited.

## Restoration, resource measurements and scope

All 14 native state-field comparisons pass in each trial. Saved records support
restoring five idle formats, WirePlumber, global tracing, removing the private
trace instance, and returning the original observer to active/running with its
source unchanged. Baseline module parameters/srcversion remain unchanged.
Corrected-trial kernel logs are byte-identical before/after. The first collector's
exception skipped its final kernel log; a later noninteractive privileged read
failed authentication, so no first-trial kernel-log comparison is claimed.

| Instrumented whole native CLI | Original deadline trial | Corrected monitor trial |
| --- | ---: | ---: |
| Wall time | 72.40 s | 73.27 s |
| User CPU time | 4.30 s | 4.24 s |
| System CPU time | 11.04 s | 12.49 s |
| Peak RSS | 31,736 KiB | 38,380 KiB |

These `time -v` measurements include native setup/acquisition/writing/cleanup;
they exclude the separate monitor, trace buffers and offline analyzers. They are
not isolated acquisition CPU measurements or an uninstrumented throughput claim.
The first trial's final images were extracted and viewed: recognizable board,
right edge clipped, no independently measured object. No calibrated surface,
physical accuracy, tracking/fusion or GUI/export qualification is implied.

## Validation, independent review and next task

The preceding fresh local full gate remains890passed/0failed/1ignored with formatting,
Clippy and release success; implementation is unchanged by these diagnostics.
**Current acquisition-head CI is mixed:** PR41702142669 passed, push41702132020
failed the synthetic extended-capacity test with recording-queue overflow. The
independent reviewer opened **SCAN-REV-031**: preview consumption does not guarantee
recording-consumer progress. One constrained local reproduction on CPU0 with four
test threads passes all four capacity tests in4.34s; that is not a fix or grounds
to discard the CI failure. Existing bounded, nonblocking overflow behavior remains.

Next priority is deterministic recording-progress synchronization for the synthetic
success fixture and a controlled delayed-writer regression, preserving independent
overflow/cancellation tests. Do not increase sleeps/queue sizes or retry until green
as a substitute. Then assess the remaining real request-recycling delays without
weakening framing checks. Physical measurements and reconstruction remain required.

Private evidence: `no-lid-sustained-20261104`, `monitor-deadline-fix-20261104` and
`no-lid-sustained-corrected-20261104` retain plans, bound inputs, original failures,
recordings/replays, full host traces, partial/full USB analyses, strict denials,
restoration, helper tests and CI logs. CodeRabbit quota lookup remains unavailable;
no completed or paid review is claimed. Raw scans and private binaries stay out of
public Git. No protected-main merge or review bypass occurred.
