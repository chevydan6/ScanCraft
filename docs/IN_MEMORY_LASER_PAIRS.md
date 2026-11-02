# In-memory laser-pair computation

`scancraft_depth::laser_pair::Processor` now contains the numerical processing
previously embedded in the automatic recorded workflow. It accepts borrowed raw
frames and returns bounded candidates, camera counters and an outcome for every
query. It performs no file access or JSON work. The file adapter uses the same
processor through a diagnostic sink and retains its original outputs.

This prepares reusable computation for live acquisition; it does not introduce a
live scan mode, tracking/fusion, qualified geometry or scanner controls.

## Interfaces and boundaries

| Component | Responsibility |
| --- | --- |
| `depth/laser_pair.rs::Processor` | Raw-frame validation, detection, continuity, work admission and complete matching |
| `laser_pair::Sink` | Optional borrowed observation/query events with full numerical detail |
| `automatic.rs::Diagnostics` | Recording provenance, expected CRC/source guards, JSON byte caps and publication |
| `workflow::ArtifactWriter` | Owned streaming counterpart of the existing checked artifact writer |

`Processor::new` borrows immutable geometry/settings and prepares camera geometry.
It can then process multiple pairs without retaining prior frames or outcomes.
Settings retain the initial Raptor `[0,2]`/`[2,0]` interface restriction, explicit
reflections, and existing numerical limits. No new third-party package was added:
depth now directly depends on the existing internal core crate for raw-frame types
and validation, with one corresponding lockfile dependency edge.

```rust,ignore
use scancraft_depth::laser_pair::{FrameInput, Processor};
let processor = Processor::new(&geometry, &settings)?;
let report = processor.process(
    [FrameInput { profile: &first_profile, frame: &first_frame },
     FrameInput { profile: &second_profile, frame: &second_frame }],
    &work, &mut (), // explicitly omit full query diagnostics
)?;
assert!(!report.units_verified);
```

Input handoff borrows raw samples unchanged; internal contrast/observation/trace
buffers still allocate. Profile, dimensions, payload/metadata validity, equal raw
PTS and host window are checked before detection. Historical identity, expected
CRCs and exposure synchronization are not established by these checks.

The original detector/continuity structs and implementation moved from workflow
to depth. The older seeded adapter uses that same detector. Sheet matching,
triangulation, ambiguity and projected-root admission retain their existing math
and caps. This extracts/reuses project code, rather than importing a new algorithm.

## Results, streaming and failures

`Report` contains typed `Candidate` values and one `QueryOutcome` per query:
candidate index, nonunique support/ambiguous-segment counts, or the original error.
`CameraSummary` retains observation, supported-trace/observation, rejected-run and
continuity counters without requiring observation JSON. Saturation-rejected runs
are not whole-image clipping counts, and these counters are not a quality pass.

`Sink::begin` receives borrowed detections/trace data only after admission succeeds.
`Sink::query` receives every complete match, including competing hypotheses and
rejection details. Per-query full objects are then released, not accumulated.
Passing `()` explicitly discards detailed events, while retaining compact outcomes
and camera counters. Full trace definitions are available through `begin`; later
reproduction requires retained raw data and exact model/settings/code.

Cancellation, admission or sink failure returns no complete report. A journal sink
may have written a prefix, which adapters must preserve/report. The processor does
not cancel WorkControl itself or touch acquisition. Optional display consumers
should omit rejected previews with counters rather than return required-journal
failures; display preparation must not become an acquisition-abort policy.

Candidates remain in supplied, unverified geometry units. Physical, unit, binding
and synchronization flags remain false; per-pixel uniqueness does not establish
cross-pixel identity. There is no metric Cloud/Editor/export conversion or temporal
tracking state. Stateless reuse does not resolve the session-capacity cap or prove
sustained operation. Existing allocation/cancellation bounds remain; no hard
deadline or worst-case RSS qualification is added.

## File compatibility and verification

The adapter still validates complete recordings/footer, expected CRCs, interface
order and source identity. Its sink stages after admission and streams the same
capped diagnostic records before candidate/manifest publication. Source checks,
synchronized no-overwrite publication, parent sync and truthful failure paths remain.
Owned `ArtifactWriter` uses the original open/checked-write/flush/fsync/rehash path;
the prior closure helper delegates to it. Cancellation cannot return a completion
digest, and existing targets are rejected. No persistent schema or license changed.

Seven new regressions cover pure/detailed sink equivalence and known geometry;
invalid profiles/flags/payload/PTS/host timing; reuse/empty/stale state; duplicate
hypotheses and per-query budgets; admission before callbacks; sink/cancellation
semantics; and owned-artifact digest/no-overwrite/cancellation. Camera-counter
checks extend these tests. Final 732-test/fmt/strict-Clippy/release gate passes.

On eleven private saved pairs (nine rotating poses, box and original synthetic),
every full match byte, observation value and candidate value agrees with baselines.
Camera feedback agrees with old observations. All seven recorded artifacts and
the manifest are byte-identical for every pair. Three seeded pairs also reproduce
all artifact/manifest bytes, retaining 79/81/74 correspondences. Input hashes and
output lengths/CRCs pass. Native-app processing renders 2532 rotating-pose and
1764 box candidates identically; private screenshots were inspected and processes
held no scanner descriptors. These checks do not establish physical accuracy.

## Measured diagnostic overhead

A release-linked same-binary probe reuses decoded frames and one processor per
case. Eleven iterations alternate order; the first is warm-up. It compares complete
reports outside timers. The second mode converts every query to full JSON and
serializes it to a counting writer. Final shared Ryzen 9 6900HS medians:

| Pair | No query JSON | With query JSON | Median paired ratio |
| --- | --- | --- | --- |
| Expensive rotating pose | 104.10 ms | 160.51 ms | 1.54 |
| Saved box | 65.43 ms | 98.52 ms | 1.51 |

Both modes include real cancellation checks, detection, preparation, matching and
compact results. Timers exclude model creation, recording I/O/history/CRC checks,
observation-header/final-candidate formatting, file writes and fsync. Baseline
diagnostic bytes remain allocated during timing. Frames are reused and machine
load/thermal state uncontrolled. Earlier constant-false-cancellation phase probes
are not directly comparable. These are specific paired workloads, not general
speedup, live frame-rate or memory qualification.

Compute alone remains too slow for a 30-fps claim here. Next work is profiling
matching/cancellation costs and adding a bounded acquisition consumer with raw
replay and explicit errors/drops. Optical coverage, calibration/units, tracking,
cleanup and sustained acquisition remain incomplete. Private raw evidence remains
local; automated review/publication await CodeRabbit's quota/billing lookup.
