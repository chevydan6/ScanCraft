# Live payload-hash validation and enabled-candidate result

**Confirmed developer result:** a fresh capture using the existing inline-copy
candidate contains seven accepted images with partial duplicated strips. All 3,600
accepted images agree with the full-transfer SHA-256 values observed by usbmon.
Each affected frame has four software requests still outstanding after every
completion. Zero ongoing rejected buffers therefore remains insufficient evidence
of intact pixels. No correction or qualification is claimed.

This continues [the historical candidate audit](RAPTOR_CANDIDATE_PIXEL_AUDIT.md),
independently verified in reviewer assignment 580. SCAN-REV-034 remains High/Open.
Application source is unchanged from `d91fc4f696ee275e77145406ea55126475b515c6`;
capture documentation HEAD is `00284ea7c1b33f7d2285b11cdb8ce802ae806372`.
The release CLI SHA-256 is
`8ec0f44f18a090a45c90539ec78af2e8220fb8fdfb351e1236259f5ef3d57706`.

## Validate the observer before interpreting its results

Two baseline-driver pilot captures each retain 120 images per camera at 30 FPS,
line mode and raw gain 15000. Simultaneous observers retain complete USB bodies
in PCAP and a compact record containing the usbmon header, 32-byte prefix and
SHA-256 of the entire captured transfer. Both use 64 MiB rings and bounded
deadlines. They open the bus monitor with privilege, then drop to the owner account.
No device permissions are changed.

| Pilot | Startup rejected L/R | Ongoing rejected L/R | Hash events | PCAP events | PCAP drops |
| --- | ---: | ---: | ---: | ---: | ---: |
| Packet-flushed PCAP | 1 / 0 | 0 / 0 | 91,518 | 90,509 | 1,009 |
| Buffered PCAP | 98 / 0 | 1 / 1 | 92,202 | 92,202 | 0 |

The first runner correctly fails its zero-drop assertion after native capture and
restoration. That global PCAP is not admitted as a complete trace. Its retained
data nevertheless contains all 188 payloads for each of two affected images at PTS
2857993771, left index 89 and right index 90. Literal original USB image bytes equal
both recorded images. Repeated strips displaced by 24,552 bytes span 15,940 left /
15,296 right bytes, with 112 / 93 distinct values. The right image has an additional
336-byte run. The complete hash observer independently matches all 240 recordings.

The second pilot removes only tcpdump's `-U` packet-flush option. This is a finite
observer validation, not a controlled performance comparison. Every one of its
92,202 PCAP events matches the compact log's non-timestamp header fields, prefix
and full-payload digest. Per-reader callback timestamps differ by 0–62 µs; literal
64-byte header identity is **not** claimed. Submission/completion accounting
balances, with no truncated successful bulk payloads, drops or monitor I/O errors.
All 240 accepted images equal literal original PCAP bytes and have one FID and one
terminal EOF. Both rejected ongoing images at PTS 2992575209 contain 2,119,860
image bytes in 173 payloads; native and USB lengths agree. The 98-buffer startup
burst and 188 nonstandard successful bulk boundaries remain reported.

Both pilots pass complete native replay and restore seven device readbacks,
saved camera formats and the media service. A verifier's initial tuple-index
error is retained alongside its corrected run; no source data or criterion was
changed. Native exports reproduce the affected pair and a second-pilot image.
Finite agreement supports use of the compact observer, not physical-wire,
exposure-sync, optical-integrity or long-duration qualification.

## Fresh enabled-candidate capture

One predeclared trial temporarily loads the existing candidate
`08adf1be077be172538131d3accf42f317bb7421`, enables inline copy and pauses the optional
session logger. No kernel source, firmware, affinity or permanent installation
changes are made. The runner retains lock/consumer/resource checks, five camera
formats, seven native state snapshots, ordinary module unload/load and restoration.
This time only the full-payload hash observer runs; no full PCAP or host trace.
Line mode, the current box scene and observer configuration differ from historical
flood-board trials, preventing an isolated causal comparison of their rates.

The capture saves 1,800 accepted images per camera. Accepted host spans are
59.974146 / 59.973634 seconds, approximately 29.9963 / 29.9965 accepted buffers/s.
Startup rejection counts are 1 / 0; ongoing rejections and sequence gaps are 0 / 0.
The observer saves 1,354,538 selected events, with zero queued/dropped events or
I/O errors; submission/completion accounting balances. Both native replays pass.

All **3,600** accepted images match every observed full-transfer hash and prefix.
Each has 188 payloads, 2,304,000 image bytes, one FID and one terminal EOF. This is
cryptographic correspondence to observed full payloads; the original USB bodies
are not retained in this experiment. It must not be described as direct literal
PCAP-body comparison. usbmon remains a software/HCD boundary, not a wire analyzer.

The unchanged diagnostics find no full aligned-block repeats and seven images
with partial repeats. These are diagnostic observations, not a prevalence estimate
or a production rejection policy. Indices are zero-based; physical exposure
synchronization is not established by matching PTS alone.

| Raw PTS | Left index | Right index with signature | Longest repeated run L/R, bytes |
| --- | ---: | ---: | ---: |
| 4104686842 | 592 | 593 | 12,504 / 10,720 |
| 4113654484 | 861 | None | 2,640 / — |
| 4113954517 | 870 | 871 | 4,960 / 3,932 |
| 4137390399 | 1573 | 1574 | 12,640 / 10,720 |

There are nine repeated runs across these seven images. Every run is byte-equal
at its two positions and contains 81–207 distinct values. Both adjacent images
have unequal contents at those positions and differ from the incident run.
Native exports of all seven affected images and the last image from each camera
are byte-exact, including the PNG pixels. Viewed left index 592 has a narrow
horizontal displacement near the top crossing projected stripes. Viewing two
images does not qualify all image content.

Every completion in every affected frame leaves four requests outstanding, with
no empty-software-queue interval overlapping the frame. Maximum within-frame
completion gaps are 164–177 µs. Repeated-run starts fall in distinct requests two
payloads apart; their completion separations are 110–149 µs. Most starts are at
transfer offset 5120, with additional starts at 6144 and 12. These are observations,
not proof of a particular DMA, packet, firmware or controller defect. Software
request counts do not measure physical controller-ring occupancy.

**Engineering inference:** this extends the baseline boundary localization to the
enabled candidate. Duplication exists in the data observed before ScanCraft's
reconstruction and storage, including when the software request queue remains
populated. Neither the candidate nor eliminating an empty software queue supplies
a complete remedy. Scanner production, link/controller and HCD/DMA behavior remain
unresolved; choosing among them requires a discriminating experiment.

## Restoration, costs and evidence

The candidate runner exits 0, restores baseline srcversion
`9D1275CDD5E013864E4C564` and every module parameter, and verifies 21 state comparisons
(seven each before capture, after capture and after restoration). Logger source and
active state, saved formats, media service and global tracing configuration restore.
There are no cleanup-error events. An earlier lost-session sudo prompt had no child
and no runner artifacts; it was canceled before the single actual capture began.

The timed native capture process uses 73.91 s wall time, 4.17 s user, 14.31 s system,
25% CPU and 40,320 KiB peak RSS. This includes recording finalization and excludes
the separate observer; it is not system-wide utilization, an isolated observer-cost
benchmark or reconstruction throughput. No lower-cost claim follows from removing
the second observer.

Private evidence retains all raw recordings, logs, frozen inputs, failures and
derived checks. `compare_candidate.py` adapts only capture paths and time-window
schema from the pilot analyzer; it checks all images against their transfer hashes.
`verify_affected.py` checks native exports, neighbors, whole-log request accounting
and affected-frame context. The original analyzer and capture inputs are preserved.

Pilot evidence is sealed as 218 files / 2,287,094,321 bytes, manifest SHA-256
`6346426bedb10e02d618a805d7fc8f9f573172162f3534b04172ead9c4a4b09a`.
Candidate evidence is sealed as 127 files / 8,515,752,336 bytes, manifest SHA-256
`c0a3dad9bcb913050d57c43341ff08ce32f661d08079be2f396c4e3cbfdf82ea`.
It uses private roots `candidate-hash-20261104` and external
`candidate-hash-publication-20261104`.
Private imagery and device data are not included in public Git or review uploads.

No coefficients, thresholds, rejected-frame rules, recordings or application code
are changed. Rotation is offered but not confirmed; the box's independently stated
dimensions remain held out. Full object coverage, proven physical units, dimensional
accuracy, three repeat trials and complete native end-to-end scanning remain unmet.
Only the independent reviewer can verify and close SCAN-REV-034.

Publication software gate: `tools/check.sh` passes formatting, strict all-target
Clippy, 935 tests / zero failures / one ignored, and optimized workspace build.
Application source is unchanged; this is regression evidence, not hardware
qualification. CodeRabbit's included-capacity/billing lookup returns `lookup_failed`;
no substantive automated review or paid usage was performed. The PR remains draft.
