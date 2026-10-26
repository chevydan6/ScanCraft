# Cooperative native stereo cancellation

`stereo-probe` and the stereo recording variants now handle SIGINT/SIGTERM as a
cooperative stop request. Both capture workers return through normal Rust cleanup,
join, release the session lock and attempt normal V4L2 format/rate restoration.
Recording variants keep their existing control-restoration scopes active until
cleanup completes. A cancelled recording is not published as a completed bundle;
partial staging data remains available under the existing failure policy.

The signal callback only stores a 32-bit atomic request. It performs no allocation,
logging, device access, locking or cleanup. A scoped guard installs the two Linux
signal actions before acquisition/control leases and restores prior handlers when
finished. Overlapping scopes are rejected without resetting an existing request.
Blocking calls are not automatically restarted; capture maps an interrupted return
to the explicit cancellation error, allowing its resources to unwind. Checks also
cover replay and the pre-publication checkpoint. Cancellation is cooperative, not
an atomic promise to stop a publication that already committed.

The separate single-camera `capture`/`validate` CLI paths now also use cooperative
termination; their twelve-case hardware evidence is documented in
SINGLE_CAMERA_CANCELLATION.md. The native GUI's existing worker cancellation is separate.
SIGKILL, process crashes, unplugging and unresponsive drivers cannot be treated as
verified restoration paths. The soak runner allows 12 seconds for owned-child
cleanup before its force-kill fallback; force-kill restoration remains unqualified.
Gain/flood/line scopes are protected by the new guard, but signal-driven control
restoration during those modes has not been hardware-qualified here.

## Read-only state snapshots

```
scancraft-cli stream-state INTERFACE
```

For a discovered Raptor video interface, this issues only G_FMT/G_PARM. It reports
width, height, fourcc, field, stride, image size, colorspace and frame interval.
It does not allocate capture buffers, start a stream or set properties. The two
GETs form a non-atomic snapshot and can race another client's reconfiguration;
they do not acquire the acquisition session lock. The tested snapshots were taken
at stable before/active/after checkpoints.

## Hardware evidence

```
python3 tools/check_stereo_cancel.py NEW_EVIDENCE_DIRECTORY
```

Evidence: `evidence/stereo-cancel-hardware-20261026/`. The test changes both IR
streams from their observed original intervals (left 1/15 s, right 1/30 s) to
1/10 s, confirms the active GREY format/rate using read-only snapshots, and signals
only the owned native process. It then compares all reported fields with the
original snapshots and reopens the scanner. No projector, firmware or calibration
controls are used; no image payloads are saved.

| Signal | Native result | Signal-to-exit | Reported state restored |
| --- | --- | --- | --- |
| SIGTERM | exit 1, explicit cleanup-request error | 0.16450 s | PASS, both cameras |
| SIGINT | exit 1, explicit cleanup-request error | 0.16587 s | PASS, both cameras |

A subsequent reopen accepts ten buffers per camera with zero rejects in that
short observation. These are two interruption cases, not full initialization,
reconnection, illumination, transport, latency or reliability qualification.
The short-data truncation issue from the earlier soak remains unresolved.

## Offline verification and scope

406 Rust tests, strict workspace lint, formatting and release build pass. An
isolated child test exercises both real signal callbacks, nested-scope rejection,
request reset and restoration of prior custom handlers, keeping the parent test
process untouched. The regular helper entry is activated only by that subprocess
test. Twelve offline diagnostic/owned-child tooling tests pass.

The initial full run exposed an unrelated parallel test-fixture collision:
frame-document temporary directories relied on a timestamp alone. A process-local
atomic sequence is now included in each name. The original failure log is retained
as `stereo-cancel-tests-before-temp-fix.log`; the subsequent full suite passes.
No frame-document production parsing or integrity validation was relaxed.

The prepared kernel queue-depth experiment is still unloaded/uninstalled.
`sudo -n true` reports that authentication is required, so no module experiment
was attempted. That constraint does not prevent native application progress.
