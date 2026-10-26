# IR viewing in the native workbench

Open ScanCraft, select Cameras, then Left IR or Right IR and start preview.
The panel displays raw p01, median, p99, mean and zero/255 clipping percentages.
These are whole-frame intensity summaries, not focus or valid-depth scores.

The optional **Stretch IR display contrast** maps the raw p01..p99 range to
0..255 for the displayed texture. It clips the display tails and leaves flat
percentile ranges unchanged. It updates a paused image too. The source data,
raw recording and saved snapshot retain original levels; no device exposure,
gain, illumination or calibration setting changes. Switching cameras clears
stale imagery and statistics.

The shared Rust histogram uses exact 256-bin counts and nearest-rank
percentiles. `GrayImage` enforces the existing nonempty bounded dimensions.
This implementation does not claim optical validity, depth accuracy, exposure
normalization or a qualified preview frame rate.

`scancraft-app --smoke-test-ir` explicitly starts left IR hardware acquisition
and raw recording with display stretch enabled. Set `SCANCRAFT_SMOKE_OUTPUT`
to a new private output folder. The smoke path stops after at least 20 accepted
frames or its existing timeout; an in-flight frame can produce a larger total.
It keeps its test window for at least five seconds for screenshot inspection,
then saves the raw-level snapshot and closes. It does not alter existing GUI
instances. Keep sensor images/screenshots private.

The recorded private-display test captured 21 frames and rendered the actual
IR image, checkbox and statistics. The saved snapshot is pixel-for-pixel equal
to recording frame 20, verified after whole-recording CRC/footer replay. The
screenshot was inspected. Summary: `evidence/ir-preview-render-summary.json`.
Startup rejects and sequence gaps persist; this test does not qualify
reliability, latency, sustained scanning, optical focus or genuine geometry.
