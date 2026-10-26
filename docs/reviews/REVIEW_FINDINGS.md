# ScanCraft independent findings register

Maintainer: independent review agent exclusively. Development Astra should respond
in `docs/reviews/REVIEW_RESPONSES.md`; it must not edit statuses in this register.
Statuses: Open, Under Review, Resolved, Rejected. History is retained permanently.
Reviewer charter: [INDEPENDENT_REVIEW_AGENT.md](INDEPENDENT_REVIEW_AGENT.md).

## Baseline and evidence — 2026-10-26

Source baseline: `ba5ec7ca187fe9da6aa94e0dbc706e96c3ee15ae`, pending workflow PR #1.
Application source is identical to main `4125c96`; PR #1 adds review guidance and
changes the local check script. Audit documentation lives on `dev/protection-audit`,
PR #2. Application source and the developer response file were not modified.
`REVIEW_RESPONSES.md` did not exist at baseline; no developer responses are claimed.

Independent execution: `PATH=/home/dan/.cargo/bin:$PATH bash tools/check.sh`
completed with exit 0: rustfmt, strict all-target workspace Clippy, **497 tests
passed, 0 failed, 0 ignored**, and release workspace build. An initial invocation
failed because cargo was absent from shell PATH; the corrected invocation above
completed. Raw local log: `/tmp/scancraft-independent-baseline-check.log` (temporary,
not committed). No hardware, firmware, scanner controls or physical calibration
were exercised. Passing offline tests do not qualify real reconstruction.

Inventory: 11 crates, 174 Rust files under crates (including tests); 501 static
`#[test]` annotations, including feature-gated USB experiments. The executed count
is 497; annotation counts are not executed-test evidence. All crate manifests and
source inventory were inspected; selected high-risk implementations were read.
This is an initial architecture baseline, not a claim of exhaustive line-by-line
verification or an all-clear for every algorithm.

| Area | Inspected implementation / architectural observation | Remaining review |
| --- | --- | --- |
| Core/device | Frame validation, UVC metadata bounds, V4L2 poll/dequeue, mmap boundaries, session locks and capture lifecycle | Driver error injection, restoration failures and hardware reliability |
| Recording | Length/CRC framing, declared image cap, context defaults, indexed identity-guarded playback | Full parser fuzzing, mutation races and sustained memory bounds |
| Calibration | Brown model finite checks, bounded CRC storage, device-binding gate | Independent numerical sweeps; physical accuracy remains deferred |
| Image/depth | Borrowed GREY validation, bounded remaps, stereo provenance and explicit false timing/accuracy flags | Matching failure distributions, real exposure synchronization |
| Geometry | Rigid fitting finite/degeneracy checks, bounded tracking/fusion state, module inventory | Deep independent mesh, fitting and topology algorithm audit |
| Workflow | Shared no-overwrite Linux publication and source identity checks | Crash durability and fault injection across all publishers |
| GUI/CLI | Capture worker lifecycle, preview decoding, cancellation, adapter boundaries | End-to-end UI and large-data latency distributions |
| USB experiment | Separate feature-gated hardware path, control/restore structure | No hardware execution authorized; failed historical restoration remains unqualified |
| Dependencies | Cargo manifests: numerical nalgebra, serde/CRC persistence, Linux libc/V4L2, eframe/image UI, nusb experimental backend | No dependency is proven unnecessary; vulnerability/license audit not completed |

No new application-source correction is made by this reviewer. Positive design
observations include bounded preview queues, explicit calibration provenance,
CRC/size guarded formats, separate workflow adapters, and synthetic-versus-physical
qualification flags. These do not negate the open findings below.

## SCAN-REV-001 — Single-maintainer approval deadlock

- Severity: High
- Status: Resolved
- Affected: `docs/DEVELOPMENT_REVIEW.md:85`; live `main` protection's
  `required_pull_request_reviews.required_approving_review_count`; PRs #1 and #2.
- Problem: mandatory one-review approval cannot be satisfied for owner-authored
  PRs when the owner is the only collaborator and the developer uses that identity.
- Evidence/consequence: REST and GraphQL independently showed count 1, admin
  enforcement, only owner collaborator. PR #1 had passing CI and a resolved review
  thread but `REVIEW_REQUIRED`. CodeRabbit's actual review was COMMENTED.
  Documentation-only PR #2 independently reproduced the same approval block with
  both test runs passing at `b65cd61`.
- Corrective action: retain the PR requirement while setting only the approval
  count to 0; preserve all other protections and prohibit automatic merging.
- History: owner explicitly authorized the proposed one-field change with “yes”.
  Applied 2026-10-26 08:54 UTC. Full REST before/after objects differed only in that
  field. GraphQL retained `requiresApprovingReviews: true`, count 0, test/strict/
  conversation/admin enforcement and deletion/force-push protections. At `f7f14ae`,
  PR #2 had both tests successful and CLEAN with no review requirement; PR #1 was
  also CLEAN. Both remained OPEN/unmerged; auto-merge false. No merge was attempted.
  Shared-credential authority remains separately Open under SCAN-REV-002.

## SCAN-REV-002 — Shared owner credentials defeat technical merge separation

- Severity: High
- Status: Open
- Affected: `AGENTS.md:7`; `.github/PULL_REQUEST_TEMPLATE.md:9` on PR #1;
  `docs/DEVELOPMENT_REVIEW.md` owner authority section; runtime GitHub credentials.
- Problem: an instruction requiring owner consent is not an authorization boundary
  when Astra can call GitHub with the owner's ADMIN identity.
- Evidence/consequence: `gh api user` returned chevydan6; repository permissions
  and GraphQL viewer permission returned ADMIN. After required approvals become
  zero, an agent with these credentials can manually merge a green PR or change
  settings. Auto-merge disabled only prevents automatic merge scheduling. No merge
  probe was performed because it would change main without merge authorization.
- Recommended action: remove owner credentials/browser/connector access from the
  automation environment. Give a separate developer identity upstream read access
  and fork-only write access; reserve upstream merge/admin credentials for the
  owner outside the runtime. Alternatively use organization branch push/merge
  restrictions with a separate developer identity. Ordinary upstream write access
  and narrowed owner contents/PR write tokens do not alone enforce owner consent.
- Verification required: independently inspect the replacement identity's upstream
  permissions and applicable protections without attempting an unauthorized merge.
- History: identified and disclosed before owner authorized count 0. Owner accepted
  the one-field change; no credential separation has been performed or verified.

## SCAN-REV-003 — Successful CodeRabbit status can mean skipped review

- Severity: Medium
- Status: Open
- Affected: `.coderabbit.yaml:12` on PR #1; `docs/DEVELOPMENT_REVIEW.md:66`;
  GitHub CodeRabbit statuses on PRs #1 and #2.
- Problem: enabled auto-review and a green status do not establish an independent
  review of the current head. CodeRabbit explicitly skips this repository's
  automatic reviews because it has fewer than 10 stars.
- Evidence/consequence: PR #2 produced SUCCESS with description “Review skipped:
  manual review required for this OSS repository”. PR #1's completed COMMENTED
  review covered `3db7aa8`, not its later corrected head `ba5ec7c`. Unreviewed heads
  can look reviewed if owners rely only on the status badge.
- Recommended action: retain the App and auto-review config; verify completed
  review coverage/head and dispositions before owner merge. Use one manual App
  review request when needed and included capacity permits. Do not require the
  existing skip-success status as proof of review or buy capacity implicitly.
  A reliable machine gate would need to distinguish skipped/current/completed
  reviews rather than trusting this legacy status.
- History: integration verified from actual App feedback. No automatic-review
  service restriction was removed. YAML remains in unmerged PR #1.

## SCAN-REV-004 — Capture worker panic leaves GUI permanently active

- Severity: Medium
- Status: Open
- Affected: `crates/scancraft-app/src/main.rs:125`, `:267`, `:394`.
- Problem: receive handles only `Ok(result)` from the result channel. If the capture
  worker panics before `done.send(run())`, the sender disconnects without a result;
  running is never cleared and the worker is never joined by the receive path.
- Evidence/consequence: independently read the spawn/send/receive lifecycle.
  Start/interface controls are disabled while running is Some; Stop only sets the
  flag and cannot recover a terminated worker. The “Capture worker panicked” branch
  is reached only after receiving a result, so it does not cover a panic in run().
  A standalone Rust std-channel probe reproduced Disconnected after injected worker
  panic and confirmed the identical Ok-only transition leaves running=true. This
  verifies the channel mechanism, not a full GUI fault-injection test.
- Recommended action: distinguish Empty from Disconnected, detect/join finished
  workers without blocking, clear running and report unexpected termination while
  preserving partial recordings. Handle thread creation failure as well.
- Verification required: add a hardware-free application/controller regression that
  injects worker panic/disconnection and verifies stopped/error state, restart
  availability and preservation of incomplete recordings. Reviewer has not patched
  source. Temporary reproduction: `/tmp/scancraft-capture-disconnect-probe.rs`.

## SCAN-REV-005 — GUI shutdown can wait indefinitely for capture worker

- Severity: Medium
- Status: Open
- Affected: `crates/scancraft-app/src/main.rs:425`;
  `crates/scancraft-device/src/lib.rs:25`, `:395`;
  `ARCHITECTURE.md:14`.
- Problem: App::drop sets a flag then synchronously joins. Capture checks the flag
  between frames; device setup/control ioctls and close/restoration are synchronous
  with no established shutdown bound. An atomic flag cannot interrupt those calls.
- Evidence/consequence: direct inspection of Drop, worker loop, next_frame and
  ioctl wrapper. Architecture already acknowledges setup/control I/O is outside
  the approximate frame deadline. Closing the GUI may block on a stuck device;
  a hardware hang is not reproduced or asserted as observed here.
- Recommended action: define an explicit shutdown protocol and measured bound;
  investigate process isolation for uninterruptible driver calls rather than
  dropping threads that own restoration duties. Preserve cleanup/error evidence.
- Verification required: hardware-free blocked-worker lifecycle tests plus later
  owner-authorized disconnect/driver-stall qualification. Do not claim whole-app
  bounded cancellation from geometry-worker cancellation tests.

## SCAN-REV-006 — Physical reconstruction qualification remains unavailable

- Severity: High
- Status: Under Review
- Affected: `ACCEPTANCE_RESULTS.md:21`, `:24`;
  `crates/scancraft-depth/src/pipeline.rs:47`, `:166`;
  `crates/scancraft-calibration/src/storage.rs:59`.
- Problem: reliable calibrated live scanning is not established by the offline
  workspace. This is a release-readiness gap, not proof of a new numerical defect.
- Evidence/consequence: repository acceptance table records rejection proxy
  0.505717% versus a <0.1% target, post-soak asymmetry, and deferred calibration.
  Pipeline correctly emits synchronization_verified=false and
  physical_accuracy_qualified=false. Binding metadata validation does not establish
  optical calibration or simultaneous exposures. Historical private hardware
  reports were not independently re-run or published by this audit.
- Recommended action: keep experimental/unqualified labels and block accuracy or
  stable-release claims. Require traceable mode-specific ground truth, repeated
  dimensions/pose tests, synchronization evidence and reliability distributions.
  Respect the owner's calibration deferral; do not perform physical calibration
  without new authorization. Continue feasible synthetic/reference review.
- History: acknowledged limitation in current documentation; remains unqualified.

## SCAN-REV-007 — Generic cancellation contract lives in geometry crate

- Severity: Low
- Status: Open
- Affected: `crates/scancraft-image/src/lib.rs:6`,
  `crates/scancraft-geometry/src/work.rs:1`, `crates/scancraft-image/Cargo.toml`.
- Problem: generic Stage/WorkControl cancellation/progress requires image processing
  to depend on the broad geometry crate. This couples a lower-level image layer to
  mesh, editing and persistence implementations for an infrastructure concern.
- Evidence/consequence: inspected manifests and imports; calibration is independently
  separated, but image -> geometry dependency exists for work control. This expands
  compile/dependency surface and makes future reuse less direct. No dependency cycle
  or measured performance regression is claimed.
- Recommended action: consider a small shared work-control module/crate after
  higher-priority fixes; preserve API semantics and avoid a broad unsolicited rewrite.
- Verification required: dependency graph remains acyclic and all cancellation/
  progress tests pass after any developer-proposed extraction.

## Final documentation review availability

CodeRabbit CLI quota rechecked after the charter/register edits: remaining included
reviews 0, usage billing inactive, next included slot reported in approximately
15 minutes. No paid usage or repeat unchanged full review was requested. The
earlier completed audit review and its dispositions are recorded in
DEVELOPMENT_REVIEW.md. No final passing bot gate is claimed for this documentation
head; preserve it for owner/developer review.

## Concurrent development inspection

During finalization, the developer checkout changed AGENTS.md and spatial.rs;
these edits were not made, staged, reverted or committed by the reviewer. Reviewed
the subtree-minimum-index pruning change in `crates/scancraft-geometry/src/spatial.rs`
against the prior implementation. The shortcut applies only when all k distances
are exactly zero and the entire subtree cannot improve the worst original-index
tie; exclusion makes the stored minimum conservative. Public queries still check
cancellation before/after traversal. No actionable defect identified in this
focused inspection; this is not a general performance qualification.

Independently ran `cargo test --frozen -p scancraft-geometry spatial::tests`:
5 passed, 0 failed. Inspected-file SHA-256:
`d01420e1f50c71f46cbcfe891c99b31576dc80efc362597de2fb092b7d704f85`.
No developer response file existed when checked. These uncommitted edits are
outside the original 497-test baseline; they do not resolve any registered finding.

## Review continuation

On meaningful commits or PR updates, first read REVIEW_RESPONSES.md if present,
compare actual commits with this baseline, inspect claimed corrections and run
focused checks before changing status. Preserve all finding history. Do not repeat
unchanged full reviews or schedule polling implicitly. Current follow-up priorities:
SCAN-REV-002 technical authorization separation; SCAN-REV-004 panic recovery;
then device lifecycle and deeper numerical/mesh/parser reference testing.
