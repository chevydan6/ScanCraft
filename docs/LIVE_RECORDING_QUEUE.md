# Bounded recording in the live stereo worker

This report preserves the initial bounded-writer milestone. Its disclosed peer
shutdown defect is addressed by the subsequent [completion correction](LIVE_SOURCE_COMPLETION.md);
the original measurements below are unchanged.

Implementation commit: `e3db8f9e16ac349e29560757d124a68a7b8182e7`.

## Problem and correction

The native live IR path still called `Writer::push` directly on the camera thread,
although the finite CLI path had already demonstrated susceptibility to storage
stalls and adopted a bounded writer. The live worker also finished and synced
its recording before explicitly closing its source. This integration applies the
existing recorder to that live path without changing image processing, geometry,
recording formats or calibration checks.

`live_stereo/runner/recording.rs::record_controlled` owns the source on its original
thread and sends required raw frames to `QueuedWriter` at capacity eight. The
recording crate's new `push_copy` uses the same admission function as owned `push`;
it validates existing size/frame constraints before cloning. Processing retains
its own frame, and the unchanged four-event processing channel remains optional.
A full required recording queue aborts capture explicitly. A processing-channel
drop retains the admitted raw frame and its existing separate counter.

The acquisition unwind boundary leaves writer ownership outside the closure.
Acquisition failure/panic sets peer cancellation before source close; cleanup
failure also sets cancellation before draining storage. Recording drain, footer,
flush and sync must finish successfully before a source report can succeed.
Acquisition, cleanup and writer errors are combined rather than replacing one
another. The existing bundle still joins source workers, restores settings,
replays raw files, verifies artifacts and publishes without overwriting a target.
A finalized prefix alone never establishes successful capture.

The public `Source` and bundle APIs remain unchanged. Successful report metadata
adds `recording_queue_capacity_per_camera: 8` and
`recording_completion_verified: true`; existing reports remain readable. Live
progress counts admitted frames, not per-frame durability. No dependency or
recording schema changes are introduced.

## Bounds and limitations

Eight queued 1920 × 1200 GREY payloads consume 18,432,000 bytes per camera, plus
metadata. The writer also owns its active frame and serialization buffer; the
caller, processing channel, pending association and preview retain their own
bounded frames. This is a payload calculation, not a measured whole-app RSS bound.
Copying retained frames costs memory bandwidth and allocations. No benchmarked
application throughput improvement is claimed.

File/header creation, driver calls, joining storage, syncing and source cleanup
can still block. This does not establish a hard cleanup deadline or resolve
SCAN-REV-005. The separate normal peer-completion policy and 1,000-frame live
capacity remain unresolved; SCAN-REV-021 is not closed. The laser runner is a
separate path. These limits remain visible instead of being silently removed.

## Device-free verification

Five new tests run the actual private recording helper with controlled source and
sink seams. A channel handshake holds the first frame write; no timing sleep is
used to infer progress. They verify:

- Four frames are acquired and the source closes while storage remains blocked;
  recording completes only after release, and output equals the original writer.
- With one frame blocked and eight queued, the next admission fails; cancellation
  is visible at close, and exactly the nine admitted frames form the replayable
  prefix. The overflow cannot finish successfully.
- A source panic closes the source before joining the blocked recorder.
- Read, source-close and background-write errors all survive in the result.
- A failed final sync cannot report success after otherwise normal acquisition.

A recording-crate regression checks borrowed/owned byte equality, unchanged input,
invalid payload/metadata rejection and poisoned completion. Existing live bundle
success, failure, restoration, cancellation, artifact and blank-scan tests pass.
The complete Rust gate passes **869 tests**, one existing ignored test, formatting,
strict all-target Clippy and optimized workspace builds.

The repeated-PTS replay regression now uses five frames advancing by 0x40000000
instead of nine advancing by 0x60000000. It still completes a modulo cycle and
rejects a repeated PTS hidden by processing drops. The shorter burst fits the
recording queue independently of background scheduling; replay validation is
unchanged. Source files are `scancraft-recording/src/queued.rs` and its tests,
`scancraft-workflow/src/live_stereo/runner.rs`, and its new recording module/tests.

## Real Raptor worker validation

The private experiment `live-queued-recording-s9me7fm3/hardware-v2` compiles the
**exact recording worker source** against the final optimized libraries. It uses
native `Capture` and scoped flood/settings restoration. A private control shim
fixes `stop_requested=false` and exposes the same two progress counters, because
the application's counters are crate-private. The source adapter records device
statistics. Processing delivery is intentionally not consumed: its four-slot
channel fills, while required raw recording must continue. This tests the real
recording worker, not a calibrated processor, complete GUI, or user-stop command.

The installed baseline driver stays fixed. A bounded 20,000-row tracer injects
one pause at the left recording's 200th large write, without synchronous trace-file
writes during acquisition. The three fixed cases request 300 frames/camera:

| Trial | Native result | Saved frames L/R | Startup rejects L/R | Ongoing rejects L/R | Sequence gaps L/R |
| --- | --- | ---: | ---: | ---: | ---: |
| Ordinary | Success | 300/300 | 98/0 | 96/1 | 96/1 |
| One 200 ms write stall | Success | 300/300 | 0/1 | 2/2 | 2/2 |
| One 600 ms write stall | Explicit queue overflow | 208/210 | 0/0 | 0/89 | 0/89 |

All **1,618 saved frames in six recordings** pass complete native replay. No bad
frames are accepted to improve the counters. The 200 ms write interval contains
six complete left video dequeues on another thread. The 600 ms interval contains
nine; all four successful STREAMOFFs complete before that delayed write ends.
The left source captured 209 frames but admitted only 208 before overflow; the
unadmitted frame is not counted as saved. The entire capture remains failed,
although its finalized prefixes are readable. This private probe does not publish
a scan bundle. The synthetic bundle regressions separately check failed
publication and restoration behavior.

Trace rows total 6,017 / 4,877 / 3,931, including one footer per trial; each footer
has zero trace errors and matches the retained row count. Full raw replay,
sequence accounting and settings restoration are checked by `analyze_hardware.py`.
All seven native state groups match after each trial and at final restoration;
five idle formats/intervals restore, WirePlumber is active and original module
parameters remain unchanged. An exported accepted image shows the board, still
clipped on the right. No new calibration or physical-accuracy evidence follows.

**Remaining lifecycle defect:** in the ordinary run, 95 of the left camera's 96
ongoing bad buffers occur after successful right video STREAMOFF. In the overflow
run, all 89 right bad buffers occur after left video STREAMOFF. The live path
still lacks the finite CLI's normal peer-completion barrier, and an in-progress
source read may encounter a peer shutdown before observing cancellation.
`peer-close-correlation.json` retains exact intervals. This supports prioritizing
lifecycle integration; it is not a claim that every rejection has that cause.
Neither the ordinary run nor this bounded writer qualifies lossless scanning.

Preflight failures remain recorded: an ambiguous dependency-file glob was replaced
with Cargo's exact fresh artifact metadata, and private control-field access was
replaced with the explicitly described probe shim. The first hardware harness
created a root-owned output directory; the tracer constructor exited 74 before
entering the capture program. Unprivileged creation independently reproduced
EACCES, and settings were restored. A new retained run uses an owner-writable
directory and passes a tracer-only preflight. These failed attempts are not
included as camera timing evidence. No firmware, reset or module changes occurred.

CodeRabbit's current quota/billing lookup returns `lookup_failed`; no completed
new review or paid request is claimed. Independent review remains pending. The
next task is coordinated live-source completion and cancellation, followed by
another real-worker trial. The first independently measured object and complete
native scanning qualification remain outstanding.
