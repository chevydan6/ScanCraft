# Development and independent review

Astra develops ScanCraft; CodeRabbit provides independent automated findings.
Human owner approval is required to merge. Neither a green CLI result nor a bot
approval proves correctness or authorizes merging.

## Local setup

The official CodeRabbit CLI is installed and authenticated using its supported
GitHub browser OAuth flow. Verify with `coderabbit --version`, `coderabbit auth
status`, and `coderabbit doctor`. Use `coderabbit auth login` if authentication
expires; never paste tokens into a tracked file. The official CodeRabbit Codex
skills were installed using `coderabbit skills install` (release v1.1.1). Start a
new Codex session to discover newly installed skills. Existing sessions can invoke
the CLI directly. CLI 0.9.1 is the initial verified version; consult `--help` after
upgrades because old skill examples include flags that have since changed.

GitHub PR review separately requires the official CodeRabbit GitHub App to have
access to ScanCraft. YAML alone cannot install/authorize an App. Install/configure
only the intended repository using https://github.com/apps/coderabbitai, then confirm
an actual bot review on a PR. Authentication and paid-plan prompts require the
owner; never claim integration works merely because the config file exists.

Before the offline gate, install the pinned toolchain and its rustfmt/clippy
components and run `cargo fetch --locked` while network access is available.
`tools/check.sh` uses `--frozen` for clippy/tests/build: missing cached dependencies
fail instead of being downloaded. Rustup/toolchain setup is a separate online step.

## Build output isolation

Use a separate Cargo target directory for each checkout path. Do not point a
review worktree at the active developer checkout's `target`: same-name/version
workspace packages can leave relative dependency-info and artifacts from the
other source tree, including an older release library or executable despite a
later `cargo build` reporting it fresh. This occurred during discovery/corner
integration and was detected by running the actual release command, not merely
checking Cargo's exit status. Preserve suspect artifacts for diagnosis, rebuild
in a never-shared directory, and verify both the exact source revision and the
actual executable's behavior before claiming release validation. Package-scoped
cleaning did not remove every artifact from the other checkout in that incident.
No scanner/firmware/setup change is part of build-cache recovery.

## Repeatable loop

1. Start a focused `dev/...` branch from current main; preserve unrelated work.
2. Implement a meaningful change with necessary numerical/regression evidence.
   Run `tools/check.sh` (fmt, strict all-target clippy, workspace tests, release build)
   plus focused checks. Reference Python requirements are separate from Rust CI.
3. Stage intended new files, then inspect the COMPLETE selected comparison with
   `git status --short` and `git diff main`, including committed changes since the
   base, staged edits and tracked unstaged edits. The default review includes all
   of those, not just `git diff --cached`. Check deletions/context and current
   source too; review may upload surrounding source context. Remove private data
   from the entire selected scope before upload; use an isolated clean worktree
   if unrelated tracked private edits exist. Never assume staging hides them.
   Run `coderabbit usage --agent`; check included capacity and billing mode. The
   command may exit nonzero with partial credit data even if quota is available.
   Do not use `--use-credits`, enable usage billing or approve charges without the
   owner's authorization. Stop if the service requests billing consent. Do not
   assume the absence of `--use-credits` overrides an account's automatic billing mode.
4. Run `coderabbit review --agent --gate major --base main`. CLI 0.9.1 emits NDJSON.
   Save raw output outside Git; record a sanitized findings disposition under
   docs/reviews. For the verified 0.9.1 binary, `review --help` lists `--gate`; a completed
   review with exit 2 and a matching `complete.gate.status: blocked` means the
   severity gate blocked. Check the complete record, findings and process status
   together: exit 2 alone is insufficient. Other failures, a missing completion,
   or an outcome other than completed are unavailable reviews, never passes.
   If a future CLI lacks `--gate`, inspect completed NDJSON finding severities
   explicitly instead of invoking an unsupported flag. Compare the
   reported scope/head with the intended changes. A scoped review is not a full
   existing-code audit.
5. Evaluate findings. Fix genuine defects, add focused regression evidence and
   rerun affected checks. Record rejected suggestions with code references and a
   technical reason. Use normal incremental review for changed content. Rerun a
   fresh severity gate before declaring the final diff ready. Stop unchanged loops;
   after three review passes per meaningful change, report remaining blockers
   rather than silently spending more quota. Preserve unresolved critical/major
   findings as blockers, even when the iteration budget is exhausted.
6. Push the development branch; open a ready PR with validation and review scope.
   Drafts skip automatic review. `.coderabbit.yaml` requests balanced reviews and
   pauses automatic incremental reviews after three reviewed commits. Batch fixes.
7. Retrieve PR reviews, issue comments, inline comments and unresolved threads:
   `gh pr view NUMBER --json reviews,comments,statusCheckRollup` and
   `gh api --paginate repos/chevydan6/ScanCraft/pulls/NUMBER/comments`. Use GraphQL
   reviewThreads for resolved/outdated status. `coderabbit pullrequest --help`
   describes the CLI's stored GitHub-feedback viewer. Reply with evidence to valid
   bot feedback, fix defects and update the branch; do not bulk-resolve findings.
   If automatic review paused or the service skips automatic review for a small
   OSS repository (observed: fewer than 10 stars), request one
   `@coderabbitai review` after meaningful
   fixes. Do not trigger repeated full reviews of unchanged commits.
8. Require passing checks and no unresolved actionable high-severity findings.
   Ask the owner to approve merging only after the change is reviewable. Do not
   merge, enable auto-merge or relax protection without explicit authorization.

## Read-only review readiness diagnostic

Run `python3 tools/review_readiness.py chevydan6/ScanCraft PR_NUMBER` before
interpreting the App badge. This uses read-only `gh` calls and emits JSON/exit0
only for known current-head CodeRabbit completion evidence with no open
conversations on an open, ready PR. It verifies every batch of one publication
attempt. It also recognizes the observed bot no-actionable issue-comment format,
requiring one recent-review section with the exact repository/commit URL and run
UUID plus one final coverage marker whose source and covered heads match and
whose kind is reviewed. Owner replies, stale/triaged/ambiguous summaries and
check badges do not satisfy that path. It detects a head change during collection
and refuses incomplete thread
or comment pagination. Pending/dismissed/spoofed/unknown-format reviews and green
or skipped check badges do not prove coverage. All unresolved conversations,
including outdated ones, block this diagnostic; current-head changes-requested
reviews are conservatively retained until an independent reviewer changes them.

This is a point-in-time diagnostic, not an installed CI requirement or an
authorization boundary. It does not test CI, correctness, quota, owner consent,
or privileged credentials, and always reports `merge_authorized: false`.
Re-check after every head change and immediately before any owner-approved merge.
Changing CodeRabbit's publication format can make coverage unknown until the
parser is updated with evidence. It never requests a review, resolves a thread,
changes protection, posts a comment or performs a merge. Tests run offline via
`python3 -m unittest discover -s tools -p test_review_readiness.py`.

## Repository controls

Main protection requires the `test` CI check, an up-to-date branch, the PR requirement,
conversation resolution, and applies to administrators. Force pushes and deletion
are disabled. GitHub reviews are distinct from the owner's explicit merge approval.
The owner authorized reducing the approving-review count to zero after the
independent reviewer demonstrated a single-maintainer deadlock. The PR requirement,
CI, conversations and administrator enforcement remain. Explicit owner consent is
still required before merge. Shared owner ADMIN credentials mean this consent is
procedural, not a technical permission boundary; genuine separation needs a distinct
developer identity with fork-only write access and owner authority outside this
runtime. Do not claim the present runtime is technically unable to merge.

CodeRabbit automatic approvals are disabled. No GitHub Actions API-key review job,
paid usage flag, recurring polling daemon, code autofix task or auto-merge bot is
installed. Review uploads only the intended public source diff, not local private
hardware evidence. Source-tree instructions and path-specific reviewer guidance
preserve ABI, units, bounded parsing, persistence compatibility and modularity.

References: https://docs.coderabbit.ai/cli and
https://www.coderabbit.ai/blog/coderabbit-plugin-for-codex .
