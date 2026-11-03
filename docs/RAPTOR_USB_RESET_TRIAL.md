# Idle USB connection reset does not reliably fix startup — 2026-11-03

**Confirmed:** four successful idle Raptor USB connection resets did not reliably
prevent startup rejection bursts. Two reset trials still rejected 96 and 190
startup buffers respectively; all four control trials also had bursts. This is
negative evidence against using an unconditional connection reset as a reliable
startup remedy. It does not locate the defective state or rule out every possible
reset/recovery procedure. No production reset path was added.

## Fixed implementation and intervention

Repository HEAD: `efabf9bdb5f94a7bdcd260154590842480cb5334`.
Application implementation: `907484e4fa2967e45ca10840fc48f0eed523f124`.
Release CLI SHA-256:
`3a46c4995b134912482af20a303e76c024ea6cea38e699935a6a4eef430cae55`.
The **installed baseline driver** remains loaded throughout, srcversion
`9D1275CDD5E013864E4C564`; no temporary deeper-queue candidate is loaded here.
Module parameters and application validation/counters remain unchanged.

Eight test captures follow counterbalanced order **ABBA BAAB**, A = control,
B = USB connection reset. Each test is preceded by its own 30-accepted-frame/camera
seed capture, then records 60 accepted frames/camera. Both use the existing finite
`stereo-flood-capture ... --fps 30` command, 1920 × 1200 GREY, temporary flood and
the corrected metadata return order. Every seed and test is retained, including
all startup/ongoing rejects, sequence gaps and failed-quality results.

The administrative harness checks normal scanner/Wine locks, idle PipeWire and
absence of camera/media/Raptor USB-device consumers. It temporarily stops idle
WirePlumber. A small original private C helper opens the freshly resolved USB
device node, verifies a character device and its exact 18-byte USB descriptor
(VID/PID 2bc5:06a8), then issues Linux `USBDEVFS_RESET` once and closes it. The
helper passes a warnings-as-errors build and a `/dev/null` rejection test; the
latter issues no reset. Local Linux 7.2.11 `drivers/usb/core/devio.c` function
`proc_resetdevice` delegates this ioctl to `usb_reset_device`. This is a USB
connection reset, not a power cycle, firmware write or evidence of a sensor reset.

Both arms wait until at least three seconds after intervention-window start,
settle udev, restore the same saved idle format/interval structures and verify
native state before capture. All four reset calls return success and take
148.716–149.466 ms. These are ioctl durations, not complete reconnection latency.
The procedure includes driver/device reinitialization effects; it does not isolate
a particular FIFO. No automatic reset retry or forced device access is used.

## Results

All counts are V4L2 rejected-buffer proxies, not physical exposure counts.
Seed captures are part of the experiment, not a discarded warmup:

| Test | Arm | Seed startup L/R | Seed ongoing L/R | Test startup L/R | Test ongoing L/R |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | Control | 94 / 89 | 0 / 0 | 0 / 93 | 0 / 0 |
| 2 | Reset | 0 / 1 | 1 / 1 | 0 / 0 | 1 / 1 |
| 3 | Reset | 0 / 98 | 0 / 0 | 1 / 95 | 0 / 0 |
| 4 | Control | 0 / 93 | 1 / 1 | 87 / 1 | 1 / 1 |
| 5 | Reset | 1 / 1 | 0 / 0 | 0 / 1 | 0 / 0 |
| 6 | Control | 0 / 1 | 0 / 0 | 0 / 95 | 0 / 0 |
| 7 | Control | 96 / 99 | 0 / 0 | 98 / 0 | 1 / 1 |
| 8 | Reset | 1 / 83 | 1 / 1 | 102 / 88 | 0 / 0 |

Control tests total 480 accepted buffers, 374 startup rejects and four ongoing
rejects. Reset tests total 480 accepted, 287 startup and two ongoing rejects.
Seeds total 480 accepted, 657 startup and six ongoing rejects. All accepted-sequence
gaps equal the corresponding ongoing rejects. These short, four-test groups do
not establish a statistically reliable rate improvement; both still fail reliable
startup. Reset failures are retained and not replaced with the cleaner trials.

All **1,440 accepted frames in 32 recordings** pass complete native CRC/order/footer
replay. Recording data total 3,317,945,952 logical bytes. The unchanged private
20,000-row buffered tracer retains 20,252 rows across all 16 processes, including
one footer per process, with zero lost/error rows and no injected delays. All
2,770 traced matching metadata returns precede video reuse; none is unmatched.
The previously tested tracer SHA-256 is
`9c19a1982720d552bb36df050c9742e5ad4770dd889f7954f6aeb2655f802bd8`.
Its CPU/memory/scheduling overhead remains; this is not a throughput benchmark.

## Restoration, evidence and next step

Every seed/test restores its native IR/color stream, illumination, standard
control, reported device-version and mirror-property snapshots. The same seven
snapshot groups match the initial baseline after each intervention and at the
end. Saved format/frame-interval structures for all five idle capture nodes also
restore; WirePlumber is active, the installed module/parameters are unchanged,
and the harness exits zero. Reported version equality is not authenticated
firmware identity or a proof about every unsnapshotted device state.

Private evidence `startup-reset-b7itq90j` retains the plan, reset helper/source,
harness, descriptor guard result, per-trial raw recordings/outputs/traces,
`analyze.py`, `aggregate.py`, reset timestamps, all state comparisons and a
SHA-256 inventory. Nothing from the private image/device payloads is published.
Unchanged source has the existing 863-test/fmt/Clippy/release gate; exact preceding
HEAD efabf9b passed CI41646883074. New evidence is hardware/replay/trace/restoration,
not additional synthetic algorithm tests or independent review approval. The new
CodeRabbit quota check again returns `lookup_failed`; no new completed or paid
review is claimed.

**Next diagnostic:** correlate a successful connection reset, UVC negotiation and
initial payload headers in one complete USB trace to determine whether post-reset
bursts share the previously demonstrated shifted-header mechanism. Existing
[startup framing evidence](RAPTOR_USB_STARTUP_FRAMING.md) establishes that mechanism
for earlier non-reset captures; these native counters alone do not prove it for
this new intervention. Any future bounded framing recovery must preserve rejected
bytes/errors and reject false headers in image data. An arbitrary production reset
or relaxed frame validation is not supported by these results.

The first independently measured object scan, full board coverage/physical units,
ongoing transport reliability and end-to-end native GUI qualification remain open.
