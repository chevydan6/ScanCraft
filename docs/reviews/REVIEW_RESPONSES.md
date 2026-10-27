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
26files, zero findings, major gate passed, exit0. Head/tree unchanged before/after;
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
major gate passed, exit0. Head/tree unchanged before/after; operator manifest,
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
hashes are recorded in the reference JSON. CodeRabbit review remains pending;
only the independent reviewer can change finding status.

CR-PLANAR-DOC-ROUNDING accepted: the exact OpenCV discrepancy
0.00021100840581311786 pixels rounds to 0.000211, not 0.000212. Corrected
docs/PLANAR_CALIBRATION_INITIALIZER.md only; numerical source and retained
measurements are unchanged. CodeRabbit CLI0.9.1 reviewed immutable44a2630 versus
31cfbe1:11 files, one minor finding, zero blocking findings, major gate passed,
exit0. Full source-context/diff secret scans passed; fresh included capacity2 and
inactive billing were confirmed. Evidence:2026-10-27-planar-initializer-review.json.
The existing589-test/fmt/Clippy/release result applies to unchanged Rust source.
No paid/repeated unchanged code review or physical calibration performed.
