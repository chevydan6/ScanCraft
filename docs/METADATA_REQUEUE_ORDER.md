# Return metadata before video-buffer reuse — 2026-11-03

**Confirmed:** a controlled pause after returning a video buffer, but before
returning its metadata buffer, reproduces a later full-size image with empty
timestamp metadata. Returning metadata first eliminates this reproduction in
both corrected-binary trials. Startup corruption and paired truncation remain
separate problems. This is an acquisition correction, not scan qualification.

## Implementation

Source commit: `907484e4fa2967e45ca10840fc48f0eed523f124`, parent
`87bd0809613df32aa97f24feb051db8cf1c7b474`.
Changed source:

- `crates/scancraft-device/src/lib.rs`: `Stream::dequeue_then` and
  `Capture::next_frame_until`.
- `crates/scancraft-device/src/paired_dequeue.rs`:
  `matching_metadata`, `complete_then_requeue` and five regression tests.

Previously `Stream::dequeue` copied and requeued video before the owning capture
thread read/requeued matching metadata. It now keeps that video slot unavailable
while completing the metadata operation, then attempts video requeue before
returning. Packet data remain owned copies; no mapped-buffer borrow escapes.
The metadata sequence comparison, wrapping arithmetic, 32-packet search bound,
pending future packet, frame validation, rejection counters and timestamp checks
are preserved. No PTS is synthesized or borrowed from the other camera.

Ordinary copy/metadata failures still attempt video requeue. If both the primary
operation and requeue fail, both diagnostics are retained. Any requeue failure is
reported as an `Other` error with its original kind/message in the diagnostic,
so the cooperative wait policy cannot silently interpret a failed requeue as a
cancelled timeout. Unwind still relies on the owning Stream's existing close and
restoration behavior. No hard driver/filesystem cleanup deadline is introduced.

This is original MIT Rust code with no new dependency, recording-format change or
public function signature change. It affects the shared native Capture path, rather than only the finite
recording CLI. Device/GUI release qualification still requires broader testing.

## Evidence leading to the correction

The [preceding ten-minute trace](RAPTOR_SUSTAINED_QUEUED_CAPTURE.md) retained an
empty left metadata buffer following approximately 106 ms between video and
metadata requeue. Its synchronous trace writes could themselves contribute to
that pause. The cause of that particular natural pause remains unknown.

The existing bound kernel candidate activates a metadata buffer in
`uvc_video_decode_start` only when its video buffer becomes ACTIVE;
`uvc_video_decode_meta` ignores non-ACTIVE metadata. This suggested a timing
mechanism, not an initially proven attribution. Controlled experiments below
test the queue timing independently of the original trace-file write behavior.

## Controlled reproduction

An original private diagnostic uses 20,000 bounded, prefaulted in-memory trace
rows and flushes them only at process teardown. It adds no acquisition-time
trace-file writes. It still has CPU, memory, syscall and scheduling overhead;
prefaulting is not a guarantee against every subsequent page fault. A device-free
four-thread self-test verifies 8,000 complete rows, zero lost rows, and a zero-byte
trace file until teardown. The first compile failed a misleading-indentation
warning; it was corrected before hardware execution and the failed source/log
are retained. Final diagnostic builds use warnings as errors.

The intervention sleeps 110 ms immediately after successful left video QBUF
ordinal 204, including the four initial buffer submissions. It does not change
the ioctl result, image data, metadata, flags or counters. Both cameras record
300 accepted 1920 × 1200 GREY frames at requested 30 FPS with temporary flood
illumination. Four video and four metadata V4L2 buffers remain in use. Each trial
uses the same temporary 32-URB candidate, reloaded before the trial; its source,
hashes and parameter-name mapping are in [the queue experiment](RAPTOR_QUEUE_DEPTH_TRIAL.md).

The unchanged queued-recorder binary first ran control/delay/delay/control:

| Trial | Delay | Startup rejects L/R | Ongoing rejects L/R | Left ongoing reason |
| --- | ---: | ---: | ---: | --- |
| Control 1 | none | 91 / 4 | 1 / 1 | Paired truncation |
| Delay 1 | 110 ms | 1 / 0 | 1 / 0 | Missing PTS, empty metadata |
| Delay 2 | 110 ms | 1 / 0 | 1 / 0 | Missing PTS, empty metadata |
| Control 2 | none | 1 / 0 | 0 / 0 | None |

Both delayed trials have a full-size video buffer and empty metadata four
sequences after the delayed return. Their left sequence-gap count is one.
Controls do not have that empty-metadata failure. All eight recordings replay.

## Old/corrected comparison

Old binary SHA-256:
`68335a91b77ec88f3b5cd32aa5c0d8ab8b46b1a259e9929ab467fdfe5a559b07`.
Corrected binary SHA-256:
`3a46c4995b134912482af20a303e76c024ea6cea38e699935a6a4eef430cae55`.
Both use the same buffered tracer SHA-256:
`9c19a1982720d552bb36df050c9742e5ad4770dd889f7954f6aeb2655f802bd8`.

Order is corrected/old/old/corrected. Every trial applies the same delay and
records 300 accepted frames per camera. Measured delays range 110.061–110.366 ms.

| Trial | Startup rejects L/R | Ongoing rejects L/R | Sequence gaps L/R |
| --- | ---: | ---: | ---: |
| Corrected 1 | 0 / 1 | 0 / 0 | 0 / 0 |
| Old 1 | 0 / 94 | 1 / 0 | 1 / 0 |
| Old 2 | 0 / 1 | 1 / 0 | 1 / 0 |
| Corrected 2 | 0 / 1 | 0 / 0 | 0 / 0 |

Both old failures are empty metadata/missing PTS at the expected later sequence.
In both corrected traces, matching metadata QBUF completes **before** video QBUF
starts. In both old traces, metadata requeue occurs **after** the injected pause.
All 2,400 accepted frames in these eight recordings pass native complete replay.
The recorded order change, repeated intervention and preserved validation support
the correction for this reproduced failure. They do not prove why the original
natural stall happened or guarantee losslessness under arbitrary delays.

A following corrected, uninjected 900-frame/camera capture has startup rejects
1/0, ongoing rejects 1/1 and sequence gaps 1/1: a paired truncation remains.
Both complete recordings replay. This 30-second observation is not a new
ten-minute or 30-minute reliability qualification. Including initial reproduction
and comparison/natural trials, all **6,600 accepted frames in 18 recordings** replay;
every trace reports zero lost/error rows and every per-trial state comparison passes.

## Cancellation and restoration

After restoring the installed baseline driver, a separate private tracer pauses
the corrected CLI after a successful left video DQBUF, while that slot is held.
A nonblocking one-byte notification tells the harness to send SIGINT during the
200 ms pause. The finite CLI checks termination around `next_frame()` calls;
it does not use the mid-frame cooperative stop predicate for this operation.
It completes the held pair, returns metadata before video, reports SIGINT, closes
all four streams and publishes no successful capture directory or manifest.
The retained 98/100-frame prefixes both pass complete native replay. Raptor
left/right/color stream state and illumination readbacks match the initial state.

The first cancellation harness wrongly expected no metadata dequeue after SIGINT
and failed that assertion after the native process had exited nonzero and its
state comparisons had passed. The initial harness and capture remain retained.
Source inspection and corrected offline analysis verify one matching metadata
read/requeue before video requeue and stream shutdown. The exact native exit code
was not persisted before that harness assertion; no numerical exit-code claim is
made. This is frame-boundary cancellation evidence, not mid-frame preemption or a
hard cleanup bound. No repeat capture was substituted for the original evidence.

Both administrative experiments exited successfully after restoring the original
module/parameters, active WirePlumber and saved format/interval structures for
all five idle capture nodes. Native Raptor IR/color/illumination readbacks also
matched. No firmware write or persistent kernel installation occurred. The
ordinary session locks and no-forced-unload policy were retained.

## Validation, review and remaining work

`tools/check.sh` passes formatting, strict all-target Clippy, **863 tests** with
one pre-existing ignored test, and optimized workspace builds. Five new tests
cover dependent-operation/requeue order, wrapping/future metadata, bounded search,
stop versus missing, copy failure, and combined errors. These tests establish
software contracts; the hardware interventions above establish the reproduced
device behavior. Recorded source hashes match the tested binary's source snapshot.

Private reproducible evidence sets are `metadata-requeue-bu1_jio4` and
`metadata-order-ox4xx69i`, under the developer artifact directory. Plans, source/
binary hashes, diagnostic/self-test sources, native outputs, complete replays,
raw recordings, traces, analysis scripts, cancellation failure/reanalysis and
restoration events are retained with SHA-256 manifests. Private data are not in Git.
CodeRabbit quota lookup remains `lookup_failed`; no new completed automated review
or paid review is claimed. Independent implementation review remains outstanding.

**Follow-up:** [sustained corrected capture](RAPTOR_SUSTAINED_METADATA_ORDER.md)
now extends this evidence to ten minutes, preserving startup, truncation and
sequence-gap accounting. The all-rejection target still fails. Holding a video slot during
metadata work reduces available video slots during that wait; no throughput
advantage is claimed without measurement. SCAN-REV-005 cleanup bounds,
SCAN-REV-006 physical qualification and SCAN-REV-021 live-session capacity remain
separate. Physical scale, a measured real object surface and full native scan/edit/
save/reopen/export qualification are still required.
