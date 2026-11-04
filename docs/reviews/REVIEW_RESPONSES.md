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

## Completed GitHub review at 1ae418f and advisory disposition

The supported manual request completed in CodeRabbit's GitHub walkthrough at exact
head `1ae418fb357e26ddf2eb223acb6891c86988d7cb`, base ba5ec7c. The walkthrough says
“No actionable comments were generated”; its command reply says review finished.
The API contains no formal approving Review object and no inline threads. Do not
infer owner consent or an approving Review from the successful status. Sanitized
completion metadata and the source comment URL are retained in
evidence/2026-10-26-github-pr3-review.json; scoped service access links are excluded.

Advisory: Docstring Coverage 7.14%, default threshold 80%, 42 functions analyzed
across four files (eight unsupported). **Disputed as a blanket release gate**, not
hidden: these changes include private controller helpers, trait/adapter functions
and regression tests. Module documentation, public/internal contracts, invariants,
source comments, architecture flow and descriptive regression names document the
lifecycle. Repeating those as docstrings on every private/test function to reach an
arbitrary percentage does not establish correctness or maintainability. Public API
and non-obvious lifecycle contracts remain documentation obligations; targeted
clarification is appropriate when a concrete ambiguity is identified. No concrete
missing-contract defect was supplied by this metric. The advisory remains reported;
no threshold/check/configuration was lowered or disabled. No claim that every
CodeRabbit pre-merge check passed (four passed, one warning).

All 511 Rust tests, formatting, strict Clippy and release build pass; CI at reviewed
1ae418f passed both runs. Later response/evidence bookkeeping changes no reviewed
application source and does not receive an invented fresh-review claim. The prior
independent 004/008 resolutions remain the reviewer's decisions. 005 remains only
mitigated on normal close, not qualified for bounded forced teardown. No merge,
protection change, paid consent, firmware or physical calibration operation.

## DEV-RECON-001 — optional fractional stereo observations

Developer-identified reconstruction work, not an independent finding. Accepted:
add opt-in three-sample quadratic ZNCC peak refinement while retaining integer
winners, coarse confidence checks, old JSON defaults and provenance flags. The
processor uses fractional observations for original-camera triangulation. New
curvature, support, bidirectional consistency and work-budget checks reject
unsupported fits. Independently derived Rust mathematics; no copied upstream code.

Sources: depth/src/matching.rs, matching/subpixel.rs, pipeline.rs; depth tests and
reference example; CLI tests/subpixel_reconstruction.rs; tools/check_subpixel_reference.py.
Detailed method, compatibility and reproduction: docs/SUBPIXEL_STEREO.md.

Validation: frozen formatting, strict all-target Clippy, workspace tests (518 pass)
and release build; final log /tmp/scancraft-subpixel-final-checks.log. Independent
NumPy polynomial-fit oracle agrees within 7.4e-15 on four synthetic signed shifts;
legacy integer OpenCV comparison passes. Native CLI integration saves/reopens
synthetic geometry, preserves inputs and rejects overwrite. These are synthetic
results, not physical scanner accuracy. No firmware/calibration operation.

Implementation commit and automated review evidence follow after immutable commit.
Limitations: local parabolic bias, horizontal-only refinement, additional patch
work and inline optional diagnostics memory (not benchmarked), no live qualification.
SCAN-REV-006 remains unqualified; no reviewer status is changed.

DEV-RECON-001 implementation: `61e4c931476f055fb1d008d49a46675c9d4bc523`,
stacked PR https://github.com/chevydan6/ScanCraft/pull/4. CodeRabbit CLI completed
13 files against cc1322b with zero findings and passed major gate, exit 0. Head/tree
observed unchanged before/after; operator evidence and report hash are in
evidence/2026-10-26-subpixel-validation.json, not a server head attestation.
Included capacity only, inactive billing, no paid consent. GUI startup/render of
legacy three-point project passed on private display, visually inspected, no camera
FDs. This is not manual end-to-end GUI or hardware scanning qualification.
GitHub CI was pending at recording; skipped draft CodeRabbit badge is not review.
Later evidence-only bookkeeping does not change the reviewed implementation.
Independent reviewer retains verification/closure authority.

### DEV-RECON-001 rotated-source regression follow-up

Accepted developer-identified evidence gap: prior fractional pipeline regression
used aligned cameras. Added a test with both sensors rotated clockwise and with
the right sensor inverted. Expected source pixel coordinates are computed directly
from image rotation formulas, independently of rectification mapping helpers. The
test requires fractional source y in the vertical case, preserves unqualified
flags, and checks all three triangulated points against analytic plane depth.
Production source is unchanged. Sources: depth/tests/subpixel.rs; docs updated.
Focused test and full frozen gate pass: 519 Rust tests, fmt, strict Clippy/release;
log /tmp/scancraft-subpixel-rotated-checks.log. Synthetic rotations are not real
Raptor mode/calibration or physical accuracy qualification. Commit/review follows.

Rotated regression committed at `264960d5dc719c4e3fc42fa84649221abb58bf4e`.
CodeRabbit completed the four-file incremental review against f33dcfd, zero
findings, passed major gate and exit 0. Head/tree unchanged before/after; operator
evidence in evidence/2026-10-26-subpixel-rotated-review.json. Included capacity only,
no paid consent. Both GitHub test jobs passed at earlier f33dcfd; later heads require
their own CI. No fresh review is inferred from a skipped GitHub App badge.
Evidence-only follow-up does not alter reviewed tests or production source.

## DEV-TIMING-001 — recorded stereo timing investigation

Developer-identified mission gap, not an independent finding. Accepted: provide a
read-only native CLI report of nearest host-value candidates, ambiguity, reuse,
raw PTS equality and explicit unknown clock/synchronization/physical qualification.
No automatic reconstruction pairing or changes to hardware/calibration. Complete
recording validation and playback source guards precede final reports.

Sources: recording/src/indexed.rs cached FrameTiming, workflow recording_playback.rs
and recording_timing.rs, CLI main.rs and tests/recording_timing.rs. Method independently
implemented standard binary search, no upstream source import/dependency. Details
and memory cost: docs/RECORDING_TIMING.md. The index cache adds per-frame memory to
all indexed playback; no measured memory/performance advantage is claimed.

Tests: exhaustive independent-distance comparison, ties/reuse/endpoints/u64 limits,
capped samples, cancellation, corrupt suffix/profile rejection, source byte
preservation, cached reads after seeking and actual native CLI JSON. Full gate and
commit/review outcomes follow. Initial compilation exposed missing profile accessor
and CLI path/return errors; these were corrected before passing verification.
Clock domains, physical exposure pairing and real geometry remain unqualified;
no existing reviewer finding is closed or changed.

DEV-TIMING-001 full gate: 524 Rust tests, fmt, strict Clippy and release PASS; log
/tmp/scancraft-recording-timing-final-checks.log. Read-only genuine replay at a
caller-selected 1000us limit found 6 unique nearest candidates / 94 outside among
100 left and 100 right frames, no ties/reuse, 3 equal raw PTS candidates. Both
source SHA256s unchanged. Aggregate-only evidence is published; no private images
or timestamps. These observations do not establish shared clocks or exposures.

### CR-TIMING-PTS — CodeRabbit major finding disputed

Local identifier for the CLI finding, not an independent-review register ID.
Reviewed implementation `b2d8417b58585623a574054d5729b7c14fc90ffd` vs12fc3a2.
Bot alleges the new cache rejects previously valid missing/non-UVC PTS recordings.
**Disputed:** unchanged recording/lib.rs Reader::read_frame_inner invokes core
validate_frame (requires PTS) and validate_order (also requires PTS) before returning
any frame to IndexedRecording::build. Both v1/v2 use this path. Writer::push likewise
requires it; recovered valid prefixes do not make absent PTS legal. Optional fields
in display structs do not override the reader's actual contract.

Added regression indexed::tests::existing_reader_rejects_crc_valid_missing_or_malformed_pts_before_index_caching
constructs CRC-valid missing-PTS, empty-metadata and malformed-header frames in v1
and v2, verifies the underlying reader and index both reject them for the PTS/header
reason rather than CRC. It also passes when inserted alone into an isolated exact
12fc3a2 snapshot: one test passed, production unchanged. Log
/tmp/scancraft-baseline-pts-test.log. Current focused regression passed as well.
No Option conversion or relaxation of established integrity gates is justified.

Raw review completed 11 files, one major finding, blocked major gate, exit 2.
That raw gate is preserved, not renamed passed. Evidence/report hash and baseline
reader blob are in evidence/2026-10-26-recording-timing-review.json. Included quota
only, no paid consent. Independent reviewer retains verification authority; PR5
remains draft pending disposition visibility. No repeated paid/bot loop requested.
GUI startup/render also visually passed on a private display with no camera FDs;
not full manual workflow or hardware qualification. Added regression commit and
final whole-workspace count follow.

Post-disposition full frozen gate passes 525 Rust tests, formatting, strict Clippy
and release build. Log /tmp/scancraft-recording-timing-disposition-checks.log.
This adds test evidence and documentation only; production implementation is still
b2d8417. Raw bot gate remains blocked with its specific major finding disputed.

## DEV-TIMING-002 — cross-check raw PTS independently of host nearest

Accepted developer investigation: genuine recordings contain many equal raw PTS
values with differing kernel host times. Added independently implemented sorted
exact-value lookup in workflow recording_timing/raw_pts.rs. Unique means exactly
one occurrence on each side; repeated values are explicitly ambiguous, never
assigned across an assumed wrap epoch. No automatic geometry pairing or clock
qualification. Matching clock/exposure/accuracy flags remain false. Iterator Clone
contracts expose existing cloneable cached iterators; no new index fields/format.

Sources: workflow recording_timing.rs, recording_timing/raw_pts.rs; recording
indexed.rs/playback iterator signatures, CLI regression, docs/RECORDING_TIMING.md
and RECORDED_CLOCK_OBSERVATIONS.md. Method is original project code using standard
sort and partition-point lookup, no imported upstream algorithm/dependency. Added
temporary key/index memory and sorting cancellation limits are documented.

529 Rust tests, fmt, strict Clippy and release pass; full log
/tmp/scancraft-clock-comparison-checks.log. Four new raw-value regressions include
an independent exhaustive equality oracle, collisions/wrap values, extremes, absent
matches and cancellation. Native CLI verifies nested fields and unchanged inputs.
Genuine replay reproduces 72 unique equal-PTS pairs, 3 within1000us, min/max8/20026us;
independent CRC-validating Python parse agrees; original SHA256s unchanged.
Raw clocks and exposures remain unqualified, not physical geometry evidence.

Independent Assignment30 rejected SCAN-REV-009; development acknowledges reviewer
authority without modifying that register or reclassifying the raw blocked bot
gate. PR5 exact b452b09 CI both passed. New feature is separate from that reviewed
implementation. CLI included quota is exhausted (about40min at latest check); no
paid usage/retry requested. New feature review remains pending; commits follow.

DEV-TIMING-002 implementation `61582f9e11afe419e64b22c4ba3fe72053408cf7`, stacked draft PR6
https://github.com/chevydan6/ScanCraft/pull/6. Validation identity/log hash in
evidence/2026-10-26-clock-comparison-validation.json. Automated review pending due
to verified exhausted included quota; no paid service or duplicate call. Later
response/evidence bookkeeping is not a reviewed source revision. Reviewer-owned
register and all unrelated work remain untouched.

## DEV-SEQUENCE-001 — recorded stereo to native tracking/fusion

Accepted developer mission work: connect explicit increasing recorded-frame pairs
to unchanged reconstruction and ScanSession tracking/fusion. New RecordingProcessor
reuses full validated indexes and rectification maps; process_with_processor shares
existing per-pair processing. Existing six adapter regressions still pass and a
new cached-vs-standalone report comparison plus mutation guard regression passes.
No context/binding/confidence gate is relaxed. No pair chosen by timing heuristics.

Sources: workflow lib.rs, recording_reconstruction.rs, recorded_sequence.rs; CLI
main.rs and tests/recorded_sequence.rs; workflow tests/recording_reconstruction.rs.
This is independently implemented orchestration of existing project algorithms,
not imported upstream code or an added dependency. Detailed interface/bounds and
source retention: docs/RECORDED_STEREO_SEQUENCE.md.

535 Rust tests, frozen fmt, strict Clippy and release PASS; log
/tmp/scancraft-recorded-sequence-verified-checks.log. Tests retain all raw sources,
reopen project, refuse overwrite, preserve originals, reject invalid/cancelled work
without final publication, retain no-geometry diagnostics, and exercise actual
reconstruction->tracking loss->recovery with rejected geometry excluded from fusion.
Initial wrong-depth fixture omitted the existing inverted right-sensor orientation;
corrected before passing. A later strict-order preflight required updating the
out-of-range fixture to keep indices increasing before reaching its intended error.
No production gate was weakened to accommodate either test.

Limits: offline explicit pairs, point-to-point only, unsupported mesh/normal request
explicitly rejected, no blank-frame resumption, multi-frame lineage remains external
to fused project. Physical accuracy/sync/binding flags stayfalse. Scanner discovery
empty, calibration/firmware unchanged. No existing reviewer finding is closed by
this implementation. Automated review and exact commit evidence follow; no paid
service authorization is given.

Release CLI demonstration integrated three synthetic frames and reopened/rendered
its fused project using the existing GUI on a private display, no camera FDs. Input
SHA256s unchanged and output overwrite rejected. Fused cell count5 from3stationary
positions reflects tiny pose perturbations crossing voxel boundaries; a regression
checks all fused coordinates and source coverage against analytic points within
1e-7mm instead of assuming exactly3cells. This does not qualify physical precision.
Local proof root is recorded in /tmp/scancraft-recorded-sequence-proof-root.txt;
GUI evidence /tmp/scancraft-recorded-sequence-proof-gui. Fresh included CLI quota0
with about21minutes until availability; no paid review attempted.

DEV-SEQUENCE-001 implementation `6128be4602e15db29660542e274443dad37a2f74`,
stacked draft https://github.com/chevydan6/ScanCraft/pull/7. Final535 gate log,
source tree, release CLI/GUI hashes and synthetic runtime summary are bound in
evidence/2026-10-26-recorded-sequence-validation.json (operator evidence only).
Final source GUI startup/render visually inspected on private display, no cameraFDs.
Reviewer Assignments37/38 independently found no actionable defect in their stated
scope, ran11 adapter/sequence+1 nativeCLI tests on stable snapshot, then mapped
that review to6128be4 and separately passed the added context assertion test.
Full535 remains developer evidence; no transfer of review to later bookkeeping.
GitHub CI pending at recording, App draft-skip is not completed review. Included
CLI quota still unavailable, no paid retry. Reviewer statuses/authority preserved.

## DEV-SEQUENCE-GUI-001 — native recorded-sequence workflow

Accepted developer mission work: add native background panel and startup operation
for the shared recorded-stereo sequence pipeline. Unsaved-document replacement
uses existing submit/save/discard guard. Workflow prepare/publish split lets the
worker build Editor/Preview BEFORE atomic publication, then commit document/selection
without fallible view construction. Original CLI run wraps these stages unchanged.
Publication rechecks sources/cancellation; late pre-publication errors preserve
staging, and post-publication sync errors explicitly identify committed output.

Sources: app cloud_ui.rs, cloud_jobs.rs, main.rs; workflow recorded_sequence.rs,
workflow tests/recording_reconstruction.rs; private-display helper. No new algorithm
copy/dependency or scanner action. Existing architecture and algorithms retained.
Regressions extend dirty guard, verify GUI worker document/selection persistence
on cancelled/overwrite errors, exact saved/live geometry equality, startup arity,
and cancellation/source mutation between preparation/publication. Detailed
interface/publication contract and memory overlap: docs/RECORDED_STEREO_SEQUENCE.md.
Final gate/runtime/review/commit evidence follows. Fused project multi-frame lineage
still external; clocks/exposures/physical accuracy remain unqualified. No finding
status changed. No paid review authorization; included quota is checked separately.

DEV-SEQUENCE-GUI-001 final source gate:538Rusttests, frozenfmt/strictClippy/release
PASS /tmp/scancraft-sequence-gui-complete-checks.log; helper Python syntax passes.
Private-display synthetic startup background operation completed; all29outputfiles
match CLI byte-for-byte, inputSHA256s unchanged, three frames integrated, no camera
FDs. Screenshot window-3.png visually inspected, completed panel/result visible.
Local /tmp/scancraft-sequence-gui-operation and datasetpointer
/tmp/scancraft-recorded-sequence-gui-dataset.txt. Aggregate evidence in
evidence/2026-10-26-sequence-gui-equivalence.json. Capture precedes a help-string-only
addition; numerical/operation/UI behavior unchanged, fullgate rerun afterwards.
No claim of manual dialog interaction or physical scanner qualification.

Independent Assignment43 inspected stable uncommitted GUI snapshot and passed
16 focused tests (app2, dirtyguard1, workflow12, nativeCLI1), no actionable defect
in that scope. This is not a full538rerun or committed-head/service attestation.
CLI included capacity was0 with nextslot~11:47UTC; no paid request or repeated
unchanged review. Pending combined review should cover rawPTS+sequence+GUI versus
b452b09 to avoid duplicate review of unchanged earlier source. Commitrefs follow.

DEV-SEQUENCE-GUI-001 implementation `1bed149cdc5e998d77710a04d25b41d39e76bb4a`,
PR https://github.com/chevydan6/ScanCraft/pull/8. Included capacity returned1 with
inactive billing; one combined CodeRabbit CLI review against exact b452b09 completed
26files, zero findings, major gate passed, exit 0. Head/tree unchanged before/after;
operator manifest/report and full538gate hashes plus file blob IDs in
evidence/2026-10-26-combined-recorded-sequence-review.json. No paid consent/flag.
This covers current combined implementation, not a server immutable-head attestation
or an invented fresh review of previous variant PR heads. The old PR5 raw blocked
result and independent009rejection remain preserved separately.
Later evidence-only bookkeeping does not change reviewed code or receive a new
review claim. GitHub App feedback remains separate; no owner merge consent implied.

### Completed GitHub App review at3129761

Supported single manual request completed at exact
`312976145d08afbc96cc9466535e0ec54028152e`, base47293dd. Walkthrough6415231559
explicitly marks source/covered head reviewed and says no actionable comments;
reply6415310710 says Review finished. Post-completion API: zero inline comments,
zero review threads and zero formal Reviews. This is not formal APPROVED or owner
merge consent. Both test checks passed at covered head121555435820/121555459781.
Sanitized completion metadata in evidence/2026-10-26-github-pr8-review.json; raw
service-access links excluded. No paid consent/billing/configuration change.

Docstring advisory8.33% versus80%,24touched functions across6files,6unsupported
skipped: **disputed as blanket merge blocker**, not hidden. This change contains
private/startup/controller helpers and descriptive regression functions alongside
public staged-publication APIs. Module/type/API docs and the architecture/interface
document explain ownership, ordering, cancellation, source checks, staging retention,
committed-output sync errors and external provenance. Repeating those as docstrings
on every private/test function to hit a percentage does not establish correctness.
Concrete missing public/non-obvious contracts remain legitimate documentation work;
this metric supplied no specific such defect. No threshold/check was weakened.
Four premerge checks passed and one warning remains; no all-checks-pass claim.

Current implementation remains1bed149;3129761 and later evidence-only bookkeeping
do not change app code. Full538gate and GUI/CLI29file equivalence remain recorded
with their actual scopes. Independent reviewer retains all finding/closure authority.

## DEV-SEQUENCE-REF-001 — durable initial sequence references

Accepted developer mission gap: fused project previously lost multi-frame metadata
on standalone save. Added bounded typed sequence references in Editor/ORPROJ07,
kept outside geometry history, retained in the producer's saved/live GUI editor.
Old02..06save/load behavior remains when sequence metadata absent; strictv7 headers
and safe complete artifact sets prevent downgrade/silent loss. No filesystem I/O
on project load. Raw files/reports remain external; initial references do not claim
current per-point correspondence or qualified physical data.

Sources: geometry sequence_references.rs/lib.rs/formats.rs; workflow recorded_sequence.rs
and source_verification.rs; app source panel/verification/editor creation; CLI/GUI
and workflow regressions. Existing serde/CRC implementation reused, no imported
algorithm or dependency. Artifact fingerprints captured at writing; project/root
manifest excluded from embedded references to avoid checksum cycles. Explicit
verify-source-bundle now handles sequence folders, rejects symlink parents and
checks nested directory/file stamps. It verifies bytes, not semantic replay or
cryptographic/historical/device authenticity. Details: SEQUENCE_PROJECT_REFERENCES.md.

Tests cover metadata and CRC-valid format rejection, old/no-source round trips,
trim/undo/redo/save/reopen, moved-folder verification, changed source bytes and
symlink-parent refusal, plus native CLI and live/saved GUI reference equality.
Initial shared verifier refactor shadowed artifact count in progress reporting;
compile caught it and total count now captured before loop. Final full gate, runtime
and automated review outcomes follow. Reviewer register remains untouched; all
physical/merge/protection/paid constraints preserved.

DEV-SEQUENCE-REF-001 final full gate541Rusttests/fmt/strictClippy/release PASS:
/tmp/scancraft-sequence-references-final-source-checks.log. Synthetic release proof
createsv7, preserves exact references through native filtering/save, moves sources
and verifies27artifacts in CLI and GUI; inputSHA256s unchanged. GUI screenshot
/tmp/scancraft-sequence-ref-verify-gui/window-2.png visually inspected. All27non-
location-dependent files match GUI/CLI; normalized projects and root-manifest
project fingerprint differences behave as documented. Aggregate evidence:
evidence/2026-10-26-sequence-references-runtime.json. Runtime preceded only added
parent-loop cancellation checks/deduplicated stat calls; final541gate covers those.
No live physical, whole-original-recording, semantic replay or authentication claim.

Independent Assignment55 inspected stable uncommitted snapshot, passed30targeted
tests including8legacy origin tests, newformat/history/verification tests and
nativeCLI/GUI; no actionable defect in that scope. It is not committed-head/bot
attestation or full541rerun. No finding status changed. Fresh included review
capacity/billing will be checked before one incremental review; no paid consent.

DEV-SEQUENCE-REF-001 implementation `c67a46fba687fb941cacd7b1d01912c52ea92597`,
PR https://github.com/chevydan6/ScanCraft/pull/9. Included CodeRabbit CLI review
against exact13790e3 completed15files, one minor finding, zero blocking findings,
major gate passed, exit 0. Head/tree unchanged before/after; operator manifest,
file blob IDs and log/report hashes in evidence/2026-10-27-sequence-reference-review.json.
This is not server immutable-head attestation or a new review of later bookkeeping.

CR-SEQREF-ID-BOUNDS (local bot identifier, not an independent finding): **accepted
confirmation request, no added code needed**. ScanSession::new delegates to
new_with_method, session.rs:55ff, enforcing nonempty <=128bytes with exactly the
same ASCII set before RecordingProcessor::open or staging. storage.rs::read calls
CalibrationRecord::validate, enforcing record_id1..128bytes; RecordingProcessor::open
loads this before either Playback::open/index. Therefore valid upstream inputs
cannot fail these same ID restrictions only after expensive staging. Duplicating
these existing checks would add maintenance without closing a demonstrated gap.
Minor conditional request satisfied by source verification; no defect or validator
weakening introduced. No repeated unchanged bot call or paid consent.

Independent Assignment56 maps prior30targeted tests to this committed source and
separately passes5verifier tests for parent-loop delta; operator runtime artifacts
independently inspected/recomputed. Full541 remains developer evidence. Reviewer
register/statuses untouched. GitHub review/CI remain separate from this CLI result.

### PR9 App minor findings — accepted

CR-SEQREF-EMPTY-DIRECTORY (GitHub comment4381521555): accepted. The sequence
verification button now follows its single-frame sibling's nonempty/trimmed field
guard, preventing a pointless empty-path job. Existing outer busy guard and click
behavior retained. Source: app/cloud_ui.rs. No mirrored button-condition unit test
added for this low-impact UI correction; formatting/build plus existing GUI guard
and verification tests remain appropriate.

CR-SEQREF-PARENT-DIAGNOSTIC (comment4381521578): accepted. Nested-parent metadata
errors now name the affected directory; non-directory/symlink errors also include
its path. Behavior remains fail-fast, with no unsafe fallback or per-artifact
report promised. Source: workflow/source_verification.rs. Extended moved-source
regression removes a frame parent and requires frame-0000 in the error. Existing
mutation/symlink checks remain. Neither original bot comment nor reviewer register
was altered/resolved by development. Final tests, commit and review evidence follow.

SCAN-REV-010: accepted, implemented via the same egui add_enabled/trim guard as
the single-frame panel. Empty and whitespace-only values disable dispatch by
source inspection; nonempty-folder pipeline/GUI verification remains covered.
No new manual empty-field click proof claimed. Source cloud_ui.rs. Reviewer alone
may verify/close the register finding; development does not change its status.

SCAN-REV-011: accepted, errors identify the computed retained frame directory.
Existing moved-folder regression now removes frame-0000 and asserts that directory
and diagnostic context in the propagated error. Source source_verification.rs and
workflow tests/recording_reconstruction.rs. Full541Rusttests/fmt/strictClippy/release
PASS /tmp/scancraft-sequence-reference-app-fixes-checks.log. No change to fail-closed
checks, source references or geometry. Both corrections await independent closure;
commit and incremental review evidence follow.

Correction commit `dff5748834049f456eef6128129b4affe2c9078b` full541gate passes.
One included incremental CLI review versus exacta68666f completed4files, zero
findings, majorPASS, exit0; head/tree unchanged, evidence/report/log hashes in
evidence/2026-10-27-sequence-reference-corrections.json. No paid consent/flag.
Independent Assignments63/64 verified7targeted tests and mapped the source to this
commit; reviewer independently resolved SCAN-REV-010/011. Development acknowledges
that decision without changing the register. No live empty-field click claimed.

GitHub initial App review at a68666f was completed COMMENTED, two minor findings,
not APPROVED. Both have factual source/test replies. Threads currently outdated
and unresolved; developer has not closed them. One scoped bot confirmation of
these corrections requested, not another full-PR review. The premerge docstring
warning28.57% vs80%,28functions/9files/7unsupported is retained: disputed as blanket
functional blocker for the same documented contract-based reasons recorded earlier.
Four premerge checks passed, one warning; no threshold/check weakened.

PR9 scoped GitHub confirmation: comment6425555338 independently inspected exact
dff5748 correction/source/regression/replies and says both findings addressed.
No new full review or test rerun claimed. Follow-up6425624258 reports its GitHub
connection has viewerCanResolve:false for both threads, so it left them unresolved.
A permitted reviewer must close them; development will not self-resolve, grant
permissions or bypass conversation requirements. No approval/merge change.
Sanitized initial App/review/scoped confirmation metadata in
evidence/2026-10-27-github-pr9-review.json. Raw service-access links excluded.
Initial App review is COMMENTED at a68666f with2minor comments, not clean/no-findings;
subsequent incremental CLI dff5748 is clean with a passed major gate. Independent
file reviewer resolved010/011 separately. Correction CI: one passed, one still
installing Linux dependencies; neither pending nor skipped badges count as pass.

## Developer follow-up: preserve stream cleanup diagnostics

Related SCAN-REV-005: accepted need for cleanup evidence; this is a limited
diagnostic correction, not a resolution of the unbounded driver-shutdown finding.
Inspection found Stream::close overwrote earlier failures with each later failure.
The cleanup collector now retains every failed phase in order and the first
io::ErrorKind. Drop identifies the V4L2 buffer type (1 video /13 metadata).
Files: crates/scancraft-device/src/{lib.rs,cleanup_errors.rs}. Restoration order,
remaining-attempt behavior, state flags, Drop ownership and synchronous ioctls
are unchanged. All restoration steps still execute after earlier failures.
No new retries, detachment, timeout or driver-cancellation guarantee is introduced.

Two hardware-free tests check multiple failure messages, OS error context, first
kind/order, intervening successes and empty/success-only cleanup. They exercise
the error collector, not a real or stalled V4L2 driver. The scanner is absent
from the current lsusb inventory, so hardware cleanup/disconnect qualification
was not performed. Full frozen gate passed543Rust tests, formatting, strict Clippy and release build
(/tmp/scancraft-cleanup-diagnostics-checks.log). Review/commit evidence follows.
This is independently identified development work; original reviewer register
and its finding status are untouched. Implementation is project-local MIT Rust;
no dependency or upstream source was added.

Actual CLI review of72c5cf6 against8da307e completed3files, zero findings,
major gatePASS, exit0. Earlier empty-diff skip was not counted as review.
Further developer inspection found Capture::close also hid metadata failure
when video closure failed via Result::and. Both node results now use the same
collector, labeled video/metadata, with a nested multi-error regression. Both
close calls execute before returning as previously; no new hardware operations.
This later correction is outside the72c5cf6 CLI review scope; final validation
and incremental review are required. SCAN-REV-005 remains reviewer-owned/open.

Final local gate passed544Rust tests/fmt/strictClippy/release; evidence in
evidence/2026-10-27-stream-cleanup-review.json includes hashes and exact reviewed
commit. Final two-node follow-up remains pending incremental review: fresh quota
read0included, billinginactive, next availability approximately18minutes at check.
No paid review attempted; initial72c5 review must not be transferred to this delta.
Publish as draft for source inspection while required final automated review is
pending. No approval/merge or finding closure claimed.

## Developer investigation: SDK byte mirror numerical reference

Related SCAN-REV-006: continue narrowing unverified runtime orientation without
physical calibration. New optional tools/probe_sdk_byte_mirror.py and
sdk_byte_mirror_probe.c execute only an exact-hash93-byte private arithmetic
leaf on synthetic pixels under device-inaccessible/network-isolated bubblewrap.
No DLL loaded or vendor bytes published; no production code/settings changed.
The leaf matches horizontal row reflection over2,304,075pixels/sixshapes in each
direction, with exact involution. docs/MIRROR_STATUS.md records static accessor/
dispatch addresses and limits. Evidence under reviews/evidence records DLL/leaf/
helper hashes. This is actual vendor-leaf execution, not proof of Raptor runtime
orientation, active mirror flag, calibration convention, geometry or accuracy.
Helpers are project-original diagnostics, not translations of vendor code.
Three hardware-free negative-input tests PASS, Python syntax PASS, and frozen
fullgate544Rust tests/fmt/strictClippy/release PASS. Automated review pending
included quota; no paid usage. Independent Assignment73 separately confirms
PR10 diagnostic source and30device tests, with005 still Open.
Independent register remains untouched and006 is not claimed resolved.

Extended the reference to compare actual leaf output directly with the existing
Rust GrayImage::reflect_x_into, using the new synthetic-only sdk_mirror_reference
example (no production function change). All2,304,075outputbytes match exactly;
outputSHA985732008ce07bbcc0ce40176c1207d7f890b771c3daa9631e3ef3edba19399b.
An actual wrong-reference probe fails closed. C helper emits synthetic pixels
only to captured stdout; report goes to stderr and Python emits aggregates only.
New fullgate544/fmt/strictClippy/releasePASS, three guardtestsPASS; evidenceupdated
with finalhelper/fullgatelog hashes. Actual vendor execution remains developer
evidence; independent Assignment77 inspected source/guardtests only and explicitly
did not execute private bytes.006 remains UnderReview, not resolved by this result.

Documentation correction: CALIBRATION.md still claimed no factory decoding and
no integrated calibrated workflow. Updated it to distinguish existing bounded
factory-field extraction and declared-calibration offline/GUI reconstruction
from still-unqualified physical/live Raptor scanning. Source contracts remain
factory_named/factory_pose/factory_laser, storage, image/depth/geometry crates and
recorded_sequence workflow; no historical rationale invented or code changed.

Combined final CLI review completed at d79959850bfa211cb558ea75476ba26b0553c82d
(treeb39ff49d488e59077b9c9c99b5d3b757f126b6b7) versus72c5cf6:11files,
zero findings, majorPASS, exit0. Sourcehead/tree unchanged before/after. Fresh
quota1/inactivebilling preceded launch; no paidflag/consent. Source/blob/report/
fullgatelog evidence: evidence/2026-10-27-cleanup-mirror-combined-review.json.
Covers PR10 finaltwo-node correction (bothsourceblobs exactlymatcha6fa2d5) and
PR11 reference/probe/docs. Together with earlier72c5 review this covers current
cleanup implementation; it is not a full prior-code audit or GitHub Appapproval.
Earlier pending statements are historical and superseded by this completion.
Independent73/77/79 scopes acknowledged without changing their findings/status.
Both PRs may now leave draft for actual Appreview; no merge authorized.

PR10 GitHub App review COMPLETED at exacta6fa2d5 versus8da307e. Walkthrough
6425906538 covers4files, reviewed marker maps source/coveredcommits exactly;
reply6426474136 says Review finished. No actionable comments, noinlinecomments,
noformalAPPROVED review; GraphQL confirms zero threads. Both exacta6faCIjobsPASS.
Initialcommand wasrate-limited; oneafterresetcommand6426471987 accepted.
Sanitized evidence: evidence/2026-10-27-github-pr10-review.json. No merge.

Docstring warning9.09% vs80%,11functions/2files,2unsupported retained. Accept
clear cleanup-contract documentation as useful; dispute a blanket requirement
to document trivial private/test functions merely to reach a percentage. Module
and finish contracts describe ordered accumulation/firstkind/no retries.
Explicit limitation: aggregated io::Error preserves firstkind and all OS error
text, but raw_os_error() is no longer structurally available; inspected callers
report diagnostics rather than matching errno. Further public API documentation
can be improved alongside transport work. Four reported passing premergechecks
include skipped checks, plusonewarning: NOT allchecks passed. No threshold weakened.

PR11 GitHub App request6426490393 afterreset remained rate-limited; reply6426492648,
summary6426065548 at20:24:35Z says nextfreeOSSreview59minutes (~21:24UTC).
Do not retry before thatwindow or enablepaidusage. CLI combinedreview remains
completedPASS; Appreview is separately pending.006 remains unqualified.

## Developer milestone in progress: native live stereo scan pipeline

Related SCAN-REV-006 accepted end-to-end gap: new shared live_stereo Processor
and runner connect live Frame inputs to existing rectification/matching/depth
and ScanSession tracking/fusion. CLI scan-live-stereo uses native Capture sources
for interfaces0/2, explicit declared calibration/settings, and a finite1..1000
raw frames per-camera cap. Physical entry rejects synthetic records; source
binding/sync/physical qualification remain false. No factory mapping is guessed.

Raw recording append precedes try_send into channel4; pending association retains
2frames/side, exactrawPTS+explicit hostwindow selects candidates only. Drops and
unmatched frames are accounted; full raw CRC/footer replay and uniquePTS check
precede publication. Queue drops do not discard recording data. No geometry
produces no fake tracking pose; rejected alignment is not fused and later pairs
can recover. Tracking uses deterministic evenly-spaced source cloud indices.

Related005: camera owners are always joined on error/unwind, group settings
restore/readback precedes atomic no-overwrite publication, source/read/close and
restoration failures are retained. Synchronous kernel calls still lack a bounded
shutdown guarantee;005 is not claimed resolved. Source trait/runner are in
workflow for GUI reuse; CLI holds only native adapter and signal ownership.
No projector, mirror, gain, firmware or physical calibration operation is added.
Native hardware validation is unavailable while scanner absent; synthetic injected
sources validate plumbing only. Sources workflow/live_stereo.rs,live_stereo/runner.rs,
CLI/live_scan.rs+command, workflow tests/live_stereo.rs. Fullgate/review evidence
follows; no independent finding status changed. Implementation is project-original
MIT orchestration using existing Rust algorithms, no new dependency/upstream copy.

Live-pipeline validation milestone: final560Rusttests/fmt/strictalltargetClippy/
releasePASS;16new tests (6engine,10runner). Source-read panics now still attempt
explicit close; closepanics are diagnostics, cameraowners never detached.
Outputpreflight now checks parent/existing paths before sourceopen, with an
explicit open-counter regression (earlier caught-panic assertion was inadequate).
Wrong-depth test uses existing fixture's180degree right-camera convention and
correct synthetic intensity transform; actual pipeline rejects it asLost and
recovers without fusing it. No thresholds weakened to make tests pass.

Release synthetic_live_scan example executes same runner, produces10files/9CRC
manifestentries/3analyticallyexpectedpoints within1e-7mm numericaltolerance.
Both generated rawrecordings pass releaseCLI replay--decode; all sourcefixture
SHA256 unchanged. Runtimeproof is synthetic with injected restorationcallback,
NOT hardwarequalification. Actual release native-entry command rejects synthetic
calibration with exit1 beforecreatingtarget or openingdevice. Proofmetadata in
evidence/2026-10-27-live-stereo-runtime.json; privatefiles under/tmp pointer
/tmp/scancraft-live-proof-root.txt, notpublished. Independent97 inspected earlier
snapshot and passed13targetedtests; laterpanic/blank/repeatPTS tests/example/doc
changes are beyond that snapshot. Sourcecommit andCodeRabbit evidence follow.

Immutable live source484524d20ccc2707cbb62951d1dedfff28bd4642 tree58b61cd1c602ba5855e57601a1a2a41a19ed7395
committed and pushed; draft PR12 basee1c4fc6. CodeRabbit CLI0.9.1 versus exact
e1c4fc686f5451c6a0ef9a82ff4eba546fb5dfee completed12files,1minor,0blocking,
majorPASS,exit0; sourcehead/tree unchanged. Freshquota2/inactivebilling checked,
no paidflags/consent. Rawreport/blob/log evidence: evidence/2026-10-27-live-stereo-review.json.

CR-LIVE-IDEMPOTENT-RESTORE: confirmation accepted, proposeddefect/change disputed.
CodeRabbit conditionally warned a second lease.restore after publication could
perform I/O/fail. Actual SettingsLease::restore (stream_group.rs:95) immediately
returnsOk when self.restored; all ioctls are belowthatguard. Successful restore
sets restored=true; bundle requires thatsuccess before publishing. Drop calls
the same guardedmethod. Thus post-publication outercall performs no deviceI/O
and cannot turn a successful published scan into an error. Failure paths have
not published and retain useful besteffort retry/Drop cleanup. No production
change or extra hardware test is warranted for this deterministic guard; do not
replace actual implementation evidence with a hypothetical nonidempotent lease.
Required560test/runtime validation remains scoped as above, not hardwareproof.
Earlier pending CLI statements superseded; App review stillpending/freequota only.

PR12 GitHub App completed COMMENTED review5659331857 at00114d89d6258146723172e8f0c3eead863faab9.
One minor documentation comment4382715737 accepted: separate tolerance value
from words/unit in LIVE_STEREO_SCAN.md. Numeric1e-7mm tolerance unchanged.
Corrected typography only; no production/test/parameter change. Verified exact
line diff and git diff --check; no new numerical test or paidreview warranted.
Existing560test evidence applies to unchanged executable source. Original App
comment/register untouched; only independent reviewer may close findings.

## Developer milestone — live GUI controls (uncommitted)

Independent013 resolution acknowledged; typography-only commit84b89f4 remains
unchanged. Independent012 rejection acknowledged; idempotent restore semantics
retained in the shared acquisition adapter. No reviewer status edited.

Developer implementation adds one-slot previews, pause/resume, graceful stop
versus abort, shared native acquisition ownership and guarded completed-project
handover. Source: app/live_ui.rs, workflow/live_stereo/control.rs and runner.rs,
geometry/fusion.rs and session.rs, new acquisition crate; CLI retains signals.
571-test gate passed before final documentation; final gate is running.
Actual release GUI on private authenticated X display passed synthetic pause/
resume/stop-save/editor-open interactions. Report46pairs,16paused,46rawframes
per side,6fusedpoints,zeroprocessingdrops,gracefulStoptrue; no camera/USBfds.
Evidence: evidence/2026-10-27-live-gui-runtime.json and LIVE_STEREO_GUI.md.
This is developer evidence on uncommitted source, not independent closure,
physical qualification or benchmark.005 remains unbounded-driver limitation;
006 physical binding/accuracy remains unproven. Exact source commit/review pending.

## CodeRabbit live GUI source review — accepted minor corrections

Source5aa93c35ce911f83bde8b5103e5be049e2101cc1 versus84b89f4 completed
23files,2minor,0blocking,major gatePASS,exit0.
CR-LIVE-FINAL-PREVIEW accepted: tick previously drained before polling completion,
so a final offer between those operations could be abandoned when job cleared.
Poll/join now precedes mailbox drain; an unfinished worker retains the job for
future ticks, while a finished worker can make no later offer. Source live_ui.rs.
This developer-observed race was independently also identified by CodeRabbit.
CR-LIVE-STATUS-CONTRADICTION accepted: PROJECT_STATUS preceding paragraph now
states GUI exists with synthetic validation; hardware remains unverified.
No reviewer-owned finding status changed. Existing final-snapshot runner and
worker lifecycle regressions apply; full gate after corrections follows.

Correction506c01c1c69378d6e31339944969213213f43e3e passed the full571-test,
fmt/strictClippy/release gate. Incremental CodeRabbit review versus5aa93c3
completed4files,0findings,majorPASS,exit0. Source tree4fc3b4b unchanged.
Freeincluded1/billinginactive checked beforehand; no paidconsent. Sanitized
metadata added to live-gui-review.json; rawreports remain local. PR13 ready:
https://github.com/chevydan6/ScanCraft/pull/13 base dev/live-stereo-scan.
No auto-merge or ownerapproval inferred. App/current-headCI checked separately.

## SCAN-REV-014 — accepted remaining completion-counter correction

Assignment110 is accepted. Correction506c01c fixed mailbox loss but still read
recorded/drop/skipped counters before completion polling. live_ui.rs now polls
and joins before reading every final display value and draining the mailbox.
A synchronized GUI-consumer regression releases the real one-pair synthetic
runner only after tick reaches its poll boundary, waits for worker completion,
and checks recorded1/1, final preview/thumbnails, completed project and no job.
The old ordering failed deterministically with counters0/0 instead of1/1:
/tmp/scancraft-live-counter-red.log, exit101. No scanner opened. The fixture
helper accepts a private frame limit so this regression uses one existing
analytic pair; ordinary diagnostic startup remains100frames. Production native
acquisition is unchanged. Full corrected gate/results/commit follow. Only the
independent reviewer may verify/close014; original findings remain untouched.

Corrected complete frozen gate passed572Rusttests, formatting, strictalltarget
Clippy and releasebuild, exit0 (/tmp/scancraft-live-counter-final-gate.log).
New regression checks actual displayed cloud count as well as counters; no
threshold or lifecycle guard weakened. Source/docs fullcontext secret scanPASS.

SCAN-REV-014 correction committed/pushed as
beb962816ca817cd9b12f1ae13a04954fe1a1adb tree2882e56d4ae8e6c6a00eee176e4ffd8147de1de6.
CodeRabbit CLI incremental review versus9e8a21e completed3files,0findings,
majorPASS,exit0. Head/tree unchanged. Fresh includedcapacity1/billinginactive
confirmed before execution; no paidflags/consent. Numerical/GUI gate572tests
and regression red/green log hashes recorded in live-gui-review.json.
This provides requested developer evidence, not independent closure of014.

## Developer milestone — live-session provenance integration (WIP)

014 independently Resolved by Assignment113 acknowledged; developer does not
change the register. New live-session lineage addresses the mission’s retained
raw/project relationship without forcing fused sessions into single-pair or
frame-directory schemas. Added optional Editor metadata and ORPROJ08; legacy
02–07 remain readable. Live runner retains the Editor until raw/report fingerprints
exist, attaches references, saves project, then outermanifest. No fullcloudreload
or added exportclone is needed. Explicit CLI and GUI sourceverification reuse
existing guarded checker; loading never follows external paths.
Geometry focused2tests and live runner10tests passed, including new edit/save/
reopen/movedfolder/raw+cal+reporttamper/symlink checks. Full gate pending.
No hardware, physical binding, synchronization or accuracy claim; CRC notauthenticity.
No independent finding invented, no new library/version or copiedvendorcode.

Live-lineage final gate passed574Rusttests,fmt,strictalltargetClippy,release
exit0 (/tmp/scancraft-live-lineage-integrated-gate.log). Actual releaseexample
publishedORPROJ08; releaseCLI verify-source-bundle passed8artifacts (exit0),
modifiedraw failed(exit1), rawbytesrestored. Aggregate evidence
evidence/2026-10-27-live-lineage-runtime.json. Newexternaldependencies:none.
CodeRabbit preflight reports0includedreviews, nextslot~2796247ms; billinginactive.
No actualreview requested while unavailable and no paidconsent. Source review
remains pending; do not describe this feature as independently approved.

Live-lineage release GUI validation on exact source d30ae9e: existing private
authenticated X display harness opened ORPROJ08 and ran background explicit
verification. Screenshot inspected:8/8match, live-session panel,3-point cloud.
A separate copied bundle with alteredraw produced7/8match and
left.raptor:crc_mismatch, preserving cloud and storedreferences. Bothprocesses
closed; nocameraUSBfds. Originalproofbytesunchanged. No newproductionchange,
so574testgate applies to unchanged code; no redundant fullgate/review requested.
Aggregate/runtime screenshot hashes in live-lineage-runtime.json. CLIcapacity
stillpending from zero-capacitypreflight; draftPR14 notapproved.

## Developer milestone — complete referenced live-session export (WIP)

Separate dev/live-session-export branch based97c85f9 extends existing bounded
project_bundle copy/verify/replay/publication path to LiveSession references.
Only copied Editor metadata changes location; original state/bytes preserved.
GUI live panel enables explicit export and reports full referenced raw data;
legacy selected-frame labels remain accurate. No new format/dependency or
geometry change. Two new byte-identity tests and all8 project_bundle tests
passed; new fixtures deliberately dummyraw, not replay claims. Finalgate/runtime
proof pending. No independent finding/status changed; reviewquota remainsblocked
from priorzero-capacity preflight, no paid attempt.

Live-session export finalgate576Rusttests/fmt/strictalltargetClippy/releasePASS
exit0 /tmp/scancraft-live-export-full-gate.log. Actual releaseCLI exported8
sourceartifacts from valid syntheticlivebundle, verified8, replay--decode both
complete rawrecordings; all originalfileSHAs unchanged. ReleaseGUI startup
export onprivateauthenticateddisplay visuallyconfirmed success/fullrawlabel/
loadedcloud; resultingproject verifiedwithCLI, nocameraUSBfds. Fixturetest
dummybytes are not confused with this actualsyntheticrecording replay proof.
Runtimeaggregate live-session-export-runtime.json; physicalqualificationfalse.
CodeRabbit remains unavailable from priorzero-capacitypreflight; no paidreview
attempt. Source commit and draftPR follow; no mergeauthorization inferred.

## SCAN-REV-005 — cooperative capture follow-up, not closure

Accepted limitation remains: kernel ioctls/setup/restoration/joins have no
established shutdown bound. New native next_frame_until stop predicate checks
between dequeuecalls and caps poll by remaining frame deadline. LiveNative
passesControlstop; CLIsignal sets bothabortWorkControl andstop (publication
stillaborted). A stoppedincompletevideo counter distinguishes intentional
pre-metadata abandonment from validated recordings/processingdrops. Successful
reads retained; disconnect/othererrors preserved. No thread detachment/kill.
Three injected-read policytests pass; full579Rusttests/fmt/strictalltargetClippy/
releasePASS /tmp/scancraft-cooperative-stop-full-gate.log. NoV4L2latency or
physicalshutdown proof;005remainsOpen under reviewercontrol. Docs
COOPERATIVE_CAPTURE_STOP.md. No newdependency/vendorcode/hardwareoperation.
CodeRabbitCLI capacitypending as recorded; actualsource reviewnotclaimed.
PR13 oneApprequest6428805372 at22:26:32Z acceptedreply6428807089 at22:26:38Z;
reviewtriggered, NOTcompletion. No repeatedrequest/paidconsent.

## CR-LIVE-PAUSE-BOUNDARY — accepted GitHub App finding

Appreview5660000912 COMMENTED completed at478fc64 on22:32:48UTC. One minor
inline4383238987 identifies pause sampled beforeblockingrecv, causing one
post-request pair to reconstruct/fuse with stale state. Accepted aftersource
inspection. Correction9a071e2 samples sharedpause at completed-pair processing
entry anduses the same snapshot for Processor andphase. In-flight CPU work is
not retroactively interrupted; latercommands apply at nextpairboundary.
A deterministic rendezvous producer sets pause afterinitialRunning sample and
hands over a pair: oldcodefails Trackingvs paused(exit101); fixedcode skips
integration and resumes onfollowingpair. Fullsource13gate573tests/fmt/strict
Clippy/releasePASS. Source-onlypatch copied into14(7effbbf),15(448ab6c),16(b9f934a);
no PR/branchmerge or mainchange. Factualreply4383285896; threadnotresolved.
Appdocstringwarning12.39%vs80%,113functions/13files/10unsupportedskipped retained
asadvisory, not claimed a functionaldefect. Currentcorrectionreviewpendingquota.
IndependentAssignment129 verifiedd939 cooperative-stopdelta and33devicetests;
005Open retained. No newindependentfindingID/statusinvented.

Combinedgate initially failed due generatedartifactcachecontamination after
developer shared one targetdir with an older worktree; canonicalsource methods
werepresent. Cleared generatedworkspacepackage artifacts only, source untouched;
freshgate log /tmp/scancraft-pause-propagated-fresh-gate.log follows. All future
worktree builds use separatetargetdirs. Priorfailure log retained, not hidden.

After clearing debug andrelease workspaceartifacts separately, combinedcurrent
sourceb9f934a passed580tests/fmt/strictClippy/release,exit0. Log
/tmp/scancraft-pause-propagated-debug-release-gate.log; hashes inlive-gui-review
evidence. Cachefailure wasreproducedonlywithshared olderworktreeartifacts;
source unchanged by cleanup. NewactualCodeRabbitreview of correctedsource
remains pendingcapacity; App478completion isnot transferred to9a/b9.

## SCAN-REV-015 — independent resolution acknowledged

Reviewer136 independently verified9a071e2 andresolved015;137–139 verified exact
sourcepatch propagation to16/15/14. No developer registerchange. Existing005
remainsOpen; no approval/merge inferred.

## Developer milestone — measured paired map concurrency

Original Rust orchestration only: StereoImageMaps largeoutputusesone scoped
helper, callerprocessesothermap; same UndistortMap sampler/numericalmath unchanged.
Smallmaps/CPUhint<2/queryfailure/spawnerror staysequential. Explicit sequential
API retained. Bothownerscomplete before pairedresult; helperjoinedonright-error.
Three focusedtests +full583Rusttests/fmt/strictClippy/releasePASS. Benchmark
initially hadClippy let-and-return; fixedwithoutweakeninglint. Final actualAPI
20rounds+warmups, alternatingorder, eachfloatbit/maskmatchedreference. Fullsize
median89.930msseqvs43.795adaptive on16reportedCPUs; smallcaseunchanged. SingleCPU
childfallbackpasses; RLIMIT_NPROC0childthread-denialverifiedfallbackwith16CPUs.
No parent/userprocesspolicychanged. No hardware, accuracy or liveFPS claim.
Evidence docs/benchmarks/paired-remap-2026-10-27.json, PAIRED_REMAP_CONCURRENCY.md;
code image/stereo.rs +benchmarkexample. No dependency/license/versionchange.
Automatedreviewpendingincludedcapacity; notindependentlyapproved ormerged.

## Integrated review on c33c453 — findings evaluated

CodeRabbit CLI 0.9.1 completed 28 files versus corrected PR13 source 9a071e2:
3 minor findings, zero blocking findings, major gate passed, exit 0. Fresh included
capacity was 1 and billing inactive. The detached checkout head/tree stayed fixed.
The source tree is also unchanged by the later development-base synchronization.
Evidence: evidence/2026-10-27-integrated-review.json; raw output retained privately.

CR-INTEGRATED-DOC-SPACING accepted: separate prose, numbers and finding IDs in
acceptance, architecture and status reports; clarify recent public method notes.
Numerical values and executable behavior remain unchanged.
CR-INTEGRATED-SESSION-ID confirmation accepted; no defect or code change needed.
Processor::open calls ScanSession::new, which calls new_with_method and enforces
the identical nonempty, <=128-byte ASCII alphanumeric/"-_ ." rule before any
native Source opens. Native acquisition invokes Processor::open before session
locking/settings capture. Thus an ID rejected by LiveSession cannot pass this
constructor and fail only after acquisition. This is verified constructor ordering
and predicate inspection, not a new hardware experiment. No duplicate validator
or speculative refactor was added.
CR-INTEGRATED-LOCAL-PATHS accepted: remove worktree location fields from the current
public GUI review evidence. Commit IDs, trees and log hashes retain verification
value. Earlier development commits retain the historical path strings; no Git
history rewrite or credential disclosure is claimed.

## SCAN-REV-016 — accepted integration correction, verification running

The branch had one pause regression, but GitHub's PR14 merge snapshot da4d79a
contained it twice, causing E0428. Development bases were synchronized in order
14→15→16→17; the automatic PR14 candidate's duplicate was replaced with the exact
preexisting single-definition branch file. Each resulting development commit has
an unchanged full source tree and now contains its base in its ancestry. No PR or
protected-main merge, history rewrite or coverage removal occurred. GitHub's new
PR14 integration 4cc519c has tree 379ecabd, identical to the previously reviewed
branch tree, with one canonical regression. An isolated checkout is running the
full gate on that exact integration revision using its own target directory.
Only the independent reviewer may verify and close SCAN-REV-016.

SCAN-REV-016 exact corrected integration4cc519c passed the full frozen gate:
575 Rust tests, formatting, strict all-target Clippy and release build, exit0.
Its source tree remains379ecabd with one regression definition. The build used
an isolated checkout and its own target directory; no shared-cache substitution.
Evidence and gate-log hash are recorded in integrated-review.json. This is
developer verification; finding status remains under independent reviewer control.

Documentation correction0570a643eb8dcb5566d89fbbc97b6db7d4de7450 passed an
incremental CodeRabbit review versus c33c453: 10 files, zero findings, major gate
passed, exit0. The detached checkout stayed clean with unchanged head/tree.
Fresh included capacity and inactive billing were confirmed before execution.
No numerical source changed; existing 583-test code gate remains applicable.
Metadata-only review evidence needs no unchanged paid/repeated review loop.

Independent reviewer Assignment156 has resolved SCAN-REV-016 after inspecting
exact4cc519c and successful integration CI40417772893/check121904268081 (575 tests),
plus push40417764389/check121904240273. Developer acknowledges that independent
resolution without changing findings. SCAN-REV-005 remains Open and006 Under Review.

## Development milestone — planar calibration initializer

No new independent finding identifier is assigned. In response to the continuing
SCAN-REV-006 qualification gap, add software-only native planar pinhole
initialization as a prerequisite, not a claimed resolution. Source: calibration
src/planar.rs and module export; synthetic example/reference tool and method
documentation. Six Rust regressions and three independent OpenCV/NumPy cases
pass, including known skew, poses, degeneracy and metric-scale/origin invariance.
No hardware/physical calibration, active record installation or device workflow
changes. Distortion, nonlinear refinement and actual Raptor calibration remain
incomplete. Full workspace gate passed 589 Rust tests, formatting, strict all-target Clippy
and release build (exit 0). The reference checker also rejected an intentionally
perturbed focal length (exit 1), preventing a vacuous success claim. Source/log
hashes are recorded in the reference JSON. CodeRabbit CLI review is complete;
its disposition follows. Only the independent reviewer can change finding status.

CR-PLANAR-DOC-ROUNDING accepted: the exact OpenCV discrepancy
0.00021100840581311786 pixels rounds to 0.000211, not 0.000212. Corrected
docs/PLANAR_CALIBRATION_INITIALIZER.md only; numerical source and retained
measurements are unchanged. CodeRabbit CLI0.9.1 reviewed immutable44a2630 versus
31cfbe1:11 files, one minor finding, zero blocking findings, major gate passed,
exit0. Full source-context/diff secret scans passed; fresh included capacity2 and
inactive billing were confirmed. Evidence:2026-10-27-planar-initializer-review.json.
The existing589-test/fmt/Clippy/release result applies to unchanged Rust source.
No paid/repeated unchanged code review or physical calibration performed.

CR-PLANAR-REVIEW-STATUS accepted: replace the initial pending-review sentence
above with the completed CLI state. The incremental documentation review on
3603af2 versus 44a2630 completed: three files, one minor status inconsistency,
zero blocking findings, major gate passed, exit 0. This final follow-up changes
review metadata only; no Rust, test, numerical method or measured value changes.
No third unchanged code review or functional test repetition is warranted.
GitHub App review and owner merge approval remain separate. SCAN-REV-006 is
unqualified/under independent review and SCAN-REV-005 remains open.

## Development milestone — native Brown5 refinement

Advances part of SCAN-REV-006's tooling gap; does not resolve or reclassify the
qualification finding. New refinement module fits intrinsics, Brown5 and poses
with analytic Jacobians, bounded block Schur LM, residual/stationarity/information
and numerical monotonicity checks. Diagnostic results never install calibration.
Workflow planar_calibration and CLI calibrate-planar provide bounded input, exact
request retention and atomic no-overwrite publication with existing primitives.
No new runtime dependencies or device/firmware/physical calibration changes.
Seven math and two workflow tests, independent three-case OpenCV/NumPy checks and
release CLI preservation/rejection checks pass. Exact workspace/review evidence is
recorded separately by source revision. Finding status belongs to the reviewer.

The combined source gate passed 598 Rust tests, formatting, strict all-target
Clippy and release build (exit 0). Source/log and synthetic runtime hashes are
recorded in 2026-10-27-brown-calibration-reference.json. These are software gates,
not hardware qualification.

Brown5 milestone review: CodeRabbit CLI 0.9.1 completed immutable source
7f1ad202fe4b4039d632e6727bcf82e2af19f7aa versus 761036d: 18 files, zero
findings/blocking findings, major gate passed, exit 0. Full context/selected-diff
secrets scans passed, fresh included capacity was 1 and billing inactive.
Before/after head and tree were fixed; source attestation is operator-recorded.
No suggestion was accepted blindly and there are no findings to dispute.
Evidence: 2026-10-27-brown-calibration-review.json.

An isolated mutation removing the cancellation latch made the targeted transient
request regression fail with exit 101: a cancelled fit incorrectly returned Ok.
The production 598-test gate passes with the latch. This is an intentional
mutation control, not a newly discovered historical production defect. Separate
checkout/target and source/log hashes are retained in the reference evidence.
This follow-up changes evidence only; Rust and numerical parameters remain at
the reviewed source. SCAN-REV-005/006 status remains under reviewer control.

GitHub App completed exact 3b4dff705d661e3d56bb547b4c70d3384695a07a:
run 2cd1dfda-ed4a-4b14-80bf-13e949fdc170, request 6430863767, finished reply
6430868796 and summary 6430777570. Exact source/covered markers both match with
kind reviewed; no actionable comments, inline comments or review threads. There
is no formal APPROVED review object and no owner merge consent. The summary
explicitly confirms included allowance used (one per hour, zero remaining). No
repeated request or billing consent. Exact integration CI log confirms 598 tests
and the new named calibration regressions at merge checkout 4d002be.

CR-APP-BROWN-DOCSTRING-COVERAGE — advisory evaluated; blanket threshold disputed
as a readiness gate, warning retained unchanged. The App reports 5.71% versus
80%, 35 functions across eight files, 11 unsupported files skipped. Both new public
fitting/workflow entry points describe their contracts/limitations, and the method,
request schema, numerical policies and provenance have dedicated documentation.
The function metric also counts private/test helpers and the touched CLI dispatcher;
it does not measure numerical correctness, public API adequacy or physical
qualification. No threshold/configuration is lowered and no boilerplate is
generated to inflate a score. Additional helper documentation can accompany
specific interface or maintainability improvements. This is an App advisory, not
a new independent SCAN finding. Evidence-only follow-up leaves reviewed Rust/tool
source unchanged. SCAN-REV-005 remains open and 006 unqualified/under review.

## Development milestone — fixed-intrinsic stereo calibration

Advances the SCAN-REV-006 tooling gap without resolving/reclassifying qualification.
New stereo_fit jointly estimates a shared rig and target poses while holding
supplied lenses fixed. It uses bounded native inverse/homography seeds, shared
Brown/pose derivatives and generic six/ten-parameter Schur blocks. A dense-system
comparison verifies both consumers. Existing mono reference fixtures and all three
CLI artifact sets are byte-identical. Workflow calibration_bundle shares existing
atomic publication; public mono provenance path is preserved by re-export.
New stereo_calibration workflow and calibrate-stereo-extrinsics CLI never open or
configure hardware or install calibration. 607 Rust tests/fmt/strict Clippy/release
pass; three OpenCV/NumPy stereo cases, held-out synthetic triangulation, release
CLI preservation/rejection and a corrupted-transform reference control pass.
No new dependencies/licenses/runtime, firmware, physical calibration or source
accuracy claims. Exact source/review evidence is recorded separately; only the
reviewer controls independent finding status.

CR-STEREO-ERROR-SPACING accepted: add the missing space and inclusive upper bound
in stereo_fit::initialize's count diagnostic (expected 4..=4096). No mathematical
behavior changes. Actual CLI invalid-count request exits 1 with the corrected
message and no published output. Source ff5b17b versus 26987bb completed CodeRabbit
CLI review: 24 files, one minor finding, zero blocking findings, major gate passed,
exit 0. Fresh included capacity 2 and inactive billing were checked; full context/
diff secret scans passed and detached source stayed fixed. Corrected full gate
passes 607 tests/fmt/strict Clippy/release; fixture bytes and all three stereo/three
mono release artifact sets remain unchanged. Exact hashes and scope are in the
stereo-calibration-reference/review JSON reports. No independent finding status
changed. GitHub App review is separate; next manual free window after 01:44 UTC.

The incremental stereo correction review completed at
42d0b40b9833b441a32d0fde37e7fac8b54739a3 versus ff5b17b: four files, zero
findings, major gate passed, exit 0. Fresh included capacity 1/inactive billing
and complete context/diff scans were verified; checkout head/tree stayed fixed.
Final evidence-only follow-up leaves this corrected Rust/tool source unchanged.
No repeated unchanged code review or paid usage. When initial CLI evidence was
recorded, App review had not yet been requested; its included window followed the
PR19 review. The completed App result is recorded below. Skip status alone is
not review completion.

App20 completed exact ce5d66a19622948b949a749faa86d3449dbb9cab: run
a5e01b1d-20b4-4ba5-b47d-a0e8f5a745bb, request 6431680969 at 01:45:59 UTC,
finished reply 6431682290, summary 6431458213. Exact source/covered markers match
with kind reviewed; no actionable/inline comments or review threads. No formal
approval or owner merge consent is inferred. Included allowance was used (one per
hour, zero remaining); no paid consent or repeated request.

CR-APP-STEREO-DOCSTRINGS — advisory evaluated; blanket threshold disputed as a
readiness gate, warning retained. The App reports 39.53% versus 80%, 43 touched
functions across 13 files, 12 unsupported files skipped. New public entry points
and shared projection/normal/publication helpers document their contracts; the
method guide explains frames, equations, bounds, limitations and provenance. The
aggregate also counts private/test helpers and existing touched code; it does not
measure mathematical correctness, public API adequacy or physical qualification.
No threshold/configuration is lowered and no boilerplate is generated to inflate
the score. Specific maintainability improvements can receive targeted docs. This
is an App advisory, not a new independent SCAN finding. Final follow-up changes
review metadata only; corrected source and the 607-test evidence remain unchanged.
SCAN-REV-005/006 remain under independent reviewer control.

## Developer milestone — passive SCR observations

Developer-originated work, not a new independent finding. Source commit
78f92ac adds `scancraft-device/src/scr_observations.rs` and CLI
`recording-scr` in `scancraft-cli/src/recording_scr.rs`/`main.rs`.
Accepted implementation priority: make the independently established Raptor SCR
numeric tag contract observable before introducing any mode-dependent frame
selection. Retain all per-record disagreements and unknown tags. Reject malformed
headers with checked slices; preserve capture acceptance, recording format and
existing device state. Provenance and qualifications are documented in
`docs/RAPTOR_SCR_OBSERVATIONS.md`. No proprietary implementation is included.

Validation: `tools/check.sh` passes formatting, strict all-target Clippy, 611
workspace tests and release build. Four new focused tests cover all numeric tags,
byte-three overrides, PTS present/absent, missing/unknown/conflicting SCR records,
truncated headers, device error flags and truncated recording footer. The release
CLI read four private genuine Raptor recordings through their verified footers:
400 frames total, 400 SCR records, numeric type 1 throughout, zero missing,
unknown or conflicting known tags. Private captures and raw results are excluded
from commits and review uploads. This is hardware-recording replay evidence,
not a fresh hardware run, mode-name validation or geometric accuracy result.

SCAN-REV-005 and SCAN-REV-006 remain outstanding under independent reviewer
control; this passive diagnostic does not resolve acquisition shutdown or physical
qualification. No firmware, physical calibration, paid services, protection change
or owner merge approval is involved. Automated review outcome follows separately.

CR-SCR-UVCH-LENGTH — accepted. The initial CLI review of 78f92ac versus
9f853105 completed across five files, with one minor finding and a passed major
severity gate. Verified against Linux v6.17 `uvc_video_decode_meta`: standard UVCH
copies `2 + PTS*4 + SCR*6` bytes while retaining the wire length byte. Corrected
both the new SCR parser and existing `scancraft_core::uvc_pts`; otherwise the
recording reader would reject valid extended headers before diagnostics ran.
This deliberately expands acceptance to correctly formed standard UVCH records
with extended declared wire lengths; malformed standard lengths still fail.
Added consecutive extended-header tests to both parsers, including conflicting
PTS detection, and changed the CRC-backed CLI fixture to extended lengths.
This does not resolve independent acquisition shutdown/optics findings.
Corrected full-gate and incremental-review results follow when completed.

Correction commit 106a8a32b90caaeec1276025c2ae511c41882f74 passes the full
offline gate: formatting, strict all-target Clippy, 613 tests and release build.
The corrected release revalidated all 400 saved genuine Raptor frames; a
truncated recording and missing CLI argument return nonzero without partial JSON.
Initial gate failure exposed a malformed absent-PTS test fixture retaining PTS
bytes; the corrected fixture removes those bytes and still verifies both legacy
and current readers reject missing PTS before indexing. No check was weakened.

Incremental CodeRabbit CLI review of 106a8a3 versus 78f92ac completed across six
files, zero findings, major gate passed, exit 0. Both reviews used included
capacity with usage billing inactive; full tracked source context and complete
selected diff secret scans passed before each upload. Reviews ran on isolated
fixed commits without private/reviewer-owned files. No redundant third review
is requested for this outcome-only documentation. GitHub App review and exact
PR integration CI remain separate verification steps; owner merge consent is
not inferred from either. Independent finding statuses remain untouched.

SCAN-REV-017 — accepted. This independently identified UVCH stride defect
matches CR-SCR-UVCH-LENGTH above. Correction commit
106a8a32b90caaeec1276025c2ae511c41882f74 changes the core PTS and device SCR
parsers, associated recording fixture and extended-header regressions. Full gate
passes 613 tests, formatting, strict Clippy and release; corrected-source
CodeRabbit review reports zero findings. No remaining claim that extended wire
length equals copied metadata length is intentional. Vendor-specific metadata
formats remain outside the standard UVCH parser contract.

Independent reviewer assignment208 has now inspected the exact correction and
response-only descendant eda0347, run 57 focused tests and an external reproduction
harness, and marked SCAN-REV-017 Resolved. This paragraph records that independent
verification; development did not alter the reviewer's finding or its status.
Physical reconstruction and whole-application shutdown bounds remain unqualified.

PR21 GitHub App review completed for exact eda0347b21361144d6c917c76aeeaed336edca8c:
run cb6b7864-ac17-47bf-be9b-a833883177d0, summary comment6433274923 and
finished reply6433279946, following manual request6433278727. Summary states no
actionable comments; exact source/covered markers both identify eda0347 with
kind reviewed. API inspection found zero inline comments, zero review threads
and no formal approving review. App check is successful. This is completed
review evidence, not owner merge approval. The fetched PR integration object
9b83bb4d9aec1a1212dbf27c2edcb0b98d25caec has tree
9684f9852c2d50b09d7dd144ab103eb3c1b376b3, identical to eda0347. Integration
CI run40446307260 is still running at this observation; no CI pass inferred.

CR-APP-SCR-DOCSTRINGS — advisory evaluated; blanket readiness threshold disputed.
The App reports 16.67% versus 80%, 12 functions across six files, two unsupported
files skipped. The public observation/inspection entry points document framing,
error behavior, unknown/conflicting tags and bounded streaming; the method guide
documents provenance and hardware limits. Most new functions are synthetic test
cases or fixture helpers. The aggregate is not a physical qualification or
correctness measure. No threshold/configuration is weakened and no redundant
source review is requested for this evidence-only follow-up. Specific missing
API contracts remain eligible for targeted documentation work.

Developer hardware-recording evidence follow-up (no new source change): release
`recording-scr` additionally verifies four private line-control/off-check files,
60 frames per camera per state. Trial120 frames all numeric type2; off120 frames
all type1. Four derived single-frame recordings agree but are not independent
samples. The trial manifest reports property151 enabled/read back and restored,
while physical emission, wavelength and calibration binding remain unverified.
Recorded the qualified comparison in `docs/RAPTOR_SCR_OBSERVATIONS.md`; no mode
activation, vendor code import, new accuracy claim or SCAN-REV-006 closure.
Existing 613-test/source review evidence remains for unchanged Rust code.

SCAN-REV-005 — accepted, preparation in progress (uncommitted developer work).
Separated preview session ownership into `scancraft-app/src/capture_session.rs`
from `main.rs` GUI/channel handling. The session now explicitly calls device
close after ordinary recording setup/write or acquisition failure and combines
operation and restoration errors; publication still requires successful close.
Two hardware-free regressions verify close invocation/order on errors and that
cleanup failure cannot return successful publication state. Full offline gate
passes formatting, strict all-target Clippy, 615 tests and release build.

This does not implement process isolation or bound shutdown. Both preview and
native live-scan UI still have device-owning worker joins; synchronous kernel
calls remain. `docs/CAPTURE_PROCESS_ISOLATION.md` records the required helper
ownership, bounded IPC, durable failure evidence, lock retention and validation
contract. No claim that the independent finding is corrected or closed. No
implementation commit, CodeRabbit review or PR is claimed for this unfinished
work; preserve it on `dev/capture-session-isolation` for continued implementation.

SCAN-REV-005 — preview helper milestone (partial correction, not closure).
`capture_process.rs` now starts the same executable in a validated internal
helper mode before GUI initialization. Only the helper invokes
`capture_session::run` and owns scanner descriptors, locks, recording and cleanup.
`capture_ipc.rs` bounds binary frames and JSON results and rejects malformed or
incomplete streams. The parent monitor uses nonblocking pipe reads with bounded
batches; cancellation closes stdin and applies a two-second software grace policy.
Timeout reports restoration unconfirmed with helper PID and private 0600 log path.
A waiter allocated before child launch reaps surviving children independently;
no kill/reset or device-owning thread detachment is used. The GUI no longer
labels a failed/pending cleanup worker as a stopped scanner.

Evidence: full `tools/check.sh` passes formatting, strict all-target Clippy,
624 tests and release build. Real device-free child processes test frame/result
transport, nonzero and missing-result exit, cancellation during a stall, and a
lock inherited by a stalled child. The monitor worker returns while that lock
remains held; releasing the fixture's cleanup marker makes it acquirable again.
Codec tests cover fragmented messages, corrupt lengths and truncated/trailing
completion. A release invocation with unsupported helper interface255 exits1
without stdout or GUI/device initialization. Private diagnostic permissions and
retention across IPC loss are tested. No real scanner/driver-stall qualification
is inferred. `docs/CAPTURE_PROCESS_ISOLATION.md` records the ownership contract.

Native `live_ui` still runs acquisition in its existing thread and joins on Drop;
other synchronous device-discovery calls are not newly isolated. Process spawn,
log filesystem operations and scheduler latency have no hard real-time bound.
The preview-only synthetic results therefore do not establish whole-application
bounded shutdown or resolve the independent finding. No hardware, firmware,
physical calibration, paid service or merge action occurred. Automated review
for this meaningful diff follows separately; existing independent statuses remain.

Preview source commit283df43 completed CodeRabbit CLI review versus5336d8f:
six files, one minor finding, zero major/critical findings, major gate passed,
exit0. Review ran on an isolated fixed commit after full context/diff secret
scans; included capacity and inactive billing were verified.

CR-PREVIEW-IPC-FLUSH — accepted. The helper's stdout may buffer a partial frame,
so a completed capture must not depend on the next frame to become visible to
the parent. `capture_ipc::write_frame` now explicitly flushes after writing the
frame; the existing helper transport-error path receives flush failures too.
Added a buffered-writer regression that decodes the frame from underlying bytes
before any second frame/completion, and a writer that fails only on flush to
verify BrokenPipe is propagated. No unmeasured latency figures are claimed.
Corrected full-gate and incremental review availability are recorded separately.

Corrected flush source passes full formatting, strict all-target Clippy, 626 tests
and release build (`tools/check.sh`). The included review allowance is currently
zero, with next available capacity reported in about24 minutes; usage billing is
inactive. No credits, charges or redundant rate-limited review request is used.
A fresh corrected-source gate is pending included availability, not claimed as
completed. This preview milestone is published as draft work; native live-scan
isolation and whole-application shutdown qualification remain outstanding.

SCAN-REV-005 — native live-scan process boundary in progress; status unchanged.
`live_process` now owns the parent-side supervision path used by native
`LiveUi::start_native`. Its child owns the full `scancraft_acquisition::scan`,
including session lock, settings lease, both sources, restoration, verification
and publication. The existing child waiter, nonblocking pipe configuration and
private diagnostics are shared with preview supervision. Startup diagnostics
include helper PID and mode; timeouts describe acquired resources conditionally,
without asserting a lock was acquired before initialization completed.

Graceful Stop sends a latched command without cancelling WorkControl or starting
an abort deadline. Explicit abort/control EOF cancels WorkControl and starts a
separate two-second software grace policy. A prolonged ordinary-close wait offers
an explicit abort action retaining partial data. Failure-intent reset now belongs
to the root close handler itself, preventing a pending retry from bypassing its
failure handling. Native worker Drop joins an IPC supervisor, not a thread that
owns scanner cleanup. Synchronous discovery and filesystem/process/scheduler
bounds remain unqualified; no whole-application hard deadline is claimed.

`Control::display::DisplayUpdate` validates a display-only transfer: at most
50000 finite cloud points and bounded thumbnails with matching pixel counts.
Remote display controls cannot run a local scan; local controls reject imports;
parent pause/stop commands are never overwritten. Skipped parent replacements are
counted in addition to child skips. Progress serialization has a16MiB cap during
writing/before parsing; final IPC is a bounded summary, while full per-pair data
stays in the engine's report.json. The synthetic session is extracted to
`live_fixture` to avoid a process/UI dependency cycle. No engine mathematics,
scanner control values, recording format or calibration activation is changed.

Evidence so far: full tools/check.sh passes formatting, strict all-target Clippy,
637 tests and release build. Real child/control tests distinguish pause/resume,
graceful stop beyond abort grace, EOF abort and stalled-child timeout without
kill. New DTO/codec tests reject invalid thumbnails, nonfinite/excessive clouds,
local import, unknown command fields, oversized JSON and mismatched frame sizes.
The actual parent/helper/engine integration diagnostic publishes a synthetic scan,
verifies both CRC-backed recordings through footers (two frames each), retains
synthetic labels and false hardware qualification flags, and rejects overwriting
an existing target. The release diagnostic also passes; final source proof and
review evidence are recorded separately. No actual Raptor disconnect/stall or
physical accuracy qualification is claimed.

Reviewer assignment220 independently verified the earlier preview-only commits
283df43/83daa8e with26 focused capture tests and retained SCAN-REV-005 Open. This
is acknowledged; those results do not independently verify the new live changes.
Only the independent reviewer may verify/close the finding. No paid usage,
firmware, physical calibration, protection bypass or owner merge approval occurs.

Native extension source5d07769 completed CLI review versus283df43 across eleven
files: one major verification/coupling concern, one minor stdout-lock concern;
major gate blocked, exit2. The blocked result is retained, not treated as a pass.

CR-LIVE-DISPLAY-REJECTION — accepted. Both production offer paths in
`live_stereo/runner.rs` use `Processor::preview_cloud`, which delegates to
`Session::preview_cloud(50000)` and `VoxelFusion::preview_cloud`'s stride over
its own voxel set. Thus current offers use a bounded fused subset rather than a
per-pair pre-fusion cloud. Nonetheless, validation and serialized-size rejection
are display concerns and must not cancel recording/publication. Preparation is
now separate from pipe writes. Invalid/oversized previews are omitted with a
small counter-only update, accumulated skip count and private diagnostic; final
summaries expose helper_omitted_previews. Only actual pipe write/flush failure
requests WorkControl cancellation. Engine geometry/output validation remains
independent and unchanged. Regressions inject invalid thumbnails and oversized
tracking JSON, verify no scan cancellation and retained counters, verify valid
geometry/thumbnails survive transfer, and confirm real BrokenPipe requests abort.
The actual synthetic process integration asserts zero omissions on its healthy
engine path. No observed physical-scan failure is asserted.

CR-LIVE-STDOUT-LOCK — verification/advice accepted; no offending diagnostic calls
needed relocation. Static search of acquisition, live runner/control, device,
depth, image, calibration and geometry source found no print!/println!/dbg!
calls; existing cleanup diagnostics use eprintln! to stderr. The parent's
synthetic diagnostic println occurs after child supervision, outside the engine
call. The sender now holds stdout only for a fully prepared packet write/flush,
not for the scan lifetime or encoding/sleep. Stdout remains reserved for binary
IPC; future engine diagnostics must use stderr. This prevents the proposed
scan-lifetime lock dependency without claiming arbitrary stdout text is valid IPC.

Corrected full tools/check.sh passes formatting, strict all-target Clippy,
640 tests and release build. Corrected-source numerical/physical accuracy is not
claimed from these transport tests. Fresh incremental review and exact commit
coverage follow separately; no independent finding severity/status is changed.

SCAN-REV-018 — accepted. Exact correction
beac45e04bbae5010bb4c60eca28c63545559d52 separates display preparation from
transport writes, omits rejected previews while forwarding counters/diagnostics,
counts omissions, and reserves scan abort for pipe write/flush errors. Source
files are capture_ipc.rs/live_process.rs and the actual-process integration test;
policy and regression details are recorded above under CR-LIVE-DISPLAY-REJECTION.
Full offline gate passes640 tests, formatting, strict all-target Clippy and
release. The corrected release synthetic diagnostic passes with zero omitted
previews and retained false qualification flags. Independent assignment225's
exact oversized-presentation case is covered by the new injected-budget
regression; no observed hardware failure or physical qualification is asserted.
Only the independent reviewer may verify and close018; its last-read status is
Open.005 also remains Open, and actual driver/disconnect qualification is pending.

Fresh incremental CLI review of exact beac45e versus5d07769 completed across five
files: zero findings, major gate passed, exit 0. This is the third CLI pass for
this feature; no unchanged fourth review is requested. Initial blocked review
history is preserved. Complete corrected context/diff secret scans passed after
rebuilding them in private developer storage; the earlier interrupted /tmp-quota
scan is not counted. Included capacity and inactive billing were checked before
reviews, with no paid credits/charges. GitHub App review and integration CI are
separate steps; owner merge consent is not inferred.


CR-PR22-PREVIEW-GRACEFUL-STOP (GitHub comment4385482626) — accepted.
The App review of1b1c8cf correctly identified that ordinary preview Stop/close
started the abort deadline, permitting slow disk finalization to return an
unconfirmed error before successful publication. A successful GitHub check was
not a clean review: the review posted this actionable minor finding.
Correction in capture_process.rs/main.rs separates the graceful-stop flag from
explicit abort. Ordinary Stop closes helper stdin and continues draining until
completion and actual exit; explicit abort, lost consumer or App Drop retains
the supervision grace and honest cleanup-unconfirmed diagnostics. The slow-close
UI offers abort for preview recording as well as native live scanning.
A real device-free child waits for EOF, delays finalization beyond a shortened
abort grace, then emits success; the supervisor must return that success. Existing
stalled-child and lock-retention abort regressions remain. Validation/commit
evidence follows when complete. No physical driver or slow-disk qualification
is claimed. The three-pass CLI budget for this feature is exhausted; no fourth
review or paid request is authorized by this correction. New review readiness
must remain pending until appropriate included review is available within the
recorded workflow. Independent reviewer findings remain untouched.

Correction validation: tools/check.sh completed successfully: formatting, strict
all-target Clippy,641 tests and release build. Private gate log:
preview-graceful-stop-gate.log in developer artifacts. Correction remains local
pending follow-up review; the previous clean review covers beac45e, not this edit.


SCAN-REV-019 — accepted. Assignment230 independently identifies the same ordinary
preview Stop/abort coupling as CR-PR22-PREVIEW-GRACEFUL-STOP above. Exact local
correction224621b491a6a1811d184ac30f619e09d2713754 and641-test gate are recorded
above; independent verification and closure belong exclusively to the reviewer.

Development follow-up for SCAN-REV-005: discovery_process.rs moves the remaining
GUI-thread discovery/QUERYCAP into a supervised read-only child; main.rs polls
and gates conflicting starts. capture_process.rs exposes existing supervision
within the crate without changing its policy. No hard whole-application shutdown
bound is claimed: Discovery Drop joins the IPC monitor, and launch/filesystem/OS
scheduling are unbounded. Actual driver stalls and calibrated hardware remain
unqualified. Independent005 status is not changed by development.

Discovery follow-up full offline gate passes644 tests, formatting, strict
all-target Clippy and release build. Initial new test used the wrong Worker::join
return type; corrected to observe the actual polled result before this final
gate. Complete tracked source plus intended new module and selected diff pass
gitleaks; protected/private untracked files are excluded. CLI review pending.


CR-DISCOVERY-CHILD-LIFETIME — accepted. First CLI review of6cd439e versus224621b
completed with one major finding and blocked gate (exit2). Cancellation timeout
could clear the UI discovery guard while a probe child was still alive, allowing
repeated stuck children. Correction separates ordinary Stop from teardown abort.
Normal Stop keeps supervising; after any protocol/supervision error, the worker
retains the pending guard until actual child exit. Only Discovery Drop may stop
waiting during application destruction. No forced kill or device reset occurs.
New real child regressions check retained waiting until exit and teardown that
does not falsely claim exit. Discovery can remain pending indefinitely on a stuck
QUERYCAP; responsiveness does not establish a hard shutdown or hardware bound.
The correction gate completed successfully; fresh review remains pending. The initial blocked review is
retained in private developer artifacts/discovery-process/coderabbit.ndjson.

Correction gate completed successfully:646 tests, formatting, strict all-target
Clippy and release build. Fresh incremental review is still pending; this does
not convert the initial blocked CLI result into a pass.


SCAN-REV-020 — accepted. This is the independently recorded counterpart of
CR-DISCOVERY-CHILD-LIFETIME. Exact correction
c84655d467130591db875bb6523202e5c05e2180 and646-test full gate are recorded above.
Independent assignment233 verified five isolated discovery tests and retained-
child semantics and marks020 Resolved; development did not edit its findings.
Fresh corrected-source CodeRabbit review remains pending included capacity.
Independent assignment231 likewise verified224621b and marks019 Resolved.
Neither resolution closes005 or establishes physical scanner qualification.

Developer milestone (not an independent finding): offline seeded target-corner
measurement preparation. New image::corners solves gradient-orthogonality normal
equations with bounded Gaussian windows, explicit rank/contrast/displacement/
border/convergence failures and cancellation. Shared workflow::target_corners
and CLI refine-target-corners preserve exact PNG/request, association order,
refined PlanarView, diagnostics and CRC manifest in no-overwrite output. No
calibration is fitted/activated and no hardware/configuration is touched.
Independent mathematical implementation references OpenCV cornerSubPix docs;
no upstream code/assets copied and no new production dependencies.
Source: image/src/corners.rs, image/examples/corner_reference.rs,
workflow/src/target_corners.rs, their lib.rs declarations, cli/src/main.rs,
tools/compare_target_corners_opencv.py; design/provenance/limitations documented
in docs/TARGET_CORNER_REFINEMENT.md plus status/roadmap.
Validation: full tools/check.sh passes652 tests, formatting, strict all-target
Clippy and release; twelve synthetic cases match independent NumPy/OpenCV
under fixed tolerances. Actual release CLI retained exact inputs/association,
verified all artifact lengths/CRC, refused overwrite and left saved bytes intact.
Private evidence lives under developer artifacts/corner-refinement (full-gate.log,
fixture.json, reference-report.json, runtime-proof.json). Fresh CodeRabbit review
is pending; no paid capacity or clean-review claim. Full-board detection/indexing,
real corner accuracy, uncertainty, camera/mode binding and native hardware
geometry remain incomplete. Physical calibration stays deferred.


Discovery correction CLI review of c84655d versus 6cd439e completed: three files,
zero major findings, major gate passed, exit 0. Two minor findings accepted:
CR-DISCOVERY-TEST-TIMER: the child could start its100ms sleep before the test
timer. Its script now waits for stdin EOF, which retain_until_exit requests
after timer initialization, eliminating that scheduling race. This changes only
the fixture, not production discovery ownership.
CR-DISCOVERY-STALE-STATUS: corrected the previous pending-gate sentence to reflect
the completed646-test gate, retaining review history rather than altering
independent findings. Fresh validation of this fixture correction follows. The
second CLI result is retained in private discovery-process/correction-coderabbit.ndjson;
it covers c84655d, not this later minor edit. No paid credits were used.


Publication and follow-up evidence:
- Preview Stop correction224621b is published in PR22. Both new GitHub Rust
  checks passed; the CodeRabbit App check skipped because the base is a
  nondefault development branch. Skip is not verification. Evidence reply:
  https://github.com/chevydan6/ScanCraft/pull/22#discussion_r4385599810 .
  The existing thread remains for independent verification; no merge occurred.
- Discovery final minor correctionce5600022421008cbcf0cc429ddbce9fdeb65036
  passes the exact branch's full646-test gate and five focused discovery tests.
  Published draft PR23: https://github.com/chevydan6/ScanCraft/pull/23 .
  Both GitHub Rust checks passed; App skipped for the draft. The preceding
  c84655d CLI review passed major severity with two accepted minor findings;
  final minor edit remains pending fresh included review, not silently marked
  reviewed. No paid credits or additional unchanged review was requested.

Developer build-evidence correction (not an independent finding): sharing the
Cargo target directory between checkout paths left an older release workflow
library/executable in the normal output. A nominally fresh build did not prove
that the new command existed. Actual command verification caught the mismatch;
subsequent shared-directory gates passed652 debug tests but failed release and
are explicitly not full passes. Targeted cleaning initially left artifacts from
the other checkout. Suspect remaining ScanCraft release artifacts were preserved
in private quarantine (83 entries; relocation manifest), not confused with new
source outputs. All source, reviewer files, captures and external dependencies
were preserved. A never-shared build directory then passed the full source gate
at cac7f5b:652 tests, formatting, strict all-target Clippy and release build.
The isolated release CLI successfully published a new measurement bundle,
verified exact inputs/associations and all lengths/CRC, and refused overwrite.
Rebuilt normal-path binaries passed those same actual CLI checks; the normal app
also passed its explicit synthetic parent/helper/engine diagnostic with zero
omitted previews and false physical qualification. No actual GUI/GPU launch or
hardware qualification is inferred. Documentation now requires per-checkout
build isolation. Private evidence: corner-refinement/isolated-clean-gate.log,
isolated-runtime-proof.json, restored-current-release-proof.json,
restored-app-synthetic-summary.json and stale-release-cache/relocation.json.
The corner algorithm itself is unchanged from92374f8; its first CodeRabbit review
remains pending included capacity. This documentation-only follow-up does not
claim new source coverage from old reviews or alter independent finding status.


SCAN-REV-003 — accepted; partial workflow correction. New read-only
`tools/review_readiness.py` reports actual current-head formal CodeRabbit review
coverage separately from a successful/skipped badge. It requires known completed
publication batches, a stable head snapshot, complete conversation evidence and
no unresolved threads; draft/closed PRs and changes-requested reviews block.
It performs only GitHub reads and always reports merge_authorized=false. This
is a diagnostic, not an enforced authorization gate, and does not resolve
SCAN-REV-002 credential separation or prove CI/code correctness. Unknown formats
and evidence failures report not ready. No branch protection or existing review
mechanism was changed. Only the independent reviewer may verify/close003.
Ten offline synthetic tests pass, including stale/spoofed/pending/dismissed
reviews, incomplete batches/pagination, changed heads, drafts and unresolved
outdated conversations. Actual read-only runs correctly report PR22 not ready
(no current-head completed App review; one open thread) and PR23 not ready
(draft; no completed current-head App review), despite passing Rust checks.
Source: tools/review_readiness.py, tools/test_review_readiness.py and
DEVELOPMENT_REVIEW.md. Full repository gate/fresh CodeRabbit review are pending;
no paid capacity, comment posting, thread resolution or merge occurs in this tool.

Readiness utility validation: full repository gate completed successfully with
652 Rust tests, formatting, strict all-target Clippy and release. Ten Python
regressions pass, including contradictory publication totals failing closed.
Current source remains unreviewed by CodeRabbit; review capacity is reserved
first for final discovery corrections and then the queued corner measurements.


Final discovery CLI review: exact ce56000 versus c84655d completed across two
files with zero major findings, one minor documentation-spacing finding, major
gate passed and exit 0. CR-DISCOVERY-PROVENANCE-SPACING accepted: revision and
exit-status tokens are separated for readability. Production Rust and fixtures
remain identical to the646-test validated ce56000 source. This is the third
CLI pass for the discovery feature; no fourth review is requested for whitespace
or the review outcome note. The remaining GitHub App review is separate and
pending; CLI readiness does not provide owner merge consent.


DEV-READINESS-CLEAN-SUMMARY (developer-identified format limitation) — accepted.
The actual PR22 follow-up App review completed with no actionable comments for
224621b, run e80421b6-3071-4107-b7a9-6c6f29fbf176, in bot issuecomment6433690801.
It did not create a formal bot PR review. The initial checker conservatively
rejected that unknown evidence shape; the owner's reply review was correctly
not mistaken for a bot review. This is not a new independent finding or closure.
Correction adds a separate known-summary evidence path, requiring the API bot
identity, one recent-review section, exact repository/head URL, UUID and one
final_review_risk_coverage object with matching source/covered head and reviewed
kind. Skipped, triaged, stale, spoofed, ambiguous or malformed summaries fail
closed; open threads, state/head races and changes-requested checks remain.
The tool still never changes threads/protections/credentials or authorizes merge.
Four new unique tests (14 total) pass. Actual read-only PR22 now recognizes the
current-head summary; the bot independently resolved its old thread, giving
coverage/conversations ready with merge_authorized=false. The initial shell
assertion expected an open thread and returned nonzero when that real state
changed; inspect the JSON, not that obsolete expectation, as readiness evidence.
Source: tools/review_readiness.py, tools/test_review_readiness.py and workflow
documentation. Full gate and incremental CodeRabbit review follow separately.
No physical calibration, firmware, application algorithm or owner setup changes.

Known-summary correction gate completed successfully in its dedicated checkout
build directory:652 Rust tests, formatting, strict all-target Clippy and release;
14 unique Python tests pass. These checks do not prove merge authorization.
Incremental CodeRabbit review remains pending. The original combined ced7697
review completed14 files/zero findings/major-pass, and exact ced7697 is published
in PR24; this later checker correction is not falsely included in that result.

Known-summary correction review of exact99de067 versus ced7697 completed across
four changed files: zero findings, major gate passed, exit0. This is the second
CLI pass for the combined corner/readiness milestone; no unchanged repeat is
requested. Complete diff and tracked context pass secret scans; fresh included
capacity and inactive billing were checked, with no paid flags. Source code
remains the tested/reviewed99de067; this outcome note is documentation-only.


CR-AUDIT-ENTRYPOINT-INDEX — accepted as a clarity/completeness improvement.
The first audit-documentation CLI review covered all eight new reports at
 eadd10b versus ced7697: zero major findings, one minor entry-index finding,
major gate passed, exit 0. Source inspection qualifies its premise: CLI main.rs
has private binary/dispatch functions and no local #[test]; output.rs only
re-exports shared publication functions. The original lexical index intentionally
selected public definitions/local tests, so no public implementation or local
test was actually dropped by that filter. Nevertheless both modules are now
explicit rows, accurately labeled private entry/re-export with no invented tests.
Source files changed: ALGORITHM_PROVENANCE.md and ARCHITECTURE.md under the audit
folder, plus this developer-owned response. Also clarified the data-flow diagram
so unorganized fused clouds do not appear to feed the organized-grid mesher,
and replaced three generic module descriptions with inspected responsibilities.
No application implementation or independent reviewer file changes. Report links,
frozen source identity, notice inventory, whitespace and secret scans are checked;
no historical hardware/numerical benchmark is claimed rerun. Incremental doc
review follows separately. Audit source remains ced7697 even while normal
development/checker fixes proceed outside the frozen snapshot.

Audit clarification review of exact 50c0a07 versus ba6b3a1 completed across four
documentation files with zero findings, major gate passed and exit 0. This was
the second audit-doc review; no unchanged third review is requested. All eight
reports still identify frozen source ced7697, and 364 local source links resolve
to tracked files at that revision. No application Rust, calibration, firmware or
reviewer-owned record was changed. The final outcome note is documentation-only.


DEV-LIVE-SESSION-CAPACITY — developer-identified mission gap, not an independent
review finding. At audited Rust source ced7697, Processor::open and
Association::new reject more than 1,000 frames per camera. The source runner
records at most that many frames, verify_recording rejects larger files, and
LiveSession::validate independently caps counts/pairs at 1,000 and references
at 1 MiB; ScanSession has a separate 100,000-history cap. A no-device synthetic
probe against the verified release libraries confirms 1,000 accepted and 1,001
rejected at both public boundaries. At nominal 50 FPS this is 20 seconds of frame
time, not a measured hardware duration; 30 minutes at 50 FPS means 90,000 frames
per camera. Conditional raw-storage arithmetic and assumptions are documented in
architecture-audit/PERFORMANCE_AND_VALIDATION.md. Recording/write/finalization,
PTS handling, per-pair files, memory and reference compatibility all require
analysis; changing one limit is insufficient. Source files modified: audit
ARCHITECTURE, PERFORMANCE_AND_VALIDATION, RECOMMENDATIONS and EXECUTIVE_SUMMARY,
plus this developer-owned response. No production code, independent findings,
scanner state, firmware or physical calibration changed. The audit recommendation
is documented for owner direction; it has not been implemented. The developer
probe is stored privately under live-capacity-audit; reviewer verification and
any independent finding/status remain solely under reviewer control.

SCAN-REV-021 — accepted. Independent assignment 262 corroborates the live
session capacity gap previously recorded as DEV-LIVE-SESSION-CAPACITY. The
current limits are coordinated across acquisition, recording verification and
persisted references; changing a single bound would not establish sustained
scanning or bounded resource use. The audit recommends a coordinated design,
but the audit instructions reserve implementation of recommendations for explicit
owner authorization. No production correction has been made. The audit reports
changed in commit d46decb36f4e262314e01a2d4a98d0c4bf3b57c2; this response
records the subsequent independent finding. Developer testing: synthetic release-library boundary probe accepted
1,000 and rejected 1,001 frames at both public entry points. The reviewer checked
the probe's retained hashes and source but did not independently run it. No
physical scan duration or throughput has been measured. The finding remains
under independent reviewer control; long-session design and validation remain
outstanding.

CR-DISCOVERY-APP-REVIEW — GitHub App review completed for exact PR #23 head
 a5deec7249d944c4f9f9fa70d0c019c5821fa37f against 224621b491a6a1811d184ac30f619e09d2713754.
Run 5ced998c-8bd3-4223-9345-a78c2c37fb75 reports no actionable comments across
five selected files; API returned no formal reviews or inline comments. The
read-only readiness diagnostic identifies the matching current-head clean summary
and zero unresolved threads; it explicitly does not authorize a merge. Both
Rust CI checks for that PR passed; 646 tests and the actual CI integration tree
were previously verified. The App's separate docstring-coverage warning is
acknowledged: the helper lifecycle and its limits are described in
CAPTURE_PROCESS_ISOLATION.md, and requiring generated docstrings on private
functions solely to reach an automated percentage would not establish correctness.
No source changes or further review request were made for this warning. Whole-app
cleanup and physical hardware qualification remain unproven (SCAN-REV-005/006).
This included review used the available allowance; no paid review was requested.

DEV-NATIVE-COMPONENTS — developer milestone, not an independent finding. Add
original bounded eight-connected component measurements to scancraft-image and
the offline CLI detect-components command. This moves the observation stage used
in real board investigation into Rust without activating factory hypotheses or
embedding private board coordinates. Existing reviewer findings retain their
independent status. Source changes: image components/lib, CLI component_diagnostics/
main, reference tools and BRIGHT_COMPONENT_OBSERVATIONS documentation. Full Rust
gate passed 659 tests, formatting, strict Clippy and release compilation. The
seeded OpenCV reference check passed 60 synthetic cases and a negative payload-CRC
binding test; eight real frames matched reference component statistics exactly.
These are segmentation checks, not physical accuracy qualification. Limitations:
binary centroids, no circle fit or board indexing, no marker identity/tracking,
bounded per-image scratch allocation, and no real-time performance claim.
Implementation commit: 352d6629cca91f5f999cbe41f53497764faad864, published in
PR #26 against 008f9b76334c871f6b769c5ae24f22a6ab9e04a3. The complete selected
diff and tracked source context passed secret scanning before review upload.
One included CodeRabbit CLI review completed with zero findings, major gate
passed and exit 0. GitHub App run 467ac6cb-8cca-4dfe-8343-2d24339a5227
completed on that exact head with no actionable comments across all eight files.
Both GitHub Rust CI runs (41434551576 and 41434472246) passed. The read-only
readiness diagnostic confirmed exact-head clean-summary coverage and zero
unresolved threads; it reports merge_authorized=false. No merge or paid review
was requested, and no private hardware evidence was uploaded.

The App's separate 10% docstring-coverage warning is acknowledged, but a blanket
80% requirement is not adopted as a correctness gate. The public detector's
rustdoc documents connectivity, rejection order, scratch bounds and cancellation;
the module and BRIGHT_COMPONENT_OBSERVATIONS.md document provenance, pixel
semantics, command behavior and qualification limits. Private dispatch/reference
helpers and tests do not need generated prose merely to increase a percentage.
Specific missing interface contracts should still be corrected when identified.
This disposition does not alter the independent reviewer's findings or statuses.
The review evidence above covers implementation commit 352d662; this subsequent
response-only outcome update is not claimed to have been reviewed by that run.

DEV-TARGET-ASSOCIATION — developer milestone, not an independent finding. Add
original bounded four-anchor planar pattern association in calibration/target.rs
and an offline associate_target example. Reuse the existing normalized DLT
solver; no camera geometry, physical units or device-specific target data is
embedded. Reject ambiguous support, conflicting neighborhoods, horizon-crossing
models, degenerate anchors and refinement that loses/relabels seed matches.
Preserve existing independent findings, scanner controls and calibration state.
Seven focused tests pass. Six real saved-image comparisons retain all 37/26/25
previously indexed small marks per camera across three scenes; these are
image-correspondence comparisons, not physical accuracy qualification or a
held-out parameter test. Documentation: TARGET_ASSOCIATION.md. Full gate passed
666 tests, formatting, strict all-target Clippy and release compilation.
Implementation commit b7124d917784afcf4f70eb06bac0ff96e6c565fd is published
in PR #27, stacked on reviewed PR #26. The complete diff and tracked source
context passed secret scanning. One included CodeRabbit CLI review against
352d6629cca91f5f999cbe41f53497764faad864 completed with zero findings across
five files, major gate passed and exit 0. Both GitHub CI runs (41437848205 and
41437792579) passed on the published head. The App initially
skipped the non-default base. After the included hourly interval elapsed, one
manual review was requested (comment 6576876169); App run
7f685d52-4093-4760-9c84-f5f43f81eb86 is in progress. No paid usage was requested.
The CLI pass does not substitute for completed GitHub feedback. No merge is authorized.
This subsequent outcome-only text is not covered by the b7124d9 review.

Additional private validation connects native association outputs to the existing
Rust triangulator with the unchanged fixed-factory hypothesis and numerical
limits. It accepts 37, 33 and 30 pairs across the three saved scenes, but this
remains unqualified raw-unit geometry. The expanded partly obscured scene has
a much larger target-fit residual than its earlier unobscured subset despite
low reprojection error. Inspection shows the affected left-camera mark is
partly occluded while its partner is circular. Binary area centroids therefore
need image-quality checks before use as precision circle centers; successful
association/reprojection alone does not establish measurement accuracy. No
outlier was silently removed, no rejection bound relaxed and no calibration
installed. Raw observations/results stay local; the physical qualification
finding remains controlled by the independent reviewer.

DEV-COMPONENT-SHAPE-QUALITY — developer milestone, not an independent finding.
Add original normalized general-conic/ellipse validation, revalidated component
boundary measurements and the recording-backed inspect-components CLI. Preserve
all raw observations, explicit fit errors and false physical-center/accuracy
flags. Sources: calibration/ellipse, image/shape, CLI component_diagnostics/main,
module exports, optional NumPy reference tools and COMPONENT_SHAPE_QUALITY.md.
No copied upstream implementation or added dependency. Synthetic/real numerical
reference checks pass 32 images and 227 fits; the explored residual threshold
flags the known obscured mark and other irregular regions. It is not a qualified
default scanner filter or physical-center correction. The first full Rust gate
failed when /tmp returned Disk quota exceeded; retained its log and reran with
a dedicated home-filesystem TMPDIR. The final gate passed 675 tests, formatting,
strict Clippy and release compilation. Commit
9e72156970e75df96be9f57f0bff791edd8a5222 is published in PR #28, stacked on
PR #27. One included CodeRabbit CLI review of the complete ten-file diff against
b7124d917784afcf4f70eb06bac0ff96e6c565fd completed with zero findings,
major gate passed and exit 0, after full diff/context secret scanning. Both GitHub Rust CI runs
41440955042/41440857334 passed; the App review remains pending. No paid review was requested.
This outcome-only addition is not claimed covered by the earlier CLI run.
No independent reviewer file, hardware controls, calibration or the Git `main`
branch was changed. CLI `main.rs` was updated to expose `inspect-components`.

DEV-FRAME-PAIRING-AUDIT — developer evidence correction, not an independent
finding. An unused-frame probe caught that equal frame indices in two of the
three board recordings have different sensor PTS. Earlier index-5 geometry
comparisons used a stationary target and retained false synchronization flags;
they must not be described as synchronized pairs. Source recording timestamps
and native selected-frame reports now determine exact-PTS associations. Equal
PTS still does not prove simultaneous physical exposures. With fixed shape and
association parameters on first/last matching timestamps, five of six stereo
comparisons proceeded; one early frame failed the existing refinement-preserves-
seed guard. No threshold was relaxed or failure omitted. Same-scene unused-frame
results do not establish new-pose generalization or physical accuracy.

The retained capture manifests also show acquisition faults: one board session
recorded 100 left-camera buffers rejected before the first valid frame and 103
afterward, with 103 sequence gaps. Another session had one right-camera rejection
and gap after startup; the third had only startup rejections. Each recording
contains ten validated saved frames, but exact-PTS matches across cameras are
9, 9 and 10 respectively. CRC-valid saved files are not evidence of lossless
acquisition. These are existing-session observations, not a regression attributed
to the new offline shape code. SCAN-REV-006 remains under independent reviewer
control. Next work should investigate capture reliability and integrate explicit
pairing/quality evidence without installing unqualified calibration.

DEV-CAPTURE-REPEATABILITY — developer hardware evidence, not an independent
finding. At application commit 9e72156970e75df96be9f57f0bff791edd8a5222,
three bounded 30-frame/camera recordings passed full replay/restoration; all
had 30 exact-PTS matches. Two had no rejected buffers; the middle trial had
one startup and one ongoing rejection per camera. A subsequent 18,010-frame
probe ran over ten minutes per camera, exited 0 and restored stream settings.
It reported 98/102 ongoing and 1/87 startup rejected buffers, with 18,007
matching sensor PTS. Both ongoing and all-buffer rejection proxies exceed the
0.1% mission threshold. This confirms sustained acquisition, not full reliability
or scanning qualification. Sanitized statistics/source/binary binding are in
[capture statistics](../benchmarks/capture-soak-2026-11-02.json) and
[capture repeatability](../NATIVE_CAPTURE_REPEATABILITY.md);
raw data remains local. No transport implementation, rejection guard, firmware,
calibration or reviewer status was changed. Source code is unchanged; documentation
checks passed: 675 tests, formatting, strict Clippy and release compilation;
Sanitized statistics/binding checks passed. Automated review follows separately.

CR-CAPTURE-DOCS — accepted two minor CLI findings on b364468. Evidence references
in this response now link relative to docs/reviews; BENCHMARKS no longer lists
release compilation or ten-minute structural acquisition as pending. The failed
buffer-rejection gate and unqualified full pipeline remain explicit. Initial
review completed with two minor findings, no major blockers and exit 0; the
corrected documentation requires an incremental review. No source tests need
changing; relative-link, sanitized JSON arithmetic and whitespace checks cover
these corrections. The 675-test full gate covers the unchanged application code.

CR-TARGET-INPUT-LIMIT — accepted PR #27 comment 4442298976 from completed App
run 7f685d52-4093-4760-9c84-f5f43f81eb86 on b7124d9. The example accepts
1 MiB but reads one extra byte to detect overflow. TARGET_ASSOCIATION now states
both bounds accurately; implementation is unchanged. The standalone correction
will also be applied to PR #27 after incremental review. This is a documentation
correction, not a relaxation of input bounds or independent finding closure.


CR-CAPTURE-DOCS outcome — incremental included CLI review of
0d6eb4119e79e2220b3bd88227d41db4c5edb975 against b3644683b5d436ff96a16ad02df5f2592c4abd87
completed with zero findings, major gate passed and exit 0. Published PR #29,
stacked on PR #28; both Rust CI runs 41445940125 and 41445857654 passed.
The App skipped the non-default base; no current-head App coverage is claimed.
This outcome-only response addition is not covered by the preceding CLI review.

SCAN-REV-022 — accepted. The documentation correction in 8b9e7dd is propagated
to PR #27 as fe5470ee431b1a0d0490af714f98431f7508b571, preserving the implementation.
Identical guide bytes were also propagated to PR #28 as 9181d0f so that its
stacked diff does not undo the upstream fix. Static source-bound comparison and
whitespace checks passed; both PR #27 Rust CI checks passed on the corrected head.
The correction itself was covered by the zero-finding incremental CLI review
above, not an exact-head GitHub App review of either propagated branch. Replied
to inline 4442298976 with evidence in comment 4442447947; did not resolve the
thread. The independent reviewer owns verification and status. No source change
or redundant runtime test was necessary for this prose correction.

DEV-BOARD-LASER-COMPARISON — developer experiment, not an independent finding
or a production feature. New ten-frame stereo off/on recordings passed whole-file
validation, with nine equal-PTS pairs each. Fixed Boolean 151 read back 0→1→0;
complete illumination status matched before/after. Inspected images show lines
on the board in the on capture and no such pattern in the off capture. Native
shape/association/triangulation with prior fixed parameters accepted 36 off-image
mark pairs, 35 passing the exploratory shape gate. Their plane RMS is about
0.081 in unqualified factory raw units. On-image left target association fails
its four-anchor requirement where lines intersect anchors; failure is retained.

A private Rust probe uses existing CameraModel and QuadraticSheet methods under
the prior reflected interface-2/lens-0 hypothesis. All 5,657 off/on candidate
intersections satisfy independent Python substitution into the sheet equations
to below 9e-14 raw units. This is numerical consistency, not physical accuracy.
Opposite-camera support within the previously used 3-pixel bound, unique sheet
candidate and greedy distinct matching yield 284 interior on-image pairs versus
zero off-image pairs; shifted on-image controls yield 6/13/15. Selection does
not use the reference plane. Crucially, retained on-image points include large
false matches: plane RMS 23.30 raw units and maximum absolute residual 234.28,
despite median absolute residual 0.510. No residual trimming or expected-distance
fit is applied. Nearest-sheet-to-plane selection is retained separately and
explicitly labeled circular as a line-identity/accuracy test. The reference plane
shares factory camera geometry, and off/on stationarity, physical units and
exposure timing remain unqualified. Raw captures, private probes and hashes stay
local; no calibration is installed or firmware changed. Next reconstruction work
needs reliable stripe correspondence and line identity, not more favorable
reporting of selected points. Existing physical-qualification findings remain
under independent reviewer control.


DEV-STRIPE-CONTINUITY — accepted development priority arising from the private
false-correspondence experiment, not an independent review finding. Added
original image/laser_continuity.rs with mutually unique endpoint association,
bounded gaps/displacements/comparison work, explicit fork/join termination and
full input-index partitioning. The CLI trace-stripes command adds diagnostics
while retaining all observations and recording integrity/provenance checks;
no automatic filter, calibration installation or live pipeline change. Sources:
image module/export, CLI laser_diagnostics/main, optional NumPy reference tool,
STRIPE_CONTINUITY.md and architecture documentation. No upstream code copied
or production dependency added. The first full gate caught one Clippy style
violation; fixed it without disabling the lint. Final gate passed 680 tests,
formatting, strict Clippy and release build. The optional matrix reference
matched 32 synthetic recordings/7,620 observations and 12 real images. The same
criteria on unused first/last equal-PTS pairs reduced provisional greedy matches
292→83 and 284→75, with raw reference-plane RMS 22.392→0.671 and 19.963→0.667.
No plane residual was used for selection. These remain same-pose exploratory
results with substantial lost coverage, shared factory geometry and unqualified
units/exposure timing. No physical-accuracy claim or independent finding closure.
Commit and CodeRabbit outcomes follow after review; raw scans stay local.


DEV-STRIPE-CONTINUITY review outcome — commit
b8c772078a7e9478301d39d2f4ee5988f40547e5 published in PR #30, stacked on PR #29.
One included CodeRabbit CLI review against 0d6eb4119e79e2220b3bd88227d41db4c5edb975
completed with zero findings, major gate passed and exit 0. Complete diff and
tracked source context passed secret scanning before upload. The App skipped
its non-default base, so no App coverage is claimed. One Rust CI run passed and
the second was still running at the last check. No merge or paid usage requested.
This outcome-only addition is not covered by the preceding review.

DEV-EPIPOLAR-INTERPOLATION-PROBE — private follow-up, not production code or an
independent finding. Comparing the same continuity-supported pairs with native
stereo triangulation gives reference-plane RMS about 0.260–0.264 raw units versus
about 0.667 for factory-sheet intersections. Intersecting the stereo epipolar
constraint with adjacent image-trace segments, without extrapolation and within
three pixels of the original partner, retains 79/80, 81/83 and 74/75 pairs across
middle/first/last frames. RMS becomes 0.231/0.233/0.226 raw units. All failures and
residuals are retained. Low reprojection error is imposed by construction and is
not independent accuracy evidence. Original matches still depend on provisional
sheet hypotheses; camera geometry and the target reference are shared, while
physical units and target-surface assumptions remain unqualified. No production
method, rejection limit, calibration or device setting changed. A public method
would need analytic/degeneracy/ambiguity tests and interpolation provenance before
integration; these private probes are not a released reconstruction capability.

PR #30 subsequent CI recheck: run 41449538212 completed successfully at
13:05:53 UTC; both Rust checks now pass on b8c7720. App coverage remains pending.


DEV-EPIPOLAR-TRACE — development continuation, not an independent finding.
Added calibration/epipolar_trace.rs and the bounded intersect_epipolar_trace
example. Reuses existing camera preparation and essential-matrix calculation;
segments interpolate in undistorted normalized coordinates, supporting distortion
and skew. Every intersection and near-coincident segment remains explicit; only
a repeated shared vertex is deduplicated. Invalid/gapped input, cancellation and
camera-domain errors fail without partial output. No projector identity, depth
export, physical-unit inference or calibration activation is introduced.

Source changes: calibration module/export/example, optional NumPy known-point
reference tool, EPIPOLAR_TRACE_INTERSECTIONS.md and architecture/response docs.
Original implementation, no upstream code copied or new dependency. Four new
unit tests cover known geometry, nontrivial camera pose, distortion/skew,
ambiguities, numerical tolerance, input bounds, cancellation and invariance.
Final full gate passed 684 tests, formatting, strict Clippy and release build;
32 synthetic references passed with max pixel error 5.25e-10. Private existing
Raptor traces retained 79/80, 81/83 and 74/75 candidates with unchanged stereo
guards; raw-plane RMS remains about 0.23 and is not a physical accuracy result.
No camera/device controls changed. Commit/review outcomes follow separately.
Separate review backlog item, not coverage of DEV-EPIPOLAR-TRACE: PR #28 App
review requested once after its included hourly interval, comment 6578089151.
That older PR contains component-shape work. Its completed review is documented
below; it does not review this epipolar change. No paid usage was requested.


CR-EPIPOLAR-REVIEW-SCOPE — disputed the suggested PR-number correction after
verification. Comment 6578089151 is attached to GitHub issue/PR 28 and contains
@coderabbitai review, as recorded. It intentionally requests an older pending
shape review, not review of this new epipolar change. Clarified the backlog
paragraph to prevent readers from confusing coverage. Initial CLI review of
c484556a60e373b5493ede3b3786a304377ee817 against b8c7720 completed with this one
minor finding, no major blockers and exit 0. Corrected documentation requires
incremental review; application source remains identical to the 684-test gate.

CR-SHAPE-MAIN-WORDING — accepted ambiguity in PR #28 inline 4442858468,
formal review 5737158429 on 9181d0f, run 509fca5f-b384-44f5-b996-11c55c6f6f7d.
The earlier statement referred to the Git main branch, not main.rs. Standalone
commit 1f284b9 explicitly names the Git branch and acknowledges the CLI main.rs
change. No implementation change is needed. Verified the actual changed-file
scope and whitespace; no redundant runtime test added for prose. Propagation
back to the older PR and stacked descendants follows incremental review. No
independent finding or GitHub thread was resolved by the developer.

DEV-RECORDED-TRACE-CORRESPONDENCE — developer milestone, not an independent finding.
Added workflow/recorded_traces.rs, its library export and CLI dispatch, with an
explicit bounded request and recording-backed observation indices. Whole-recording
validation, selected frame CRC/profile binding, raw-PTS/host-window pairing and
source rechecks precede no-overwrite publication. Native detection, continuity and
epipolar geometry retain observations, rejected seeds, ambiguity and interpolation
lineage. Exact request/geometry and selected frame data accompany candidate pairs.
No calibration activation, inferred units, projector identity or 3D export.

Five new regressions cover known interpolation, artifact preservation, reflection,
input mismatch, late corruption, ambiguity, cancellation and overwrite refusal.
The initial full gate exposed CLI error-adapter/import issues; corrected using the
existing qualified WorkControl and string-error adapter pattern. Final gate passed
689 tests, formatting, strict Clippy and release build. Three private saved Raptor
pairs reproduce prior native-example coordinates exactly (79/80, 81/83, 74/75),
with artifact lengths/CRCs verified. This is reproducibility on existing same-pose
evidence, not physical accuracy or reliable automatic seed discovery.

CodeRabbit CLI quota/billing lookup currently reports unavailable/lookup_failed
even from the repository. No new review was started without verified included
capacity, and no paid usage was authorized. Automated review and publication of
this change remain pending; earlier epipolar documentation review is also pending.
Source commit binding follows in the recovery checkpoint. Independent reviewer
files, Git main, scanner controls and installed calibration remain untouched.

DEV-SHEET-SUPPORTED-TRACES — developer continuation, not an independent finding.
Accepted the development need to remove externally supplied partner seeds rather
than treating sparse seeded interpolation as an end-to-end scanner. Private
all-trace evaluation found three depth-valid crossings for every observation in
the fixed target ROI (96/99/92), demonstrating unresolved stripe identity from
stereo alone. Explicit sheet support retained 93/96/89 ROI points, with no
reference-plane selection. A 200px translated-trace control retained zero/592.

Added calibration/laser_trace.rs and export, bounded match_laser_traces example,
known-point reference tool, architecture/status and SHEET_SUPPORTED_TRACES docs.
Reuses original native primitives; retains both quadratic roots, competing traces,
all supports, triangulation rejection reasons and ambiguous segments. Numerical
or budget failures cannot produce a partial unique selection. No factory parsing,
calibration activation, recording attestation or project export is introduced.

Six new tests and full 695-test/fmt/strict-Clippy/release gate passed. Independent
known-point projection reference: 36 synthetic cases, maximum position error
9.05e-9 synthetic units, pixel error 1.24e-10. Actual private saved-frame matches
exactly agree with the previous explicit composition (473/480/473 whole-image,
93/96/89 within target region); translated control gives zero. Shared primitives,
same-pose data and provisional geometry do not establish physical accuracy or
projector identity. Current matcher repeats trace preparation per first pixel;
live performance and cross-pixel association remain unqualified.

Current automated review remains unavailable: usage lookup fails, while doctor
reports authenticated/reachable services and nine passing checks. No paid usage
or review bypass. This is local development evidence, not an automated-review pass.
Commit/source/evidence bindings are retained in the private recovery checkpoint.
Independent reviewer findings, main branch and physical setup remain untouched.

DEV-RECORDED-SHEET-MATCHING — developer milestone, not an independent finding.
Added recorded_traces/automatic.rs and match-recorded-laser-traces CLI command.
Shared only detection/frame-binding helpers with the preserved seeded schema;
extracted existing geometry Config::validate methods so empty datasets cannot
skip numerical settings validation. Full recording validation/CRC/PTS/host-window
and source guards precede native detection, continuity and automatic matching.
All diagnostics stream with a byte cap; explicit query/total work bounds reject
oversized inputs without sampling. Candidate coordinates retain source/trace/sheet
lineage and unverified units; no calibration activation or PLY/project export.

Five new regressions cover known geometry, exact artifacts/lineage/no-overwrite,
ambiguity, invalid bounds/CRC/index/footer, cancellation, streaming-limit partial
retention, reflection and empty continuity. Initial gate failed an existing error
message regression during validator extraction; restored the original text.
Final 700-test/fmt/strict-Clippy/release gate passed, and all 36 independent
known-point reference cases pass. No tests or guards were weakened.

Three private Raptor pairs reproduce every complete earlier native matcher result
and coordinate exactly (473/480/473 candidates; no query errors), with original
detections, exact geometry/request and artifact CRC/length checks. Older seeded
workflow still reproduces 79/81/74 candidates exactly. Separate low per-query
budget run preserves all 592 errors and zero points. Offline end-to-end observations
are 0.38s elapsed, 0.29–0.30s user CPU, 0.06s system CPU, 18352–18360KiB peak RSS
on Ryzen9 6900HS; shared workload/reused files and validation/fsync included, not
isolated matching benchmarks or live throughput. Samepose/model evidence remains
unqualified physically. Details in RECORDED_SHEET_MATCHING.md.

CodeRabbit usage remains unavailable/lookup_failed; no automated-review pass or
publication claimed, and no paid usage/merge/protection change. Local commit and
evidence bindings follow in the recovery checkpoint. Reviewer files and physical
scanner setup remain untouched. GUI/live integration and qualification remain open.

DEV-PREPARED-LASER-TRACES — developer optimization, not an independent finding.
Profiled before changing implementation: uncached matching about189ms, a separate
repeated right-ray loop164ms, detection70ms and recording validation/selection57ms.
Added crate-private normalized traces and public PreparedQueries borrowing its
exact matcher, traces, sheets and config. Automatic recorded workflow prepares
once per pair. Shared intersection math preserves raw endpoint/interpolation
lineage and error/budget semantics. No precision changes, sampling or relaxed limits.
One-shot preparation cancellation is latched rather than cached as a numerical error.

Changed calibration/epipolar_trace.rs, laser_trace.rs, workflow automatic.rs,
diagnostic example/reference flag and architecture/status/contract documentation.
Three new equivalence/model-separation/budget/cancellation tests; full703-test,
format/strict-Clippy/release gate PASS. Both cached and uncached36-case independent
known-point references pass. Three real saved pairs preserve EVERY complete query
result and candidate coordinate exactly (473/480/473), as well as detections and
artifact checksums. Seeded workflow79/81/74 remains exact; low-budget592 errors
remain byte-identical. No hardware/model qualification is inferred.

Ten measured iterations after one warm-up, alternating cached/uncached order in
the same optimized probe, give matching median187.26ms versus7.87ms plus0.310ms
preparation; median paired matching speedup including preparation22.9x on this
specific592-query workload. Shared machine, one scene, results retained in memory;
not a full scanner speedup. Subsequent offline CLI runs0.19s each versus earlier
0.38s are limited end-to-end observations, not a controlled paired benchmark.
PREPARED_LASER_TRACES.md records method, limits and remaining70ms-class detection cost.
Private source/build/library/binary hashes and raw timing/result evidence stay local.

Automated review/publication remain pending the unavailable CodeRabbit quota and
billing lookup. No paid service, review bypass, merge or reviewer-file change.
Commit and evidence binding are recorded in the recovery checkpoint.

DEV-BLOCK-LASER-OPENING — developer optimization, not an independent finding.
Measured the next bottleneck before integration. A private same-binary paired
probe compares the original project queue function with independently written
neutral-padded block prefix/suffix extrema, alternating order over ten measured
iterations after a warm-up at each of five radii. Every contrast byte matches.
At current radius12, medians70.22ms versus14.60ms, median paired speedup4.81x for
two-image morphology only. Method family/reference and original implementation
provenance are documented in BLOCK_LASER_OPENING.md; no upstream code/dependency added.

Changed image/laser.rs row extrema and tests, plus architecture/status/detector
documentation. Detector thresholds, raw saturation, centroid math and ordering are
unchanged. Extra row scratch is O(width),3888bytes at1920/radius12; image/window
bounds and row cancellation remain. Existing direct-window regression retained;
two new tests cover all supported radii and complete detections against independent
neighborhood extrema. Initial gate caught a Clippy test expression, corrected;
the next formatting check caught its layout, formatted without disabling checks.
Final705-test/fmt/strict-Clippy/release gate PASS.

All12 saved on/off camera JSON reports equal prior queue reports exactly. Automatic
473/480/473 candidates and EVERY complete query result remain exact; seeded79/81/74
and artifact/input checks pass. Public-library detector phase observed18.79ms
versus earlier71.52ms, but those are separately built phase probes, not a paired
detector benchmark. Whole CLI0.13/0.13/0.14s with about18MiB RSS are shared-host,
reused-file observations includingvalidation/fsync, not live FPS/accuracy evidence.

No hardware/configuration/calibration changes or independent-reviewer-file edits.
CodeRabbit quota/billing lookup remains unavailable; no paid usage, review pass,
publication, merge or protection change claimed. Recovery checkpoint binds commit,
private source/binary/library hashes, raw timing rows and full numerical evidence.

DEV-ROTATING-TARGET-EVIDENCE — developer hardware investigation, not an independent
finding. Owner operated turntable, moved board and supplied a stationary cardboard
box before leaving. Existing authorized line-capture paths restored illumination
and stream settings; no firmware/physical calibration or persistent control change.
Saved900 accepted frames/camera at about29.863Hz,900 equalPTS/host-window pairs;
four ongoing rejected buffers/camera (~0.442%) plus80/108startup rejects remain
unqualified and exceed the provisional reliability target. Full recording integrity
and exposure-content validity remain distinct.

Timing diagnostics expose only100example pairs, so a private indexed helper
selected9pairs across all900 frames rather than calling that truncated list a full
rotation. Helper initially assumed optional PTS; actual FrameTiming is u32, corrected
and rebuilt with original failure/source retained. Frame copies preserve timing/CRC
lineage. Eight matching bundles pass checksums; one pose fails conservative work
admission. Counts and zero-match failures are retained in ROTATING_TARGET_EVIDENCE.md.
Front-facing saturation removes most supported second traces; that is an input
limitation requiring quality feedback, not grounds to silently relax clipping.

New box color/IR captures show weak laser signal. Software threshold10/20 produces
1764 candidates from2604 queries withzero perqueryerrors; offcontrolzero. Original
30/50 produceszero;5/12 exceeds the explicit query bound. No target residual was
used for selection. Candidate surface identity/units/accuracy remain unverified.
No production code changed after705-test gate2441a17; this report records actual
failure modes and updates priorities. Reviewer files untouched, no independent
finding states changed. Private source/build/data/evidence references are retained
in the recovery checkpoint; no private payload uploaded or paid review requested.

DEV-CANDIDATE-VIEWER — developer integration, not an independent finding.
Added workflow/candidate_bundle.rs, app/candidate_ui.rs, a synthetic recorded-laser
fixture and native --laser-candidates/tab integration. Reuses scan-project Guard
internally, cloud_jobs ownership/cancellation and cloud_ui rendering. The diagnostic
path builds Preview directly, never treating raw positions as metric Cloud/Editor.
Normal editable state remains separate. Failed loads preserve the prior view and
its loaded-folder label; root close guards include the new job.

Fixed seven-role layout, byte/CRC/identity checks and complete retained one-frame
recordings precede bounded finite coordinates. Frame/request bindings and pairing
are checked; historical originals, physical calibration and numerical matching are
not attested. Typed summaries skip unused observation data and enforce100000/4096
array limits during parsing, avoiding a large generic observation JSON tree.
No filesystem/parse hard deadline or worst-case memory qualification is claimed.

Nine added regressions and full714-test/fmt/strict-Clippy/release gate pass.
Initial focused build required Debug on Loaded for error assertions; added it.
Initial full gate caught test placement/default initialization style; fixed without
disabling lints. A later parser-memory concern justified typed bounded parsing and
its regression, followed by the final gate. Native renders of126synthetic and1764
saved box-scene points were inspected with unverified-unit labels. Both process
checks showzero scanner descriptors. Initial Xvfb lookup failed, then explicit
legacy installation path worked. Corrected render metadata mode to laser_candidates
and reran final screenshots; source/binary evidence retained privately.

CodeRabbit official updater moved0.9.1 to0.9.2 after a download EDQUOT failure was
resolved with homeTMPDIR. Authstatus reports authenticated; usage still reports
lookup_failed for included allowance/billing, so no paid usage or review started.
Review/publication remain pending, not passed. Local source/commit/evidence hashes
are in the recovery checkpoint. No reviewer-file, firmware/calibration, hardware
capture, main-branch merge or protection change occurred in this viewer task.

DEV-CANDIDATE-FRAME-FEEDBACK — developer continuation, not an independent finding.
Accepted the need to expose input evidence alongside sparse/empty candidate views.
Added candidate_bundle/frame_view.rs and loader-returned frame views, cloud_jobs
report transport and CandidateUi camera panels. Reuses existing exact raw histogram
statistics and monotonic display LUT rather than implementing another algorithm.
Full original samples determine statistics; bounded nearest thumbnails keep raw
orientation. USB interface labels do not infer physical camera orientation.
Display-only contrast never modifies frames, thresholds, geometry or candidates.
Failed loads preserve both camera and point views. No scanner control is sent.

Two added regressions and extended loader/preservation checks pass; full716-test,
formatting, strict Clippy and release gate PASS on the final source. Private release
probe plus independent Python histogram/sample checks agree exactly for synthetic,
box, zero-candidate front-board and edge-board bundles. Raw CRCs match requests and
candidate counts remain126/1764/0/1275. Native private-display renders show camera
panels and point views, with zero scanner descriptors. A separate private-display
checkbox check exercises contrast; screenshots stay private. No matching or format
change; no new dependency, hardware capture, calibration or firmware action.

Limitations: subsampled previews can miss narrow stripes; endpoint counts are not
detector rejected-run counters or causal diagnosis. Full-image statistics do not
qualify object visibility, focus, synchronization, units or physical accuracy.
Statistics have bounded input but no internal cancellation/hard-time guarantee;
SCAN-REV-005/006 and long-session021 remain unchanged and open/unqualified.
Fresh CodeRabbit usage still reports unavailable/lookup_failed for included quota
and billing; no review/paid usage/publication claimed. Local commit/source/evidence
bindings follow in the recovery checkpoint. Independent findings remain untouched.

DEV-NATIVE-RECORDED-LASER-PROCESSING — developer continuation, not an independent
finding. Accepted the need to make the existing recorded laser reconstruction
workflow directly usable in the native app. Added candidate_ui/processing.rs,
Operation::ProcessLaserCandidates and explicit startup action; extracted existing
candidate view handling into cloud_jobs/candidates.rs. Calls run_automatic then
verified bundle loading. Numerical selection, limits and formats are unchanged;
only per-query/finishing WorkControl progress was added to automatic.rs. Inputs
equality traits support path/index startup checks. No new algorithm or dependency.

Publication and display are separate outcomes: a failure/cancellation while loading
an already published result reports its saved folder and reopen action. Prior
camera/point views and loaded-source label survive failure. No editable document,
selection or history mutation, hardware opening, calibration activation or device
control occurs. Existing output is rejected; incomplete workflow diagnostics and
their paths remain available. Saved per-query failures or zero points are not
claimed successful scanning.

Three actual synthetic-worker regressions pass: four reconstructed candidates and
verified display without editable-state mutation; pre-cancellation/no overwrite;
post-publication display cancellation with retained output and successful reopening.
Startup index/path validation and GUI form-button/error preservation add two more
tests. The GUI regression sends real egui pointer events to the header and Process
pair button. Initial full gate caught three test-only owned-path comparisons;
fixed to borrowed Path comparisons without lint suppression. Final721-test/fmt/
strict-Clippy/release gate passes. No numerical acceptance gates were relaxed.

Four private native-app runs check actual processing from recordings: synthetic126,
box1764 and front-board0 candidates reproduce EVERY match record and candidate-point
file byte-for-byte, including input hashes and output artifact CRCs/lengths. A path
with spaces works. Total-work1 control is rejected before publication with visible
error. Box/error screens inspected; no scanner descriptors held. Source/binary/test
and runtime evidence remain local and are bound in the recovery checkpoint.

Limitations remain unqualified physical geometry/synchronization/units, whole-app
shutdown005, sustained-session capacity021, and continuous laser tracking/fusion.
No private payload upload, firmware/calibration/control action, reviewer-file edit,
main merge or protection change. Fresh CodeRabbit quota/billing lookup remains
unavailable/lookup_failed; automated review and publication are pending, not passed.

DEV-PROJECTED-ROOT-WORK-BOUND — developer optimization, not an independent finding.
Measured the rotating-pose rejection using original public native components,
before changing production code: sample2 has3145 queries, coarse bound169811130,
actual counted matching work11414357 and2532 candidates withzero errors. Its actual
projected-root bound64691538 fits the existing100million cap. The baseline's other
eight complete match files equal historical workflow files byte-for-byte.

Added PreparedQueries::prepare_pixel/PreparedPixel in calibration/laser_trace.rs.
Private roots borrow their exact preparing queries and move into consuming execute;
shared project_sheets math remains unchanged. work_bound=V+S*actualRoots is proved
by at most one crossing per segment, retaining both roots, duplicate models and
all competing traces. Workflow automatic.rs uses this only when cheap admission
fails; checked sum precedes matching/staging, with early geometry errors retained.
Hard caps, thresholds, numerical acceptance and qualification flags are unchanged.
Fallback manifest adds method/coarse/preparation-failure metadata. No new dependency,
copied upstream implementation, hardware control, calibration or firmware action.

Four new regressions and full725-test/fmt/strict-Clippy/release gate PASS. Covers
full-result equality withzero/one/tworoots/zigzags/coincidentsegments, invalidpixel/
cancellation/perquerybudget, exacttight2016boundary and2015rejection/no partial,
and retention of32earlygeometryerrors. Updated prior coarse3007 failure fixture to
the newproved2015 boundary; explicit equality test protects the numerical output
and configured cap. Reference example/tool add --prepared-pixel; all36 independent
known-point cases pass in all3 modes (maxposition9.05e-9 synthetic units,
maxpixel1.24e-10), with no physical accuracy inference.

New CLI publishes all9 saved poses:0/779/2532/953/1275/1449/648/1276/0 candidates,
zero query errors. ALL complete match bytes equal pre-change direct-probe results;
all8 earlier candidate files exact; new2532positions/supports checked individually.
Input hashes and output artifact CRCs/lengths pass. Realdata bound64691537 rejects
before staging;64691538 admits identical matches. Nativeapp also displays2532points
and both images; screenshot inspected and no scanner descriptors. Private evidence
and exact source/binary/commit bindings are in the recovery checkpoint.

Limits: geometric coverage stillfails atfront views; signal/model/units/accuracy,
tracking/fusion, shutdown005 and long-session021 remain unqualified/open. Pre-change
probe6iterations/pose(firstwarmup) holds results in memory: sample2matching60.58ms,
detection16.07ms, JSON-to-memory53.30ms. Complete newCLI0.29s/29216KiB RSS is a
shared-host warmfile observation, not speedup/liveFPS qualification. Additional
fallback root storage is bounded but worstcaseRSS/deadline unqualified. Match work
units exclude root/image/serialization costs; no wholeoperation budget claim.
CodeRabbit usage stillalllookup_failed; no review/upload/paidusage/publication or
merge/protection change. Reviewer files and independent finding statuses untouched.


DEV-IN-MEMORY-LASER-PAIRS — developer continuation, not an independent finding.
Accepted the measured need to separate raw-pair computation from diagnostic JSON
and file publication before live integration. Added depth/laser_pair.rs Processor,
borrowed FrameInput, immutable Settings, optional Sink events, typed candidates,
per-query outcomes and compact camera counters. Existing detector/continuity code
moved into depth and is reused by seeded and automatic workflows. Numerical
matching/admission methods and limits remain unchanged. Depth adds only a direct
existing internal core dependency; no third-party package or license change.

Recorded automatic.rs is now a provenance/CRC/source/byte-cap adapter with a
Diagnostics sink. Owned ArtifactWriter factors the original checked write/flush/
fsync/rehash operations; the old closure helper delegates to it. Admission still
precedes staging. Partial/publication error paths, no-overwrite, CRC/header/footer
and parent sync remain. Core report flags stay unqualified and it never creates a
metric Cloud/Editor. Default () omits full detail explicitly, retains every compact
outcome/error and camera rejection/continuity counters. Sink errors stop that call
without setting WorkControl cancellation or controlling acquisition; optional
preview omissions must remain separate from required-journal failures (018).

Six new core tests plus one owned-artifact regression pass; final732-test/fmt/
strict-Clippy/release gate PASS. Covers raw validation/timing, finite known geometry,
full/sparse sink equality, empty/reused/invalid pair state, ambiguity/per-query and
batch bounds, cancellation/error/no complete report, and writer CRC/no-overwrite/
cancellation. Camera-feedback assertions were added before the final gate.
Eleven real/synthetic saved pairs reproduce all full match bytes, observations and
candidate values; camera counters agree with baseline observations. Every artifact
and manifest in all11 automatic bundles and all3 seeded bundles is byte-identical;
input hashes and output CRCs/lengths pass. Final native-app pose/box renders show
2532/1764 candidates and unchanged matches/points, with no scanner descriptors.

Final same-binary paired probe11iterations(firstwarmup), alternating queryJSON/
Noop order: worstpose104.10ms without queryJSON vs160.51ms with it, median paired
ratio1.54; box65.43vs98.52ms,ratio1.51. Real cancellation checks active in both;
model/input I/O, history/CRC binding, header/final formatting and disk/fsync excluded.
Baseline full bytes remain allocated during timers. Shared warm host; not a general
speedup, liveFPS, worstcase memory or physical qualification. Early prototype probe
and renders preserved separately; final/ artifacts bind final camera-counter source.

Owner asked about accelerometer: official Raptor manual confirms IMU. Read-only
sysfs confirms HID6/25-byte descriptor/248-byte input declaration, but axis, chip,
units, rate, actual HID-to-IMU mapping and decoding remain unverified. Corrected
protocol notes to distinguish manufacturer confirmation from HID interpretation;
no HID/control writes or inertial fusion added. Scanner-motion IMU cannot measure
an independently rotating object when scanner remains stationary.

Private evidence/commit bindings are in recovery checkpoint. No raw data/model
upload, hardware capture/control/firmware/calibration action, reviewer-file change,
merge or protection bypass. Freshest CodeRabbit usage still alllookup_failed;
review/publication pending, not passed. Goals005/006/021 remain open/unqualified;
compute speed, calibration, coverage, tracking/fusion and sustained live scanning
remain incomplete.


DEV-INDEXED-EPIPOLAR-TRACES — developer optimization, not an independent finding.
Accepted the measured need to reduce full-trace matching cost before live laser
integration. Generic cancellation helpers alone gave only small gains. Added
conservative 64-segment normalized bounds in calibration/epipolar_trace/index.rs,
indexed preparation/query paths and shared original scalar interpolation in
epipolar_trace.rs; laser_trace.rs selects indexed preparation. Independently
written interval culling, adapting existing project code from72ea37c5d07bf09ca54eb5ee26b23b14e4e3364a;
no copied upstream source or new dependency. Entire raw trace validation remains.
Risky bounds force original scalar distance prepass before any emitted result.
Global indices/shared-vertex dedup, all competing/ambiguous supports, arithmetic,
configured budgets and unqualified physical flags are unchanged.

Six new tests cover extreme exponents, conservative tolerance/rounding, long
reversed/distorted traces and length threshold, boundary/coincident lineage,
numerical fallback/error precedence and cancellation through skipped/retained
work. Extended existing full matcher equivalence with long competing traces.
First adversarial fixture mistakenly expected a projection error for a valid
center pixel; corrected its geometry and explicit expected errors. Initial gate
caught a test-only Clippy integer-expression warning; fixed without suppression.
Final738-test/fmt/strict-alltarget-Clippy/release PASS. Reference tool adds
--dense-traces; all36known-point cases in each sparse/dense and three preparation
modes PASS, dense maxposition1.303e-8syntheticunits/maxpixel3.222e-10. No physical
calibration or accuracy qualification inferred.

All11 actual/synthetic saved pair reports retain complete match bytes,
observations, camera counters and candidate values. All7artifacts+manifest exact
for every11automatic and3seeded bundles, inputSHA/CRC/lengthsPASS. Native renders
retain2532/1764 diagnostic points and both images, inspected withzero scannerfds.
Original source is byte-verified against72ea37c in a same-binary benchmark using
final production library, own isolated Cargo targets,13iterations/firstwarmup/
rotating3mode order. Matching-only realWorkControl medians86.610→32.485ms on
expensivepose (medianpairedratio2.672),49.997→23.592ms box (2.117), small fallback
1.502→1.524ms (0.985; no improvement claimed). Full results equal outside timers.
Counts drop only with skipped numerical work; perblock and retained scalar/final
checks remain. No wholeapplication cancellation bound or universal speedup claim.

Separate fullengine noJSON medians51.968/41.272ms on pose/box are saveddata
observations, not paired index speedups/liveFPS. Optical coverage, physical
model/units/synchronization/accuracy, tracking/fusion, shutdown005 and session021
remain unqualified/open. Generic work-control architecture007 remains unchanged.
No scanner capture/control/firmware/physical calibration, private payload upload,
reviewer-file modification, finding-state change, merge or protection bypass.
OfficialCLI0.9.2; fresh repository-scoped usage still returns lookup_failed for
includedquota/billing. First usage invocation from home reported not_git_repository;
corrected cwd and retained both logs. No paid review or independent pass claimed;
review/publication pending. Exact implementation commit/source/build/evidence
bindings are recorded in the developer recovery checkpoint; independent reviewer
retains sole authority to verify/close findings. See INDEXED_EPIPOLAR_TRACES.md.


DEV-BOUNDED-LASER-STREAM — developer integration, not an independent finding.
Accepted the need for a bounded raw-journal-first acquisition handoff before native
laser GUI integration. Added workflow/laser_stream.rs, association.rs, runner.rs
and tests.rs plus lib.rs export. Reuses generic Source ownership, recording Writer
and existing laser_pair engine from1904d7ed06ef51547d9229a50290d3d9952f733b; no new
thirdparty, copied upstream code, device or calibration activation. Two source owners
append valid frames before nonblocking four-frame queue admission. Queue-full frames
remain journaled and omissions counted. Pending frames/state are constant-size,
with u64 source indices and explicit modular-counter/host-window/order policy.
Initial nearest-peer counter alignment and cycle extension are inferred policy,
not evidence of synchronized exposures or reset behavior. Ambiguous half-range
gaps fail. No full-session seen set, query JSON, result list or metric Cloud retained.

StopToken includes internal failure, external graceful stop and WorkControl.
Source/journal failures and panics still attempt source close. Required callback
failure aborts; optional display omissions must remain nonfatal (018). Pair-level
processing failures are delivered/countable while raw recording continues; compact
per-query errors remain in Report. Failure.partial retains available source/count/
cleanup evidence. Finalized journal flag separates successful append from footer/
flush/fsync. Queued-but-undelivered counts are explicit, None when unknown. Added
4096metadata cap before queue/push to bound admitted frames. No hard-time cleanup
claim, no automatic publication, hardware/GUI adapter or fusion in this component.

Eight regressions:4000wrapped-counter associations; temporal/host/epoch ambiguity;
1200controlled synthetic pairs with exact direct reports and Writer/Reader CRC;
deterministic raw retention under pressure; admission errors; consumer failure/
panic; source/journal/finish/cleanup and oversized-metadata failures; cancellation
and graceful stop. Initial tests needed Debug on reports; initial Clippy caught
nested-if. Corrected without suppression. Final746test/fmt/strictClippy/release
PASS after metadata guard. Private replay of9 saved rotating pairs delivers every
prior report exactly under consumer pacing; all9 raw frames/camera byte-identical
through actual filejournals/footer/CRC and fsync. Unpaced replay also retains all
raw frames with explicit processing drops; it is not live throughput qualification.
Source/binary/test/replay bindings and exact commit are in recovery checkpoint.

Current consumer also performs association; expensive processing can drop separate
camera frames from the raw channel and reduce pair yield. Reports expose this;
future complete-pair dispatch is an optimization opportunity requiring measurement.
No constant-RSS/60min/driver-deadline claim. Old IR1000frame/history path unchanged,
so021 remains open;005 and006 remain unresolved/unqualified. No scanner capture/
control/firmware/physical calibration, private-data upload, reviewer-file/status
change, main-branch merge or protection bypass. Fresh CodeRabbit quota/billing still
lookup_failed, no paidusage/review/pass. Review/publication pending. See
BOUNDED_LASER_STREAM.md for contracts, provenance and validation scope.

DEV-BOUNDED-LASER-STREAM commit binding: implementation `16b9092dbcd5fcd09162569a941691cbfa75fee3`.
Follow-up acceptance/roadmap documentation records this exact revision; application
source and final746-test/replay evidence are unchanged. Independent review pending.


DEV-NATIVE-LASER-CAPTURE — developer integration, not an independent finding.
Accepted the need to connect bounded frame computation to native source ownership,
required raw retention and reviewable, recoverable outputs. Added workflow
laser_stream/bundle.rs, bundle/io.rs and bundle/tests.rs; acquisition/laser.rs;
CLI dispatch/run_laser and device-free preflight integration test. Reuses existing
Source, recording Writer/Reader, Guard, RENAME_NOREPLACE publication, SessionLock,
SettingsLease and fixed ScopedLine. Settings DTOs gain Serialize only; shared test
fixtures gain test-only visibility. Existing IR context/statistics helpers are
reused and now expose before/after-first-valid rejects with V4L2 buffer units.
No geometry/matching arithmetic, third-party dependency or license change.

Plan preflights original detector validators on blank discarded images before
hardware, retains exact bounded inputs/identity guards, caps raw and typed compact
pair journals, compares write-time lengths/CRCs to streamed readback, verifies raw
v2 header/profile/frame CRC/order/footer/count, then checks identities and publishes
without overwrite only after cleanup/restoration. Required journal failures retain
staging/failure.json; optional preview omission/panic never aborts raw journaling.
Parent-sync failure after rename identifies the published target. No metric PLY,
mesh or editable cloud export; diagnostic coordinates remain in pairs.ndjson and
all physical flags are false.

Native default preserves illumination. Explicit backend source_options accepts
only preserve or line_boolean_trial, enabling existing property151 with range and
readback checks, verifying at end and restoring original before publication.
Opaque options stay in workflow; native adapter validates them before hardware.
No arbitrary property, mode, energy, gain, mirror, firmware or installed-calibration
write. Native header stores the declaration. CLI Ctrl-C requests graceful stop and
retains finalizable journals; old IR signal semantics are unchanged. No hard driver,
cleanup or whole-application shutdown bound claimed (005 remains open).

Final756-test/fmt/strict-alltarget-Clippy/release gate PASS, 10 added tests cover
bundle/lineage/integrity, caps/required-journal failure, optional-preview independence,
restoration/mutation/race/no-overwrite, preflight and strict native options. First
compile needed a named result lifetime and removed unused import; fixed without
suppression. Private offline-adapter request initially omitted its separate report
byte cap; corrected that test input, retaining failed logs, without production
algorithm changes. One private probe command initially used a not-yet-created
working directory; created it before building in an isolated Cargo target.

Actual warm Raptor tests: initial preserve16/camera→16pairs/zeroqueries; option
preserve4/camera→3pairs/zeroqueries, with explicit pending inputs and startup issues.
Final counter-build line trial16/camera→12pairs (four processing drops/camera),
1857–2023candidates/pair, zero query failures; startup rejects4/85 and ongoing0/0.
That75% processed fraction is below90% target. Graceful own-child SIGINT saved3/2raw,
processed2pairs (1987/1904candidates), retainedoneunmatchedinput and completed about
0.190s aftersignal in one warm observation. Startup2/1, ongoing0/0. All raw replay/
restoration pass; full stream and illumination readbacks exactly equal before/after,
including original property151=0 restored. Earlier preserve reopening had1/89total
rejects andoneongoing rejection sample; do not cherry-pick clean warm observations
or equate V4L2 buffers with physical exposures. No qualification thresholds relaxed.

Separate offline same-engine check reproduces all17 compact reports and all27249
candidate values across preserve/line/graceful captures, with every frame-reference
CRC/sequence/flags/time/PTS verified. Selected pair2023candidatevalues exactly match
existing full recorded diagnostics and display in native viewer with both raw
images and unverified labels; screenshots inspected, zero scannerfds, app exited.
Source/build/data/evidence bindings and exact implementation commit are in recovery
checkpoint. Basea5e640a08537424a2963c29a48785dc31f931cc4; commit binding follows milestone.

No private payload uploaded or committed, reviewer file/status modified, physical
calibration performed, merge or protection bypass. Firmware untouched. CodeRabbit
included-quota/billing lookup still lookup_failed; no paid review/pass/push claimed.
Live GUI controls, performance/coverage, calibrated geometry, tracking/fusion and
long-session qualification remain incomplete. Old IR1000frame/session021 unchanged;
006 remains physically unqualified,007 coupling unchanged. See NATIVE_LASER_CAPTURE.md.

DEV-NATIVE-LASER-CAPTURE implementation commit: `c86762c1af279bfb5a1c1201e64e035480c72659`.
This documentation binding changes no application source or validation result.

DEV-LIVE-LASER-GUI — developer integration, not an independent finding.
Accepted the need to connect native laser acquisition to the existing diagnostic
viewer. Added app laser_display/laser_process (helper/replay/tests) and
candidate_ui/capture; extended IPC/root close/start exclusion and source progress
boundaries. Reuses existing OwnedChild, Worker, bounded stream/bundle, raw frame
statistics, candidate renderer and source/settings leases. No geometry arithmetic,
new upstream algorithm or metric Cloud/Editor mutation. App/image add direct
existing serde edges only; no new external package/license change.

Applies the resolved018/019/020 principles: display preparation rejection omits and
counts, BrokenPipe requests cleanup, ordinary Stop finalizes beyond abort grace,
and failed IPC/Abort retains the guard until helper exit. Only teardown may leave
cleanup unconfirmed. Final mailbox is drained after completion. These are developer
regressions, not independent re-verification or changes to finding status.005
whole-application hard bounds remain unproved;006 physical accuracy unqualified;
021 old IR capacity and007 generic control coupling remain unchanged.

Full fmt/strict-alltarget-Clippy/771tests/release gate PASS. Regressions include
bounded decode/outcome/counters, fragmented wire, delayed real children, actual
eGUI form dispatch, root-close/failed-start view preservation, and a real replay
helper with synthetic recordings, exact raw/preview equality and no overwrite.
Initial tests had a shadowed helper name, an invalid Rust float spelling, a Drop
struct-update error, a valid-statistics negative fixture and an enabled-widget
transition assumption; corrected test setup. Initial strict Clippy caught large
IPC enum and nested/dense conditionals; corrected with boxing/readable conditionals,
without suppressions. All failed logs retained privately.

Actual saved-data GUI replay shows both images/2003candidates, with unpaced input
explicitly dropping processing only. Headless native helper saves32raw/camera,
processes19pairs, preserves source context and restores exact settings. Two default
private software-renderer native GUI starts acceptzero frames and fail on partial/
error buffers/timeouts; retained failure diagnostics and cleanup evidence remain.
A subsequent private LP_NUM_THREADS=2 GUI trial saves32raw/camera,19pairs,1873–2104
candidates/pair,13/12processingdrops. Root close during another capture saves7/7raw,
6pairs and restores/publishes before app exits; observedclose-to-exit0.338s is not
hardbound. Both successful GUI trials' entire stream/illumination readbacks match,
including restoredoriginal151. GUI deviceFDs remainzero; initial childFD sampler
missed monitor-thread children, so no positive helperFD sample claimed from it.
Renderer-load/startup causality remains a hypothesis; no productiongraphics/OS
settings changed.19/32processed is below90%, no20FPS/p95/soak qualification.

Separate retained-frame same-engine check reproduces49completecompactreports and
95833candidatevalues across5bundles, all frameCRC/sequence/time/flags/PTS refs pass;
all5manifestartifact lengths/CRCs pass. All19headlessnativeIPC snapshots exactly
match savedpositions/cameracounters/acquisition/frame refs. Actualscreen images
inspected. No independent physical truth/calibratedmodel/coverage/tracking/fusion
or complete end-to-end scan claimed. Streambundle directoldviewer reopen remains
unsupported; explicit devicefree replay produces diagnosticpreviews into newfolder.

See docs/LIVE_LASER_GUI.md and updated architecture/status/roadmap/acceptance/
benchmarks; exact source/binary/evidence and implementation commit binding follow
in the recovery checkpoint. Private geometry/raw/scenes/probes/screenshots remain
outsideGit/reviewuploads. OfficialCodeRabbit0.9.2 latestusage stillincludedquota/
billinglookup_failed; no paidreview/pass/push/PR/merge/protectionbypass. Firmware,
physicalcalibration and reviewer-owned files/status unchanged. Scoped151line trial
was temporarily enabled and verified restored, as authorized for capture testing.

DEV-LIVE-LASER-GUI implementation commit: `a570c783111c1b6f25d64af8fb08c0f569e23d17`.
This follow-up binds documentation only; the 771-test build and hardware/replay evidence
remain the exact implementation source. Independent review remains pending.

DEV-BUFFERED-LASER-JOURNAL — developer optimization, not an independent finding.
Accepted a measured serializer/checksum bottleneck before considering broader
scheduling changes. A private same-binary probe uses five retained Raptor pairs,
11 iterations per pair (first warm-up), and alternating direct/buffered order.
Final report-plus-newline encode/flush medians are 6.875–7.076 ms direct versus
1.894–2.029 ms buffered, with median paired ratios 3.412–3.622. Every output's
bytes, length and CRC match. Fsync is measured separately; creation, raw I/O,
publication and the full record wrapper are excluded. Engine time remains
42.788–46.298 ms and full preview preparation 6.70–6.77 ms. These are component
observations on a warm shared host, not an application-wide speedup or live pass.

Added a fixed 64 KiB outer BufWriter in workflow laser_stream/bundle/io.rs::pair_record.
bundle.rs flushes the complete record and newline before optional preview.
CappedFile limits, CRC, final fsync/readback and publication remain unchanged.
On serialization/cap/write/flush error, into_parts explicitly discards pending
bytes without a Drop retry; accepted prefixes remain in failed staging. No session
history buffer, copied upstream algorithm, mathematical/format/threshold change,
new device control, dependency or license change was introduced.

Three new regressions and the full 774-test/format/strict-Clippy/release gate pass.
Coverage includes Unicode/floating-point multi-record equality and CRC across
buffer boundaries, exact/one-byte-short caps, partial write and flush errors,
no Drop retry, serializer failure, and a parseable record visible before preview.
The initial test assumed four queries where the fixture has five queries/four
candidates; it now compares the actual borrowed report. An unused Write import
moved to tests. The first private probe needed its direct image-crate dependency;
its manifest was corrected. The exploratory newline-less probe is separate from
the final exact-helper comparison. All failed logs and both binaries are retained;
no lint suppression was added.

An actual GUI trial with private LP_NUM_THREADS=2 rendering saves 32 raw frames per
camera, processes 21 pairs, records 11/10 processing drops and one unmatched second
input, and yields 1869–2089 candidates per pair. Startup rejects are 91/86, ongoing
0/0. The 65.625% processed fraction remains below 90%. The earlier 19/32 trial had
different frames/scheduling, so it is not a controlled application comparison.
A separate root-close trial saves 9/10 raw frames and seven pairs, with two drops
per camera, one unmatched input and startup rejects 101/1 (ongoing 0/0). Both pass
raw replay/publication/cleanup/restoration and exact full stream/illumination GET
readbacks, including original property 151 restored. One 0.281-second close-to-exit
observation is not a hard bound. Sampling all GUI tasks positively observes helper
camera descriptors with none in the GUI; helpers/GUI processes exit. Screenshots
are inspected. Default software-renderer acquisition failures remain unresolved.

Separate saved-frame checks reproduce 28 reports / 54,381 candidate values, every
frame CRC/sequence/flags/time/PTS reference and the complete pair-record bytes using
the unchanged base serializer. Both artifact manifests match lengths/CRCs. Physical
geometry/units/coverage, tracking/fusion and sustained performance remain unqualified.
SCAN-REV-005/006/021 and 007 remain unresolved/unqualified; no independent finding
status, severity or closure was changed.

See docs/LASER_STREAM_PROFILE.md and updated status/acceptance/architecture/roadmap/
benchmarks. Exact source, binary, evidence and commit bindings are in the recovery
checkpoint. Private probes, raw frames, models and screenshots remain outside Git
and review uploads. Official CodeRabbit 0.9.2 still returns lookup_failed for included
quota/billing; no paid review, upload, review pass, push, PR, merge or protection
bypass. Reviewer-owned files, firmware and physical calibration are untouched;
the authorized scoped property-151 capture trial was restored.

DEV-BUFFERED-LASER-JOURNAL implementation commit: `fdd02f2551fe606d1cd38c158c5fb9219f8b270d`.
The later binding changes documentation only; tested Rust source and evidence are unchanged.
Independent review remains pending.

DEV-ASYNC-LASER-PREVIEW — developer integration, not an independent finding.
Accepted the measured need to remove full-frame statistics/thumbnail preparation
from the reconstruction callback. Added app laser_display/pending.rs and helper
worker tests; laser_display.rs retains unchanged wire DTO/validation while
helper.rs offers bounded owned inputs and finishes them on its existing publisher.
Raw frames are validated before copying, metadata is limited to 4096 bytes/frame,
and positions/queries to 4096. One pending slot, one worker item and one producer
copy bound additional raw payload to 24 MiB at the existing image maximum, plus
bounded metadata/results, allocator and display buffers. No session history/full
Report clone, core Frame rewrite, thread per frame, new algorithm or dependency.

Required raw/pair journaling precedes optional handoff. Worker preparation or
encoding rejection increments omissions without cancelling raw work; BrokenPipe
still requests cleanup. Expensive preparation/output holds no mailbox lock.
Completion acquires done before final take and joins the publisher. These apply
resolved SCAN-REV-018/014 principles, not independent re-verification or closure.
005 hard bounds, 006 physical qualification, 007 control coupling and 021 old IR
capacity remain unresolved/unqualified. No independent finding status changed.

Final formatting/strict-all-target-Clippy/779-tests/release PASS. Five regressions
cover source reuse with exact output, input/cancellation/error limits, maximum
4 MiB raw plus 4096-byte metadata boundaries, a slow publisher while offers
continue with latest/final drain and omission counts, and worker rejection followed
by healthy publication without cancelling raw work. Existing actual replay-helper,
Stop/EOF/Abort/BrokenPipe/root-close tests pass. Initial compile required an explicit
PreviewInput lifetime; corrected without suppression. The first 778-test pass
precedes the maximum-boundary test; the final gate covers all 779.

A same-binary private probe embeds exact old preparation and new Pending code on
five actual pairs, 13 iterations/pair (first warm-up), alternating order. All 65
complete Snapshot encodings match. Producer medians are 0.186–0.224 ms versus old
6.665–6.710 ms; worker 6.662–6.710 ms; new sequential total 6.872–6.960 ms. This
moves work and adds copying/total work, not an application/preview-rate speedup.
Actual payload copied is 4.608 MB/pair. Three-pending RSS observations include all
replay/engine allocations and allocator retention; no whole-app/60-minute proof.

An actual private LP_NUM_THREADS=2 GUI trial saves 32 raw frames/camera and processes
22 pairs, with 10/8 processing drops, one unmatched second input and 1879–2066
candidates/pair (42,593 values). Startup rejects are 1/2, ongoing 0/0. The 68.75%
processed fraction remains below 90%; helper reports seven preview omissions.
Root close during another capture saves nine raw frames/camera and eight pairs
(15,467 values), with one drop/camera, no unmatched input, startup rejects 1/90
and ongoing 0/0. Both pass raw replay/publication/restoration and exact full stream/
illumination GET readbacks, original property 151 restored. Helpers hold camera
FDs, GUI none; all recorded processes exit. Screenshots inspected. One observed
close-to-exit time of 0.334 seconds is not a hard bound. Default-renderer failures
remain unresolved; no controlled whole-pipeline comparison or FPS/p95 pass.

Actual device-free helper replay retains all 32 raw frames/camera, processes three
pairs and explicitly drops 29 processing inputs/camera because replay is unpaced.
It emits three previews/completion. Every actual preview body matches old
preparation on retained input, including statistics/images/positions/timing/counters,
with its runtime omission counter set; processed equals published plus omitted.
Separate checks reproduce 33 reports / 63,924 candidate values, every frame CRC/
sequence/flags/time/PTS reference and full record bytes from the unchanged base
serializer; all three manifests match lengths/CRCs. Same-engine integrity proof,
not calibrated physical truth, tracking/fusion or a complete scan-to-model pass.

See docs/ASYNC_LASER_PREVIEW.md and updated architecture/status/acceptance/roadmap/
benchmarks plus GUI contracts. Source/binary/evidence and commit bindings follow
in the recovery checkpoint. All data/models/screenshots/private probes stay outside
Git/review uploads. Official CodeRabbit 0.9.2 included-quota/billing lookup_failed;
no paid review, upload, review pass, push/PR, merge or protection bypass. Reviewer
files left untouched by development, firmware and physical calibration unchanged; authorized 151 trials restored.

DEV-LASER-PAIRLOG-CAPACITY — additional developer-identified mission gap, unresolved.
Plan::open caps the full required pair log at 1 GiB. The new finite trial averages
1,110,905 bytes/record. Conditionally, at 27 processed pairs/s (90% of 30 FPS),
that cap holds about 966 records/35.8 seconds. This is extrapolation at observed
record size, not a soak test or claim about every scene. Dense 30-minute scans
need a coordinated bounded journal/storage/replay/recovery policy. Preview
preparation and CPU speed alone cannot satisfy that requirement. Do not blindly
remove caps or silently sample observations. No capacity change is implemented;
this neither duplicates nor closes the independently recorded old IR issue 021.

DEV-ASYNC-LASER-PREVIEW implementation commit: `89089708cefcce6e45ce98e7965c741f5673a7b4`.
The later binding changes documentation only; tested Rust and evidence are unchanged.
Independent review remains pending.

SCAN-REV-024 — Accepted. The reviewer independently confirms the required laser
pair-log capacity gap identified in DEV-LASER-PAIRLOG-CAPACITY. Plan::open rejects
budgets above 1 GiB, and every processed pair must append its complete record to
the single capped file. Exhaustion correctly fails publication and retains raw
prefixes/cleanup evidence; it does not provide sufficient dense-session duration.
The reviewer's conditional synthetic-workload arithmetic and our separately bound
actual-data estimate are distinct evidence, not interchangeable measurements.

Proposed correction: design explicit bounded log segments, total storage admission,
versioned indexing/replay/finalization, cancellation and crash recovery together;
preserve raw/complete required observations and compatibility. Validate rollover,
exact boundaries, disk/write failure and sustained resource behavior device-free
before any sustained hardware qualification. Merely increasing the cap or sampling
observations is not the correction. This documentation commit implements no source
fix and does not alter finding status; only the independent reviewer can verify
and close it. Existing 779-test and finite native evidence remain narrow in scope.
Relevant current implementation: 89089708cefcce6e45ce98e7965c741f5673a7b4; response
follows Assignment350's concurrent append, which development preserves read-only.

SCAN-REV-024 — Accepted; segmented storage implementation submitted for independent
verification (finding status remains exclusively reviewer-owned).

Schema 1 keeps its single-file layout, successful record bytes and 1 GiB quota.
Opt-in schema 2 adds explicit segment/record/total-pair/bundle budgets, at most
256 segments, conservative fragmentation admission, raw frame-count admission and
an available-space snapshot before native ownership/control changes. Complete
records are retained; oversize/quota/I/O failures abort successful publication.
The writer checkpoints segment bytes/CRC/count/span before opening the next segment;
read-only complete/recovery visitors validate bounded structure/order/CRC and
identity. Recovery handles a torn manifest, indexed and one unindexed prefix, and
torn final bytes without repair/publication. Source close/settings restoration,
raw validation and no-overwrite finalization remain mandatory. No reconstruction,
association, raw format, exposure, calibration or optional-display policy change.

Changed Rust: workflow `laser_stream.rs`, `laser_stream/bundle.rs`,
`bundle/{io,log,tests}.rs`, `pair_journal.rs`, `pair_journal/{write,read,tests}.rs`;
acquisition `laser.rs` admission ordering; CLI `main.rs` read-only inspection;
app `tests/laser_process.rs` actual schema-2 helper regression. Format/ownership/
limits are in `docs/SEGMENTED_LASER_JOURNAL.md`; root architecture/status/acceptance/
benchmarks/roadmap and native/preview guides link current versus historical scope.
Base commit: `4fa7e6e5b619d55ff2a75951790411cbc1ccf04d`.
Implementation commit: `234c2b6af2309c0bce0bc6e87813276a509b1230`. This follow-up binds documentation only;
tested Rust and executables remain byte-identical to the implementation commit.

Verification:

- Frozen full gate: formatting, strict all-target Clippy, 791 passing workspace
  tests, one explicitly ignored capacity test, successful workspace release build.
  Initial visibility/Clippy/test-assumption defects were corrected; no suppression
  or weakened acceptance threshold. Final authoritative log `verified-gate.log`.
- Separate ignored release capacity test: 9000 synthetic records, 1,182,790,236
  bytes, 71 segments, full streaming readback. Direct final release test executable
  observation: 3.62 s, 5740 KiB max RSS, no compiler included. Initial time wrapper
  included compilation and is not used for journal-only memory claims. This is
  neither a 30-minute hardware scan nor a 60-minute application memory soak.
- Regressions cover quota/record/slot/cancel/create/write/sync failure, poisoned
  finish, rollover prefixes, corruption, symlinks, order, torn manifest and
  mutation. Final-index fsync failure deliberately leaves a visible footer while
  finish returns failure: reader `journal_complete` is observed structure, never
  proof of durability/capture success/restoration/accuracy. Callback data remain
  provisional until final visitor success; numerical results/raw links are not
  established by structural inspection alone.
- Native GUI finite: 32 raw frames/camera, 21 processed pairs / 21 segments,
  22,530,100 pair bytes / 38,616 candidate values; 11 processing drops/camera,
  unmatched 0/0, startup rejects 1/1, ongoing rejects 0/0. Fraction 65.625%, below
  90%. Existing private software-renderer condition LP_NUM_THREADS=2 retained;
  default-renderer/startup failures remain unresolved. No throughput gain inferred.
- Native root close: 9 raw/camera, 7 pairs / 7 segments / 13,238 values, drops 2/2,
  unmatched 0/0, startup rejects 1/0, ongoing 0/0; one stopped incomplete buffer per
  camera. One close-to-exit observation 0.280 s is not a hard bound. Both native
  trials pass raw integrity/publication/restoration and exact full before/after
  stream/illumination GET readbacks, including original scoped property151 restored.
  Helpers own camera descriptors; GUI owns none; all observed processes exit.
  Saved screenshot inspected: both raw views, unverified-unit label and candidates.
- Actual unpaced helper replay: 32 raw/camera, 4 pairs / 4 segments / 7500 values,
  28 processing drops/camera; 4 preview packets plus completion, zero omissions.
  Three prior legacy plus three new bundles: 65 reports / 123,278 candidate values,
  every raw reference CRC/sequence/flags/time/PTS and complete old serializer bytes
  match the unchanged engine. New manifest lengths/CRCs all pass. Same-engine
  equivalence is not independent calibrated ground truth.
- A real-data crash-prefix copy recovers one indexed and one unindexed complete
  record plus 128 torn bytes despite a torn manifest; complete mode rejects;
  all input hashes remain identical after inspection. No raw files repaired.

Private evidence directory: developer state `artifacts/segmented-laser-journal-gd1ijwlc`;
source/binary maps, native readbacks/results, compatibility JSON, capacity logs,
recovery checks and final secret/commit bindings remain outside the repository.
No private raw/model/screenshots/probes are staged or uploaded. New implementation
is independent project code reusing existing Rust processing/CRC/guard primitives;
no new dependency or imported upstream algorithm/license.

Limitations: storage snapshot is not reservation or user-quota enforcement; runtime
record/total quotas can still fail; legacy inspector refuses >8 MiB individual
records; finalization rereads segments and holds O(segment count) descriptors;
maximum-count latency/resource pressure and crash/power-loss durability remain
unqualified. Physical accuracy/units/coverage, tracking/fusion, editable models,
long duration and live processing targets remain incomplete. 005/006/007/021 are
not closed by this work. Only independent review can verify/close 024.

CodeRabbit skill/workflow consulted. Official CLI0.9.2 fresh usage lookup exits1,
all included quota/billing fields lookup_failed. No review request/upload, paid
usage, review-pass claim, push/PR, merge or protection change. Work remains local
and independently reviewable; reviewer-owned findings/charter are untouched.

SCAN-REV-024 — Acknowledgement of independently owned resolution, Assignment354.
Read the current reviewer-owned finding while developing the next milestone. The
reviewer verifies exact234c2b6af2309c0bce0bc6e87813276a509b1230 / fcd8be8ecf4bcd508ba75037fa79fcdf3375d035,
370 focused frozen tests, the separately run 9000-record capacity test, controlled
legacy/segmented bundle equivalence, crash-prefix inspection and a constructed
256-segment reader-boundary fixture. The reviewer marks the specific unavoidable
1 GiB defect Resolved for schema2. Development acknowledges that disposition;
no reviewer file/status/severity/conclusion was changed by development. This does
not qualify sustained hardware, maximum-count capture finalization, power-loss
storage durability, physical accuracy or whole-app memory. Historical responses
and the reviewer's original problem/evidence remain intact.

DEV-PARALLEL-LASER-DETECTION — Developer-identified performance opportunity,
implemented for independent review; not an invented independent finding.

Source: `crates/scancraft-depth/src/laser_pair.rs`, new `laser_pair/detection.rs`,
and `laser_pair/tests.rs`. Base `fcd8be8ecf4bcd508ba75037fa79fcdf3375d035`.
Implementation commit: `bb2d79b49edda77feb2f2c16013ac937550e62e6`. This binding follow-up changes
only developer documentation; tested Rust and executables remain identical.
One scoped standard-library worker detects the first large image while the caller
detects the second, then rejoins in first-camera order. CPU availability >1 is
cached at Processor construction; unknown/single CPU, either image <256*1024 pixels,
or spawn failure selects serial processing. Panic propagation and first-camera
error precedence are retained; no detached borrowed work. Existing detector,
continuity, matching, calibration, raw/pair formats, query limits and raw-before-
preview policy remain unchanged. Progress may interleave approximately, and two
detectors' scratch memory can overlap. No new dependency or upstream algorithm.
Detailed method/tradeoffs: docs/PARALLEL_LASER_DETECTION.md.

Tests and measurements:

- Full frozen formatting/strict all-target Clippy/workspace tests/release gate
  passes 794 tests, with the existing explicit storage-capacity test ignored in
  this ordinary gate (its preceding separate evidence is unchanged). Nine focused
  laser-pair tests pass. Three new regressions check full 480-candidate report
  byte-equivalence/raw preservation, distinct first-camera error precedence in
  both image orders, and pre-cancellation before starting borrowed work. Actual
  thread-creation exhaustion is not forced; fallback is statically inspected.
- Private same-executable detector comparison: five native pairs plus five
  synthetic sizes, 15 alternating iterations each, first excluded from timing;
  every observation, counter, trace and membership entry matches. Native serial
  medians16.463–16.531ms versus concurrent8.598–8.783ms. Tiny64x48 is slower with
  concurrency, supporting a small-image serial path; threshold remains heuristic.
- Exact old depth source at the base is compiled beside new processing against
  identical dependencies in one private release executable. Five actual saved
  pairs x13 alternating iterations x2 implementations =130 complete numerical
  executions equal the saved reports. First iterations excluded: serial medians
  40.090–40.941ms, concurrent32.346–33.165ms; paired median ratios1.231–1.246.
  Raw input loading/JSON output are outside timers; scoped startup/join are inside.
  CPU availability observed16. This is component latency, not a live FPS claim.
- Native finite A/B order serial/concurrent, concurrent/serial, serial/concurrent;
  preserved old App executable and exact tested new App under the same private
  LP_NUM_THREADS=2 software-renderer condition. All runs retain64raw/camera.
  Serial41/41/42pairs(64.0625/64.0625/65.625%); concurrent50/47/49pairs
  (78.125/73.4375/76.5625%). Processing drops serial23/23,23/22,22/22 versus
  concurrent14/14,16/17,15/15; unmatched0/0,0/1,0/0 versus0/0,1/0,0/0.
  Startup rejects serial1/1,1/0,103/92 versus concurrent1/1,1/87,1/0;
  ongoing rejects serial1/1 each versus concurrent1/1,0/1,2/2. These V4L2 buffer
  counts are not physical exposure counts. All six captures pass integrity,
  publication/restoration and full before/after stream/illumination GET equality.
  Helpers own camera descriptors; GUI owns none. Short trials still fail90% and
  retain startup/ongoing rejection issues; do not infer a universal speedup.
- Separate close attempt fails: helper IPC ends before final result, no staging,
  advertised diagnostic log absent. Helper already absent at inspection; after
  the failed harness ends, app/server also absent with exit cause unestablished.
  Full before/after stream/illumination readbacks remain identical. Failure is
  retained as DEV-LASER-HELPER-EARLYEXIT, cause unverified, not marked corrected.
- Environment follow-up: Xvfb cannot write its /tmp PID lock; a direct private
  small write verifies EDQUOT122 while home write/fsync passes. This mission's
  cached Gitleaks executable moves to home cache, identical SHA/mode and original
  path preserved by compatibility symlink; /tmp small write/fsync then passes.
  No unrelated files deleted. A nonroot -nolock attempt is unsupported and is
  retained as a failed private test, not used to weaken authentication. Subsequent
  early XOpenDisplay failure is resolved in the private harness by waiting for
  successful xdpyinfo, rather than only a socket pathname (19 retries observed).
  No production renderer fix or original-helper-exit cause is established.
- Final ordinary authenticated-display close test, with home temporary diagnostics:
  10/9raw,8pairs/8segments,processingdrops1/1,unmatched1/0,startup1/96,ongoing0/0,
  one stopped incomplete buffer/camera. Publication/raw integrity/restoration and
  complete stream/illumination GET equality pass; original property151 restored.
  Close-to-exit0.293s is one observation, not a bound. All observed successful
  apps/helpers exit; accessible camera/USB FD inventory subsequently finds0holders.
  Screenshots inspected: both raw views,1842diagnostic candidates in one finite
  view, explicit unverified-unit warning. No firmware or physical calibration.
- Seven new successful native bundles plus three preceding bundles:310 complete
  records/580698candidate values match the original serializer, all numerical
  results and every raw CRC/seq/flags/time/PTS reference. Additional actual unpaced
  helper replay retains64raw/camera,processes4pairs/7707values,reports59processing
  drops and1unmatched per camera,emits4previews+completion/zero helper omissions.
  It also matches, giving314records/588405values total. All eight new successful
  native/replay manifests pass length/CRC verification. These checks establish
  saved-data integrity, not independent calibrated geometric truth.

Private evidence: developer state artifacts/parallel-laser-detection-8bo89lpy,
including raw profiles/binary/source hashes, initial/retry failures, readiness and
cache migration evidence, actual native/compatibility/replay results. Failed
attempts remain failures. Production Rust is unchanged since the passing gate.
Full selected diff and complete tracked source context pass secret checks before
commit; raw/models/screenshots/probes and reviewer-owned files stay excluded.

Official CodeRabbit0.9.2 usage6236 exits1 with every included-quota/billing field
lookup_failed. A diagnostic recheck10446 using writable home TMPDIR still exits1
with the same fields unavailable; fixing /tmp is not a billing-readiness fix.
No review request/upload/paid usage/review-pass claim/push/PR/merge/protection change.
Independent review of this performance change remains pending. Geometry/coverage,
tracking/fusion, reliable helpers/rendering,90%yield/20FPS/p95,30minscan/60minmemory
and full editable scan-to-mesh release remain incomplete;005/006/007/021 unchanged.

DEV-DIAGNOSTIC-WRITE-PANIC / DEV-HELPER-EXIT-STATUS — Developer-identified defects,
accepted and corrected for independent verification. These are not invented
independent findings. The earlier DEV-LASER-HELPER-EARLYEXIT remains unattributed;
this reproduction does not establish that its cause was stderr failure.

At base `e87c14b6d1294bdeb728191d5f5d81ee180c8674`, real release laser/live helper
processes with deliberately invalid arguments (no device access) normally exit1
with kind6/kind4 error packets. Redirecting stderr to /dev/full instead gives
panic exit101 and no stdout. An initial app-only fix corrects this, but the actual
synthetic capture regression still fails because the workflow's per-pair eprintln
panics the coordinator. That failed run is retained; the test is not weakened.
Also, capture/laser/live Decoder::finish errors discard the already known exit
status on missing final IPC, obscuring normal/nonzero/signal termination.

Implementation: shared `scancraft-core/src/diagnostics.rs::note` bounds each UTF-8
note to4096 bytes and total attempted notes to1MiB/process, ignores optional output
failure and does not retry it. All app helper, workflow and device Drop/unwind
stderr sites use this primitive. Typed raw/journal/transport/cleanup errors retain
their authority; no scanner or reconstruction state is routed through notes.
There is no new dependency or copied algorithm. The existing core crate provides
this small shared leaf utility without introducing a GUI dependency into libraries.

App `helper_diagnostics::{private_log,failure}` replaces temporary diagnostics with
private state logs, checked absolute XDG_STATE_HOME or HOME/.local/state, owner/
mode checks, directory-anchored openat/mkdirat/unlinkat, no-follow leaves, header+
file/directory sync before launch, and mode0700 directory/mode0600 files. Inventory
is bounded1024; pruning retains64 inactive recognized files plus active/new files.
Inherited flock protects a live helper's log, even after the GUI stops waiting.
Unknown names/headers, unsafe files, symlinks and active files are preserved.
No user scan/project or unrelated file is pruned; permissions are never silently
changed. Parent retains an append descriptor and stores bounded supervisor failure
metadata/status. Exit lookup uses try_wait; original ownership/reaping/Stop/Abort
loops remain. Missing/nonregular log paths and observation-write errors are reported
without replacing the actual operation error. EOF protocol errors keep exit status.

Changed sources: core lib.rs/diagnostics.rs; device lib.rs/stream_group.rs/ir_gain.rs/
control/{line_trial,flood_trial}.rs; workflow live_stereo/runner.rs and laser_stream/
runner.rs; app capture_process.rs/discovery_process.rs/laser_process.rs/
laser_process/{helper,tests}.rs/live_process.rs/main.rs, new helper_diagnostics.rs
and its tests; app process integration tests. Documentation links current behavior
and limitations. Implementation: `1507747f46b4b32b3d846f8b26b30a4e3da78ae5`.
This follow-up binds documentation only; verified Rust and executables are unchanged.

Verification:

- Final frozen full gate:808 tests, one existing explicitly ignored storage test,
  formatting, strict all-target Clippy and release build pass, verified-gate.log.
  Initial app-only gate exposes the deeper coordinator panic; core atomic API
  deprecation and an intermediate function-signature mismatch are corrected without
  suppression or lost coverage. Final Rust/binary maps match the frozen build.
- Pure/core tests: bounded valid UTF-8/truncation, exact atomic attempt budget,
  failed writer behavior and oversized JSON formatting. Private-state tests cover
  explicit/absolute paths, symlink/mode/non-directory rejection, persistent private
  files, inherited active-file protection, inactive retention, unknown preservation,
  inventory cap and stored process status. Tests use isolated state fixtures/process
  environments, never mutate process-global environment or prune user diagnostics.
- Supervisor tests retain exit0/42 plus self-SIGTERM from a synthetic script (not
  a scanner owner); failed IPC/abort tests still retain locks until child exit,
  only teardown abandons waiting, graceful Stop still permits finalization.
- Actual subprocess tests prove invalid-request final IPC is byte-identical with
  normal/full stderr, setup failure precedes launch/publication with unchanged
  input, and successful synthetic/full-stderr reconstruction saves both verified
  two-frame recordings with explicit synthetic provenance. No physical fault injection.
- Exact final release malformed helper tests now exit1/send identical error packets
  under both stderr sinks. Actual saved-Raptor replay with /dev/full stderr exits0,
  retains32raw/camera,processes4pairs/7662values,reports28drops/camera,no unmatched,
  sends4previews+completion and zero helper omissions. Original failure class fixed;
  no claim that every possible abnormal helper exit is prevented.
- Final healthy native GUI finite:32raw/camera,25pairs/24segments/46843values,
  processingdrops7/7,pending0/0,startup rejects2/1,ongoing1/1;78.125% below90%.
  Rootclose:10/9raw,8pairs/8segments/14931values,drops1/1,pending1/0,startup0/87,
  ongoing0/0,one stopped incomplete buffer/camera. Close-to-exit0.292310s is one
  observation, not a bound. Both raw integrity/publication/restoration and complete
  before/after stream/illumination GET equality pass; original scoped151 restored.
  Helper camera descriptors positive/GUIzero; all observed helpers/apps exit.
  LP_NUM_THREADS=2 isolated software-renderer condition remains explicit. Earlier
  draft native captures are retained separately and not bound to the final binary.
- Preserved pre-change engine/serializer verifies all37 final native/replay records,
  69436candidate values, every rawCRC/sequence/flags/host/PTS reference and full old
  record bytes exactly. All3 final artifact manifests pass lengths/CRCs. Same-engine
  integrity is not calibrated physical truth. No geometry/threshold/format change.
- Device-free GUI existing-target replay refusal displays real exit1/reason/logpath,
  appends supervisor_failure with that status, retains the log after normal app
  exit, leaves all existing capture hashes unchanged and observes zero scanner FDs.
  Screenshot inspected. Default-state native logs are600 within700 directory,
  survive GUI closure, and are privately copied before future retention can prune.
  Subsequent accessible-FD inventory finds zero camera/USB holders.

Private evidence: developer state artifacts/helper-diagnostics-tzj3ssk2, including
baseline/failing intermediate/final tests, synthetic failures, release probes,
retained state logs, native readbacks and source/binary/commit hashes. Private
models/raw/screenshots/probes/logs are excluded from Git/review uploads. Source
has not changed since the final passing gate; later changes are documentation.

Unsafe review: openat uses a live held directory, validated NUL-terminated leaf and
explicit mode; FromRawFd consumes only a newly returned valid descriptor exactly
once. mkdirat/unlinkat operate on retained directories/validated leaves; managed
header/mode/owner/nlink/lock checks precede retention deletion. flock borrows live
files and survives the stderr duplicate; geteuid has no preconditions. RAII closes
failed-path descriptors. Relevant symlink/permission/retention/inheritance tests
pass. Same-user hostile mutation is not an authenticity boundary or a power-loss
qualification. No new driver/firmware/sysfs/protection change.

Limits: later notes and supervisor records are not individually fsynced; only
setup is synchronized. Pruning is inactive retention, not a global disk quota
against arbitrarily many active/external writers. Other library/panic stderr writers
are outside the core attempt budget. A blocking filesystem/sink can still block;
no hard shutdown guarantee or005 closure follows. Initial log setup can refuse
launch; future log write failure is optional, while required capture errors remain
fatal. Original native early-exit attribution, default renderer/rejection issues,
physical calibration/accuracy, tracking/fusion,90%/20FPS/p95,30minscan/60minmemory
and complete editable scan-to-mesh release remain incomplete.005/006/007/021
unchanged;024 remains independently Resolved per354.

CodeRabbit skill/workflow consulted; fresh22652 usage exits1 with all included-quota/
billing fields lookup_failed. No review request, paid usage, upload, review-pass
claim, push/PR, merge or protection bypass. Independent review of this change remains
pending. Reviewer-owned findings/charter remain read-only and unstaged.

DEV-SUPERVISOR-RECORD-APPEND — Developer follow-up from source inspection: serialize
one complete bounded supervisor record before append, avoiding its small formatter
writes interleaving with live-child notes. No incident reproduction is asserted.
This builds on implementation1507747f46b4b32b3d846f8b26b30a4e3da78ae5; final follow-up
commit and exact source/binary/evidence hashes are bound in the recovery checkpoint.
A repeated808-test full gate passes (append-record-gate.log). Final source/binary
maps and actual runtime probes use this follow-up, with the original150 evidence
preserved separately. The change does not alter scanner ownership or numerical work.

Follow-up native finite:32raw/camera,24pairs(75%,below90%),drops7/7,pending1/1,
startup4/1,ongoing0/0;rootclose10/9raw,8pairs,drops1/1,pending1/0,startup0/5,
ongoing0/0,incomplete1/camera. Close0.293860s is one observation, not a deadline.
Both raw/publication/restoration/fullGET stream+illumination equality pass;
original151 restored. Final release malformed-request stderr normal/full packet
comparison passes. Full-stderr actual saved replay:32raw/camera,3pairs,drops28/27,
pending1/2,3previews+completion/zeroomissions.
Preserved pre-change engine verifies all35 records/65579values, every raw reference
and complete old serializer bytes; all3 manifests match lengths/CRC. The final
GUI-error test shows retained exit1/reason/path and a valid persisted supervisor
record after app exit, with all existing capture hashes unchanged and no scanner
FDs. Screenshot inspected. Source has not changed since the repeated gate.


## SCAN-REV-025 — Accepted; permanent concurrent append regression

The independent reviewer resolved the specific formatter-fragment interleaving
defect in Assignment 359 after inspecting committed 6fc557db5c1b6a46ec5531b7c1590a1cfbbe5563.
This acknowledges the reviewer's decision and supersedes the earlier pending-review
statement for that specific correction; no reviewer-owned status was edited.
The correction's Vec/newline/write_all remains unchanged.

Added `helper_diagnostics/tests.rs::concurrent_child_notes_do_not_fragment_retained_supervisor_records`:
a synthetic child shares the real append description and writes 20,000 short notes
while the real supervisor failure function appends 2,000 bounded JSON records. Every
line must parse as one complete supervisor record or one child note, with all
counts and 384-byte UTF-8 reason prefixes/truncation verified. This independently
written invariant test does not import the reviewer's probe. Focused test and full
812-test/fmt/strict-Clippy/release gate pass. A private harness importing the exact
old 1507747 module (test path adjusted) fails this same permanent regression with
malformed JSON; the old module and failed log are retained. No production rollback
was performed. Commit binding is retained in the
developer recovery checkpoint after commit.

Limits remain: partial writes/external writers/FS faults/power loss can corrupt
best-effort diagnostics; there is no hard I/O deadline. No original native-incident
attribution, numerical/hardware qualification or 005 closure follows. Findings 006,
007 and 021 remain unchanged; 024 remains reviewer-resolved for schema 2 only.

## Developer milestone — per-pixel epipolar constraint reuse

Developer optimization, not an invented independent finding. Source changes:
`scancraft-calibration/src/epipolar_trace.rs` and `laser_trace.rs`, with three new
`epipolar_trace/query_tests.rs` regressions. A query-local OnceCell avoids repeating
first-camera inverse distortion/essential-line computation for every second trace.
Arithmetic, complete results, per-trace cancellation sites/error priority, budgets,
ambiguity and scalar fallback remain unchanged. No new dependency/unsafe/global
cache/format or physical calibration change. Rationale and provenance are documented
in docs/CACHED_EPIPOLAR_QUERIES.md; checkpoint binds final commit/source/binaries.

Validation: full 812-test gate with one existing ignored test, 216 analytic known-point cases;
all 24 retained pairs preserve 63,436 complete queries / 185,094,875 serialized detail bytes,
compact reports and 624 timed numerical executions. Final same-binary production
versus exact 6fc557d baseline copy saves about 4.8–5.9 ms/pair on five reported inputs,
with JSON/I/O/display excluded and compiler namespace effects acknowledged.

Actual baseline/current/current/baseline GUI trials save 64 raw frames/camera and process
49/54/54/47 pairs. Current 84.375% remains below 90%. All helpers exit; GUI owns zero camera FDs;
full stream/illumination readbacks restore; raw integrity and manifests pass.
Current startup rejects 1/0 and 89/88, ongoing 2/2 and 1/1 remain fully retained; no
reliability qualification. Active close saves 8 raw frames/camera and 8 pairs, zero queue drops,
startup 95/92, ongoing 1/1, one incomplete buffer/camera; 0.313477 s close-to-exit is one observation.
The LP_NUM_THREADS=2/private authenticated display readiness condition is explicit.
Inspected saved/active GUI images. The frozen earlier engine/serializer validates
all 212 new native records / 403,462 candidate values, raw references and whole old record
bytes exactly. All five manifests pass lengths/CRCs; final accessible deviceFD
inventory is empty. Private raw/calibration/screenshots/probes stay outside Git.

Fresh repository-scoped CodeRabbit 0.9.2 usage returns 1: included quota, billing and
credits all lookup_failed. No review request/upload/paid flag/push/PR/merge or
protection bypass. This is unavailable review, not a pass. Independent review of
the cache and permanent regression is pending. Sustained processing, calibrated
accuracy, tracking/fusion, full scan-to-editable-mesh and release remain incomplete.


## Independent follow-up — SCAN-REV-025 regression verified

Assignment 360 independently inspected and ran the permanent concurrent append
regression at 285f1914e392ff0f48b62d158bf85ab20804d94d. Its pass supplies the
recommended regression; the original fix remains 6fc557db5c1b6a46ec5531b7c1590a1cfbbe5563.
This acknowledges the reviewer's existing Resolved decision. It does not assert
independent review of the per-pixel cache, hardware performance or this new writer
change. No findings file or severity/status/conclusion was modified.

## Developer milestone — buffered schema-2 record encoding

Developer optimization, not a new independent finding. Baseline commit:
285f1914e392ff0f48b62d158bf85ab20804d94d. Changed `laser_stream/pair_journal/write.rs`
to reuse existing `bundle/io.rs::pair_record` and check cancellation before every
coalesced bounded-sink write. Removed the redundant byte counter. No quota,
checkpoint/format, numerical method, frame admission, queue, preview ordering,
source ownership, dependency or unsafe change. Added two regressions through
`pair_journal.rs` / `pair_journal/encoding_tests.rs`. The final implementation
commit/source/binary binding is retained in the developer recovery checkpoint.

Private measurement preceded implementation. Final probe imports actual production
write.rs, exact baseline writer and exact CappedFile body (scaffolding imports/type
visibility adapted). Five saved typed reports, 13 iterations per path with first
warmup excluded; every segment/index file matches. Final 2 MiB append medians are
4.623–5.256 ms versus 5.590–6.018 ms; separate 16 MiB medians are 1.623–2.083 ms versus
2.680–2.852 ms, with final sealing excluded and reported separately. Instrumented
phase results distinguish encoding from checkpoint costs. No policy change is
misrepresented as an implementation speedup. Same-host/compiler-context limits
are explicit in docs/SEGMENTED_PAIR_ENCODING.md.

Full gate: 814 tests pass, one deliberately ignored capacity test; fmt, strict
frozen all-target Clippy and release builds pass. Explicit capacity test then
passes 9,000 records above 1 GiB with complete readback (57.50 s debug, not a release
benchmark). New tests cover Unicode/escapes across buffer boundaries, exact record
limit/newline and one-byte-short rejection, poisoning, late serializer failure,
mid-record cancellation and unchanged segment/index prefix. No Rust edits follow
the final gate. Initial private harness Digest-field access failed and was corrected
with a local type mirror; failed build and all prototype revisions are retained.

Actual native GUI baseline/current/current/baseline with 2 MiB segments retains
64 raw frames/camera and processes 54/57/58/55 pairs. Current 89.0625% / 90.625% do
not establish the sustained 90% gate. Current drops 7/7 and 6/6, pending zero;
startup rejects 2/101 and 91/1, ongoing 1/1 and 2/2 remain fully retained. A separate
16 MiB request processes 61/64 (95.3125%), drops 3/3, startup 92/1, ongoing 0/0;
it is a distinct checkpoint policy and a single short observation.

Active root close saves 9 raw frames/camera and 9 pairs, zero drops/pending,
startup 1/1, ongoing 0/0, one incomplete buffer/camera; close-to-exit 0.324523 s is
one observation without a deadline. All six runs pass publication/raw integrity
and full stream/illumination GET restoration; helpers own device FDs and GUI none,
all observed processes exit, final accessible-FD inventory is empty. Authenticated
private X11 readiness and LP_NUM_THREADS=2 remain explicit. Saved/active screenshots
inspected. Frozen earlier engine/serializer reproduces all 294 records / 558,364
candidate values, exact raw references and whole old record bytes. All six artifact
manifests pass lengths/CRC. No firmware, physical calibration or installed geometry
change. Private raw/models/probes/screenshots stay outside Git and review uploads.

Fresh CodeRabbit quota/billing/credits lookup returns lookup_failed, exit 1.
No review/upload, paid usage, push/PR, merge or protection bypass; unavailable is
not a review pass. Independent review of this optimization remains pending.
005/006/007/021 unchanged; 024 remains independently resolved only for schema-2
capacity. Original native early-exit attribution, transport/startup reliability,
physical geometry, tracking/fusion, preview latency/FPS, long sessions/memory,
complete editable scan-to-mesh workflow and release remain incomplete.


## Developer milestone — reopen completed laser captures

Developer usability/integrity work, not an invented independent finding. Baseline:
8ebaf26265cfc5ac83088b02d7b50209f088081c. Latest shared review read: Assignment 360;
existing findings/statuses remain reviewer-owned and unchanged. Implementation
commit and exact source/binary/evidence bindings are recorded in the recovery
checkpoint after commit. No reviewer file was modified or staged.

New `laser_stream/bundle/load.rs::load_pair` and `load/record.rs` reopen completed
schema-1/2 captures as one bounded read-only diagnostic pair. Reuse Plan validation,
Guard inventory/CRC/identity, complete streamed journal verification and full raw
Playback indexing, with exact selected-frame references/PTS/host-window checks.
Typed candidate/outcome limits and consistent references/counters precede display.
`candidate_bundle::load_at` routes legacy single-pair and capture layouts; its old
seven-artifact implementation remains. `bounded_json::Items` moves the existing
bounded array reader for actual shared use without changing its behavior.

App changes: CandidateUi one-based Pair selector and completed-capture folder/last
pair presets; background ViewCandidates operation retains the previous view on
failure and labels the actually displayed pair independently of an invalid request.
CLI adds inspect-laser-capture with existing scoped termination monitoring. No
scanner access occurs on opening data. No metric Editor/export, numerical matching,
capture/helper IPC, format/quotas, dependency/unsafe, firmware or physical calibration
change. Source rationale and limits: docs/SAVED_LASER_CAPTURES.md.

Validation: final 824-test frozen workspace gate, formatting, strict all-target
Clippy and release builds pass. Eight new workflow tests cover legacy/segmented
selection/immutability, references/counters, false qualifications, corruption and
layouts, incomplete raw data, early/late cancellation and final mutation, explicit
stored processing failure, array bounds and old restoration metadata. App tests
cover indices, completed-capture presets and preservation of a prior displayed pair.
No Rust source edits follow the final passing label-final-gate.log.

A final release-library probe checks nine older folders / 18 selected views /
26,750 positions, exact observation indices and complete serialized FrameViews
against saved results and raw frames. All 129 original input files retain SHA-256.
A real historical report exposed missing reported_source_restoration: support only
schema-1 absence as explicitly unknown; schema-2 absence or explicit FAIL still
reject. Initial raw-reference failure is retained, not omitted. Initial synthetic
array test hit its byte guard first; increased only that fixture's declared budget
to exercise typed item admission. Corrected an OsString parse compile error and
one strict Clippy comparison without suppressions; all failed logs retained.

Final native finite: 32 raw/camera, 28 pairs, drops 3/4, pending 1/0, startup rejects
82/102, ongoing 1/1. Active close: 9 raw/camera and 9 pairs, zero drops/pending,
startup 81/2, ongoing 0/0, one stopped incomplete buffer/camera; close-to-exit
0.326171 s is one observation. Both raw/publication and full stream/illumination
GET restoration pass, helpers own devices and GUI none. These are functionality
regressions; 87.5% finite processing remains below 90%, with no sustained/physical
qualification. Preliminary native trials are retained separately.

Actual final read-only application restarts open five cases including legacy,
earliest missing-metadata capture and last pairs of both new captures. Real XTest
input switches Pair 1 to Pair 2, then rejects Pair 999999 while retaining Pair 2's
images, 1,945 points and actual source label. All five apps exit normally and have
zero scanner FDs; inspected final images confirm the labels and views. One earlier
harness coordination wait expired while the agent inspected evidence; it closed
normally. Passing retry uses pre-supplied actions; timeout and images are retained.
This is a test harness timeout, not a claimed production defect.

Release CLI opens the last older 61-pair capture as 1,915 points / raw frames 63/63.
SIGINT/SIGTERM tests observe installed handlers and return cancellation exit 1 with
no partial summary. No wall-clock guarantee follows. Frozen earlier engine verifies
all four preliminary/final native bundles: 74 records / 138,873 candidate values,
exact raw references and whole old record bytes. All four manifests pass CRC/length;
final accessible device-FD inventory is empty. Private X11 readiness / LP_NUM_THREADS=2
condition is explicit. No raw/model/probe/screenshot data is committed or uploaded.

CodeRabbit 0.9.2 fresh usage returns exit 1 with all quota/billing/credit fields
lookup_failed. No review request/upload, paid usage, push/PR, merge or protection
change; unavailable is not a pass. Independent review remains pending. 005/006/
007/021 and other existing findings are unchanged. Complete editable scan projects,
calibrated geometry/accuracy, optical binding, tracking/fusion, sustained preview/
processing/resources, transport reliability and final release remain incomplete.


## Developer milestone — verified sequential laser capture reader

Developer integration work, not an invented independent finding. Accepted next
step: expose complete saved Raptor pairs to staged reconstruction/tracking consumers
without repeated raw-recording indexing. Baseline commit
`cbd9f53ee9da567da590c6d54e0fa8b449c6462b`; branch
`dev/verified-capture-sequence`. The implementation commit and tested source/binary
bindings are recorded in the recovery checkpoint and private evidence after commit.
Latest shared findings read: Assignment 360. Existing findings, including
SCAN-REV-005/006/007/021, remain reviewer-owned; no closure is claimed.

Changed sources: workflow `laser_stream/bundle.rs`, `bundle/load.rs` and
`load/tests.rs`; CLI `live_scan.rs` and `main.rs`. `visit_pairs` shares the existing
pair binding/display checks with `load_pair`, verifies the complete journal before
callbacks, reuses two Playback indexes and streams a second journal pass. Callbacks
remain provisional until final identity/counter checks succeed. A later failure
must discard staged output. All stored processing errors remain visible, and their
count must match the terminal report. CLI `inspect-laser-capture DIR --all` keeps
only counters and prints after success. No geometry, acquisition, protocol, GUI
control, format/quota, dependency or calibration changes.

Validation: 828 passed / one existing ignored test; formatting, strict frozen
all-target Clippy and release workspace build passed. Four new tests cover exact
schema-1/2 individual/sequential views, late lineage failure, callback failure,
early/late cancellation, final path mutation, preflight journal corruption,
preserved processing errors and terminal error-count discrepancies. Initial Clippy
reported a needless borrow, corrected without suppression before the passing gate.
No Rust edits after that gate; the existing >1 GiB capacity test was not rerun.

A release-library probe validates nine saved Raptor capture folders: 222 pairs /
391,481 candidate points, exact stored coordinates/observation references and full
FrameViews independently reconstructed from raw frames. The frozen pre-change
selected-pair reference executable also passes those folders. All 199 original
input files retain SHA-256. Complete view checksums match individual traversal on
61- and 28-pair datasets. One elapsed observation, including reference checks,
is 29.748 versus 2.315 seconds for 61 pairs and 3.497 versus 0.752 for 28; no
controlled benchmark or sustained throughput/RSS claim follows.

Release CLI --all matches reference totals. A private copied legacy capture with
a bad second-pair raw reference and updated journal digest fails with no stdout.
SIGINT/SIGTERM tests observe installed handlers before signalling; both return
cancellation exit 1 without a partial summary. No hardware acquisition, GUI launch
or scanner control test was performed in this read-only milestone; previous native
and GUI evidence is historical, not claimed as freshly repeated.

Private exploratory use of the unchanged Tracker: one declared diagonal-relative
policy and at most384 uniformly ordered samples, 61 pairs yield1anchor/57accepted/
2nonconverged/1residual rejection;28pairs yield1anchor/26accepted/1residual rejection.
Poses, residuals, overlaps and reasons retained. The numerical API's mm fields
are used only as unverified supplied-unit distances in that private probe, never
as metric application state. No independently measured trajectory/scale truth,
physical accuracy, fusion qualification or calibrated scanning capability follows.
Method, assumptions and limitations: docs/LASER_CAPTURE_SEQUENCE.md.

Evidence: private artifact directory `capture-sequence-6o6k20vn`, reference-all.json,
single.json, track-final.json, cli-all.json, cli-bad-later-reference.json,
cli-signals.json, gate-fixed.log and retained initial failure logs. Raw data and
probes excluded from publication. Fresh CodeRabbit0.9.2 usage exit1 returns
lookup_failed for included capacity, billing period and credits. Review remains
unavailable: no review request/upload, paid service, push/PR or merge. No protected
review file or main branch changed. Full mission remains incomplete; next work is
verified reconstruction/model binding and staged tracking/fusion with independent
pose truth, not promotion of diagnostic coordinates into millimeter projects.


## Developer mission — first genuine scan integration and finite stereo completion

Accepted the owner's priority: one independently measured Raptor scan and three
repeat trials, not additional editing features. Physical calibration remains
explicitly deferred; docs/FIRST_SCAN_CALIBRATION_PLAN.md is prepared and explicit
owner authorization has been requested, with no reply at this milestone.

Integration baseline1d7380a733e56a988e493279931f1df10eaf0bd6. Audited all30public PR
heads/dependencies/discussion/threads/current-head readiness. All29developer heads
are retained by local dev/first-genuine-scan-integration at
c5c8ed77bdcd5cfc54b49c8cd8506efc34f9aab4 after two history-only merges; the tree is
identical to baselinef3be5e6595fea22595cd0f3b401d2b70d89ec651. PR2 reviewer records
are excluded. Five unresolved GitHub threads on9/12/27/28 remain unchanged, even
where legitimate corrections already exist locally. No finding is closed by this
response. Reviewer statuses002/003/005/006/007/021 remain as published; latest shared
review read360. Current-head readiness passes only10/13/22/23/26 in this snapshot;
this is not owner consent. Full audit and evidence: FIRST_SCAN_INTEGRATION_AUDIT.md
and evidence/first-scan-integration-audit.json. Three initial CI API timeouts were
retained and one read-only retry each returned successful PR-triggered runs.

Developer-identified DEV-FINITE-STEREO-PEER-CLOSE: accepted a correction for finite
CLI camera close/restore overlapping its peer's still-active acquisition. Read-only
USB tracing of the existing kernel monitor observed tiny peer buffers after the
first camera's shutdown; this is distinct from continuing paired truncations.
The existing completion policy allowed independent close at each frame limit.

Changed source: scancraft-cli/src/stereo_probe.rs and its new completion.rs.
Two-worker Completion waits before camera.close while keeping each non-Send
Capture on its owning thread. Existing peer-error/panic/spawn-failure cancellation
and scoped termination release the wait. No new unsafe/dependency, relaxed image
validation, error suppression or recording-format change. Stream-group restoration
remains after worker joins. No live_stereo/laser_stream correction is claimed;
those paths remain separate. Driver/cleanup blocking can still retain ownership
and joins indefinitely, so SCAN-REV-005 is not resolved by this change.

Validation: full integration gate828pass before source edits; corrected finite
capture gate831pass/1existingignored plus fmt/strict frozen all-targetClippy/release.
Three regressions verify first worker waits, failed peer cancellation releases it,
and termination/excess arrivals return errors. No Rust edits after that gate.
No >1GiB capacity test or forced driver hang was run. The exact final source/binary
and implementation commit are bound in the private evidence/checkpoint after commit.

Two initial1200frame/camera30FPS trials: old baseline ongoing89/5 withstartup82/1;
oldUSBtraced ongoing10/95 withstartup0/82. Declared new/old/old/new comparison:
ongoing7/8,9/8,5/95,8/8;startup67/0,1/0,0/81,2/0. All exit0, zero reported missing
metadata, full stream/illumination GETbeforeafter equal. New code avoids the large
observed terminal burst in these trials; old code reproduces it intermittently.
Continuing rejects stillFAIL<0.1%; no reliability/synchronization qualification.
Sampled processRSS/CPU is narrow evidence only, not whole-system or60minute memory.

USB trace909688bulk events has balanced tags, with only ten nonzero(-2) completions
atshutdown.9/10continuing paired rejection byte counts match successful same-PTS
completion lengths minus observed12byteheaders; one disagreement remains. Partial
headers, unretained monitor-drop stats and driver/HCD vsphysical-wire limits prevent
firmware/controller/kernel attribution. Atleast22retained tiny peer rejects follow
the first shutdown; do not infer an exact whole-tail count from capped samples.
No module load, kernel tuning, firmware or permanent control write was performed.

Three further native finite line recordings retain16frames/camera each:96rawframes,
all six recordings passCRC/footer replay; each has15equal-PTS/within1000us candidates.
Temporary property151line trial and full reported stream/illumination states restore.
Last right image inspected: visible stripes/bright spots, not physical surface or
metric accuracy evidence. Initial private harness expected nonexistent report.json;
capture succeeded and was preserved, corrected harness used the actual stdout report
and resumed without overwriting/recapturing. Failure and correction logs retained.
No GUI/edit/save/export or calibrated reconstruction trial was falsely claimed.

Evidence: private first-scan-integration-ao3yvkv3 includes all source/binary hashes,
API snapshots, runner/probe scripts, logs, counterbalance design, USB trace/analysis,
raw captures and replay. Only aggregate source/docs/public GitHub metadata enter Git.
CodeRabbit0.9.2doctor passes9checks including auth/backend/WebSocket; usage still
exit1 lookup_failed forquota/billing/credits. No review was requested or claimed,
no paid usage/upload/push/PR/mainmerge/protection bypass. Independent review pending.
See FINITE_STEREO_COMPLETION.md for exact scope, evidence and remaining defects.

First independently validated object scan remains NOT ACHIEVED. Calibration
approval, physical scale/ground truth, model-bound reconstruction, tracking/fusion
integration, live duration capacity, continuing transport loss and repeated native
project/export qualification remain required. Stop unrelated feature expansion.


## Real board calibration and DEV-STEREO-STATIONARY-ROUNDOFF — 2026-11-03

Developer-identified defect, not a new independent finding. Accepted and corrected
in b417ebd6903772fc1465be107d09c4e1a0522719. A real timestamp-matched stereo fit
reached gradient5.09e-8 (existing final gate1e-6), then failed because rounded costs
could not strictly decrease. `crates/scancraft-calibration/src/stereo_fit.rs` now
lets that current stationary state undergo all unchanged final gates. No candidate
with worse cost is accepted. A fully synthetic regression reproduces the original
plateau and verifies residual/iteration rejection remains. Full tools/check.sh:
832passed,1existingignored,fmt/strictClippy/release PASS. Real retained input now
succeeds and agrees with independent OpenCV fixed-intrinsic fit. Details and exact
scope are in docs/REAL_BOARD_CALIBRATION.md; raw/vendor data remain private.

SCAN-REV-006: accepted qualification gap remains. Owner lifted calibration
restrictions. Native fits and sparse held-out board reconstruction now exist;
physical units, coverage, synchronization, actual object dimensions and three
complete object trials remain unverified. No closure or stable-release claim.
A full-domain fit failure and all excluded markers remain recorded. No relaxed
error/domain gates, synthetic substitutes or fitted-to-ground-truth scale.

CodeRabbit usage check returned exit1 lookup_failed; no review or paid request
performed. Independent review of source commit remains outstanding. No reviewer
record changed; no new finding severity/status asserted. Prior PR28 documentation
propagation remains separate from this integration branch's local correction.

## Native firmware identity observation — 2026-11-03

Developer integration work supporting SCAN-REV-006's calibration provenance gap;
that finding remains accepted and physically unqualified. Source commit
f81353b6ace324396f0b2edc80875060553ebc44 adds fixed GET_STRUCTURE_DATA property1000
in crates/scancraft-device/src/control/device_version.rs, exposes its module in
control.rs and adds the device-version CLI dispatch/help in main.rs. No generic
setter, firmware update, automatic acquisition query or calibration binding is
introduced. Device-reported strings and possibly placeholder serials do not prove
identity or optical accuracy. Provenance/notice retained in docs/DEVICE_VERSION.md
and THIRD_PARTY_LICENSES/OrbbecSDK_v2-protocol-reference/LICENSE-MIT.

Three native queries agree exactly with every decoded field of the retained raw
reference and report firmware1.5.7. The downloaded official1.4.8 package is older;
no update/downgrade performed. Following ten-frame-per-camera native acquisition
passes both replays; startup rejectsL/R0/1, ongoing0/0, missing metadata0/0. Full
reported stream/illumination snapshots before/after equal. No duration/accuracy
claim follows. Three new synthetic decoder/envelope tests pass. Full gate:
835passed,1existingignored,fmt/strictClippy/optimized build PASS.

CodeRabbit usage again returns exit1 lookup_failed; no paid request or review
approval. PR32's earlier f7a9a02 GitHub Rust offline run41609776225 completed
successfully; that result does not cover this new source commit. No PR32 reviews
or review threads were present at the checked snapshot. Source remains available
for independent review; developer has not modified or closed reviewer findings.

## Recorded target observation integration — 2026-11-03

SCAN-REV-006 remains accepted and physically unqualified. Source commit
6697e14355c7b4f2e2affc9bd27e7c372b2bae39 connects recorded genuine board images
to existing calibration observation types through the new workflow
calibration_observations.rs/tests.rs, workflow lib.rs export and CLI main.rs
dispatch. Reuses original component, shape, target and recording code. Exact
request/frame snapshots, full source integrity, explicit frame indices/PTS/window,
shape decisions and unit declarations are retained. Unverified units cannot emit
mm-typed view files. Caller-declared millimeters remain unverified physically.

Four focused synthetic regressions and the complete 839-pass/1-existing-ignored
gate pass, including strict Clippy, formatting and release build. Initial fixture
PTS ordering and one Clippy style issue were corrected before the final gate.
Four retained genuine board poses match every prior raw observation, shape result
and association exactly; source hashes unchanged, eight snapshots replay and
artifact CRC/length checks pass. Counts and reproduction evidence are documented
in docs/RECORDED_TARGET_OBSERVATIONS.md. No new capture, calibration activation,
physical scale/accuracy result or completed object scan is claimed. GUI/multi-pose
fit assembly and full end-to-end qualification remain future work.

CodeRabbit0.9.2 usage returned lookup_failed; no review or paid request performed.
PR32 had no reviews or review threads at the checked snapshot; its prior c292711
CI passed, which does not cover this source commit. Independent review remains
outstanding; no reviewer findings file, status, severity or closure modified.

## Native board preparation panel — 2026-11-03

SCAN-REV-006 remains accepted and unqualified. Source commit
e7679be6384e720cd686b207a4b00279725bfa44 integrates recorded-board observation
preparation into the native app; it does not close physical-accuracy findings.
Files: new app board_ui.rs, main.rs tab/startup/lifecycle wiring, cloud_ui.rs stale
deferral text, workflow calibration_observations.rs/tests.rs, and the existing
private-display rendering tool. No dependency or numerical algorithm change.

A cancellable worker receives published measurements plus exact retained source
frames; display never rereads mutable output files. Saved-directory reporting
survives postpublication preview failure, with prior display identity retained.
Root close/cancel handling includes the worker. Existing CLI artifact bytes are
unchanged on four real saved-board cases. The actual native app produces the same
held-out bundle byte-for-byte. Initial visual inspection exposed the justified
column response-rectangle overlay error; explicit image/overlay allocation fixes
it and the final screenshot was inspected. Missing-source GUI publishes nothing.

Final gate842passed/1existingignored,fmt/strictClippy/releasePASS. Three app tests
cover startup, close confirmation and failure/saved-state handling; strengthened
workflow test checks returned original frames/measurements. Final success/error
private-display runs bind to the final binary; initial and final logs are retained.
No GPU/FPS/live qualification inferred from software-rendered startup checks.
Native GUI multipose fitting/reopening, independent units/accuracy and first real
object surface remain open. Details: docs/NATIVE_BOARD_PREPARATION.md.

CodeRabbit0.9.2 usage returned lookup_failed; no new review or paid request. Prior
b516f27 publicCI41614080917 completed success, not coverage of this new commit.
Independent reviewer files remain untouched; no severity, conclusion or closure
was changed. Existing cleanup005 and broader architectural007 findings remain
separate from this bounded offline worker; no hard cleanup deadline is claimed.

## Recorded board session integration — 2026-11-03

SCAN-REV-006 remains accepted and physically unqualified. Source commit
10049dd46548a73cc27053c615653d5cea01b600 adds workflow board_session.rs/tests.rs,
lib.rs export and CLI prepare-board-session dispatch. calibration_observations
Completed now also transfers its owned request for consistency checks. Existing
CLI and single-pose bundle bytes remain unchanged; no new numerical algorithm or
third-party dependency. Training/validation split is explicit and retained; same
target/unit/profile and per-camera reused-frame checks prevent accidental mixing.
Declared millimeters remain unverified; unknown units cannot produce mm requests
or trigger camera fits. No stereo-domain assumption or capture activation.

Full845pass/1existingignored gate,format/strictClippy/releasePASS. Three new
synthetic-image tests run both camera fits on six training poses and retain one
validation pose separately, with unit/mismatch/reuse/cancel/nooverwrite/digest
checks. Initial /tmp quota failure was corrected by the established homeTMPDIR;
all final checks passed. Genuine three-training/one-validation collection retains
all preceding single-pose artifact bytes exactly, all recursive CRC/length checks
pass, sourceSHAunchanged. A separate unverified-unit fit attempt correctly fails
and retains its partial dataset; no real coordinates were relabeled millimeters
for testing. See docs/BOARD_CALIBRATION_SESSION.md for evidence and limits.

Session GUI integration, held-out scoring, physical scale/stereo qualification and
first measured object scan remain outstanding. Snapshot/source validity is per
pose, not a simultaneous entire-session snapshot. Frame fingerprints do not prove
optical pose diversity or cryptographic identity. No measured RSS/FPS or hard
cleanup deadline claimed. CodeRabbit usage returned lookup_failed; no review/paid
request or approval. Prior ba495f6 publicCI41616702836 completed success. Independent
reviewer records and finding status/severity/closure remain exclusively unchanged.

## Native fixed-camera held-out pose validation — 2026-11-03

SCAN-REV-006 remains accepted and physically unqualified. Source commit
68cf31fc4e6f4871c364c706aad22019903d9a3e adds calibration planar_pose.rs/tests.rs, workflow
planar_pose.rs and CLI validate-planar-pose dispatch; module exports and existing
stereo pose seed visibility change. No existing solver behavior or dependencies
change. Six pose parameters fit against fixed lenses, with explicit supplied-unit
translation and retained per-observation residuals. Existing domain, validation,
normal-system and publication helpers are reused. No auto calibration activation.

Five focused tests and the complete850-pass/1-existing-ignored gate pass, including
formatting, strict all-target Clippy and release builds. Collinearity regression
uses distinct collinear locations; objective gradients agree with finite
differences. Genuine held-out left38 observations give RMS0.135029585575px matching
OpenCV5.0. Fullright36 correctly FAILS inverse lens domain; two observations exceed
training coverage. A separate explicitly retained inner98% subset excludes3 and
fits33 at RMS0.168662045384px, also matching OpenCV. No camera domain expansion or
hidden rejection. Independent native-residual recomputation differs<=1.37e-12px;
sourceSHA, exactrequest and CRC/length checks pass. These are six-parameter pose
fits and numerical reference agreement, not metric accuracy or an object scan.

See docs/FIXED_CAMERA_POSE_VALIDATION.md for inputs, provenance, evidence and limits.
Physical units, broader lens coverage, session/GUI automatic integration and real
object reconstruction remain open. No private observations published. CodeRabbit
usage returned lookup_failed; no CLI review, paid request or approval claimed.
PR32 had no inline review threads at the checked snapshot. Prior15dd52b publicCI
41618852246 completed success, not new-source coverage. Independent reviewer
files/status/severity/conclusions remain exclusively unchanged.

## Real startup sequencing experiments — 2026-11-03

SCAN-REV-006 remains accepted and unqualified. No application-source change in
this milestone. Sourcecf59693664e40507408ce44f10b91c565819ffd0 used for nine short
stereo captures,580accepted rawframes/18recordings. All replayed and reported
stream/illumination readbacks restored. First fresh board view retained38/38
ordinarymarks but moved onlyabout5pixels from priorpose; no diverse calibration
pose or independent physical scale inferred.

Three traced startup runs reproduce errors even when setup finishes before video
start. A controlled60-frame/camera ABBA experiment serializing onlySTREAMON has
startup L/R0/0,0/1,1/104,0/91, ongoing1/1,1/1,0/0,0/0. Serialization does not fix
startup corruption; no production ordering or validators changed. Initial shim
compile/preload failure and its extra uninstrumentedcapture are retained and
excluded from traced evidence. See docs/RAPTOR_STARTUP_INVESTIGATION.md.

SCAN-REV-005 cleanup deadline remains open: restored short-trial readbacks do
not prove hard deadlines. Existing850-test source gate and currentcf59693 CI
41620572099 passed; no redundant fullsourcegate for evidence-only documentation.
No new CodeRabbit review or independent closure claimed. Reviewer-owned files
were not modified or staged. No firmware, kernel, main merge or protection change.

## USB startup framing evidence — 2026-11-03

SCAN-REV-006 remains accepted/unqualified. Application source unchanged at
68edb336e1e31b2fa053646a6179b23a96db15e8. One30-frame/camera native capture
retained60acceptedframes; startup rejects1/96,ongoing0/0,metadata missing0/0.
Both replays and reported format/rate/illumination restoration pass.

Full USB monitor reports24420packets,zero kernel drops,balanced URB IDs and no
truncated successful bulk payloads. Right startup headers are displaced6144bytes
from12288byte transfer boundaries. Offline parsing finds188consistentheaders
and exactly2304000genuine imagebytes. A private finite boundary/overflow model
reproduces31/31left+126/126right dequeued buffer lengths,including97rejected.
Two accepted native images equal USB-derived pixels byte-for-byte. This explains
the observed framing mechanism, not device/HCD root cause or a production fix.
Initial model's omitted overflow clamping is retained and corrected explicitly.

Documentation: docs/RAPTOR_USB_STARTUP_FRAMING.md. No production parser,driver,
kernel,firmware or acceptance-rule change; no rejected data relabeled accepted.
The current board is clipped at the right edge; an owner reposition request is
pending, not assumed fulfilled. Physical scale/object/trials remain missing.
Existing850-test source gate retained; no redundant source gate for docs-only
change. No new completed CodeRabbit review or reviewer-owned record change.

## Consecutive restart and ongoing truncation evidence — 2026-11-03

SCAN-REV-006 remains accepted and unqualified. Unchanged source
c3ba2fd7486a260658ad733f26a6e5660c4ce94b captured three30-frame/camera trials,
180acceptedframes/sixrecordings; replay and reported state restoration pass.
ContinuousUSBmonitor72342packets,zero reported drops,balancedIDs,full successful
bulkpayloads. StartupL/R0/1,0/106,0/98;ongoing0/0,1/1,0/0. Lasttrial followed an
extra3second pause. Rightstartup prefix18432bytes in trials2/3 shifts headers6144
mod12288. ItsPTS equals priorcapturelastLEFT headerPTS after successful endpoint
clear-halt requests. Carry-over is supported; physical storage/rootcause and fix
remain unproven. Waiting3seconds did not prevent the failure.

The paired ongoing rejection has1997100bytes each,exactly matchingUSBpayload sums;
25full payload blocks missing with4.539/4.656ms completiongaps. No zero-outstanding
URB interval within these frames, so no simple queue-exhaustion proof. Separate
queue-depth candidate remains unloaded; no speculative kernel/application fix.
See updated docs/RAPTOR_USB_STARTUP_FRAMING.md and private restart-usb-qitbu8ek.
No validators/counters weakened, no rejected data relabeled accepted. Existing
850-test sourcegate retained; documentation-only publication. No new CodeRabbit
review or independent finding closure. Reviewer-owned records remain untouched.

## Controlled USB queue-depth hardware comparison — 2026-11-03

SCAN-REV-006 remains accepted and unqualified. Source
23d075c30244b489c612b622291142e85b48b5f3 and the unchanged native CLI ran two
ABBA comparisons with the existing GPL-2.0-only experimental kernel candidate.
No application, kernel-patch or validation implementation was changed here.
The local pre-rebrand parameter is raptor_urbs, versus published scancraft_urbs;
explicit name mapping reproduces the candidate sources. A clean-copy rebuild
is byte-identical and the existing policy cases pass79/79.

Depth5:8400accepted/37ongoing rejects (0.438545% proxy); depth32:8400accepted/
0ongoing rejects. All16800acceptedbuffers/16recordings pass full native replay.
Startup rejects467/181 remain counted; all-rejection proxies5.660377%/2.109311%.
Allpertrial IRformat/rate/illumination readbacks pass. One depth32 stream has a
reported sequence gap and doubledPTS step without a rejectedbuffer, after a
153.1ms gap between dequeue calls. This supports investigating application
servicing/backpressure separately; it does not establish the pause's cause.
Selected equalPTS images remain recognizable; the right board edge is clipped.

Original module/parameters and the idle WirePlumber service are restored.
Re-enumeration changed IRformats, explicitly corrected under scannerlocks and
verified against saved native readbacks. Initial unload refusal and the first
smoke harness's empty-trace failure are preserved, not silently discarded.
No forced unload, persistent install, firmware write or main-branch merge.
SCAN-REV-005 hard cleanup deadlines and021 live IRcapacity remain separate/open;
finite1800frame CLI recordings do not resolve those findings.

Changed publication files: docs/RAPTOR_QUEUE_DEPTH_TRIAL.md,
experiments/kernel-queue/README.md, PROJECT_STATUS.md and this developer response.
Private evidence: queue-preflight-v7m5pshf. Full method, resource limits and exact
source/binary bindings are in the trial report. Existing850-test Rust sourcegate
and exact23d075c publicCI passed; no redundant Rustgate for evidence-only docs.
No new independent review closure or qualified geometry claim. Reviewer-owned
files, severities, statuses and conclusions remain untouched.
CodeRabbit usage was rechecked from the repository and returned lookup_failed
for included reviews, billing period and credits; no paid request or completed
CLI review claimed. An initial usage invocation from outside the repository
returned not_git_repository for included reviews and is retained separately.
PR32's current inline-thread list was empty at the checked snapshot, not review
approval; its existing draft review behavior remains unchanged.

## Bounded finite recording and storage-stall validation — 2026-11-03

Developer finding DEV-FINITE-RECORDING-STALL accepted: synchronous Writer::push
on the camera thread makes acquisition vulnerable to storage pauses. This is
not an invented independent finding or proof of the earlier natural153.1ms
pause's cause. A fresh natural trace maxwrite6.470ms did not reproduce it.
Explicit200ms left-write fault injection reproduces two additional sequence gaps.

Source correction2e3e2d8639b091a0001f803c64d1384dd1df2a15 adds original
scancraft-recording::queued::QueuedWriter and integrates it into finite CLI
stereo_probe recording. Capacity8/camera, nonblocking ownership transfer,
explicit poisoned failure on overflow/disconnection, unchanged Writer format/
validation/CRC, joined workers and no successful publication after errors.
stereo_probe/recording::complete closes cameras before draining, cancels peers
and aggregates acquisition/close/write errors. Unwind ownership order retained.
Changed source: recording/src/{lib.rs,queued.rs,queued/tests.rs},
cli/src/{stereo_probe.rs,stereo_probe/recording.rs}.

Full858passed/1existingignored fmt/strictClippy/releasegate PASS. Six worker and
two cleanup regressions include byte identity, blocked storage/capacity, overflow
poisoning, failures/panic, slow sink destructor, ordering and joined cleanup.
Real controllednew/old/old/new300frame/camera trials with200mswrite delay show
ongoingrejects3/3,1/1,2/2,2/2 and sequencegaps3/3,3/1,4/2,2/2. Newwriter services
sixleftbuffers onanotherthread duringeachblockedwrite; oldservicesnone. All2400
acceptedbuffers replay; pertrialsettingsrestore. Naturalnew1800/camera stillhas
12/12ongoingrejects/gaps; this does not fix underlyingUSB failures.

Real600msstall failsqueue explicitly; allfourSTREAMOFFs precede delayedwriteend,
208/212frame prefixes replay, no successdirectory/manifest, settingsrestorePASS.
SIGINTafter100leftframes similarly retains100/101frame prefixes and restores.
Private exactsource/binary/traces/evidence: recording-stall-a0_zqzay. Initial
diagnosticcompilefailure/untracedcapture retained, excluded fromtiming evidence.
See docs/BOUNDED_RECORDING.md for scope, memory cost and reproduction details.

SCAN-REV-005 remains accepted/open: joins/filesystem/kernelcalls have no hard
deadline.006 remains accepted/unqualified;021 live IRcapacity unchanged. This
integration is finite CLIrecording, not alllive/GUIpaths. No independentfinding
status/severity/closure changed and no reviewer-owned file modified or staged.
CodeRabbitusage fromrepo again lookup_failed; no completednewreview, approval or
paidrequest claimed. Prior23d36d7 CI41628369876 passed, not newsource CI coverage.

## Sustained combined hardware evidence — 2026-11-03

SCAN-REV-006 accepted: physical qualification remains unproven. This milestone
tests finite acquisition, not calibrated surface reconstruction or a release.
SCAN-REV-005 cleanup bounds and SCAN-REV-021 live-stereo capacity remain accepted
and unresolved; no independent finding status, severity or conclusion is changed.

Exact unchanged implementation 2e3e2d8639b091a0001f803c64d1384dd1df2a15 at audited
HEAD 2bdb9348bf93af2c6671ace1230a75f82d0565de combined the eight-frame recording
worker with the temporary 32-URB candidate. A 300-frame/camera smoke passed without
rejects. The following 18,000-frame/camera run spanned 600.242/600.208 seconds,
with startup rejects 1/0, ongoing rejects 6/5 and sequence gaps 6/5. All 36,600
accepted frames across four recordings passed complete native replay. Sustained
all-rejection proxy 12/36012 = 0.033322%; ongoing 11/36011 = 0.030546%.
This meets the numerical proxy for one run only, not repeatable qualification.

Five paired truncations remain, plus a distinct left full-size video buffer with
zero-byte metadata at sequence 17575. The latter was rejected for missing PTS;
accepted-frame missing-metadata counters do not erase this failure. Causes remain
unproven. Native filtering/counters were unchanged. The tracer reported no errors.
Whole-CLI max RSS 55,824 KiB; partial 485.102-second capture CPU sample 8.726% of
one core. Resource scopes and all limitations are detailed in the report.

The original installed module, parameters and active WirePlumber were restored.
Saved format/interval structures for all five idle capture nodes compared equal
after restoration; native IR/color/illumination readbacks also matched. Six
beginning/middle/end images were inspected; the right board edge remains clipped.
No firmware write, permanent driver install, application source change or merge.

Changed documents: PROJECT_STATUS.md, docs/RAPTOR_QUEUE_DEPTH_TRIAL.md,
docs/RAPTOR_SUSTAINED_QUEUED_CAPTURE.md and this developer-owned response.
Private source-bound evidence: sustained-queued-hln5i2yy. Unchanged source's prior
858-test gate and exact 2bdb934 CI run41632695649 passed; no redundant Rust suite
was run for this evidence-only milestone. CodeRabbit quota again lookup_failed;
no new completed review or paid request claimed. PR32 remains draft/unmerged;
an empty thread snapshot is not approval. Reviewer-owned files are untouched.

Sustained-evidence follow-up: timing around sequence17575 shows approximately106ms
between video and metadata requeue for index3 at sequence17571, while metadata
poll itself takes about5us. Kernel ACTIVE-state handling suggests a late-metadata
queue hypothesis. This is not a confirmed root cause: synchronous tracer writes
are not independently timed and scheduling/faults remain possible. Queue/thread
trace contexts are retained privately and the report now states this observer
limitation. No acquisition change or independent-review closure is claimed.

## SCAN-REV-026 — accepted documentation correction — 2026-11-03

Accepted. The sentence "Other long writes had no same-camera dequeue inside
their intervals" overgeneralized the inspected examples. A fresh developer
streaming analysis of all 288,119 retained trace rows independently reproduces
the reviewer's counts: 38 writes longer than 33.333333 ms, 26 containing 30
complete successful same-camera video DQBUF intervals, 12 containing none.
Both dequeue timestamps must lie inside the write interval and the camera and
writer thread identifiers differ. The longest write still contains one dequeue.

Correction: docs/RAPTOR_SUSTAINED_QUEUED_CAPTURE.md now gives these counts and
the precise containment rule. This developer response records acceptance;
the reviewer retains exclusive authority to verify and close the finding.
Evidence: private sustained-queued-hln5i2yy/all-long-write-concurrency.json.
Affected original evidence commit e6b8989a3af960c9cfa838ee26e2757c6cef9cd2;
the wording remained in d3992a646720b807de18866427acc08b4879b50c before this fix.
No application source or hardware change; no repeated hardware/Rust suite is
needed for this evidence-only correction. Whitespace and publication secret
checks apply. Causal, observer-overhead and qualification limitations remain.

## DEV-METADATA-REQUEUE — controlled reproduction and correction — 2026-11-03

Developer-identified defect accepted; this is not a new independent finding.
Source 907484e4fa2967e45ca10840fc48f0eed523f124 changes device/src/lib.rs and adds
device/src/paired_dequeue.rs. Video is copied but not requeued until exact-sequence
metadata servicing/requeue completes. Existing wrapping comparison, 32-packet
bound, pending-future handling, validation and rejection counters remain intact.
Copy/metadata errors still attempt video requeue; combined errors retain both
diagnostics and requeue failure cannot be swallowed as cooperative cancellation.

A bounded in-memory tracer removes acquisition-time trace-file writes. Its
4-thread/8000-event offline self-test and strict C build pass. Initial control/
delay/delay/control runs reproduce empty metadata only in both110ms delayed
trials. Corrected/old/old/corrected300-frame/camera runs then report ongoing L/R
rejects0/0,1/0,1/0,0/0 and matching sequence gaps. Each old failure is empty
metadata at the expected later sequence; traces verify actual slot-return order.
The uninjected corrected900-frame/camera run still has paired truncation1/1.
All6600acceptedframes/18recordings replay and all per-trial settings comparisons
pass. Driver/parameters/service/allfiveidleformats are restored after both series.

On the installed baseline driver, SIGINT while a video slot is held completes the
current pair in metadata-first return order, then stops allfourstreams, restores
settings and preserves replayable98/100-frame prefixes without success publication.
The original harness incorrectly expected mid-frame cancellation and failed that
assertion; it remains retained. Corrected offline analysis verifies the finite
CLI's actual frame-boundary semantics. Exact native exit code was not persisted
before the harness assertion; nonzero exit was already asserted successfully.

Full863tests/1existingignored,fmt/strictClippy/releasePASS; five new device tests
cover ordering, sequence/wrap/bounds/stop and failure handling. Private source-bound
evidence: metadata-requeue-bu1_jio4 and metadata-order-ox4xx69i. Report:
docs/METADATA_REQUEUE_ORDER.md. CodeRabbit quota lookup again unavailable; no new
completed review or paid request claimed. Independent implementation review pending.

SCAN-REV-005 and021 remain accepted/unresolved;006 remains physically unqualified.
The independent reviewer resolved026 at87bd080 in Assignment403; development did
not close or modify that record. No independent finding file/status/severity,
firmware, permanent installed driver, protected branch or crate dependency graph was changed
by development in this milestone. Holding a video slot during metadata wait can
reduce video queue slack; no unmeasured throughput or universal cleanup claim.


## DEV-METADATA-SUSTAINED — corrected-source duration validation — 2026-11-03

Developer evidence for 907484e, not a new independent finding or closure.
No application source changes. The same corrected CLI/temporary 32-URB candidate
completes 18,000 accepted frames/camera over 600.275 s. Startup L/R 0/96, ongoing
7/7, gaps 7/7: ongoing 0.038874% but all 110/36,110 = 0.304625% FAIL. Seven paired
truncations remain; no ongoing empty metadata. Trace verifies 36,110 metadata
returns before video QBUF with zero unmatched returns/trace errors. Preflight
300/camera retains startup 0/91 and ongoing 1/1. All 36,600 accepted frames in
four recordings replay CRC/order/footer; all trial/global state and five idle
formats restore. Six selected sustained images show the board, still clipped
at the right camera edge; no new calibration pose or measured object is claimed.

Evidence: private sustained-ordered-s2y99uc1; public
RAPTOR_SUSTAINED_METADATA_ORDER.md, PROJECT_STATUS, ACCEPTANCE_RESULTS, BENCHMARKS
and metadata-order crosslink. The fixed 400,000-row tracer uses 308,800,000 bytes,
passes strict build and four-thread 400,000-event boundary test, and writes its
trace file only at teardown; observer cost remains. Whole-CLI maximum RSS
357,472 KiB includes the tracer; partial 445.064 s CPU is 8.084224% of one core.
No new Rust implementation gate is needed: source 863 tests/fmt/Clippy/release
and 5943cd1 CI pass remain applicable. CodeRabbit quota retry is lookup_failed;
no paid/completed review. The independent reviewer retains exclusive authority;
005/006/021 qualifications remain open. Physical units, a measured object and
repeated native end-to-end scans remain unproven. Reviewer-owned findings and
agent instructions were not modified or staged.


## DEV-STARTUP-RESET — negative controlled recovery evidence — 2026-11-03

Developer investigation, not a new independent finding or closure. No application
source change. The installed baseline driver and corrected source 907484e are
held fixed. Eight ABBA BAAB tests compare idle USBDEVFS_RESET with control; each
has a retained 30-frame/camera seed followed by a 60-frame/camera test. Two of four
successful reset trials still reject 96 and 190 startup buffers. All four controls
also have bursts. A routine production reset is not justified by these results.

All 1,440 accepted frames in 32 recordings replay CRC/order/footer; sequence gaps
match ongoing rejects. Traces retain 20,252 rows with zero errors and 2,770 matching
metadata returns before video reuse. Seven native snapshot groups match after
each intervention and at final restoration; all five idle formats restore, service
is active, module/parameters unchanged. No firmware write or permanent install.

Private startup-reset-b7itq90j binds plan, helper, guard test, all seeds/tests,
analysis and state snapshots. Public RAPTOR_USB_RESET_TRIAL.md, startup-framing
crosslink, PROJECT_STATUS and ACCEPTANCE_RESULTS document failures and limitations.
Source 863-test gate is unchanged; efabf9b CI41646883074 passed. No new independent
approval, physical accuracy or sustained-live qualification is inferred. Next is
complete USB tracing across reset/negotiation/startup; native counters alone do not
prove the earlier shifted-header mechanism in these post-reset failures.

The repository-scoped CodeRabbit quota retry again returns lookup_failed; no paid
request or completed new review is claimed. Independent findings remain unchanged.


## DEV-POST-RESET-FRAMING — traced persistence and SDK lead — 2026-11-03

Developer evidence, not an independent finding or closure. Source 907484e remains
unchanged. A predeclared first-burst/max-four-reset diagnostic stops after one seed
and post-reset test; this is not a rate comparison. Seed startup L/R 106/0, post-
reset 102/94; post-reset ongoing/gaps 1/1. All 120 accepted frames/four recordings
replay. The 49,890-packet USB monitor has zero reported drops, balanced IDs and
complete captured payloads. Reset succeeds; configuration/interface controls fall
inside its call interval, followed by successful UVC commits. Both post-reset
streams retain an 18,432-byte prefix and the 6,144-byte header displacement.

Offline header-chain checks yield complete diagnostic board images without
changing native acceptance. Four accepted sample images equal USB payload removal
byte-for-byte. All seven native snapshot groups, five idle formats, service and
module state restore. Private reset-usb-gixqz92r retains reproducible analysis,
raw traces, recordings and readbacks. Public reset-trial/startup-framing reports,
PROJECT_STATUS and this response record the evidence; no production code changes.

MIT SDK VideoSensor::stop at 85e94c4 has a conditional higher-level vendor stop;
38/39/139 are defined stop property IDs. Raptor read-only probes return status0x2
for all three. Write-only SDK registrations prevent treating that as proof about
setter support. No stop setter is sent. Device-specific operational evidence is
still needed before adopting this lead. Source 863-test gate is unchanged and
6c44cd2 CI41649077142 passes. No independent closure, physical accuracy or release
qualification is claimed.

The current repository-scoped CodeRabbit quota lookup again reports `lookup_failed`
for included reviews, billing period and credits. No paid request, completed new
review or independent approval is claimed.


## DEV-VENDOR-STOP — acknowledged commands do not clear framing — 2026-11-03

Developer investigation, not a new independent finding or closure. Source907484e
remains unchanged. At acquisition head351f53f, a fixed setter probe gets status0
for39 and status2 for139; its next right stream still shifts. A separate eight-arm
ABBA BAAB comparison tests the SDK G2 alias39+38 versus control. All eight setter
requests/replies are byte-matched to USB and report success. All four stop arms
still have startup bursts (100,207,101,190 rejected buffers); shifted headers
remain in every stop arm. No speculative production setter path is added.

All1,440 accepted frames/32recordings replay CRC/order/footer. Full568,622-packet
USB trace has balanced requests, complete successful payloads and zero reported
drops. All32acceptedindex10images equal USB-derived pixels byte-for-byte.
21,062ioctlrows verify2,905metadata returns before video reuse, no unmatched/trace
errors. Seven snapshot groups, five idle formats, service and baseline module
restore. Initial private probe-script indentation failure was caught/fixed before
hardware; the failed text is retained. Private vendor-stop-ou3_pbkp and
vendor-stop-pair-p55hloh5 bind plans/results; public RAPTOR_VENDOR_STOP_TRIAL.md,
reset crosslink and PROJECT_STATUS document scope and limits.

The existing863-test/fmt/Clippy/release gate remains applicable; exact351f53f
CI41652524927 passed. No application-source or dependency change. CodeRabbit
quota remains unavailable from the preceding same-session check, so no completed
new or paid review is claimed. Independent findings005/006/021 remain unqualified
and exclusively reviewer-controlled. A successful command reply does not prove
internal sensor mapping, FIFO flushing or firmware cause. This sequence is not a
reliable recovery; physical units, measured object and three complete native scan
trials remain outstanding. No firmware write, persistent driver install or merge.


## DEV-LIVE-RECORDING — integrate bounded storage into live IR — 2026-11-03

Developer finding/implementation; no independent finding closure. The live runner
still used synchronous recording on camera threads despite the finite CLI fix.
Accept the need to propagate the validated recorder: runner/recording.rs now uses
QueuedWriter8, source cleanup before storage join/fsync, early peer cancellation
on acquisition/cleanup errors, and aggregation of acquisition/close/storage
failures. QueuedWriter::push_copy validates before cloning, sharing owned-admission
logic. Public Source/bundle API and recording formats remain unchanged. Successful
reports add explicit recording queue/completion metadata.

Changed source: scancraft-recording/src/queued.rs and queued/tests.rs; workflow
live_stereo/runner.rs plus runner/recording.rs and recording/tests.rs. Six new
regressions cover blocked storage, overflow prefix/cancellation, panic cleanup,
combined errors, failed sync and borrowed/owned validation/equality. Full869tests
plusoneexistingignored,fmt,strictClippy,releasePASS. Existing repeatedPTS test uses
five quarter-cycle steps instead ofnine three-eighth-cycle steps; still rejects
a full-wrap duplicate hidden by processing drops without relying on writer speed.

Private live-queued-recording-s9me7fm3 binds finalgate/source/probe. Exact worker
with nativeCapture and explicit fixed-no-command control shim:300/camera normal
and200msstall bothcomplete;600msstall fails withqueuefull and208/210savedprefixes.
All1,618savedframes/sixrecordingsreplay. Six/nineleftdequeues complete during the
injected200/600mswrites respectively; allfourSTREAMOFFs precede600mswriteend.
Seven snapshot groups/fiveformats/service/baselinemodule restore. Preflightbuild
failures and firstroot-owned-directory/EACCES harnessfailure retained, corrected
before finaltrial; application source unchanged during finalgate/hardware.

Ordinaryrun stillhasongoing96/1rejects,95leftafterrightvideoSTREAMOFF; overflow
has89rightrejects afterleftSTREAMOFF. These failures are explicitly retained and
make live peer completion/cancellation the next task. Not a lossless scan, full
GUI validation or physical reconstruction. Progress counters mean queueadmission,
not durability; copying adds memory/bandwidth cost. File setup,drivercalls,joins
and sync retain no hard deadline. SCAN-REV-005 and021 remain accepted/unresolved;
006physicalqualification remains open, solely reviewer-controlled. CodeRabbit
quota remainslookup_failed, no completed/paidreview. No firmware/driver/mergechange.
See LIVE_RECORDING_QUEUE.md and updated LIVE_STEREO_SCAN.md/BOUNDED_RECORDING.md.

Implementation commit for DEV-LIVE-RECORDING: `e3db8f9e16ac349e29560757d124a68a7b8182e7`.
Independent verification/closure remains pending.


## SCAN-REV-027 — coordinated live completion and cooperative cancellation

Accepted. Two unique pre-spawn participant slots now coordinate acquisition
completion before explicit source close. Failed-open, unstarted and unwinding
slots cancel and retire, avoiding an absent-peer wait. Acquisition errors still
cancel before completion; a stop flag does not prematurely release an in-flight
peer. Source cleanup remains before recording drain/sync, and all errors are
retained. Additive Source::next_until preserves existing adapters through a default
method; the native adapter forwards internal failure and public GUI stop to
Capture's existing cooperative boundaries. No forced close or driver preemption.

Changed source: workflow live_stereo/runner.rs, runner/completion.rs and its two
test modules, runner/recording.rs and its tests, and acquisition/src/lib.rs. Six new
regressions cover EOF, maximum frames, GUI stop, read error/panic, missing/unwinding
slots, actual open errors/panics, and both cancellation signals during a read.
Existing bundle/writer regressions remain. The full gate passes 875 tests, one
existing ignored test, formatting, strict Clippy and optimized workspace builds.
Initial test Debug-format and malformed synthetic metadata fixture failures are
retained and corrected; production validation is unchanged.

Private live-completion-20261104 binds exact source, libraries and probes. Fixed
corrected/old/old/corrected hardware trials request 300 frames per camera, holding
the right final read until after left reaches 300. Post-peer-STREAMOFF right
dequeues are 0/1/1/0; the old worker's returned last frames remain valid. This proves
ordering, not rejection-rate improvement. Startup rejects are 4/0, 1/1, 1/0, 2/0;
ongoing rejects are 2/2, 2/2, 3/3, 2/2. Corrected 600 ms write overflow fails with
208/209 saved from 209/209 captured, one stopped incomplete right video buffer,
and zero ongoing rejects. Nine left dequeues complete during the write; all four
STREAMOFFs finish before it ends, with no dequeue during/after peer close. All
2,817 saved frames in ten recordings replay. All seven native groups, five idle
formats, service and baseline module restore. No firmware write, reset or installed
driver change. LIVE_SOURCE_COMPLETION.md documents methods, checks and limits.

Only the independent reviewer may verify/close 027. Findings 005 (cleanup deadlines),
006 (physical qualification) and 021 (capacity) remain unresolved. Cooperative
polling cannot bound synchronous driver calls/joins; partial source construction
remains adapter-owned. No complete GUI, reconstruction or physical-scale claim.
CodeRabbit quota lookup failed; no new completed or paid review is claimed.

Implementation commit: `01b183aa9c7d33563e11d7726b892bc0357f6e48`.


Independent follow-up to SCAN-REV-027: assignment427 verified 01b183aa/447d960,
reran the full 875-test gate and independently checked the ten recordings and
trace/restoration evidence. The reviewer, not development, resolved 027. The
exact publication CI41661430467/run231 has now completed successfully. Existing
005/006/021 findings remain unresolved; no broader qualification is inferred.

## DEV-CALIBRATION-HANDOFF — connect fitted observations to reconstruction

Developer integration finding, not a new independent-review finding. Native
stereo fitting produced diagnostic JSON, while reconstruction required RAPCAL01.
Added workflow calibration_candidate.rs and its tests, module registration and
CLI fit-calibration-candidate dispatch/help. The existing fitter operates on
retained supplied observations; the workflow creates and readbacks the candidate
record using fitted geometry. No algorithm, dependency or format rewrite.

Explicit declared millimeters and a unit reference are required; unknown units
are rejected without a scale conversion. Synthetic input always remains synthetic
and cannot pass physical-capture validation. Binding remains declared/unverified,
with existing image-size validation. Exact requests, fit diagnostics and record
fingerprints are retained. All verification/activation/accuracy flags remain false.
No hardware is opened or calibration automatically selected or installed.

Three new tests cover fit-to-image reconstruction, provenance, integrity, invalid
unit/binding/observation refusal, existing-output preservation and cancellation.
Full 878 tests, one existing ignored, formatting, strict all-target Clippy and
release build pass. The actual release CLI fits a synthetic candidate and passes
it through existing reconstruction to three points. The real retained Raptor QR
observations with unverified units are rejected before candidate publication;
original source bytes remain unchanged. This is integration and refusal evidence,
not a real calibrated reconstruction or measured accuracy trial.

Private calibration-handoff-20261104 retains the reproducible CLI script, exact
source/executable hashes and all outputs. Public CALIBRATION_CANDIDATE.md explains
scope and downstream use. No candidate-export GUI control is added in this step.
CodeRabbit quota remains unavailable; no completed new/paid review or independent
approval is claimed. Physical scale, coverage, device binding, measured object
trials and full GUI qualification remain outstanding. Reviewer-owned files were
read only. Implementation commit: `29e3769da87fea02cbfaec5eafff91ca24d988aa`.


## DEV-CALIBRATION-GUI — native fitting and explicit reconstruction selection

Developer integration, not a new independent finding. Extended BoardUi with a
separate candidate view using the existing workflow on a joined/cancellable worker.
Failures and panics retain previous results; error text distinguishes no previous
candidate. The child participates in BoardUi tick, busy state, cancellation and
root close confirmation. Startup accepts --fit-calibration-candidate with explicit
request/output paths. An explicit Use action fills calibration paths in image,
recorded-pair, recorded-sequence and live controls, opening the recorded-pair panel.
It preserves the loaded model, dirty state and other inputs and starts no acquisition.
Relevant busy/closing states block selection. Workflow algorithms/formats unchanged.

Sources: app board_ui.rs and new board_ui/candidate.rs/tests.rs, main.rs,
cloud_ui.rs, live_ui.rs; tools/check_workbench_render.py adds a matching startup
mode. Four new regressions cover previous-result retention on error/panic,
cancellation/duplicate start, startup/root close/busy selection and preservation
of dirty-document/input state. Full 882 tests plus one existing ignored, formatting,
strict Clippy and release builds pass. Prior handoff b92df8a CI41663583357/run233
also passed. No new independent reviewer closure is claimed.

Actual final release GUI on isolated authenticated X: synthetic fitting publishes
four files byte-identical to the prior CLI output; an actual Use click populates
recorded reconstruction, then Live scan shows the calibration with capture idle.
Camera/USB descriptors are absent at recorded checks. Real retained QR observations
with unverified units display rejection and create no candidate directory. Exact
screenshots, source/binary binding and reproducible harness remain private in
calibration-gui-20261104. Initial missing-Xvfb-path preflight and the earlier
ambiguous no-previous-result error label are retained; final gate and screenshots
cover corrections. No user desktop or hardware state change.

This is advanced explicit-request GUI integration, not an automatic multi-pose
wizard or calibrated Raptor reconstruction. Physical units/coverage, actual device
binding, measured object trials and full GUI geometry qualification remain open.
CodeRabbit quota lookup failed; no completed new or paid review claimed. Reviewer
records were read only. Implementation commit: `bb0e3702ac75d494cd542241fdd9d333648a048b`.


## SCAN-REV-028 — accepted: reopen existing collapsed reconstruction controls

Accepted. `default_open` only initializes egui's header state and cannot override
an existing collapsed panel. `CloudUi::select_calibration` now requests one explicit
open; the actual recorded-pair header consumes that request with `open(Some(true))`
and returns to normal user control on subsequent frames. The same request applies
to recorded-pair startup. No forced permanent expansion, acquisition, document
replacement, algorithm or persistence-format change is introduced.

Changed source: `crates/scancraft-app/src/cloud_ui.rs`. The new headless
`selecting_calibration_reopens_visited_controls_without_forcing_them_open` renders
the actual workbench before selection, verifies the destination controls become
visible, clicks the header to collapse it and repeats selection. It checks retained
geometry, selection, dirty state, file path, other recording inputs and indices.
The exact test failed with the old implementation and passes with the correction.
Full `tools/check.sh`: 883 passed, one existing ignored, formatting, strict all-target
Clippy and optimized workspace build passed. No new hardware or physical-accuracy
measurement is implied. Evidence is retained privately in calibration-selection-20261104.

CodeRabbit 0.9.2 quota lookup returned unavailable/lookup_failed for every category;
no new review or paid usage was requested. This is a developer correction awaiting
independent verification; only the reviewer may close the finding. Reviewer-owned
records were read only. Implementation commit: `602b3fb0be1b3bdf3fd2985afa5500aa687e7f87`.


## SCAN-REV-021 — accepted: opt-in coordinated extended IR sessions

Accepted. Schema 1 remains compatible and capped at 1,000 frames. New explicit
schema 2 admits at most 100,000 frames per camera with raw, segmented pair-journal
and total bundle quotas. Admission checks maximum raw-frame/header metadata, segment
slots and available filesystem space before native setup; actual raw/journal and
auxiliary output limits remain enforced. Full reports stream to the existing
segmented writer; only compact lineage and latest tracking remain in the runner.
Existing 100,000-frame session history, bounded queues, PTS checks, error counters,
source completion, restoration and failure publication behavior remain.

Sources: workflow live_stereo.rs, new storage.rs / pair_journal.rs /
runner/pair_log.rs, runner.rs and runner/recording.rs; shared laser journal reader
policy and re-export; acquisition lib.rs; geometry live_references.rs / formats.rs;
CLI main.rs. ORPROJ09 explicitly requires reference schema 2 (100,000 pairs, 16 MiB
metadata, bounded contiguous segment references); ORPROJ08 retains schema 1.
Existing verification, edits, history, reopening and project-bundle export operate
on the complete referenced artifacts. CLI adds read-only inspect-live-journal.
No external dependency, proprietary component or geometry algorithm was added.

Validation: full tools/check.sh passes 888 tests, one existing ignored, formatting,
strict all-target Clippy and optimized workspace builds. New tests exercise the
actual runner beyond 1,000 pairs, rollover, every raw reference, trim/undo/redo,
save/reopen/export, maximum 100,000-pair project metadata, downgrades, bad admission,
required record-budget failure/cleanup, semantic and CRC errors, and interrupted
read-only recovery. The initial exact-three-voxel fixture assumption failed because
repeated alignment near voxel boundaries produces ten voxels; the retained corrected
capacity test verifies reported geometry count and preservation, not accuracy.

A private reproducible release-library trial records/processes 100,000 synthetic
pairs with zero processing drops, nine segments / 603,827,864 journal bytes and a
6,969,211-byte project. Every raw CRC/sequence/index/host link passes; project
load/save is byte-identical. Capture/finalization35.992905s, totalincludingextra
verification42.378556s; time-v peakRSS53,200KiB/user30.10s/system1.07s. These are
64x32 synthetic inputs and ten voxels, not Raptor-resolution performance. Release
CLI confirms complete journal inspection and two-record/three-torn-byte recovery;
complete mode refuses the interrupted fixture. Release GUI opens the maximum-count
project on isolated X with no camera descriptors at the recorded check.

Evidence: private live-capacity-20261104 plus docs/LIVE_EXTENDED_SESSIONS.md.
CodeRabbit quota64188 returns lookup_failed; no new/paid review was requested.
Only the independent reviewer may resolve021. Sustained Raptor throughput/rejects,
90% processing, 60-minute memory, bounded driver cleanup, physical calibration and
three measured object trials remain unqualified. Reviewer-owned records untouched.
Implementation commit: `2cbd5e04d790d17efee9cb16cdb90938e799d38c`.


## DEV-EXTENDED-IR-HARDWARE — genuine budgeted worker validation

Developer evidence supplement to SCAN-REV-021, not an independent closure. No
application source change. Exact recording/completion modules and CappedFile code
block from 2cbd5e04 / 690643c are compiled with a private fixed-no-command Control
shim and native Capture adapter. Optional processing queues are undrained; this
explicitly excludes the full calibrated runner, journal publication and GUI path.

The ordinary trial saves/replays 1,200 frames per camera. Independent streaming
validation compares every saved frame with pre-recording image/metadata CRCs,
sequence, host, flags and lengths. Startup rejects 0/0, ongoing rejects/gaps 12/11;
combined rejected-buffer proxy 0.949237% fails the target. Host spans are
40.371684/40.338363 s; whole-probe wall41.36s, user3.10s, system3.37s, peakRSS58,932KiB.
Equal-PTS candidates: 1,199, host deltas2–159us/median19/p95 80; synchronization remains
unverified. Images show the board with its right edge still clipped.

A deliberate private insufficient-left-budget trial exits2 with a required storage
error, preserves 200 left complete frames +8 torn bytes and 202 complete right
frames, from202/202returned. Normal schema2 admission would reject that configuration.
Separate recovery preserves original bytes and produces valid 200/202-frame files.
All2,802complete original records match the sampled input; no unsaved frame is counted
as successfully stored. Both trials restore seven state groups; final checks restore
five idle formats, service, module bytes/parameters. No firmware/permanent setup change.

Reproducible private live-capacity-hardware-20261104 binds sources, scripts, binaries,
raw/recovered recordings, state and resources. Public methods/results are in
LIVE_EXTENDED_SESSIONS.md#native-worker-validation. Existing full888-test/fmt/Clippy/
release gate is unchanged; exact690643c CI41671761778/run239 passed. Same-session
CodeRabbit quota remains unavailable; no new/paid review. Physical qualification,
sustained reliability, 90% processing, independent assessment and hard cleanup bounds
remain outstanding. Reviewer files were read only.


## DEV-ARCHIVE-WAIT — response to assignment442 CI evidence

Accepted: exact690643c CI is mixed, not all-green. GitHub run41671746358
(push) failed while41671761778 (pull_request) succeeded. The retained failed
log reports177passed/1failed app tests in24seconds; only the ten-second wait
assertion identifies the failure. This does not establish an export defect or a
scheduler-only explanation. No new numbered independent finding is invented.

`crates/scancraft-app/src/archive_ui.rs` test helper now observes completion
before evaluating its deadline, preserves the ten-second bound for pending work,
and reports elapsed/limit, progress, thread completion, mode, inputs, destination
and UI message on timeout before requesting cancellation. Previously it could
assert after tick consumed a valid completion. That is a confirmed helper defect,
not a proven cause of the CI incident. Production worker/export code is unchanged;
cooperative cancellation and Drop joining still provide no hard I/O deadline.

Deterministic regressions feed ready success/error through the actual UI tick at
an expired deadline and check pending-work timeout diagnostics, cancellation and
subsequent completion. Existing real file archive/project-export tests still run.
Local baseline two archive tests passed; initial new-test compilation failed on
struct update syntax for a Drop type, corrected with explicit fields. Focused
four-test run passed. Full gate/publication commit recorded below when terminal.
CodeRabbit0.9.2 quota check returned lookup_failed for every quota/billing field;
no new review or paid request was made, and unavailable review is not a pass.
Original CI delay remains unexplained pending stronger evidence or recurrence
with diagnostics. Only the independent reviewer can assess closure.

Validation completion: implementation `fb7214f8cc74dca27ba260f25d2e9db56b72616c`.
`tools/check.sh` passed890tests/oneexistingignored, formatting, strict all-target
Clippy and optimized workspace build. An additional complete app run constrained
to one local CPU with16testthreads passed180tests in2.33s (whole command2.547s);
this does not reproduce the GitHub host or explain its earlier delay. No deadline
was increased and no validation was removed. Private evidence archive-wait-20261104
retains failed-CI metadata/log, baseline, initial compile failure, focused/gate/
constrained-run logs and exact public source/diff fingerprints. Original CI
incident remains unproven as fixed; current-head CI and independent review follow.


## DEV-IRQ-COMPLETION — genuine transport timing evidence

Evidence-only work on2ce5903; no production changes or proposed independent finding
closure. Both exact-head CI runs41675798228/41675808266 passed, corroborating
assignment446 without resolving the original690643c timeout cause.

Three600-frame/camera trials preserve all3600savedframes with complete native
replay. Initial wrong USB event filter retains IRQ-only evidence; corrected trace
balances228236enqueue/giveback pairs and227370IRQ pairs with no reported loss.
Twelve rejected-frame group gaps span1423–4452us; one IRQ interval is1678us.
Short-transfer grouping matches1200accepted and11/12rejected lengths under the
explicit12-byte header inference. Rightseq451 has20664unexplainedbytes. No status
or UVC payload exists in formatted trace, so no causal driver/device claim follows.

Final header-monitor follow-up loses2497packets and fails the collector gate;
nativecapture exits0 and recordings replay, cleanup restores checked state. Its
ftrace buffer was removed before dumping because the monitor assertion fired;
this limitation and the initial analyzers' failures are retained. All35checked
state-group comparisons match;5formats/service/trace configuration restored per
trial; baseline module unchanged. No hardware fix or physical accuracy claimed.

Source files changed: documentation only, RAPTOR_IRQ_COMPLETION_TRACE.md,
PROJECT_STATUS.md and this response. Existing890test/fmt/Clippy/release gate and
current source CI remain applicable. Private three artifact sets bind actual
commands, data, failed diagnostics and final analysis. CodeRabbit quota unavailable
from last check; no new/paid review. Next: complete status/header evidence with
failure-safe retention, then distinguish handler/controller/device causes.


## DEV-USBMON-RING — complete transfer-header observation

Developer investigation, not independent finding closure. Runtime probe confirms
8192byte tcpdump USB mapping with96bytesnap/-B16384; original private monitor
sets/readbacks64MiB through documented usbmon ABI, drops privileges, retains only
32payloadbytes and preserves complete status/length fields. StrictC11build, framing
and no-overwrite checks pass. No application dependency or production change.

First new600/camera run hascompleteUSB headers but603982ftrace drops because IRQ
affinitywasCPU12 and oldbufferallocationassumed14/15. Collector failure now retains
trace/native output beforecleanup. All-CPU64MiB bufferscorrectobservabilitywithout
affinitywrites: secondrunzeroUSB/ftraceloss,226688bulk completionscorrelatebyidentity/
length/order. ActualUVCheadersmatchall1206accepted/rejectedframes, including6short
frameswith5–15missingimageblocks, status0 andnormalfinalEOF. Gaps2026–3011us;one
IRQspan1915us. Missingbytesoccurbeforeapplicationrecording inthissample, butdevice
versusHCDcauseandprevious20664byte discrepancyremainunresolved.

2400newstoredframesreplay;28state-groupcomparisonsmatch;5formats/service/tracing
restoredpertrial,baseline moduleunchanged. Firstcollector exits1 ontrace-lossgate;
secondexits0. No weakenedvalidation or improvedperformanceclaim. Detailsin
RAPTOR_IRQ_COMPLETION_TRACE.md follow-up;private usbmon-ring-20261104 and
usbmon-ring-allcpu-20261104 bindsource,failures,completeobservations andanalysis.
Unchangednativecode21e85d2passesbothCI41680530781/41680549123 andexisting890-test
localgate. CodeRabbitquotaunavailable;no new/paidreview. Physicalboard-scale question
pending; no inferredmeasurement or physicalqualification. Documentationonlychanges.

Assignment447 recommendation accepted: retain diagnostics when collection fails.
The first new failed-ftrace run preserves its data before cleanup, and both new
paths save native output before monitor checks. The requested missing transfer
status is now retained with UVC headers through the documented USB monitor ABI;
raw ftrace is an alternative, not required to validate these paired observations.
Independent closure remains the reviewer's responsibility. Assignment449's exact
21e85d2 CI completion is acknowledged. A fresh CodeRabbit quota check exits1 with
lookup_failed in every billing/quota field; no new review or paid request follows.

## DEV-QUEUE-HEADERS — larger request queue is not a complete fix

Developer investigation, not a new independent finding or finding closure.
Assignment450's bounded acceptance of preceding header correlation and
failure-safe retention is acknowledged. Both a55eedb publication CI runs
41685116970/41685126431 subsequently completed successfully. Its physical,
startup, causal and exceptional-byte limitations remain applicable.

Using the same verified experimental module in all arms, a temporary 5/32/5
comparison saved 600 frames per camera per arm, with complete replay of all
3,600 frames. Startup rejects L/R: 0/97, 1/1, 94/0. Ongoing: 2/2, 1/1, 2/2.
Every complete rejection proxy fails <0.1%. Depth 32 has one paired short frame
missing five image blocks per camera during 6081/6059 us completion gaps.
Software-pending counts are 32/31 at their ends, not proof of hardware capacity.
No qualified driver fix, performance advantage or causal diagnosis is claimed.

All three USB/ftrace collections report zero loss and balance identities.
All 3,610 accepted/ongoing-rejected frame PTS/FID/EOF/length associations match.
A1 right/A2 left each have 188 unrecognized successful prefixes before the
first accepted frame; these are preserved, not excluded from reported startup
failure evidence. Native startup samples are capped, so full per-buffer startup
reconstruction remains incomplete. All 63 state-group comparisons match, five
formats per trial and the baseline module/service/tracing are restored after
each arm. Final images remain a clipped board without a measured object.

Source files changed: documentation only, RAPTOR_QUEUE_HEADER_COMPARISON.md,
RAPTOR_QUEUE_DEPTH_TRIAL.md, PROJECT_STATUS.md and this response. No production
code or validation changes. Existing 890-test/fmt/Clippy/release gate applies to
unchanged code. Private queue-header-aba-20261104 binds commands, module hashes,
recordings, traces, analyzers, replay and restoration. Publication commit is
recorded in the private publication metadata and Git history. This new evidence
awaits independent review; only the reviewer may change its findings/statuses.

Assignment453 independently confirms the a55eedb CI completion above. Fresh
CodeRabbit repository-level usage exits1 with all quota/billing fields reporting
lookup_failed; no review or paid request was made. An initial lookup outside the
repository also reported not_git_repository and was retained separately rather
than interpreted as account capacity. The corrected lookup remains unavailable.

## DEV-HOST-SERVICE — observed request starvation and broader event gaps

Developer investigation; no independent finding closure. Assignment454's bounded
acceptance of the preceding queue comparison and assignment457's confirmation of
both 4102fd4 CI runs are acknowledged. Existing limitations/statuses remain intact.

One baseline-driver600-frame/camera capture adds filtered camera-copy workqueue,
scheduler and local-timer events. All4,014,236 trace events match retained per-CPU
counts with zero reported loss; USB identities balance, and all1,208 accepted/
ongoing-rejected frames match actual headers/lengths. All1,200 stored frames replay.
Startup94/0 and ongoing4/4 remain reported:102/1302=7.834101%, FAIL.14 state-group
checks and five formats/service/tracing restore; baseline module is unchanged.

All227,666 work cycles,90,486 timer pairs and1,334,445 scheduler switches account
consistently. Three paired short frames overlap3.869–5.318ms without selected events
on any CPU, which is not proof of a platform/firmware cause. A fourth has ten copy
jobs waiting909–1506us and zero software-pending requests for923/895us. One worker
wakes towardCPU1 and first switches in onCPU0 1500us later. This localizes a
software scheduling delay and request starvation in this instrumented sample; tracing
overhead, scheduling cause and the other longer gaps are unqualified.

Changed source files: documentation only, RAPTOR_HOST_SERVICE_TRACE.md,
PROJECT_STATUS.md and this response. No application or kernel implementation edit.
Existing890-test/fmt/Clippy/release evidence applies to unchanged code. Private
host-service-trace-20261104 retains collector/bindings, complete raw evidence,
analyzers and lifecycle self-tests, replay/restoration and viewed final images.
The board is still clipped and no measured object/physical scale is established.
Publication hash is bound in private metadata and Git history. New evidence awaits
independent review; no status/severity change or completed automated review claimed.

Fresh repository-level CodeRabbit usage exits1 with every included/billing/credit
field reporting lookup_failed. No new review, paid request or upload follows.

## DEV-BOUNDED-COPY — prepared opt-in completion-copy experiment

Developer implementation proposal grounded in DEV-HOST-SERVICE's measured
software request starvation, not an independent finding closure or qualified fix.
The GPL-2.0-only patch in experiments/kernel-inline-copy defaults off and selects
only original Raptor IR bulk-input capture, positive/consistent transfer lengths
at most12288bytes, one copy operation and copy length no greater than actual bytes.
It copies, releases the decoder-acquired reference once, clears completed work
and reuses the original atomic resubmit/error path. Ineligible cases preserve
the original zero-operation immediate submit or remaining-operation worker;
cancellation/error handling precedes the new call. Load-time
parameter freeze and read-only sysfs permissions prevent runtime mode changes.

The baseline rebuild is byte-identical to installed579c7633…127e5. Candidate
bcce45bd…76bbbf compiles with matching kernel vermagic/srcversion12034367675945449357638.
Public patch application reproduces the candidate source.54 tests extract the
actual completion/worker/helper/setter functions into strictC11stubs and verify
boundary dispatch, guarded bytes, reference release, fallback, cancellation,
frozen/error paths, submission flags/errors and parameter freeze. The first
test expectation incorrectly omitted unexpected-error fallthrough when frozen;
it was corrected to preserve existing behavior, with the initial failure retained.
Linux patch-check indentation/header issues were corrected; final check has
zeroerrors/warnings. No validation or error-reporting rule was weakened.

Full Rust gate:890passed,1existingignored,formatting/strictClippy/releasePASS.
These stubs do not establish real kernel concurrency, USB/parser semantics,
IRQ latency or hardware improvement. No candidate module has been loaded.
The new copy-byte cap is not a deadline. Longer host-event gaps, startup framing,
the exceptional-byte case and physical reconstruction remain unresolved.

Changed files: experiments/kernel-inline-copy patch/test/license/README,
THIRD_PARTY_NOTICES.md, PROJECT_STATUS.md and this response. No Rust runtime or
installed driver changes. Private inline-copy-20261104 retains initial/final
tests, builds, source bindings and reproduction. Publication hash is recorded
in Git history/private metadata. The proposal awaits independent review;
reviewer-owned records and statuses remain untouched. Fresh CodeRabbit quota
lookup again reports all fields unavailable; no paid or completed review claimed.

Clean patch application and a separate full rebuild reproduce the candidate
module byte for byte. Assignment458 independently verifies the preceding
host-service observations, including the shorter incident's worker/task identity
and queue-starvation accounting, and confirms both e178af3 CI runs passed. That
supports the experiment's motivation, not this candidate's runtime correctness.


## DEV-INLINE-HARDWARE — runtime comparison and framing exceptions

Developer investigation; not a new independent finding or status change.
Accepted assignments461/464's limits: offline source/stub verification and CI do
not qualify kernel concurrency, cancellation bounds or acquisition improvement.
At source08adf1be077be172538131d3accf42f317bb7421, two30-frame/camera smoke trials
and a600-frame/camera off/on/off comparison save/replay3,720 genuine frames.
Four SIGTERM/SIGINT cases restore state and reopen. Installed baseline, service,
five formats per arm and tracing restore;119native state-field comparisons match.

Ongoing rejects per camera2/9/3; startup0/86,1/1,1/0 in the long trials. All total
proxies FAIL. Enabled trace has zero copy-worker cycles but paired truncations
remain. No permanent install, fix, isolated speedup or qualified scan is claimed.
Strict B1header checks retain3semantic exceptions and2length mismatches. Separate
firstEOF/FID accounting explains native lengths, including20,664trailing bytes
in each sequence147buffer. Original analyzers remain unchanged; recycling refuses
B1. Their earlier exit0despite JSON failures allowed A2before semantic inspection;
this diagnostic orchestration mistake is recorded, not reclassified as a pass.
No further hardware trial followed discovery. All saved accepted frames replay.

Changed public files: docs/RAPTOR_INLINE_COPY_TRIAL.md, PROJECT_STATUS.md,
experiments/kernel-inline-copy/README.md and this response. No application/kernel
source or reviewer-record changes. Acquisition revision08adf1b; publication commit
is recorded in Git history and private publication metadata. Freshfullgate:
890passed0failed1ignored,fmt/strictClippy/releasePASS. Exact08adf1b push/PR CI passed,
independently confirmed464; evidence publication CI is separate. CodeRabbit usage
lookup again unavailable; no completed/paid review claimed. PR33threads currently
empty; draft skip is not substantive review. Independent runtime review pending.

Private inline-smoke-20261104/inline-aba-20261104 bind all raw data, restoration,
trace counters, input hashes, initial diagnostic failures and framing follow-up.
Keep005hardcleanup bounds/006physical qualification and prior open limitations.
Remaining: host-wide gaps and observer effects, malformed headers/startup, exact
cause of earlier exception, disconnect safety, physical scale and measured object
reconstruction. Recommend keeping candidate disabled and investigating observed
boundaries with explicit semantic gates; do not weaken frame validation.


## SCAN-REV-029 — accepted: correct IRQ affinity comparison

Accepted. The constant-CPU8 sentence in docs/RAPTOR_INLINE_COPY_TRIAL.md was
incorrect. Independent465 identified B1readbacks8→14 and A2readbacks14→14.
Development reread all ten saved readbacks and independently counted IRQ39entry
CPU fields in all five complete traces, agreeing with saved irq-analysis.json:
smokeoff CPU8:12104, smokeon CPU8:12042, A1CPU8:226539, B1CPU8:228157,
A2CPU14:226565. No exact migration time or reason is established.

The report now provides each arm's readbacks and observed distribution and names
affinity drift as an uncontrolled variable. Measurements, raw evidence, failed
qualification results, application/kernel code and affinity settings are unchanged.
No hardware trial was repeated for this correction. Source publication8a97cb2;
correction commit is recorded in history/private metadata. Changed files are the
report and this response. Private irq-affinity-correction-20261104/verification.json
retains readback/report hashes and fresh raw-trace counts. Markdown whitespace
check passes; unchanged source retains890-test/fmt/Clippy/release evidence and both
8a97cb2CI successes. No claim of independent closure: only reviewer may verify029.


## DEV-OBSERVER-TIMING — baseline capture comparison, not a complete fix

Developer investigation; no new independent finding or closure. Following465's
finite runtime evidence, saved task correlation identifies the local session
logger around14preceding paired incidents. Eight timed readonly snapshots show
lid-state reads taking1.382848–5.122096ms; local ACPI source evaluates_LID.
This is an operation-level lead, not firmware cause or resident-source attestation.

Unchangedbaseline-driver600-frame/camera observer-running/stopped/restored-running
trials produce ongoing rejects per camera4/0/3 and startup96/0,92/95,1/0. All3,600
saved frames replay and3,614accepted/ongoing-rejected header associations match.
CompleteUSB/ftrace accounting and42native state-field comparisons pass. Observer
restoresactive with unchanged source;camera formats/service/tracing restore.
ActualIRQCPUs4/11/9 and readbacks4→4,11→11,9→10 are explicitly uncontrolled.
Separate A2worker starvation remains; zeroB1ongoing loss is finite, not qualification.

Privateadmit.py now refuses the preceding inlineB1semantic/byte failures despite
original analyzer exit0, and accepts precedingA1. Each currentarm passes the
explicit gate before the next starts. No original artifact/analyzer or native
validation rule is changed. No new app/kernel code, monitor patch, firmware/affinity
write or permanentservicechange. Newdocs: RAPTOR_OBSERVER_TIMING.md,PROJECT_STATUS
andthisresponse. Acquisitionhead8a97cb2;publicationhashinhistory/private metadata.
Existing890-test/fmt/Clippy/releasegate applies to unchangedcode;8a97cb2bothCIpass.
Prior029correctionf92cf09remainsforreviewerverification. Newobserverevidenceawaits
independentreview; no completedCodeRabbit/paidrevieworownermergeclaimed.

Nextisolateindividualdirectlidreadsagainstcachedreadcontrols withinonecapture,
with timedcallrecords andrestoreobserver. Physicalscale/measuredobjectandcomplete
native3Dqualificationstillpending; don'tsubstitutethisdiagnosticforagenuine scan.

## SCAN-REV-030 — accepted: distinguish file reads from all operations

Accepted. The observer report incorrectly called the lid read the slowest
operation in every snapshot. Independent assignment472 establishes that it is
the slowest file read, while a timed command takes longer in seven of eight
snapshots. Corrected `docs/RAPTOR_OBSERVER_TIMING.md` to state both facts without
changing measured durations or causal limitations. No application, collector,
raw evidence or reviewer record changed. The correction is based on the saved
operation-ranking evidence cited by the reviewer; no new hardware trial is
necessary for this wording correction.

Affected publication: c907ae55e4f16f1074d813e43ea2e37a809c2a2b. Correction commit
is separately identifiable in Git history. Validation: whitespace/diff inspection;
both exact c907ae5 push41699629965 and PR41699643105 CI runs now pass. Existing
890-test/fmt/Clippy/release evidence applies to unchanged code. This response
accepts the finding; only the independent reviewer may verify or close it.


## DEV-LID-READ-CONTROL — specific host-read lead, strict denial retained

Developer investigation, not a new independent finding or closure. Capture at
c907ae55e4f16f1074d813e43ea2e37a809c2a2b uses the unchanged installed baseline.
All16 predeclared read windows are reported in docs/RAPTOR_LID_READ_CONTROL.md.
Three longest lid reads coincide with all three ongoing rejected pairs; no cached
control read is associated. Actual226,937 IRQ39 entries remain CPU15, readbacks15→15.
This narrows the timing lead but does not prove firmware cause or general reliability.

All1,200 saved frames replay; startup94/0, ongoing3/3 and total100/1300 rejection
proxy FAIL. Left378/right286 retain strict early-EOF/length failures. Separate
first-EOF accounting matches all1,206 native lengths and explains20,664-byte tails
without accepting malformed frames. Admission exits1; no further hardware follows.
All18 input hashes,16read records,14native state comparisons, fullUSB/ftrace counts,
observer/module/trace restoration and unchanged kernel logs are checked. Seven
probe-verifier cases pass. Complete freshRust gate:890passed0failed1ignored,fmt,
strictClippy,releasePASS. Raw evidence and private helper code remain private.

Public changes: new report, PROJECT_STATUS.md and this response; application and
kernel code unchanged. Correction053527d separately accepts SCAN-REV-030, without
claiming reviewer closure. c907ae5 bothCI pass; new publication hash is recorded in
Git history/private publication metadata. New evidence awaits independent review;
no completedCodeRabbit review is claimed. Physical calibration is authorized, but
independent scale, measured object reconstruction and repeatability remain absent.

## SCAN-REV-031 — accepted; correction remains outstanding

Accepted. `tests/live_capacity.rs::Paced::next_until` acknowledges a consumed
processing preview; it does not wait for either independent recording consumer.
The documented eight-frame queue can correctly fail closed before the synthetic
success fixture reaches its1,001-pair assertions. Push41702132020 at a4d2de1 fails
with that error; PR41702142669 passes. The original failure log is retained.
One unchanged local run of all four capacity tests on CPU0 with four test threads
passes in4.34s. That does not correct the missing synchronization or supersede CI.

No production/test source changed for this response. Proposed correction is a
bounded test seam that acknowledges both recording consumers before advancing the
synthetic source, plus a deliberately delayed-writer regression. Preserve the
actual nonblocking eight-frame admission, poisoning, overflow, cancellation and
retained-prefix contracts. Do not replace synchronization with longer sleeps,
larger queues, skipped tests or retries until green. Implementation, a fresh full
gate and new-head CI remain outstanding. Only the reviewer may verify/close031.

## DEV-NO-LID-SUSTAINED — remaining losses and diagnostic correction

Two declared1,800-frame/camera trials at a4d2de17f77fb3cb0d4baef9af467f00fd779d06
save/replay7,200 frames with unchanged baseline driver and temporarily paused
optional logger. First startup107/102,ongoing2/2; corrected startup84/0,ongoing4/4.
Every complete and ongoing-only proxy fails. IRQ trace placement is CPU15 then
CPU10, so no isolated between-trial performance comparison is claimed.

The first USB monitor incorrectly retains its old60-second deadline and exits2;
collector exits1, strict parser refuses, admission remains denied. Manual replay
passes. Separate explicitly incomplete-prefix analysis accounts for observed
rejections but retains47 missing native associations and two partial tails. No
full USB qualification or first-trial kernel-log comparison is claimed.

The private corrected helper uses an explicit bounded monotonic deadline (180s
in the corrected trial). Strictcompile,19deadlinecases,6invalidCLIcases and2PCAP
checks pass. Corrected USB/host traces are complete and all3,600 accepted headers
match, but left321/right239 lack EOF. FID-toggle accounting matches all3,608 native
lengths without accepting malformed frames; strict admission exits1. All four
incident pairs overlap zero software-pending intervals and copy-worker waits.
Complete byte/trace-based software queue diagnostics do not override EOF failure.

All28 native state comparisons across both trials and saved formats/services/
observer/tracing restore; corrected kernel logs are unchanged. Source/driver and
original logger remain unchanged. Public report docs/RAPTOR_NO_LID_SUSTAINED.md,
PROJECT_STATUS.md and this response retain methods, failures, exact counts,
resource scope and CI031. Publication commit is recorded in history/private metadata.
No completedCodeRabbit review or physical reconstruction qualification is claimed.


## SCAN-REV-031 — implemented: bound fixture admission by both recordings

Accepted and implemented on dev/capacity-recording-progress from75759bc. The
synthetic fixture now waits for processing previews AND observed recording-byte
progress on both sides. ActualWriter-derived header/frame lengths and default
BufWriter capacity give a five-frame outstanding allowance below the unchanged
eight-frame queue. Missing progress blocks; ambiguity/errors/cancellation/deadline
remain explicit. This uses real test files and introduces no production hook, API,
queue-size change, blocking admission, longer sleep or retry-until-green behavior.
It is specific to the fixture's constant-size records and current buffered sinks,
not a durable-write or hardware-throughput guarantee.

Changed source: crates/scancraft-workflow/tests/live_capacity.rs and support/
recording_progress.rs,recording_progress_tests.rs. Controlled regression holds
both actualQueuedWriter sinks, observes both sources stop at five, releases left
only and verifies neither source advances, then releases right and replays32
frames/side. Cancellation/expired-deadline regression passes. Existing1,001-pair
CRC/journal/edit/reopen/export/recovery assertions remain; queuecapacity8 is now
explicitly asserted. Fulltools/check.sh:892passed0failed1ignored,fmt/strictClippy/
releasePASS. Six capacity tests on CPU0 with six test threads PASS.

Documentation: docs/CAPACITY_TEST_PACING.md,PROJECT_STATUS.md and this response.
Exact correction commit is recorded in Git history/private publication metadata.
Originala4d2de1 push41702132020failure and PR41702142669success are retained; both
75759bcCI runs pass but do not independently close031. New-headCI and independent
review remain required; only reviewer may verify/close. No hardware action or
physical-accuracy claim follows from this test-only correction.

Negative regression evidence: isolated source copy, separateCargo target, only the
recording-progress conjunct removed. The held-consumer test fails with explicit
queue overflow on both sides and exits101 after10.01s; live worktree unchanged.
This checks sensitivity to preview-only admission, not the exact CI scheduling
cause. CodeRabbit quota/billing lookup remains unavailable; no review upload,
completed automated review or paid usage is claimed.


## SCAN-REV-031 — verification follow-up and separate hardware evidence

Accepted correction remains test-only at dbfc2fa4b966fa6a49cc7688dc8bb451521cebf9.
Independent assignment485 reports it Resolved in the fixture scope; development
has not altered the reviewer's findings. Exact-head push41706905583 and
PR41706960114 both complete successfully. The review-readiness diagnostic still
finds no known completed current-head CodeRabbit review; no merge approval follows.

Separate developer experiment: docs/RAPTOR_COMBINED_ACQUISITION.md records a finite
1800frame/camera candidate-plus-paused-logger diagnostic. All3600 saved frames
replay, zero ongoing rejects/gaps, startup1/1, total2/3602. Complete USB/host and
strict accepted headers pass, checked state/original module/services restore.
Sixteen private inputs bound before capture; original prior failures retained.
No production source change, generalized reliability or dimensional-accuracy claim.
Source files changed in this milestone: this response, PROJECT_STATUS.md and the
new report. Exact evidence publication commit is retained in Git history. Full
physical qualification remains outstanding; independent hardware review pending.

Fresh full tools/check.sh completes:892passed0failed1ignored,fmt/strictClippy/release
PASS. Complete host analysis counts0copy-workers/350653timerpairs; recycling model
self-tests pass and actual338590cycles/side account. Initial missing-directory
analyzer invocations retained and corrected, no raw-data change. CodeRabbit usage
lookup_failed again; no upload or paid review. All report claims are finite.


## DEV-SUSTAINED-COMBINED — developer hardware evidence, not finding closure

Separate declared18300frame/camera capture using the reviewed temporary bounded-copy
candidate and paused optional logger, acquisitionHEAD7947954e3ee81709a0b9ea03565a882207269c7b.
No production implementation changed. All36600savedframes pass native internal and
two external CRC/footer replays; spans610.042251/610.042255s, startup/ongoing rejects
and gaps0/0. Complete13761774USB events and strict36600header associations pass;
21nativefields/fiveformats/originalbaseline+params/logger/WirePlumber/globaltrace
restore. New explicitUSB/native-duration/proxy assessment returns true. No private
host trace or physical-exposure/accuracy claim. Candidate remains experimental.

Private inputs15boundbeforeexecution;54offlineadmissioncases pass, genuine prior
good and missingEOF parser outputs match exactly. Initialnoop fault-injection test
failure retained/corrected without assessment-code change. New public sources:
docs/RAPTOR_SUSTAINED_COMBINED.md,PROJECT_STATUS.md,this response only. No finding
status/severity changed; independent verification of this evidence remains pending.
Prior892-test/fmt/Clippy/release gate covers unchanged implementation; both7947954
CI runs pass, independently confirmed495. CodeRabbit completion remains absent.
Physicalscale/objectreconstruction/threeaccuracytrials/nativeGUI qualification remain
unfinished. Exact evidence-publication commit is retained in Git history.

Native final-frame extraction65910 passes for both full recordings; images viewed,
right-board-edge clipping persists. Fresh CodeRabbit usage85256 exits1 with all
lookup_failed; no review upload or paid usage. No new Rust gate repeated because
application/source/manifests are unchanged from the verified892-test implementation.


## SCAN-REV-032 — accepted; developer evidence correction supplied

Accepted. The original private sustained assessment trusted present command successes
and summary counters without requiring an exact capture, distinct per-camera replay
outputs or reconciliation with actual samples. The independent five counterexamples
are legitimate. Actual capture verification in assignment 496 remains distinct from
assessment completeness; assignment 500 independently confirms both 36e4adb CI passes.

Correction: private sustained-admission-fix-20261104/admit_complete.py requires exact
capture argv/output and one replay per declared path, matching saved/fresh raw
profile/context/count/payload/integrity. raw-check/src/main.rs uses production
IndexedRecording::build for complete CRC/footer/sample validation and guarded source
identity; both paths are rechecked after both reads. Fresh unchanged strict USB
analysis runs in a NEW directory; summaries/rows, actual samples, sequence/metadata
and derived timing/association metrics must reconcile. Original assessment thresholds,
state/restore/collector checks remain mandatory. Original evidence is immutable.
Normal CLI never admits solely from cached receipts; pure functions are test seams.

Tests: fresh validation 84719 exits 0; 36,600 complete raw frames and 13,761,774 USB events
reconcile, input stamps unchanged. Regression 86725 exits 0: 95 new cases (including
all five independent counterexamples, coupled sample/header faults and three
JSON-type regressions),54 original
summary cases unchanged, plus5 actual CLI refusal cases. Private helper fmt/strict
Clippy 54446/release and Python compilation pass. Sixteen correction-source bindings
remain unchanged; they were recorded during validation, not pre-execution attestation.
No application change; prior 892-test/fmt/Clippy/release gate remains the implementation
baseline. No new hardware, physical accuracy or malicious-writer attestation claim.

Public source files changed: docs/RAPTOR_ADMISSION_RECONCILIATION.md,
docs/RAPTOR_SUSTAINED_COMBINED.md,PROJECT_STATUS.md and this response only. The private
helper is not shipped in the application. Exact documentation commit follows in Git
history/publication metadata. Previous capture commit 36e4adbeef88adbaba0a7b7ab3ab5e6b2b02443e
and raw evidence remain retained. SCAN-REV-032 stays Medium/Open until the reviewer
independently verifies it; development does not close or alter the finding.

Limits: finite 30 Hz/GREY/v2 diagnostic, complete ongoing rejection samples required,
no fresh host trace, calibration/units/object accuracy/repeatability still unqualified.
The header analyzer retains its finite in-memory grouping. Source guards do not
establish authenticity against malicious writers. CodeRabbit substantive review is
still unavailable; no paid usage or review/merge approval follows.

Final developer inspection found Python equality could equate False with 0 and
integer with floating-point counts. The first correction and 92-case results are
retained in first-correction-version; a three-case negative control confirms that
gap. Final same() comparison preserves JSON types. Full raw/USB validation-2 and
95-case regressions pass after that correction. Initial CodeRabbit usage was run
outside Git and failed; the corrected repository-scoped lookup also exits 1 with
all quota/billing fields lookup_failed. Neither invocation uploads a review.


## SCAN-REV-032 — independent verification follow-up

Independent assignment 501 marks 032 Resolved within the mandatory private
reconciliation scope at ba99e7bcda45ab16874353fce56c0520931114ed. It independently
rebuilds the helper byte-identically, rereads actual recordings/USB and verifies
95+54 regressions, original counterexamples and five CLI refusals. Assignment 506
confirms both publication CI runs passed. The original summary-only assessment
remains insufficient alone. Development has not modified the reviewer's records.
Substantive CodeRabbit review and broader physical qualification remain outstanding.

## DEV-PHYSICAL-SCALE — frozen caliper interpretation and current scene

Developer evidence, not an independent finding or closure: docs/CALIPER_SCALE_EVIDENCE.md
corrects the earlier overbroad claim that 50 mm caliper observations failed reprojection.
Historical refined factory reference reproduces exactly; corrected native frozen
refined guesses yield 49.492247260 RAW units and pass reprojection. No physical point,
contact-plane or unit qualification follows. Frozen ±1/±5-pixel sensitivity retains
all 32 cases and 8 rejections; no endpoint selection, refit or scale correction.
52 source, 11 analysis and 3 extraction bindings verify unchanged. Actual original
10-frame/side recordings were fully validated and selected manifests/images match.
Same-scene board training prevents claiming complete capture holdout independence.
Private caliper probe calls existing StereoCalibration::prepare/triangulate only.

Fresh baseline-driver scene inspection retains an initial WirePlumber-consumer refusal,
then saves 10 scanning frames/side and 3 color frames after idle-media pause. Native
stereo replays pass, startup 98/1 and ongoing 0/0; nine equalPTS associations, matched
indices 5/6 differ 22 microseconds. Existing extract-frame refuses MJPG, causing collector exit1;
cleanup still restores seven native comparisons, formats and WirePlumber. Existing
export-image then validates/decodes all 3 color frames successfully. No code fix or
clean collector success is fabricated. Images show the board/right-camera cropping;
no measured-object reconstruction. All private inputs/captures remain outside Git.

Files changed: docs/CALIPER_SCALE_EVIDENCE.md,docs/FIRST_SCAN_CALIBRATION_PLAN.md,
PROJECT_STATUS.md and this response. No production/kernel/dependency changes;
unchanged implementation retains the 892-test/fmt/Clippy/release baseline. Relevant
source parent ba99e7b and exact documentation commit are retained in Git/publication
metadata. No new independent finding, metric accuracy, first scan or mission
completion is claimed. Next: actual independently measured object and physical scale.


## DEV-LINE-GAIN — actual measured-object acquisition and sparse reconstruction

Developer work, not a new independent finding or closure. Accepted practical need:
combine existing scoped line illumination and raw gain so real object stripes are
usable. `line_options.rs`, CLI dispatch and `stereo_probe.rs::record_line_gain`
expose the already implemented dual-lease path without changing device-control,
recording or geometry implementations. Regression file `native_laser_preflight.rs`
adds actual malformed/unsupported CLI refusals; parser unit tests preserve defaults
and reject ambiguity. Source parent 876c3af5e49214d466534cbcca25c24de67c7956;
exact new commit is available in Git/publication metadata.

Tests: full final gate 895 passed/0 failed/1 ignored, fmt, strict Clippy and release
PASS. Initial chunks_exact Clippy failure retained and fixed with as_chunks. Actual
10-frame/camera combined capture plus 3 color frames validates/replays; 0/97 startup
and 0/0 ongoing rejects remain reported. Gain and line restore, seven native state
comparisons/five format restores/original WirePlumber state pass. Matched indices5/3,
equal sensorPTS and25us host difference do not establish exposure synchronization.
Private verification and manifest are detailed in docs/LINE_CAPTURE_GAIN.md.

Frozen existing native reconstruction passes produce only8/5/5 points; no credible
surface, physical dimension, repeatability, metric export or first genuine scan is
claimed. Owner dimensions are held out, no model/scale/acceptance fitting performed.
Private probe code/data are not shipped. Source/lens domains remain unchanged.
Immediate priority is robust real correspondence and coverage. CodeRabbit usage
lookup exits1 with all quota/billing fields lookup_failed; no review/paid usage
requested. Substantive review remains outstanding. Independent507's verification of
prior caliper evidence and CI is preserved; reviewer-owned findings/statuses untouched.


## DEV-LINE-GAIN — publication and trace follow-up

Implementation ebb77a2954f8563d15e28f2d0b02928da0835cd2 is pushed in draft PR37;
both exact-head PR41787075489 and push41786941673 CI pass. Independent and CodeRabbit
review of this change remain outstanding. No merge/protection changes.

Additional private all-trace diagnostic preserves396queries/709candidate points,
including321queries with two valid intersections. No best match, measured surface,
physical units or dimension error is claimed. Native detector initial100000-observation
cap failure retained; stronger contrast gates reduce noise without increasing caps.
Existing native detector/continuity/epipolar/triangulation implementations are used
unchanged; private probe Clippy/release and input/pair/count reconciliation pass.
All known limits and remaining trace-identity requirement are documented in
LINE_CAPTURE_GAIN.md. Next investigate native projector calibration from retained
board data, preserving historical factory-coordinate distinctions and failed anchors.

This follow-up changes only LINE_CAPTURE_GAIN.md, PROJECT_STATUS.md and this response.
The895-test implementation gate remains applicable; no redundant Rust gate was run
for prose. Exact documentation commit follows in Git. Findings/statuses belong to
the reviewer and remain untouched. Current CodeRabbit lookup failure is already
recorded; no repeated paid review or review success is inferred.


## DEV-NATIVE-BOARD-SHEETS — failed provisional transfer retained

Developer investigation, not an independent finding/closure. Independent516 reproduced
the prior396queries/709candidates and518 verified both5643105 CI runs; record their scope
without changing reviewer records. New private board-plane/sheet work is documented in
NATIVE_BOARD_SHEET_PREPARATION.md. It uses unchanged native detector, association,
triangulation, plane fitting and ray-intersection APIs. Original anchor/stability
failures and five far lens-domain rejections remain. Saturation exclusion strengthens
quality checks; no limit is relaxed. Preparatory filtering/grouping is private,
exploratory and not a production scanning feature.

Evidence:80 actual GREY frames fully validated during native component measurements;
all marker requests reproduce; private Rust Clippy/release passes; independent NumPy
plane/projection comparisons agree numerically.32/27 native line-on marker points
support board planes,944/703 stripe centroids intersect them. All25 cross-pose sheet
hypotheses retained; three provisional models reject every prior709 box candidate at
0.75RAW. Closest24.129303209RAW. No dimension fit, sheet activation, correct stripe
identity or first genuine measured scan claimed. Additional board views/coverage and
model assumptions remain the next task, with source units and physical accuracy
unqualified. Manifest/source/procedure references are in the document.

Only docs/NATIVE_BOARD_SHEET_PREPARATION.md, PROJECT_STATUS.md and this response change
publicly. Production source remains ebb77a2; previous independent895-test/fmt/Clippy/
release gate applies. No redundant full Rust run for prose. Exact documentation commit
will be recorded by Git/publication metadata. Reviewer files untouched; no hardware,
firmware, paid service, protection or merge action. New evidence awaits independent
review; no substantive CodeRabbit completion inferred.


## DEV-BOARD-CURVATURE — supported fragment, physical qualification incomplete

Developer investigation, not an independent finding or closure. Accept independent519's
verification of the prior87ce1f4 board-plane and failed transfer evidence and its stated
limits. Reviewer findings/statuses remain exclusively reviewer-owned and untouched.

New private experiment preserves that failure and adds426 actual edge-board observations.
All30 native planar pair fits retain the same quality gates; added5/3 fails. A separately
planned linear/quadratic comparison uses unweighted centered/scaled NumPy least squares,
twofold100-row spatial holdouts in both poses, all samples and the declared per-pose
X-residual/mutual-minimum gates. Four quadratic hypotheses pass; addedpair held-out RMS
is0.060876645RAW versus0.253399768linear. Shared camera/board error and two-pose coverage
remain limitations; this is not an independent physical validation or production fitter.

The private Rust probe uses existing bounded PreparedCamera and QuadraticSheet APIs on
all709 prior candidates.201queries have one candidate/sheet combination,195none,0multiple;
all survivors form one stripe. Stereo geometry is preserved, no snapping or dimensionfit.
Every2,836sheet result remains; independent numerical formulas reconcile roots/projection,
residuals and all30 plane fits. Private fmt/strictClippy/release pass. Original edge exits
were not recorded; later native reruns have exit0 receipts and byte-identical results.
Full procedure, immutable36file manifest, sources and bounds are documented in
NATIVE_BOARD_SHEET_PREPARATION.md. Raw evidence/private probe are not published.

Public changes only PROJECT_STATUS.md, NATIVE_BOARD_SHEET_PREPARATION.md and this response.
Application/dependencies unchanged from ebb77a2954f8563d15e28f2d0b02928da0835cd2; independent
895-test/fmt/Clippy/release result applies. Exact documentation commit follows in Git.
No repeated full Rust gate, metric export, measured dimensions, full surface, GUI scan,
physical accuracy, reviewer closure or substantive CodeRabbit completion claimed.
No new hardware, firmware, paid service, main merge or protection action. Immediate
priority remains additional real sheet coverage/held-out poses and physical validation.


## DEV-NATIVE-DOMAIN — held-out checks reveal full-image integration failure

Developer-identified integration gap; no independent finding/status change. Accept
independent522's scoped reproduction of2552411/201-point evidence and CI confirmation.
New NATIVE_LASER_DOMAIN_FAILURES.md documents the subsequent evidence and limitations.

Additional rotating-board sample1 supplies24 original unsaturated marker points;
original four-anchor failures, sample0/8 insufficient support and one native inverse-
domain rejection remain. Coarse opening is used only for identity. Three frozen sheets
pass conservative held-out checks; the fourth fails its edge mask maximum. The initial
small-component hypothesis is disproved and retained. Unioning every original shape-
passing component outline with the opened masks removes two contaminated observations;
allfour frozen checks pass without sheet refitting or threshold/domain changes. This
mask correction is data-informed and not pristine physical qualification.

Actual unchanged native full-image processing publishes zero candidates/2,206errors.
All data/CRC/source/count checks pass; publication exit0 is not reconstruction success.
Native audit explains268first-ray,546root-projection and1,392trace-domain failures,
reconciling every error. Right traces0/2/49 have212/46/23invalid vertices; trace0also
has170valid vertices. Strict whole-trace/root errors block otherwise valid hypotheses.
Next implement explicit domain-rejection/contiguous-segment handling with full rejected
source provenance, no bridging, no silent ambiguity loss, preserved resource/cancellation
failures and strict legacy compatibility; replay this exact recording after changes.

Only docs/NATIVE_LASER_DOMAIN_FAILURES.md, PROJECT_STATUS.md and this response change
publicly. Source stays ebb77a2, independent895-test gate applies; no redundant fullgate.
Private probes fmt/Clippy/release and numerical/raw reconciliation pass,223-file
manifest/source refs in the document. New evidence awaits independent review. Exact
publication commit follows in Git. No production fix, first scan, nativeGUI success,
physical accuracy, paid review, merge or independent closure claimed.


## SCAN-REV-033 — accepted; explicit domain correction awaiting independent verification

Accept assignment524's High/Open finding and causal reproduction. Implemented on
`dev/domain-bounded-laser` from baf1d6dc9e1011d95a6022ecc5d091d3a2acdb1b; correction
commit 8e2826052a8f323e49a8008b9d02ee822807b56c. The developer does not change severity/status or claim closure.

New `calibration/laser_trace/domain.rs` records every finite in-image inverse failure
and adjacent segment, splits valid contiguous runs, preserves original indices and
all valid roots/ambiguity. First-ray and malformed-input/overflow/cancellation/resource
failures remain failures. Shared `laser_trace.rs` selection protects strict behavior.
`depth/laser_pair.rs` adds explicit builder selection, journal exclusions and unchanged
conservative budget policy. `workflow/recorded_traces/automatic.rs` adds default-false
request opt-in and complete diagnostic provenance; `candidate_bundle.rs` and app
`cloud_jobs/candidates.rs` validate/display conditional-domain summary on processing
and reopening. Tests in those modules and `laser_pair/tests.rs` cover the contracts.
No dependency, live default, lens domain, quality threshold or private model changed.

Nine added regressions; full904passed/0failed/1ignored gate, fmt/strictClippy/release PASS.
Exact original strict replay yields all eight byte-identical failed artifacts. Opt-in
full-image replay gives336candidates/268first-ray errors/1,602without unique match;
281vertex/279segment/596root exclusions retained. Every4,040crossings and selection
reconciled by developer NumPy formulas with pre-fix native ray/root audit. Exact
12,297,860projected bound accepts unchanged numerical artifacts; one less rejects
before publication. Native GUI processes bound retained frames, displays336points
and exclusions, with six byte-identical numerical/input/frame artifacts and no
scanner descriptors. Initial Clippy/default-Xvfb harness failures and corrected runs
are retained, not omitted. Full source/method/manifest details are in
DOMAIN_BOUNDED_LASER_MATCHING.md; private76-file evidence is not published.

Limitations: conditional numerical uniqueness is not physical line identity. Only201
points belong to the previously identified narrow box stripe;135other scene candidates
remain unqualified. No complete object, verified metric scale, measured accuracy,
three-trial reconstruction, metric editing or full export workflow is claimed. Existing
006/acquisition/cleanup findings remain unaffected. CodeRabbit usage returned all
lookup_failed; no included-capacity/billing proof, review upload, paid service or
substantive CodeRabbit completion. Independent verification of033 correction pending.


## DEV-BOX-SURFACE — measured-object evidence, not a new independent finding

Accepted follow-up to the first-scan qualification work. No production source
changed from 37b8dd6cb3f5e26eeaa6aeeeca48af34289e8bdb. New documentation:
`docs/BOX_SURFACE_VALIDATION.md`, `PROJECT_STATUS.md` and this response, on
`dev/box-surface-evidence`. The associated documentation commit is recorded in Git.
Private capture/probe/source bindings and the 524-file evidence-manifest hash are
specified in the report; private images/models are not published.

Three real line captures give 612/1030/593 diagnostic candidates, all 2235 independently
numerically reconciled; selected inputs/CRCs/outcomes and restoration receipts pass.
Actual native GUI opens the 1030-point result. Private native probes pass fmt/Clippy/
release; surface residuals match NumPy SVD. One 199-point top patch passes provisional
raw-unit gates; five other surface cases fail. Putative front selection may mix faces:
calibration is not established as the sole cause. Retain original corner/refinement/
edge-support failures, gain16000 rejection and high-gain ongoing transport rejection.
The mistaken GREY-only absence interpretation was corrected by fresh color evidence.

Assignment528 independently resolves SCAN-REV-033; this response acknowledges that
reviewer decision without editing its record. The scoped fix and its independent
904-test/fmt/Clippy/release gate do not close physical metrology or other findings.
No complete object, qualified units/accuracy, three repeatable scans or native
metric edit/export is claimed. Owner dimensions remain held out. CodeRabbit usage
reports unavailable lookup data; no substantive review or paid upload is claimed.
Next investigation must separate surface selection/correspondence from model error
and establish independent scale, rather than forcing the box to known dimensions.


## Assignment530 provenance qualification — accepted; fresh reproduction supplied

Accept the historical corner execution-binding discrepancy. The retained source
hash a26ce845 differs from the receipt's c25b74ff; the transition is unknown, and we
do not assert a formatting-only cause. Original receipt/source/results remain
unchanged. New private `corner-provenance-reconciliation-20261104` records all
inputs before a fresh isolated frozen-dependency strict Clippy/release build and
execution; exit0 reproduces original results byte-for-byte and all input hashes
remain unchanged. This is current-source reproduction, not historical attestation.
See BOX_FACE_REPEATABILITY.md for full hashes and the new evidence manifest.
Only the independent reviewer may assess this response or change finding status.

## DEV-BOX-FACE-ASSIGNMENT — corrected interpretation and new capture evidence

Image averaging for display reveals that the earlier lower group mixed the left
face (sheet0) with the right face (sheets1/2). Original failures remain. All18pairwise
plane hypotheses are retained, and native image-component comparisons support
ambient/line image alignment without a warp. No original candidate or calibration
coefficient changes. A new plan freezes corrected grouping and all old numerical
gates BEFORE three fresh gain15000 line captures.

All six top/right-face fits pass provisional raw-unit gates; all three single-stripe
left-face cases fail coverage. Full native outputs1051/1046/1059 retain failures and
other scene candidates; all3156 positions/source decisions reconcile numerically.
All80rawframes replay, original device states/formats/service restored. Startup
rejections remain; these short trials do not qualify sustained transport. Fixed-row
repeatability has explicit missing keys and measured jitter, not millimeter accuracy.
Actual native GUI opens1051points with unverified-unit/identity warnings.

Changed public files: BOX_FACE_REPEATABILITY.md, BOX_SURFACE_VALIDATION.md,
PROJECT_STATUS.md and this response; documentation commits on dev/box-surface-evidence
(PR39) record exact hashes. No application source changed from37b8dd6. Recent904-test/
fmt/strictClippy/release gate remains applicable; new native data/probe/reference
checks pass. CodeRabbit lastlookup unavailable; no paid or substantive review claimed.

Limitations: post-failure face diagnosis, sparse coverage, unverified metric units,
physical correspondence/calibration uncertainty, same-pose short acquisitions only;
not three qualified complete scans or metric edit/export. Owner dimensions remain
held out. Additional independent calibration/scale and physical-edge evidence remain
priorities; no new independent finding identifier or closure invented.
