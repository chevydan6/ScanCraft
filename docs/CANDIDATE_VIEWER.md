# Native recorded-candidate viewport

Open **Laser candidates** in the native app, enter the result folder from
`match-recorded-laser-traces`, and choose **Open candidates**. Orbit by dragging,
pan with Shift-drag and zoom with the scroll wheel. The loaded folder remains
identified if a subsequent load fails; the previous view is retained.

```sh
target/release/scancraft --laser-candidates RESULT_FOLDER
```

This is a read-only diagnostic view of coordinates in an explicitly unverified
camera/laser model. It displays candidate count, query failures and unmatched
queries. Units and physical accuracy are labeled unverified. The normal editable
project, selection and history remain in the Point clouds tab. These coordinates
do not enter `Cloud`/`Editor`, metric measurements or geometry exports.

## Components and validation

- `workflow/candidate_bundle.rs::load` validates a fixed bundle layout and returns
  finite coordinates, observation references and explicit verification limits.
- `scan_project_load::Guard` is reused internally for bounded regular-file reads,
  no-follow root/artifact opens, lengths/CRCs and before/after identity checks.
  Its existing scan-project behavior is unchanged.
- `cloud_jobs::Operation::ViewCandidates` uses the existing owned background job
  to prepare a bounded `Preview`; it leaves `Document` unchanged.
- `candidate_ui::CandidateUi` owns only the diagnostic preview and view controls.
  `cloud_ui::draw` supplies the existing depth-sorted renderer. Root startup,
  tab selection, cancellation and close handling include this loading job.

The loader requires the matching schema/kind, consistent query counters and the
seven known artifact roles. Unknown/path-traversal/duplicate roles fail. Every
artifact's complete length/CRC is checked; fixed limits are 1 MiB for manifest,
geometry and request, 32 MiB per retained recording, 128 MiB each for observations
and streamed match diagnostics, and 4 MiB for candidate points.

Both retained recordings must fully validate and contain one GREY frame, matching
the supplied numerical geometry dimensions and request's interface/payload/
metadata CRC bindings. Interface order, raw-PTS equality and host window are
checked again. These checks do not establish physical exposure synchronization.

Typed summaries skip unused observation details rather than allocating a complete
JSON value tree. Limits of 100000 observation entries and 4096 coordinate entries
are enforced during deserialization. Coordinates must be finite triples with
distinct, in-range first-observation references. Qualification flags must remain
false. Counts and coordinates are not evidence that matching itself is correct.

The complete `matches.ndjson` file is integrity-checked as bytes, without numerical
recomputation or validating each match record's semantics. Original full recordings
remain historical references; the loader checks retained frame files only. CRCs
are not authentication. Verification describes the loaded snapshot, not ongoing
monitoring of later filesystem changes. Parsing/filesystem phases have bounded
input sizes but no hard wall-clock deadline; worst-case memory/latency qualification
and the existing application-shutdown limitation remain open.

## Reproducible synthetic check

The generator creates original synthetic camera/stripe data without hardware or
private calibration values. It refuses an existing output directory and runs the
real recorded automatic workflow, producing 126 candidates from 128 observations.

```sh
cargo build --frozen --release --workspace
cargo run --frozen --release -p scancraft-workflow --example recorded_laser_fixture -- NEW_DATASET
target/release/scancraft --laser-candidates NEW_DATASET/result
python3 tools/check_workbench_render.py NEW_DATASET/result NEW_RENDER_EVIDENCE --laser-candidates --captures 2
```

The render checker uses a separate authenticated Xvfb display. Pass `--xvfb` when
that executable is installed outside its default search locations. No scanner is
opened for this check. Generated records declare synthetic input and the viewport
retains unverified-unit/accuracy labels.

## Verification and current scope

Nine new regressions cover artifact corruption, role/path/symlink rejection,
qualification/count/reference checks, retained-frame binding/footer validation,
input mutation/cancellation, typed item bounds, failure preservation, diagnostic
labels and root-close handling. Startup parser coverage includes the new option.
The complete 714-test, formatting, strict Clippy and release gate passed.

Private-display renders were inspected for both the 126-point synthetic fixture
and the saved 1764-point box-scene bundle. Both show the unverified-unit label and
candidate counts, and neither process held scanner device descriptors. Source and
binary hashes accompany the screenshots locally. The initial render attempt lacked
the configured Xvfb location; the existing explicit-path option resolved it.
No private screenshots or scan/calibration data are distributed.

This connects existing recorded numerical candidates to native visualization.
It does not yet start reconstruction from the tab, continuously scan, qualify
the box surface, track/fuse poses or generate a calibrated model. Signal-quality
feedback and bounded continuous processing remain important next steps.
