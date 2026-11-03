# Sustained capture evidence reconciliation

## Finding and scope

**Confirmed:** independent SCAN-REV-032 identifies a completeness defect in the
private sustained-trial assessment, not a production capture regression. The
original assessment could accept missing or inconsistent capture/replay evidence.
Assignment 496 separately verified the genuine 36,600 raw frames, both actual native
replays, 13,761,774 selected USB events and state restoration. Assignment 500 confirms
both publication 36e4adb CI runs passed. Neither fact closes the assessment finding.

**Developer correction:** the new private `sustained-admission-fix-20261104` bundle
contains `admit_complete.py`, `raw-check/src/main.rs` and `test_complete.py`.
The original `inline-sustained-20261104` inputs, assessment, results and failures
remain unchanged. No application, kernel implementation, Cargo workspace, scanner
settings or firmware changed. The diagnostic remains specific to this declared
30 Hz, 1920x1200 GREY, recording-v2 trial and is not a general qualification system.

## Mandatory checks

The normal command takes an evidence directory and a **new** result directory.
It refuses an existing result directory and always performs both fresh reads:

1. The Rust helper calls the production
   `scancraft_recording::indexed::IndexedRecording::build` to check both complete
   recordings, including CRC/footer, metadata and ordering. It emits actual sample
   identities, profile, context, frame and payload counts. Source opening refuses
   symlinks/nonregular inputs; device/inode/size/time guards check each read and both
   paths again after the pair completes. The private helper caps each input at 1 TiB
   and indexing at 30,000 frames; these are diagnostic limits, not application changes.
2. An unchanged copy of the existing strict USB analyzer runs in the new result
   directory against read-only references to the original PCAP/report inputs. Fresh
   summaries and all associations must equal the retained originals. Original
   semantic, length, PTS, FID, EOF and identity checks remain mandatory.

The assessment then requires exactly one successful expected capture invocation,
with the declared count/output path and parsed stdout equal to the saved report.
It requires exactly one successful invocation for **each** expected recording;
each replay stdout must equal its saved replay output and the fresh raw result,
including context/profile/count/payload/integrity. A duplicated left replay cannot
stand in for the right camera.

All actual recording samples must equal report samples. The checker recomputes
sequence gaps, first/last identities, host span/rate, sensor tick ratio, unique PTS
associations, host overlap and the existing nearest-host diagnostic. Every USB row
must match its native sample and payload semantics. Required ongoing rejection
samples must be complete; unknown successful USB boundaries or duplicate identities
fail. Original summary checks remain a mandatory conjunction, including strictly
below 0.1% rejection thresholds, restoration, collector success and queue capacity 8.
Duplicate-key and nonfinite JSON inputs are rejected. Recursive comparisons preserve
JSON value types, so false cannot stand in for integer zero. Input metadata guards bracket
the whole reconciliation; source hashes are retained separately.

## Verification

**Developer-verified saved data:** fresh validation exits 0 with no rejection reasons.
Both 18,300-frame recordings pass full parsing; all 36,600 native/USB associations and
all 13,761,774 selected events reconcile. Sixteen correction-source bindings remain
unchanged, as do the evidence file identity/size/time guards before/after the run.
Source hashes were recorded during this validation, not independently attested
before execution. This is a new offline check, not a new hardware trial.

**Developer regressions:** 95 complete-reconciliation cases pass, including all five
reviewer counterexamples, missing/duplicate/wrong-camera invocations, mismatched
count/context/profile/output, missing prerequisites and deliberately coupled
sample/header mutations that bypass simple saved-output equality. The unchanged
54 original summary cases also pass, retaining the exactly 0.1% rejection case.
Final developer inspection reproduced three additional first-version gaps caused
by Python numeric/boolean equality: a false camera interface, floating-point frame
count and numeric rejection flag. That version and its successful 92-case results
are retained; the final correction uses type-preserving comparisons and adds three
regressions. This is a developer finding, not a new independent-review identifier.
Five actual CLI cases refuse missing/empty/truncated/symlinked recordings and reuse
of an existing result directory. Positive tests use fresh receipts from the actual
bundle; mutations are in memory and never alter original evidence.

The private Rust helper passes formatting, strict Clippy and optimized compilation;
Python modules compile and the regression process exits 0. Existing production
recording parsing is reused, not replaced with a new file-format implementation.
The unchanged application retains the previous 892-test/fmt/Clippy/release pass;
no repeated application gate is claimed for this private-tool/documentation change.

## Limits and independent review

The pure assessment/receipt loader is a regression-test seam. Calling it with cached
receipts alone is **not** the mandatory fresh admission procedure. File identity/time
guards and internal consistency do not authenticate evidence against a malicious
writer, attest resident kernel code, or independently count physical exposures.
USB prefixes contain only header/prefix bytes, not complete independent image data.
The existing USB analyzer groups the finite trial in memory; this is not a bounded
streaming analyzer for arbitrary-duration captures. Hard filesystem timeout and
crash guarantees are not established by these tests.

SCAN-REV-032 remains **Medium/Open** pending independent verification. Development
does not alter its status. CodeRabbit substantive review remains unavailable; no
completed review, paid usage, merge approval or new hardware qualification is
inferred. Private raw data, calibration values and binaries remain outside Git.

**Next priority:** preserve this evidence correction and return to genuine calibrated
object reconstruction with independent physical measurements. The sustained
acquisition success does not establish physical units, dimensional accuracy,
repeatability, blue-laser reconstruction, tracking/fusion or native GUI qualification.
