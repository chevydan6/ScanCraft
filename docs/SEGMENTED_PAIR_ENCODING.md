# Buffered segmented pair encoding

Experimental performance work on the existing schema-2 writer. Baseline:
`285f1914e392ff0f48b62d158bf85ab20804d94d`. This does not establish calibrated
geometry, tracking, reconstruction quality or sustained scanning.

## Change and preserved boundaries

`laser_stream/pair_journal/write.rs::Journal::append` reuses the existing
`bundle/io.rs::pair_record` helper to coalesce serializer fragments in a fixed
64 KiB buffer before passing them to the byte-limited in-memory encoder. The
helper already supplies newline termination, successful flush and explicit
discard of pending bytes on a returned error. No replacement serializer,
dependency, unsafe code, new thread, queue or on-disk format is introduced.

The encoder checks `WorkControl` before every resulting sink write, replacing
its earlier per-fragment byte counter. Small writes are grouped; a large serializer
slice can bypass the buffer and still faces the existing byte-limit check before
extension. Cancellation also remains checked before encoding and afterward.
This is cooperative cancellation, not preemption of an arbitrary serializer,
filesystem or scheduler. Callback counts and exact observation instants need not
match the old implementation; accepted complete bytes do.

Logical encoded length remains bounded by `maximum_record_bytes`, including
newline. The existing Vec may reserve additional capacity; allocator overhead is
not a process-memory guarantee. The added buffer has fixed 64 KiB capacity for
one append, with no retained history. Serialization, quota or cancellation errors
discard temporary encoding before touching the active segment/index prefix and
poison the writer as before. File-write failures retain the existing incomplete
prefix policy. Required records still flush before optional preview handoff.

Whole-record rollover, segment/directory/index fsync, aggregate and segment quotas,
available-space admission, CRCs, ordering, final verification and atomic publication
are unchanged. In particular, there is no reduction of the checkpoint frequency
for a given request. Legacy schema-1 encoding already uses this helper and is
unchanged.

This is reuse of ScanCraft's independently written Rust implementation, introduced
in `fdd02f2551fe606d1cd38c158c5fb9219f8b270d`, with the standard library BufWriter
and existing serde_json. No third-party source was imported. Existing project
licensing and dependency notices apply.

## Measurement method

Private release probes first measured unchanged production numerical processing
and copied exact baseline journal/CappedFile bodies, then tested buffering before
promotion. Five saved Raptor pair ordinals (0, 4, 9, 14, 18) provide complete typed
reports and raw references. Each path runs thirteen times, excluding the first
warmup, alternating old/new append order in one binary. Complete record bytes,
every segment and the entire final index are compared outside the timers.

The separate instrumented pass measures encoding, rollover/checkpoint/open, and
final segment-write/flush phases. The first implementation probe omitted private
Digest field access; an isolated mirror corrected the harness without changing
production source. Its failed build log is retained. Probe-only unused-import
warnings are not represented as production gate warnings.

Numerical engine timing includes validation, detection, trace/admission preparation,
matching and compact report generation, with real WorkControl checks. A Sink::begin
timestamp separates preparation from subsequent query/report work; it is not a
direct measurement of individual mathematical kernels. Record timers exclude model
and raw-file loading, file creation before append, JSON result checking, GUI and
source verification. Append includes rollover checkpoints when they occur; finish
is measured separately. Shared warm host, Rust 1.99.1, no CPU/frequency isolation;
copied-module compiler context and instrumented timing overhead limit inference.

The 2 MiB segment policy deliberately exercises rollover: roughly 1.1 MiB records
require almost one segment each. A separate 16 MiB policy comparison distinguishes
configuration-dependent checkpoint costs from the code change. Different policy
results must not be attributed to buffering or used to imply weaker durability
rules for the same request. No production defaults are changed.

## Regression checks

Two tests in `pair_journal/encoding_tests.rs` cover multi-buffer escaped Unicode,
exact complete-record admission and one-byte-short rejection (including newline),
writer poisoning, a late serialization error after more than one buffer, and
latched cancellation during serialization. Rejected records leave the segment and
index bytes unchanged; recovery sees only the preceding complete record.
Existing rollover, CRC/order/recovery, required-callback failure, raw preservation,
preview ordering, write/sync fault and no-overwrite tests remain required.

```sh
bash tools/check.sh
cargo test --frozen -p scancraft-workflow laser_stream::pair_journal
cargo test --frozen -p scancraft-workflow \
  segmented_journal_crosses_old_one_gib_limit_with_bounded_live_state -- --ignored
```

The last test writes and validates more than 1 GiB; use a temporary directory with
sufficient space. Passing it is a storage-capacity regression, not a duration,
memory-stability or physical scanning qualification.

## Final measurements and native checks

The final probe directly imports the actual production `write.rs`; the exact
baseline writer and shared CappedFile are compiled in separate local namespaces.
No prototype timing is substituted for the following append medians (milliseconds).
Each 2 MiB journal contains thirteen segments; each 16 MiB journal contains one.
All ten old/new journal comparisons preserve every complete output file exactly.

| Saved pair ordinal | Record bytes | Old / current append, 2 MiB segments | Old / current append, 16 MiB segments |
| --- | ---: | ---: | ---: |
| 0 | 1,110,255 | 5.590 / 5.256 | 2.767 / 2.083 |
| 4 | 1,086,415 | 5.772 / 4.725 | 2.724 / 1.667 |
| 9 | 1,130,802 | 5.913 / 4.842 | 2.829 / 1.726 |
| 14 | 1,072,236 | 5.808 / 4.623 | 2.680 / 1.623 |
| 18 | 1,134,188 | 6.018 / 4.840 | 2.852 / 1.728 |

Separate instrumented observations place baseline encoding around 2.6–2.7 ms,
buffered encoding around 1.4 ms (about 1.8 ms on the first pair), and tiny-segment
rollover around 3 ms. Those phases do not sum exactly to medians of total calls.
Final production numerical processing is 27.9–30.3 ms in the 2 MiB pass; the earlier
instrumented preparation/query-report split is about 9.2 / 18.8–21.3 ms.
Numerical code is unchanged. Ordinary Vec serialization is a comparison workload,
not an acceptable replacement for bounded/cancellable admission.

Final sealing of each thirteen-record journal takes about 3.0–3.6 ms with 2 MiB
segments, versus 6.9–7.5 ms with 16 MiB segments, outside the append medians.
Larger segments amortize checkpoints but increase the unindexed tail between
checkpoints and change power-loss exposure. No power-loss guarantee is inferred.

The full gate passes **814 tests**, formatting, strict all-target Clippy and release
builds, with one deliberately ignored capacity test. That test was then explicitly
run and passed: 9,000 synthetic records cross 1 GiB and stream back completely.
This debug test took 57.50 s here; it is not a release throughput benchmark.

Actual native GUI order was baseline/current/current/baseline, with unchanged
2 MiB segment requests and 64 raw frames saved per camera in every finite trial:

| Trial | Pairs | Queue drops, cameras 1 / 2 | Pending | Startup rejects | Ongoing rejects |
| --- | ---: | --- | --- | --- | --- |
| Baseline 1 | 54 / 64 | 10 / 9 | 0 / 1 | 1 / 88 | 1 / 0 |
| Current 1 | 57 / 64 | 7 / 7 | 0 / 0 | 2 / 101 | 1 / 1 |
| Current 2 | 58 / 64 | 6 / 6 | 0 / 0 | 91 / 1 | 2 / 2 |
| Baseline 2 | 55 / 64 | 9 / 9 | 0 / 0 | 0 / 1 | 1 / 1 |

Current fractions are **89.0625% and 90.625%**: one remains below 90%, and neither
is a sustained-rate qualification. Sequential scenes/scheduling are not identical
measurements; a universal or fully isolated application speedup is not claimed.
All startup/ongoing rejects are retained in V4L2-buffer units, not treated as
physical-exposure counts. Their frequency still fails to establish reliability.

A separate current-build trial changed only the explicit segment limit to 16 MiB:
61/64 pairs (**95.3125%**), queue drops 3/3, no pending pairs, startup rejects 92/1,
ongoing 0/0. This combines the implementation with a distinct checkpoint policy.
It is one short observation; no default, corruption, sustained-rate, preview-FPS,
latency, RSS, calibration or tracking gate is declared passed.

An active-window-close trial saves 9 raw frames/camera and 9 pairs, zero queue
drops/pending, startup rejects 1/1, ongoing 0/0 and one stopped incomplete buffer
per camera. One close-to-exit observation is 0.324523 s, without a deadline claim.
All six trials exit successfully, pass raw verification/publication, and restore
all measured stream/illumination GET readbacks. Helpers own camera descriptors;
the GUI owns none; all observed processes exit and the final accessible device-FD
inventory is empty. Saved and active screenshots were inspected. The authenticated
private X11/xdpyinfo readiness and LP_NUM_THREADS=2 software-renderer condition are
explicit; default-renderer acquisition is not qualified.

The frozen earlier engine/serializer checks every new bundle: **294 complete
records and 558,364 candidate values**, exact raw references and complete old
serialized record bytes. All six artifact manifests pass lengths/CRCs. Same-method
equivalence and diagnostic laser lines are not calibrated geometry or fusion.

Private evidence, source/binary hashes, all probe revisions/failures and raw data
remain in developer recovery artifacts `pair-encoding-profile-awtjuh9k` and are
excluded from Git/review uploads. Fresh CodeRabbit quota/billing/credit lookup is
unavailable, so there is no automated review pass or review upload. Independent
review of this optimization is pending. Findings 005/006/007/021 remain unchanged;
the original native early-exit cause is unproven. No firmware, physical calibration,
paid service, protection bypass, push or merge is part of this milestone.
