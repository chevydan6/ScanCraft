# Local background removal for stripe observations

`scancraft_image::laser::detect_rows_with_opening` provides optional row-wise
white top-hat processing: raw samples minus a morphological opening (minimum
filter followed by maximum filter, using the same symmetric window). This
removes broad background variation before the existing threshold-run centroid
detector. It does not identify projector lines, validate camera mapping, remove
all texture/noise, or produce calibrated depth.

```
scancraft-cli detect-stripes RECORDING INDEX BACKGROUND MIN_PEAK MAX_WIDTH \
  SATURATION MAX_OBSERVATIONS --opening-radius RADIUS
```

With this option, BACKGROUND, MIN_PEAK, peak and centroid weights refer to
8-bit contrast above the local background. SATURATION still refers to the
original raw samples inside a candidate run. Any candidate run containing a clipped source sample is rejected even when
its contrast falls below SATURATION. The report explicitly names these units and
records the radius; source payload/metadata CRCs always refer to original
recorded bytes. Without the option, raw detection and its JSON fields retain
their existing behavior. Whole-file integrity validation precedes detection.

Radius must be 1..128 and the 2*radius+1 window must fit the image width.
Windows truncate at image boundaries; edge-connected candidates remain
rejected. Two monotone queues implement erosion/dilation in O(pixels),
independent of radius. Buffers comprise one bounded contrast image, two reused
row buffers and a bounded window queue. Raw input is borrowed and never
modified. Cancellation, dimension/output limits and explicit rejection counts
are retained; no partially truncated observation list is returned.

Tests compare window extrema with independent brute-force windows at every
boundary on many synthetic rows. Analytical cases cover stripes over a strong
pedestal, varying background, subpixel centers, original-source clipping,
unchanged source bytes, invalid windows, cancellation and observation overflow.
A recording-level test checks raw CRC lineage, contrast labels and rejection
of later file truncation. These are image-processing checks, not physical
scanner accuracy qualification.

## Genuine saved on/off comparison

`tools/audit_laser_opening.py CLI ON_DIRECTORY OFF_DIRECTORY NEW_PRIVATE_DIRECTORY`
runs six fixed profiles on frame 30 of each saved 60-frame dual recording.
It reruns complete native recording validation for every detector invocation,
checks equal saved PTS within each pair, stores observations only under
`evidence/private`, and prints aggregate counts. Equal PTS does not establish
exposure synchronization; separate on/off captures do not establish that the
scene was unchanged between them.

| Profile | Background / min peak / max width / radius | On left/right | Off left/right |
| --- | --- | --- | --- |
| Raw baseline | 30 / 50 / 12 / none | 294 / 606 | 0 / 0 |
| Raw wider | 30 / 50 / 48 / none | 331 / 736 | 0 / 0 |
| Low global | 15 / 30 / 48 / none | 0 / 0 | 0 / 0 |
| Faint global | 8 / 16 / 48 / none | 0 / 0 | 0 / 0 |
| Local strict | 3 / 10 / 48 / 32 | 2764 / 3763 | 130 / 0 |
| Local faint | 1 / 5 / 48 / 32 | 3889 / 5192 | 275 / 106 |

All profiles use raw saturation 255 and an observation cap of 100,000.
Lower global cutoffs connect background into broad/edge-touching runs. Local
processing recovers substantially more observations, but off-capture detections
and texture/noise remain. This experiment does not establish a true-positive
rate or prove that the extra detections are laser centers. No default detector
or hardware setting was changed. Aggregate results and rejection counts:
`evidence/laser-opening-on-off-audit.json`.

## Mapping hypothesis remains unvalidated

Re-running the existing factory sheet hypothesis with local-strict observations
does not establish a reliable mapping. For region 202000, original interface
order yields 228 greedy distinct pairs, while +200/+400/+600 shifted controls
yield 239/99/129. Reversed order yields 199 pairs versus 332/98/132 controls.
Region 200000 shows the same limitation (224 vs 251/72/129, and reversed
195 vs 339/104/132). The duplicated CROSS record is not an independent dataset.
Reports: `evidence/laser-opening-sheet-hypothesis-200000.json` and `...-202000.json`.
Greater detection density increases accidental support. Sparse baseline
support must not be interpreted as calibration validation. Camera assignment,
model/mode binding, line identity and physical units remain open; no mapping is
selected and no physical geometry is exported.

The offline example now indexes observations by image row and checks exactly
the rows that can intersect the three-pixel search disk. Exact distances and
tie ordering remain unchanged. An independent test compares against brute
force on random fractional coordinates, borders and ties. On the original
region-202000 baseline observations, every result/metadata field except the
comparison count is identical: 6,281,160 distance comparisons become 61,656.
This is a comparison-count reduction, not a measured timing speedup or live
throughput qualification. `evidence/laser-opening-index-equivalence.json`.
The hard 100-million-distance-comparison cap still applies; no radius or
acceptance threshold was relaxed to make the larger audit run.
