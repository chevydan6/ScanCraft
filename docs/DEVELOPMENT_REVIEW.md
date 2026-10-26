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
   If automatic review paused, request one `@coderabbitai review` after meaningful
   fixes. Do not trigger repeated full reviews of unchanged commits.
8. Require passing checks and no unresolved actionable high-severity findings.
   Ask the owner to approve merging only after the change is reviewable. Do not
   merge, enable auto-merge or relax protection without explicit authorization.

## Repository controls

Main protection requires the `test` CI check, an up-to-date branch, one approval,
conversation resolution, and applies to administrators. Force pushes and deletion
are disabled. GitHub reviews are distinct from the owner's explicit merge approval.
A sole author cannot approve their own PR; another eligible reviewer is needed for
the GitHub approval requirement. Never bypass that requirement automatically.

CodeRabbit automatic approvals are disabled. No GitHub Actions API-key review job,
paid usage flag, recurring polling daemon, code autofix task or auto-merge bot is
installed. Review uploads only the intended public source diff, not local private
hardware evidence. Source-tree instructions and path-specific reviewer guidance
preserve ABI, units, bounded parsing, persistence compatibility and modularity.

References: https://docs.coderabbit.ai/cli and
https://www.coderabbit.ai/blog/coderabbit-plugin-for-codex .
