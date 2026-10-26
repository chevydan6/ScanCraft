# Recover interrupted recordings

**Cameras → Recover interrupted recording** creates a new verified recording from
the contiguous valid prefix of a stopped or interrupted native recording. Choose
the source, a new output bundle and a maximum frame count, then **Recover and
verify**. Progress/cancellation run in a background worker; active recovery is
included in the app's close handling. Recovery does not start camera acquisition.

```
scancraft-cli recover-recording INPUT MAX_FRAMES NEW_OUTPUT_DIRECTORY
scancraft-app --recover-recording INPUT MAX_FRAMES NEW_OUTPUT_DIRECTORY
```

The source is opened read-only and never replaced. Existing output destinations
are refused. Publication is a no-overwrite atomic directory rename after file and
directory sync. Failed/cancelled work retains its staging directory and reports
its location; the final destination remains absent unless publication already
committed. Cancellation is cooperative at I/O/frame/publication checkpoints.

## Data policy

Recovery validates the source header, then uses the same strict frame reader as
normal replay: body CRC, bounded lengths, payload framing, UVC metadata/PTS and
sequence/host/sensor timestamp ordering. It copies only complete validated frames
up to the first invalid/truncated frame, damaged/missing footer, or explicit frame
budget. It never scans for a later possible frame boundary and does not interpolate
missing frames. With no validated frames, recovery fails instead of publishing
an empty substitute.

A new version-2 completion footer describes the retained frames, not the originally
intended scan length. `report.json` and the manifest explicitly label the result
as a recovered prefix, report the stop reason and whether the source footer was
verified. **Keep the report with `recovered.raptor`.** Recovery provenance is in
the bundle, not a new embedded recording-header field; moving the raw file alone
loses that provenance. The supported original profile/context values are copied;
JSON header serialization may differ from noncanonical inputs.

Version-2 session headers require a valid CRC. Version-1 headers had no CRC or
capture context: the report marks this absence, the app warns about the legacy
header, and the output gets an explicitly unknown default context. Recovery does
not invent calibration, control states, source completion or physical accuracy.
JPEG checks in the recovery reader are structural framing checks; full image
decoding is a separate operation, not implied by CRC/footer replay.

Underlying input I/O failures are distinguished from parser-detected truncation
or corruption and are fatal, including input errors using InvalidData/UnexpectedEof
kinds. Caller cancellation is also fatal and cannot masquerade as a repairable
suffix. Source identity, size, mtime and ctime are rechecked before publication;
observed changes abort it. These checks do not lock out concurrent writers.
Recover stopped files and retain originals for further investigation.

The output is fully CRC/footer-replayed before publication. A report includes
source version/header protection, recovered count, last validated boundary, logical
bytes consumed and CRC32 of those consumed bytes. That CRC can include an inspected
bad suffix and is not a whole-source checksum when a budget stops reading early.

## Bounds

Frame budget: 1–1,000,000. Source: regular non-symlink file, at most 1 TiB or the
smaller host indexing bound. Reads are capped at the initial source size. Existing
format limits remain: 1 MiB header, 64 MiB frame body, 4,096 metadata bytes and
16,777,216 declared image pixels. Frames stream individually rather than loading
the file. Large-file speed, peak memory and recovery under every failure timing
remain unqualified.

## Evidence

417 Rust tests pass. New tests cover all truncation points around three frames,
exact frame/context preservation, corrupt middle frames with later valid data,
CRC-valid semantic/huge-length errors, footer damage, explicit budgets, invalid
headers/zero frames, legacy conversion, fatal I/O and cancellation. Workflow tests
cover publication/replay, original bytes, no overwrite, source mutation and
special-file/symlink rejection. The native GUI worker recovers/replays a prefix
and reports a refused overwrite without changing its source. Strict lint,
formatting and release workspace/example builds pass; twelve existing tooling
tests also pass.

`evidence/recording-recovery-proof-hardware-20261026/` recovers genuine cancelled
left-IR, right-IR and color captures on the final release binary. Each retains and
successfully decodes three frames. Source bytes stay identical; each output's
header/frame prefix matches the native version-2 source byte-for-byte, followed
only by a new footer. Existing destinations are refused. Raw image files are
confined to private temporary storage and removed; public evidence contains only
reports, hashes and counts. This is not calibrated geometry qualification.

Synthetic fixtures/examples are under `recording-recovery-fixtures/` and the
`recording-recovery-*-example/` directories. The corrupt-middle example retains
one frame and never adopts the later third frame. They are not scanner captures.
The fixture generator is a native Rust example, `recovery_fixture`.

Private-display startup capture and worker evidence verify the native panel and
result, not a full mouse workflow. The screenshot review initially misclassified
apparent partial images: byte/pixel checks found the repeated saved PNGs identical
with full window data. That failure interpretation was withdrawn in
`recording-recovery-render-review.json`; no rendering bug fix is claimed.

## Idle repaint observation

The app now polls redraws while camera acquisition, background jobs or smoke
checks are active, instead of unconditionally redrawing idle windows. Existing
camera/worker polling remains. On the same short private-display task, app CPU
use changed from 52 ticks / 2.350 s to zero measured ticks / 2.349 s at 100 ticks/s.
This is approximately 0.52 CPU-seconds before the change and below counter
resolution afterward, not a universal zero-CPU or scanning-throughput claim.
Evidence: idle-baseline/idle-controlled render metadata and the render review.
