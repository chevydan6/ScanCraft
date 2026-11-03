# Helper diagnostics and failure isolation

An optional stderr write error must not abort acquisition, restoration or final IPC.
This milestone fixes a reproduced failure class and improves retained failure
information. The earlier unexplained native helper exit remains unproven: these
experiments do not establish that stderr caused that specific incident.

## Reproduction and correction

At base `e87c14b6d1294bdeb728191d5f5d81ee180c8674`, a device-free invalid-argument
request to either internal laser or live helper returns a structured error and
exit 1 with normal stderr. With stderr directed to `/dev/full`, both instead panic,
exit 101 and send no IPC bytes. `eprintln!` failures are therefore able to mask the
actual error and prevent the final packet. Separately, all three supervisors call
`Decoder::finish` before reporting the already observed process status, so an
incomplete packet/result can hide the exit code behind a generic protocol error.

An initial app-only correction passes the malformed-request check but fails a
successful synthetic capture: `live_stereo/runner.rs` still uses a per-pair
`eprintln!`, which panics the reconstruction coordinator when stderr fails. That
failed run is retained. The final correction uses one shared primitive for app,
workflow and device cleanup notes, including logging during unwinding. It preserves
all typed cleanup errors, required journal/transport writes and protocol validation.
The test is strengthened through the actual processing path rather than relaxed.

`scancraft-core::diagnostics::note` formats at most 4096 UTF-8 bytes per note,
including newline/truncation marker, and attempts at most 1 MiB per process.
Suppressed/failed notes are not retried. Formatting/write errors do not become
panics in this optional channel. The atomic attempt budget includes failed writes.
Library/panic-hook stderr writes are outside this bound. A blocking stderr sink
can still block; this is not an asynchronous logger or a shutdown deadline.

No required scan state is routed through these notes. Raw/pair journal failure
still prevents successful publication; stdout IPC write/flush failure still requests
cleanup. Reconstruction, calibration, frame data, numerical limits and formats
are unchanged. The primitive is independently written Rust standard-library
orchestration in the existing common core crate, with no new dependency, copied
upstream algorithm, proprietary runtime or license change.

## Private state logs

`scancraft-app/src/helper_diagnostics.rs::private_log` creates a log under:

- `$XDG_STATE_HOME/scancraft/diagnostics`, when a nonempty absolute XDG state root
  is provided;
- `$HOME/.local/state/scancraft/diagnostics` otherwise.

Relative roots, missing configuration or unsafe/unwritable state directories fail
explicitly before launching a helper. There is no silent `/tmp` fallback. The state
and app directories must be owned by the effective user and not writable by other
users; diagnostics is private (0700), and newly created logs are 0600. Existing
permissions are checked, not silently changed. Configured root/app/diagnostics leaf
symlinks are rejected; this is not a claim that every configured ancestor is free
of user-managed symlinks.

Directory descriptors anchor `mkdirat`/`openat`/`unlinkat`; leaf names are validated,
symlinks are not followed at these operations, and newly owned file descriptors
are transferred to Rust File exactly once. Startup writes a versioned JSON header,
fsyncs the file and log directory, and checks directory identity before launch.
The child inherits the stderr append description, while the supervisor keeps a
clone for its failure observation. Header/file/directory setup is checked; later
notes and failure observations are best effort and are not fsynced individually.
This is retention across process exit, not power-loss durability qualification.

The file holds an advisory exclusive lock inherited through child stderr. At the
next launch, pruning retains the newest 64 inactive recognized logs, plus the new
and active logs. A helper retaining cleanup ownership also retains this lock,
including after the GUI stops waiting. Pruning never removes locked files,
symlinks, hard-linked/nonregular files, files with incompatible ownership/mode,
unrecognized names/headers or unrelated user notes. Inventory stops at 1024 entries
and reports an error rather than scanning an unbounded directory. This is bounded
retention of inactive managed files, not a global disk cap across arbitrarily many
active processes or external stderr writers. Export important diagnostics before
later operation cycles make them eligible for pruning.

The header has `schema_version: 1`, `kind: scancraft_helper_diagnostic` and the
supervisor PID. A supervisor failure appends `kind: supervisor_failure`, helper PID,
observed status and a bounded reason prefix with a truncation flag. The record uses
the held append description without reopening the path; it survives normal GUI
closure. The displayed error also reports the current status and whether the log
path is present/unavailable. Observation-write failure is reported without replacing
the underlying error. If an external actor removes the pathname, this mechanism
does not recreate or recover it automatically.

`failure` uses nonblocking `Child::try_wait`; normal/nonzero/signal status survives
an incomplete final packet. It never infers restoration from process exit alone.
A live/unknown status explicitly leaves cleanup unconfirmed. Parent monitoring,
ordinary Stop versus Abort, ownership guards, reaping and the prohibition on killing
a scanner owner remain unchanged. Filesystem calls and existing driver/worker joins
have no established hard latency bound; SCAN-REV-005 remains separate.

## Validation

The final frozen workspace gate passes formatting, strict all-target Clippy,
**808 tests**, and release build, with one existing explicitly ignored capacity test.
Relevant tests include:

- core `diagnostics::tests`: UTF-8/truncation, exact attempt budget and failed output;
- app `helper_diagnostics::tests`: absolute state paths, private persistent files,
  symlink/mode rejection, inherited active-log protection, inactive retention,
  unknown-file preservation, inventory bound, retained status and oversized JSON;
- capture/laser/live supervisor tests: missing final result with exit 0/42, plus a
  device-free self-signaled child in the laser case;
- actual `tests/helper_diagnostics.rs` subprocesses: identical invalid-request IPC
  under normal/full stderr, prelaunch setup refusal with unchanged input, and
  successful synthetic reconstruction/raw recording verification despite full stderr.

Reproduce offline with `tools/check.sh` and focused `cargo test --frozen -p
scancraft-app --test helper_diagnostics`. The internal malformed requests never
open a device; the successful fixture is explicitly synthetic and retains that
provenance. No fault is injected into physical hardware.

Implementation `1507747` release subprocess checks reproduce the corrected error packets (laser kind 6,
live kind 4, exit 1) under both stderr sinks. Saved Raptor replay with `/dev/full`
stderr succeeds: 32 raw/camera, 4 pairs, 28 processing drops/camera, no unmatched
input, four previews and completion with zero helper omissions.

Two short healthy native GUI trials use the existing scoped line lease and private
software-renderer condition (`LP_NUM_THREADS=2`). Finite capture retains 32 raw/camera,
processes 25 pairs in 24 segments, drops 7/7, unmatched 0/0, startup rejects 2/1 and
ongoing 1/1. The processed fraction is 78.125%, below 90%; no performance gain is
claimed from this logging change. Root close saves 10/9 raw frames and 8 pairs in 8 segments, drops 1/1,
unmatched 1/0, startup 0/87, ongoing 0/0, one stopped incomplete buffer/camera. One observed
close-to-exit time is 0.292 s, not a bound. Both pass raw integrity/publication/restoration
and exact complete before/after camera-stream and illumination GET readbacks.
Original property 151 is restored; helpers own camera descriptors, the GUI owns none,
and all observed processes exit. Default-renderer and transport-reject issues remain.

All 37 records / 69,436 candidate values from these captures and the full-stderr replay
match a preserved pre-change engine/serializer, including raw CRC/sequence/flags/
host-time/PTS references. All three new artifact manifests have verified lengths/CRCs.
Native diagnostic files remain private and present after GUI exit; copies stay in
private evidence. These checks prove integrity, not independently calibrated geometry.

A device-free GUI replay to an existing output shows exit 1, the refusal reason and
real log path; the persisted supervisor record contains the same status. The app
closes normally, the helper exits, existing capture hashes remain identical, and no
scanner descriptors are observed. The screenshot is inspected. A subsequent accessible
FD inventory finds no camera/USB holders.

Private evidence and failed intermediate runs are bound in the developer response
and recovery checkpoint. Raw frames, models, screenshots, probes and runtime logs
are excluded from Git and review uploads. Original helper-exit attribution, hard
shutdown bounds, sustained reliability, physical calibration/accuracy, tracking/fusion
and full end-to-end scanning qualification remain incomplete. Independent review is
pending; no bot result or local test authorizes merging.

The follow-up encodes each supervisor observation into a complete append buffer
before file I/O, avoiding interleaving at serializer-fragment boundaries with a
live child's notes. This is a code-inspection correction, not a claim that an
interleaving incident was reproduced. Short writes/I/O failure remain best effort.
The 808-test gate is repeated successfully. On the follow-up binary, native finite
capture saves 32 raw/camera, processes 24 pairs (75%, below 90%), drops 7/7,
unmatched 1/1, startup rejects 4/1 and ongoing 0/0. Root close saves 10/9 raw,
8 pairs, drops 1/1, unmatched 1/0, startup 0/5, ongoing 0/0 and one stopped
incomplete buffer/camera. One close-to-exit observation is 0.294 s. Both pass
raw/publication/restoration and full stream/illumination readback equality.
Full-stderr saved replay retains 32 raw/camera and processes 3 pairs, drops 28/27,
unmatched 1/2, with three previews/completion and zero helper omissions.
All 35 follow-up records / 65,579 values match the preserved engine, raw references
and original serializer; all three manifests pass byte/CRC checks. GUI existing-target
error/status/log retention is rerun and inspected; its input hashes remain unchanged.
Earlier results stay attached to their implementation build, rather than being
misrepresented as repeatable rate guarantees. No scanner fault is induced.
