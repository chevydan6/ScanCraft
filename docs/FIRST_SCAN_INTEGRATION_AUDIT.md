# Integration audit for the first genuine scan

Snapshot: 2026-11-03. All 30 public PRs are still open. The developer integration
branch is `dev/first-genuine-scan-integration`, starting from `1d7380a733e56a988e493279931f1df10eaf0bd6`.

The local development history already contained 27 developer PR heads. PR27 and
PR28 each had a documentation correction whose patch was already present under
another commit. `git merge-tree` predicted the identical tree for each merge.
History-only merge commits now retain those exact public heads too, ending at
`c5c8ed77bdcd5cfc54b49c8cd8506efc34f9aab4`; tree
`f3be5e6595fea22595cd0f3b401d2b70d89ec651` is identical to the starting tree.
No algorithms or working source were replaced during consolidation.

PR2 is exclusively independent reviewer work and is deliberately excluded. Its
findings and charter remain read-only and untracked in the developer checkout.
No source merge into `main`, review bypass, thread resolution or branch protection
change was performed. Existing PRs were not closed or rebased.

## Dependencies and review status

The developer stack is PR1 → PR3 → PR4 → … → PR30. PR2 separately targets main.
Base/head refs are preserved in `evidence/first-scan-integration-audit.json`.
All 29 developer heads are now ancestors of the integration branch.

| PR / scope | Depends on | Audited head | State / review evidence | Returned PR Actions runs |
| --- | --- | --- | --- | --- |
| [#1](https://github.com/chevydan6/ScanCraft/pull/1) Establish CodeRabbit independent review workflow | main | `ba5ec7ca18` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#2](https://github.com/chevydan6/ScanCraft/pull/2) Independent protection audit, reviewer charter and baseline findings (do not merge) | main | `f385c1dcc7` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#3](https://github.com/chevydan6/ScanCraft/pull/3) Recover capture lifecycle and keep ordinary close responsive | #1 | `cc1322b797` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#4](https://github.com/chevydan6/ScanCraft/pull/4) Add opt-in fractional stereo refinement | #3 | `12fc3a2b80` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#5](https://github.com/chevydan6/ScanCraft/pull/5) Inspect stereo recording timing without inferring synchronized exposures | #4 | `b452b09eb8` | draft; current-head completion unconfirmed; 0 unresolved | success |
| [#6](https://github.com/chevydan6/ScanCraft/pull/6) Cross-check stereo raw PTS values against recorded host timing | #5 | `0d6e96ee07` | draft; current-head completion unconfirmed; 0 unresolved | success |
| [#7](https://github.com/chevydan6/ScanCraft/pull/7) Reconstruct explicit recorded stereo sequences through native tracking and fusion | #6 | `47293ddac0` | draft; current-head completion unconfirmed; 0 unresolved | success |
| [#8](https://github.com/chevydan6/ScanCraft/pull/8) Add native guarded background workflow for recorded stereo sequences | #7 | `13790e34aa` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#9](https://github.com/chevydan6/ScanCraft/pull/9) Retain sequence source references through project editing and reopening | #8 | `8da307e102` | open; current-head completion unconfirmed; 2 unresolved | success |
| [#10](https://github.com/chevydan6/ScanCraft/pull/10) Preserve all stream cleanup failure diagnostics | #9 | `a6fa2d5f74` | open; known current-head completion; 0 unresolved | success |
| [#11](https://github.com/chevydan6/ScanCraft/pull/11) Verify SDK byte mirror arithmetic with an isolated synthetic probe | #10 | `e1c4fc686f` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#12](https://github.com/chevydan6/ScanCraft/pull/12) Connect native stereo capture to incremental reconstruction and fusion | #11 | `84b89f42c9` | open; current-head completion unconfirmed; 1 unresolved | success |
| [#13](https://github.com/chevydan6/ScanCraft/pull/13) Add native live scan GUI controls and bounded previews | #12 | `9a071e293f` | open; known current-head completion; 0 unresolved | success |
| [#14](https://github.com/chevydan6/ScanCraft/pull/14) Retain live scan session lineage in editable projects | #13 | `5871a7a5b1` | draft; current-head completion unconfirmed; 0 unresolved | success |
| [#15](https://github.com/chevydan6/ScanCraft/pull/15) Export edited live projects with complete raw session references | #14 | `80bca7080b` | draft; current-head completion unconfirmed; 0 unresolved | success |
| [#16](https://github.com/chevydan6/ScanCraft/pull/16) Observe cooperative stop between native capture waits | #15 | `b2ca99c7d2` | draft; current-head completion unconfirmed; 0 unresolved | success |
| [#17](https://github.com/chevydan6/ScanCraft/pull/17) Remap large stereo pairs concurrently with measured sequential fallbacks | #16 | `31cfbe1b8c` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#18](https://github.com/chevydan6/ScanCraft/pull/18) Add native planar camera initialization and numerical reference checks | #17 | `761036d5a9` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#19](https://github.com/chevydan6/ScanCraft/pull/19) Fit Brown5 camera calibration with native refinement and offline CLI | #18 | `26987bbf0b` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#20](https://github.com/chevydan6/ScanCraft/pull/20) Add fixed-intrinsic stereo calibration and diagnostic CLI | #19 | `9f853105d2` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#21](https://github.com/chevydan6/ScanCraft/pull/21) Decode Raptor SCR observations and inspect recording tag consistency | #20 | `5336d8fd71` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#22](https://github.com/chevydan6/ScanCraft/pull/22) Isolate scanner preview and live scanning in supervised Rust helpers | #21 | `224621b491` | open; known current-head completion; 0 unresolved | success |
| [#23](https://github.com/chevydan6/ScanCraft/pull/23) Move scanner discovery off the GUI thread with retained child ownership | #22 | `a5deec7249` | open; known current-head completion; 0 unresolved | success |
| [#24](https://github.com/chevydan6/ScanCraft/pull/24) Add offline target-corner measurements and read-only review coverage diagnostics | #23 | `53f374a220` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#25](https://github.com/chevydan6/ScanCraft/pull/25) Document the frozen ScanCraft architecture provenance and qualification audit | #24 | `008f9b7633` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#26](https://github.com/chevydan6/ScanCraft/pull/26) Add native component observations for recorded scanner images | #25 | `352d6629cc` | open; known current-head completion; 0 unresolved | success |
| [#27](https://github.com/chevydan6/ScanCraft/pull/27) Associate planar targets using four-anchor image hypotheses | #26 | `fe5470ee43` | open; current-head completion unconfirmed; 1 unresolved | success |
| [#28](https://github.com/chevydan6/ScanCraft/pull/28) Report component shape quality while retaining raw observations | #27 | `9181d0fd48` | open; known current-head completion; 1 unresolved | success |
| [#29](https://github.com/chevydan6/ScanCraft/pull/29) Document sustained Raptor capture evidence and remaining reliability failures | #28 | `0d6eb4119e` | open; current-head completion unconfirmed; 0 unresolved | success |
| [#30](https://github.com/chevydan6/ScanCraft/pull/30) Add bounded stripe continuity diagnostics without discarding raw detections | #29 | `b8c772078a` | open; current-head completion unconfirmed; 0 unresolved | success |

Review coverage uses the existing `tools/review_readiness.py` against current
heads, complete known publication markers and unresolved conversations. A green
CodeRabbit status is not a completed review. Only PR10/13/22/23/26 pass that narrow
coverage-and-conversation diagnostic in this snapshot; it does not establish CI,
code correctness or merge authority. Missing recognized completion is not proof
that no historical review occurred. API metadata heads were rechecked unchanged.
GitHub connector workflow retrieval is limited to the first page of PR-triggered
runs; three initial timeout results are retained; one read-only retry for each returned
a successful PR-triggered workflow run. The local integration gate
separately passes 828 tests, formatting, strict Clippy and optimized build.

Five unresolved review threads remain across PR9/12/27/28. Their legitimate
corrections are present locally: empty-directory guard in `cloud_ui.rs`, path
context in `source_verification.rs`, spacing in `LIVE_STEREO_SCAN.md`, the accepted
stdin bound in `TARGET_ASSOCIATION.md`, and explicit Git-main/CLI-main distinction
in `REVIEW_RESPONSES.md`. Existing reviewer findings010/011/013/022/023 document
those dispositions. Source fixes are not authority to close independent threads;
all remain unchanged by development. PR28's public head predates its prose fix;
the integration branch retains the corrected descendant.

## Integration gaps, duplication and priorities

- Genuine acquisition and saved diagnostic laser pairs are implemented, but a
  model-bound, physically qualified calibration and independent dimensions remain
  absent. Accepted candidate correspondences and ICP results are not an accurate
  object scan. See `CALIBRATION.md` and `FIRST_SCAN_CALIBRATION_PLAN.md`.
- Recorded-stereo reconstruction/session/GUI/editable project code exists across
  PR7–15. Its synthetic demonstrations do not qualify actual Raptor geometry.
  Laser diagnostic views deliberately remain outside metric Editor/project state;
  integrating them by relabeling their units would be incorrect.
- The old IR live path (`live_stereo`, `live_references`) still has its 1,000-frame
  bound. The newer laser path has opt-in bounded segmented journals and a sequential
  reader, but does not provide a calibrated tracked scan. These are overlapping
  acquisition/orchestration paths with different guarantees, not interchangeable
  implementations. Do not copy limits or remove caps without coordinating lineage,
  history, storage admission, replay and finalization.
- `stereo_probe`, native IR live capture and laser source workers have distinct
  lifecycle code. The new mission's USB experiment investigates finite CLI peer
  teardown overlap. Any correction there must not be claimed as a fix for all
  live acquisition paths or the earlier repeated transport truncations.
- There is no evidence-based reason here to rewrite the Rust geometry engines.
  Required integration is calibrated observations → reconstruction → staged
  tracking/fusion → retained project provenance, followed by physical trials.
- Shared-owner credential separation (002), truthful review coverage (003),
  unbounded driver/forced shutdown (005), physical qualification (006), work-control
  coupling (007), and IR duration capacity (021) remain recorded limitations.
  Independent reviewer statuses are not modified by this audit.

No new editing features are planned. The immediate priorities are the measured
acquisition failure, calibration authorization/ground truth, and one repeatable
real reconstruction. Full 30-minute/60-minute and final release qualification
remain open. CodeRabbit CLI quota/billing queries still fail; no paid request or
review upload is made. The integration branch is local pending its review gate.

## Reproduction and scope

Use `git merge-base --is-ancestor <public-head> <integration-head>` for each entry;
compare trees for the two history-only merges. Run `tools/check.sh` with the pinned
cached toolchain and `tools/review_readiness.py chevydan6/ScanCraft NUMBER` for a new
point-in-time review snapshot. Private artifact `first-scan-integration-ao3yvkv3`
retains API responses, graph script, gate logs and per-PR readiness outputs.
This audit covers all PR dependency/coverage states and relevant integration
boundaries; it is not a fresh line-by-line numerical review of every algorithm.
