# Raptor acquisition: timed lid reads and cached-read controls

**Confirmed finite observation:** in one 600-frame/camera capture, all three
ongoing rejected camera pairs coincide with the three longest of eight direct
laptop lid-state reads. None is associated with the eight cached-display reads.
The installed baseline driver is unchanged. This narrows the host-side timing
lead; it does not establish a firmware mechanism, eliminate other failure modes,
or qualify reliable scanning. All 1,200 saved frames replay, but strict diagnostic
admission **fails** on one malformed pair and no further hardware arm followed.

## Method and provenance

Acquisition checkout c907ae55e4f16f1074d813e43ea2e37a809c2a2b uses CLI SHA-256
`be02dd6a39ecf18d451377517a19b3d3a03a1fe57fffd59a7e188ef958fecda0` and baseline
module srcversion `9D1275CDD5E013864E4C564`. Eighteen collector/probe/plan inputs
were hash-bound before acquisition and rechecked afterward. Application code,
driver parameters, firmware, power settings and IRQ affinity were not changed.

The original logging observer is temporarily stopped and restored with unchanged
source. An owned unprivileged Python child performs exactly 16 reads in a
predeclared balanced order: eight `/proc/acpi/button/lid/LID/state` reads and eight
`/sys/class/drm/card1-eDP-1/status` reads. The schedule starts one second after the
probe origin and uses absolute 1.1-second intervals. The plan is retained as its
original pre-execution record; actual execution is established by the saved
ready/read/completion records, not by rewriting the plan's status.

Each operation records monotonic start/open/read/end times, scheduled time, byte
count and errors. Both read types use the same process and logging path. The
verifier checks exact order, paths, count, timestamps and completion. Seven offline
cases pass: valid input succeeds; missing completion, wrong path, empty read,
read error, early start and duplicate index fail. The probe exits successfully.
The collector owns its cleanup and restores the observer in `finally`; this finite
success does not prove a hard deadline for an uninterruptible kernel read.

Capture requests 30 FPS, 1920 × 1200 GREY, temporary flood illumination and 600
accepted frames per camera. The established USB/header and full host-event trace
collectors remain active. All 16 predeclared windows are reported below. The order
is balanced but not randomized. There is only one
capture: this is neither a blinded trial nor a repeated performance benchmark.

## Every read window

Duration includes open, read, close and scheduling. Completion gaps use the host
trace clock and are the largest consecutive endpoint-completion interval that
overlaps each operation. Sequence columns identify native rejected frames whose
PTS is adjacent to that interval; they are not inferred physical exposure counts.
Indices are zero-based. USB-monitor timestamps differ slightly from ftrace;
the table consistently uses ftrace for gaps.

| Index | Read | Elapsed ms | Largest overlapping completion gap L/R, µs | Associated rejected sequence L/R |
| ---: | --- | ---: | ---: | --- |
| 0 | cache | 0.068128 | 159 / 158 | None |
| 1 | lid | 3.088140 | 2876 / 2862 | 147 / 55 |
| 2 | lid | 1.011203 | 776 / 761 | None |
| 3 | cache | 0.055784 | 165 / 139 | None |
| 4 | lid | 1.022574 | 865 / 837 | None |
| 5 | cache | 0.059561 | 157 / 163 | None |
| 6 | cache | 0.054522 | 169 / 164 | None |
| 7 | lid | 0.973582 | 674 / 794 | None |
| 8 | lid | 5.486739 | 5237 / 5209 | 378 / 286 |
| 9 | cache | 0.052518 | 158 / 168 | None |
| 10 | cache | 0.058840 | 158 / 158 | None |
| 11 | lid | 1.557496 | 1302 / 1285 | None |
| 12 | cache | 0.059973 | 146 / 146 | None |
| 13 | lid | 0.975857 | 761 / 738 | None |
| 14 | lid | 3.028107 | 2824 / 2803 | 576 / 484 |
| 15 | cache | 0.069159 | 145 / 145 | None |

All six ongoing rejected frames are associated with reads 1, 8 and 14; there are
no unassociated ongoing rejects. Those three windows overlap 2.775, 5.171 and
2.771 ms, respectively, without any selected host events across the traced CPUs.
This is absence of the selected trace events, not proof that CPUs stopped or that
no traffic occurred on the physical bus. Five other lid reads do not coincide
with rejected frames. Eight cached reads take 0.052518–0.069159 ms.

All 226,937 observed IRQ39 entries occur on CPU15, agreeing with CPU15 before/after
affinity readbacks. Unlike the preceding between-capture comparison, this sample
has no observed IRQ placement change. It still cannot exclude unobserved host
activity, shared-clock effects, tracing overhead, or confounding within a read.
The local ACPI/DRM source distinction and its limitations are recorded in
[RAPTOR_OBSERVER_TIMING.md](RAPTOR_OBSERVER_TIMING.md); resident execution and the
specific firmware/interrupt mechanism have not been attested.

## Native capture and strict failure

Startup rejected buffers are **94 left / 0 right**. Ongoing rejects and sequence
gaps are **3 / 3**. Total rejected-buffer proxy is **100/1300 = 7.692308%**; the
ongoing-only proxy is **6/1206 = 0.497512%**. Both fail the below-0.1% target.
These are V4L2-buffer proxies, not a known number of missing physical exposures.

All 1,200 saved accepted frames replay and have strict matching header associations.
At read 8, left sequence378 and right sequence286 have the same PTS2359187529,
173 same-FID payloads and EOF flags at indices170 and172. The full PTS-group sum
is 2,119,860 image bytes; the native rejected buffer contains 2,099,196. A separate
first-EOF accounting analysis matches that native length exactly and identifies
the remaining two payloads as a **20,664-byte tail**. All 1,206 accepted/ongoing
rejected associations match this accounting model. It explains the observed byte
boundary; it does not reconstruct missing pixels or make anomalous frames valid.

The original header and correlation failures remain intact. `admit.py` returns
exit1 with both strict-header and PTS-group-byte reasons. The first-EOF analysis
does not replace admission, and the recycling analyzer is not run on this failed
input. No subsequent hardware trial has been admitted. The malformed frame pattern
also occurs with the installed baseline; it is not unique to the experimental
inline-copy candidate, nor proof that either driver is generally correct.

## Completeness, restoration and validation

The USB capture contains 454,998 saved events, balanced identities, no unmatched
requests and zero reported collector loss. Host tracing contains 4,169,718 parsed
events matching saved per-CPU counts, 227,499 balanced xHCI request identities
and ten stable bulk-identity mappings. The camera endpoints have 113,712/113,710
completions. Startup retains 188 unrecognized successful prefixes on the left;
those are not qualified by accepted-frame validation.

All 14 native state-field comparisons pass. Saved records support restoration of
five idle formats, WirePlumber, global tracing and removal of the private trace
instance. The original observer is active/running again with unchanged source.
The installed baseline module/parameters remain unchanged. Before/after kernel
logs are byte-identical. These are finite restoration observations, not universal
cleanup, disconnection, suspend or concurrency guarantees.

Fresh `tools/check.sh`: **890 passed, 0 failed, 1 ignored**, formatting, strict
Clippy and optimized release build pass. Application/kernel implementation is
unchanged. Acquisition head c907ae5 passed push41699629965 and PR41699643105 CI.
New report CI and independent review are separate. CodeRabbit quota lookup was
previously unavailable; a skipped review is not validation.

Private `lid-read-control-20261104` retains bound inputs, full recordings/traces,
probe plan and actual logs, seven verifier cases, native replay/restoration,
strict denial, read-window analysis, separate framing accounting, host/IRQ
analysis, developer verification and Rust gate output. No raw scan, private
observer source, binary or factory data is included in the public repository.

## Interpretation and next priority

**Engineering inference:** the specific direct lid read is a stronger intervention
lead than the earlier whole-observer comparison. Same-process cached controls,
complete timing correlation and unchanged IRQ placement narrow possible causes.
One short capture still does not establish a universal causal mechanism or a
production fix. The earlier independent worker-delay incident and startup failures
remain separate unresolved problems.

Next, preserve the failed admission and review the first-EOF evidence, then design
a separately declared mitigation test that avoids diagnostic lid polling during
capture and retains all native rejection checks. Do not silently disable host
monitoring permanently, relax malformed-frame checks, or describe the current
result as passing. Physical scale, an independently measured target, first genuine
object reconstruction and three-trial GUI/export qualification remain outstanding.
