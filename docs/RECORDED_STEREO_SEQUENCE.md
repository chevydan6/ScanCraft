# Offline recorded stereo reconstruction, tracking and fusion

```
scancraft-cli reconstruct-recorded-sequence CALIBRATION LEFT_RECORDING RIGHT_RECORDING RECONSTRUCTION_SETTINGS SEQUENCE_SETTINGS NEW_DIRECTORY
```

This connects existing native recorded-stereo reconstruction to the existing
point-to-point `ScanSession` tracker and voxel fusion. It is an offline workflow,
not live scanning or qualified Raptor reconstruction. Supply an explicit calibrated
image model and source declaration using the existing reconstruction settings.
No calibration, camera assignment, projector line or exposure relationship is
inferred from raw factory bytes or timing candidates. Hardware is never opened.

Sequence settings schema (illustrative synthetic values, not scanner tolerances):

```json
{
  "schema_version": 1,
  "session_id": "synthetic-recorded-sequence",
  "tracking": {
    "max_correspondence_mm": 2.0,
    "max_rms_mm": 0.01,
    "minimum_overlap": 0.8,
    "max_step_translation_mm": 1.0,
    "max_step_rotation_rad": 0.1,
    "iterations": 20,
    "tolerance_mm": 1e-8
  },
  "voxel_mm": 0.1,
  "maximum_voxels": 1000,
  "pairs": [
    {"id": 1, "left_index": 0, "right_index": 0, "tracking_indices": [0,1,2]},
    {"id": 2, "left_index": 1, "right_index": 1, "tracking_indices": [0,1,2]},
    {"id": 3, "left_index": 2, "right_index": 2, "tracking_indices": [0,1,2]}
  ]
}
```

There must be 1..1000 pairs with strictly increasing IDs and indices on **both**
recordings; no repeated capture is silently counted as a new frame. Indices are
zero-based. No automatic pairing, sorting, dropping or clock-offset correction
occurs. Increasing indices and IDs do not verify that the physical exposures
matched. `tracking_indices` contains 3..512 unique indices in each frame's
**accepted cloud**, not the input seed array. Existing session limits apply:
1..100000 points per cloud, bounded fusion voxels, finite millimeter data and
validated tracking/coverage/motion gates. No subsetting is silently performed to
fit these bounds. Reconstruction settings may enable opt-in subpixel matching.
Meshing/surface-normal requests are currently rejected explicitly in this path;
existing individual reconstruction and mesh workflows remain available.

`RecordingProcessor` owns two fully validated recording indexes, the original
calibration/request bytes and one rectification-map set for the run. Each pair
reuses them, rechecks selected-frame fingerprints/source guards and applies the
same context/binding checks as individual recorded reconstruction. This avoids
reindexing full recordings and rebuilding maps per pair by construction; no
measured speedup or live throughput is claimed. `process_with_processor` shares
existing matching/triangulation/surface logic; individual CLI/GUI reconstruction
continues through the same checked adapter. Only the current pair/reconstruction
is retained alongside bounded session/fusion history; all previous raw selections
are written to staging rather than accumulated as image buffers.

Each `frame-NNNN/` keeps exact normalized-v2 selected recordings, original
calibration/request bytes, source lineage, matching/triangulation report, accepted
cloud PLY when present, and a CRC artifact manifest. Even a frame rejected by
tracking retains its geometry and inputs. Local tracking rejection has no pose
and is not fused; recovery uses the last accepted reference through the existing
tracker. Blank/no-geometry pairs stop the run with current raw sources and
rejection diagnostics retained in incomplete staging; they do not manufacture a
pose or masquerade as a tracked frame. This first path has no automatic blank-frame
recovery or scan resumption.

The root retains `sequence-request.json`, per-frame tracking decisions/poses and
artifact directory references in `report.json`, fused `points.ply`, editable
`project.orproj`, and a root artifact manifest. Root publication is atomic and
refuses overwrite. Errors or cooperative `WorkControl` cancellation leave the
incomplete staging directory with a diagnostic path; successful publication is
not retracted by a later cancellation. Original recordings/calibration/settings
are read-only. Complete-recording guards are the existing point-in-time filesystem
checks, not cryptographic authenticity or historical completeness qualification.
No whole-bundle verification/resumption command is introduced here.

The fused project stores geometry and edit history. Multi-frame reconstruction
provenance remains in the **complete external sequence bundle**, not a fabricated
single-frame project origin. Keep that bundle; standalone project export does not
embed the raw inputs or preserve a multi-frame association. Opening the fused
project in the existing GUI is supported; a dedicated sequence GUI and persisted
multi-frame source verification remain future work. Frames in this path use
point-to-point tracking; other registration methods remain available in the
existing cloud-sequence workflow. All binding/synchronization/physical accuracy
qualification fields remain false regardless of successful matching or local ICP.

Implementation: workflow `recording_reconstruction::RecordingProcessor`,
`lib.rs::process_with_processor`, `recorded_sequence::run`, CLI command arm.
This is project-local orchestration of existing Rust algorithms/crates, no imported
upstream algorithm or new dependency. Tests compare cached and standalone reports,
check mutation guards, complete raw/context retention, saved-project reopening,
overwrite refusal, malformed/index/cancellation errors, no-geometry diagnostics,
and genuine reconstruction-to-tracking loss/recovery on **synthetic** stereo data.
A native CLI integration verifies command wiring and original-byte preservation.
The loss fixture changes disparity from four to eight pixels, which produces a
wrong-depth cloud rejected before fusion; the next valid pair recovers. Its right
sensor is inverted, so generated test pixels use that explicit orientation.

Reproduce the source fixture with the existing example, save the JSON above as
`SEQUENCE_SETTINGS`, and run the command with generated calibration/request/files:

```
cargo run --frozen --release -p scancraft-workflow --example recorded_stereo_fixture -- NEW_DATASET
cargo test --frozen -p scancraft-workflow --test recording_reconstruction
cargo test --frozen -p scancraft-cli --test recorded_sequence
```

No synthetic result completes the genuine Raptor point-cloud, physical accuracy,
transport, live tracking, end-to-end qualification or stable-release gates.

The release CLI demonstration integrated three synthetic frames and reopened the
fused project in the existing GUI on a private display without camera descriptors.
Three stationary source positions occupy five voxels at 0.1mm in that run: tiny
fitted-pose perturbations straddle cell boundaries. A regression independently
checks every fused position and coverage against the three analytic coordinates
within 1e-7mm; it does not assume exactly three cells or claim physical precision.
This is a synthetic numerical consistency check, not hardware accuracy evidence.
