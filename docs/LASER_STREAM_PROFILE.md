# Laser stream profiling and buffered pair journals

Status: experimental performance work; no live-rate or physical qualification.
Base: `9dea6ac3f97bbddc14a91beaa44f397dbdab50c5`.
The developer response and recovery checkpoint bind the implementation commit.

## Measurement before implementation

A private release-mode Rust probe loads five retained Raptor pairs from the prior
32-frame GUI trial (pair records 0, 4, 9, 14, 18). Each is processed 11 times; the
first iteration per pair is warm-up. The probe uses the production laser engine,
real WorkControl checks, exact CappedFile body and new pair_record body. Copied
bodies, source revision, probe source and executable hashes are retained privately.
Only imports/visibility are adapted for the private harness; no upstream code is
copied. Baseline and buffered paths execute in the same binary, alternating order.

Each journal timing serializes the same complete typed Report plus newline to a
new CappedFile and flushes it. Creation precedes the timer; final fsync is measured
separately. Outputs are read back and checked byte-for-byte and by length/CRC.
The report has 1.11–1.14 MB of JSON; the stream record's additional frame-reference
wrapper is excluded from this microbenchmark. The actual wrapper is covered by
separate native saved-record byte equality checks below.

Final ten-iteration medians (milliseconds):

| Retained pair | Candidates | Engine total | Before Sink::begin | After Sink::begin | Direct journal | Buffered journal | Median paired journal ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 1969 | 45.070 | 17.462 | 27.635 | 6.897 | 2.008 | 3.453 |
| 4 | 1948 | 45.093 | 17.336 | 27.575 | 6.875 | 1.894 | 3.622 |
| 9 | 1985 | 46.298 | 17.355 | 28.888 | 6.953 | 2.029 | 3.416 |
| 14 | 2027 | 44.963 | 17.344 | 27.606 | 7.076 | 2.011 | 3.510 |
| 18 | 1965 | 42.788 | 17.349 | 25.392 | 6.901 | 2.014 | 3.412 |

Before Sink::begin includes validation, both detections/continuity, trace
preparation and projected-root admission. After includes query matching,
selection and report construction. These are not isolated detector/matcher-only
timings. The probe's Sink only timestamps begin; comparisons confirm complete
reports equal the retained results outside timers.

Full preview preparation takes 6.70–6.77 ms median and preview JSON encoding
0.76–0.78 ms on these pairs. The preview code is byte-verified against the app's
Snapshot implementation and includes both full-frame histograms, bounded
thumbnails, copying positions and validation. The probe also separately measures
raw FrameView construction. These are warm saved-data component observations on
a shared host, not acquisition timing, preview FPS/p95, CPU utilization or an
application-wide speedup. File creation, raw recording, source ownership,
publication/readback and most finalization costs are excluded.

An earlier exploratory probe omitted a direct scancraft-image dependency, then
passed after correcting its private manifest. Its initial comparison lacked the
newline; the table above is the later exact-new-helper comparison. Both logs and
binaries are retained. No failed result was substituted into the final table.

## Implemented change and invariants

`laser_stream/bundle/io.rs::pair_record` wraps the existing capped/checksummed
writer in a fixed **64 KiB BufWriter**. Coalescing serializer fragments avoids a
checksum update for every tiny JSON token. It writes the complete newline-delimited
record and flushes before `bundle.rs::Plan::capture` offers any optional preview.
This flush makes accepted bytes visible to the file writer; it does not add a
per-pair fsync or claim power-loss durability. Existing final fsync/readback and
no-overwrite publication remain required.

On serialization, cap, write or flush error, `into_parts` explicitly discards the
unwritten buffer. Drop cannot retry those rejected bytes after the error return.
Already accepted file prefixes remain subject to the existing failure/staging
policy. A failed record may leave a shorter prefix than the prior tiny-write
path; it is still incomplete and is never published as successful. There is no
whole-session report buffer, new format, altered byte cap, numerical threshold,
frame-validity relaxation, new dependency or geometry-processing change.

## Regression and native evidence

Formatting, strict all-target Clippy, **774 Rust tests** and release build pass.
Three additional regressions cover multi-record byte/CRC identity across buffer
boundaries with Unicode/floating-point data; exact and one-byte-short budgets;
partial write/flush failures with no Drop retry; serializer failure discarding an
unwritten tail; and a complete parseable record visible before its preview.
Existing required-journal failure, raw finalization/restoration, optional-preview,
mutation, no-overwrite and process-boundary tests also pass.

A new native GUI trial using the same private Xvfb environment with
`LP_NUM_THREADS=2` saves **32 frames per camera**, processes **21 pairs**, retains
11/10 processing-queue drops and one unmatched second input, and produces
1869–2089 diagnostic candidates/pair. Startup rejected-buffer counts are 91/86,
ongoing rejections 0/0. The processed fraction is **65.625%, below 90%**. The prior
19/32 trial had different incoming frames and scheduling: the difference is not a
controlled application speedup measurement.

A separate actual root-close-during-acquisition trial saves **9/10 raw frames and
7 pairs**, with 2/2 processing drops and one unmatched second input. Startup rejects
are 101/1, ongoing 0/0. It publishes verified recordings and restores settings
before exit; one observed close-to-exit time is 0.281 s, not a hard deadline.
Both trials' raw replay/integrity and complete stream/illumination readbacks pass,
including original property 151 restored. Sampling every GUI task's child list
positively observes helper-owned camera descriptors while GUI descriptors remain
absent. Both helpers and GUI processes exit. Active/saved screenshots are inspected.
These short trials do not establish the full corruption/reliability/latency gate;
startup rejects are retained and are V4L2 buffer units, not physical exposures.

Separate retained-frame replay checks all **28 reports / 54,381 candidate values**,
frame indices, payload/metadata CRCs, sequence/flags/time/PTS, and reproduces every
complete pair-record byte using the unchanged base serializer definitions. Both
bundle manifests' artifact lengths/CRCs match. This proves same-engine recording
and serialization consistency, not calibrated physical geometry.

Private evidence and source/binary bindings live in the developer state directory;
no scenes, models, raw data, screenshots or probes are uploaded/committed. See the
recovery checkpoint for exact paths and hashes. CodeRabbit 0.9.2 included quota
and billing still return lookup_failed, so independent review/publication remain
pending. No paid service, firmware change, physical calibration, reviewer-file
modification, merge or protection bypass is part of this work.

## Next priorities

Measured engine time alone exceeds the 33.3 ms interval at 30 FPS. Journal buffering
therefore cannot meet the live target by itself. Profile detector/continuity and
admission more finely, and compare bounded parallel computation or a separate
complete-pair dispatch stage against full numerical results before choosing them.
Moving expensive optional thumbnail preparation off the processing path is also
worth measuring. Avoid hiding overload by increasing queue/history capacity or
silently sampling observations. Default software-renderer acquisition failures
remain unresolved. Physical geometry/units/coverage and real tracking/fusion remain
higher release priorities than more editing features; physical calibration is deferred.
