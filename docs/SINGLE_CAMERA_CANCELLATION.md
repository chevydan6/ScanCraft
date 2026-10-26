# Single-camera validation and recording cancellation

Native `validate` and `capture` now use the same scoped SIGINT/SIGTERM handling as
the stereo commands. Cancellation checks surround camera setup, each frame read,
the end of acquisition, cleanup and recording publication. Errors identify an
unfinished recording's staging path instead of silently deleting it or claiming
that the requested capture completed.

The signal callback remains atomic-only. Capture resources unwind normally and
previous signal handlers are restored afterward. Existing profile/rate allowlists,
frame validation, no-overwrite publication and recording integrity checks are
unchanged. Once the completion footer is written and synced, a final interruption
check precedes the hard-link commit to the requested destination. This remains
cooperative: a publication that already committed is not retroactively undone.
SIGKILL, crashes, disconnects, unresponsive drivers and every possible signal
arrival point are not qualified by these tests.

## Real hardware checks

```
python3 tools/check_single_cancel.py NEW_EVIDENCE_DIRECTORY
```

`evidence/single-cancel-hardware-20261026/` records twelve interruption cases:
left IR, right IR and color, each with `validate` and `capture`, each receiving
SIGTERM and SIGINT. Every test waits for the first **validated buffer**, observes
the requested changed rate, then signals only its owned native process. State
snapshots compare all reported format/rate fields on all three cameras before and
after, including cameras not targeted by that single stream.

- All twelve returned exit 1 with explicit cooperative cancellation errors.
- All twelve restored the reported pre-existing states. Observed signal-to-wait
  completion spans 0.01529–0.06539 seconds, including Python scheduling/polling.
- All six interrupted recordings left the requested final target absent, retained
  nonempty `.raptor.partial` files, and reported their locations. Replay returned
  a controlled incomplete-buffer error for each partial, instead of accepting an
  unfinished recording as complete.
- Three subsequent normal captures, one per camera, published three-frame version-2
  recordings and replayed all three frames with integrity PASS. This is CRC/footer
  replay; `decoded_frames=0`, so no new JPEG decoding or optical claim follows.

Raw scanner recordings are created only inside a private temporary directory and
automatically removed after the test. Public evidence retains diagnostics, state
snapshots, sizes and replay results, with no image payloads. Real interrupted user
recordings retain their staging files; the test's temporary cleanup is specific to
its disposable captures. Firmware, calibration, exposure/gain and projectors are
not changed by the test.

The earlier two-case stereo cancellation harness also passes against this same
release binary. Evidence: `single-cancel-stereo-regression-20261026/`. This protects
stereo cleanup while broadening the shared interruption error text.

## Verification limits

406 Rust tests, strict lint, formatting and release build pass; twelve offline
diagnostic tooling tests pass. Hardware snapshots use the GET-only, non-atomic
stream-state command described in STEREO_CANCELLATION.md. These runs cover normal
steady acquisition interruptions and early recording aborts, not a universal
recovery guarantee, illumination-control cancellation or full reliability gate.
The transport truncation problem and calibrated reconstruction remain open.
