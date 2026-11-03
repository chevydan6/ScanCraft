# Coordinated live stereo source completion

Implementation commit: `01b183aa9c7d33563e11d7726b892bc0357f6e48`.

## Problem and implementation

SCAN-REV-027 independently confirmed that a live worker reaching EOF or its frame
limit could close its camera while its peer remained inside a read. The earlier
real-worker recording trial correlated 95 left rejected buffers with right stream
shutdown, and 89 right rejects with left shutdown during overflow. These are
retained observations, not proof that every transport rejection shares that cause.

`scancraft-workflow/src/live_stereo/runner/completion.rs` now allocates two unique,
non-cloneable participant slots before spawning source workers. Each owner marks
acquisition finished and waits for the other slot before explicitly closing its
source. A dropped unstarted/failed-open/unwinding slot requests cancellation and
retires itself without waiting. A two-bit predicate and condition variable retain
arrivals across wakeups and missing-worker paths. Slots live outside the worker's
recording call, so early source-open errors and outer unwinding retire them.

`runner/recording.rs::record_with_sink` sets the shared stop flag on acquisition
failure, reaches coordinated completion, closes its source, and only then joins
and syncs recording storage. Existing acquisition, cleanup and storage error
aggregation remains. Ordinary completion does not request peer cancellation.
A stop flag alone does not prove an in-flight read has returned: the rendezvous
still waits for both acquisition owners to retire before source closure.

`runner::Source::next_until` is an additive method with a default that checks
cancellation before calling the existing `next`. Existing implementations remain
source-compatible. Polling adapters should override it for cancellation during a
read. `scancraft-acquisition::Native` forwards the supplied predicate and public
GUI stop to `Capture::next_frame_until`, so peer failure and coordinator cancellation
reach existing cooperative driver boundaries. The worker no longer relies solely
on the GUI command flag. GUI stop remains distinct from failed capture; original
errors still prevent successful publication.

No dependency, recording schema, frame-validation threshold, calibration or
geometry algorithm changes. The finite CLI's separate completion guard is not
rewritten. Coordination does not preempt a synchronous driver call, filesystem
operation or join. A permanently blocked peer can still prevent completion;
SCAN-REV-005's hard cleanup deadline is not resolved. A partially constructed
source's internal cleanup remains its adapter's responsibility.

## Device-free validation

Six new regression tests cover:

- A completed participant waits for its peer even with cancellation requested.
- Dropping an unstarted slot releases/cancels its peer (the failed-spawn ownership
  path); this does not force an operating-system thread-creation failure.
- An unwinding worker retires its slot without detaching the waiting owner.
- Actual recording workers coordinate EOF, frame-limit completion, graceful GUI
  stop, read error and read panic while the peer is deliberately held in a read.
  No close occurs before release; both recordings replay to complete footers.
- Actual source-open error/panic paths retire their slot and release the peer.
- An in-progress adapter read receives the internal failure flag and public stop
  independently through the runner's callback, without falling back to `next`.

Existing blocked-writer, overflow-prefix, combined-error, sync-failure, bundle
publication/restoration/cancellation and legacy source tests also pass. The full
`tools/check.sh` gate passes **875 tests**, one existing ignored test, formatting,
strict all-target Clippy and optimized workspace builds. Initial test-only failures
are retained privately: a diagnostic tried to format a non-Debug report, and a new
synthetic maximum-frame fixture lacked required UVC metadata. The assertion and
fixture were corrected; production validation was not relaxed.

## Controlled Raptor comparison

Private evidence directory `live-completion-20261104` contains the fixed plan,
exact old/current recording source, completion source, Cargo-selected optimized
libraries, probe source/binary hashes, ioctl traces, original recordings, replay
results, source binding, analysis script and gate logs. The old worker is taken
from `0ef103999138f7678bfe0319dc7cdeffd9c33cff`; new source hashes are in
`native-build.json`. The common native adapter uses actual `Capture`, session
ownership, scoped flood and settings restoration. It records device statistics.
A private Control shim exposes progress counters and fixes public stop false;
actual Control command behavior is tested in the device-free regressions.
The optional processing channel is deliberately not drained. This is the real
recording/completion worker with a native adapter, not the complete GUI or a
calibrated reconstruction run.

Fixed order: corrected / old / old / corrected, 300 frames per camera. In these
four trials only, the right adapter delays its final read until left has accepted
300 frames, then waits 100 ms; the count wait has a five-second failure limit.
This deliberately exposes uneven completion. It is not an ordinary throughput
benchmark. A fifth corrected-worker trial injects one 600 ms pause at the left
recording's 200th large write, without the final-read intervention.

| Trial | Saved L/R | Startup rejects L/R | Ongoing rejects L/R | Video dequeues after peer STREAMOFF L/R |
| --- | ---: | ---: | ---: | ---: |
| Corrected first | 300/300 | 4/0 | 2/2 | 0/0 |
| Old first | 300/300 | 1/1 | 2/2 | 0/1 |
| Old second | 300/300 | 1/0 | 3/3 | 0/1 |
| Corrected second | 300/300 | 2/0 | 2/2 | 0/0 |
| Corrected overflow (expected failure) | 208/209 | 0/0 | 0/0 | 0/0 |

All **2,817 saved frames in ten recordings** pass full native replay with integrity
checks. Normal runs succeed; overflow returns explicit queue-full failure. It
captures 209/209 frames, but the unadmitted left frame is excluded from recording.
The right camera reports one cooperatively stopped incomplete video buffer, kept
separate from rejected and saved frames. Nine complete left video dequeues occur
on another thread during the delayed write. All four STREAMOFF calls finish before
that write ends; neither source dequeues during or after its peer's STREAMOFF.

Each old run demonstrably performs one right video dequeue after left STREAMOFF;
the returned frame remains usable. Thus this controlled comparison proves
the lifecycle ordering difference, **not elimination of a reproduced rejection
storm or an improved rejection rate**. The earlier 95/89 shutdown-associated
rejection bursts remain separate evidence. The two corrected normal runs still
have ongoing rejections and sequence gaps; acquisition is not qualified lossless.

Trace row counts, including status footer: 4,895 / 4,883 / 4,889 / 4,883 / 3,392.
All bounded trace footers report zero errors and exact row counts. Analysis checks
successful video dequeue intervals against peer video STREAMOFF start and end,
sequence-gap accounting, replay counts and required-write concurrency. All seven
native snapshot groups match after each trial and final restoration; all five idle
formats/intervals restore, WirePlumber is active, and baseline module parameters
and srcversion remain unchanged. No USB reset, firmware write or driver replacement.

## Review and remaining qualification

Developer accepts SCAN-REV-027 and submits this correction for independent review;
only the independent reviewer can verify and close it. CodeRabbit quota/billing
lookup returned `lookup_failed`; no new paid or completed automated review is
claimed. SCAN-REV-005 cleanup bounds, SCAN-REV-006 physical qualification and
SCAN-REV-021 live capacity remain unresolved. This fix does not extend the
1,000-frame live limit, establish camera synchronization, provide physical scale,
or deliver the first independently measured object scan. Next priorities remain
real calibrated reconstruction and the remaining acquisition/capacity defects.
