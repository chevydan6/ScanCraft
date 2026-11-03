# Sustained combined acquisition: 610-second finite pass

**Confirmed:** one temporary bounded-copy + paused-logger trial captures and saves
18,300 frames per camera over more than 610 seconds, with **zero startup or ongoing
rejected buffers and zero sequence gaps**. All 36,600 frames pass native replay and
strict USB header association. Original module, services, formats and checked
scanner state restore. This meets the declared duration and rejected-buffer proxy
checks for this trial. It is not a production fix or a physically validated scan.

## Relation to the earlier trial

This extends the [one-minute combined experiment](RAPTOR_COMBINED_ACQUISITION.md)
at acquisition checkout `7947954e3ee81709a0b9ea03565a882207269c7b`. The native CLI,
kernel candidate, original baseline and logger source hashes are unchanged from
that report. The option is temporarily enabled with the original module parameters;
no persistent driver installation, firmware, affinity or power-policy change occurs.
Earlier failed individual interventions remain in the evidence history.

The private plan binds 15 inputs before execution. It requests 18,300 accepted
frames/camera at 30 FPS, 1920 × 1200 GREY, temporary flood illumination, at least
600 seconds of accepted host span, 110 GB free disk and 15 GB available memory.
Raw image data alone requires 84,326,400,000 bytes; the two complete recording files
occupy 84,328,383,162 bytes including format/context/frame metadata.

**Instrumentation differs deliberately:** this longer trial retains the 64 MiB
USB monitor ring and 32 payload-prefix bytes, with a 900-second monotonic watchdog,
but does not allocate a private scheduler/IRQ trace. A fixed trace buffer sized
for one minute could overflow over ten minutes. Global trace configuration is
read before and after and remains unchanged. Effective affinity readbacks are
CPU10 → CPU10; without a host trace they do **not** establish actual IRQ placement,
copy-worker execution, request-recycling timing or an isolated performance change.
The preceding full-trace admission is not rewritten or weakened.

## Finite results

| Observation | Left | Right |
| --- | ---: | ---: |
| Saved and replayed frames | 18,300 | 18,300 |
| Startup rejected V4L2 buffers | 0 | 0 |
| Ongoing rejected V4L2 buffers | 0 | 0 |
| Sequence gaps | 0 | 0 |
| Accepted host span, seconds | 610.042251 | 610.042255 |
| Accepted buffer rate, Hz | 29.996283 | 29.996283 |
| Successful video bulk completions | 3,440,400 | 3,440,400 |
| Terminal canceled bulk completions (status -2) | 5 | 5 |

Both total and ongoing-only rejected-buffer proxies are **0/36,600 = 0%**.
These are reported V4L2 buffer/sequence observations, not independently counted
physical exposures or a statistical long-run reliability bound. All 18,300 pairs
have equal sensor PTS; physical exposure synchronization remains unverified.

The native CLI performs complete CRC/footer replay before publication; two further
native replays of the complete files also succeed. The existing recording queue
remains eight frames per camera. USB monitoring exits0 after signal, before its
watchdog, saving 13,761,774 selected events with zero queued/dropped events and no
I/O failure. Submission/completion identities balance with no unmatched or pending
identity. Ten terminal cancellations retain their negative status.

All 36,600 accepted frames associate with exactly 188 valid-header payloads,
2,304,000 image bytes, one FID and one terminal EOF. PTS and native lengths agree;
there are no semantic/length mismatches or unrecognized successful boundaries.
USB prefixes are not an independent full-pixel reconstruction; the recordings
supply the separately integrity-checked image payloads.

## Explicit assessment and offline validation

The new private `admit_sustained.py` assesses the declared USB/native scope. It
requires recorded collector success, exact candidate/baseline parameters, all three
seven-field native comparisons, format/service/observer restoration, unchanged global
trace configuration, complete USB identities/counts, strict headers, complete native
association, successful replays and unchanged queue capacity. It checks both actual
sample timestamp spans and their reported values, frame counts, metadata counters,
and strictly-below-0.1% total/ongoing rejection proxies. It returns **admitted:true**
with no reasons for rejection on this capture. This is not a full mission gate.

Before hardware execution, 54 copied-evidence fault-injection cases pass, including
missing/duplicate state records, USB loss, malformed headers, incomplete recordings,
incorrect duration, restoration failures and exactly 0.1% (which must fail).
An initial global-trace mutation test failed because it set an already-set value;
the corrected test flips the actual value. Assessment code was unchanged by that
correction and the initial test failure is retained.

The strict header analyzer changes only its declared USB identity and capture path.
Against read-only copies/references of earlier genuine PCAP inputs, it reproduces
both the previous passing result and the two missing-EOF failures exactly. Those
regressions establish preserved parsing behavior, not sustained hardware accuracy.
Collector source inspection confirms removal of private trace writes/allocation;
ordinary locks, no-force module handling and restoration remain. These finite
checks do not prove cleanup under arbitrary kernel/filesystem stalls or crashes.

## Restoration and resource scope

All 21 native field comparisons across candidate-before, capture-after and final
snapshots agree. All five saved camera formats restore. Saved commands/readbacks
support original baseline module `9D1275CDD5E013864E4C564` and parameters, original
logger source/active state, WirePlumber and unchanged global tracing. Binary hashes
match the reviewed artifacts; readbacks are not independent resident-code attestation.
Kernel logs add two ordinary module reload/discovery sequences and the existing
deprecated-nodrop warning; 12 old ring-buffer lines roll out. No other added kernel
warning/crash signature appears in the saved comparison.

Whole native CLI time-v: **661.97 seconds wall, 38.06 user + 53.18 system seconds,
54,448 KiB peak RSS**, exit0. This includes recording and its internal replay;
it excludes the USB monitor, later replays/extraction, kernel cache and offline
analysis. It does not qualify whole-app memory, rendering, reconstruction throughput,
latency, or a 60-minute memory-growth target. It is not directly comparable with
the previous full-host-trace timing as an isolated optimization benchmark.

Both final frames were extracted through the native integrity-checked export and
viewed. They show the calibration board, still clipped on the right in the right
camera. A recognizable board image is not a measured object reconstruction.

## Development state and next priority

No application, kernel implementation, Cargo manifest or recording format changes
in this milestone. The same implementation passed the preceding 892-test,
formatting, strict Clippy and release gate; both exact7947954 CI runs also pass.
Independent assignment489 verified the preceding one-minute finite evidence, and
495 verified its CI completion. The new sustained evidence awaits independent review.
A fresh CodeRabbit usage lookup again returns unavailable/lookup_failed; no review
upload or paid usage occurs. Completed substantive review remains absent; no merge
approval follows from CI or these hardware results.

**Recommendation:** keep the candidate experimental/default-off while checking
repeatability, controlled cancellation/disconnection and operation through the
real scanning pipeline. Reliable flood-image acquisition does not qualify blue-laser
reconstruction, physical scale, object accuracy, tracking/fusion, mesh quality or
GUI edit/save/export. The first independently measured object scan remains the
primary unfinished deliverable. Do not substitute this acquisition milestone for it.

Private evidence directory `inline-sustained-20261104` retains the predeclared plan,
source/input bindings, offline tests (including the initial failure), raw recordings,
USB headers, complete replay logs, resource measurements and restoration records.
All private scans and binaries remain excluded from public Git.
