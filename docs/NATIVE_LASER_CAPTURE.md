# Native laser candidate capture

Status: experimental native Linux CLI and reusable acquisition API. This captures
and processes genuine Raptor frames without Wine or a Creality runtime. It saves
diagnostic candidate coordinates; calibrated dimensional accuracy, tracking,
fusion, meshing and live GUI controls remain incomplete.

## Running

Build with the pinned toolchain and use:

```sh
cargo build --frozen --release --workspace
./target/release/scancraft-cli scan-live-laser geometry.json capture-request.json capture-001
```

The output directory must be new and its parent must already exist. Geometry is a
plain `StereoCalibration` JSON numerical hypothesis in explicitly supplied units.
This entry point does not load/activate a physically qualified calibration or infer
units, camera orientation or projector identities. Keep personal models, scan
frames and captures outside the source repository. Output permissions follow the
process umask and selected parent directory.

The capture-request schema is:

| Field | Contract |
| --- | --- |
| `schema_version` | 1 |
| `fps` | Nonzero workflow declaration; native adapter permits requests at 5/10/15/20/25/30/40/50 FPS and the device must accept the exact profile |
| `maximum_frames_per_camera` | Nonzero u64; does not allocate a session history |
| `maximum_recording_bytes_per_camera` | At most 1 TiB, enough for a maximum header plus one full frame/metadata/footer; exhaustion fails capture with retained diagnostics |
| `maximum_pair_log_bytes` | 4096 through 1 GiB; required compact-pair journal cap |
| `processing` | Existing `laser_pair::Settings`: ordered interfaces, reflection, host window, detector/continuity, supplied sheets, triangulation, matching and work/query limits |
| `source_options` | Optional backend declaration, retained unchanged; native accepted values described below |

See [in-memory processing](IN_MEMORY_LASER_PAIRS.md) and
[bounded streaming](BOUNDED_LASER_STREAM.md). The existing settings DTOs are now
serializable as well as deserializable, so applications can construct requests
without duplicating their schema.

Omitted/null `source_options`, or `{"illumination":"preserve"}`, leaves illumination
unchanged. `{"illumination":"line_boolean_trial"}` explicitly requests the existing
scoped SDK property-151 boolean trial. No other source-option fields are accepted
by this native adapter. It validates the boolean range, enables/readbacks the
fixed property before camera setup, checks it after capture and restores/readbacks
the original value before publication. It never guesses mode, energy, master-laser,
gain or other unsupported property values. A successful SDK readback does not
verify physical emission, wavelength, projector identity or calibration binding.
Existing illumination prerequisites and cold-start behavior remain unqualified.

Ctrl-C requests graceful stop, followed by queue draining, source cleanup, property
and stream-settings restoration, recording verification and publication. Driver
calls, fsync and cleanup are synchronous and have no established hard bound.
A desktop caller should use the device-owning supervised helper pattern; this
milestone has not connected new controls to that GUI yet.

## Implementation and provenance

- `workflow/laser_stream/bundle.rs::Plan::open` retains exact regular, bounded
  inputs with existing `scan_project_load::Guard` identity checks. It validates
  geometry and all detector/continuity policies using the existing functions on
  blank images before hardware ownership. Those images are discarded and never
  represented as sensor/calibration evidence.
- `Plan::capture` connects the existing bounded stream pipeline to required raw
  journals and a typed streaming pair log. Optional preview `Omitted` or panic is
  counted; it does not abort required journaling or cancel WorkControl. Required
  journal errors remain fatal. Failed sessions retain a staging path and bounded
  `failure.json`, including available source counts and restoration results.
- `bundle/io.rs::CappedFile` owns an explicitly capped buffered file and computes
  its write-time CRC/length. `RawJournal` uses the existing recording writer and
  final flush/fsync. `verify_raw` streams the existing reader without a frame index
  or global PTS set, checking v2 profile, per-frame/header CRC, order, footer/count,
  and whole-file write/readback equality in the same pass. Other artifacts also
  require matching write/readback digests and retained inode/path guards.
- `acquisition/laser.rs::scan` owns SessionLock, group SettingsLease and optional
  existing ScopedLine. Sources use `Capture::open_recording` and poll StopToken.
  All source owners close before every restoration and before publication. Source
  context records the illumination declaration; units and optical binding stay
  unverified. Existing IR context/statistics helpers are reused; stats now separate
  before-first-valid and after-first-valid rejection counts (V4L2 buffer units).
- `cli/live_scan.rs::run_laser` supplies signal-driven graceful stop. Its shared
  monitor preserves the old IR path's cancellation behavior. `main.rs` adds the
  command; existing scanner/GUI/editing workflows remain intact.

These are independently written orchestration/format adapters reusing ScanCraft's
Source, Writer/Reader, CRC dependency, output staging/RENAME_NOREPLACE publication,
Guard and scoped line-property implementation. No new third-party dependency,
proprietary source/asset, firmware, mode/energy change or physical calibration is
included. The geometry/matching math is unchanged; added serde derives affect
configuration serialization only. Borrowed model and device/UI boundaries remain.

## Bundle and publication

A successful folder contains `geometry.json`, `stream-request.json`,
`first.raptor`, `second.raptor`, `pairs.ndjson`, `report.json`, and `manifest.json`.
The first/second role follows configured interface order; USB names do not infer
physical camera orientation. Every pair record carries raw-record indices,
sequence/timing/flags, payload/metadata CRCs, raw PTS and either a compact engine
report or its explicit error. Full crossing/support details can be recomputed from
retained frames; they are not stored for every live query.

The manifest lists six artifact lengths/CRCs; it is an integrity record, not an
authenticity proof. Settings restoration, raw replay and input/artifact identity
checks precede no-overwrite publication and parent-directory sync. Failures after
rename identify the published target; earlier failures retain the staging folder.

`geometry_exported=false` denotes no PLY, mesh or editable metric project export.
Diagnostic coordinates remain saved in `pairs.ndjson`. All physical-qualification
flags remain false. The streaming bundle has its own schema; the existing candidate
viewer consumes a recorded-pair diagnostic bundle, not this folder directly.

## Verified evidence and limits

Final formatting, strict all-target Clippy, **756 tests** and release build pass.
Ten added tests cover bundle integrity/lineage and absence of metric exports,
preview omission/panic independence, restoration failure, input mutation,
no-overwrite/races, detector preflight, exact caps/readback corruption, required
pair-log exhaustion with retained raw prefixes, raw mutation, native source-option
restrictions and device-free CLI boundary rejection. Shared synthetic fixture
visibility changed only for these tests. Initial compilation needed an explicit
borrow lifetime; corrected without suppressing checks.

Actual connected Raptor warm trials used unchanged supplied geometry/settings:

| Trial | Accepted raw per camera | Processed pairs | Processing queue drops per camera | Candidate counts |
| --- | ---: | ---: | ---: | --- |
| Preserve, initial build | 16 / 16 | 16 | 0 / 0 | 0; no supported observations |
| Preserve, option-capable build | 4 / 4 | 3 | 0 / 0 | 0; unpaired inputs reported |
| Scoped line, final counter build | 16 / 16 | 12 | 4 / 4 | 1857–2023 per processed pair, zero query failures |
| Scoped line, graceful Ctrl-C | 3 / 2 | 2 | 0 / 0 | 1987 / 1904; one unmatched frame retained |

The final 16-frame line trial recorded 4/85 rejected buffers before first valid
frames and zero after; the graceful trial recorded 2/1 before, zero after. Earlier
preserve reopening reported 1/89 total rejects with one ongoing rejection sample.
No reliability claim follows from these short, variable startup observations;
V4L2 buffer rejects are not physical-exposure counts. Stream state for both
interfaces and all returned illumination readbacks matched their pretrial values.
The existing property-151 original value was zero and was restored after each trial.

The final line trial processed 75% of accepted frames in this short workload,
below the provisional 90% target. Processing still shares the association consumer;
expensive work can fill the four-frame channel. No live FPS/p95 or sustained-session
acceptance is established. One warm Ctrl-C trial exited about 0.190 seconds after
the signal, including drain/verification/restoration; this is not a hard shutdown
bound and does not resolve SCAN-REV-005.

A separate private read-only replay through the same engine reproduces all 17 compact reports across
preserve/line/graceful captures, including all frame-reference CRCs, sequence,
flags, times and raw PTS. A selected native pair also matches all 2023 candidate
values through the existing recording-backed full-diagnostic adapter, then displays
in the desktop candidate viewer with both source images and unverified-unit labels.
The render holds no scanner descriptors. Initial private adapter validation omitted
its separate maximum_report_bytes field; corrected the test request, preserving
the failed attempt. No production matching change was made to pass that check.

Private source/binary hashes, requests, raw captures, readbacks, replay checks,
process/signal logs and screenshots are retained in the developer recovery evidence.
These tests do not qualify physical dimensions, optical coverage, synchronization,
tracking/fusion, meshing, cold initialization, 30-minute sessions or stable release.
Independent automated review remains pending unavailable quota/billing lookup.
