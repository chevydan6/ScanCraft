# Read-only standard camera diagnostics

`scancraft-cli controls` acquires the shared native/bridge session lock and queries
only standard V4L2 controls on discovered Raptor image nodes. It neither starts
streams nor writes camera settings or vendor commands. Wait for another capture
or diagnostic session to finish before invoking it.

The fixed allowlist is exposure-auto mode, absolute exposure, auto-exposure frame
priority, autogain, gain, brightness, contrast and gamma. Each result retains
QUERYCTRL metadata (driver name/type, range, step, default and flags) and the
current G_CTRL value. Unsupported/query/read errors are retained individually;
missing controls do not become invented defaults. Disabled or write-only controls
are not read. A node-open/discovery failure fails the command explicitly.

Numeric gain/brightness/gamma values remain in driver-defined raw units. For
positive `V4L2_CID_EXPOSURE_ABSOLUTE` values only, a separate field interprets
standard units as 100 microseconds per step. This is the V4L2 convention documented
in the installed kernel's `Documentation/userspace-api/media/v4l/ext-ctrls-camera.rst`;
it is not an independent physical measurement of actual exposure duration.

This diagnostic helps investigate the current dark/banded IR captures. A snapshot
does not prove which settings were active for an earlier recording, nor establish
illumination/projector state, correct optical decoding or exposure synchronization.
Recording context stays explicitly unknown unless captured with verified state.
The snapshot API has no setter; reversible controlled exposure experiments and
native GUI controls remain separate work.

Two tests check the ioctl UAPI layouts/allowlist uniqueness and exposure-unit
interpretation. Hardware snapshots must be reported separately from offline tests.

A post-soak hardware snapshot is saved in evidence/native-standard-controls-01.json.
All cameras report exposure 3000. The standard conversion yields 0.3 seconds,
which is not demonstrated actual sensor timing and requires reconciliation with
the observed stream rates. IR gain reports 1000. Left exposure-auto is unsupported
by QUERYCTRL; right/color return mode 2. No setting was changed.
