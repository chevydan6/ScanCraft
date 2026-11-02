# Within-image stripe continuity

`scancraft_image::laser_continuity::trace_rows` groups row detections into
conservative image traces. It consumes immutable `LineObservation` positions and
returns their original indices. It does not assign a projector line, track motion
between frames, change coordinates, discard detections or reconstruct depth.

```text
scancraft-cli trace-stripes RECORDING INDEX BACKGROUND MIN_PEAK MAX_WIDTH \
  SATURATION MAX_OBSERVATIONS MAX_ROW_GAP MAX_X_STEP MAX_SLOPE MIN_SPAN MIN_POINTS \
  [--opening-radius RADIUS]
```

The existing `detect-stripes` command and its output remain unchanged. The new
command adds a `continuity` object after the same whole-recording CRC/footer
verification and detector. All raw observations, detection rejections, source
CRCs, timestamps and false physical-qualification flags remain present. Every
observation belongs to exactly one trace, including unsupported singletons.

## Method and bounds

For each occupied row, consider endpoints whose last observed row is at most
`MAX_ROW_GAP` rows earlier. An endpoint and current observation are candidates
when `abs(dx) <= MAX_X_STEP + MAX_SLOPE * dy`. Extend only mutually unique pairs:
exactly one candidate for the observation and exactly one for its endpoint.
At forks or joins, terminate contested old endpoints and begin new traces at
the unmatched observations. Endpoints with no candidates may survive gaps up to
the requested limit. No nearest-distance tie-break chooses between ambiguous
candidates. Span and count checks label the completed traces without deleting
their members. `MIN_SPAN` is last row minus first row; it is not the number of
samples. The displacement bound is inclusive.

Input positions must be finite, inside the image, at integer rows and strictly
ordered by row then x. Only positions are consumed; other observation fields
remain the detector's responsibility. The existing image cap is 4,194,304 pixels;
at most 100,000 observations are accepted. The library requires an explicit
comparison budget of 1 through 100 million; the CLI uses 20 million. Budget
exhaustion and cancellation return errors without partial traces. Storage is
O(observations), and work is O(observations + comparisons). These are complexity
bounds, not measured latency or real-time guarantees.

The implementation is original Rust code, developed from an image-only Python
experiment in this project. It adds no production dependency and copies no
upstream algorithm implementation. The optional NumPy adjacency-matrix reference
in `tools/check_stripe_continuity.py` checks partitioning, ambiguity counts,
comparison counts and support flags; it is not physical ground truth.

## Intended use and limitations

This diagnostic addresses marker-edge/noise observations that can accidentally
support a wrong factory sheet hypothesis. Long near-vertical traces can provide
additional evidence, but texture can also form continuous traces. Nearby lines,
crossings, clipping and occlusion can fragment real stripes. Horizontal or
rapidly bending lines may violate the chosen displacement bound. Gap bridging
does not prove physical continuity. Parameters are explicit because no default
has been qualified across Raptor modes, surfaces, poses or exposure settings.

The initial private stationary-board experiment fixed gap 12, horizontal step 2,
slope 0.25, minimum span 100 and minimum count 20 before evaluating its geometric
residuals. On the development frame, continuity retained 1,423/1,988 left and
592/1,286 right observations, and none of 97/92 off-image observations. Applying
this image-only condition before the existing provisional sheet/stereo support
check reduced interior greedy pairs from 284 to 80. The corresponding raw-unit
reference-plane RMS changed from 23.30 to 0.667, with maximum absolute residual
changing from 234.28 to 1.357. All retained residuals were reported. This is an
exploratory coverage/error tradeoff, not a claimed accuracy improvement in mm.
The plane shares factory camera geometry; physical units, exposure timing and
off/on stationarity remain unqualified. No calibration was installed.

The earlier nearest-sheet-to-reference-plane experiment is not used for this
selection: it would make line-identity evaluation circular. Here continuity and
opposite-camera image support determine selection before the reference plane is
examined. Shifted image controls and unused frames remain necessary checks, and
neither establishes a true-positive rate without independent labels.

Sources: `crates/scancraft-image/src/laser_continuity.rs`,
`crates/scancraft-cli/src/laser_diagnostics.rs`, and the `trace-stripes` command
branch in `crates/scancraft-cli/src/main.rs`. Unit tests exercise bounded gaps,
slopes, forks/joins, inclusive boundaries, malformed positions, cancellation,
budgets, input preservation and whole-recording integrity. The optional reference
tool uses synthetic recordings and can compare saved native reports.

Validation at this development milestone: 680 workspace tests, formatting,
strict Clippy and release build passed. The adjacency-matrix reference matched
32 synthetic recordings (7,620 detections) and all 12 checked real images. Native
middle-frame indices also matched the earlier Python experiment exactly.

With the same parameters, first/last equal-PTS pairs withheld from the initial
continuity experiment produced these results against the unchanged middle-off
reference plane:

| Frame pair | Baseline pairs | Continuity pairs | Baseline plane RMS, raw | Continuity plane RMS, raw | Continuity max absolute residual, raw |
| --- | ---: | ---: | ---: | ---: | ---: |
| First | 292 | 83 | 22.392 | 0.671 | 1.397 |
| Middle (development) | 284 | 80 | 23.304 | 0.667 | 1.357 |
| Last | 284 | 75 | 19.963 | 0.667 | 1.410 |

No off-image observations passed the continuity criteria in these checked frames.
These are unused frames from the same recordings and pose, not independent
surface/pose generalization or physical metrology. The substantial loss of
coverage and remaining residuals preclude treating this as a qualified scanning
filter. Private inputs, native reports, probe sources and binary hashes are
retained locally; no raw scans or factory coefficients are included here.
