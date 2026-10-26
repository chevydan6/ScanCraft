# Reconstruction from selected recorded frames

Point clouds → **Reconstruct recorded stereo frames** accepts a calibration file,
left/right recordings, explicit zero-based frame indices, reconstruction settings
and a new result folder. It runs in the existing cancellable geometry worker and
opens accepted geometry after publication. Unsaved document changes require the
existing save/discard choice before replacement. No-geometry results retain the
current document and save diagnostics. No scanner is opened or calibrated.

Commands:

```
scancraft-cli reconstruct-recordings CALIBRATION LEFT_RECORDING LEFT_INDEX RIGHT_RECORDING RIGHT_INDEX SETTINGS NEW_FOLDER
scancraft-app --reconstruct-recordings CALIBRATION LEFT_RECORDING LEFT_INDEX RIGHT_RECORDING RIGHT_INDEX SETTINGS NEW_FOLDER
```

Each source is read-only, fully validated and indexed before selection, using the
same guarded source layer as image playback. File identity/timestamps are checked
around selected reads and after processing. Only packed GREY is accepted here:
left interface 0 and right interface 2, with exact supplied calibration image
sizes. Display-converted JPEG/Y14 data does not silently enter reconstruction.
Selected raw pixels feed the same undistortion, rectification, matching,
triangulation and optional surface/normal pipeline as PNG inputs. No automatic
pairing, timestamp offset correction or exposure synchronization is inferred.

The settings schema is the existing reconstruction request schema. A synthetic
source requires explicitly synthetic recording contexts and synthetic calibration.
Physical observations require a declared binding matching the supplied non-
synthetic calibration. Where present, context device_identity must equal the
binding serial; controls.firmware/mode must match; embedded calibration_bytes
must contain a valid RAPCAL01 record equal to the supplied calibration. Vendor
factory bytes are not interpreted as a verified calibration. Missing context is
preserved as unknown, rather than invented or treated as proof of device binding.
A declared binding is not an independent physical attestation. Every result keeps
synchronization_verified and physical_accuracy_qualified false; the sidecar also
keeps source_device_binding_verified false. This path does not activate or approve
any calibration hypothesis. Physical calibration remains deferred.

The new, atomically published result bundle retains:

- `left-frame.raptor` / `right-frame.raptor`: one-frame v2 recordings with exact
  original payload, UVC metadata, sequence, host time, flags and supported context.
- Exact supplied `calibration.rapcal` and `request.json` bytes.
- `recording-sources.json`: original file references, source versions/header
  protection, frame counts/indices, profiles, contexts, sequence/host/PTS and raw
  payload/metadata checksums, together with explicit qualification limits.
- Existing reconstruction diagnostics, manifest, and accepted PLY/editable
  geometry project plus any requested mesh artifacts.

The retained frame files are normalized to v2. For legacy v1, the sidecar retains
source version 1, absent context and unprotected-header status; a new v2 checksum
does not retroactively qualify the original header. Keep recovery reports with
recovered sources: a valid footer is not proof that an original scan was complete.
Keep the complete result bundle. The editable project retains initial source
metadata and artifact references; raw frame payloads remain external. Full originals are references, not copied snapshots, and no full
original-file cryptographic identity is claimed. Checksums protect accidental
corruption, not adversarial changes. Source mutation guards have the same
per-operation and filesystem limitations documented for recording playback.

Verification is synthetic, explicitly distinct from genuine scanner geometry:

- Six workflow tests cover exact report equality with the PNG pipeline, exact
  selected raw frame/context and calibration/request retention, overwrite refusal,
  corrupted later suffixes, invalid interface/index, cancellation before work and
  after preparation, embedded calibration mismatch, synthetic/physical declaration
  separation, known identity/mode mismatch, unknown context and legacy status.
- A startup parser test verifies explicit numeric indices; the existing unsaved-
  document UI regression now covers recorded reconstruction too.
- Initial adapter checkpoint: 431 workspace Rust tests, strict all-target lint, format and release binaries/
  examples pass. 17 reference/tooling tests pass with .venv-reference Python.
  Logs: evidence/recorded-stereo-*.log.
- evidence/recorded-stereo-cli-proof checks three accepted synthetic points and
  report equality against the PNG path, unchanged input SHA256s and existing-
  output refusal. evidence/recorded-stereo-gui-render shows the native operation
  completed, geometry opened, no camera descriptors and the preserved bundle.

Reproduce the dataset from the bundled synthetic PNG fixture:

```
cargo run --release -p scancraft-workflow --example recorded_stereo_fixture -- NEW_DATASET
python3 tools/check_workbench_render.py NEW_DATASET NEW_EVIDENCE --reconstruct-recordings --captures 3
```

The generator refuses existing directories. The example selects left index 1
and right index 2 deliberately: image content repeats in this fixture; different
sensor times are retained and never called synchronized. No real scanner point
cloud, physical accuracy, live tracking or master-mission completion is claimed.

The editable project now also retains bounded initial source metadata and retained-
artifact references in ORPROJ05. Raw frame payloads remain external; keep the full
bundle. Initial lineage survives edits but is not current per-point correspondence.
See PROJECT_ORIGIN.md for validation, compatibility and persistence evidence.
