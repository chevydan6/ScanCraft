# Live domain-matching integration validation

Developer-identified integration correction based on source commit
`9de19880506b690dfe4243281e373d1945fb70ec`. Existing domain-aware matching now reaches
live capture, device-free replay, previews and saved-bundle inspection. Matching
mathematics, calibration, thresholds and acquisition settings are unchanged.

## Reproduction and implementation

`crates/scancraft-workflow/src/recorded_traces/automatic.rs` already accepted
`retain_valid_domain_segments: true`. The live request previously rejected that
field and always used strict processing. On the same genuine Raptor pair, frozen
geometry and numeric settings, recorded processing produced 983 candidates while
the actual application replay helper produced zero with 2,576 query errors. Its
first error was inverse-distortion nonconvergence in the declared lens domain.

`laser_stream.rs::Pipeline::with_valid_domain_segments` delegates to the existing
`laser_pair::Processor` option. `laser_stream/bundle.rs::Request`, `Plan::open` and
`Plan::capture` propagate the option for both schemas, with strict/default false.
`laser_stream/bundle/load/record.rs::Record::display` requires successful reports
to match the request policy, rejecting invented/missing or unbounded summaries.
Bounds match the recorded-candidate loader: 65,536 rejected vertices/segments and
256 rejected sheet projections per query. `load.rs::PairReader` exposes counts
and the conditional uniqueness scope.

`scancraft-app/src/laser_display.rs` and `laser_display/pending.rs` carry/validate
counts through helper preview transport. `candidate_ui.rs` keeps the warning with
the displayed pair after completion messages arrive; successful view replacement
clears the prior live summary. Geometry still has unverified physical identity.

## Tests and real-data replay

Complete `tools/check.sh` passes: formatting, strict all-target Clippy, 939 tests,
zero failures, one ignored test, and optimized workspace build. Added regressions:

- `domain_option_reaches_capture_preview_journal_and_reopening_in_both_schemas`
  checks omitted/false/true, both schemas and journal byte compatibility.
- `domain_option_rejects_non_boolean_declarations_before_capture` checks invalid
  null/numeric/string/object options before device acquisition.
- `domain_policy_and_bounded_exclusions_must_agree_after_redigest` rejects forged
  policy, missing/unknown summaries and count bounds even after redigest.
- `domain_preview_roundtrip_retains_exclusions_and_rejects_unbounded_or_failed_counts`
  checks preview compatibility and failed-pair/count semantics.

The extended `completed_capture_presets_saved_folder_and_last_display_pair` also
renders and checks warning text after completion.

Six pairs extracted from one genuine stationary Raptor recording were replayed
through the release application's `--laser-helper-replay` path. Raw inputs, model
and numeric settings are retained; only the new live option is enabled. All 6,006
candidate records, coordinates, observation/trace/support indices and sheet
identities equal recorded results exactly. Every compact query outcome and domain
count agrees with the detailed recorded output, including failures and ambiguity.

| Pair | Queries | Candidates | Errors | Nonunique |
| --- | ---: | ---: | ---: | ---: |
| Start | 2714 | 990 | 428 | 1296 |
| Middle | 2616 | 949 | 502 | 1165 |
| Last common | 2576 | 983 | 350 | 1243 |
| Known affected | 2595 | 1013 | 299 | 1283 |
| Before affected | 2613 | 1028 | 343 | 1242 |
| After affected | 2665 | 1043 | 347 | 1275 |

Strict replay with the new executable preserves baseline geometry, request, both
raw recordings and complete pair journal byte-for-byte. Input hashes verify before
and after comparison. Release executable SHA256:
`c69a787ca5a2cce10b178e32c84e217aefe94795844a2edc2963dae86f7a7fa1`.

Baseline, corrected helper-preview and reopened-bundle GUI screenshots were
inspected. Corrected/reopened views show 983 points, retained camera images and
411 excluded vertices, 401 segments, 545 sheet projections, with conditional
uniqueness warnings. Tests use an isolated authenticated display and software
rendering, without camera/USB descriptors in the GUI. Owned processes exit afterward.

Private evidence label `live-domain-integration-20261104` retains baseline hash
bindings, executable identities, source diff, gate log/result, requests, bundles,
helper IPC, `verify_replay.py`, comparison results and screenshots. References remain
in sealed `current-box-reconstruction-20261104`; underlying capture remains in
sealed `candidate-hash-20261104`. Private scans/model/device data are not published.

## Limits

These are real sensor recordings replayed offline, not fresh acquisition or three
independent physical trials. Equality proves integration consistency, not physical
correctness of the shared matcher/calibration. Displayed geometry remains sparse
fragments, not a complete measured box. Owner dimensions were not fit inputs.

No pixel repair/exclusion, driver/gain change, tracking, fusion, metric export or
physical calibration is introduced. SCAN-REV-034's accepted-image corruption
remains High/Open under independent reviewer control. Physical units, full lens
coverage, synchronization and dimensional accuracy remain unqualified. Next:
confirm current device/scene state, capture usable moving-object observations and
validate reconstruction against held-out measurements.

CodeRabbit usage lookup exits 1 with included capacity/billing/credit fields
`lookup_failed`. No substantive review or paid usage; publication stays draft.
