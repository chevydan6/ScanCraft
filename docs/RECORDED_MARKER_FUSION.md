# Native recorded marker tracking and laser fusion

Status: experimental, unverified model units. This extends the separate tracking
API in [SEPARATE_TRACKING_FUSION.md](SEPARATE_TRACKING_FUSION.md) into a native CLI
workflow. It is not a qualified object scan or metric export. Read-only native result viewing
is now documented in [FUSED_SCAN_VIEW.md](FUSED_SCAN_VIEW.md); live GUI fusion remains
outstanding.

## Invocation and boundaries

```sh
cargo run --release --frozen -p scancraft-cli -- \
  fuse-recorded-candidates settings.json new-result-directory
```

Inputs are ordered **single-pair** `recorded_sheet_supported_matching` bundles
produced by `match-recorded-laser-traces`, not a live-capture directory, arbitrary
point arrays or display thumbnails. Each bundle must retain all seven artifacts,
including both original raw frames, its geometry, matching request and candidates.
Paths resolve relative to the settings file. Preserve these input bundles: the
output records their absolute paths and manifests but does not copy their raw data.
Private captures, geometry and result bundles must stay outside the public repo.

Example settings (illustrative experimental gates, not validated defaults):

```json
{
  "schema_version": 1,
  "session_id": "recorded-trial",
  "markers": {
    "threshold_exclusive": 100,
    "minimum_area": 5,
    "maximum_area": 1000,
    "maximum_components": 100000,
    "maximum_shape_rms_px": 0.5,
    "minimum_depth_model_units": 100.0,
    "maximum_depth_model_units": 1000.0,
    "minimum_ray_angle_rad": 0.0001,
    "maximum_ray_gap_model_units": 1.0,
    "maximum_reprojection_error_px": 2.0
  },
  "tracking": {
    "max_correspondence_model_units": 3.0,
    "max_rms_model_units": 0.25,
    "minimum_overlap": 0.7,
    "max_step_translation_model_units": 15.0,
    "max_step_rotation_rad": 0.05,
    "iterations": 30,
    "tolerance_model_units": 1e-8
  },
  "voxel_model_units": 0.25,
  "maximum_voxels": 100000,
  "maximum_stereo_attempts": 1000000,
  "frames": [
    {"id": 0, "bundle": "pair-0"},
    {"id": 1, "bundle": "pair-1"}
  ]
}
```

All length settings are in the supplied geometry's **unverified units**. Existing
numerical kernel fields named `*_mm` are mapped internally without scaling or
asserting physical millimeters. The retained geometry also preserves its original
field names. No calibration is fitted or activated by this command.

## Implemented data path

- `candidate_bundle::load_recorded_pair` validates the existing bundle, artifact
  lengths/CRCs, raw frame bindings, image dimensions, interfaces, equal raw sensor
  PTS and bounded host timestamp difference. Its owned `RecordedPair` includes
  exact raw frames, candidate coordinates, geometry and the original manifest.
  Display-only `FrameView` copies never feed marker processing. CRC is integrity
  evidence, not authentication; numerical laser matching is not recomputed here.
- `tracked_candidates::markers::reconstruct` calls existing eight-connected
  `components::detect`, boundary-conic `shape::measure`, and
  `StereoCalibration::prepare` / `PreparedStereo::triangulate`. It uses binary
  area centroids of shape-accepted components, not fitted ellipse centers. Every
  Cartesian pairing is attempted and recorded, including geometric errors; only
  mutual unique accepted pairs enter tracking. Uniqueness is not physical identity.
- `ScanSession::process_with_tracking_cloud` estimates pose from marker candidates
  and integrates **only laser surface candidates**. Existing ICP quality gates
  and voxel averaging remain unchanged. An accepted pose with no surface creates
  a tracking-only row. A rejected pose has no current pose and adds no geometry.
  Fewer than three markers creates an explicit unavailable row without calling
  the tracker; the last accepted reference remains available for a later attempt.
- `ArtifactWriter` streams complete marker diagnostics, source manifests and
  per-frame outcomes. JSON candidates retain `units_verified: false` and all
  physical qualification flags false. There is no automatic metric PLY, editable
  project, mesh, fitted cuboid, object segmentation or invented missing geometry.

Output: `request.json`, `geometry.json`, `markers.ndjson`, `sources.ndjson`,
`frames.ndjson`, `fused-candidates.json`, and a digest-bearing `manifest.json`.
Marker records have explicit frame-start boundaries, component/shape outcomes,
all pairing outcomes, uniqueness decisions and tracking observations. Frame rows
separate integrated, tracking-only, rejected and unavailable outcomes.

## Bounds and failure behavior

The request is at most 1 MiB, with 1–256 frames enforced during deserialization.
Each source loader enforces its existing artifact limits. Components count all
examined noise before rejection (maximum 100,000); shape processing rejects more
than 512 accepted regions per image rather than truncating them. Complete stereo
attempts have an explicit sequence budget at most one million. Fusion is capped
at one million voxels. Processing retains one frame pair and bounded graph at a
time; diagnostics stream to disk. These are offline workflow bounds, not a removal
of live acquisition's separate capacity limits.

Frame IDs and both host timestamp sequences must strictly increase. Geometry and
camera-interface order must remain identical; repeated source frames or mixed
models fail. Geometry/image errors, exhausted budgets, cancellation and fusion
errors abort publication, preserving a named partial directory for diagnosis.
Only a complete stopped session publishes atomically using no-overwrite rename.
Source files are read-only. Existing destinations are never replaced. CLI SIGINT
and SIGTERM feed `WorkControl` through a scoped 20 ms monitor. Filesystem calls and
bounded numerical kernels are not hard realtime or wall-clock-preemptible.

## Verified evidence, 2026-11-04

Private evidence root: `recorded-marker-fusion-20261104` under the developer's
local artifacts directory. It contains source/binary binding, settings saved before
execution, full outputs, numerical verifier scripts, logs and process receipts.
Inputs come from previously retained genuine Raptor recordings; no hardware action
or new calibration took place in this milestone.

| Replay | Source surface candidates | Integrated / tracking-only frames | Fused candidates |
| --- | ---: | ---: | ---: |
| 31 consecutive rotating-board pairs, previous box laser preset | 218 | 10 / 21 | 192 |
| Same 31 pairs, previous board laser preset | 0 | 0 / 31 | 0 |
| Three saved stationary yellow-box trials | 3,156 | 3 / 0 | 2,188 |

`verify.py` reconciles **all 35,853** rotating stereo attempts, component and shape
results, marker clouds, poses and final fused positions against the archived
prototype. Marker clouds, poses and fused positions match exactly. Independent
NumPy camera math checks 893 accepted marker pairs: maximum coordinate difference
8.177e-10 model units; independent transformed voxel means differ by at most
5.685e-14. `verify_box.py` checks 63 accepted marker pairs plus all 3,156 surface
candidates: maximum coordinate discrepancy 2.440e-9 model units. Independent ICP
reconciles both later box poses within 4.548e-13, and independent voxel means within
1.137e-13. The first pose is an identity anchor convention, not a measured error.
Box marker RMS values for the two later trials are 0.103079 and 0.160413 model
units; no external pose accuracy is established.

A single warm rotating replay measured **0.81 s wall**, 0.66 s user CPU, 0.14 s
system CPU and **16,172 KiB peak RSS** with `/usr/bin/time -v`, including bundle
loading, processing and output. Zero filesystem inputs indicate cached reads.
This is not a live throughput, rendering FPS, repeated benchmark or long-run
memory qualification. It excludes acquisition, laser extraction/matching and rendering;
surface candidates were already computed. Existing comparison evidence also uses warm saved data.

The real zero-surface replay retained 31 accepted tracking-only poses with zero
fused points. A real CLI SIGINT after staging began returned exit 1/cancellation,
retained partial output and published no target. A second invocation against a
completed target failed and every existing output hash remained unchanged.

Four new workflow regression tests cover actual raw-frame dispatch, laser/marker
separation, empty surface, unavailable observations, rejected pose/recovery, mixed
geometry, nonmonotonic IDs/timestamps, corruption, budget failure, cancellation and
no overwrite. Final `tools/check.sh`: **913 passed, 0 failed, 1 ignored**, formatting,
strict all-target Clippy and optimized workspace build pass. Initial fixture
failures (missing skew, insufficient conic shape support, boundary-sensitive voxel
assertion) were corrected in fixtures without relaxing production gates; failed
full-gate log and failure summary remain private evidence.

## Remaining limitations and next work

These are sparse candidate reconstructions. The inspected box plot contains
fragmented surface traces and distant/background or erroneous candidates, not a
complete isolated box. No points were removed to make the result look better.
Three stationary trials do not supply new viewing angles or three qualified scans.
Marker identity, association to the moving object rather than its surroundings,
laser identity, exposure synchronization, calibration validity, verified scale,
dimensional accuracy and full coverage remain unqualified. Known box dimensions
were not used to tune coordinates, scale, association or selection.

The native diagnostic viewer now loads this explicit multi-frame format with
source verification (see FUSED_SCAN_VIEW.md). Next: a controlled rotating-box
recording to test object-associated tracking and surface coverage. Preserve failed frames and competing hypotheses. Then validate
physical scale and dimensions independently before permitting metric edit/export.
No new dependencies or geometry algorithms were introduced. CodeRabbit 0.9.2 quota
lookup returned `lookup_failed`/exit 1; no review upload, paid usage or substantive
review pass is claimed. Independent reviewer verification remains separate.
