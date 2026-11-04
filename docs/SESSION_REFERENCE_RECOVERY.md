# Native session reference recovery

## Scope

Confirmed implementation: `crates/scancraft-geometry/src/session/recovery.rs`,
`ScanSession::process_with_reference_recovery`. It implements the bounded policy
studied in [the retained-reference experiment](RETAINED_REFERENCE_RECOVERY.md).
No new dependencies, optical coefficients, acceptance thresholds or world reset.
The [recorded workflow and GUI](RECORDED_REFERENCE_RECOVERY.md) now invoke this API
when explicitly enabled. The existing session
methods keep their default behavior; mixing them into a recovery session is refused.

## Contract and transactions

Start an empty point-to-point session and use the recovery method for each frame.
The caller supplies the actual surface cloud and a selector of current raw tracking
observations. The selector borrows session-owned prior positions and frame ID; it
cannot replace the saved tracker or inject a world transform. Camera coordinate
systems, units, synchronization and physical observation identity remain caller
responsibilities. Existing `_mm` configuration names do not qualify the units of
experimental recordings.

The local attempt runs first. On failure, every retained reference is tried with
unchanged Tracker gates. All passing poses must agree within `max_rms_mm` on the
union of their selected observations; otherwise no recovery is accepted. Choose
the oldest consistent passing reference. Retain the anchor and every eighth accepted
frame, including tracking-only frames, with at most 16 references; evict the oldest
non-anchor when full. These constants are experimental choices, not tuned optima.
Each attempt selects at most 512 finite observations; surface capacity is 100,000.
The selector runs at most 17 times. Its external side effects cannot be rolled back.

All candidate trackers and bank changes are staged before fusion. History capacity
is reserved before successful fusion commits. A returned error or cancellation
preserves semantic tracker, bank, counters, history and fusion state, including a
failed insertion/eviction. Ordinary unavailable tracking is different: it consumes a
strictly increasing frame ID and session capacity, returns every attempt and no
fabricated pose/history record. Valid rejected tracking updates retain their normal
history record. Callers must retain Outcome diagnostics; `history()` alone does not
represent unavailable attempts. Missing observations may leave `tracking_state()`
unchanged; the returned outcome is the authoritative result of the current attempt.

No accepted update means no surface fusion. Recovery maintains the selected
reference's composed world transform. It is local relocalization near saved views,
not global registration or evidence of motion through the lost interval. Conflicting
or insufficient candidates are never silently converted into successful frames.

## Validation

Five tests in `session/recovery/tests.rs` exercise recovery to the original world,
failed fusion, cancellation, a late selector failure after a passing trial,
contradictory passing poses, monotonic IDs/capacity, invalid selectors, tracking-only
cadence, bounded retention and insertion/eviction rollback. `tools/check.sh` passes:
930 tests, zero failures, one ignored; formatting, strict Clippy and release build.
An initial constructor-placement syntax error and a Clippy `is_multiple_of` lint
were corrected; original failed checks remain in private evidence.

A private adapter calls the native session API on the same 225 saved Raptor frames
used by the independently verified PR47 experiment. Its JSON equals the recovery
configuration of that experiment exactly: all 1,135 attempts, selections, diagnostics,
poses, bank histories, fusion counts and 5,884 final point positions. This is 44
accepted frames (31 integrated, 13 tracking-only), 181 unavailable, one recovery
at original frame 880 against reference 0, last accepted/integrated 896. Surface
input remains the same 277,704 genuine candidate observations; only 6,471 integrate.
There is no new capture or generated replacement geometry. Private evidence lives
under `session-reference-recovery-20261104`, outside the repository.

The prior experiment's independent NumPy checks provide a reference for equivalence;
this is developer validation of new code, not independent approval. Real data still
exercises only one passing recovery reference and a six-reference bank; contradictory
passes and eviction are covered synthetically. Frame 880 physical marker identities
remain unverified. No calibrated accuracy, complete object, live throughput or
sustained-memory claim follows. CodeRabbit usage lookup failed; no paid/substantive
review ran. Independent findings remain reviewer-owned.

## Recorded integration follow-up

Prepare stereo hypotheses once per recorded frame, select them against each borrowed
reference, retain all attempt diagnostics with bounded storage, and persist the
selected outcome without omitting failures. Extend reader/reopen validation and
native display, keeping the current disabled path reproducible. Measure real-object
identity, coverage and repeated dimensional accuracy separately. These steps were unfinished at the PR48 core milestone and are now addressed in
the linked recorded integration. Live operation and full measured scanning remain
unqualified; the core implementation is not an end-to-end scanning release.
