# Recorded scan recovery integration

## Running and reopening

The existing recorded workflow now supports the experimental session-owned
recovery policy. In an otherwise valid `fuse-recorded-candidates` request, enable:

```json
{
  "temporal_marker_support": true,
  "retained_reference_recovery": true
}
```

This is a fragment, not a complete request. Preserve the existing explicit marker,
tracking, voxel, capacity and source-bundle settings. Run
`scancraft-cli fuse-recorded-candidates request.json new-output`, then open the
result through **Open fused scan** in the native GUI. The GUI shows all fused
candidates and the selected original camera pair; outcome rows include recovery
attempt counts, recovered reference IDs and conflicting-reference refusals.

The new option defaults false and requires temporal support. Existing request and
result bytes are unchanged when it is absent. Older applications that reject new
fields/artifacts cannot open opted-in recovery results. The current reader opens
both layouts; recovery does not enable metric editing or imply verified units.

## Implementation and data ownership

`tracked_candidates/markers.rs::prepare` computes image components, shape fits and
all Cartesian stereo hypotheses once per frame. Stereo budget is charged once.
`Prepared::select` borrows those original hypotheses and uses the same nearest-prior
support and mutual-uniqueness rules for each trial. No new geometry, threshold,
optical model, feature identity, motion prediction or fitted physical scale enters
selection. Recovery may perform up to 17 selection/tracking passes per frame; this
additional work is not hidden inside the stereo-attempt count.

`tracked_candidates/recovery.rs::process` connects selection to
`ScanSession::process_with_reference_recovery`. The geometry core owns references,
consensus and atomic tracking/fusion history. The workflow owns file publication
and diagnostics. A write/cancellation error aborts the run, retaining an explicitly
incomplete staging directory without publishing the final scan. The session is
local to that failed run and discarded; output writes are not claimed to be a
rollback-capable external callback. Existing no-overwrite publication and source
checks remain intact.

## Saved evidence and resource limits

The six original artifacts remain. Opted-in output adds checksummed
`recovery.ndjson`, one row per input frame, with every local/retained attempt:
original selected coordinates, prior frame ID, retained/local flag and full tracking
result or error. Each row also records selection, pose disagreement, recovery source
and reference-bank history. `frames.ndjson` carries the selected summary; unavailable
frames retain no invented pose. Tracking errors with at least three observations
have explicit `tracking_error` status and reason, counted under unavailable frames.

`markers.ndjson` preserves the original component and Cartesian stereo evidence,
plus attempt boundaries and every support/uniqueness decision. It is streamed with
an enforced 1 GiB cap; overflow fails with retained partial diagnostics, never drops
rows to make publication succeed. Recovery journal lines are at most 1 MiB, 256 rows,
with typed parse bounds of 17 attempts, 512 observations per attempt and 16 retained
IDs. The producer checks its row size before writing. Existing source/frame/fused
artifact quotas remain. These are resource ceilings, not throughput benchmarks.

The fused reader's leftover 4,096 surface/observation-count limits now use the shared
8,192-query ceiling, including aggregate surface totals and source snapshots. A full
8,192-point fixture round trip covers this compatibility correction.

## Reopening validation

`tracked_candidates/bundle/recovery.rs` validates journal/frame order, complete local
and retained trial enumeration, prior IDs, monotonic accepted reference history,
cadence/eviction, bounded finite positions, pose validity and tracking diagnostics.
It recomputes disagreement between passing poses, checks oldest-consistent selection,
and requires selected tracking fields to match the frame outcome. Artifact lengths
and CRCs and every original source binding are also checked before returning data.
The GUI consumes this validated summary through the existing background loader.

Reopening does not re-run optical correspondence selection, ICP or voxel fusion.
It checks recorded evidence for consistency; CRCs are integrity checks, not signatures
or proof of physical identity. A coherent malicious rewrite is not excluded by
these checks. Units and physical-accuracy qualification flags remain false.

## Validation and remaining scope

The full Rust gate passes 935 tests, zero failures and one ignored, with formatting,
strict all-target Clippy and optimized workspace build. Five added regressions cover
image-to-recovery-to-reopen, missing/altered journal trials and selected outcomes,
diagnostic byte limits, the dense source ceiling and fusion failure after an accepted
frame without final publication. Prior core tests cover conflicting poses, rollback,
reference eviction and cadence. Original Clippy failure from an unnecessary explicit
borrow-wrapper drop was corrected and its log retained.

The final recorded workflow replays the unchanged 225 genuine Raptor pair bundles.
All 1,135 attempts, selected poses, graph decisions, bank histories and 5,884 fused
positions exactly match the prior session-core result independently verified in
reviewer assignment564. All 71,770 stereo attempts remain; support-distance comparison
also matches exactly for all 5,700 emitted support rows. Results are 44 accepted
(31 integrated, 13 tracking-only), 181 unavailable, with one recovery at original880
against reference0 and last accepted896. No intermediate geometry was invented.
All 28 files from four existing disabled/original recorded cases remain byte-identical.

The result reopened in the native GUI with all 5,884 candidates, 225 source frames
and original cameras at the recovery frame. The geometry remains sparse fragments.
The initial render invocation found no Xvfb at its default cache path; using the
existing explicit test-tool path resolved this without installation or scanner access.
Evidence and original failures remain private under `recorded-reference-recovery-20261104`.
CodeRabbit usage lookup failed; no substantive automated review or paid usage claimed.
Independent review of this integration is still required.

This completes a recorded recovery connection, not live recovery, global registration,
a complete box scan or calibrated accuracy. Physical identities at first recovery
remain unverified. Next priority is real object coverage and independently measured
units/accuracy, with repeated trials and acquisition reliability still outstanding.
