# Read-only stereo recording timing diagnostics

```
scancraft-cli recording-timing LEFT_RECORDING RIGHT_RECORDING MAX_HOST_DELTA_US
```

This command prints JSON to standard output without writing input files, choosing
reconstruction frames, opening a scanner or applying clock offsets. It validates
both complete recordings including CRCs, ordering, footer and EOF before producing
results. Playback's regular-file, size, index budget, identity and timestamp guards
apply, including final source checks. It requires matching dimensions, frame rate
and packed GREY profiles on interfaces 0 (left) and 2 (right). Different frame rates
or non-IR profiles are rejected rather than implicitly compared. It supports the
existing recording versions and inherits their integrity limitations.

For each left frame, binary search identifies the nearest recorded right host
value. Candidates outside the caller's inclusive microsecond limit are counted;
equal-distance ties are counted separately and never arbitrarily resolved. There
is no default acceptable exposure skew. A unique nearest value is unique only for
that left frame: the report separately counts right frames reused by multiple left
candidates. This is not a global one-to-one matching or optical correspondence.
The first 100 unique in-limit candidates include indices, sequence numbers, both
host values, absolute host difference, temporal ordering and raw sensor PTS. Counts
cover all frames; `samples_truncated` explicitly identifies limited output.

Raw PTS equality is counted only for unique in-limit candidates. No PTS subtraction,
unwrap, cross-camera tick-frequency conversion or clock-offset estimation occurs.
A different clock epoch can make nearest-host results meaningless. The recording
format does not establish common clock domains or exposure phase. Both host values
and PTS are observations, not calibration. Every report explicitly keeps
`clock_domains_verified`, `synchronization_verified` and
`physical_accuracy_qualified` false, even for zero delta or equal raw PTS.

Implementation: `scancraft-recording/src/indexed.rs` caches `FrameTiming` while
performing its existing full validation; `recording_playback.rs` exposes these
historical observations under source guards; `recording_timing.rs::inspect` and
its local `associate` implement profile checks and nearest-value diagnostics.
`scancraft-cli/src/main.rs` provides the command. Independently implemented standard
binary search and absolute-distance comparison, MIT project code; no upstream port
or new runtime dependency. Runtime comparison work is O(L log R), not all pairs.
No throughput or real-time performance claim is made.

The index now retains an inline sequence/host/PTS struct per frame (for all indexed
playback users). Diagnostics additionally retain right timing values and one u32
usage counter per right frame; output is bounded to 100 candidates. Both index
counts are bounded by the existing one-million-frame limit. This adds memory;
actual allocation layout and peak process memory are not benchmarked. Payloads are
not retained in these timing collections or reread for association. Cached values
are historical; they are not fresh cryptographic evidence that file bytes remain
unchanged. Existing filesystem guards are not adversarial authenticity guarantees.

Tests cover endpoints, u64 extremes without subtraction overflow, inclusive limits,
equidistant ties, candidate reuse, output bounds, cancellation, full corruption
rejection, profile mismatch and unchanged inputs. A separate exhaustive-distance
oracle checks every candidate/count on a synthetic 500-by-180 timestamp case at
four limits. The index regression verifies cached times remain unchanged after
out-of-order seeks and read no further bytes. Native CLI integration verifies JSON,
false qualification flags, unchanged input bytes and invalid-limit rejection.
These tests do not qualify genuine camera clocks or exposure synchronization.

This tool supports investigation before manual frame selection with
`reconstruct-recordings`; it deliberately does not feed candidates automatically
into geometry. Genuine optical content, mode/calibration binding and independent
physical measurements remain necessary for a qualified Raptor scan.

The existing reader already requires valid PTS for every stored frame, including
v1/recovered-prefix inputs; timing caching does not introduce that restriction.
A CRC-valid adversarial regression covers missing PTS, empty metadata and malformed
UVC headers for both versions and also passes on the isolated pre-feature source.
See developer review responses for the disputed bot compatibility claim.
