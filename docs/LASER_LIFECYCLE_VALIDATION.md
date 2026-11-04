# Laser acquisition lifetime and cleanup corrections

Baseline: `973fbc1eeeceb0bc18bc6a716941e7213460160e` (PR52). These changes
address two source-level lifetime problems. They do not qualify acquisition,
camera synchronization, pixel integrity, physical units or dimensional accuracy.

## Coordinated camera retirement

`laser_stream/runner.rs::produce` previously closed a finished source while its
peer could still be acquiring. Commit `0bb9f63` reuses the existing two-participant
guard in `live_stereo/runner/completion.rs`. Slots exist before spawning workers;
an unstarted, failed or unwinding worker retires its slot through Drop. A read,
journal or statistics failure requests peer cancellation before waiting. Stop
requests do not imply that an in-flight driver call has returned.

`laser_stream/runner/retirement_tests.rs` holds the peer inside a read and checks
closure ordering after observing the first participant's arrival. Nine cases
cover EOF, read error/panic, graceful stop, frame quota, WorkControl cancellation,
journal push/finalization failure and statistics panic. The retained pre-fix
regression fails on premature close; the corrected regression passes. The
942-test workspace gate, formatting, strict Clippy and release build pass.

This change introduces no new synchronization algorithm. It does not establish a
hard cleanup deadline: a blocked driver read can still delay retirement.

## SCAN-REV-035: gain restoration after format retries

**Accepted reviewer finding; independent verification remains pending.** A failed
format restoration could be retried after gain had already been restored and
marked complete. The retry could reset gain, while cached gain restoration and
Drop skipped the necessary rewrite. Previous hardware trials began with the
default gains and did not exercise this failure path.

`scancraft-acquisition/src/laser/cleanup.rs::Settings` now owns stream settings
and optional gain together. Every unsuccessful cleanup attempt can be retried;
gain is reapplied and read back after stream restoration even if it previously
succeeded. Finalization consumes the stream owner before restoring gain.
`SettingsLease::finish` reports its final attempt and prevents a later destructor
from changing formats. The coordinator also runs this ordering on early return
and unwind. Both stream and gain errors are retained. Successful callback cleanup
does not introduce additional device writes after bundle publication.

`ScopedIrGain::capture` validates and snapshots both controls without mutation;
`apply_requested` runs only after the coordinator owns both leases. This covers
partial gain-application failure as well as illumination/startup/capture failure.
The existing `ScopedIrGain::apply` interface remains available to other callers.
`restore_after_settings` invalidates both cached gain successes, then uses the
existing per-camera restore/readback loop. Failure on one camera does not skip
the other. Source profiles, gain bounds, request schemas and recording formats
are unchanged. No dependencies or imported algorithms are added.

Five coordinator tests use originals 8000/9000 and a mocked format operation
that resets both to 1000. They cover first failure followed by successful retry,
repeated failures, explicit finish, Drop/unwind, either gain error, preservation
of earlier errors, omitted gain and no additional writes after success. A sixth
device test exercises the actual gain invalidation/restoration loop with either
camera failing. These are deterministic device-free tests, not physical
restoration-failure injection. Existing application rollback tests still pass.

Combined final gate: **948 passed, zero failed, one ignored**, formatting,
strict all-target Clippy and optimized workspace build. The first gate attempt
stopped on a test-only needless-range-loop lint; it was corrected and the entire
gate rerun. Private evidence label: `gain-cleanup-retry-20261105`.

## Hardware comparison and remaining limits

A four-arm ABBA native-helper comparison was prepared with identical 16-frame
requests, frozen geometry and gain settings, source/binary hashes and seven
post-capture state comparisons before external safeguards. Its preflight failed:
the previously observed USB device was absent, and a fresh sysfs inventory found
no Creality USB device. **No acquisition, illumination, gain, format or service
change occurred.** The baseline/current comparison remains unperformed. The
current build contains both lifecycle corrections; a future comparison must not
attribute differences exclusively to one of them without further isolation.

The prior helper's 111 right-side tail rejections remain an unproven retirement
hypothesis; actual STREAMOFF timing was not recorded. These changes do not explain
the two zero-frame GUI startup failures or resolve SCAN-REV-034's accepted-image
corruption. No complete measured box, moving-object reconstruction or qualified
mesh is claimed. The owner's box dimensions remain held out from fitting.

CodeRabbit's included-capacity, billing and credit lookups return `lookup_failed`.
No substantive review or paid usage was initiated. Keep independent review and
owner merge requirements; developer tests do not close reviewer-owned findings.
