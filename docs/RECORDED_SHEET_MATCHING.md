# Automatic laser matching from recorded frames

```text
scancraft-cli match-recorded-laser-traces GEOMETRY_JSON FIRST_RECORDING FIRST_INDEX SECOND_RECORDING SECOND_INDEX REQUEST_JSON NEW_DIRECTORY
```

`scancraft_workflow::recorded_traces::run_automatic` validates complete recordings,
selects explicit zero-based indices, detects stripes, builds continuity traces and
invokes the [sheet-supported matcher](SHEET_SUPPORTED_TRACES.md). No partner seeds
or externally prepared image observations are required. Geometry and sheets remain
explicit hypotheses. No calibration, PLY, editable project or live session is created.

Numerical processing delegates to the shared [in-memory depth processor](IN_MEMORY_LASER_PAIRS.md).
Recording validation, source/CRC guards, diagnostic byte limits and publication
remain in this adapter; all existing recorded outputs remain byte-identical in
the saved-data checks.

The workflow now [prepares corrected trace vertices once per pair](PREPARED_LASER_TRACES.md),
preserving the output schema and complete query results while avoiding repeated
inverse distortion. The verification/timing checkpoint below predates this optimization;
the linked report contains current 703-test and comparative performance evidence.

## Input contract

Geometry uses `StereoCalibration` JSON. The request has these fields:

| Field | Meaning |
| --- | --- |
| `schema_version` | `1` |
| `expected_frames` | Two interface/payload-CRC/metadata-CRC bindings |
| `reflect_x` | Explicit raw-pixel-to-camera reflection per side |
| `maximum_host_delta_us` | Pairing window, at most one second |
| `detection`, `continuity` | Same fields as [seeded diagnostics](RECORDED_TRACE_CORRESPONDENCE.md) |
| `sheets` | 1–128 first-camera `QuadraticSheet` objects |
| `triangulation` | Existing `TriangulationLimits` fields |
| `matching` | `intersection` limits, `maximum_sheet_distance_px`, `maximum_work` |
| `maximum_queries` | Explicit cap, 1–4096 supported first observations |
| `maximum_total_work` | Conservative matching bound, at most 100 million; projected-root fallback when coarse admission fails |
| `maximum_report_bytes` | Maximum `matches.ndjson` bytes, at most 128 MiB |

The seeded schema's `seeds`, `seed_radius_px` and top-level `intersection` are
absent. Unknown fields are rejected; geometry/request accepted sizes are each at
most 1 MiB. Matching settings are validated even for empty continuity. Sheets must
have finite coefficients; per-ray geometric failures remain diagnostic errors.

Both full recordings must validate, including later frames/footer. Selected frames
must match explicit interface order (`[0,2]` or `[2,0]`), GREY dimensions, payload
CRC and metadata CRC. Equal raw PTS and the host-time window are mandatory, without
claiming synchronized exposure. Source identity is rechecked before processing
and publication. Camera assignment, physical units and device binding are not inferred.

All supported first observations and second traces participate; no silent sampling
or dropping competing hypotheses. Bounds also include 128 second traces, 4096
vertices per trace and 65536 vertices total. The conservative matching work bound
is `queries * (vertices + segments * 2 * sheets)`, accounting for up to two roots
per sheet and one crossing per segment. When this exceeds the configured limit,
[projected-root admission](PROJECTED_ROOT_WORK_BOUND.md) prepares actual roots and
checks the tighter sum `vertices + segments * roots_for_this_pixel` before trace
matching/staging. Early projection failures remain explicit errors. Detection,
root preparation and continuity have separate bounded inputs; the counter is not
a whole-operation CPU/memory cap. The per-query matcher budget also applies.

Each query's numerical/budget errors are explicit `Err` records; other queries
may succeed. No second traces produces an error per first query; no first queries
produces an empty report. Cancellation, report-size overflow, source failure or
publication failure aborts the bundle. Successful CLI exit means diagnostics were
published, not that every query succeeded: inspect manifest failure/candidate counts.

## Output and guarantees

- Exact geometry/request bytes and normalized one-frame recordings preserve selected
  payload, metadata, sequence, time, profile and supported context.
- `observations.json` retains raw detections/rejections, trace vertex indices,
  transformed matcher pixels, source lineage and qualification limits.
- `matches.ndjson` streams all crossings, sheet roots/supports, ambiguities,
  triangulation rejections and errors, one complete result per first observation.
- `candidate-points.json` retains unique candidate coordinates, first observation
  and trace IDs, second pixels, sheet/root indices and full-result selection references.
- `manifest.json` lists counts, conservative work bound and artifact lengths/CRCs.
  Fallback admission also records its method, original coarse bound and number
  of early projection failures; the existing reader accepts this metadata.

The report byte cap applies only to `matches.ndjson`. Native matching still
allocates bounded per-query crossing/support results; this is not a comprehensive
peak-memory guarantee. Candidate units and cross-observation uniqueness remain
unverified. Nested library results retain the historical `position_mm` field;
no physical millimeter claim is valid without qualified input units.

Existing synchronized staging/no-overwrite publication preserves prior outputs.
Failed staged writes retain partial directories; parent-sync failure after publish
is explicitly reported. CRCs protect accidental corruption, not malicious changes.
Filesystem calls have no hard time bound. Keep private captures/context out of
public repositories and review uploads. The older seeded command/schema is preserved.

## Evidence and limits

Five new integration tests cover known geometry and lineage, exact inputs/checksums,
no-overwrite, duplicate-sheet ambiguity, input/work/query limits, bad CRC/index/later
footer, cancellation, report-size failure with retained partials, reflection and
empty continuity. Full 700-test/fmt/strict-Clippy/release gate passed. The initial
gate caught a changed error string during validator extraction; the original
message was restored, preserving its regression. The 36-case synthetic matcher
known-point reference also passes.

Three private saved Raptor pairs yielded 473/480/473 candidates from 592/615/619
queries, with no query errors. Every complete match result and candidate coordinate
exactly equals the earlier native-example output; detections, exact inputs and
artifact lengths/CRCs were checked. The older seeded workflow still reproduces
79/81/74 candidates exactly. A per-query budget control retains all 592 rejection
reasons and zero candidate points.

On the local Ryzen 9 6900HS (16 logical CPUs), `/usr/bin/time` observed 0.38 seconds
elapsed per offline run, user CPU 0.29–0.30 seconds, system CPU 0.06 seconds and
peak RSS 18352–18360 KiB. Each run validates two existing ten-frame recordings,
selects one pair, processes and publishes/fsyncs diagnostics. Files were reused
and the machine shared with other work, including a seeded regression check.
These limited observations are not isolated algorithm timings, scaling benchmarks
or live frame-rate/latency qualification.

No new capture, physical calibration, installed setting, firmware or GUI scanning
behavior changed. Data uses provisional geometry and a fixed scene. Dimensional
accuracy, cross-pose reliability, occlusion handling and live scanning remain unqualified.
