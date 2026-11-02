# Native bright-component observations

`scancraft_image::components::detect` measures eight-connected regions above an
explicit intensity threshold in packed GREY images. This supplies candidate
regions for reflective-marker and calibration-target work. It does not identify
markers, fit circles, index a board, match cameras, estimate calibration, or
activate a camera convention. A binary area centroid is not necessarily the
projection of a physical circle's center, particularly under perspective,
distortion, clipping or nonuniform illumination.

```sh
scancraft-cli detect-components recording.raptor 5 100 201 10000 10000 > observations.json
```

Arguments are recording, zero-based frame index, exclusive intensity threshold,
inclusive minimum/maximum area in pixels, and maximum examined components.
All components count toward the resource limit, including rejected noise.
Exceeding a limit returns an error instead of truncated observations. Area
rejection precedes image-border rejection; a clipped component is never returned
as a usable centroid. Saturated samples are counted and reported, not corrected.
Bounds are inclusive pixel-center indices. Output order follows each component's
first foreground pixel in row-major order and conveys no marker identity.

The CLI uses `scancraft_recording::selection::read_selected`, which validates all
frames and the footer, including frames after the selected index. The report
retains profile, capture context, timestamps, selected index, total frame count,
payload/metadata CRCs, parameters and rejection counts. Legacy recordings remain
readable and explicitly report their lack of header CRC protection. Reports may
contain private device metadata; keep real captures and their reports local.

The original Rust implementation uses iterative eight-neighbor flood fill. No
OpenCV source is translated or copied, and no dependency is added. Each foreground
pixel enters the traversal once. Time is O(image pixels); scratch storage is one
byte plus one u32 slot per pixel, at most 20 MiB at the existing 4,194,304-pixel
image bound, plus output for at most 100,000 examined components. Cancellation
is checked before allocation and at most every 1,024 scan/fill steps. The existing
`WorkControl` interface and crate boundaries are preserved. No CPU/GPU performance
or real-time throughput claim is made.

Tests in `components.rs` exercise diagonal connectivity, threshold equality,
exact binary centroids, saturation counts, immutable input, area/border rejection,
resource-limit exhaustion by rejected noise, invalid parameters, cancellation,
singleton dimensions and dense/empty frames. CLI tests verify raw observation
provenance and rejection of later corruption/missing footers despite selecting
an earlier frame.

`tools/compare_components_opencv.py` is an optional offline reference check using
OpenCV's `connectedComponentsWithStats`. It requires identical PNG/raw payload
bytes, checks accepted observations and rejection accounting, and compares
centroids, areas, bounds, peaks and saturation counts. It does not independently
replay the source recording or prove physical accuracy:

```sh
python tools/compare_components_opencv.py extracted/image.png observations.json
python tools/check_components_reference.py target/release/scancraft-cli
```

OpenCV/NumPy are reference-environment tools, not production dependencies.
The separate board QR, camera-reflection hypotheses and physical calibration
remain outside this command. Private board coordinates are not embedded in code
or test fixtures.

Development validation on 2026-11-02: the complete Rust gate passed 659 tests,
formatting, strict all-target Clippy and release compilation. The seeded reference
check passed 60 synthetic images (including singleton dimensions and sparse/dense
patterns) with zero centroid difference from OpenCV 5.0.0. Its mismatched-payload
CRC negative control was rejected. Eight saved real Raptor frames from four scenes
also matched the reference exactly for component counts, accepted statistics and
rejections. These included the board, caliper/board scenes and laser-on imagery;
the latter yielded zero regions passing the chosen area limits. This establishes
agreement of the specified segmentation operations, not target identification,
dimensional accuracy, calibration or scanning throughput. Raw hardware evidence
and board-specific coordinates remain private.
