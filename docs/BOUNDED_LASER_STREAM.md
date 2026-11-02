# Bounded laser frame handoff

Status: reusable workflow component, tested with synthetic sources and saved
Raptor frames. The [native CLI adapter and verified bundle publisher](NATIVE_LASER_CAPTURE.md)
now use this entry point. Live GUI controls remain pending. This does not provide qualified live scanning, tracking or fusion.
The existing IR live path remains unchanged, including its 1000-frame/session cap.

## Interfaces and ownership

`scancraft_workflow::laser_stream::Pipeline::new` borrows explicit stereo geometry
and `laser_pair::Settings`, deriving first/second GREY profiles in that configured
interface order (0/2 or 2/0). It neither activates calibration nor changes scanner
settings. FPS is a nonzero declaration; a hardware adapter must validate actual
supported configurations and own session locks, group settings and restoration.

`Pipeline::run` constructs two source owners using the existing generic
`live_stereo::runner::Source` contract. The supplied factory also receives a
`StopToken`, which combines owner-requested graceful stop, WorkControl cancellation
and internal failure. Source implementations should poll it during frame waits;
a token cannot interrupt blocking driver, filesystem or foreign adapter calls.
Sources are constructed inside their owning threads rather than transferred.

Each valid frame is appended through a required `Journal` before processing
admission. Payload validation and a 4096-byte metadata cap precede journaling and
queueing. Invalid input fails that source; it is not reported as a recorded frame.
A full nonblocking four-frame channel omits processing of that frame while raw
retention continues. `Writer<BufWriter<File>>` implements Journal with existing
recording validation/CRC/footer/flush and final fsync. Successful push means the
journal accepted a frame; power-loss durability is not established until successful
finalization. Adapters provide safe no-overwrite paths, source identity, context,
recovery and publication. The workflow never publishes a bundle itself.

A single consumer associates frames and invokes the unchanged in-memory laser
processor with a no-op full-query sink. Every completed pair produces an owned
compact report or an explicit processing/admission error to the supplied callback.
Per-query errors and ambiguity remain inside successful reports. Per-pair processing
failures are counted and do not stop raw capture. A required callback error/panic
stops the run; optional preview consumers must account for omissions and return
success instead of turning display failure into recording failure (SCAN-REV-018).
The callback can inspect original paired frames and retained-journal indices.

Sources close after journal/source failures or panics. Graceful stop finalizes
journals and drains queued processing. WorkControl cancellation aborts processing,
signals sources and still joins owners. Structured `Failure.partial` preserves
available source counts, outcomes, finalization/cleanup status, pending frames and
queue delivery gaps. Unknown gaps after a missing thread report are `None`.
No cleanup, fsync, source call or callback hard deadline is claimed. SCAN-REV-005
remains open; this function belongs inside a supervised helper for a native GUI.

## Pairing and memory policy

`laser_stream/association.rs::Association` retains at most two pending samples per
camera, the previous index/host/sequence/counter per camera and the last paired
indices. Strict forward modular sequence/PTS increments must be below half the
u32 range. The first camera anchors zero; the second initially uses the nearest
observed peer counter, rejecting an exact half-range offset. Signed extended ticks
then prevent an established pending old-cycle value matching the same raw PTS in
a later observed cycle. Host-window and strict pair-order checks remain mandatory.

This is an explicitly inferred counter policy, not proof of clock frequency,
exposure synchronization or resets. Large gaps, skipped full cycles and ambiguous
initial alignment are not physically qualified. If queued samples advance by half
the counter range or more, association fails rather than guessing. No session-wide
PTS set, cloud history, query JSON or result list is retained by the runner.

Maximum frames is a nonzero u64 limit and does not size storage. The runner holds
at most four queued frames, four pending frames, one currently processed pair and
one frame being handled by each producer, with bounded per-frame payload/metadata.
Recording serialization scratch, source buffers, detector/matcher allocations,
caller-held callback results, terminal adapter statistics, disk usage and allocator
overhead are separate. No measured worst-case RSS or 60-minute stability guarantee
follows from constant pairing state. Raw recording disk use grows with duration.

Processing runs on the association consumer, so slow processing can fill the raw
handoff queue and omit individual camera frames. Counts expose queue drops,
pending evictions, rejected host pairs, pending leftovers and queued-but-undelivered
frames. This can reduce pair yield under load; a measured future opportunity is
pairing before expensive processing and dispatching complete pairs. Do not hide
this loss or infer 90% live processing from a short successful replay.

## Validation and provenance

Eight added tests cover 4000 candidate pairs across repeated counter wraps,
ambiguous ordering/initial offsets/established epochs and host windows, a 1200-pair
barrier-controlled synthetic pipeline with complete direct-engine report equality
and recording CRC roundtrips, deterministic overload retention, processing admission
failure, consumer error/panic, source/journal/finalization/cleanup failures,
oversized metadata, and pre/inflight cancellation/graceful stop. Barriers establish
test boundaries; they do not simulate a real camera clock or hardware throughput.
Initial test compilation required Debug on terminal reports; an initial strict
Clippy check caught a test nested-if. Both corrected without disabling checks.
Final gate: 746 tests, formatting, strict all-target Clippy and release build pass.

Private replay uses nine previously saved rotating-pose pairs. Consumer-paced
replay delivers all nine, exactly matching every prior compact engine report.
Both required file journals retain all nine original frames, passing full Reader
CRC/footer checks and frame byte/metadata/flags/timing equality. An unpaced replay
also retains all nine frames per camera; processing omissions are reported rather
than silently discarding raw frames. Replay timing includes cloned source frames,
file journals and test comparisons; it is not a live FPS or latency benchmark.
Private replay sources themselves hold a small selected dataset in memory, so their
memory consumption must not be confused with native streaming-source memory.

New association/concurrency code is independently written Rust. Source ownership,
recording format and laser computation reuse ScanCraft's existing interfaces from
1904d7ed06ef51547d9229a50290d3d9952f733b. No proprietary or upstream source, dependency,
firmware or physical calibration change is included. All geometry, units,
projector identities and synchronization remain unqualified. Full mission gates,
long-session GUI/session SCAN-REV-021 and independent automated review remain open.
