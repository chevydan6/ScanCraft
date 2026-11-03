# Deterministic pacing for the synthetic capacity fixture

SCAN-REV-031 identifies a test precondition error: a consumed processing preview
acknowledges neither independent recording worker. At a4d2de1, push CI41702132020
fails the1,001-pair success test with recording-queue overflow while PR41702142669
passes. The production eight-frame queue correctly fails closed; that contract is
unchanged. This correction changes only integration-test code and documentation.

## Progress condition and bound

`tests/live_capacity.rs::Paced::wait_for_progress` now requires both the processing
acknowledgment and `support/recording_progress.rs::RecordingProgress::ready`.
The latter observes both real recording files inside the fixture's unique staging
directory. Missing files mean no progress; ambiguous staging directories and other
I/O errors fail explicitly. It does not write or flush the recording files.

The helper derives header and fixed frame lengths by running the existing
`scancraft_recording::Writer` into a byte-counting test sink. These synthetic frames
have fixed metadata/pixel lengths. The current record stride is2,096 bytes. Both
production file sinks use `BufWriter::new`; the helper obtains that default capacity
rather than hard-coding a sleep or requiring every record to be flushed separately.
It permits `ceil(buffer_capacity / record_stride) + 1` outstanding frames, currently
five, and asserts that this stays below the unchanged eight-frame recording queue.
The end-to-end result also explicitly checks that queue capacity remains eight.

If `E` frames have been emitted, admitting the next requires file length at least
`header + (E + 1 - window) * stride` on **both** sides whenever `E >= window`.
Observed complete-record byte lengths imply the writer has consumed those frames;
there can then be at most `window` admitted but unobserved frames after admission.
A stalled consumer cannot fill eight queue slots. The buffered-output allowance
also lets a healthy writer produce enough bytes to trigger its normal flush.

This is a fixture-specific progress bound, not proof of durability, CRC validity,
physical throughput or hardware accuracy. Final recording replay, CRC/metadata
references, journal rollover,1,001-pair capacity, edit/undo/redo, save/reopen, export
and recovery assertions remain in the existing test. A change to fixture framing,
file buffering or queue bounds must revisit the pacing assumptions; unsupported
small-frame/buffer combinations fail the bound assertion rather than enlarging the
production queue.

## Controlled regressions

`support/recording_progress_tests.rs::both_delayed_recording_consumers_bound_the_synthetic_source`
uses the actual `QueuedWriter` with two file-backed buffered sinks. Channels hold
both writer threads after header construction. Processing is deliberately marked
ready to isolate storage pacing. Both sources reach the five-frame bound and stop.
Releasing only the left sink still permits neither source to advance. Releasing
both lets all32 frames per side complete; both recordings replay through their
footers. Channel state and recorded progress determine success, not a presumed
sufficient sleep. A release-on-drop guard and ten-second held-write deadlines
prevent ordinary test failure from leaving those deliberately held workers stuck.

`recording_wait_preserves_cancellation_and_deadline` checks cancellation and an
expired deadline while recording progress is absent. The fixture keeps its existing
ten-second per-wait deadline and100µs polling interval; no longer sleeps, increased
queue capacities or retry-until-green behavior are introduced. These are cooperative
checks around local test I/O, not hard bounds for arbitrary blocked filesystems.

Validation: full `tools/check.sh` passes **892 tests,0failed,1ignored**, formatting,
strict Clippy and release build. All six capacity tests also pass on CPU0 with six
test threads. The full gate includes the unchanged recording-overflow, cancellation
and retained-prefix tests. Source changes are confined to `tests/live_capacity.rs`
and its two support modules. No application, recording implementation, API, data
format, scanner setting or firmware changes are included.

An isolated source copy removes only the recording-progress conjunct from the
new fixture, restoring preview-only admission. With a separate Cargo target,
the controlled-stall test fails with recording-queue overflow on both consumers
and exits101 after its10.01-second bounded failure path. The corrected live
worktree is unchanged by that negative check. This verifies that the new regression
detects the missing synchronization; it does not reproduce the unknown CI I/O delay.

The original mixed CI remains recorded; a later pass is not a retrospective fix.
New publication CI and independent verification of031 are separate requirements.
Physical acquisition/reconstruction qualification remains outstanding.
