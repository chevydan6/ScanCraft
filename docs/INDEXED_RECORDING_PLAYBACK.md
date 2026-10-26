# Indexed recording access

`scancraft-recording::indexed::IndexedRecording` is the offline access layer for
native recording playback. Construction uses the strict streaming reader to
validate every frame, global frame order, the footer count and EOF before any
selected frame can be returned. It retains offsets, predecessor order and CRC32
fingerprints, not all image payloads. The caller supplies a cancellation/progress
checkpoint and a maximum of 1,000,000 frame entries. Exceeding the budget is an
error, never a silently shortened recording. Empty recordings are supported by
the library; the diagnostic requires at least one frame.

Subsequent reads seek to one frame and validate its structure, CRC, predecessor
order, extent and original fingerprint. Tests instrument actual Read calls: a
52-byte synthetic frame record requires exactly 52 bytes per navigation,
including last/first/middle/last order. This avoids repeated full-record scans.
It does not eliminate the initial full-file read or existing raw-frame copies.
Image decoding and source identity monitoring belong in the playback workflow.

The source must remain immutable during use. CRC32 protects against accidental
corruption, not malicious collisions. Seeking revalidates the selected frame;
it does not rescan the header, footer or unselected frames for later mutations.
A filesystem workflow must open a regular source read-only and compare file and
path identity/length/modification timestamps around work, as recovery already
does. This library alone does not qualify concurrent file modification handling.
Legacy v1 context/header CRC remain unavailable; no new provenance is invented.
Keep recovered recordings with their recovery report.

Run the diagnostic without camera access:

```
cargo run --release -p scancraft-recording --example indexed_replay -- recording.raptor
```

It validates the file, reads last/first/middle/last and reports checksums and
local timings without outputting images or device identity. Timings are not
throughput acceptance measurements.

Verification: four index regression tests cover bounded/cancelled construction,
truncated/corrupt/trailing suffix rejection, precise random-read byte counts,
out-of-range recovery and selected-frame changes even with recomputed file CRC.
Workspace tests and strict all-target lint pass; see
`evidence/recording-index-{tests,clippy,release}.log`.
`evidence/recording-index-hardware.json` records short genuine captures from both
IR cameras and color, full decoded replay, nonsequential indexed access,
unchanged source hashes and restored camera settings. Raw recordings remain in
private temporary storage and are removed after checking.

A cancellable native playback panel now wraps this access layer with guarded
file identity and bounded decoding; see RECORDING_PLAYBACK.md. Calibrated
reconstruction and full mission qualification remain open.
