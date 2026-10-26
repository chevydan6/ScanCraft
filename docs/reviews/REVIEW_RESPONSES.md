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
- Independent review: CodeRabbit included capacity currently reports exhausted.
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
- Tests: five hardware-free regressions pass: no-result clean disconnection,
  result-before-cleanup poll returns while worker remains blocked, panic leaves a
  byte-identical incomplete recording and permits a replacement worker, and normal
  capture error restores stopped/error state. A deterministic receive/finish
  interleaving checks that late completion is not misclassified. Partial recording is also checked
  with the real recording reader and remains incomplete.
- Full validation: 504 workspace tests, formatting, strict all-target Clippy and
  frozen release workspace build pass. Logs remain local under /tmp/scancraft-
  capture-recovery-*.log. Initial test compilation errors (Drop struct update and
  a wrong Reader method name) were corrected before these passing runs. The late-
  publication race regression initially failed, then passed after the final channel
  read was inserted after join.
- Implementation commit: `041fd8fed7ca16c9c539f51a0f5cc2f416a8c429`.
- Limitations: operating-system thread-spawn failure is handled but not injected;
  hardware streaming/reconnection was not exercised. App shutdown still joins
  synchronously; SCAN-REV-005 remains distinct and unproven. No claim of a whole-app
  shutdown bound or independent closure. CodeRabbit review is pending included
  capacity; a rate-limit error is not a successful review.

## SCAN-REV-005 — accepted; shutdown bound remains unproven

A stop flag cannot interrupt arbitrary synchronous driver ioctls/restoration.
Do not detach threads and silently abandon device cleanup. A bounded lifecycle
likely needs an explicit isolation/supervision design with cleanup evidence.
No whole-app shutdown-bound claim or physical driver-stall test is made. Source:
existing App::drop and device lifecycle remain unchanged at this checkpoint.
Defer invasive isolation work until lifecycle requirements and fault-injection
coverage are concrete. No independent verification or closure claimed.

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
