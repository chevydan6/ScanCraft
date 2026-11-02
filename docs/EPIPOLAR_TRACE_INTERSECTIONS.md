# Epipolar intersections with image traces

`scancraft_calibration::epipolar_trace::PreparedTrace` intersects a supplied
right-camera polyline with the epipolar constraint of one left-camera pixel.
It supports the existing Brown camera model, nonzero camera skew and an explicit
right-from-left rigid pose. It does not find projector-line identities, choose
between competing traces, reconstruct a point cloud or activate calibration.

This is original Rust code using the existing camera implementation and
`epipolar::raw_field_matrices`; it adds no dependency or copied upstream code.
The mathematical method is line/segment intersection in undistorted normalized
image coordinates. The essential matrix is `E = [t]x R`, with translation
normalized because its magnitude does not affect the constraint. For a unit
left ray `a`, the second-image line is `E a`. Right vertices are independently
undistorted and expressed as `(ray.x/ray.z, ray.y/ray.z, 1)`.

## Explicit interpolation and ambiguity

Segments are straight **in normalized undistorted coordinates**. With distortion,
this differs from a straight segment in the recorded image. Interpolated hits
are projected back through the right camera model. The output records the
segment index, normalized-segment fraction, resulting pixel and normalized
epipolar distance. A vertex index distinguishes a tolerance-qualified original
endpoint from an interpolated point; interpolated coordinates are not additional
measured sensor samples.

The numerical distance tolerance is used to classify endpoints. Two nearby
endpoints produce an ambiguous segment, not an arbitrary midpoint. A lone
near-line endpoint is reported with its actual residual. Otherwise same-sign
distances yield no hit, and opposite signs yield a fraction in [0,1]. The signed
distances are scaled before division to avoid overflow. No extrapolation occurs.
Multiple crossings remain separate; only duplicate references to the same
shared vertex are collapsed. Ambiguous segments remain present even if another
segment supplies a hit. A caller must consider these flags and other candidate
traces before claiming a unique correspondence.

Preparation validates both cameras and SO(3)/baseline conventions. Each query
accepts 2 through 4,096 vertices and an explicit maximum raw endpoint separation
of at most 4,096 pixels. Adjacent duplicates, longer gaps, unsupported camera
domains, numerical failures and cancellation fail the whole query without
partial results. Numerical tolerance must be positive and at most 0.001 in
normalized coordinates. An explicit minimum epipolar-plane sine rejects rays
near the baseline direction. These numerical bounds are not physical accuracy
specifications. Work and storage are O(vertices), with bounded existing camera
inverse-distortion iterations; cancellation is checked per vertex/segment.

## Offline example

```text
cargo build --release -p scancraft-calibration --example intersect_epipolar_trace
target/release/examples/intersect_epipolar_trace < request.json
```

The JSON request contains `calibration` (`StereoCalibration`), `first_pixel`,
`right_trace` (ordered pixel vertices) and `config` with
`maximum_segment_length_px`, `normalized_tolerance`, and `minimum_plane_sine`.
The maximum accepted stdin input is 1 MiB; one additional byte may be read to
detect overflow. Unknown request fields are rejected. The example reports every
intersection and ambiguous segment, and explicitly leaves calibration activation,
physical accuracy, units, projector identity and recording integrity unverified.
It does not read a scanner or infer provenance from supplied JSON.

## Evidence and limits

Unit tests recover analytical fractions, distorted/skewed projections and known
stereo depth with nontrivial camera rotation/translation. They also cover missing
and repeated crossings, shared vertices, coincident/near-coincident segments,
reversed trace order, baseline-scale invariance, excessive vertex count, gaps,
invalid inputs, cancellation and epipole degeneracy. The optional
`tools/check_epipolar_trace.py` generates known 3D points and independently
projects them with NumPy/Brown equations. Across 32 synthetic cases, maximum
pixel discrepancy was 5.25e-10 and fraction discrepancy was 3.41e-11. This
validates numerical behavior on those cases, not scanner calibration.

A private saved-frame probe uses adjacent segments of the continuity trace
containing each earlier provisional stereo partner. It requires one candidate
within the previous three-pixel seed neighborhood and retains all failure
reports. Middle/first/last frames yield 79/80, 81/83 and 74/75 usable candidates;
all pass the unchanged stereo-triangulation guards. Against the unchanged
target reference plane, RMS values are 0.2313/0.2332/0.2256 in unqualified raw
units, compared with about 0.261/0.264/0.260 for integer-row partners.

The observed tiny reprojection residual is imposed by the epipolar construction
and is **not independent accuracy evidence**. Original seeds still depend on
provisional factory-sheet hypotheses; the reference plane shares camera geometry,
and surface assumptions, units, timing and same-pose generalization remain
unqualified. No residual-based filtering, guard relaxation, calibration fitting
or installation was performed. Raw captures and model fields remain local. A
complete scanner workflow still needs validated correspondence, provenance,
mode/units binding, tracking/fusion and physical measurements.
