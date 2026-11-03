# Raptor vendor stream-stop investigation

## Result and scope

**Confirmed:** Raptor replies successfully to SDK stop properties 39 and 38, but
sending both after native capture does not reliably clear startup framing faults.
All four intervention tests in the fixed eight-test comparison below retain a
startup burst. No production stop-command path or relaxed validation is added.
An acknowledged request does not establish the internal action or sensor mapping.

Repository head at acquisition: `351f53f99b4a36ef7e0758942ccb489bda2fe810`.
Application implementation: `907484e4fa2967e45ca10840fc48f0eed523f124`.
Actual CLI SHA-256:
`3a46c4995b134912482af20a303e76c024ea6cea38e699935a6a4eef430cae55`.
Installed baseline module srcversion `9D1275CDD5E013864E4C564` and its parameters
remain fixed. No temporary queue candidate, USB reset, firmware write or permanent
installation occurs. Captures use the existing native scoped flood command,
1920 × 1200 GREY at requested 30 FPS; raw validation and rejection accounting
are unchanged. These are acquisition experiments, not calibrated object scans.

## Source of the hypothesis

The MIT OrbbecSDK_v2 revision
`85e94c4ee4032817df2e33228dbd7f51517d3d4e` conditionally sends a vendor stop after
closing its backend stream in
[VideoSensor::stop / trySendStopStreamVendorCmd](https://github.com/orbbec/OrbbecSDK_v2/blob/85e94c4ee4032817df2e33228dbd7f51517d3d4e/src/device/component/sensor/video/VideoSensor.cpp).
The generic IDs are left IR 39 and right IR 139. Some models instead alias right
IR stop to depth stop 38; see
[G2Device property registration and alias](https://github.com/orbbec/OrbbecSDK_v2/blob/85e94c4ee4032817df2e33228dbd7f51517d3d4e/src/device/gemini2/G2Device.cpp).
[Protocol::initSetPropertyReq](https://github.com/orbbec/OrbbecSDK_v2/blob/85e94c4ee4032817df2e33228dbd7f51517d3d4e/src/device/protocol/Protocol.cpp)
uses opcode 2, four payload halfwords, property ID and integer value. No upstream
source is copied into ScanCraft in this investigation. Device-specific support
and behavior require evidence; the G2 mapping is a hypothesis for this Raptor.

The preceding [read-only probes](RAPTOR_USB_RESET_TRIAL.md) returned status 0x2
for GET_PROPERTY 38/39/139. Because these commands can be write-only, that result
was not treated as evidence that their setters were unsupported.

## Initial fixed setter probe

After a retained 30-frame/camera seed capture, a private fixed allowlist sends
39=true once, then 139=true once through the discovered vendor extension unit.
There are no arbitrary property/value arguments, firmware commands or retries.
Response magic, opcode, request ID and bounded response length are checked.

| Property | SDK meaning | Device reply status |
| --- | --- | ---: |
| 39 | Left IR stop | 0 |
| 139 | Generic right IR stop | 2 |

The next native 30-frame/camera capture has startup rejects L/R **0/101** and
ongoing rejects/gaps **1/1**. The seed has startup **0/1**, ongoing **0/0**.
All 120 accepted frames/four recordings replay completely. The USB monitor retains
47,456 packets, balanced requests, full successful payloads and zero reported
kernel drops. The right post-command stream has a 30,720-byte prefix and the
6,144-byte displacement modulo 12,288; its first header PTS equals the preceding
last left PTS. Its offline diagnostic image shows the actual board, still clipped
at the right edge. It is not inserted into accepted recordings.

This probe establishes a difference in replies, not that property 39 corrected
the left stream: the left seed was already aligned. Seven native snapshot groups,
five idle camera formats/intervals, WirePlumber and module state restore.

## Fixed paired-command comparison

A new predeclared **ABBA BAAB** series compares A=no stop commands with B=39=true
then 38=true. Each arm starts with a fresh 30-frame/camera seed, followed by a
three-second intervention window and a 60-frame/camera test. Both arms restore
the same saved idle format structures and run the same native state checks before
the test. The window is measured from the intervention start, including command
time. All seeds, successful trials and failures remain in the results.

Cooperative session locks, no camera/media/USB holders and no active PipeWire nodes
are required. Idle WirePlumber is stopped and restored. Each setter is sent once;
a nonzero/malformed reply or failed capture/state check aborts the entire series.
An initial generated-script indentation error was caught by compilation before
any hardware call; the failed text and correction are retained privately.

All eight setter requests (four each of 39/38) return status 0. Independent pcap
parsing matches all eight transmitted requests and eight replies byte-for-byte
to the command log, including matching IDs and successful USB completion. The
following startup failures therefore cannot be described as rejected setters.

| Trial | Arm | Seed startup L/R | Test startup L/R | Test ongoing L/R | Test sequence gaps L/R |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | Control | 0/1 | 0/1 | 1/1 | 1/1 |
| 2 | Stop 39+38 | 0/111 | 99/1 | 0/0 | 0/0 |
| 3 | Stop 39+38 | 102/0 | 99/108 | 0/0 | 0/0 |
| 4 | Control | 1/0 | 91/0 | 0/0 | 0/0 |
| 5 | Stop 39+38 | 96/88 | 1/100 | 1/1 | 1/1 |
| 6 | Control | 0/94 | 0/1 | 1/1 | 1/1 |
| 7 | Control | 1/0 | 80/0 | 0/0 | 0/0 |
| 8 | Stop 39+38 | 94/97 | 97/93 | 0/0 | 0/0 |

Control tests: 480 accepted, 173 startup rejects, four ongoing rejects.
Stop tests: 480 accepted, 598 startup rejects, two ongoing rejects.
Seeds: 480 accepted, 685 startup rejects, three ongoing rejects.
The seed ongoing rejects are one paired event in seed 3 and one left event in
seed 8. Short observations, seed histories and diagnostic overhead limit rate
interpretation. This does not establish that stop commands make failure more
likely, or improve ongoing transport. It directly disproves reliable startup
recovery by this particular post-close sequence.

## Integrity, USB framing and restoration

All **1,440 accepted frames in 32 recordings** pass full native CRC, order and
footer replay. The 21,062 ioctl trace rows include all 16 footers, with zero trace
errors/loss and 2,905 matching metadata returns before video reuse; no unmatched
returns occur. Gaps match ongoing rejections. No artificial delay is injected.

The continuous monitor retains **568,622 packets**, matching its received count,
with zero reported kernel drops, balanced request IDs and no truncated capture
packets or successful bulk payloads. It uses the same selected-device USB/HCD
monitor as the reset trial: 245,824-byte snapshot, 65,536 KiB monitor buffer,
explicit drop to the owner account. This is not a physical wire observation.

The first complete frame's header displacement modulo 12,288 is:

| Test | Left | Right |
| --- | ---: | ---: |
| Control 1 | 0 | 0 |
| Stop 2 | 6,144 | 0 |
| Stop 3 | 6,144 | 6,144 |
| Control 4 | 6,144 | 0 |
| Stop 5 | 0 | 6,144 |
| Control 6 | 0 | 0 |
| Control 7 | 6,144 | 0 |
| Stop 8 | 6,144 | 6,144 |

Thus every stop arm still contains a displaced stream. Header chains preserve
consistent PTS/FID, error-bit checks, final EOF and exact image-size requirements.
Offline diagnostic reconstruction is kept separate from native accepted frames.
For accepted index 10 in every recording, all 32 native exported images equal
USB-derived pixels byte-for-byte (`verify_accepted_pixels.py`). This samples
stream association; it does not qualify geometric accuracy.

All seven native groups (left/right/color stream, illumination, standard controls,
version, mirror) match after every intervention and at final restoration. All five
idle camera format/interval structures restore; WirePlumber is active; installed
module and parameters match baseline. These checks do not expose every internal
firmware state or prove the physical effect of a successful stop reply.

Private evidence directories `vendor-stop-ou3_pbkp` and
`vendor-stop-pair-p55hloh5` retain fixed plans, scripts, exact requests/replies,
complete pcap, recordings/replays, ioctl traces, state snapshots and SHA-256
manifests. Reproduce the analysis with `analyze.py`, `analyze_usb.py`,
`verify_stop_controls.py` and `aggregate.py` in the paired directory. Raw sensor
and device data remain outside the public repository.

## Decision and next work

**Engineering decision:** do not add these setters as a startup fix. Successful
replies, clear-halt and USB reset have each proved insufficient in observed
sequences. No conclusion about other command timing or firmware internals follows.
Further transport changes need a concrete mechanism and a retained failure test;
blind reset/stop retries would add complexity without demonstrated reliability.
Bounded framing recovery, if pursued, must retain discarded-byte/error accounting
and reject header-like image data before hardware deployment.

The unchanged Rust implementation retains its 863-test/fmt/strict-Clippy/release
gate; exact acquisition head 351f53f passed CI41652524927. No new Rust gate is
claimed for these documentation-only changes. CodeRabbit quota remains unavailable
from the preceding same-session check; no paid/completed new review is claimed.
Independent review findings remain under reviewer control. Genuine measured object
reconstruction, verified physical scale, three-trial reproducibility and native
GUI qualification remain the mission priority and are still incomplete.
