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

## Repository controls

### Live protection audit — 2026-10-26 UTC

Verified directly with GitHub REST branch protection, repository settings,
collaborators, rulesets and effective branch rules, plus GraphQL protection rules
and PR review state. These are observed settings, not inferred from this file.

| Control | Verified live value | Proposed single-maintainer value |
| --- | --- | --- |
| Required CI | `test`, GitHub Actions app ID 15368 | Keep |
| Branch up to date | Required (`strict: true`) | Keep |
| Approving reviews | 1; stale approvals dismissed | 0, retaining PR requirement |
| Code-owner / last-push approval | Both disabled | Keep |
| Conversations resolved | Required | Keep |
| Administrators subject to protections | Yes | Keep |
| Force pushes / main deletion | Both prohibited | Keep |
| Repository auto-merge | Disabled | Keep |
| Repository/effective rulesets | Both empty | Keep |
| Actions token / approving reviews | Read default / disabled | Keep |

**Approval deadlock confirmed.** The only listed collaborator is `chevydan6`
(admin), and the authenticated developer session is also `chevydan6`. PR #1 is
owned by that account; its required `test` checks pass, its sole inline thread is
resolved, but GitHub reports `REVIEW_REQUIRED` and `BLOCKED`. GitHub does not allow
PR authors to approve their own PRs. The owner cannot satisfy this requirement
for PRs created using the owner's identity. This conclusion is specific to the
verified shared-identity model; a separate developer identity would let the owner
approve its PRs.

**CodeRabbit findings do not satisfy the current approval requirement.** The
actual review on PR #1 is `COMMENTED`, by `coderabbitai[bot]`, not `APPROVED`.
Its successful `CodeRabbit` status is distinct from an approving review. Do not
claim that all bots are intrinsically ineligible: eligible App approvals can be
used in other configurations. No App approval is demonstrated here, and enabling
bot approval would not constitute owner merge authorization.

The official App is operational: it posted a review and actionable feedback on
PR #1. `.coderabbit.yaml` in pending PR #1 enables automated reviews and disables
automatic approvals. This YAML and the original development instructions are not
yet merged into main. The service explicitly reports that automatic reviews are
skipped for this repository because it has fewer than 10 stars. Keep the App and
auto-review configuration enabled, but verify a completed review for each head;
request `@coderabbitai review` when necessary and within included quota. A green
status for a skipped review is not evidence that the head was reviewed. Do not
enable billing to eliminate this service limitation without owner authorization.

**Configuration change pending owner authorization.** Minimum proposed action:
PATCH only `required_approving_review_count` from 1 to 0 at
`repos/chevydan6/ScanCraft/branches/main/protection/required_pull_request_reviews`.
Retain the PR requirement and every other protection above. Do not delete the PR
review protection object or replace the entire branch protection configuration.
Re-read REST and GraphQL settings afterward, then inspect the harmless audit PR
for passing CI and removal of `REVIEW_REQUIRED`, leaving it unmerged. Until that
change is authorized and verified, the approval deadlock remains unresolved.

**Owner authority is procedural with the current credentials.** AGENTS.md and the
PR template prohibit merging without explicit owner approval; the inspected CI
workflow contains no merge step and Actions defaults to a read token with review
approval disabled. Nevertheless, this session has owner ADMIN permissions and can
invoke the merge/settings APIs. Auto-merge being off does not disable manual API
merges. It is therefore impossible to verify that Astra is technically unable to
merge using these credentials. No destructive merge or bypass probe was attempted.

For technical separation, remove owner tokens and authenticated owner browser /
connector sessions from the automation environment. Use a separate identity with
read access to the upstream repository and write access only to its fork, opening
cross-fork PRs; reserve upstream write/admin credentials for the owner outside the
agent runtime. This retains the single human maintainer model and prevents the
agent from merging upstream. Alternatively, an organization repository can restrict
protected-branch pushes/merges to the owner while allowing a separate developer to
push feature branches. A separate identity with ordinary upstream write access
alone is insufficient after required approvals become zero. A token carrying
owner identity, even if narrowed to contents/PR writes, is not an owner consent
gate. Tool confirmations alone are procedural, not a GitHub permission boundary.

### Harmless workflow probe

A separate `dev/protection-audit` branch from the live main contains only this
document. The original workflow PR and local development branch are preserved.
The audit PR is intentionally left open and must not be merged without explicit
owner authorization. Check its live CI and review results; post-change behavior
cannot be claimed before the settings change is approved and applied.

Evidence:
- PR #1: https://github.com/chevydan6/ScanCraft/pull/1
- Actual bot review: https://github.com/chevydan6/ScanCraft/pull/1#pullrequestreview-5649182265
- GitHub approval rules: https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/approving-a-pull-request-with-required-reviews
- GitHub protection API: https://docs.github.com/en/rest/branches/branch-protection
- CodeRabbit configuration: https://docs.coderabbit.ai/reference/configuration


References: https://docs.coderabbit.ai/cli and
https://www.coderabbit.ai/blog/coderabbit-plugin-for-codex .
