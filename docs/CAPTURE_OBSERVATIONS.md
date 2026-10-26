# Recorded setup observations

Native GUI recording, single-camera CLI capture and stereo recording now include
`controls.capture_observations` in the existing CRC-protected recording-v2 header.
The recording API remains decoupled from hardware: CaptureContext accepts typed
serializable observations and preserves existing trial/calibration/identity fields.
Older recording readers already understand opaque control JSON; synthetic and old
recordings are not retroactively populated.

`Capture::open_recording` observes the selected video descriptor after format/rate
setup and buffer preparation, before video STREAMON. The metadata node has already
been initialized. `Capture::open` (plain preview/validation/probe) does not collect
these extra observations. The observer contains GET-only calls; normal capture
setup/cleanup still negotiates and restores format/rate as before.

Stored fields include requested profile, reported stream format/rate, eight
allowlisted standard V4L2 control descriptors/current values/errors, setup wall-clock
Unix microseconds, and bounded reported USB attributes: vendor/product IDs, serial,
manufacturer/product strings, bcdDevice, bus/device numbers and speed. Stream/control
GETs use the actual selected open video descriptor rather than rediscovering a node.
USB attributes are independent path-based reads. Each text attribute is limited to
512 bytes, trimmed and UTF-8 checked. Missing/empty/invalid/oversized values and
unsupported controls retain errors; no current value is replaced by a default.

These are non-atomic setup observations, not per-frame exposure or device-binding
attestation. Sysfs descriptor serials are not promoted into CaptureContext's bound
identity. bcdDevice is only a USB descriptor revision; firmware version and projector
mode are not queried/inferred. Host calibration remains unknown and is not activated.
Auto controls can change after streaming starts. The standard absolute-exposure
interpretation is labeled as such, not qualified as a device-specific physical
measurement. Trial gain/illumination metadata retains its separate scope.

A tested restoration defect was also exposed during this work. With differing
original IR rates (15/30), concurrent stereo probe and recording could leave one
reported rate at the test value (10). The probe failure occurred without the new
observations, so it is not evidence that GET-only observations caused the defect.
The exact kernel/device coupling mechanism remains unproven. Evidence:
`capture-observations-stereo-state-audit/result.json`; known original rates were
restored before the corrected comparison.

Stereo now snapshots both cameras before either worker starts. After all workers
stop, `stream_group::SettingsLease` restores every saved format, then every saved
rate, then reads back all reported state fields. It attempts all entries/phases on
failure; failed restoration prevents a successful report/publication and Drop makes
a best-effort retry. The guard uses only saved settings, with no generic vendor,
firmware or calibration writer. The same short probe/recording comparison passes
with original reported state restored; see stereo-state-fixed/result.json under
the capture-observations evidence prefix. This qualifies these cases, not all
hardware/disconnect/failure timings or physical exposure/gain restoration.

Verification:

- Two device tests cover unknown GET results on a non-device descriptor and bounded
  descriptor parsing; two recording tests cover trial/identity/calibration preservation,
  full header/frame replay, duplicate/type failures and serialization-error atomicity.
- Three group tests cover format-before-rate ordering, attempting all entries despite
  errors/readback failures, and rejecting invalid groups before device access.
- 468 workspace Rust tests / 19 reference-tooling tests, strict all-target lint,
  formatting and release binaries/examples pass. Logs: capture-observations-*.log.
- `evidence/capture-observations-hardware-final/result.json` verifies seven genuine
  entry cases: three single-camera CLI files (3 frames each), both stereo files
  (10 frames each), and native GUI color/IR smoke recordings (at least 20 frames).
  Observed reported format/rate matches each recording profile, unsupported values
  stay explicit, decoded replay succeeds and all reported camera states restore.
- Current-binary signal checks pass 12 single-camera and two stereo cases; three
  interrupted-prefix recoveries still replay decoded frames with exact canonical
  prefixes and unchanged sources. See *-single-cancel-final, *-stereo-cancel-final
  and *-recovery-final under evidence/capture-observations.
- The first harness used an invalid three-frame stereo request, then a second run
  exposed the state mismatch. Both failed attempts are retained as failure.json
  alongside the controlled failing/passing comparison. They are not counted as passes.

Hardware tools retain no raw sensor media/context values in public evidence.
Recordings do contain observed device descriptors; genuine test recordings and GUI
images stay in private temporary directories and are removed. Replay evidence
helpers now explicitly redact the whole context while retaining frame/integrity
counts and presence flags. Redaction has two unit tests. Run the full hardware
check with `python3 tools/check_capture_observations.py NEW_EVIDENCE_DIRECTORY`.

No syscall-trace timing proof, startup/disconnect cancellation bound, throughput,
optical accuracy, calibration, synchronization or complete mission qualification
is claimed. Kernel setup/GET requests can block independently of frame validation's
approximately eight-second deadline. Existing transport truncation and genuine
calibrated reconstruction remain unresolved; calibration stays deferred.
