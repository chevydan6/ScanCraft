# Finite native live stereo candidate scans

`scancraft-cli scan-live-stereo CALIBRATION RECONSTRUCTION_SETTINGS SCAN_SETTINGS NEW_DIRECTORY`
connects native Raptor acquisition directly to the existing Rust stereo processor,
point-to-point tracking, voxel fusion, recording and editable-cloud export. This
is a finite CLI scanning path, not a qualified live GUI mode. The scanner is
currently unavailable for this implementation's hardware validation. Do not use
synthetic tests to infer optical accuracy, exposure synchronization, sustained
throughput, reliable reconnection or bounded driver shutdown.

Calibration is an explicitly supplied RAPCAL record. The native entry requires
`observations` source declarations matching a nonsynthetic record's declared
binding; synthetic calibrations are rejected before device opening. Matching
serial/firmware/mode declarations are **not hardware attestation**. Factory-field
mapping, active image orientation and physical qualification remain unresolved;
the command never guesses those values or promotes a record to qualified status.
Physical calibration remains deferred. There is no illumination, gain, mirror,
projector, firmware or physical calibration operation in this command. It does
configure and restore V4L2 format/rate through the existing native device API.

## Inputs and policy

`RECONSTRUCTION_SETTINGS` uses the existing stereo request schema described in
[offline stereo](OFFLINE_STEREO.md), including source, output grid, seeds or
explicit automatic sampling, matching and triangulation limits. Meshing and
surface-normal requests are rejected here. Rectification maps are built once.
A discarded blank-image preflight validates the existing matching policy before
acquisition; those pixels are not observations or published reconstruction data.

`SCAN_SETTINGS` is bounded schema-1 JSON, with no unknown fields:

```json
{
  "schema_version": 1,
  "session_id": "candidate-scan-001",
  "fps": 30,
  "maximum_frames_per_camera": 100,
  "maximum_host_delta_us": 1000,
  "tracking_samples": 256,
  "voxel_mm": 0.2,
  "maximum_voxels": 100000,
  "tracking": {
    "max_correspondence_mm": 2.0,
    "max_rms_mm": 0.25,
    "minimum_overlap": 0.8,
    "max_step_translation_mm": 5.0,
    "max_step_rotation_rad": 0.1,
    "iterations": 40,
    "tolerance_mm": 0.000001
  }
}
```

These are example algorithm policies, not validated Raptor settings. Accepted
rates are 5,10,15,20,25,30,40,50 FPS; frame count is 1..1000 per camera, host
window 0..1000000 microseconds, and tracking samples 3..512. Existing session,
tracking and fusion bounds also apply. The example's nominal 30 FPS is requested,
not a throughput measurement. Files can be large: two GREY streams require
`2 * width * height * frames` payload bytes plus recording/report overhead.

Interface 0 is explicitly assigned the declared left geometry and interface 2
the declared right geometry, using their exact declared dimensions. This does
not verify that optical assignment. No image reflection is selected implicitly.

## Concurrency and association

Each camera opens, reads, writes its own recording and closes in its owning
thread. Raw recording append precedes `try_send` into one four-event processing
channel. Disk buffering is flushed and synced at normal recording completion,
not after each frame; crash recovery is not a per-frame durability guarantee.
A full processing queue drops only that processing delivery; the raw frame was
already appended. `recorded_frames`, `queued_frames` and
`processing_queue_drops` account separately for those outcomes. Driver rejection,
missing metadata and sequence-gap counters remain separate buffer diagnostics;
they are not counts of lost physical exposures.

Association retains at most two pending frames per side. Candidate pairs require
identical raw sensor PTS and an absolute host-timestamp difference no greater
than the supplied limit. Per-stream source indices, host times and modulo-PTS
progression must advance; repeated PTS and regressing pair order are errors.
Unmatched pending evictions, remaining frames and host-window rejections are
counted explicitly. No interpolation, timestamp-rate conversion, clock fitting,
nearest-time pairing, physical synchronization or exposure identity is inferred.

Every raw recording is fully replayed through existing integrity/footer checks
before publication. Repeated PTS anywhere in either complete recording aborts
publication, including frames not delivered to processing. This conservative
policy refuses ambiguous association rather than assigning wrap epochs.

Each candidate goes through the shared `StereoProcessor`, original-camera
triangulation and `ScanSession`. Tracking uses up to the requested number of
deterministically evenly spaced indices from the reconstructed cloud; fusion
receives the full cloud. Fewer than three points produce a `no_geometry` report
without a new pose or integration. A rejected alignment is not fused; subsequent
frames can recover against the retained reference. All reports keep binding,
synchronization and physical-accuracy qualification false.

## Output and failure handling

A successful new bundle contains exact calibration/request/scan-settings bytes,
`left.raptor`, `right.raptor`, per-candidate `pair-NNNN.json` reports,
`report.json`, and an artifact length/CRC manifest. Nonempty fusion also produces
`points.ply` in declared millimeters and `project.orproj`. No-geometry runs keep
raw data and diagnostics without inventing a model. The project contains the
derived editable cloud, not resumable fusion weights or embedded raw lineage;
retain the whole bundle. Existing project/recording formats are unchanged.

All capture workers are joined, including on coordinator failure/unwind. Source
read/recording panics are caught so explicit source closure is still attempted;
cleanup panics become failure diagnostics. Settings
are restored and read back as a group after camera closure and before publication.
Read/close/restoration errors are combined, and incomplete output is retained at
the reported staging path. Existing targets and dangling target symlinks fail
before source opening. Final publication uses existing atomic no-overwrite rename;
a target appearing later is never overwritten. Unexpected artifacts and nonregular
or symlinked files are rejected during manifest construction. CRC detects
accidental corruption; this is not an authenticated or adversarially atomic snapshot.
A post-publication parent-sync error explicitly identifies the published directory.

SIGINT/SIGTERM request cooperative processing cancellation. A short-lived monitor
relays the CLI signal request to `WorkControl`. Camera workers stop and clean up
before group restoration; none are detached to make shutdown appear bounded.
Kernel setup/control/close can still block indefinitely: **SCAN-REV-005 remains
open**. Do not claim a full shutdown time bound from hardware-free worker tests.

## Implementation and evidence scope

- `crates/scancraft-workflow/src/live_stereo.rs`: policy, bounded association,
  explicit input-mode guards, cached stereo processing and session updates.
- `crates/scancraft-workflow/src/live_stereo/runner.rs`: source trait, worker ownership,
  raw recording, bounded channel, replay, cleanup and publication.
- `crates/scancraft-acquisition/src/lib.rs`: native source adapter, session lock
  and settings lease shared by GUI and CLI.
- `crates/scancraft-cli/src/live_scan.rs`: CLI signal ownership and cancellation.
- `crates/scancraft-app/src/live_ui.rs`: live controls and bounded previews.
  See [live GUI behavior](LIVE_STEREO_GUI.md).

New tests cover PTS wrap/order/corruption, pending/host-window accounting,
physical-mode fixture rejection, repeated-pair protection, cancellation,
no-geometry continuation, tracking loss/recovery through the actual reconstruction pipeline on synthetic
images, queue saturation with complete raw retention, combined failures, cleanup
on unwind, restoration-before-publication, artifact CRC/project equality,
restoration failure and late cancellation. Injected sources exercise the complete
bundle path without touching a scanner. These are synthetic software results,
not real Raptor acquisition or dimensional qualification. No new algorithm was
copied or dependency added; orchestration is project-original MIT Rust.

## Reproduce the device-free release proof

```sh
cargo run --frozen --release -p scancraft-workflow --example synthetic_live_scan -- NEW_SYNTHETIC_OUTPUT
scancraft-cli replay NEW_SYNTHETIC_OUTPUT/left.raptor --decode
scancraft-cli replay NEW_SYNTHETIC_OUTPUT/right.raptor --decode
```

The example explicitly selects `InputMode::SyntheticFixture`, uses the existing
redistributable synthetic image/calibration fixture, and calls the same source
runner and processor as the native adapter. It opens no device. The expected
result is one candidate pair, three points, two complete raw recordings, ten
output files and nine verified manifest entries. Expected points in the declared
synthetic millimeter frame are (-57.5,-27.5,400), (22.5,22.5,400), and
(92.5,42.5,400); numerical comparison tolerance is 1e-7 mm. This is not scanner
accuracy, a dense reconstruction benchmark or a realistic throughput workload.
