# Explicit calibrated-domain handling for recorded laser pairs

## Problem and scope

Confirmed: SCAN-REV-033 describes full-image queries aborted by an unrelated
second-camera trace with an invalid inverse-distortion vertex, or by one invalid
sheet projection. The strict low-level APIs intentionally fail the whole query.
The original reproduction and failures remain in
[NATIVE_LASER_DOMAIN_FAILURES.md](NATIVE_LASER_DOMAIN_FAILURES.md).

This correction adds an **explicit opt-in** partial-domain contract. Add
`"retain_valid_domain_segments": true` to the schema1 automatic recorded-matching
request. Both CLI `match-recorded-laser-traces` and the native **Process recorded
laser pair** action consume it. Missing/false uses the original strict path.
Existing requests, results, thresholds, model domains and live capture defaults
are unchanged. Older strict request parsers may reject the new field; it is not a
promise that an older executable can execute the new policy.

Uniqueness is conditional on the retained valid segments and supplied sheets.
Excluded observations are not recovered points or evidence of physical identity.
Calibration, units, synchronization, projector identity and physical accuracy
remain explicitly unverified. This policy does not create an editable metric
Cloud or activate calibration.

## Architecture and contracts

- `scancraft-calibration/src/laser_trace/domain.rs`:
  `PreparedMatcher::prepare_domain_traces` validates the original trace/sheet/work
  bounds, every raw pixel and every original gap, then evaluates native camera
  rays. Every finite in-image inverse failure is retained with original trace/
  vertex/pixel/error. Both adjacent segments are excluded. Contiguous valid runs,
  including unusable singletons, are recorded. Runs of at least two vertices use
  the existing indexed epipolar implementation; rejected gaps are never bridged.
- `PreparedDomainQueries::prepare_pixel` preserves first-camera failures and all
  valid root alternatives. Finite roots behind the second camera or outside its
  unchanged calibrated radius are recorded with sheet/root indices, both camera
  coordinates and error. Numerical overflow/intersection errors remain fatal.
  Out-of-image but in-domain projected roots retain the original semantics.
- `PreparedDomainPixel::execute` remaps original segment/vertex indices and uses
  `PreparedMatcher::match_intersections`, the same triangulation, support and
  ambiguity selection as strict matching. Rejections survive a later query error.
  No nearest-candidate shortcut or interpolation across a rejected vertex occurs.
- `scancraft-depth/src/laser_pair.rs::Processor::with_valid_domain_segments`
  selects the mode without altering public Settings or live defaults. `PairInfo`
  carries trace exclusions; `Query` carries root exclusions; Report contains the
  compact DomainSummary. Original source trace IDs still map all candidates.
- `scancraft-workflow/src/recorded_traces/automatic.rs` retains complete recording,
  CRC, pairing, source-change, staging and publication guards. `observations.json`
  stores vertex/segment/run records; each `matches.ndjson` record stores excluded
  roots; manifest stores counts and `uniqueness_scope`. Query journal bytes remain
  capped. Other artifacts keep their original schemas and qualification flags.
- `candidate_bundle::load_inner` checks optional summary bounds and agreement with
  the retained request policy. `app/cloud_jobs/candidates.rs::view_at` displays
  counts and conditional uniqueness alongside the geometry. Loading verifies
  artifact/frame consistency, not a fresh numerical computation of the journal.

The new orchestration is independently implemented Rust using existing native
camera, sheet, epipolar, stereo and selection algorithms. No new dependency or
third-party source was imported. `laser_trace.rs` shares the former selection loop
with the additive path; existing strict regression tests and exact recording
replay protect compatibility.

## Resource and failure behavior

Original limits remain: at most128 traces/sheets, 4,096 vertices per trace,
65,536 total vertices and4,096 queries in automatic processing. All original
vertices and segments, including excluded ones, contribute to admission bounds.
Fragment/rejection storage is O(original vertices); no unbounded trace splitting.
Valid runs currently repeat native inverse evaluation during indexed preparation;
that bounded cost is not presented as an optimization.

The ordinary conservative bound is `V + S * 2 * sheet_count` per query. If it
exceeds the configured total cap, prepared pixels retain roots and rejection
records, and admission sums `V + S * valid_projected_root_count` for queries whose
preparation succeeds. Failed preparation counts zero matching work and remains a
query error. Per-query work and cancellation checks remain in the shared loop.
Preparation itself is bounded separately; these counters are not CPU instructions
or a claim that preparation costs nothing. Lower budgets still fail before sink
publication. Root errors from malformed sheets or nonfinite transformations are
not reclassified as ordinary domain exclusions.

## Verification on the exact genuine recording

Developer evidence, awaiting independent verification of this correction:

| Check | Result |
|---|---|
| Strict replay, exact original full-frame recordings/model/request | All eight artifacts, including manifest, byte-identical to pre-fix failure |
| Explicit domain mode, only new mode bit | 2,206 queries; 336 candidates; 268 first-ray errors; 1,602 without unique match |
| Trace exclusions | 281 vertices; 279 segments; 18 valid runs, with original indices |
| Projected root exclusions | 596, each retained with sheet/root/coordinates/error |
| First/second detections | Unchanged6,309/7,022;20 right traces/2,130 original vertices |
| Ordinary admission / counted matching work | 41,936,060 / 4,135,562 |
| Exact projected-root fallback | 12,297,860 admitted; matches, points and observations byte-identical |
| One less than exact fallback | 12,297,859 rejected before output publication |
| Native GUI processing of retained original frames | 336 points displayed; six numerical/input/frame artifacts byte-identical to CLI; no camera descriptors |
| Numerical cross-check | All4,040 crossings;3,447 successful triangulations; every support/selection/work counter reconciled |
| Workspace gate | 904 passed,0 failed,1 ignored; fmt, strict Clippy and optimized release pass |

The source pair remains original left interface0/index5 and right interface2/index3,
no reflection, with the frozen native model and four provisional curved sheets.
The developer NumPy checker uses the immutable pre-fix native ray/root audit, then
separate epipolar line/segment formulas, Brown forward projection and least-squares
stereo. Largest coordinate difference is3.638e-11 raw model units; largest forward
pixel difference is1.706e-12 pixels. This is numerical consistency, not independent
physical accuracy. The journal has no multiple-supported combinations on this
particular recording; synthetic tests exercise ambiguity.

Candidate groups: source first/right traces83/376:4,225/376:2,265/376:35,
265/565:86,368/662:201,382/565:8. The201-point group recovers the earlier identified
box stripe. Other135 candidates are unqualified scene geometry, not additional
verified box surface. Visually inspected native preview shows sparse separated
fragments, not a recognizable complete measured box. No fit used the owner's
79.70×59.78×39.80mm dimensions. Raw model units remain unqualified.

One warmed/shared-host CLI run took0.24s wall,0.16s user,0.08s system and39,380KiB
maximum RSS, including record checks and diagnostic serialization. This is a
single offline observation, not live throughput, renderer FPS, a controlled speedup
or memory qualification for long sessions.

Nine added regression tests cover valid-only equivalence, duplicate-sheet and
coincident-segment ambiguity, mixed-domain roots, run splitting and original
indices, no bridging, malformed inputs, work bounds, pulse cancellation during
preparation/execution, exact pair admission, persisted diagnostics and native
worker/reopening/error behavior. Existing tests remain. Initial Clippy failure
(`unnecessary_unwrap`) was corrected using `if let`, with no lint suppression.
Initial GUI harness invocation lacked the default Xvfb path; the successful rerun
uses the already installed private-display binary. Both failures are retained.

## Evidence, review and remaining work

Private root `native-domain-correction-20261104` contains76 files/84,250,736bytes,
including replay/verification scripts, inputs/results, timings, source/binary
bindings, archived executables, test logs, GUI screenshot/receipt and admission
boundary evidence. External original recordings/audit remain bound by the previous
immutable evidence. Manifest:
`native-domain-correction-milestone-20261104/evidence-manifest.json`, SHA256
`b66a9aeaf1371f9cd3f42f27e617567f765a752fca23fea8c9306404ae0822e8`.
Private data/binaries are not published. Run recorded `replay.py` and `verify.py`
from a new output root with the bound originals; archived roots must not be overwritten.

Independent assignment524 opened033 and confirmed the pre-fix failure. The developer
accepts it; only the reviewer can verify/close it. CodeRabbit usage currently returns
`lookup_failed` for included quota and billing. No paid review, upload or substantive
CodeRabbit pass is claimed. Keep the focused PR draft pending review availability.

Immediate priority remains physical qualification and genuine surface coverage:
validate scale/calibration, obtain additional justified sheet coverage, distinguish
object/background and correspondence errors, then repeat measured reconstruction
in three trials. A complete point cloud, tracking/fusion, metric editing, reliable
mesh export and end-to-end qualification remain unproven. No scanner, firmware,
calibration activation, main merge or protection change was needed for this fix.
