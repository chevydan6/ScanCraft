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
Selecting that file for reconstruction is a separate explicit action. The native Calibration panel now offers explicit fitting and selection.

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


## Native application fitting and selection

GUI implementation commit: `bb0e3702ac75d494cd542241fdd9d333648a048b`.

Open **Calibration → Fit calibration**, choose the candidate request and a new
folder, then select **Fit and save candidate**. The same workflow runs on a joined,
cancellable background thread. Saved candidate identity and provenance are shown;
errors retain the previous successful candidate and distinguish the absence of a
previous result. A worker panic reports uncertain publication status so partial or
published outputs are not silently described as absent.

Fitting does not select the result or start acquisition. **Use for reconstruction**
is a separate action: it fills the calibration fields for image, recorded-pair,
recorded-sequence and live reconstruction, and opens the recorded-pair controls.
Other input fields, the loaded model and its dirty state remain unchanged. Selection
is disabled during relevant capture/reconstruction/fitting work and closing. The
existing reconstruction validators still read and validate the selected file when
processing starts. Choosing a synthetic candidate does not make it usable for
physical capture.

The recorded-board observation view remains separate and retains its previous
images/results while fitting. Both tasks participate in the root window's close
confirmation and cancellation; worker destruction joins rather than detaches.
The application also accepts an explicit startup request:

```sh
scancraft --fit-calibration-candidate REQUEST_JSON NEW_OUTPUT_DIRECTORY
```

This is an advanced explicit-request workflow. It does not yet collect a complete
multi-pose calibration session, infer a device identity, confirm physical scale,
or provide an automatic calibration wizard.

Four additional app regressions verify failure/panic retention, cancellation and
duplicate-start protection, startup/busy/close integration, and preservation of
dirty-document state and unrelated input paths. The final full gate passes
**882 tests**, one existing ignored test, formatting, strict all-target Clippy and
optimized builds. The underlying fitting/storage workflow is unchanged.

The exact release GUI was inspected on an isolated authenticated X display. It
fit the synthetic request; all four candidate artifacts equal the earlier CLI
output byte-for-byte. An actual **Use for reconstruction** click opened the
recorded-pair panel with its calibration populated, and a subsequent **Live scan**
click showed that field populated with acquisition still idle. Camera/USB file
descriptors were absent at the recorded checks. A separate run using the genuine
retained QR observations with unverified units showed the rejection and created
no candidate directory. Final screenshots and exact binary/source bindings are
retained privately in `calibration-gui-20261104`.

The first render preflight failed before launching because Xvfb remained in the
legacy tool-cache directory; supplying its existing path corrected the harness.
An initial error label incorrectly mentioned a previous candidate when none
existed; the final label, regression, full gate and GUI checks cover that state.
These are device-free UI and integration checks, not a real calibrated scan or
complete object-edit/export qualification. No user desktop, scanner firmware,
factory calibration or installed driver was changed. Independent review remains
pending and CodeRabbit quota lookup remains unavailable.
