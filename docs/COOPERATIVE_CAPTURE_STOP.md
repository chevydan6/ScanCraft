# Cooperative native capture stop

`Capture::next_frame_until` accepts a stop predicate and returns `None` for a
cooperative stop. The native live adapter passes its shared Control stop flag.
CLI signal handling sets both WorkControl cancellation and the stop flag: it
still aborts publication, rather than converting an interrupt to a saved scan.
Existing `next_frame` callers retain the no-stop interface.

Before each new video/metadata dequeue, the wait policy checks stop and caps the
poll timeout by the remaining eight-second validated-frame deadline. Previous
metadata waits each used a fresh 200ms timeout without that remaining-time cap.
A completed read is retained even if stop arrived during it; timeout/interrupted
errors can become stop only when stop is requested. Disconnect and other errors
are preserved. If a video buffer is abandoned because stop arrives before matching
metadata, `stopped_incomplete_video_buffers` records that event in native live
source statistics. It is distinct from validated raw frames and processing drops.

This is cooperative behavior between driver calls. It cannot interrupt setup,
DQBUF/QBUF ioctls, ioctl EINTR retry, close/restoration, blocked filesystem I/O or
worker joins. Poll timeouts and the nominal frame deadline are not whole-session
shutdown bounds. SCAN-REV-005 remains Open; process isolation and physical stall/
disconnect qualification are still required before claiming bounded shutdown.
No thread is detached or device ownership abandoned by this change.

Three hardware-free policy tests cover stop/expired-deadline read prevention,
remaining-time caps, stop during timeout, successful data retention and disconnect
error preservation. The full frozen gate passed 579 Rust tests, formatting, strict
all-target Clippy and release build. These tests inject read closures; they do
not exercise V4L2 or establish physical stop latency. No scanner was available
for runtime qualification. No frame validation threshold, firmware, illumination,
calibration, gain or mirror setting is changed.

Sources: device/wait_policy.rs, Capture::next_frame_until in device/lib.rs,
acquisition::Native::next/statistics and CLI live_scan::run.
