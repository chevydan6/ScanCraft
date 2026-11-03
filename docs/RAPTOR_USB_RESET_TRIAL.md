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

The post-reset trace below completes the next diagnostic from this comparison.
The native counter failure is now linked to the same displaced-header layout in
one newly traced reset incident. No production reset or relaxed validation follows.

The first independently measured object scan, full board coverage/physical units,
ongoing transport reliability and end-to-end native GUI qualification remain open.


## Complete trace across one reset and startup

A separate observation uses unchanged application source 907484e and repository
HEAD `6c44cd252a1f0fe4731337640f779953fd9a21de`. Its predeclared stopping rule is
the first post-reset test with at least ten startup rejects, or four reset/seed
pairs, retaining every attempt. The first pair meets the rule. This selected
failure observation is **not** a new reset success-rate estimate.

The seed and post-reset test each save 30 accepted frames/camera:

| Capture | Startup rejects L/R | Ongoing rejects L/R | Sequence gaps L/R |
| --- | ---: | ---: | ---: |
| Seed | 106 / 0 | 0 / 0 | 0 / 0 |
| After reset | 102 / 94 | 1 / 1 | 1 / 1 |

All **120 accepted frames in four recordings** pass complete native replay.
Their native ioctl traces have 2,878 rows including footers, zero lost/error rows
and 424 matching metadata returns before video reuse. All original rejects remain.

Tcpdump 4.99.6/libpcap 1.10.7 opens only the Raptor's discovered USB bus monitor,
filters its verified device address and explicitly drops to the owner account
before acquisition. Snapshot size is 245,824 bytes; monitor buffer is 65,536 KiB.
It records **49,890 packets**, with the same received count and zero reported
kernel drops. Every request ID balances; there are no pending/unmatched requests,
truncated capture packets or truncated successful bulk payloads. Twenty terminal
bulk completions have -ENOENT; 24,411 succeed. These are HCD/driver observations
for the selected address, not physical wire proof or root-hub reset packets.
The device address remains unchanged across the reset.

The reset ioctl succeeds in **148.869898 ms**. Successful SET_CONFIGURATION(1)
and alternate-setting-zero requests for interfaces 1, 3 and 5 occur within the
recorded reset call interval. Subsequent successful video COMMIT requests for
interfaces 1/3 specify 2,304,000-byte frames and 12,288-byte maximum payloads at
333,333 units of 100 ns per frame. Thus this observed failure follows acknowledged
configuration and stream negotiation, not merely a failed reset request.

Both post-reset streams begin with an **18,432-byte leading prefix**. The next
complete frame's headers are displaced **6,144 bytes modulo 12,288**. Each prefix
through its first short completion is 2,324,688 bytes. Offline analysis identifies
188 equally spaced, consistent headers: same PTS and FID, no error bit, EOF only
on the last 8,400-byte block. Removing those headers yields exactly 2,304,000
image bytes per camera. Both diagnostic images were inspected and show the real
board, with its right edge still clipped in the right view. These pixels are not
inserted into accepted recordings or counted as recovered qualified frames.

The leading prefix PTS in each camera is 33,337 ticks after that camera's last
observed seed header. The subsequent complete frame is 4,607,370 PTS ticks newer
than the prefix. **Engineering inference:** this supports persistence of old
stream material through the USB connection reset. It still does not locate that
material in a specific device FIFO, host-controller buffer or other component.
The seed itself has the same shift on the left, while its right stream is aligned.

For accepted frame index 10 in each of the four recordings, independent removal
of USB headers produces pixels byte-identical to the native exported PNG. This
checks stream/data association for accepted samples; it does not turn the offline
startup diagnostic into a production decoder or prove every image geometrically
accurate. The two continuing truncations remain separate errors.

Private `reset-usb-gixqz92r` retains the complete pcap, plan/stop rule, reset-call
and capture intervals, native recordings/replays, buffered ioctl traces,
`analyze_usb.py`, `analyze.py`, accepted-byte comparisons, diagnostic images and
state evidence, bound by SHA-256 manifests. All seven native snapshot groups and
five idle camera format/interval structures restore; service and installed module
are unchanged. Exact preceding head 6c44cd2 passed CI41649077142. No firmware,
application or driver implementation change is made in this observation.

## Higher-level SDK stop command: a lead, not confirmed Raptor support

The inspected MIT OrbbecSDK_v2 checkout is commit
`85e94c4ee4032817df2e33228dbd7f51517d3d4e`.
[VideoSensor::stop and trySendStopStreamVendorCmd](https://github.com/orbbec/OrbbecSDK_v2/blob/85e94c4ee4032817df2e33228dbd7f51517d3d4e/src/device/component/sensor/video/VideoSensor.cpp)
show a higher-level vendor stop after the backend stream closes, conditional on
that model's registered internal write support.
[InternalProperty.hpp](https://github.com/orbbec/OrbbecSDK_v2/blob/85e94c4ee4032817df2e33228dbd7f51517d3d4e/src/device/component/property/InternalProperty.hpp)
defines depth/left-IR/right-IR stop properties 38/39/139. These SDK definitions
are not evidence that the Raptor implements those commands or uses that mapping.
No upstream source is copied into ScanCraft by this investigation.

A fixed, read-only GET_PROPERTY probe returns status **0x2**, with no payload,
for all three IDs. No stop setter is sent. Some SDK models register these
properties as write-only, so failed reads do not establish failed write support.
The parseable records of an existing private vendor-bridge control log contain
no matching stop writes;
that limited log is not proof that CrealityScan never sends them. Native snapshots
still match after the read-only probes. Source hashes and query evidence are
retained privately in `sdk-stop-reference.json` and `stop-property-reads.json`.

**Next:** establish the device-specific stop/start command behavior from the
vendor path or a carefully bounded operational experiment, preserving failures
and state restoration. Evaluate framing recovery against complete retained bytes
and adversarial header patterns if needed. Do not equate GET failure with setter
support, add an arbitrary reset, or accept malformed V4L2 buffers to improve rates.
Physical units, the measured object and native end-to-end reconstruction remain
the qualification priority.

The subsequent [fixed vendor-stop experiment](RAPTOR_VENDOR_STOP_TRIAL.md) tests
this lead: 39/38 are acknowledged, but all four paired-stop arms still have
startup bursts. No production recovery change is justified by those results.
