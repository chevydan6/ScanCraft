# Raptor USB queue-depth trials — 2026-11-03

Later [complete-header comparison](RAPTOR_QUEUE_HEADER_COMPARISON.md) observes an
ongoing paired short frame even at depth 32. The finite zero-rejection results
below remain historical observations, not a guarantee or qualified fix.

Follow-up: [ten-minute capture with the queued recorder](RAPTOR_SUSTAINED_QUEUED_CAPTURE.md)
extends the short-trial evidence below. It retains nonzero truncation/metadata
failures and does not replace this ABBA comparison or establish release readiness.

**Confirmed:** two controlled ABBA comparisons produced 37 ongoing rejected
buffers with five USB URBs and zero with 32 URBs, for 8,400 accepted buffers in
each arm. All 16 recordings passed native integrity replay. **Not qualified:**
startup corruption persists, one depth-32 stream has a sequence/timestamp gap,
and the longest individual capture is about one minute. The original installed
module, its parameters, saved IR format/rate and illumination readbacks, and
WirePlumber service were restored. The candidate is not installed persistently.

## Source and experiment binding

Application source: `23d075c30244b489c612b622291142e85b48b5f3`; unchanged optimized
CLI SHA-256 `aebda4d4b493536668f759e9ef4c89d8d3940de9a79258552968355aec9e914a`.
Acquisition uses `crates/scancraft-cli/src/stereo_probe.rs::{record_flood,capture}`
and the existing device validation, metadata matching, recording writer and
completion barrier. No error flags, length checks, counters or parser rules changed.

The existing [GPL-2.0-only queue experiment](../experiments/kernel-queue/README.md)
changes `uvc_num_urbs`, allocation/iteration bounds and a load-time parameter.
The local historical candidate names the parameter `raptor_urbs`; the published
patch names it `scancraft_urbs`. Applying the published patch to a clean baseline
copy with this explicit identifier mapping reproduces all three modified files
exactly. All remaining C/header sources match the baseline. This name difference
must be accounted for when reproducing a test; the names are not interchangeable
arguments to the same binary.

A separate source copy rebuilt with the existing matching Clang/LLD container
produces a **byte-identical** candidate. The policy harness, adapted privately
for the actual path, historical identifier and host compiler, passes all 79
existing cases. It tests policy functions, not kernel allocation or USB behavior.

| Binding | Value |
| --- | --- |
| Kernel/vermagic | `7.2.11-6900hs-lto SMP preempt mod_unload` |
| Candidate module SHA-256 | `46248a3c5e050db16182fdf0f034cd844f4e3dc503c037ffc8656b398c202700` |
| Candidate srcversion | `10A4B6BC7346B07ADC2D755` |
| Installed/restored module SHA-256 | `579c7633261b27cfd76af16f4b1ad57c1306ce6c7d75ca914e15693e3ec127e5` |
| Installed/restored srcversion | `9D1275CDD5E013864E4C564` |

Both arms use the same candidate binary, loaded with depth 5 or 32 before every
trial. This controls for the candidate's larger structure arrays. Other module
parameters are copied unchanged, including `nodrop=1` and `hwtimestamps=0`.
The override selects only the original Raptor IR streaming interfaces; policy
tests retain five URBs for other interfaces/devices. Other cameras were not
activated or optically qualified in this experiment.

## Procedure and cleanup

An initial ordinary unload was refused with five module references. Checking
only `/dev/video*` was insufficient: WirePlumber held all five `/dev/media*`
handles. PipeWire reported no running media nodes. Briefly stopping the idle
WirePlumber service released those references; ordinary removal then succeeded.
No forced removal, interface unbind, installed-file replacement, persistent
module option or firmware write was used.

Administrative swaps hold all three normal scanner locks, check video, media
and subdevice consumers, and abort on conflicts. The locks are released while
the ordinary-user native CLI acquires its own full session locks, then reacquired
before each swap. This is cooperative coordination, not an atomic lock handoff
or exclusion of arbitrary external camera clients. WirePlumber stays inactive
in both comparison arms. Cleanup restores the original module/parameters and
service, including on the first harness failure.

Every trial records 1920 × 1200 GREY from both cameras at requested 30 FPS,
with temporary flood illumination and an observational ioctl tracer. No USB
packet monitor is active. The native executable runs under `/usr/bin/time -v`;
the preload is enabled only when `env` execs the actual CLI. Raw data remain
private. Each trial then runs both complete native replays and compares native
format/rate/illumination readbacks before and after capture.

The first 30-frame/camera smoke capture succeeded and replayed, but its timing
wrapper inherited the preload and created the exclusive trace file before the
CLI could open it. The required nonempty-trace assertion failed. That run and
the original harness are preserved, excluded from comparisons, and followed by
a corrected 30-frame/camera smoke run. Both retained 60 accepted buffers.

After the short ABBA experiment, module/service re-enumeration changed the IR
formats despite successful per-capture restoration. A private cleanup helper
restored the saved 1280 × 800 GREY/30 FPS settings through ordinary V4L2 setters
under the normal locks. Native readbacks then matched exactly. The longer
orchestrator incorporates this cleanup after restoring the service. This does
not establish restoration of unsnapshotted color/other-camera settings or a hard
cleanup deadline; SCAN-REV-005 remains separate.

## Results

Order is A1/B1/B2/A2, followed by A3/B3/B4/A4. A means depth 5; B means depth 32.
All counts are V4L2 buffers, not necessarily physical exposures. Startup rejects
remain separate and are **not** silently excluded from the complete proxy.

| Trial | URBs | Accepted per camera | Startup rejects L/R | Ongoing rejects L/R | Accepted rate L/R (Hz) |
| --- | ---: | ---: | ---: | ---: | ---: |
| A1 | 5 | 300 | 1/103 | 1/1 | 29.8961/29.8954 |
| B1 | 32 | 300 | 0/1 | 0/0 | 29.9961/29.9961 |
| B2 | 32 | 300 | 86/1 | 0/0 | 29.9956/29.9960 |
| A2 | 5 | 300 | 89/0 | 2/2 | 29.7969/29.7979 |
| A3 | 5 | 1800 | 89/0 | 9/9 | 29.8469/29.8303 |
| B3 | 32 | 1800 | 91/0 | 0/0 | 29.9796/29.9963 |
| B4 | 32 | 1800 | 1/1 | 0/0 | 29.9965/29.9965 |
| A4 | 5 | 1800 | 92/93 | 6/7 | 29.8967/29.8800 |

- Depth 5: 8,400 accepted, 37 ongoing rejects, 467 startup rejects.
  Ongoing proxy `37 / (8400 + 37)` = **0.438545%**;
  all-rejection proxy `504 / (8400 + 504)` = **5.660377%**.
- Depth 32: 8,400 accepted, zero ongoing rejects, 181 startup rejects.
  Ongoing proxy **0%**; all-rejection proxy `181 / (8400 + 181)` = **2.109311%**.
- All 16,800 accepted buffers in 16 recordings pass complete native CRC/footer
  replay. Every trial reports zero missing metadata and zero duplicate PTS.
  All per-trial format/rate/illumination comparisons pass.

**Engineering inference:** changing only queue depth in this candidate reduced
the observed ongoing rejection mechanism under these conditions. This supports
longer queue-depth experiments; it does not prove a particular HCD/device cause,
universal reliability, a statistically independent error process, or compliance
with the complete acquisition target. Depth 32 plainly does not fix startup.

## Sequence gaps and image usability

B3 left has one native-reported sequence gap despite zero rejected buffers:
sequence 1763 to 1765, sensor PTS and host timestamp both advancing 66,674 units
and microseconds respectively. Right has no corresponding gap. The ioctl trace
contains no video or metadata dequeue for sequence 1764. Earlier, dequeue calls
for consecutive video buffers 1759 and 1760 are separated by about 153.1 ms;
1760–1763 then return in a short batch. The native device code requests four
V4L2 buffers (`crates/scancraft-device/src/lib.rs`, stream initialization).

That pattern is consistent with delayed application servicing and finite video
buffer capacity, but the trace does not identify whether storage, scheduling or
another operation caused the delay. No QBUF/REQBUFS or system-wide scheduling
trace was collected here. A3 right also has one more sequence gap than reported
ongoing rejects. **Rejected-buffer rate alone is not a losslessness metric.**

Native `extract-frame` validates complete recordings and exports equal-PTS pairs
from A3 and B3 (left index 900, right index 902). Host deltas are 5 and 6 µs.
All four images were inspected: the board is recognizable, without prominent
horizontal banding in those samples; the right board edge is still clipped.
This is selected-image inspection, not all-frame optical validation, exposure
synchronization, new calibration coverage or an object reconstruction.

## Resource observations and limits

`time -v` records maximum process RSS of 31,264–31,804 KiB across the comparison
captures. Reported process CPU is 17–18% in short trials and 22–23% in longer
trials. These include native recording work and the observational interposer;
they exclude later replay and do not measure machine-wide CPU, kernel USB queue
memory, GPU use or rendering. Per-trial user/system times are retained privately.
No resource advantage of either queue depth is established by these measurements.

The finite CLI accepts these 1,800-frame captures. This does **not** remove the
separate live IR runner's 1,000-frame limitation (SCAN-REV-021). No continuous
ten-/thirty-minute qualification, 50 FPS comparison, disconnect recovery, laser
reconstruction, physical units, three measured scans or native GUI end-to-end
acceptance was completed by this experiment.

## Reproduction evidence and next action

Private artifact `queue-preflight-v7m5pshf` retains source/build comparisons,
patch application, policy/build logs, original and corrected administrative
harnesses, module/service transitions, exact commands, all recordings and replay
outputs, ioctl traces, resource logs, explicit format restoration and selected
images. `analyze_trials.py` regenerates `trial-analysis.json`; gap context and
sample selection have separate JSON records. A SHA-256 inventory binds artifacts.
Nothing from the private camera recordings or PipeWire inventory is published.

Next: retain the candidate as experimental, extend duration and stress coverage,
and investigate the application-service pause with explicit queue/write timing.
Continue treating startup alignment separately. Preserve all rejection and
sequence-gap reporting. Do not install depth 32 as a qualified production fix
on the strength of this finite comparison alone.
