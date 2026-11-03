# Finite stereo completion and USB truncation investigation

This change addresses a specific finite CLI lifecycle hazard: one camera could
close and restore its stream settings while its peer was still acquiring the
requested number of frames. It does **not** fix or qualify the continuing paired
truncations, startup behavior, live IR/laser pipelines or physical reconstruction.

## Observations before changing code

Native `stereo-probe 1200 --fps 30` ran with the existing release binary, no
concurrent compilation, and full stream/illumination GET snapshots before and
after. Both cameras returned 1,200 accepted frames. One camera reported 89 ongoing
rejects versus five on its peer. A second run with a read-only trace of the already
available USB bus-4 monitor reported ten versus 95. All reported settings restored.
No firmware, module loading, kernel parameter or permanent device change occurred.

The second run's trace retained 909,688 bulk submission/completion events, with
no unmatched tags or pending submissions at its end. The only ten nonzero bulk
completion statuses were -2 during stream shutdown. Monitor drop statistics were
not retained: balanced tags do not prove that no events were lost. The trace is
private, including all captured payload prefixes and device addresses.

Nine of ten continuing, paired short-frame observations had V4L2 byte counts
exactly matching the sum of successful same-PTS USB completion lengths minus the
observed 12-byte headers. One observation disagreed and remains unresolved. This
is a diagnostic partial-header analysis, not a replacement UVC decoder. It supports
investigating missing data before application validation; it does not establish
whether firmware, the controller, transport or the kernel is responsible.

In addition, the second camera's retained sample contains at least 22 tiny invalid
buffers after the first camera's USB cancellation and before the second finished.
The first such retained rejection was approximately 33 ms after the first stop
under a timestamp-offset estimate from accepted frames. Offsets are diagnostic,
not qualified clock synchronization. The bounded sample does not classify every
one of the 95 rejects. Source inspection found independent `camera.close()` calls
inside each finite worker; this motivates the controlled completion experiment.

Linux [usbmon documentation](https://docs.kernel.org/usb/usbmon.html) describes
these events as driver/host-controller requests, rather than guaranteed physical
wire transactions, and notes that captured data may be shorter than reported
transfer lengths. Those limits apply to this analysis.

## Correction and ownership

`scancraft-cli/src/stereo_probe/completion.rs::Completion::wait` is a two-worker
completion gate. Each finite acquisition retains its camera on its original worker
thread until both workers have collected their requested valid frames. Only then
may either call `camera.close()`. There is no unsafe Send implementation, source
transfer across threads, new dependency, validation relaxation or discarded error.

`stereo_probe.rs::run_workers` supplies the shared gate and existing cancellation
flag. Peer errors, panics and thread-spawn failures already set that flag; a waiting
worker checks it and the scoped termination request every wait iteration. Waiting
uses a 10 ms park interval, not a shutdown deadline. A driver call or cleanup that
blocks can still hold both ownership and the outer join; SCAN-REV-005 is not fixed.
Frames physically produced after a worker reaches its requested count are not
observed while it waits; the report does not claim a full exposure census.

This affects the finite CLI stereo probe/record/flood/line/gain paths sharing
`capture`. It does not change `live_stereo::runner` or `laser_stream::runner`.
Those lifecycles need separate analysis before applying an equivalent change.

## Controlled hardware comparison

The old executable was preserved before building the correction. A declared
new/old/old/new order used 1,200 accepted frames per camera at 30 FPS, unchanged
validation, the same scanner/link, and separate state restoration checks. Each
process exited normally, with zero reported missing metadata and identical before/
after stream and illumination readbacks.

| Trial | Code | Startup rejects L/R | Ongoing rejects L/R |
| --- | --- | --- | --- |
| Initial baseline | old | 82 / 1 | 89 / 5 |
| USB-traced baseline | old | 0 / 82 | 10 / 95 |
| Counterbalance new-a | completion gate | 67 / 0 | 7 / 8 |
| Counterbalance old-a | old | 1 / 0 | 9 / 8 |
| Counterbalance old-b | old | 0 / 81 | 5 / 95 |
| Counterbalance new-b | completion gate | 2 / 0 | 8 / 8 |

This supports eliminating the finite peer-teardown overlap. The old code did not
produce a large burst on every trial. It is not a statistical reliability guarantee
or an attribution of every rejected buffer. Even the corrected continuing rates
are about 0.58–0.66%, above the <0.1% target; startup counts also remain included
in the full acceptance proxy. No errors were reclassified away to obtain a pass.

The four counterbalance processes sampled approximately 26.9–27.2 MB peak RSS and
0.68–0.76 CPU seconds over about 40.8 seconds wall time. These are 250 ms process
samples, not kernel/whole-system resource measurements, latency benchmarks or
60-minute memory qualification. No meaningful CPU speedup is claimed.

## Verification and limitations

- Full frozen gate: 831 passing tests, one existing ignored capacity test,
  formatting, strict all-target Clippy and optimized workspace build pass.
  Three new completion tests cover waiting for the peer, cancellation without a
  second arrival, termination errors and excess arrivals. They do not simulate an
  uninterruptible driver or prove a cleanup deadline.
- Three fresh finite line captures save 16 raw frames per camera each (96 total).
  All six recordings pass full CRC/footer replay; each trial has 15 equal-raw-PTS
  candidates within the specified 1,000 us host window. This does not prove
  simultaneous exposures. The existing temporary line control restores its prior
  state, and all full stream/illumination readbacks match before/after.
- Extracted paired frames retain original data; the last right view was visually
  inspected and contains visible projected stripes and bright spots. This is
  signal evidence, not calibrated surface identity or dimensional accuracy.
- The first private replay harness incorrectly expected `report.json`; finite CLI
  capture uses a manifest and stdout report. The successful capture and error log
  were retained; the corrected harness resumed validation without recapturing or
  overwriting the first input. This was a harness error, not a capture failure.
- No fresh GUI or calibrated reconstruction trial was performed. The old IR
  1,000-frame limit, live pipelines, disconnect recovery and sustained acceptance
  remain separate work. No qualified physical object or mesh is claimed.

Private reproducible evidence: `first-scan-integration-ao3yvkv3`, including
`native_probe{,_initial}.py`, declared `counterbalance-plan.json`, both gate logs,
all native/state reports, `usbmon-analysis.json`, `analyze_usbmon.py`, retained old
executable hash, and `record_trials{,_resume}.py`. No trace, raw scan, QR data or
factory parameters are committed or uploaded. Source/binary/commit bindings are
recorded at the milestone checkpoint. CodeRabbit doctor passes authentication and
reachability, but usage lookup still cannot establish quota/billing safety; no
paid review or review pass is claimed.
