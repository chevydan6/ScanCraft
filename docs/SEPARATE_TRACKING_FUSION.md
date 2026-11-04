# Separate tracking observations and surface fusion

## Production change

`scancraft_geometry::session::ScanSession::process_with_tracking_cloud` estimates
pose from a separate bounded observation cloud and integrates **only the surface
cloud**. This permits reflective marker observations to guide laser-surface fusion
without adding the markers themselves to the output geometry.

```rust,ignore
session.process_with_tracking_cloud(frame_id, &surface, &markers, &work)?;
```

Both inputs must describe the same capture, coordinate frame and units. The caller
must establish identity and synchronization; numerical acceptance does not certify
physical accuracy. Tracking observations are limited to3..=512 points, and surface
input to0..=100,000 points. Point-to-plane tracking takes its normals from the
observation cloud. The existing indexed `process` entry point retains its behavior,
including its nonempty surface requirement, and delegates to the same internal
tracking/fusion transaction.

An empty surface through the new method records an accepted **tracking-only** frame
when marker alignment succeeds. It does not fabricate geometry or advance
`last_integrated`. `SessionFrame::fusion == None` can therefore mean rejected
tracking **or** accepted tracking with no surface; inspect `tracking.accepted` and
its pose/reason as well. `last_accepted` can advance while `last_integrated` stays
unchanged. This lets tracking continue through temporary missing laser geometry.

Errors, cancellation and failed voxel integration leave tracking reference, fused
map and history unchanged together. Rejected alignment publishes its reason and
no fused geometry. There is no new numerical registration algorithm, dependency,
format change, scanner control, automatic filtering or GUI wiring in this change.

Source: `crates/scancraft-geometry/src/session.rs`; regression tests:
`session/separate_tests.rs`. Five new tests verify marker/surface separation and
correct surface transformation, integration rollback/retry with invalid inputs and
cancellation, loss/recovery without fusion, observation-only normals, and empty
surface tracking followed by real geometry. Final workspace gate:
**909 passed /0 failed /1 ignored**, formatting, strict all-target Clippy and release
build passed. Initial test-only exact floating equality differed by about3e-14;
the corrected numerical assertion uses1e-10. Existing physical gates were not changed.

## Genuine rotating Raptor replay

The previous `MARKER_TRACKING_VALIDATION.md` covered stationary data. This test uses
the first31 consecutive left frames from an existing900-frame rotating-board
recording. `Playback::open` validates both complete files and footers once; unique
equal-PTS counterparts within1000us host separation are selected. All31 pairs
qualify this pairing rule, but exposure synchronization remains unverified.
Original payloads, metadata, contexts and CRCs are preserved in derivative
single-frame recordings. Source hashes and playback identity guards pass.

The board fills the image, so full-image native component/shape detection is used,
not the earlier turntable y>=550 region. Three unchanged brightness thresholds
80/100/120 are evaluated; area5..1000, shape RMS<=0.5px and complete stereo graphs
retain rejected and ambiguous observations. Mutually unique stereo hypotheses feed
the existing native `Tracker`. No cross-threshold filter is promoted.

Marker tracking keeps the previous3RAW correspondence,0.25RAW RMS,>=0.7 source and
distinct-target coverage,30iterations and1e-8RAW convergence gates. The exploratory
motion envelope is declared as15RAW translation and0.05rad rotation per step:
rotation about a target distant from the camera origin has a translation component,
so the earlier stationary2RAW step bound is not reused. This is a changed motion
configuration, not improved accuracy or relaxed stereo/registration residual gates.

| Threshold | Accepted subsequent updates | Maximum alignment RMS (RAW) | Final accumulated rotation (degrees) |
|---|---:|---:|---:|
| 80 | 30 /30 | .178939 | 12.112602 |
| 100 | 30 /30 | .212933 | 12.073186 |
| 120 | 30 /30 | .232532 | 12.025754 |

Each sequence additionally has one unvalidated initial anchor. Independent
nearest-neighbor/SVD ICP reproduces all90 later decisions, coverage fractions and
composed poses within3.608e-12. Original start/end images and trajectory plot were
inspected. This supports short real-motion operation, not externally measured
angular accuracy, long-session stability or genuine motion-loss recovery.

## Actual laser input and fusion outcome

All31 selected pairs were processed by the existing native automatic laser matcher
with the frozen camera/sheet models. The box detector preset produced218 candidates
from10 frames; the other21 frames had no accepted geometry. A second replay with
previously documented board detector settings produced zero candidates in every
frame. Both complete outcomes remain available; no threshold, saturated-pixel,
continuity, lens-domain, correspondence or geometric gate was weakened to increase
point counts. First-frame box-preset saturation rejections were5031left/4253right;
fragmented traces and remaining queries outside the lens domain explain major
losses in these recordings. They do not isolate every cause of missing geometry.

The new session method was exercised on all31 frames for every marker threshold
and both surface presets, loading surfaces through the existing integrity-checking
`candidate_bundle::load`. A0.25RAW voxel size and100,000-voxel capacity were fixed.

| Surface preset | Marker threshold | Fused voxels | Integrated frames | Accepted tracking-only frames |
|---|---:|---:|---:|---:|
| Box | 80 | 183 | 10 | 21 |
| Box | 100 | 192 | 10 | 21 |
| Box | 120 | 189 | 10 | 21 |
| Board | 80 /100 /120 | 0 each | 0 each | 31 each |

All session poses equal the prior marker-only tracker outputs exactly. Independent
transformation and voxel averaging of **only original laser candidates** reproduces
all fused coordinates within1.137e-13RAW. Marker points are not inserted. Separate
inverse-Brown/closest-ray calculations check2374 accepted marker pairs and218 laser
candidates; maximum coordinate difference8.279e-10RAW. This is numerical integration
evidence, not proof that every candidate has the correct physical stripe identity.

The fused output is visibly sparse fragments, **not a recognizable complete board
or box scan**. Experimental outputs explicitly retain unverified model units;
no millimeter PLY, mesh, native GUI fusion workflow or physical metrology is claimed.
The new core API still needs a user-facing acquisition/replay adapter. A captured
marker or laser point is never replaced with synthetic geometry to fill gaps.

## Evidence and reproducibility

Private artifact root: `rotating-marker-tracking-20261104`. It contains the declared
plans, bounded playback adapter, original-file hashes,62 derivative frame recordings
and PNGs, complete marker/stereo/laser results, tracking requests/results, final
fusion adapter and outputs, source/binary bindings, gates and independent verifiers.
Initial failed test/adapter builds and the failed presets are preserved. The initial
playback build predates the additive session implementation and is verified against
Git base`bb6cca83ebdda70aec9e81e4c507b0e269d02762`; the final fusion binding matches
the tested implementation. This distinction is explicit in `verify_bindings.py`.

For reproduction in an output copy: use the private playback source and recorded
extraction arguments; `process.py`, `verify_tracking.py`, `reconstruct.py`,
`reconstruct_board.py`, the archived fusion binary, `verify_geometry_fusion.py` and
`verify_bindings.py`. Complete command/exit receipts and input hashes are retained.
Do not overwrite original evidence. The single warm playback observation was5.49s
and13,692KiB peak RSS, including validation/extraction/output; this is not a live
acquisition or full-pipeline throughput benchmark.

Manifest `separate-tracking-cloud-milestone-20261104/evidence-manifest.json` binds
1618files /571,788,419bytes; SHA256:
`a817a19f78f6c0dd422ad2501be18b348ac3c356c2c683476338cf6a1d1b676a`.
Raw/private data stays out of publication. CodeRabbit's repository usage lookup
still fails; no substantive review or paid usage is claimed. Independent review
of this implementation remains outstanding.

**Next:** expose this separation through a bounded recorded/live workflow, preserving
empty-surface and rejected-pose diagnostics. Validate it with usable unsaturated
object laser frames before claiming full reconstruction. Verified metric calibration,
three independently measured scans, native viewing/editing/save/reopen/export and
sustained reliability remain required.
