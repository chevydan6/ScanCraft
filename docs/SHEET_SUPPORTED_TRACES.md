# Sheet-supported stereo trace matching

`scancraft-calibration::laser_trace::PreparedMatcher` connects existing camera,
quadratic-sheet, epipolar-polyline and stereo-triangulation primitives. It can
find a candidate without a manually supplied partner observation. This is a
numerical component, not a qualified scanner mode or a complete scanning UI.
Implementation is original Rust composition; no new dependency or copied source.

## Method and contracts

For a first-camera pixel, intersect its undistorted ray with every supplied
`QuadraticSheet`. Sheets use the first-camera coordinate frame and the same length
unit as stereo translation and triangulation limits. Both positive in-range roots
are retained. Transform each root into the second camera and project it there.

Independently intersect the first pixel's epipolar line with every supplied
second-camera trace. Segments interpolate in undistorted normalized coordinates;
all crossings and near-coincident segments are retained. Triangulate every
crossing with explicit depth, ray-angle, ray-gap and reprojection limits.

A sheet root supports a crossing when their second-camera pixels are within the
explicit distance threshold. Keep all supports, including duplicate sheets and
both roots of a quadratic. `Matches::unique` is present only when exactly one
depth-accepted `(crossing, sheet root)` combination exists across all input traces
and no segment is near-coincident. `MatchIndex` refers to trace, crossing and
support indices, preserving the original caller order. Failed triangulations
remain in `Crossing::triangulation` with their reason, even when supported.

Invalid geometry, failed sheet projection, invalid/gapped trace, cancellation or
budget exhaustion fails the whole query without partial selection. A projection
outside the image is retained; projection outside the calibrated lens domain
fails. This conservative behavior prevents an unexamined hypothesis from being
silently excluded when declaring uniqueness. It can reject otherwise usable
queries and is not an occlusion model.

Bounds per call: 1–128 traces and sheets, 2–4096 vertices per trace, 65536 total
vertices, positive support radius at most 32 pixels. The configurable work budget
counts trace vertices and crossing-to-projection comparisons and is at most ten
million. Cancellation is checked between bounded numerical operations. Work units
are not time or CPU-instruction measurements. Storage grows with retained
crossings and support combinations. The uncached API repeats inverse distortion
per query. [Prepared traces](PREPARED_LASER_TRACES.md) now provide a measured reuse
path; general throughput, latency and large-data memory qualification remain open.

The matcher does not parse factory records, detect traces, validate recordings,
infer reflection, prove projector identity or activate calibration. Uniqueness is
conditional on supplied geometry and hypotheses. Missing stripes, repeated use of
second-camera geometry across first pixels, wrong calibration and false image
features can still produce incorrect matches. A small epipolar reprojection
residual is imposed by the construction and is not independent accuracy evidence.

## Diagnostic interface and reproducibility

```sh
cargo build --frozen --release -p scancraft-calibration --example match_laser_traces
python3 tools/check_laser_trace.py
```

The example reads JSON from stdin with fields `calibration`, `limits`,
`first_pixel`, `right_traces`, `sheets` and `config`. The reference script contains
complete synthetic requests. The maximum accepted input is 1 MiB; one extra byte
may be read to detect overflow. Unknown fields are rejected. Output includes all
results and explicitly false calibration/physical-accuracy/unit/projector/recording
qualification flags. No project or point-cloud file is exported. Existing
`position_mm` fields require qualified input units before a physical millimeter
claim; private raw-unit numerical probes do not establish that qualification.

Six module tests cover known depth with competing traces, shared-vertex
deduplication, duplicate traces/sheets, two quadratic roots, near-coincident
segments, rejected depth, absent support, distorted/skewed/rotated geometry,
invalid models/bounds, cancellation and work exhaustion. Full workspace gate:
695 tests, formatting, strict Clippy and release build passed.

The standard-library Python reference independently projects known 3D points
under 36 scale/pose/distortion/position combinations. Every first ray has three
geometrically valid trace crossings; sheet support must choose the known point.
Maximum observed position error was 9.05e-9 synthetic length units and pixel error
1.24e-10. These test composition against known truth; they are not hardware claims.

## Saved Raptor experiment — provisional geometry

On three previously captured fixed-pose frame pairs, all 96/99/92 observations
inside a fixed target-image hull had three depth-accepted epipolar crossings.
Thus stereo geometry alone did not resolve line identity in this scene.

Adding the prior sheet hypothesis and the existing three-pixel support threshold
selected 93/96/89 points in that region. Previously seeded interpolation retained
79/81/74 points. No reference-plane residual selected or trimmed points. Residual
RMS against the existing target reference was 0.231/0.236/0.230 **raw units**;
camera geometry is shared with that reference and physical units are unverified.
Outside the target region, other surfaces invalidate blanket comparisons to its
plane. No physical accuracy conclusion follows from these residuals.

Native composed results exactly reproduced the preceding explicit composition's
selected observation/trace/sheet identities and candidate pixels: 473/480/473
matches over the whole images, including the smaller target-region subsets above.
A 200-pixel artificial second-trace translation, clipped and split at image
boundaries, retained zero supported matches over all 592 first observations.
This is one negative control, not a measured false-positive rate. The native
composition comparison shares numerical primitives, unlike the synthetic
known-point reference. All crossings, rejected results and private evidence were
retained locally. No private captures or factory values are distributed.

Next integrate the matcher with recording-backed detection and lineage, benchmark
reuse of prepared trace rays, and qualify identity/geometry across different
surfaces and poses when available. Camera/mode/units binding, synchronization and
physical calibration remain separate unresolved requirements. No installed
calibration, firmware, scanner setting or GUI scanning behavior changed here.
