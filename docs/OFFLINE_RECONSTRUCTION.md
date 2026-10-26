# Native offline stereo command

`scancraft-cli reconstruct-stereo` now exposes the Rust image-to-cloud pipeline.
It reads an integrity-checked calibration record, two source-camera grayscale
PNGs and a bounded JSON request, then publishes a new result directory. It does
not contact the scanner, change calibration or verify image/exposure pairing.
Physical Raptor calibration and mode qualification remain unfinished.

```sh
cargo build --release --workspace
./target/release/scancraft-cli reconstruct-stereo calibration.rapcal left.png right.png request.json NEW_RESULT_DIRECTORY
```

A complete synthetic fixture is reproducible without any scanner or vendor files:

```sh
cargo run --release -p scancraft-cli --example synthetic_stereo_dataset -- NEW_SYNTHETIC_DATASET
./target/release/scancraft-cli reconstruct-stereo NEW_SYNTHETIC_DATASET/calibration.rapcal NEW_SYNTHETIC_DATASET/left.png NEW_SYNTHETIC_DATASET/right.png NEW_SYNTHETIC_DATASET/request.json NEW_SYNTHETIC_RESULT
```

This fixture represents a rotated camera viewing an analytic 400 mm plane. It
produces three synthetic points and one rejected border seed. It must never be
used as a Raptor calibration. Saved example inputs and outputs are under
`evidence/offline-cli-dataset` and `evidence/offline-cli-verified`.

## Request

Schema version 1 requires every field below; unknown fields are rejected. There
are no guessed physical calibration, matching or depth defaults. Seed pixels
are in the explicit rectified output grid, not the original distorted images.
The synthetic example contains a complete valid request:

```json
{
  "schema_version": 1,
  "source": "synthetic_fixture",
  "output_grid": {
    "width": 64, "height": 32, "fx": 80.0, "fy": 80.0,
    "cx": 31.5, "cy": 15.5, "skew": 0.0,
    "distortion": {"k1": 0.0, "k2": 0.0, "k3": 0.0, "p1": 0.0, "p2": 0.0},
    "valid_radius": 1.0
  },
  "seeds": [[0, 0], [20, 10], [36, 20], [50, 24]],
  "matching": {
    "patch_radius": 2, "min_disparity": 1, "max_disparity": 8,
    "min_patch_stddev": 5.0, "min_correlation": 0.9,
    "min_uniqueness_margin": 0.05
  },
  "triangulation": {
    "min_depth_z_mm": 10.0, "max_depth_z_mm": 1000.0,
    "min_ray_angle_rad": 0.0001, "max_ray_gap_mm": 0.01,
    "max_reprojection_error_px": 0.01
  }
}
```

For real observations, `source` is `{"observations": {"serial": "...",
"firmware": "...", "mode": "...", "left_size": [W, H],
"right_size": [W, H]}}`. The binding must match a nonsynthetic calibration
record. These declarations do not prove physical provenance or synchronization.
Source PNG dimensions must match their original calibration exactly. Only
monochrome PNGs decoded to 8-bit intensity are accepted; low-bit grayscale can
be expanded losslessly by the PNG decoder. Color, alpha and 16-bit images are
rejected. This command performs no implicit color conversion or image resizing.

## Outputs and integrity

The output directory must not already exist, even if empty. Processing and file
writes finish in a sibling staging directory. Files and the staging directory
are flushed, and Linux `renameat2(RENAME_NOREPLACE)` atomically publishes the
bundle. A target appearing during processing is also protected against overwrite.
A write/publication failure retains the staging directory and names it in the
error for inspection. A successful directory publication is followed by parent
directory synchronization. Unsupported filesystems/syscalls fail explicitly.

- `report.json`: original calibration/provenance, rectification, matching scores,
  rejected seeds, mapping failures, mapped observations and reconstruction.
- `points.ply`: double-precision millimeter coordinates in original left-camera
  axes. Present only when at least one point passes all processing checks.
- `project.orproj`: the same cloud in the existing editable project format,
  ready for the native workbench's project loader. Fresh edit history starts here.
- `manifest.json`: schema/version, counts/status, input sizes/CRC-32 and artifact
  sizes/CRC-32. Standard output prints the same manifest.

The report and manifest explicitly keep synchronization and physical-accuracy
qualification false. Status `geometry_produced` means points passed configured
numerical checks; it is not a physical accuracy or scanner qualification result.
`no_geometry` is a successful diagnostic run with no PLY/project and no invented
vertices. Consumers must inspect status/counts. CRC-32 detects accidental
corruption; it does not authenticate inputs. The manifest does not include a
checksum of itself. Retain the full bundle and original input files: the project
alone contains geometry/history, not the raw images or full scan provenance.

Inputs are bounded to a 1 MiB calibration payload, 16 MiB per compressed PNG and
4 MiB request. PNG decoder allocation is limited to 32 MiB before pixel decoding;
image pixel count and all existing matcher work limits remain enforced. The CLI
currently processes one pair synchronously. The shared scancraft-workflow library
also powers the GUI background job with progress and cooperative cancellation.
Automatic recording-pair selection, seed selection and live scanning remain
future work. See GUI_RECONSTRUCTION.md for the workbench workflow.

## Verification and precision fix

Three process-level tests invoke the actual CLI binary, check rotated-image
geometry and original axes, verify artifact checksums, load PLY/projects, compare
coordinates exactly, preserve diagnostics for blank pairs and refuse malformed
inputs/existing output. Two unit tests cover bounded PNG validation and a target
appearing between staging and atomic publication.

The new PLY/project comparison exposed a one-ULP project JSON parsing change.
Workspace `serde_json` now enables `float_roundtrip`. A regression test verifies
coordinate bits after save/load and undo using difficult finite values,
subnormals, signed zero, maximum finite values and deterministic random finite
bit patterns. This preserves numerical values encoded in current files without
changing the project schema; it cannot recover precision already lost by an
older reader and then resaved. Evidence of the original failure is retained in
`evidence/project-float-regression-before.log`.

See STEREO_RECTIFICATION.md and STEREO_MATCHING.md for algorithmic contracts.
No physical Raptor scan, M2 milestone or end-to-end scanner qualification is
claimed by these offline tests and fixtures.
