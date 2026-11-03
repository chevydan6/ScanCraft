# Bounded finite-capture recording — 2026-11-03

Source `2e3e2d8639b091a0001f803c64d1384dd1df2a15` moves finite stereo recording
writes off the camera acquisition threads. Controlled real-camera trials show
that an injected 200 ms storage pause no longer causes the additional sequence
loss seen with the preceding synchronous writer. A longer pause explicitly fails
capture when the bounded queue fills; it is not converted into silent dropping
or a successful scan. Ordinary USB rejections and startup corruption remain.

## Evidence motivating the change

The preceding [queue-depth experiment](RAPTOR_QUEUE_DEPTH_TRIAL.md) observed a
153.1 ms application-service pause before one sequence gap. It did not identify
the pause's cause. New observational tracing at baseline source
`23d36d7884e82854ef9256a702278e31caea088d` records write, poll, REQBUFS, QBUF and
DQBUF timings, without logging image contents. The driver grants four video
and four metadata buffers to each camera.

A natural 1,800-frame-per-camera trial has a maximum traced write duration of
6.470 ms. Ongoing rejected buffers and sequence gaps are both 14/14, left/right.
This trial **does not reproduce or explain** the previous long pause.

A separate fault-injection run delays the left recording's 200th large write
by 200 ms. The synchronous writer makes no left-camera dequeue calls during that
write, then loses two additional sequences beyond its rejected-buffer count.
This establishes susceptibility to storage stalls; it is not evidence that the
earlier natural pause was caused by storage.

The first diagnostic build failed because of an inherited libc `poll` access
annotation. An unintended untraced capture completed and replayed, but failed
the harness's trace-presence check. Its raw files, loader messages and original
diagnostic source are retained and excluded from timing evidence. The corrected
interposer uses a separate C identifier exporting the same `poll` symbol; it
passes compilation with warnings as errors and an independent trace-plumbing
self-test. Subsequent runs check that the library exists before capture.

## Implementation and invariants

`crates/scancraft-recording/src/queued.rs::QueuedWriter` owns a bounded standard
library channel and one recording thread. It calls the existing `Writer` for
frame validation, order checks, serialization, CRCs and the footer. No recording
version, raw pixel data, metadata interpretation or file layout changes.

- `new` accepts an explicit capacity of 1–32 frames. The finite CLI selects eight
  frames per camera. Ownership transfers into the queue; payloads are not cloned
  merely to enqueue them.
- `push` checks existing frame-size limits before metadata validation and uses
  `try_send`. It never waits for storage. Full or disconnected queues fail
  admission; any admission error poisons successful completion.
- Disconnected admission does not join a worker whose failing sink destructor
  might still be blocked. `finish` collects the underlying worker failure.
- `finish` and Drop close admission, drain admitted frames and join the worker.
  Write, flush and worker-panic failures cannot become successful completion.
  Joining can wait for filesystem operations; no hard deadline is promised.
- At most the configured number of queued frames plus one worker-owned frame
  are retained by the worker abstraction. The existing Writer also allocates a
  serialization buffer, and the caller owns its current frame. At this capture
  profile, eight queued image payloads total 18,432,000 bytes per camera, plus
  metadata. This is not a bound on total process or kernel memory.

`crates/scancraft-cli/src/stereo_probe.rs::capture` uses this worker for finite
recording commands, including flood, gain and line trials. The camera is still
owned by its original acquisition thread, and the existing two-camera completion
barrier remains. Samples are admitted to the report only after queue admission.
`raw_recordings.recording_queue_capacity_per_camera` records the selected capacity.

`stereo_probe/recording.rs::complete` cancels the peer on acquisition/cleanup
failure, closes the camera **before** draining recordings, and preserves errors
from all three stages. Declaration order and a borrowing finish closure also
keep camera destruction ahead of recording-worker joining during unwinding.
Successful captures still require normal recording completion, file sync, full
CRC/footer replay, control restoration and no-overwrite publication.

The original integration above applies to finite CLI recording. The subsequent
[live IR integration](LIVE_RECORDING_QUEUE.md) reuses this worker for the native
live-stereo path. The laser runner and other paths require their own capacity,
cancellation, storage-budget and publication checks; coverage is not universal.
`push_copy` now offers the same validated admission for callers retaining a frame
for processing, checking limits before making the owned copy.

## Controlled real-camera comparison

All runs use the original installed driver, unchanged module parameters,
1920 × 1200 GREY, requested 30 FPS and temporary flood illumination. No kernel
swap, service interruption, firmware change or validation relaxation occurs in
this milestone. Exact old/new binary copies are retained. The comparison order
is queued / synchronous / synchronous / queued, 300 accepted frames per camera,
with the same explicit one-off 200 ms left recording-write delay in every run.

| Trial | Writer | Ongoing rejects L/R | Sequence gaps L/R | Left dequeues during delayed write |
| --- | --- | ---: | ---: | ---: |
| queued-injected-1 | Queued | 3/3 | 3/3 | 6 |
| baseline-injected-2 | Synchronous | 1/1 | 3/1 | 0 |
| baseline-injected-3 | Synchronous | 2/2 | 4/2 | 0 |
| queued-injected-2 | Queued | 2/2 | 2/2 | 6 |

The delayed writes last 200.433–200.641 ms. In both queued trials, six left-camera
buffers are dequeued by a different thread while the recording thread is held.
Both synchronous trials have two additional left sequence gaps. All 2,400
accepted buffers in these eight recordings pass complete native replay; trace
footers report zero logging errors and all format/rate/illumination readbacks
match. Startup and ongoing rejections remain reported, not filtered away.

A following unmodified-timing queued capture retains 1,800 frames per camera,
with ongoing rejects and sequence gaps both 12/12. Both recordings replay and
settings restore. The maximum traced write is 3.197 ms. The difference from the
earlier natural trial is not presented as a proven USB or throughput improvement.

Process maximum RSS in the matched injected trials is 44,568/44,744 KiB for the
queued writer versus 31,356/31,272 KiB for synchronous recording. The retained
backlog costs memory. The natural queued run uses 31,432 KiB versus 31,144 KiB
in the preceding natural baseline. These are individual `/usr/bin/time -v`
observations with tracing, not general memory or performance guarantees.

## Failure and cancellation checks on the Raptor

A separate 600 ms injected write delay exceeds the queue's capacity. Capture
returns an explicit queue-full error. All four video/metadata STREAMOFF calls
finish before the delayed write finishes, so draining storage does not defer
normal camera stop in this trial. Complete retained prefixes contain 208 left
and 212 right frames; both pass native replay. Neither the requested successful
directory nor a success manifest is published. Saved settings restore.

A separate SIGINT after 100 left frames returns cancellation, retaining
replayable 100/101-frame prefixes and restoring settings. It also publishes no
successful directory or manifest. Prefix footer integrity is not proof of a
successful capture, fsync durability or an unconditional cleanup deadline.

## Automated checks, provenance and limitations

The full `tools/check.sh` gate passes formatting, strict all-target Clippy,
**858 tests**, one existing ignored test, and optimized workspace builds.
Six recording-worker regressions cover exact byte compatibility, bounded stalled
admission, overflow poisoning, Drop joining, I/O/flush/panic errors, slow failing
destructors, invalid inputs and existing ordering validation. Two CLI cleanup
regressions verify cancellation/order/error aggregation and successful-completion
requirements. These are synthetic tests; the Raptor trials above are separate.

The worker is original Rust code using standard library threads/channels and the
existing MIT recording implementation. No dependency or external algorithm was
added. Private artifact `recording-stall-a0_zqzay` retains exact source hashes,
the staged implementation patch, old/new binaries, diagnostic source, all logs,
raw recordings, readbacks, resource measurements and fault-injection parameters.
`analyze_stalls.py` reproduces `stall-analysis.json`; failure results are separate.
The new CLI SHA-256 is
`68335a91b77ec88f3b5cd32aa5c0d8ab8b46b1a259e9929ab467fdfe5a559b07`.
Private camera data and device-specific values are not published.

CodeRabbit's repository-scoped quota/billing/credit lookup remains unavailable;
no completed CLI review or paid request is claimed. Independent review remains
outstanding. SCAN-REV-005 hard cleanup deadlines, SCAN-REV-006 physical scanning
qualification, and SCAN-REV-021 live IR capacity are not closed by this work.
Startup framing, sustained acquisition, calibration scale, the first measured
object surface and complete GUI/project/export qualification remain open.
