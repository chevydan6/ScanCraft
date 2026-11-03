# Sustained Raptor capture with queued recording — 2026-11-03

**Confirmed:** the existing eight-frame recording worker and experimental
32-URB kernel queue completed a ten-minute dual-camera capture. All 36,000
accepted frames passed complete native replay. There were 11 ongoing rejects
and one startup reject: **0.033322% including startup**, below 0.1% for this
individual run. **Not qualified:** repeated reliability, normal-rate/longer live
scanning, exposure synchronization, calibrated geometry, and physical accuracy.
The experiment did not change application source or permanently install a driver.

## Exact implementation and method

Audited repository HEAD: `2bdb9348bf93af2c6671ace1230a75f82d0565de`.
Recording implementation: `2e3e2d8639b091a0001f803c64d1384dd1df2a15`.
Optimized CLI SHA-256:
`68335a91b77ec88f3b5cd32aa5c0d8ab8b46b1a259e9929ab467fdfe5a559b07`.

The [bounded recorder](BOUNDED_RECORDING.md) uses
`scancraft_recording::queued::QueuedWriter` from the finite CLI
`stereo_probe::{record_flood,capture}` path. It admits eight owned frames per
camera and fails explicitly on overflow. Existing validation, rejection counters,
recording layout, CRC/footer replay, restoration and publication remain intact.
This is not the separate live-stereo reconstruction runner.

The candidate, source reproduction, license, parameter-name mapping and installed
baseline are exactly those in [the earlier queue trial](RAPTOR_QUEUE_DEPTH_TRIAL.md).
Candidate SHA-256:
`46248a3c5e050db16182fdf0f034cd844f4e3dc503c037ffc8656b398c202700`;
loaded srcversion `10A4B6BC7346B07ADC2D755`. Kernel:
`7.2.11-6900hs-lto`, x86_64. The historical local binary uses `raptor_urbs=32`;
the published experiment names its parameter `scancraft_urbs`. Other parameters
were unchanged. Four V4L2 video and four metadata buffers were requested per
camera; those buffers are distinct from USB URBs and recording-worker capacity.

After confirming no running PipeWire nodes, the administrative harness held the
three normal scanner locks, saved idle camera formats/intervals, stopped idle
WirePlumber, checked video/media/subdevice consumers and used ordinary module
removal. No forced unload or permanent configuration change occurred. It loaded
the candidate separately before a 300-frame smoke test and an 18,000-frame test.
Locks were released for the ordinary-user CLI's own session locking and reacquired
for swaps. This remains cooperative coordination, not an atomic lock handoff.

Both captures used `stereo-flood-capture COUNT NEW_DIRECTORY --fps 30`, recording
1920 × 1200 GREY per camera. The observational write/ioctl/poll tracer was loaded
only into the native CLI under `/usr/bin/time -v`. No artificial write delay or
USB packet monitor was enabled. Native complete replay runs inside successful
capture publication and was repeated externally for each of the four recordings.
Private original captures, traces, scripts and reports are retained.

## Results and accounting

| Measurement | Smoke L/R | Ten-minute L/R |
| --- | ---: | ---: |
| Accepted frames | 300 / 300 | 18,000 / 18,000 |
| Startup rejects | 0 / 0 | 1 / 0 |
| Ongoing rejects | 0 / 0 | 6 / 5 |
| Sequence gaps | 0 / 0 | 6 / 5 |
| Accepted-frame missing metadata | 0 / 0 | 0 / 0 |
| Duplicate accepted PTS | 0 / 0 | 0 / 0 |
| Accepted host span, seconds | 9.967432 / 9.967443 | 600.241713 / 600.208378 |
| Accepted buffer rate, Hz | 29.997696 / 29.997663 | 29.986253 / 29.987919 |

For the sustained run, ongoing proxy = `11 / (36000 + 11)` = **0.030546%**;
all-rejection proxy = `12 / (36000 + 12)` = **0.033322%**. These count V4L2
buffers, not proven physical exposures. Startup is retained in the total.
No extra sequence gap beyond ongoing rejects was reported. The short smoke
passed without rejects but did not predict the longer run's failures.

Five ongoing events truncated both cameras at matching sequence/PTS values:
sequences 2820, 3001, 3786, 11696 and 15805. Payload sizes were respectively
2,058,480; 2,181,240; 1,812,960; 1,628,820; and 2,058,480 bytes versus expected
2,304,000. Video and metadata error flags were set. The deficits equal 20, 10,
40, 55 and 20 blocks of 12,276 bytes, consistent with the earlier observed USB
payload-block loss pattern. **Inference only:** this trace contains no USB packets
and does not locate the root cause in the device, HCD, transport or kernel.

The sixth left ongoing rejection is distinct: sequence 17575 has a full-size
video payload without a video error flag, but its metadata dequeue contains zero
bytes. The peer metadata contains 22 bytes. Native validation reports missing
sensor PTS and rejects the left frame. Trace evidence confirms the empty metadata
was returned by the ioctl; this is not just a report-formatting omission.
The accepted-frame missing-metadata counter being zero does **not** erase this
rejected frame. Its underlying cause remains unknown.

There are 17,998 exact raw PTS matches. The diagnostic nearest-host sample reports
2–162 µs deltas, median 19 µs, with equal raw PTS for those candidates. Both
`synchronization_verified` and exposure qualification remain false. Shared raw
timestamps and static images are insufficient to prove simultaneous exposure.

## Resource and image evidence

The sustained CLI, including its internal replay/finalization, took **652.63 s**:
39.61 s user and 57.88 s system CPU, reported 14% CPU, maximum RSS **55,824 KiB**,
zero swaps. External replays and image extraction are excluded. These are process
measurements, not machine CPU, kernel allocations, GPU usage or rendering FPS.

A separate `/proc` sampler observed 98 five-thread acquisition samples over a
485.102 s partial interval: RSS **28,188–37,052 KiB**, process CPU **8.726% of
one core**. This excludes most startup and finalization and includes the recording
workers and tracer. It is not a complete memory-leak qualification or a dedicated
idle-system benchmark; ordinary development/read-only diagnostic work continued.
The two sustained raw files total **82,945,950,762 logical bytes** before filesystem
compression, making explicit storage budgets important for longer sessions.

The tracer records 72,002 recording writes, zero trace errors, and no injected
delays. Maximum natural write duration was **107.200 ms**; 38 writes exceeded
33.333 ms. One right-camera video dequeue occurred on a different thread during
the longest write. Other long writes had no same-camera dequeue inside their
intervals. This supports separation of acquisition from storage, not a universal
service-latency guarantee or attribution of every pause to storage.

An original private Rust analysis harness linked the existing production Reader,
validated each complete recording again, then exported three selected equal-PTS
pairs with the existing PNG dependency. Library/binary hashes are retained.
Indices L/R are **0/1**, **8999/9000**, **17997/17999**, with host deltas 18, 18,
and 8 µs. All six images were visually inspected: the stationary calibration board
and dots remain recognizable without prominent horizontal banding in these
samples. The right board edge remains clipped. This is selected-image inspection,
not validation of every frame, new calibration pose coverage or an object scan.

## Restoration and reproducibility

The harness exited successfully after restoring the installed baseline module
SHA-256 `579c7633261b27cfd76af16f4b1ad57c1306ce6c7d75ca914e15693e3ec127e5`,
srcversion `9D1275CDD5E013864E4C564`, original module parameters and active
WirePlumber. A private helper restored saved V4L2 format/interval structures for
all five idle video-capture nodes, matched by sysfs identity, and compared fresh
GET results with their saved structures. Native Raptor left/right/color stream
state and illumination readbacks also matched the pre-experiment values.
This does not attest unsnapshotted controls, optical behavior or hard deadlines.

Local evidence set: `sustained-queued-hln5i2yy` under the developer's private
artifact directory. `plan.json` and `source-binding.json` bind the plan and source;
`run.py`, `capture.py`, `all_formats.py` and `camera_state.c` preserve administration,
capture and restoration. Per-trial stdout/stderr, raw files, replays, resources and
ioctl traces are retained. `analyze.py` reproduces `analysis.json` from them;
`missing-pts-trace-context.json`, `natural-write-pauses.json`, resource samples,
sample selections, extraction logs, images and module events retain detailed
evidence. SHA-256 manifests bind retained files. Private images and device data
are not included in the public repository.

The unchanged source had passed the 858-test gate, formatting, strict Clippy and
optimized builds; exact HEAD CI run 41632695649 also passed. This milestone adds
hardware evidence and documentation, not a new Rust implementation. CodeRabbit
quota lookup again returned `lookup_failed`; no new completed CLI review or paid
review is claimed. The observed empty PR-thread list is not independent approval.

## Remaining work

**Recommendation:** investigate the empty metadata event separately from paired
truncation, and reproduce sustained results across restarts/rates before considering
deployment of the candidate. One run below 0.1% does not establish reliability
distributions, a startup fix, 30-minute live scanning or disconnect recovery.
No new persistent kernel installation is justified by this result alone.

SCAN-REV-005 cleanup bounds and SCAN-REV-021 live-stereo capacity remain unresolved;
SCAN-REV-006 physical qualification remains under review. Do not increase one
frame cap and call the long-session design complete: association, recording,
history, artifact/reference limits, storage admission and recovery need coordinated
validation. Physical target scale, measured object dimensions, genuine surface
reconstruction, three repeatable dimensional trials and native end-to-end editing/
save/reopen/export remain required. The prepared asymmetric print is a measurement
target only after its cooled physical dimensions are independently measured.
