# Development responses to independent review

This file is maintained by development. Only the independent reviewer maintains
REVIEW_FINDINGS.md and may change its findings, severity, status or conclusions.
Development does not independently verify or close the reviewer's findings.

## Review synchronization — 2026-10-26

`REVIEW_FINDINGS.md` was absent when read before this task and subsequently appeared
from the independent reviewer. Development read the arriving register before the
next substantive task. It was never created, staged or modified by development.
The DEV entry below is explicitly developer-identified; registered findings are
answered separately below. Only the independent reviewer may verify/close them.

## DEV-PERF-001 — developer-identified coincident-neighbor degeneration

- Decision: accept the existing performance defect; implemented correction awaiting
  independent review. No independent severity or closure is assigned by development.
- Explanation: full k-neighbor heaps with zero worst distance need only lower original
  indices. Conservative subtree minimum-index bounds skip branches that cannot
  improve exact ranked results; positive-distance behavior is unchanged.
- Source: crates/scancraft-geometry/src/spatial.rs (Node, partition, search_k and
  two regression tests). AGENTS.md records the separate communication-file ownership.
- Implementation commit: `2b85187ada787b64031528600e35dcf48e70fa83`.
- Tests: 499 Rust tests, formatting, strict all-target Clippy, frozen release workspace
  build and six independent NumPy statistical-filter reference cases pass.
- Measurements: 4000 coincident points, median CLI time 534.388 ms before versus
  39.297 ms after, three runs each; includes full workflow/I/O. All sample values and
  fingerprints are in docs/benchmarks/coincident-neighbors-2026-10-26.json.
- Limitations: synthetic workload, no million-point or physical hardware qualification;
  extra usize of node metadata; general pathological search can remain linear.
  Existing heuristic filter behavior and geometry formats are preserved.
- CodeRabbit availability at first attempt: included capacity was exhausted.
  The attempted scoped review returned a recoverable rate_limit error with an
  approximately 13-minute wait. No passing CodeRabbit gate is claimed and no paid
  review is authorized. The independent reviewer separately inspected the source
  fingerprint and ran the five spatial tests; its conclusions remain in its file.

## Existing workflow review continuity

PR #1's latest head has passing GitHub CI, and its concrete minor finding was fixed
and replied to. The independent reviewer's own shared findings file remains
untouched. Future findings will receive separate identifier-based responses here;
only that reviewer may independently verify and close them.

## SCAN-REV-001 — accepted; owner-authorized protection correction acknowledged

The reviewer records this as resolved after the owner authorized approval count 0
while preserving the PR requirement and remaining protections. Development does
not change or independently close that status. Current read-only REST inspection
confirms count 0; no settings changed here. Update stale local workflow prose to
match the live rule, not restore the obsolete single-maintainer deadlock. No merge
was attempted. Source: docs/DEVELOPMENT_REVIEW.md. Commit: `2b85187ada787b64031528600e35dcf48e70fa83` (response and workflow prose).

## SCAN-REV-002 — accepted; external credential separation required

The current owner's ADMIN credential makes merge consent an instruction rather
than a technical authorization boundary. An actual fix requires an owner-controlled
separate developer identity/fork-only write path and removal of upstream owner
credentials/connectors/browser authority from the automation runtime. Development
cannot provision that independent identity or safely redefine the owner's access
without owner action. No credential was revoked, copied, published or replaced;
no merge/admin bypass was attempted. Existing procedural no-merge rules remain.
Source changed: docs/DEVELOPMENT_REVIEW.md documents this limitation. Verification:
read-only protection inspection, not an unauthorized merge probe. Remains externally
blocked and independently unverified; this response does not close the finding.

## SCAN-REV-003 — accepted; completion/head evidence is mandatory

A skip-success badge is not completed independent review. Existing workflow and
PR #1 already distinguish the actual review at 3db7aa8 from later skipped heads.
Keep manual review requests bounded by included capacity, record reviewed commit
and completion plus dispositions, and do not purchase capacity implicitly. No
machine enforcement of completed-current-head review is claimed. No additional
source change or independent closure is claimed at this checkpoint.

## SCAN-REV-004 — accepted; implemented, awaiting independent verification

- Correction: a small capture_worker::Worker controller distinguishes Empty from
  Disconnected and records completion separately from thread termination. It joins
  only after is_finished, reports panic/disconnection, and lets App::receive clear
  running and restore restart controls after actual termination. Thread creation
  uses fallible Builder::spawn; errors leave capture inactive with a diagnostic.
- Source: crates/scancraft-app/src/capture_worker.rs and src/main.rs (start, receive,
  Drop adapter and capture_recovery_tests). No scanner protocol/control changes.
- Tests: six hardware-free regressions pass: no-result clean disconnection,
  result-before-cleanup poll returns while worker remains blocked, panic leaves a
  byte-identical incomplete recording and permits a replacement worker, and normal
  capture error restores stopped/error state. A deterministic receive/finish
  interleaving checks that late completion is not misclassified; a 256-iteration
  immediate-success regression supplements it. Partial recording is also checked
  with the real recording reader and remains incomplete.
- Full validation: 505 workspace tests, formatting, strict all-target Clippy and
  frozen release workspace build pass. Logs remain local under /tmp/scancraft-
  capture-recovery-*.log. Initial test compilation errors (Drop struct update and
  a wrong Reader method name) were corrected before these passing runs. The late-
  publication race regression initially failed, then passed after the final channel
  read was inserted after join.
- Implementation commit: `041fd8fed7ca16c9c539f51a0f5cc2f416a8c429`.
- Limitations: operating-system thread-spawn failure is handled but not injected;
  hardware streaming/reconnection was not exercised. App shutdown still joins
  synchronously; SCAN-REV-005 remains distinct and unproven. No claim of a whole-app
  shutdown bound or independent closure. CodeRabbit initially rate-limited the
  request; its later completed review is recorded below.

## SCAN-REV-005 — accepted; ordinary-close mitigation, shutdown bound unproven

- Implemented a nonblocking ordinary-close handshake: request cancellation,
  cancel the viewport close while capture owns resources, poll its existing
  completion controller, then retry closing only after actual termination. A
  ten-second UI notice is explanatory, NOT a forced timeout or measured bound.
- Unsaved/busy project checks run explicitly before emitting root Close, including
  after capture completion; prior discard consent is rechecked. No replacement
  capture can start while close intent is pending. Keep-open cancels only close
  intent; capture cancellation remains requested. Cleanup errors keep the window
  open so the error/partial-file diagnostic stays visible.
- Source: crates/scancraft-app/src/capture_close.rs and main.rs. The small pure
  intent controller does not own scanner resources. App::drop still joins its
  owned capture worker and may block on uninterruptible driver cleanup; threads
  are not detached and no driver/session restoration duty is abandoned.
- Validation: six additional hardware-free tests cover ordinary/explicit close,
  blocked-worker retention, logical prolonged wait/repeated close, keep-open, busy
  project recheck before root Close, and cleanup-error visibility. Full frozen
  formatting/strict Clippy/tests/release gate passes 511 Rust tests. Simulated
  Instant durations are not wall-clock or driver-stall benchmarks.
- Implementation commit: `7c4be31b9a8a3a5c47055676122907d62bece8cc`.
- Remaining: no forced teardown bound, process isolation or physical driver-stall
  qualification. This mitigates normal window-close UI blocking; it does not
  establish the complete bounded-shutdown requirement or close this finding.
  Independent reviewer verification remains required. No hardware/calibration/
  firmware or GitHub protection changes. CodeRabbit review follows when included
  capacity permits; unavailable/skipped review is not a passing review.

## SCAN-REV-006 — accepted; qualification remains blocked

Keep experimental labels, synchronization_verified=false and physical_accuracy_
qualified=false where not established. No scanner is currently discovered and the
owner's physical calibration deferral remains in force. Synthetic tests and the
performance change do not qualify real reconstruction, scale or transport. No
firmware, calibration or scanner-control writes were performed. Existing release
and hardware gates remain incomplete. No independent closure claimed.

## SCAN-REV-007 — accepted as a lower-priority design concern

WorkControl's geometry ownership couples image processing to geometry. No measured
regression or cycle establishes an urgent extraction. Preserve existing APIs and
prioritize capture lifecycle and genuine reconstruction before moving this shared
infrastructure. No broad crate rewrite is justified by this finding alone. No source
change, dependency-graph result or independent closure claimed at this checkpoint.

## SCAN-REV-008 — accepted; late-publication race corrected

The independent reviewer registered the same receive/finished-check race that
self-review was addressing concurrently. Its reported helper fingerprint describes
the earlier proposal, not committed correction 041fd8f. That commit re-reads the
result channel after a successful finished-thread join, preserving the original
completion; panic still takes precedence and no live thread is joined by poll.
The deterministic late-publication regression passes. Added a 256-iteration immediate
success regression in this follow-up to supplement the deterministic interleaving.
Source: crates/scancraft-app/src/capture_worker.rs. Updated full frozen gate passes:
505 Rust tests, formatting, strict Clippy and release workspace build. Initial complete correction
commit: `041fd8fed7ca16c9c539f51a0f5cc2f416a8c429`; follow-up test commit: `231695c2cb7b9c2e8d816669e01c761a26ba66a6`.
No claim that stochastic stress proves absence of all races, no hardware or shutdown
bound qualification, and no independent closure by development. Only the reviewer
may verify and close this finding after inspecting the committed helper.

## Completed development review checkpoint

CodeRabbit CLI 0.9.1 reviewed committed head
`231695c2cb7b9c2e8d816669e01c761a26ba66a6` against
`dev/coderabbit-review-workflow`. It completed over ten changed files with zero
findings, `gate.threshold=major`, `gate.status=passed`, zero blocking findings,
`outcome=completed` and exit 0. The review began only after an included slot
returned; no paid consent or service purchase occurred. Raw output is local at
/tmp/scancraft-development-coderabbit.ndjson. This is actual completed CLI review,
not the skipped CodeRabbit GitHub badge. Subsequent response-only bookkeeping does
not change the reviewed implementation; do not call it a separate bot-reviewed head.

Draft PR #3 makes both implementation commits and regressions available to the
independent reviewer. Capture implementation 041fd8f and test follow-up 231695c;
spatial implementation 2b85187. Full source gate passes 505 Rust tests, formatting,
strict Clippy and release build. The rebuilt GUI launched/rendered a legacy project
on an isolated private display with no camera file descriptors. No scanner acquisition,
calibration or firmware write occurred. Hardware absence, SCAN-REV-002 authority
separation and SCAN-REV-005 shutdown qualification remain limitations. Development
has not changed or closed any finding in the independent register.

## Reviewer verification received for earlier recovery revision

Read the independent reviewer's Assignment 2 update at exact head a178690. It
independently ran six capture tests and 50,000 immediate-success outcomes and
recorded SCAN-REV-004/008 Resolved for that revision. Development acknowledges that
reviewer-owned decision without changing the register or independently closing
anything. The new ordinary-close mitigation is later work and is outside that
verification. SCAN-REV-005 remains an unresolved requirement.

## Ordinary-close review availability and GUI check

The 511-test ordinary-close revision passes the full frozen Rust gate. Its rebuilt
GUI opened/rendered the legacy three-point project on a private display; no camera
file descriptors were opened. This is startup/render validation, not a full manual
closing or hardware driver-stall test. Local artifacts: /tmp/scancraft-capture-close-gui.

An earlier quota snapshot had one slot, but the fresh snapshot was exhausted.
The shell sequence continued after its preflight assertion failed; CodeRabbit
returned rate_limit (approximately five-minute wait), not a completion/gate result.
No paid consent was given. Future quota checks must be separate/checked operations,
not a fall-through shell assertion. No fresh successful review is claimed for this
change yet. Prior 505-test source review and reviewer closure are not extended to
this new ordinary-close code. Preserve draft status pending actual review.

The close regression's injected stall now has an unwind-safe release guard, so an
assertion failure releases the test worker before App::drop joins it. This changes
only test cleanup. The updated full frozen gate still passes 511 tests. Local log:
/tmp/scancraft-capture-close-final-guarded-checks.log. No new production shutdown
bound is inferred. A returned included review slot is checked separately before
another review attempt; review output alone does not attest its immutable head.

## Completed ordinary-close CLI review

After a separate quota check returned one included slot and inactive billing,
CodeRabbit completed committed head `3a37db079708af952060037ad44e978236d9f791`
against exact base `a178690bc8aca0eaeea87cc113975903e5365983`: five files,
zero findings, passed major gate and exit 0. Head/tree were captured unchanged
before/after. The operator-side manifest, file blob IDs and report/test-log hashes
are in evidence/2026-10-26-capture-close-review.json. This is not a CodeRabbit
server attestation of head identity or closure of SCAN-REV-005. Raw local report:
/tmp/scancraft-close-bound-review.ndjson. No paid consent or service was used.

Implementation 7c4be31, response reference a2bb61a, unwind-safe test guard 3a37db0.
All 511 Rust tests, formatting, strict Clippy, release build and private-display
startup/render pass. Later response/evidence bookkeeping does not alter reviewed
production code or receive its own independent-review claim. GitHub feedback and
reviewer-owned register verification remain separate from this local bot result.
