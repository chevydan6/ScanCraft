# Pixel audit of retained inline-copy trials

**Confirmed developer result:** the previously reported 610-second combined trial
contains 19 accepted images with repeated full pixel blocks, despite zero startup
or ongoing buffer rejections. The original rejection counters, CRC results and
header checks remain correct within their scope. They do not establish intact
image content. The temporary candidate remains experimental and default-off.

This read-only follow-up starts at documentation revision
`9a404c4f376fdd1647fcb3d3c97a21ec7b52edfb`. Application code remains
`d91fc4f696ee275e77145406ea55126475b515c6`. Independent assignment 575 verified the
preceding [USB boundary evidence](RAPTOR_ACCEPTED_IMAGE_INTEGRITY.md); this new audit
requires separate review. SCAN-REV-034 remains High/Open; no correction is claimed.

## Scope and results

The predeclared audit examines every accepted image in ten original recordings:
43,800 frames / 100,917,599,010 stored bytes. Whole-file SHA-256 hashes match their
previous manifests. Independent streaming parsing checks header/frame CRCs,
lengths, error flags, sequence/host ordering, metadata PTS and final counts.
No source recording, coefficient, threshold or original assessment is changed.

The two diagnostics are unchanged from the preceding source audit: compare all
187 complete aligned 12,276-byte blocks, and find equal runs displaced by 24,552
bytes with length at least 128 and at least 16 distinct values. Neither is an
exhaustive optical integrity test or production rejection rule.

| Historical arm | Images | Full-block signature | Partial signature | Either signature |
| --- | ---: | ---: | ---: | ---: |
| A1, inline copy disabled | 1,200 | 3 | 6 | 8 |
| B1, inline copy enabled | 1,200 | 0 | 0 | 0 |
| A2, inline copy disabled | 1,200 | 1 | 0 | 1 |
| Enabled plus logger paused, one minute | 3,600 | 0 | 0 | 0 |
| Enabled plus logger paused, 610 seconds | 36,600 | 19 | 0 | 19 |
| Total | 43,800 | 23 | 6 | 28 |

One A1 image has both signatures. The historical methods and confounders remain
in [off/on/off trials](RAPTOR_INLINE_COPY_TRIAL.md),
[combined acquisition](RAPTOR_COMBINED_ACQUISITION.md) and
[sustained acquisition](RAPTOR_SUSTAINED_COMBINED.md). These are flood images of the
calibration board, not new rotating-box recordings. Scene, tracing, host conditions
and recorded IRQ-affinity differences prevent treating these counts as an isolated
performance comparison. In particular, B1 still has its previously reported nine
ongoing rejected buffers per camera. Zero tested signatures does not erase them.

## The sustained repetitions are not constant fills

The 19 sustained images are eight left-camera and eleven right-camera images.
Eight timestamps are affected in both cameras; three additional right-camera
images have the signature. Each repeated block contains 9 or 10 distinct values,
so the partial diagnostic's diversity requirement deliberately does not flag it.
The original full-block comparison does flag the exact 12,276-byte repetition.
This is another reason to retain both tests and avoid interpreting a negative
partial-signature result as an integrity pass.

All 19 repeated block contents have different SHA-256 values from one another.
For each affected image, the immediately preceding and following frames have
unequal blocks at the same two positions, and both differ from the incident block:
38 neighboring-frame comparisons. The values vary within each block; these are
not the constant-image control's repetitions. This contextual evidence supports
data duplication, rather than a naturally uniform area. It does not establish a
general false-positive/negative bound or a total corrupted-frame prevalence.

Native extraction of sustained left index 173 validates all 18,300 source frames
and the footer, then produces a raw snapshot and PNG exactly equal to the original
selected pixels. The image shows a small dark horizontal strip near the top.
Separately, all nine flagged A1/A2 images have byte-exact native exports. A1 left
index 111 visibly has a displaced strip crossing target circles. No image repair,
new exclusion or reinterpretation as valid metrology is applied.

## Observed USB prefixes with requests still outstanding

Sustained pair index 173, raw PTS **1683900145**, has repeated native blocks 6/8
in both cameras. Its original trace retains only 32 payload-prefix bytes:
12 UVC header bytes and 20 image bytes per transfer. The complete trace hash matches
its prior manifest. A targeted parse through this pair checks 130,915 events;
it is not a new complete-event audit of the 13,761,774-event capture.

Both groups have 188 payloads, 2,304,000 declared image bytes, one FID and one
terminal EOF. All 188 observed image prefixes per camera equal the corresponding
native pixels. Payloads 6 and 8 have distinct request identities but identical
20-byte image prefixes. Their completion separation is 132 µs left / 148 µs right.
The unseen remainder of each USB body cannot be reconstructed from this trace.

**Confirmed software accounting:** four requests remain outstanding after every
completion in the affected frame, on both endpoints. No zero-outstanding interval
overlaps either frame. Maximum within-frame completion gaps are 170 / 178 µs.
Software request counts are not physical controller-ring occupancy. Nevertheless,
this incident does not have the empty-request-queue pattern of the preceding
baseline partial-strip incident. An empty software queue is therefore not an
established explanation for every repeated strip. Device, link, controller,
HCD/DMA and other timing behavior remain unresolved.

The result changes the next investigation: do not promote the existing candidate
or repeat the same experiment expecting complete pixel integrity. Compare full
payload evidence for the enabled path and investigate fault boundaries; retain
both startup/truncation and image-content failures. Neither an arbitrary timing
threshold nor heuristic image repair is justified by these observations.

## Reproduction and validation limits

Private evidence `candidate-pixel-audit-20261104` contains the frozen input plan,
original diagnostic source, controls, all per-frame outcomes, native exports,
neighbor comparisons and targeted original-prefix analysis. The external
`candidate-pixel-publication-20261104/evidence-manifest.json` seals 89 files /
59,261,429 bytes; manifest SHA-256:
`3e6dbbd286ddad584d1504a2f850c87c91ebff97c78c9e0c1b5e0a9d86f63616`.
Original recordings remain in their original sealed roots, outside public Git.

`run_fast.py` preserves frame order with four workers and at most eight outstanding
frame computations. A vectorized length prefilter skips Python iteration over
runs shorter than 128; equality, diversity, thresholds and reported results are
unchanged. Twelve real/synthetic comparison cases pass, and all 2,400 retained
rows from the original slower execution equal the faster output exactly. The
slower process was deliberately stopped with SIGTERM; its exit 143 and partial
outputs remain. This is private analysis tooling, not a ScanCraft optimization.

The completed audit exits 0. Ten original whole-file hashes match. All 43,800
row counts/signatures reconcile. A private output field was mislabeled
`host_timestamp_ns`; the recording contract stores microseconds. The corrected
`frames-corrected.ndjson` uses `host_timestamp_us` with identical values and
signature results, and retains the original mislabeled output. The initial native
export helper stopped on an assertion because it omitted its intended completed-
arm filter; corrected filtering succeeds, with the initial script/log retained.

`native_check.py` verifies the nine A1/A2 exports. `characterize.py` verifies all
28 flagged images against original frame CRCs, neighboring blocks and the extra
sustained native export. `check_sustained_prefix.py` checks the retained original
trace for the specific pair. These checks do not qualify all possible image
anomalies, live hash-monitor operation, physical exposure synchronization, scale,
reconstruction accuracy, a complete object or end-to-end scanning.

Publication gate: `tools/check.sh` passes formatting, strict all-target Clippy,
935 tests / zero failures / one ignored, and optimized workspace build. Application
code is unchanged; this is software regression evidence, not fresh hardware proof.
CodeRabbit quota lookup in the repository returns `lookup_failed`; no substantive
or paid review was performed. An initial invocation outside the repository also
failed and is retained. Public source/diff privacy checks precede publication.
