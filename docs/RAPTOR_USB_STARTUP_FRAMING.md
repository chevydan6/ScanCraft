# USB startup framing evidence — 2026-11-03

Confirmed: one right-camera startup burst contains a complete image whose UVC
headers are shifted by6,144bytes relative to the12,288-byte host transfer
boundaries. Interpreting the observed transfer starts as payload headers explains
the delivered buffer lengths. This establishes a concrete framing mechanism for
this incident; it does **not** identify whether device FIFO state, host-controller
behavior, negotiation or another cause introduced the shift.

No application, driver, kernel or firmware change was made. Rejected buffers and
qualification counters remain unchanged. The diagnostic recovered image is not
inserted into an accepted recording or counted as a qualified frame.

## Capture and completeness

Source `68edb336e1e31b2fa053646a6179b23a96db15e8`, unchanged acquisition code, ran
`stereo-flood-capture 30 NEW_DIRECTORY --fps 30` with the preceding observational
ioctl tracer. No serialized-start intervention was enabled. This is60 accepted
raw frames in two recordings. Startup rejections were1/96 (left/right), ongoing
rejections0/0, missing metadata0/0. Both recordings passed native integrity replay;
reported format/rate and illumination readbacks matched before/after.

A privileged tcpdump opened only the Raptor's current USB bus monitor, then
explicitly dropped to the owner account. Its filter retained only the discovered
Raptor device address. No permissions, kernel module or device firmware changed.
The monitor was stopped after acquisition, readbacks and replay finished.

- tcpdump4.99.6, libpcap1.10.7; USB_LINUX_MMAPPED,245,824-byte snapshot limit.
-24,420 packets captured and received by the filter; **zero reported kernel drops**.
- All submission/completion IDs balanced; no pending or unmatched IDs at the end.
- All successful bulk completion payloads were captured completely, with no
  pcap or payload-length truncation.
-10 terminal bulk completions had `-ENOENT`, consistent with stream cancellation;
  the other12,046 bulk completions had status0. This is not a zero-error wire claim.

The [Linux usbmon documentation](https://docs.kernel.org/usb/usbmon.html) explains
that this monitor records driver/HCD requests, not physical wire transactions.
Zero reported monitor drops and balanced events strengthen this trace's
completeness but cannot prove that the HCD faithfully represents every physical
bus event. Interception and monitoring also affect timing; no speed claim follows.

## Observed byte layout

Both video COMMIT controls declared2,304,000-byte frames and12,288-byte maximum
payload transfers. Successful left completions begin with the expected12-byte
header pattern. On the right, the first completion starts with such a header,
but the next188 completion boundaries do not. They instead contain image-like
bytes that the driver can interpret as short headers and frame flags.

Concatenating the right startup prefix yields2,312,400bytes. At offset6,144,
187 full12,288-byte payload blocks followed by one8,400-byte block each contain
consistent12-byte headers: same PTS, no error bit, EOF only on the last block.
Removing those headers gives exactly2,304,000 image bytes,1920×1200 GREY.
The losslessly rendered image was inspected; it depicts the actual board. Its
right edge is currently clipped, as is a normally accepted image from this capture;
this setup is not accepted as a full-board calibration pose.

The leading prefix has an older PTS,33,337 ticks after the last accepted right
frame of the preceding hardware trial. **Engineering inference:** residual data
from the earlier stream is a plausible contributor. The numerical relationship
alone does not prove a stale FIFO, nor establish a hardware flush remedy.

The startup prefix and following complete frame have matching FID parity. The
left stream's initial partial payload plus a new frame can therefore overflow one
frame buffer. The right stream's shifted starts additionally expose image bytes
as apparent FID/EOF/header-length values, producing many small buffers.

## Driver and application correlation

Local kernel `uvc_video.c` functions `uvc_video_decode_bulk`,
`uvc_video_decode_start`, `uvc_video_decode_data` and `uvc_video_decode_end` explain
the relevant boundary, FID, EOF and overflow behavior. The loaded module's
srcversion matches the recorded on-disk Raptor module; installed and build-tree
module bytes are equal. Exact hashes and source paths are retained privately.
This is source/module identity evidence, not a readback of resident kernel code.
The analogous [upstream Linux source](https://github.com/torvalds/linux/blob/master/drivers/media/usb/uvc/uvc_video.c)
is a reference; mutable upstream master is not asserted to be the installed revision.

A private finite behavioral model of those boundaries reproduces **31/31 left and
126/126 right dequeued buffer lengths**, including all97 startup-rejected buffers.
The initial model omitted capacity clamping and missed two overflow lengths;
that model is retained, and the corrected model includes immediate overflow
completion. This is a trace-specific explanatory model, not a complete kernel
emulator, production decoder or independent fix qualification.

For a normally accepted frame from each camera, direct USB-header removal yields
pixels byte-identical to the native lossless exported image and matching the raw
recording's payload CRC. Thus the trace-to-camera association also holds for
accepted data. ScanCraft's `Capture::next_frame_until` rejects the malformed
kernel-delivered buffers; no evidence here supports weakening that validation.

## Reproduction, provenance and next action

Private artifact `startup-usb-ajzgmkrh` retains:

- Full pcap, monitor statistics, device discovery and source/module/binary hashes.
- `capture.py`, original recordings, native reports, ioctl trace and readbacks.
- `analyze_pcap.py`, `framing-analysis.json`, all buffer comparisons and the initial
  unclamped model.
- `verify_shift.py`, header/PTS/EOF checks, diagnostic recovered pixels and image.
- Accepted-image byte-equality results and a SHA-256 artifact inventory.

Run the retained analyzer and verifier against the original pcap to reproduce the
numbers. These scripts contain private diagnostic assumptions and real data;
none is installed as a production recovery path. Kernel code remains under its
GPL terms and was not copied into ScanCraft's MIT application. The published
change is documentation only.

The next useful test is to reproduce the boundary shift across controlled restart
trials and determine which part of stop/start negotiation leaves the leading
prefix. Both the current kernel and inspected OrbbecSDK_v2 stop paths already
issue endpoint clear-halt operations; their existence is not evidence that adding
an arbitrary reset or another clear fixes this device. Any recovery candidate must
retain discarded bytes/errors, resist false header matches inside image data and
pass independent real-stream integrity tests. No firmware flash or kernel patch
is justified by this one trace.

Continuing-frame rejection, reliable startup, physical units, calibration coverage,
first measured object surface and end-to-end scanning remain unqualified. See
[start-order experiments](RAPTOR_STARTUP_INVESTIGATION.md) for the preceding
negative serialization result.

## Consecutive restart reproduction

A continuous second monitor covered three consecutive30-frame-per-camera captures
with the same application and installed driver. The third started after an extra
three-second pause. All180 accepted frames in six recordings passed native replay;
all reported stream/illumination readbacks restored. The monitor retained72,342
packets with zero reported kernel drops, balanced IDs and no truncated successful
bulk payloads. These are short instrumented observations, not duration qualification.

| Capture | Startup rejected buffers L/R | Ongoing rejected buffers L/R |
| --- | ---: | ---: |
|1 |0 /1 |0 /0 |
|2 |0 /106 |1 /1 |
|3, after extra3-second pause |0 /98 |0 /0 |

Capture1's right startup has24,576 leading bytes before a full frame; this is an
integer multiple of the maximum transfer size, and transfer/header alignment is
retained. Captures2 and3 each have18,432 leading bytes before a full frame, again
leaving headers displaced6,144bytes modulo12,288. All six first-prefix frame
analyses identify188 consistent headers and2,304,000 image bytes, with EOF only on
the final header. This does not convert the rejected native buffers into passes.

For captures2 and3, the first right header's PTS exactly equals the preceding
capture's **last observed left** header PTS, and is33,337 ticks after the preceding
last observed right header. Successful stop-time CLEAR_FEATURE(HALT) requests for
both video endpoints appear in the continuous trace between captures. The right
completion-to-next-start intervals were about0.64 and3.89seconds. **Engineering
inference:** the repeated timestamp relationship supports carry-over from the
preceding stereo stream. The three-second wait does not prevent this incident.
This still does not locate the retained bytes physically or demonstrate that an
extra clear-halt, reset or firmware change would fix the problem.

## Separate continuing-frame truncation

Capture2 also retained one rejected ongoing frame per camera at the same sensor
PTS. Each had1,997,100 pixel bytes instead of2,304,000. Summing its successful
USB payloads, after their12-byte headers, equals the native rejected length
exactly:163 payloads including one EOF, rather than188 for a complete frame.
The missing306,900bytes equal25 full12,276-byte image-payload blocks.

The longest within-frame completion gap was4,539microseconds left and
4,656microseconds right. Complete frames in this capture had maximum gaps of305
and284microseconds respectively. There were **no observed zero-outstanding-URB
intervals within either truncated frame**. Thus a simple absence of submitted work
is not established. Pending requests in a software trace do not prove the HCD had
usable buffers throughout a delayed completion callback; larger queues, host
scheduling, controller behavior and device production remain hypotheses.

This correlation separates an ongoing short frame from the stale-prefix startup
mechanism. Neither frame is repaired, accepted or excluded from error accounting.
A deeper queue experiment must compare actual integrity and rejection evidence,
not infer improvement from compilation or these timings. The existing separately
licensed [queue candidate](../experiments/kernel-queue/README.md) remains unloaded
and unqualified at this milestone.

Private evidence `restart-usb-qitbu8ek` retains `capture.py`, `runs.json`, all native
outputs/recordings/readbacks, full `restarts.pcap`, monitor counters,
`analyze_restarts.py`, `analyze_queue_gaps.py`, their JSON results and SHA-256
inventory. Exact source is `c3ba2fd7486a260658ad733f26a6e5660c4ce94b`; no application,
module, kernel or firmware changes were made in these reproduction trials.


## Idle connection reset follow-up

A later [counterbalanced reset experiment](RAPTOR_USB_RESET_TRIAL.md) retains
large startup bursts in two of four successful USB connection-reset trials.
All captures/rejections are retained and checked state restores. This rules out
assuming that a successful reset reliably fixes startup. The initial native counters did not establish the shifted-header layout. A
subsequent complete trace in that report now verifies the same 6,144-byte shift
after acknowledged reset/configuration and UVC negotiation in both cameras. Source 907484e preserves strict
validation and corrects metadata return order, a separate reproduced defect.
