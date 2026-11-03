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
