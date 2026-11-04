# Scoped IR gain in native laser acquisition

Developer-identified integration gap, based on commit
`b50fe07369e39ca7f1df715d4ff6556685130497`: the diagnostic capture command could
request gain 15000, but the shared native live adapter accepted illumination only.
The GUI/helper and `scan-live-laser` now accept explicit `ir_gain_raw` under
`source_options`, reusing existing standard-control code.

## Implementation and software checks

`crates/scancraft-acquisition/src/laser.rs::source_options` preserves omitted/null
source options and omitted gain. Objects require known illumination and allow only
the optional integer gain field. Invalid values fail before device ownership.
Values 1000–15000 also undergo the existing hardware range/step/availability checks
in `scancraft-device/src/ir_gain.rs::ScopedIrGain::apply`, which snapshots/validates
both cameras before writes and rolls back partial application.

`scan` checks gain after each source starts and explicitly closes a newly opened
capture on readback failure, retaining cleanup errors. `Native::context` records
requested/observed values and interface under
`laser_source_declaration.ir_gain_trial`. Both end values are checked. Line,
stream and gain restorations all run despite earlier errors; gain restores after
formats/rates because setup can affect controls. An outer restoration covers
failures before the bundle callback. Existing synchronous driver calls still have
no established hard bound; SCAN-REV-005 is not resolved.

Provenance: existing ScanCraft `ScopedIrGain` and CLI `stereo_probe.rs` readback
conventions. No math, dependency, calibration, firmware, mode, energy or mirror
change. Only the acquisition module and its crate documentation change in production.

`tools/check.sh` passes formatting, strict all-target Clippy, 941 tests, zero
failures, one ignored, and optimized workspace compilation. Acquisition tests
cover defaults, allowed/invalid declarations, mismatches, both end checks despite
a first-camera error, and no gain read when omitted. Existing device tests cover
partial-write rollback, restoration failures and control range/step/availability.

## Real hardware results, including failures

Same original driver, fixed 30 FPS, explicit line boolean/gain 15000, frozen
geometry and numeric settings, with domain handling enabled. Measured box
dimensions are not inputs. Rejection counts are **V4L2 buffers**, not necessarily
physical exposures. Sequential trials do not isolate rendering or initialization.

| Trial | Accepted L/R | Pairs | Startup rejects L/R | Ongoing rejects L/R |
| --- | --- | ---: | --- | --- |
| GUI, isolated software-rendered display | 0/0 | 0 | 235/235 | 0/0 |
| Headless `scan-live-laser` | 16/16 | 13 | 100/1 | 1/0 |
| Repeated GUI, same binary/settings | 0/0 | 0 | 318/235 | 0/0 |
| Direct native app helper, no renderer | 16/16 | 11 | 1/94 | 0/111 |

Both GUI attempts retain camera poll-timeout failures and rejection evidence.
Their harness subsequently reaches its observation timeout; that failure remains.
No successful live GUI acquisition is claimed. A separate existing CLI probe
comparison captures 16/16 with startup 98/0 and ongoing 0/0.

Headless live processing produces 953–1067 candidates per pair with 2/2 processing
queue drops. The helper produces 951–1045 with 3/3 processing drops. Those omissions
are distinct from buffer rejections; all accepted raw frames are journaled. All
24 pairs reopen through native inspection; all four complete raw files pass
header/frame/footer CRC checks. Each header records the correct interface and
requested/observed gain 15000. All 11 helper IPC previews match journal coordinates,
query counts and domain summaries; completion explicitly reports native origin.

Reopening the helper bundle in the GUI succeeds. The inspected screenshot shows
1009 points in its first pair, retained camera images and domain/identity warnings.
It is sparse geometry, not a complete measured box.

A deliberate 4096-byte required pair-journal limit fails with the expected byte
budget error, leaves no completed target, retains raw prefixes of 3/2 frames, and
reports both sources closed/settings restored. Seven independent native state
comparisons after headless/helper/quota trials match originals **before** outer
test safeguard restoration. Failed GUI trials also report cleanup and pass final
external comparisons. All four wrappers restore formats/state/media service.
Both original IR gains are 1000 and restored to 1000. No module/firmware change.

## Remaining acquisition work

GUI startup remains unqualified. Headless success and two GUI failures are a useful
comparison, not proof that rendering causes truncation. SCAN-REV-034 stays open.

Another source-level difference deserves a controlled test:
`laser_stream/runner.rs::produce` closes each completed source independently,
whereas `live_stereo/runner.rs` and CLI `stereo_probe` coordinate retirement. All
32 retained right-camera rejection samples in the helper trial occur after the
left camera's last accepted timestamp. Actual close time was not instrumented;
111 rejected buffers do not mean 111 lost exposures. This hypothesis cannot
explain GUI failures with zero frames. No retirement change is included here.

Private evidence label `live-ir-gain-integration-20261104` retains source/binary
bindings, gate logs, all four hardware wrappers, failures, raw bundles, readbacks,
IPC, `verify_hardware.py`/results, retirement observations and the GUI screenshot.
These results do not qualify units, synchronization, coverage, accuracy, repeatability
or reliable end-to-end scanning. Raw scans/device data remain outside Git.

CodeRabbit capacity, billing and credit lookup returned `lookup_failed`. No
substantive review or paid usage occurred; the change remains a draft for review.

## Subsequent lifecycle correction

The source-retirement and gain-restoration behavior above describes the original
PR52 baseline. The [lifecycle follow-up](LASER_LIFECYCLE_VALIDATION.md) coordinates
retirement and corrects SCAN-REV-035's format-retry/gain-cache interaction. It
passes the 948-test software gate; new hardware comparison remains pending because
the scanner was absent at preflight. Earlier hardware failures remain evidence.
