# Offline stereo reconstruction in the native workbench

The Point clouds tab now includes **Reconstruct from stereo images**. Choose a
calibration record, left/right grayscale PNGs, a reconstruction settings file and
a new result folder. Press **Reconstruct and open**. The image sizes, declared
source binding, units, projection, matching and triangulation limits are checked
by the same shared Rust workflow used by the CLI. No scanner is opened and no
firmware, calibration or projector setting is changed by this operation.

For an existing synthetic dataset, startup can configure and run the same job:

```sh
./target/release/scancraft --reconstruct evidence/offline-cli-dataset/calibration.rapcal evidence/offline-cli-dataset/left.png evidence/offline-cli-dataset/right.png evidence/offline-cli-dataset/request.json NEW_RESULT_FOLDER
```

The example is synthetic and is not Raptor calibration. The settings format and
input limits are documented in OFFLINE_RECONSTRUCTION.md. Real Raptor calibration,
paired-exposure verification, optical image quality and live reconstruction
remain incomplete/unqualified. This is an offline workflow, not an Alpha or
validated scanning mode.

## Processing and document safety

A single background document worker performs bounded file reading, image-map
construction, resampling, matching, triangulation, preview preparation and result
writing. The UI continues repainting and exposes progress and Cancel. Existing
unsaved-work choices apply to reconstruction as well as file import: keep the
current document, save it before continuing, or explicitly discard edits.

All fallible new cloud/preview construction finishes before bundle publication.
The result folder is published through the existing Linux no-overwrite atomic
rename. Only after successful publication and parent-directory synchronization
does the worker replace the current editor and clear selection. The new project
is already saved, its path is displayed, and subsequent edits use normal undo,
redo, filters and save behavior. An existing output folder is never overwritten.

A request yielding no acceptable points publishes diagnostics/manifest only and
retains the current editor, history and selection. The UI displays point and
rejection counts. Geometry passing numerical checks is still not a physical
accuracy or timing qualification; saved reports keep those flags false.

## Cancellation boundary

The new `scancraft-workflow` crate separates immutable prepared results from durable
publication. Input files must be regular files; nonblocking open and type checks
reject FIFOs/special files before reading. Reads check cancellation at most every
64 KiB. Compressed PNG size/pixel/allocation limits remain enforced. Decode and
JSON parsing are bounded single calls, with cancellation checks between phases;
system I/O, fsync and scheduling do not have a proven latency bound.

Maps/resampling/matching/triangulation retain their existing cooperative checks.
Artifact writes and checksum reads also check cancellation. One final check
occurs immediately before atomic publication. Cancellation before that point
leaves the original document unchanged and may retain a named staging directory
for inspection. Once publication commits, a later cancellation does not retract
saved artifacts or turn a completed result into a cancelled one. A parent-sync
failure explicitly identifies an already published bundle; the current editor
remains intact so it can be reopened manually. This is not a power-loss or p95
cancellation qualification.

## Evidence

Shared workflow tests cover cancellation after preparation before publication,
writer cancellation, bounded/special-file rejection, grayscale PNG validation
and a target appearing during publication. App worker tests cover reconstruction
and exact project coordinates, failure/cancellation preserving previous history,
and no-geometry runs retaining the editor while saving diagnostics. The UI guard
test ensures dirty imports/reconstructions wait for the user's choice.

The real release app was launched on a private authenticated Xvfb display with
software rendering, reconstructed the rotated-camera synthetic pair, published
the bundle and displayed three points with the expected millimeter bounds. No
camera device descriptors were open at capture. Its four artifacts match the
CLI fixture bundle byte for byte. The screenshot and inspection metadata are
under evidence/workbench-reconstruction-final. This does not qualify a complete
interactive workflow, hardware GPU behavior, real sensor geometry or live FPS.

```sh
python3 tools/check_workbench_render.py --reconstruct evidence/offline-cli-dataset NEW_CAPTURE_FOLDER
```

The helper starts and cleans up its own test app/display, without using the user's
desktop or starting capture. Review the screenshot; process survival alone is not
proof of successful reconstruction. Full raw inputs and reports should be kept
with the project because the editable project alone contains geometry/history,
not all acquisition provenance.
