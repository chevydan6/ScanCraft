# Planar target association

`scancraft_calibration::target::associate` associates a supplied planar pattern
with image observations. Each input point has a two-dimensional `position` and
an `anchor` class. Exactly four points in each input must be anchors. Pattern
coordinates can be dimensionless; the operation does not infer millimeters,
decode a QR payload, estimate camera calibration or modify a scanner.

The implementation enumerates all 24 anchor permutations. Each yields a
normalized DLT homography using the existing `planar::homography` solver. A
pattern point matches only when exactly one same-class observation lies within
the supplied seed radius. Candidates with ambiguous neighborhoods, shared
observations, missing anchors or a projective horizon crossing the pattern are
discarded. The winning candidate must meet the minimum support and exceed the
runner-up by the explicit support margin. Symmetric patterns therefore fail
instead of receiving an arbitrary orientation.

One DLT fit uses all winning matches. Matching then repeats at the smaller
refinement radius. Every seed association must survive unchanged; additional
associations are allowed. The result retains target/observation array indices,
refined residuals, initial winning/runner-up support and a row-major homography.
Residuals measure agreement with the fitted image model, not physical accuracy.
The association search is original Rust code. The reused DLT solver's provenance
and mathematical method are documented in
[PLANAR_CALIBRATION_INITIALIZER.md](PLANAR_CALIBRATION_INITIALIZER.md).
No upstream implementation was copied or new dependency introduced.

Bounds are 8–128 pattern points and 8–256 observations, finite coordinates
within +/-1e6, positive radii at most 1e5, and at least eight required matches.
The refinement radius cannot exceed the seed radius. A cancellation callback
is checked before work, per hypothesis/pattern point and around the bounded
SVD calls. Individual SVD calls cannot be interrupted. Invalid, ambiguous and
cancelled operations return errors without partial results.

An offline JSON example provides a reproducible entry point:

```sh
cargo build --frozen --release -p scancraft-calibration --example associate_target
target/release/examples/associate_target < target-observations.json
```

Input contains `target` and `observed` arrays of
`{"position":[x,y],"anchor":false}` objects, plus `config` with
`seed_radius_px`, `refined_radius_px`, `minimum_matches` and
`minimum_support_margin`. The example accepts at most 1 MiB from stdin, reading
one additional byte to detect oversized input, and rejects unknown fields. It does not authenticate observations against a raw
recording; its output explicitly retains that limitation. Capture data and
device-specific target information must remain local. A caller integrating
this primitive must preserve the source-recording provenance separately.

The seven unit tests cover known projective transforms, reflection, shuffled
input, partial visibility, unrelated clutter, symmetric ambiguity, conflicting
neighbors, noise/refinement failure, duplicate/nonfinite/degenerate inputs,
cancellation, many-to-one matching and projective horizons. On six saved Raptor
images from three board poses, the native result agrees exactly with every
previously indexed comparison observation: 37, 26 and 25 small marks per camera
respectively. Total associations including anchors are 41/42, 37/37 and 34/35.
Those comparisons use image row/pattern indexing independent of factory stereo
geometry. No camera intrinsics or epipolar residuals enter this search.

For that development experiment, a nominal pattern and area-based anchor classes
were supplied explicitly, with seed/refinement radii 10/3 pixels, minimum support
20 and margin 3. These parameters were selected after exploratory tests on the
same images; this is development evidence, not a held-out generalization test.
Maximum fitted residuals ranged from 1.31 to 1.88 pixels. Additional associations
beyond the earlier comparison subset are not independently verified by that
subset. Raw frames, pattern details and results remain private.

Limitations: all four correctly classified anchors must be visible; anchor
geometry must be nondegenerate; a projective model only approximates distorted
images and perspective-biased binary centroids. Large distortion, ambiguous
clutter and substantial occlusion may correctly produce no result. A support
margin is a rejection heuristic, not proof of target identity. The caller must
choose suitable radii and validate the physical target independently. Automatic
board/QR import, robust circle-center estimation, a GUI workflow, qualified
stereo reconstruction, dimensional accuracy and timing/throughput benchmarks
remain separate work.

The complete development Rust gate passed on 2026-11-02: formatting, strict
all-target Clippy, 666 tests and release workspace compilation. These checks
do not qualify real scanning, accuracy or sustained hardware operation.
