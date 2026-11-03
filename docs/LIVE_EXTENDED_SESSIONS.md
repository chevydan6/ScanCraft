# Extended IR live sessions (opt-in schema 2)

## Scope and status

Schema 2 extends the existing IR capture/reconstruction runner to at most 100,000
accepted frames per camera. At 50 accepted frames/s, 30 minutes would require
90,000 frames per camera. This is capacity arithmetic, not a sustained hardware
measurement. Camera pairing remains a candidate based on raw PTS and host time;
physical synchronization and accuracy remain unverified.

Schema 1 retains its 1,000-frame policy, per-pair JSON files, report layout and
ORPROJ08 project encoding. Existing tests continue to exercise that path. Extended
sessions use segmented pair records and ORPROJ09 projects. Old applications that
only recognize ORPROJ08 will reject the new project format; this application reads
both. Raw `.raptor` recording encoding is unchanged.

## Configuration and storage admission

Keep the existing calibration, reconstruction settings, tracking and session fields.
Set `schema_version` to `2` and add `storage`:

```json
{
  "maximum_recording_bytes_per_camera": 220000000000,
  "pair_journal": {
    "segment_bytes": 268435456,
    "maximum_record_bytes": 8388608,
    "maximum_total_bytes": 42949672960
  },
  "maximum_bundle_bytes": 483486543872
}
```

These are explicit storage budgets, not suggested performance settings. For
90,000 frames of 1920 × 1200 GREY, the minimum raw budget per camera is
`(1920 * 1200 + 4096 + 32) * 90000 + 1048576 + 32` bytes. Admission includes the
maximum metadata size and recording header/footer allowance. The bundle budget
must accommodate both raw budgets, the pair-log budget and 512 MiB of metadata /
geometry overhead. The example satisfies that relationship; actual available disk
space must also exceed the bundle budget by 64 MiB before native device setup.
Available space is a snapshot, not a reservation. Disk-full and I/O failures remain
explicit failures, with partial data retained.

Each raw file is capped at its requested budget (up to 1 TiB). Journal records are
bounded at up to 8 MiB, segments at up to 1 GiB, with at most 256 segments and an
aggregate maximum of 256 GiB. Existing segment-slot admission conservatively allows
for whole-record rollover. A requested budget does not guarantee every possible
reconstruction report will fit: an oversized record or exhausted journal fails the
required capture rather than silently discarding observations.

The report is capped at 4 MiB, PLY at 128 MiB, project payload at its existing
64 MiB, and journal index at 1 MiB. Input files retain their existing bounds.
Full observations are written to the journal; memory retains compact pair lineage,
the latest tracking result and the existing bounded tracker/fusion state. Session
history and PTS duplicate sets can grow to the explicit 100,000-frame bound; memory
is bounded by admitted work, not constant with duration. This is not a 60-minute
whole-application memory qualification.

## Data flow and integrity

The existing eight-frame recording queue, four-slot processing queue, two pending
frames per side, coordinated source completion and cancellation contracts remain.
Recording overflow is fatal; optional processing drops remain visible. Storage
admission does not improve acquisition validation or rejection counters.

Schema 2 writes `pairs.index.ndjson` and `pairs-000000.ndjson`, etc. Each complete
record retains the full reconstruction/tracking result plus raw-file role, source
index, sequence, host timestamp, flags, image/metadata CRCs and raw PTS. The shared
journal implementation preserves bounded encoding, whole-record rollover, segment
and index checksums, sync checkpoints and failure poisoning. The IR reader checks
source order, result/reference linkage, pair ordinals and false qualification flags.
Its streaming visitor is provisional until the complete read succeeds.

After sources close, finalization finishes and inspects the journal, replays both
raw recordings, exports geometry when available, retains compact lineage, checks
the artifact set and fingerprints it. Device settings must restore before success
publication. ORPROJ09 requires live-reference schema 2; ORPROJ08 requires schema 1.
Extended lineage permits 100,000 pairs within a 16 MiB metadata budget and fixed,
contiguous segment names. Existing source verification, editing, undo/redo, project
saving/reopening and original-preserving bundle export use these references.

CRC/order checks detect corruption; they do not attest origin or numerical accuracy.
Explicit source verification compares external bytes to saved fingerprints. Merely
opening a project does not open its referenced recordings or prove them authentic.

## Inspection and interrupted capture

```sh
scancraft-cli inspect-live-journal SCAN_DIRECTORY
scancraft-cli inspect-live-journal PARTIAL_DIRECTORY --recovery
```

Complete mode checks the indexed segments and completion footer, plus supported
capture/export manifest digests when present. Recovery mode inspects indexed data
and a possible unindexed complete prefix, reporting a final incomplete record.
Neither mode modifies files, repairs geometry or promotes a failed capture to a
successful scan. A visible footer is not proof of durable storage or successful
scanner restoration. Existing raw-recording recovery remains a separate operation.

## Implementation and validation references

- `scancraft-workflow/src/live_stereo/storage.rs`: schema and storage admission.
- `scancraft-acquisition/src/lib.rs::scan`: admission before native setup.
- `live_stereo/runner/recording.rs`: capped raw sink with existing queue/cleanup.
- `live_stereo/runner/pair_log.rs`: legacy and segmented recording paths.
- `live_stereo/pair_journal.rs`: IR records and complete/recovery inspection.
- `laser_stream/pair_journal/{write,read}.rs`: reused segmented container; the reader
  has explicit laser versus IR role/manifest policies. No new dependency or copied
  external algorithm is introduced.
- `scancraft-geometry/src/live_references.rs` and `formats.rs`: bounded extended
  lineage, ORPROJ09 compatibility and rejection of mismatched versions.
- `scancraft-workflow/tests/live_capacity.rs`: 1,001-pair actual-runner fixture,
  rollover, raw linkage, editing/save/reopen/export, policy refusal, corruption,
  semantic record errors, required budget failure and read-only recovery.
- `live_references.rs::tests`: maximum 100,000-pair project round trip, unsafe
  segment names, count bounds and downgrade refusal.

The full local gate passes 888 tests, one existing ignored, formatting, strict
all-target Clippy and optimized workspace builds. Tests use synthetic images and
calibration; they do not establish genuine Raptor reconstruction. One initial test
incorrectly assumed repeated fusion would retain exactly the three input points;
small alignment drift near voxel boundaries created ten voxels. The corrected
capacity test checks reported count and preserved geometry through editing/export,
without treating this synthetic fixture as an accuracy measurement.

A retained optimized-library trial exercised **100,000 synthetic pairs**, with
100,000 saved frames per camera and zero processing-queue drops. It produced nine
segments totaling 603,827,864 journal bytes and a 6,969,211-byte ORPROJ09 project.
Every journal reference was checked against the corresponding raw frame's index,
sequence, host timestamp and image/metadata CRC. Project reload/save was byte
identical. The paced capture/finalization took 35.992905 s; including the additional
raw-link and project checks took 42.378556 s. `/usr/bin/time -v` reported peak RSS
53,200 KiB, 30.10 s user and 1.07 s system CPU, and 42.38 s elapsed. This uses
64 × 32 synthetic images and ten fused voxels, not native-resolution data or a
hardware/rendering benchmark. Diagnostic notes stopped at their existing 1 MiB
budget; required journal records retained all 100,000 pairs.

The actual release CLI verifies the complete retained journal. A separate
interrupted-journal fixture yields two complete unindexed records and three torn
bytes in recovery mode; complete mode refuses it. The release GUI opens the
100,000-pair project on an isolated display and shows its ten-point cloud and live
source section; sampled camera descriptors are absent. This checks project import
and display, not a full GUI scan/edit qualification. Reproducible source, library /
executable hashes, inputs, outputs and process measurements remain in private
`live-capacity-20261104` evidence; the first failed test assertion is retained.

Sustained Raptor acquisition, processing at least 90% of valid incoming frames,
30-minute reliability, whole-app memory/latency, physical calibration and repeated
measured object reconstruction remain unqualified. SCAN-REV-021 awaits independent
assessment of this implementation; only the reviewer may close it.

## Native worker validation

A subsequent genuine Raptor trial at implementation `2cbd5e04d790d17efee9cb16cdb90938e799d38c`
records **1,200 accepted frames per camera** through the budgeted raw sink. Both
original recordings replay completely. An independent streaming parser checks all
frame/header CRCs and compares every saved frame's image/metadata CRC, sequence,
host timestamp, flags and byte lengths with samples taken before recording. The
files contain 2,764,868,061 / 2,764,868,111 bytes, below their admitted worker quotas.

The installed baseline driver is unchanged. Startup rejects are 0/0, ongoing
rejects and sequence gaps are 12/11. The combined rejected-buffer proxy is
`23 / (2400 + 23) = 0.949237%`, above the `<0.1%` target. No validation threshold or
counter is relaxed, and there is no controlled comparison establishing a rate
change caused by the budgeted sink. First-to-last accepted host timestamps span
40.371684 / 40.338363 seconds. Whole-probe `time -v` reports 41.36 s elapsed,
3.10 s user CPU, 3.37 s system CPU and peak RSS 58,932 KiB.

Independent timestamp comparison and the release CLI find 1,199 unique equal-PTS
candidates within 1,000 microseconds. Their absolute host deltas are 2–159 us,
median 19 us and p95 80 us. Equal PTS and nearby host callbacks do not prove exposure
synchronization. The final left/right images show usable calibration-board markers;
the right board edge remains clipped, and no measured printed object is in view.

A second private worker trial intentionally gives the left sink a 200-frame raw
budget while requesting up to 300 frames. This bypasses configuration admission
**only in the private fault-injection harness**; normal schema 2 admission rejects
such an insufficient budget. The worker reports the required byte-budget failure
and the probe exits 2. Both cameras return 202 accepted frames before cleanup.
The left file retains 200 complete frames plus eight bytes of an incomplete record
and no footer; the right file retains 202 frames and a valid footer. Neither file
exceeds its configured budget. Recovery creates separately validated 200/202-frame
recordings, preserving the originals. The two returned but unsaved left frames are
not represented as successful storage. All 2,802 distinct complete original frame
records across both trials match their pre-recording data.

Seven scanner state groups compare equal after each trial and at final restoration
(21 comparisons). All five idle camera formats and the prior WirePlumber service
state restore. Installed module bytes and parameters match the baseline. No firmware,
calibration, permanent driver change or application-source edit is involved.

Scope: the private probe compiles exact `runner/recording.rs` and `completion.rs`,
plus the exact `CappedFile` code block with import/module scaffolding, against the
current release dependencies. It uses a fixed-no-command Control shim and leaves
optional processing queues deliberately undrained. Thus processing drops are
expected; this does **not** exercise calibrated reconstruction, full runner journal
publication, GUI scanning or the 90% processing target. It measures the native
camera/recording path, with additional pre-recording CRC sampling. The deliberate
quota failure is not a physical disk-full/power-loss test or a hard cleanup bound.

Private `live-capacity-hardware-20261104` retains reproducible build/run/parser
scripts, exact source and binary bindings, original/recovered recordings, native
counter reports, timing/resource logs, snapshots and original final-frame exports.
Production source and its passing 888-test gate remain unchanged; publication
690643c CI41671761778/run239 subsequently passed. Physical scale, broader target
coverage, genuine object reconstruction and sustained hardware qualification remain
outstanding. SCAN-REV-021 remains subject to independent assessment.
