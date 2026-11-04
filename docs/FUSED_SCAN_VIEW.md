# Native view of recorded fused scans

Experimental read-only view of the multi-frame output from
[`fuse-recorded-candidates`](RECORDED_MARKER_FUSION.md). Coordinates remain in
unverified supplied model units. Opening a result does not start scanner capture,
activate calibration or create a metric editing project.

## Open and inspect

In **Laser candidates**, enter the fused result folder and click **Open fused
scan**. **Frame / pair** selects the original camera pair to display, starting
at 1. The point view always shows the **entire fused result**, not just that
selected source frame. Expand **Tracking and fusion outcomes** for frame IDs,
accepted surface/marker counts, recorded RMS and reasons. Summary counts distinguish
integrated, tracking-only, rejected and unavailable frames. Orbit, pan, zoom and
reset use the existing renderer.

The native startup form uses a zero-based source index:

```sh
target/release/scancraft --fused-candidates RESULT_DIRECTORY
# Show the third original camera pair while retaining all fused points:
target/release/scancraft --fused-candidates RESULT_DIRECTORY 2
```

Keep the original single-pair source bundles at their recorded absolute paths.
The fused result contains references, not copies of those recordings. Missing or
changed sources produce an explicit error; a failed reopen preserves the prior
loaded view and its source label. A successful open replaces its points, cameras
and outcome summary together. Existing single-pair/capture views remain separate.
Metric save/export/edit actions are not introduced into this diagnostic panel.

## Checked implementation boundaries

`tracked_candidates::bundle::load` and `bundle::rows` in the workflow crate:

- Accept only the fixed `recorded_marker_laser_fusion` schema and six unique known
  artifacts. Validate lengths and CRCs without following final-path symlinks or
  accepting arbitrary artifact filenames. CRC is integrity, not authentication.
- Validate unverified-unit/physical flags, finite coordinates, count limits and
  consistency among the request, per-frame outcomes, final voxel count and summary.
  Check status/pose/fusion presence, finite pose/range diagnostics, frame ID order,
  last-integrated progression and resource limits. This is structural validation,
  **not a recomputation of marker, tracking, laser or fusion mathematics**.
- Reopen every referenced source pair through `candidate_bundle::load_recorded_pair`.
  Compare the retained manifest, geometry, source validation summary, observation
  references, candidate count, camera order, timestamps and raw frame bindings.
  Source pairs are checked sequentially as snapshots, not locked for the lifetime
  of the view. Retain only the selected pair's display thumbnails.
- Enforce 256 frames and one million fused points. Point JSON is capped at128MiB;
  frame rows at2MiB, source rows at512MiB and streamed marker diagnostics at1GiB.
  NDJSON rows are bounded before parsing (8KiB frame /2MiB source), with at most256
  rows each. Marker diagnostics are checksum-checked without a giant JSON tree.
  Cooperative cancellation applies while reading and between bounded computations;
  filesystem calls are not hard realtime or wall-clock-preemptible.

`cloud_jobs::candidates::view_fused` dispatches in the existing background worker,
returns `SessionView` and a diagnostic `Preview`, and never writes `Document.editor`.
Preview sampling is capped at50,000 points with full-data bounds and an explicit
"displaying N of M" label. Sampling changes only display density. `CandidateUi`
keeps the complete result separate from the selected source camera images and
shows unverified units throughout. The startup parser caps source index at255.
No geometry algorithms, dependencies, acquisition controls or file writes are
added to the opening path.

## Validation — 2026-11-04

`tools/check.sh`: **921 passed, 0 failed, 1 ignored**; formatting, strict all-target
Clippy and optimized workspace build pass. Four loader regressions cover positive
mixed frame outcomes and selected cameras, malformed counts/flags/roles, corruption,
missing sources, inconsistent frame rows, symlinks, incomplete NDJSON, cancellation
and changes before publication of a loaded view. Three app regressions exercise
startup bounds, worker document preservation and actual egui button dispatch with
failure retention. One additional depth regression confirms missing PTS in both
cameras is already rejected by existing frame validation before pairing. Investigation
initially suspected the optional equality check, but the pre-change test passed;
**no production pairing defect or fix is claimed**.

The exact release application was opened on a private authenticated Xvfb display
with software rendering, using the existing render checker extended with
`--fused-candidates` and `--fused-frame-index`. No scanner descriptors were open at
the screenshot checks. All four screens were inspected:

| Input | Displayed points | Source cameras | Observed result |
| --- | ---: | --- | --- |
| Three genuine stationary box trials |2,188| Third pair | Sparse traces and distant candidates;3 integrated frames |
|31 genuine rotating-board pairs |192| Last pair | Sparse fragments;10 integrated /21 tracking-only |
| Same board sequence with no laser candidates |0| Last pair | Empty geometry,31 tracking-only; original images remain visible |
| Copied result with an altered copied source |0| None loaded | Explicit changed-source/provenance error |

A private compiled adapter calls the same loader for all three genuine results.
Every loaded coordinate and frame outcome matches the existing saved output.
Independent NumPy/Pillow checks reconcile the selected original PNG payload CRCs,
nearest-sample thumbnails, image dimensions, clipping counts, nearest-rank
percentiles and full-image mean exactly. The changed-source case updates its
internal result/source checksums deliberately; the recorded source-manifest link
still rejects it. Original input/result hashes remain unchanged. Private captures,
models, screenshots, adapters and logs stay outside Git in `fused-scan-view-20261104`.

To reproduce startup rendering with an available private result:

```sh
python3 tools/check_workbench_render.py RESULT_DIRECTORY NEW_RENDER_EVIDENCE \
  --fused-candidates --fused-frame-index 2 --captures 2
```

Supply `--xvfb PATH` where needed. These checks establish startup/loading/display
behavior; they do not measure rendering FPS, hardware GPU performance or a complete
editing/save/export workflow. Visual inspection still shows insufficient object
coverage. No source geometry was filtered or replaced to improve appearance.

Parent recorded workflow3063814 was independently verified in assignment547,
including its numerical replay and exact-head CI. This viewer requires its own
independent review. CodeRabbit quota lookup remains unavailable (`lookup_failed`),
so no substantive automated review or paid usage is claimed. Next priority is a
controlled rotating-box capture, object-associated tracking/coverage and independent
physical calibration/scale/accuracy validation, not further editing features.
