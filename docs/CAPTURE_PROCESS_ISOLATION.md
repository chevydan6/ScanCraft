# Capture process isolation: development contract

Status: preview/recording helper implemented; native live-scan helper not implemented.
Independent SCAN-REV-005 remains open. This document does not establish a bounded
whole-application shutdown or hardware qualification.

## Existing ownership

The preview GUI starts `capture_worker::Worker`, whose closure invokes
`capture_process::run`. That supervisor starts the same executable with an
internal `--capture-helper` entry point before eframe initialization. The helper
invokes `capture_session::run`, which owns the session lock, scanner capture,
recording writer and final publication. GUI code owns the bounded two-frame
preview channel and display-drop counters. Recording precedes preview delivery;
display backpressure does not skip saved raw frames. Cancellation and consumer
disconnection both stop the capture loop at its existing frame boundary.

`capture_session::with_close` explicitly invokes device close after ordinary
operation errors, including recording setup/write errors. It preserves the
operation failure and any close failure together. Final recording publication
requires both operation and explicit close success. Panics still use owned
resource Drop and the existing worker panic-reporting path; panic cleanup is not
newly qualified. Device calls, file I/O and close remain synchronous inside the child. Parent
process spawn/log-file operations and scheduling are not assigned a hard real-time
bound.

## Implemented preview process boundary

`capture_ipc` transports raw frames with a 64 MiB body limit and a 24-byte
fixed header plus metadata and image bytes. Completion uses bounded JSON (64 KiB).
Each frame is explicitly flushed before returning to capture, so a buffered
tail does not depend on the next capture. Flush errors are transport failures.
The decoder accepts fragmented reads, rejects inconsistent lengths, unknown
message kinds, truncation and bytes after completion. This is an internal protocol
between copies of the same executable, not a new recording format.

During preview capture, only the child opens the scanner and holds session locks. Its control thread
interprets stdin EOF or any control byte as a graceful stop request. The parent
reads stdout nonblocking in bounded batches, checks cancellation between batches
and requests stop by closing stdin. It waits up to two seconds after observing
cancellation, then returns an explicit cleanup-unconfirmed failure. This interval
is a software grace policy, not a measured physical driver shutdown bound.

A reaper thread is allocated before helper launch. On parent-monitor exit it
receives the child handle and waits without joining the GUI or owning any device
restoration duties. The child retains actual device handles and locks until its
session cleanup finishes. No kill, USB reset or abandonment of a device-owning
thread is used. A helper blocked in kernel I/O may outlive the GUI; replacement
capture attempts must still acquire the existing session locks.

Each launch creates a private, exclusive, no-follow diagnostic file (mode 0600)
in the temporary directory. The helper writes its capture/cleanup outcome and
transport error to stderr independently of stdout IPC. Failure messages retain
its PID and diagnostic path. A helper that never returns cannot supply a final
cleanup outcome; timeout must remain unconfirmed. Disk failure/power-loss
persistence is not qualified. Successful completion requires the final message,
valid stream termination and successful process exit. Capture errors keep the
window open and show cleanup diagnostics rather than claiming the scanner stopped.

The native live-scan UI separately runs `scancraft_acquisition::scan` and joins
its worker on Drop. Moving only preview capture cannot establish bounded shutdown
for that live-scan path. Existing ordinary-close handling retains worker ownership
and keeps the UI responsive while awaiting cleanup, but its wait is unbounded.

## Required process boundary

The helper must exclusively own device descriptors, session locks and restoration
duties. The GUI must own no device handles and must not synchronously wait for a
helper blocked in setup, capture, close or restoration. IPC must have explicit
length limits, bounded display queues and cancellation/disconnect semantics.
Recording must remain independent of display throughput. A helper's completion
message alone is insufficient if device cleanup can still occur after it.

Parent cancellation requests graceful cleanup. Timeout or parent exit must never
be reported as successful restoration. A helper that remains in an uninterruptible
kernel call may survive the GUI; it must retain its locks until descriptor cleanup
actually finishes, preventing a replacement session from racing it. Merely
sending a kill signal does not establish that the process or kernel operation has
terminated. Cleanup failures and incomplete recording locations need durable,
private diagnostics when the GUI exits before the helper.

Both preview recording and native live scanning need the boundary. Preserve
current raw recording formats, partial-recording recovery, publication ordering,
control restoration and independent finding status. No forced scanner reset,
firmware change or weakening of validation is part of this work.

## Required validation before claiming isolation

Use real child-process tests with no scanner: stalled setup/read/close, parent
cancellation and exit, truncated/oversized IPC, full display queues, simultaneous
capture/write and restoration failure, helper panic/crash, lock retention after
parent timeout, and restart only after cleanup. Verify UI event polling and Drop
do not join a device-owning thread. Distinguish measured synthetic lifecycle bounds
from later physical disconnect/driver-stall qualification. Physical tests require
the permitted hardware environment; they have not been performed here.

## Validation to date

Device-free tests exercise protocol fragmentation and corrupt sizes, missing or
trailing completion data, actual child processes producing frames/completion,
nonzero exits, cancellation while a child remains stalled, and private diagnostics.
An actual child inherits an exclusive lock and waits for a release marker: the
supervisor worker finishes with an unconfirmed error while the lock remains
held; after child cleanup the lock becomes acquirable. Test teardown releases
the marker even on assertion unwind. No scanner, USB node, firmware or calibration
is touched by these fixtures. Native live-scan Drop remains synchronous and
SCAN-REV-005 must not be closed from these preview-only results.
