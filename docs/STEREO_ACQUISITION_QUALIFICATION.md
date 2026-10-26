# Native dual-IR sustained acquisition qualification

This experiment addresses the duration and structural-buffer parts of the device
acceptance gate. It does not establish a fully validated scanning mode, exposure
synchronization, calibration, optical usability, physical accuracy, preview
latency, reconstruction throughput or cold initialization/reconnection reliability.

## Method

```
python3 tools/check_stereo_soak.py NEW_EVIDENCE_DIRECTORY
```

The default runs the existing release Rust `stereo-probe` with 18,100 accepted
buffers per IR camera at 1920x1200 GREY / 30 FPS. Each camera opens once and streams
concurrently. At ideal rate this exceeds 600 seconds between first and last
accepted samples. No image payload is stored. The Rust path validates payload
size, UVC metadata/PTS and kernel flags, matches metadata by exact sequence, checks
accepted sequence/host/sensor timestamp order and counts rejected buffers/gaps.
No projector, firmware or calibration commands are issued by this trial.

The runner records binary/source hashes, platform, process exit, two-second sampled
RSS, timing samples and native progress. The currently installed environment is
recorded separately. A new output folder is required. Runtime is bounded; errors
retain diagnostics. The current runner handles SIGTERM/SIGINT and terminates its
own child in cleanup. The original run's exact runner source is retained as
`runner-used.py` when the analysis/cleanup tool is improved while acquisition is
already running; `run.json` binds its hash. Later review uses a separately hashed
analyzer and preserves the original assessment.

Duration uses the **accepted timestamp span of each camera**, not total process
wall time or a reported nominal rate. The strict buffer-rejection proxy threshold
is less than 0.1% of accepted plus explicitly rejected V4L2 buffers. Missing
metadata is counted separately and is not in that denominator. Sequence gaps are
also separate and may include already counted rejected buffers; they are not
summed to invent an independent exposure loss estimate. All startup rejects are
included; there is no warmup exclusion. Raw PTS agreement is diagnostic only.

```
python3 tools/check_stereo_rejection_audit.py NATIVE_REPORT_JSON NEW_AUDIT_JSON
python3 -m unittest discover -s tools -p 'test_stereo_*.py' -v
```

The audit summarizes nearest-rank accepted host-interval quantiles and only the
bounded first rejection samples retained by the native driver. Length deficits,
flags and reasons apply to that sample, not all rejected events. Matching raw PTS
fields from rejected metadata do not establish valid image/exposure pairs or a
transport root cause. Neither analyzer promotes a successful narrow trial to the
full device or 60-minute memory gate.

Ten offline tests exercise duration from samples, exact threshold boundaries,
separate metadata/gap accounting, wrapping sequence/PTS, malformed accepted order,
sample truncation, ambiguous raw-PTS matching, qualification flags and cancellation
of an owned fake child. They do not replace the real hardware run.

## Interpretation limits

The rejection counter is a buffer-level proxy: short/error buffers can be pieces
of an exposure, and a sequence gap is not necessarily a lost physical exposure.
No ignored flags, permissive payload checks or suppressed errors are used to
improve results. A duration pass with a rejection-proxy failure remains a failed
full device gate. Sampled RSS can miss a short allocation peak, and bounded stored
timestamp arrays are expected to touch additional pages as the run advances.
Ten minutes of observations cannot establish the required 60-minute workload
memory qualification. Existing user GUI/Wine environments are preserved.

## 2026-10-26 result

Evidence: `evidence/stereo-ir-soak-20261026/`. Fedora 44, Ryzen 9 6900HS,
USB 3.0 link reported at 5000 Mbit/s, patched UVC module file reported by modinfo has SHA256
`579c7633261b27cfd76af16f4b1ad57c1306ce6c7d75ca914e15693e3ec127e5`.
Native process exit 0; no restart or crash. Rust-reported elapsed time is
606.958 s; monitor wall time including polling is 608.096 s.

| Measurement | Left IR | Right IR |
| --- | --- | --- |
| Valid accepted buffers | 18,100 | 18,100 |
| First-to-last accepted span | 606.442611 s | 606.442613 s |
| Accepted buffer rate | 29.84454 Hz | 29.84454 Hz |
| Rejected buffers / accepted sequence gaps | 92 / 92 | 92 / 92 |
| Rejection proxy | 0.505717% — FAIL | 0.505717% — FAIL |
| Missing metadata / duplicate PTS counter | 0 / 0 | 0 / 0 |
| Accepted interval p99 / max | 34,186 / 66,792 us | 34,181 / 66,808 us |
| >=600 s duration check | PASS | PASS |

The complete report has 18,098 exact accepted sensor-PTS intersections. This is
not exposure synchronization qualification. Ten-minute duration passes, while
the full device gate remains failed/incomplete because the rejection proxy
exceeds 0.1% and initialization/reconnection/optical evidence remains incomplete.
The largest two-second sampled RSS is 26,932 KiB; short peaks can be missed, and
this is not the 60-minute memory gate.

All first 32 retained rejection samples per camera have metadata/video error flags
and undersized video payloads; all metadata lengths are 22 bytes. Thirty of 32
payload deficits on each camera are multiples of 61,380 bytes (5 x 12,276 data
bytes). The retained sample covers approximately 235 seconds and has 31 shared
unique raw-PTS values, with right-minus-left host timestamp differences of 3–80 us.
These observations support investigating a common truncation mechanism; they do
not identify firmware, kernel, controller or scheduling as the cause. The
retention cap prevents extrapolating these exact reason/length counts to all 92
rejected buffers. No error flags or sizes are ignored.

A subsequent short warm reopen produced 10 valid buffers per camera in 0.922 s,
but rejected **111 left buffers and one right buffer**. The first accepted left
sequence is 110, demonstrating a substantial initial burst of rejected fragments;
there is also one gap after accepted acquisition begins. This is not a clean
warm-reopen pass. Many rejected left buffers are small and have no metadata,
so 111 buffers must not be reported as 111 lost physical exposures.
The existing user GUI remains alive, and camera nodes were released after both
native processes exited. No firmware/calibration/projector state was changed.

Next: compare controlled rate/transport conditions with rejection reasons intact,
and isolate warm-start framing independently of geometric calibration. Do not
rerun the identical soak as a substitute for addressing its failed gate.
