# Dense recorded-pair capacity and full sampled replay

## Confirmed problem and measured scope

The [long rotation replay](ROTATION_COVERAGE_LIMITS.md) refused 71 of 225
predetermined pairs before matching because their supported first-camera
observations exceeded 4,096 queries. These are the same saved Raptor **board**
recordings, sampled at original indices 0,4,...896. They are not a new capture of
the owner's measured box, an incoming-rate test or an exact-angle rotation trial.
Base revision: `14f3b23d1b3bd65dd39c4807dd24b1f40c97b605`.

A private diagnostic used the unchanged native detector, continuity, lens domains,
laser sheets and prepared matcher to count every query and its projected-root
work bound. No observation was thinned, coefficient fitted or optical gate relaxed.

| Observation | Measured result |
| --- | ---: |
| Pairs exceeding 4,096 queries | 71 |
| Maximum queries, original frame 236 | 5,305 |
| Pairs also exceeding 100 million projected-work units | 4 |
| Maximum projected-work bound | 105,941,972 |
| Work-limit failures, original indices | 232, 236, 240, 244 |

Eight predeclared diagnostic executions included the first query refusal, all four
work refusals, and three previously successful controls. All query records for
controls 0/176/896 reproduced byte-for-byte. This supplied evidence for a bounded
capacity extension; merely changing a limit would not establish correct geometry.

## Implementation and tradeoff

`scancraft-depth::laser_pair` exposes shared ceilings `MAX_QUERIES = 8192` and
`MAX_TOTAL_WORK = 128000000`. The recorded adapter, candidate loader, saved-stream
loader and GUI preview use the same query ceiling. These are explicit **maximum
configurable budgets**, not automatic changes to existing requests. A request
retaining 4,096 queries and 100 million work units retains its original refusals.
The new ceilings give finite headroom above this dataset's measured maxima; they
are engineering bounds, not evidence that all future scenes fit.

`Processor::process` retains at most the previous 4,096-query prepared-root cache.
For larger pairs that need tighter admission, it preflights each query, accumulates
the complete conservative work bound and drops its prepared roots. Only after the
whole pair passes admission does it recompute each query for execution. This trades
additional root calculations for bounded cached preparation storage. No partial
admission, per-batch budget reset, hidden sampling or successful-subset publication
is introduced. The `uses_projected_roots` diagnostic describes the admission method
even when roots are recomputed instead of cached.

Outcome/candidate vectors may now contain up to 8,192 entries; this is not a claim
of unchanged total memory. Detection, retained second-camera traces, per-query work,
report bytes, capture queues and source/CRC validation keep their separate limits.
The work counter bounds matching operations, not total CPU time: detection, root
preparation/recomputation, JSON, disk access and GUI rendering are additional work.
No dependencies, numerical algorithm, units or physical qualification flags changed.
Older versions may reject new high-capacity bundles; old requests remain supported.

The first end-to-end attempt exposed a stale 4,096-query candidate-loader bound.
That failure and its partial output remain private evidence. The fix aligns all
relevant producer, persisted-data and preview bounds with the shared constant.
Unrelated 4,096-byte metadata and per-trace vertex bounds remain unchanged.

## Verification

Final `tools/check.sh`: formatting, strict all-target workspace Clippy, optimized
build and **925 tests passed, 0 failed, 1 ignored**. The dense 4,500-query regression
compares every outcome and candidate with/without tighter admission in both strict
and retained-domain modes; lower query/work limits must reject before sink output.
The preview regression round-trips the exact shared capacity and rejects excess.
Existing typed-loader rejection tests use the new ceiling and still enforce it.

All 225 native CLI operations succeed with explicit budgets 8,192/128 million;
all other request fields, original frames and supplied geometry remain unchanged.
Every manifest artifact length/CRC and source hash is checked. All **154 previously
successful pairs** retain byte-identical `matches.ndjson` and `candidate-points.json`.
The eight diagnostic cases also match completely. After the loader correction and
final build, those eight cases reproduce again and all 225 bundles pass through
the final native fusion loader. The initial producer build and final build have
separate source/binary bindings; the full 225-pair producer run is not misattributed
to the later binary.

There are **277,704 unfused surface candidates**, including scene/background data,
of which 137,207 come from previously refused pairs. All **26,488 per-query errors**
remain explicit; 13 pairs have zero candidate points. Successful bundle publication
therefore does not mean every query yielded geometry. Re-executing all 71 original
low-query requests preserves their exact errors; the four dense cases with only
the query allowance increased still refuse the original total-work budget.

Separate NumPy inverse-Brown and closest-ray calculations independently reproduce
all 137,207 newly available candidate coordinates within **2.302e-9 supplied model
units**. This checks triangulation conditional on the selected correspondences and
camera model. It does not independently prove stripe identity or physical accuracy.

The 225 single-process CLI runs have observed median elapsed time **0.28 s**, maximum
**0.55 s**, and maximum RSS **32,444 KiB** on the shared development host. These warm,
serial measurements include recording validation, processing and artifact writing;
they exclude acquisition/fusion/rendering and are not a controlled speed comparison,
worst-case allocation bound, sustained memory test or qualification for 30 FPS.

## End-to-end result and remaining priority

The existing temporal marker option now runs natively across all 225 sampled
surface bundles. All marker selections/poses/outcomes match the previously
independently verified 225-frame prototype:

| Outcome | Count |
| --- | ---: |
| Integrated frames | 26 |
| Tracking-only accepted frames | 13 |
| Insufficient tracking observations | 186 |
| Fused surface candidates | 5,785 |

Last accepted/integrated original frame: **152**. There is no global tracking
recovery. Independent surface-only voxel accumulation matches within **1.137e-13
model units**. The native GUI opens the full result at source pair 225, displaying
all 5,785 candidates, the failed-tracking counts and retained camera images.
The inspected geometry is fragmented, not a complete reconstructed object.
No GUI editing, metric export or physical measurement qualification is claimed.

Two private verifier errors (local versus original frame IDs, and optional fields
on unavailable observations) were retained and corrected to the explicit schemas;
the final checks compare original IDs and separately assert unavailable outcomes.

**Recommendation:** prioritize object-associated correspondence and retained-reference
relocalization, then calibrated units and measured object coverage/repeatability.
This capacity change removes a processing refusal; it does not solve the tracking
or calibration problems. The measured-box rotation capture still needs confirmation
that the turntable is running. CodeRabbit's usage lookup failed, so no substantive
CLI review or paid usage is claimed. Independent review and owner merge authority
remain unchanged.

Private evidence: `laser-query-capacity-20261104` (plans, original failures, source
bindings, all outcomes, numerical checks and native GUI capture); external manifest
and publication receipts: `laser-query-capacity-publication-20261104`. Raw recordings,
calibration coefficients, derived geometry and images are excluded from public Git.
