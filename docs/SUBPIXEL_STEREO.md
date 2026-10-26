# Optional subpixel stereo refinement

The offline native Rust matcher can refine a confidently accepted integer ZNCC
winner. This is an opt-in image-processing component, not calibrated Raptor
reconstruction, verified laser scanning, exposure synchronization or accuracy
qualification. The default remains the original integer matcher.

## Configuration and compatibility

Add this object inside `matching` in the existing offline settings JSON:

```json
"subpixel": {
  "min_curvature": 0.001,
  "max_bidirectional_error_px": 0.15
}
```

These are illustrative synthetic-test values, not qualified scanner settings.
Omit `subpixel` or use null to retain integer behavior. Curvature must be finite
in (0,4]; bidirectional error finite in [0,1] pixels. Refinement requires a search
span of at least three disparities, versus two for the integer method. Existing
patch-size, score, texture, uniqueness, masks, lens-domain and reverse-winner
checks remain prerequisites; no confidence gate is loosened by enabling refinement.

Old settings/results without this field deserialize as None and omit it again on
serialization. Existing project/recording magic and formats are unchanged. New
settings with this key require a supporting version; older strict config readers
reject them. Rust struct-literal callers add `subpixel: None` or an explicit option.
The native CLI and GUI's shared offline settings workflow pass the option through;
no live capture mode, physical calibration or new GUI checkbox is introduced.

## Independently implemented method

Fit the three ZNCC samples at target x-1, x, x+1 with a quadratic. Let them be
s_minus, s_center, s_plus. The positive concavity magnitude and estimated peak are:

```
curvature = 2*s_center - s_minus - s_plus
offset = (s_plus - s_minus) / (2*curvature)
```

This follows by solving a quadratic interpolant at -1,0,+1 and setting its
first derivative to zero. The code is independently derived project-local Rust,
not a translation/port of OpenCV, PCL or another source project. No upstream source
revision is claimed; MIT applies to this new project code. Existing ZNCC patch
normalization is reused. NumPy polynomial fitting is a separate numerical oracle,
not a production dependency or the origin of the implementation.

A strict local maximum, valid adjacent support, minimum curvature and finite
within-half-pixel offset are required. Both neighboring disparities must be within
the configured search interval; no extrapolation at a winner on its boundary.
The reverse integer winner must still return the original seed. Forward and
reverse local offsets must sum to within the configured pixel error. This reverse
check is a local peak-consistency heuristic, not continuous optical-flow proof.
Unsupported fits produce `subpixel_unavailable`; inconsistent offsets produce
`subpixel_reverse_mismatch`. Requested refinement does not silently fall back.

`MatchedFeature.right_pixel` retains the integer winner for diagnostics/lineage.
Its optional subpixel report contains the fractional right coordinate, offsets and
curvatures. Consumers must use `pixel_match()` for geometry; the offline processor
maps those fractional coordinates back through rectification into original cameras
before triangulating. Source seed/index lineage remains unchanged. Original
calibration and provenance checks still apply; timing/physical flags remain false.

Four extra candidate-patch reads per accepted pair are included conservatively in
the existing 100-million-sample preflight budget. The method adds CPU work; no live
performance, mature-library speed advantage or throughput qualification is claimed.

## Validation and reproduction

The synthetic scene uses independent continuous sine components, fractional
horizontal shifts, and an unclipped positive affine brightness change. It is not
a camera image or a factory calibration. Four signed cases have three accepted
features each. Independent float64 ZNCC and `numpy.polyfit` agree with the Rust
closed-form offsets/curvatures within approximately 7.4e-15 in this fixture.

| True disparity (px) | Maximum integer localization error (px) | Maximum refined localization error (px) |
| --- | ---: | ---: |
| 4.20 | 0.200 | 0.015451 |
| 4.25 | 0.250 | 0.014440 |
| 4.30 | 0.300 | 0.013114 |
| -4.25 | 0.250 | 0.012265 |

These controlled f32 fixtures do not bound error on real images. A separate
quantized 8-bit full-pipeline and CLI test requires at least fivefold reduction of
mean synthetic depth error versus the same integer configuration; it verifies saved
project coordinates, original-input preservation and no-overwrite behavior. Neither
result establishes physical millimeter accuracy or any release acceptance gate.

Other regressions cover analytic peak sign/curvature, flat/nonfinite/unusable fits,
rotated source sensors (including rectified x mapping to original y), independently
computed source-coordinate expectations, masked neighbors, boundary winners,
reverse inconsistency, retained coarse quality
gates, invalid settings, cancellation, old JSON and the additional work budget.

```sh
cargo test --frozen -p scancraft-depth
cargo test --frozen -p scancraft-cli --test subpixel_reconstruction
cargo run --frozen --release -p scancraft-depth --example subpixel_reference > /tmp/subpixel.json
python tools/check_subpixel_reference.py /tmp/subpixel.json /tmp/subpixel-report.json
```

Use the existing pinned test-only NumPy environment/requirements. The original
integer OpenCV correlation/triangulation comparison also passes with refinement
omitted. Source: matching.rs, matching/subpixel.rs, pipeline.rs,
tests/subpixel.rs, tests/support/subpixel_scene.rs and the CLI integration test.

Limitations include three-sample parabolic bias, sensitivity to image content and
quantization, poor/occluded/repeated texture, coarse uniqueness rejecting nearly
half-pixel ties, only horizontal right-coordinate refinement, and unqualified
physical calibration/mode binding. No comparison to dense BM/SGBM, learned matching
or vendor reconstruction is implied. Accurate physical calibration and genuine
Raptor end-to-end validation remain prerequisites for a real scanner release.
