# Distinguishing startup, image and metadata rejection indicators

Native `Capture` now reports `rejected_before_first_valid` and
`rejected_after_first_valid` in addition to the unchanged total rejection count.
Their sum must equal the total. These count explicit V4L2 buffer rejects; missing
metadata and sequence gaps remain separate. All startup rejects still contribute
to the qualification proxy. No flags or payload checks are relaxed.

The existing first 32 `rejection_samples` are retained. A separate bounded first
32 `ongoing_rejection_samples` captures rejects after the first accepted buffer,
even when a startup fragment burst has already filled the legacy sample. At most
64 diagnostic records are retained per camera; no image payload is stored in
these records. Once both applicable caps are full, no further diagnostic records
are constructed. Counters continue for the full run.

Each retained rejection has independent indicators:

- Video and metadata V4L2 error bits, alongside the original full flag values.
- Expected uncompressed bytes and actual length agreement for GREY/Y14. Variable
  compressed sizes remain unknown, rather than receiving a fixed-size check.
- First raw UVC flag byte and first raw PTS candidate, explicitly diagnostic.
- A strict parse result for the retained metadata's PTS structure, independent
  of the V4L2 flags. `null` parse error means a PTS was parsed, not that the image
  is valid or that every original USB payload header was retained.

The original `reason` string remains the first failure for compatibility. It is
not a root-cause label. The local baseline Linux UVC source sets video error on an
uncompressed size mismatch and can copy the video buffer error onto metadata in
`uvc_video_next_buffers`. The source matches its earlier fingerprint, and its
built module matches the installed module file. See
`evidence/rejection-kernel-source-review.json`. This is source/build evidence,
not a kernel trace proving the path of a particular rejected frame.

## Verified new observation

A 1,500-valid-buffer-per-camera 30 FPS run retained all five ongoing rejects on
each camera, with zero rejects before the first valid buffer. All five had short
image payloads and both V4L2 error flags. All retained metadata PTS structures
parsed successfully; first UVC flags were 0x8c/0x8d, without the 0x40 device-error
bit. All image deficits were multiples of 61,380 bytes. The five raw rejection PTS
values were shared between cameras. This narrows the observed failure to short
video with propagated-compatible flags, rather than demonstrating malformed
retained PTS metadata. It does not identify a controller/kernel/firmware cause,
exclude errors in unretained USB headers, or qualify exposure synchronization.

The explicit rejection proxy remains failing: five / 1,505 buffers per camera,
about 0.3322%. Different counts from earlier runs are not an improvement claim:
only diagnostics changed, and these are short, variable runs.
Evidence: `evidence/stereo-rejection-indicators-20261026/`.

```
scancraft-cli stereo-probe 1500 --fps 30 > native.json
python3 tools/check_stereo_rejection_audit.py native.json ongoing-audit.json --ongoing
```

The audit rejects `--ongoing` on legacy reports that did not record this sample;
it does not pretend unknown old data was empty or successful. Legacy assessment
phase counters remain `null`; newly recorded phase counters are checked for a
consistent sum. Existing raw/timing evidence is unchanged.

## Verification and remaining work

404 Rust tests pass, including flag/size/metadata independence, device UVC error,
missing/truncated/inconsistent metadata, compressed-size unknowns and preserving
a bounded ongoing sample after 200 startup rejects. Twelve offline tooling tests
pass, including legacy compatibility, phase consistency and independent sample
selection. Strict workspace lint, formatting and release build pass.

The earlier counterbalanced 15/30/30/15 experiment used one unchanged binary and
roughly equal 40-second accepted spans. Both rates still reject buffers; large
startup bursts occur on either camera. See the appended results in
`ACQUISITION_RATE_INVESTIGATION.md`. Lower rate is not a verified fix and does not
replace the mission's acquisition/reconstruction targets. Kernel queue depth,
USB transport, startup framing and device behavior remain hypotheses to isolate.
The existing candidate kernel module was not loaded or installed by this work.

One subsequent ten-buffer warm startup was clean on both cameras; this is one
observation, not evidence that the earlier intermittent startup issue is fixed.
Its phase counters and totals agree. The source and native binary hashes used by
the new diagnostic run are bound in its evidence checks.
