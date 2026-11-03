# Raptor interrupt and USB completion timing

## Result and scope

On source `2ce59038cfd9276cc0026105107e57dc724f9000`, a complete filtered kernel
trace correlates six paired ongoing short-frame incidents with USB completion
gaps of **1,423–4,452 microseconds**. Four or five requests remain pending
immediately before the next completion in these intervals. One interval overlaps
an interrupt-handler entry/exit span of 1,678 microseconds.

These are software timing observations, not proof of an interrupt-delivery,
host-controller, device-firmware or scheduling root cause. No production source,
module, firmware, interrupt affinity or frame-acceptance rule changed. None of
these short trials passes the acquisition rejection target or reconstructs an
object. Prior queue-depth evidence remains in [the controlled comparison](RAPTOR_QUEUE_DEPTH_TRIAL.md).

## Captures and instrumentation failures

All trials use the installed baseline driver, native `stereo-flood-capture 600
NEW_DIRECTORY --fps 30`, two 1920 × 1200 GREY cameras and temporary flood
illumination. All **3,600 saved frames in six recordings** pass native replay.
Startup and ongoing counts remain separate; sequence gaps equal ongoing rejects.

| Trial | Startup rejects L/R | Ongoing rejects L/R | Accepted host span L/R, seconds | Instrumentation result |
| --- | ---: | ---: | ---: | --- |
| Initial IRQ trace | 1 / 1 | 8 / 8 | 20.236189 / 20.236184 | Complete IRQ trace; incorrect USB event filter |
| Corrected IRQ/USB trace | 0 / 97 | 6 / 6 | 20.168783 / 20.169909 | Complete filtered trace, no reported trace loss |
| USB-header follow-up | 0 / 1 | 5 / 5 | 20.135389 / 20.135393 | Native capture succeeds; USB monitor loses packets |

The first collector incorrectly filtered `xhci_urb_*` by PCI controller name.
The event's `devname` identifies the USB device. It therefore retains 228,013
paired IRQ entry/exit events but **zero USB events**. This limitation is preserved;
the corrected run uses the observed USB device name. The local kernel source
`drivers/usb/host/xhci-trace.h`, `xhci_log_urb`, and the running event format were
inspected. The corresponding [upstream event declaration](https://github.com/torvalds/linux/blob/master/drivers/usb/host/xhci-trace.h)
is a reference, not an attestation of the resident kernel revision.

The corrected trace contains **228,236 enqueue/giveback pairs** and **227,370
IRQ entry/exit pairs**, plus two markers. All request identities balance, with
no unmatched or pending requests at the end. The maximum observed pending bulk
request count is five per video endpoint. All per-CPU statistics report zero
overrun, commit overrun and dropped events. The formatted trace is 123,090,075
bytes. It records IRQ39 and the scanner's enqueue/giveback events in an isolated
ftrace instance using the monotonic clock. Requested ring capacity is 312 MiB
across 16 CPUs; this is not measured process RSS or total application memory.
[Kernel event filtering documentation](https://cdn.kernel.org/doc/html/latest/trace/events.html)
describes the mechanism; observer overhead remains uncontrolled.

The final follow-up adds 96-byte usbmon snapshots to retain transfer status and
UVC header fields absent from formatted ftrace. It captures 452,477 packets but
reports **2,497 kernel drops**. The orchestrator rejects the monitor result and
performs cleanup; it exits1 although native acquisition exits0. The native report
is recovered verbatim from the retained command event, and both recordings are
replayed offline. No nonzero non-cancellation bulk status appears in the retained
subset, but missing packets prevent a complete-status claim. This attempt does
not explain the earlier exceptional frame. The early monitor assertion also
precedes the trace-file dump, so its temporary ftrace buffer was not retained.
Future collectors must save diagnostics on failure before deleting their instance.

## Correlation method and unresolved byte discrepancy

Native reports supply accepted-frame timestamps/sequence/PTS and independently
bounded ongoing rejection samples. USB descriptors associate video endpoints
with the two IR streams. The final analyzer groups positive-length givebacks
through each short transfer and associates the native timestamp with that group's
interval. All 1,212 accepted/ongoing-rejected timestamps map uniquely; offsets
from group start range2–121 microseconds.

Subtracting the previously observed 12-byte UVC header from each transfer matches
**all1,200 accepted frame sizes and11 of12 rejected frame sizes**. This is an
explicit diagnostic inference: the formatted trace contains neither payload
bytes/UVC flags nor transfer status. It does not independently decode images.
The rejected right frame at sequence451 has2,221,956 native image bytes, whereas
the associated183 transfers imply2,242,620 image bytes under that subtraction.
The **20,664-byte discrepancy remains unexplained**. It is not repaired, silently
accepted, assigned to the device, or removed from rejection accounting.

An initial analyzer used the nearest preceding giveback as the frame boundary.
Delayed native timestamp assignment can place that boundary several transfers
into the group; it produced225 size mismatches. That failed analysis and its
outputs are retained. The revised method uses independently visible short-transfer
boundaries and reports the remaining discrepancy instead of asserting a false
complete match. Earlier development errors (a100us timing assumption and a tuple
comparison tie) are also retained separately.

| Left/right rejected sequence | Left/right native bytes | Largest within-group completion gap L/R, us |
| --- | ---: | ---: |
| 54 / 150 | 2,242,620 / 2,242,620 | 1,835 / 1,816 |
| 235 / 331 | 2,242,620 / 2,242,620 | 1,720 / 1,689 |
| 295 / 391 | 2,058,480 / 2,058,480 | 4,334 / 4,452 |
| 355 / 451 | 2,242,620 / 2,221,956 | 2,526 / 2,502 |
| 450 / 546 | 2,242,620 / 2,242,620 | 1,454 / 1,423 |
| 476 / 572 | 2,242,620 / 2,242,620 | 1,736 / 1,707 |

Accepted groups have a maximum internal completion gap of1,419us and an empirical
99th-percentile order statistic of388us. The observed separation is specific to
this instrumented sample and is not a validated rejection predictor. The
1,678us IRQ interval overlaps the second incident; entry/exit timing cannot
separate handler work from interruptions or stalls inside that interval.

## Restoration, provenance and next work

Every trial restores saved camera formats/rates, checked control/illumination/
version/mirror values, the idle WirePlumber service and its own trace instance.
The first two trials retain seven-group comparisons immediately after capture
and at final cleanup; the failed-monitor trial retains the final comparison.
All35 group comparisons match. Five idle formats are restored per trial; global
tracing configuration matches before/after. Installed module hash remains
`579c7633261b27cfd76af16f4b1ad57c1306ce6c7d75ca914e15693e3ec127e5`,
loaded srcversion `9D1275CDD5E013864E4C564`, with baseline parameters retained.
These are checked software states, not hard cleanup deadlines or firmware attestation.

The corrected whole CLI takes25.18s wall,1.36s user and3.27s system CPU, with
31,500KiB peakRSS. Kernel tracing memory and monitor costs are outside that RSS;
no performance improvement is claimed. Its two final exported images were
viewed: the board is recognizable and remains clipped at the right camera edge.
No independently measured printed object is present.

Private evidence sets `irq-completion-20261104`,
`irq-completion-corrected-20261104` and `irq-usb-headers-20261104` retain commands,
plans, state snapshots, native recordings/replays, resource measurements,
collector failures, trace formats/statistics, analyzers and results. SHA-256
inventories bind the artifacts. Raw images, device identifiers and traces remain
local. The analysis and collection scripts are original diagnostic work; no
kernel code or proprietary algorithm was imported into the application.

**Next investigation:** retain complete transfer-status/header evidence together
with controller timing, including on collector failure. Determine whether an
additional driver-side discard explains the exceptional byte discrepancy, then
compare controller gaps with other interrupt activity before changing scheduling
or driver policy. Do not infer a firmware fix from these correlations. Physical
scale, calibration coverage and the first measured reconstruction remain open.

The unchanged source passes the890-test local gate and both exact-head CI runs
41675798228/41675808266. This does not retrospectively resolve the earlier archive
test delay. No new CodeRabbit or paid review was run for this evidence-only change;
the latest quota lookup remains unavailable. Independent reviewer records remain
read-only and their existing qualification scopes are unchanged.

## Follow-up: explicit monitor ring and complete header correlation

The earlier96-byte tcpdump capture did not have the presumed16MiB effective USB
ring. A read-only startup probe of installed tcpdump4.99.6/libpcap1.10.7 with
`-s96 -B16384` observes an **8,192-byte `/dev/usbmon4` mapping**. The inspected
[libpcap1.10.6 USB backend reference](https://github.com/the-tcpdump-group/libpcap/blob/libpcap-1.10.6/pcap-usb-linux.c)
calculates its ring from snapshot length and clamps it to8KiB minimum; that
reference is not asserted to be the installed package's exact source. Runtime
mapping evidence confirms the small allocation on this machine. Increasing a
command-line buffer value alone is therefore not evidence of a larger USB ring.

An original private diagnostic reader now uses the documented
[usbmon binary ABI](https://www.kernel.org/doc/html/latest/usb/usbmon.html): it opens
only the selected bus monitor, sets a64MiB ring, requires an exact size readback,
and drops privileges before collecting. It retrieves the64-byte event header and
at most32 payload bytes with GETX, writes96-byte maximum pcap records while
preserving original lengths, and drains on termination. Final queued/dropped
counters and I/O errors remain mandatory checks. It sends no scanner commands.
A strict C11 warnings-as-errors build, two-record framing check and existing-file
refusal check pass. This is a private measurement utility, not a replacement
scanner backend or new application dependency.

The collector also retains native output and dumps tracing before evaluating
monitor failures, with a finally-path dump before instance removal. Its first
new trial demonstrates failure retention: USB monitoring loses zero events, but
ftrace loses603,982 events on CPU12. The previous allocation incorrectly assumed
that the controller would still use CPUs14/15; before/after affinity is12. That
incomplete trace remains retained and the orchestrator exits1. No affinity is
changed. The corrected trial allocates64MiB on each of16 CPUs (1GiB requested
kernel ring capacity), and reports zero loss in **both** collectors.

| New trial | Startup rejects L/R | Ongoing rejects L/R | Accepted frames L/R | Monitor result |
| --- | ---: | ---: | ---: | --- |
| Explicit USB ring, old ftrace allocation | 0 / 1 | 1 / 1 | 600 / 600 | USB complete; ftrace incomplete |
| Explicit USB ring, all-CPU ftrace allocation | 0 / 1 | 3 / 3 | 600 / 600 | Both complete |

All2,400 newly saved frames in four recordings replay. Neither trial passes the
less-than0.1% rejection target. The different counts are not a controlled
performance comparison or evidence that changing the observer improved capture.

The first complete USB monitor retains452,120 selected-device events from452,127
bus events, zero queued/dropped at completion, and balanced submission/completion
identities. All1,202 accepted/ongoing-rejected frame associations match actual UVC
PTS, one FID, a single final EOF, and native image lengths after12-byte header
removal. Both rejected frames contain183 payloads instead of188: five missing
12,276-byte image blocks, with successful bulk statuses and normal UVC flags.

The final monitor retains453,530 selected-device events from453,537 bus events,
again with zero queued/dropped and balanced identities. Ftrace retains226,765
paired enqueue/giveback events and226,037 paired IRQ events, with zero reported
loss. All226,688 bulk completions match by order, actual length and a stable
bijection between ten request identities in the two traces. All1,206 accepted or
ongoing-rejected frames match UVC PTS/FID/EOF and native image lengths. There is
no extra unexplained native-byte discrepancy in this new sample.

| Rejected sequence L/R | Native and USB image bytes L/R | Missing image blocks L/R | Maximum completion gap L/R, us |
| --- | ---: | ---: | ---: |
| 158 / 159 | 2,242,620 / 2,242,620 | 5 / 5 | 2,047 / 2,026 |
| 219 / 220 | 2,181,240 / 2,181,240 | 10 / 10 | 2,895 / 2,872 |
| 279 / 280 | 2,119,860 / 2,181,240 | 15 / 10 | 3,011 / 2,989 |

All observed non-cancellation bulk completions have status0. Each short frame
retains a consistent PTS/FID and normal final EOF. The first incident overlaps
a1,915us IRQ entry/exit span; all six gaps end with five pending requests before
the next completion. The maximum accepted-frame internal gap is1,436us in this
sample. These facts place the observed missing image bytes **before application
recording**, at the host-controller observation boundary; they do not distinguish
what the device emitted from what the controller delivered. Software-pending
requests are not proof of requests still available to hardware during a stall.
The earlier20,664-byte discrepancy is not explained retrospectively by this run.

Both trials retain14 matching state-group comparisons each, five restored idle
formats, restored service/global tracing state and removed private instances.
The installed module, firmware settings and frame validation remain unchanged.
The final images were viewed: calibration board, right edge clipped, no measured
printed object. Final accepted spans are20.069107/20.069096s; whole native CLI
wall24.99s/user1.42s/system3.72s excludes monitor and kernel tracing costs.
No memory, synchronization or accuracy qualification follows.

Private `usbmon-ring-20261104` retains the original collector source, build and
framing/no-overwrite checks, actual tcpdump mapping probe, failed ftrace data,
complete USB data and header analysis. `usbmon-ring-allcpu-20261104` binds the
same reader plus the corrected orchestrator, all-buffer readbacks, complete
traces, `analyze_headers.py`, `correlate.py`, associations, raw recordings and
state/resource evidence. Collection uses unchanged application source21e85d2;
its push and PR workflows41680530781/41680549123 both pass. Raw evidence remains
private, and no paid or new completed CodeRabbit review is claimed.

**Next:** use this complete observation path to evaluate the known queue-depth
mitigation and distinguish interrupt/controller/device causes. Keep the prior
startup-framing and exceptional-byte cases separate. Physical reconstruction
also needs measured board scale and object dimensions; a board-spacing request
is pending, and no value is assumed from silence or nominal print dimensions.
