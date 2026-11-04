# Process recorded laser pairs in the native app

The **Laser candidates** tab can now run the existing native recording-backed
detector, continuity builder and sheet-supported matcher, save a diagnostic bundle,
then verify and display it. This is an offline reconstruction diagnostic with
unverified units and physical accuracy, not a qualified scan mode.

1. Expand **Process recorded laser pair**.
2. Choose the camera model JSON and two `.raptor` recordings. Specify zero-based
   frame indices in the same first/second order as the model and request bindings.
3. Choose the laser matching settings JSON and a **new** result directory.
4. Choose **Process pair**. Stage progress and **Cancel operation** remain available.
5. Inspect both retained camera views, candidate counts and geometry. Failed or
   unmatched queries remain explicit; saving diagnostics is not a scan-quality pass.

The model and settings are the same explicit inputs accepted by
`scancraft-cli match-recorded-laser-traces`; see
[recorded matching contracts](RECORDED_SHEET_MATCHING.md). Settings include frame
CRC/interface bindings, detector/continuity parameters, numerical laser sheets,
triangulation limits and work/report bounds. No partner observation seeds are
required. This interface does not infer optical calibration, activate a model or
automatically change thresholds. Preparing a physically qualified model remains
separate work.

The application also accepts the equivalent explicit startup action:

```sh
target/release/scancraft --match-recorded-laser-traces \
  GEOMETRY_JSON FIRST_RECORDING FIRST_INDEX \
  SECOND_RECORDING SECOND_INDEX REQUEST_JSON NEW_FOLDER
```

Ordinary opening of a project or candidate bundle does not start processing.
Neither this action nor its input validation opens scanner hardware.

## Implementation and failure boundaries

`candidate_ui/processing.rs::Processing` owns only form fields.
`CandidateUi::process` starts `cloud_jobs::Operation::ProcessLaserCandidates` on
the existing owned worker. `cloud_jobs/candidates.rs::process` calls
`recorded_traces::run_automatic`, then uses the same candidate loader/preview path
as opening a saved result. `Inputs` gains equality traits for explicit startup
validation; its serialized inputs and runtime behavior are unchanged.

The numerical detector, correspondence selection, limits and diagnostic schemas
remain unchanged. Matching updates `WorkControl` once per query; finishing marks
the publication stage. The displayed fraction concerns the current stage, not a
wall-clock estimate or a guarantee that the complete operation is finished.
Filesystem/JSON/device-independent work still has no hard cancellation deadline.
Existing whole-application shutdown qualification remains open.

Existing targets are rejected. The workflow preserves incomplete staging folders
and their error paths. If publication succeeds but subsequent display loading
fails or is cancelled, the error explicitly identifies the saved directory and
instructs reopening it. The previous point/camera view and its source label remain
available on failure. Cancelling after publication does not delete a saved result.
Saving may therefore succeed even when opening the display fails.

The diagnostic preview never becomes a metric `Cloud`/`Editor`. Existing editable
projects, histories and selections remain separate. Per-query numerical failures
and zero candidates are valid saved diagnostic outcomes, not implicit success of
reconstruction. Review their counts and complete reports. This path does not yet
provide continuous laser acquisition, cross-pose tracking/fusion, a qualified
surface or metric model export.

## Reproducible checks

The existing original synthetic generator supplies public test inputs:

```sh
cargo run --frozen --release -p scancraft-workflow \
  --example recorded_laser_fixture -- NEW_DATASET
target/release/scancraft --match-recorded-laser-traces \
  NEW_DATASET/geometry.json NEW_DATASET/first.raptor 0 \
  NEW_DATASET/second.raptor 0 NEW_DATASET/request.json NEW_DATASET/app-result
python3 tools/check_workbench_render.py NEW_DATASET NEW_RENDER_EVIDENCE \
  --match-recorded-laser-traces --captures 2
```

The render checker processes `geometry.json`, `first.raptor`, `second.raptor` and
`request.json` at indices zero into `NEW_RENDER_EVIDENCE/result` on a separate
authenticated Xvfb display. Supply `--xvfb` for a nonstandard installation. Inspect
screenshots and output manifests; process survival alone is not a quality check.

Five added regressions cover actual synthetic processing/publication/view loading,
editable-state preservation, cancellation/no overwrite, display cancellation after
publication and successful reopening, startup path/index validation, and form-button
dispatch/error preservation. The form regression clicks its header and button
through egui input events rather than calling the processing method directly.
The final workspace gate passes 721 tests, formatting, strict Clippy and release.

Private native-app integration checks process synthetic, saved box and front-board
inputs into new directories: 126/1764/0 candidates, respectively. Every complete
match record and candidate-point file is byte-identical to earlier results; all
artifact CRCs/lengths and unchanged input hashes are checked. A box input path
contains spaces. A fourth request with total-work bound 1 is rejected before
publication, with a visible error. Rendered box and error screens were inspected;
no process held scanner descriptors. Private data/screenshots remain local.

No new numerical method, dependency, scanner control, calibration operation or
firmware change accompanies this integration. Automated review and publication
remain pending the unavailable CodeRabbit quota/billing lookup.

## Explicit domain exclusions

Requests may opt in with `"retain_valid_domain_segments": true`. This retains
contiguous valid trace sections, records all excluded source vertices/segments and
sheet roots, and displays exclusion counts. Unique candidates are conditional on
the retained domain and supplied hypotheses. Missing/false keeps strict behavior.
See [policy, compatibility and genuine-recording evidence](DOMAIN_BOUNDED_LASER_MATCHING.md).
