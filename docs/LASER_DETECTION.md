# Experimental stripe observations

`scancraft_image::laser::detect_rows` scans a bounded packed raw GREY image
without altering it. Each connected horizontal run strictly above an explicit
background threshold is one candidate. Its center is the intensity-weighted
centroid after subtracting that background; variance, peak, integrated signal
and integer support bounds accompany the subpixel image coordinate.

Parameters explicitly specify background, minimum peak, maximum run width,
saturation threshold and maximum observation count (at most 100,000).
Border-touching runs, excessive widths, clipped peaks and weak peaks are
rejected in that order and counted separately. Output is ordered by row then
column. Cancellation or output overflow returns an error, never a silently
truncated detection set. Runtime is linear in image pixels plus accepted run
support; allocation is limited by the observation cap.

This is an isolated-stripe observation primitive, not a Raptor scanning mode.
It does not identify projected lines, subtract ambient images, establish
exposure synchronization, estimate background automatically, join rows,
separate merged peaks, or reject textured scene reflectance. Horizontal scan
rows suit roughly vertical stripes; other orientations require a different
sampling strategy. Thresholding truncates stripe tails and can bias centroids;
no metrological subpixel accuracy is claimed. A wide or clipped laser signal
must be diagnosed rather than silently accepted. The caller must supply raw
linear-intensity data appropriate to the method; preview contrast stretching
must never feed this detector.

Three Rust regression tests cover independently calculated asymmetric centroid
and second moment, multiple stripes, exact source preservation, every rejection
class, invalid settings, output overflow and cancellation. All 162 workspace
tests and warning-denying Clippy passed. Test log:
`evidence/laser-detector-tests.log`. These are synthetic algorithm checks;
there is no actual Raptor blue-laser detection or physical accuracy evidence.

Next: expose observation diagnostics against CRC-verified recordings, validate
controlled genuine laser images, establish mode-specific line identities and
calibration binding, then feed validated observations into triangulation.

## Recorded-frame diagnostic

```
scancraft-cli detect-stripes RECORDING INDEX BACKGROUND MIN_PEAK MAX_WIDTH SATURATION MAX_OBSERVATIONS
```

All parameters are explicit integers. INDEX is zero based. Example for a raw
GREY recording: `scancraft-cli detect-stripes capture.raptor 20 30 50 12 255 100000`.
JSON goes to standard output; the command changes no scanner controls or input
files. Input must be a regular file. Every frame and footer is validated before
the selected frame is analyzed. JSON retains capture context, profile, PTS,
sequence, selected and total frame counts, payload/metadata CRCs, parameters,
observations and rejection counts. Header protection is stated separately for
legacy recordings. Raw pixel coordinates have no projector-line identity.

Actual execution on the existing 21-frame Raptor IR recording, selecting frame
20 (1920×1200), returned zero accepted runs with these example parameters:
209 border, 703 width and 3,743 weak-peak rejections. This recording has no
verified laser projection, so this is a real-input diagnostic check, not a
laser-detection success or failure qualification. Full private report is retained
alongside the private capture; aggregate evidence is in
`evidence/stripe-recording-summary.json`.

An additional CLI regression verifies analytic output and lineage, out-of-range
selection, and late corruption/truncation rejection even when frame zero was
requested. All 163 workspace tests, warning-denying Clippy, release CLI build
and formatting pass (`evidence/stripe-recording-tests.log`).
