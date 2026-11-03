# Recorded target observations

`scancraft-cli observe-calibration-pair REQUEST_JSON NEW_OUTPUT_DIRECTORY`
connects native bright-component detection, shape diagnostics and four-anchor
association to the existing calibration fit input types. It processes explicitly
selected frames from two complete Raptor recordings. It does not access hardware,
fit or install a calibration, infer physical units, or certify marker identity.

## Request

The accepted JSON limit is 1 MiB; the bounded reader may consume one additional
byte to detect overflow. Unknown fields are rejected. All settings are explicit:

| Field | Meaning |
|---|---|
| `schema_version` | `1` |
| `target_units` | `unverified` or `declared_millimeters`; the latter is a caller declaration, not measurement verification |
| `target` | 8–128 entries with `pattern: [x,y]`, `anchor: bool`, `coordinates: [x,y]` for ordinary marks and `null` for anchors |
| `left`, `right` | Objects with `recording` path and zero-based `index`; paths are relative to the request file unless absolute |
| `maximum_host_delta_us` | Maximum absolute host timestamp difference, 0–1,000,000 microseconds |
| `detection` | `threshold`, `minimum_area`, `maximum_area`, `maximum_components`, `anchor_area_above`, `maximum_shape_rms_px` |
| `association` | `seed_radius_px`, `refined_radius_px`, `minimum_matches`, `minimum_support_margin`; existing [target matcher](TARGET_ASSOCIATION.md) semantics |

`pattern` supplies nominal layout coordinates for association. `coordinates`
supplies the exact target coordinates for calibration, independently of nominal
spacing. Four anchors are required; anchors determine orientation but are excluded
from fit inputs. Ordinary coordinates must be finite, unique and bounded by 1e9
in absolute value. An area strictly greater than `anchor_area_above` classifies
an observed component as an anchor. Threshold/area bounds retain the existing
[component detector](../crates/scancraft-image/src/components.rs) semantics.
At most 256 accepted components per camera enter association; no excess marks
are silently sampled away. The image limit is 4,194,304 pixels.

Left/right profiles must identify interfaces 0/2 and contain valid packed GREY
frames. Both selected frames must have equal, present sensor PTS and satisfy the
host timestamp window. Equal frame indices are not required and must not be
assumed to mean synchronized exposures. Timestamp agreement remains an observation,
not proof of exposure synchronization.

## Output and use

The new directory contains the exact `request.json`, `measurements.json`,
`left-frame.raptor`, `right-frame.raptor`, and `manifest.json`. The report retains
all accepted raw components, ellipse-fit diagnostics/errors, full associations,
shape rejection decisions, target indices, exact supplied coordinates, per-camera
views and their stereo intersection. Used pixels are binary component centroids;
ellipse centers are diagnostics only. The method does not correct the physical
perspective bias of a circle's image centroid.

Source lineage records original paths, frame indices, profiles, capture contexts,
sequence/timestamps and payload/metadata CRCs. Both complete source recordings
are validated, including their footers, before selection. File identity/stamp
guards detect replacement or modification through the pre-publication check.
Snapshots retain selected payloads/metadata exactly in the current recording
format; legacy missing context is represented by the default snapshot context,
with original absence retained in lineage. CRCs detect corruption, not authenticity.
Original request paths are not rewritten to the snapshots.

When units are `declared_millimeters` and both camera views and the stereo
intersection contain at least four retained ordinary marks, the bundle also emits
`left-view.json`, `right-view.json`, and `stereo-view.json`. These deserialize as
`planar::PlanarView` and `stereo_fit::StereoView`. Collect appropriate diverse poses,
then insert these views into the explicit requests described in
[BROWN_CALIBRATION_REFINEMENT.md](BROWN_CALIBRATION_REFINEMENT.md) and
[STEREO_EXTRINSIC_CALIBRATION.md](STEREO_EXTRINSIC_CALIBRATION.md). The count gate is
not a conditioning, coverage or accuracy qualification. Units marked `unverified`
never produce files with `target_xy_mm`; raw coordinates and observations remain
available in the report. No coordinate rescaling is performed.

Publication uses existing no-overwrite staging/rename/fsync helpers. Errors after
staging name the retained partial directory. Cancellation is checked through
WorkControl in image/indexing operations and through the external callback between
stages and during association; this does not establish a hard cancellation deadline.
The [native preparation panel](NATIVE_BOARD_PREPARATION.md) can run this request
and inspect its camera views/marker decisions. Guided target setup, a calibration
wizard and automatic multi-pose fit assembly remain unfinished.

## Verification and scope

Source commit `6697e14355c7b4f2e2affc9bd27e7c372b2bae39`; 2026-11-03:

- Four synthetic workflow tests check selected raw frame/context preservation,
  deliberately different left/right frame indices, exact supplied coordinates,
  typed fit views and unit boundaries; malformed pairing/request/source rejection;
  cancellation, dangling destination and mid-publication source replacement; and
  visible shape rejections without fit export.
- Full gate: 839 tests passed, one existing ignored test, formatting, strict
  all-target Clippy and optimized workspace build passed.
- Four genuine saved board poses reproduce all earlier native component pixels,
  shape measurements and associations exactly. Eight selected snapshots replay;
  all manifest artifact CRCs/lengths pass; original recording SHA-256 hashes remain
  unchanged. This is saved-data integration evidence, not a new live capture.

| Pose | Left/right indices | Shape-retained ordinary marks, left/right | Stereo intersection |
|---|---|---|---:|
| Older front | 5 / 6 | 37 / 38 | 37 |
| Older second | 5 / 4 | 31 / 30 | 29 |
| Older third | 5 / 5 | 30 / 28 | 27 |
| Previously held-out full board | 5 / 4 | 38 / 36 | 36 |

These counts precede lens-domain/triangulation rejection; they are not 3D point
counts. The real checks explicitly use `unverified` units. Physical scale,
calibration coverage, a complete object surface and independent dimensional
accuracy remain open; see [real-board evidence](REAL_BOARD_CALIBRATION.md).
The private reproduction script, exact requests, outputs, source/binary hashes
and logs are retained under developer artifacts `recorded-board-workflow-hkmwfvxc`.
No proprietary coordinates or captures are distributed with the source.

The workflow composes existing Rust implementations in `scancraft-image`,
`scancraft-calibration`, `scancraft-recording` and `scancraft-workflow`; it introduces
no third-party dependency or replacement numerical algorithm. CodeRabbit CLI
0.9.2 quota lookup returned `lookup_failed`, so no new automated review or paid
request is claimed. Independent review remains outstanding.
