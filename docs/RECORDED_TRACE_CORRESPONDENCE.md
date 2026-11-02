# Recording-backed seeded trace correspondence

Experimental offline diagnostics, implemented in
`scancraft-workflow/src/recorded_traces.rs` and exposed through the CLI:

```text
scancraft-cli associate-recorded-traces GEOMETRY_JSON FIRST_RECORDING FIRST_INDEX SECOND_RECORDING SECOND_INDEX REQUEST_JSON NEW_DIRECTORY
```

Indices are zero-based. Geometry is a serialized `StereoCalibration`, including
explicit camera order and right-from-left pose. This command does not install a
calibration, infer physical units, reconstruct depth or export a point cloud.
The existing pose field name `translation_mm` does not qualify supplied numbers
as millimeters. Geometry and initial correspondence seeds remain hypotheses.

## Request contract

The following schematic uses placeholder CRCs and observation indices; obtain
them from the actual selected frames and deterministic detection configuration.

```json
{
  "schema_version": 1,
  "expected_frames": [
    {"interface": 0, "payload_crc32": 0, "metadata_crc32": 0},
    {"interface": 2, "payload_crc32": 0, "metadata_crc32": 0}
  ],
  "reflect_x": [false, false],
  "maximum_host_delta_us": 1000,
  "detection": {
    "background": 30, "minimum_peak": 50, "maximum_width": 12,
    "saturation": 255, "maximum_observations": 5000, "opening_radius": 12
  },
  "continuity": {
    "maximum_row_gap": 12, "maximum_horizontal_step": 2.0,
    "maximum_slope": 0.25, "minimum_span_rows": 100,
    "minimum_observations": 20
  },
  "intersection": {
    "maximum_segment_length_px": 16.0,
    "normalized_tolerance": 1e-10, "minimum_plane_sine": 1e-8
  },
  "seed_radius_px": 3.0,
  "seeds": [{"first_observation": 0, "second_observation": 0}]
}
```

Geometry and request inputs are bounded to 1 MiB each. Request objects reject
unknown fields. There must be 1–4096 seeds, with distinct first and second
observation indices. Interface order is explicitly `[0,2]` or `[2,0]`; it does
not establish lens identity. Seed radius is positive and at most 32 pixels;
host delta is at most one second. `opening_radius: null` selects raw detection.
The continuity comparison budget is 20 million. See
[continuity](STRIPE_CONTINUITY.md) and
[epipolar intersections](EPIPOLAR_TRACE_INTERSECTIONS.md) for numerical bounds.

## Processing and evidence

`Playback` validates each entire recording, including later frames and footer,
then selects the requested frame. Expected interface, GREY dimensions, payload
CRC and metadata CRC must match. Raw PTS must be equal and host timestamps within
the requested window. This is a pairing check, not proof of synchronized exposure
or compatible clocks. Recording source identity is checked again before publishing.

Detections and continuity are recomputed from the original pixels. Both seed
observations must have continuity support. Adjacent vertices around the second
seed are transformed by the explicit reflection hypothesis and intersected with
the first observation's epipolar line. Interpolation occurs in undistorted
normalized coordinates. Original vertex indices, segment fractions, all roots,
ambiguous segments and per-seed numerical failures are retained. Some invalid
intersection configurations therefore yield rejected seed diagnostics rather than
a whole-request error. Cancellation aborts the operation.

A candidate requires exactly one intersection within the seed radius and no
ambiguous segments. One-to-one supplied observation indices do not prove global
uniqueness of interpolated matches, projector-line identity or physical accuracy.
No target-plane residual or reconstructed depth selects candidates. The caller
supplies seeds; automatic reliable seed discovery remains outstanding.

The new directory contains exact `geometry.json` and `request.json` input bytes,
single-frame `first-frame.raptor` and `second-frame.raptor` recordings preserving
frame/profile/context, `correspondences.json`, and `manifest.json` with artifact
lengths and CRCs. Selected recordings are re-encoded containers, not byte copies
of the full source files. Reports include source lineage, all observations and
rejection counters, coordinate conventions, signal units and false qualification
flags. CRCs detect accidental corruption, not malicious substitution.

Publication reuses staged, synchronized, no-overwrite workflow output. A failed
staged write retains the partial directory for diagnosis; failure to synchronize
the parent after publication is reported explicitly. Filesystem calls have no
hard wall-clock deadline. Keep bundles private when they contain device context
or private captures.

## Validation and limitations

Five module tests cover preservation and checksums, known subpixel intersection
and lineage, overwrite refusal, mismatched CRC/PTS/index, duplicate seeds, corrupt
later footer, cancellation, unsupported/ambiguous traces and explicit reflection.
The complete workspace gate passed 689 tests, formatting, strict Clippy and release
build. Earlier geometry references separately test analytical synthetic inputs.

On three previously captured private Raptor pairs, the adapter reproduced all
prior native-example candidate coordinates exactly: 79/80, 81/83 and 74/75 seeds
retained. All artifact lengths/CRCs and exact geometry/request bytes were checked.
These use the same pose, provisional geometry and externally supplied seeds;
they establish reproducibility, not independent metrology. Private input data is
not distributed. No new capture, firmware change or physical calibration was
performed. This original Rust adapter adds no dependencies or copied algorithms.
