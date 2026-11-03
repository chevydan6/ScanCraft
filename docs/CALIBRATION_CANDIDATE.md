# Fitted calibration handoff to reconstruction

Implementation commit: `29e3769da87fea02cbfaec5eafff91ca24d988aa`.

The existing stereo fitter produced a diagnostic bundle, while image, recorded
and live reconstruction require a RAPCAL01 calibration record. The new workflow
connects those stages without manually transcribing fitted camera coefficients.
It runs the existing fixed-intrinsic stereo fitter, applies its existing acceptance
checks, constructs the camera geometry directly from the fit, and writes/readbacks
a calibration record. It does not accept a possibly stale fit JSON as a substitute
for observations.

```sh
scancraft-cli fit-calibration-candidate REQUEST_JSON NEW_OUTPUT_DIRECTORY
```

The command opens no camera and makes no device or firmware changes. It writes an
experimental candidate, not an installed or physically qualified calibration.
Selecting that file for reconstruction is a separate explicit action. There is
no dedicated candidate-export GUI control in this milestone.

## Request and provenance

The bounded JSON request rejects unknown fields and accepts at most 16 MiB,
including the embedded observation request:

| Field | Meaning |
| --- | --- |
| `schema_version` | Must be `1`. |
| `record_id` | Nonempty record identifier, at most 128 bytes. |
| `target_units` | Must be `declared_millimeters`; `unverified` is rejected. |
| `unit_reference` | Nonblank description of the supplied unit reference, at most 2,048 bytes. |
| `binding` | Explicit device serial, firmware, mode and left/right image sizes, or `null`. |
| `observations` | Complete schema-1 request accepted by `calibrate-stereo-extrinsics`: provenance, fixed camera models, views and fitting options. |

See [stereo fitting](STEREO_EXTRINSIC_CALIBRATION.md) for observation coordinates,
right-from-left pose conventions, numerical gates and lens-domain limits. Values,
lens radii and tolerances are passed unchanged; the command does not estimate or
convert target scale. A textual unit reference and device binding remain caller
declarations. They do not independently verify measurements or actual hardware.
Do not declare the retained QR coordinates to be millimeters without corroboration.

Synthetic observations always produce `SyntheticFixture`, even when a binding is
provided; existing physical-capture validation rejects them. Supplied observations
produce `EstimatedFromObservations`, with binding retained as declared. An absent
binding remains absent and cannot pass matching physical-capture checks. Image
sizes and metadata must satisfy the existing calibration-record validator.

## Files and downstream use

A new directory contains:

- `request.json`: exact original bytes, including observations and declarations.
- `fit.json`: fitted geometry, residual/stationarity diagnostics and explicit false
  unit-verification, device-verification, synchronization and accuracy flags.
- `calibration.rapcal`: existing RAPCAL01 format, with the fitted geometry, declared
  binding, derived provenance and notes identifying the request byte count/CRC32.
- `manifest.json`: byte counts/CRC32 for those artifacts, provenance and explicit
  false verification/activation flags.

Retain the complete candidate bundle. CRC32 detects accidental changes, not source
authenticity. Raw camera recordings and their source observation bundles must also
be retained separately; they are not embedded in this candidate. Intrinsics and
valid lens radii are supplied inputs, so this handoff does not establish their
coverage or qualification.

The record is compatible with existing `reconstruct-stereo`,
`reconstruct-recordings`, recorded-sequence and live-stereo inputs. Their existing
provenance, binding and geometry checks remain. No file is automatically selected
in the GUI or installed into the scanner. A valid candidate file alone does not
prove optical accuracy or camera synchronization.

The workflow snapshots the bounded input before fitting, supports cooperative
cancellation, syncs artifacts and uses existing no-overwrite atomic directory
publication. Existing outputs, including symlinks, are rejected. Failed publication
retains its partial directory; a failure syncing the destination parent explicitly
reports that publication already happened. No recording/project format or
third-party dependency changes.

## Verification and remaining work

Three new tests verify the complete fit-to-reconstruction handoff, preservation of
synthetic provenance, exact input and artifact integrity, existing-output refusal,
unknown units, invalid bindings/observations and cancellation. A fitted synthetic
rig reconstructs the existing three-point image fixture with coordinates within
1e-5 mm of its original synthetic calibration. This is software numerical evidence,
not a Raptor accuracy measurement. Declared-input tests retain false qualification
flags; they do not impersonate hardware evidence.

The full `tools/check.sh` gate passes **878 tests**, one existing ignored test,
formatting, strict all-target Clippy and optimized workspace builds. A separate
release CLI run fits a candidate, validates RAPCAL01 length/checksum/provenance,
reconstructs the same three synthetic points and publishes the normal reconstruction
bundle. A request containing the real retained Raptor stereo observations with
`target_units: unverified` fails before candidate publication; the original source
hash is unchanged. No real-data metric calibration or scanner acquisition occurs.

Private `calibration-handoff-20261104` retains source/executable bindings, exact
requests, CLI outputs, verification script and gate logs. Public code is in
`scancraft-workflow/src/calibration_candidate.rs`, its tests, workflow module
registration and the CLI dispatch/help. It reuses existing independently developed
Rust fitting and storage code; no algorithm or third-party code was imported.

Independent unit/scale measurements, broader calibration/holdout coverage, actual
device binding, a measured object in three trials and native GUI qualification
remain outstanding. CodeRabbit quota lookup is unavailable; no completed new or
paid review is claimed. Independent review of this handoff remains pending.
