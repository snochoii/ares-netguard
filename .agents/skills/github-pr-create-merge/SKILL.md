---
name: github-pr-create-merge
description: Create GitHub PRs and perform guarded auto-merge only when validation and review gates pass.
---


# GitHub PR Create Merge

## PR

Use `gh pr create` after branch push and validation.

PR body must include:

- summary;
- validation commands;
- artifact guard result;
- privacy/safety note;
- technology selection and rejected alternatives when applicable;
- subagent, parallel, and worktree decisions;
- required review routing.

After creating the PR during normal `$netguard-orchestrator` execution, do not
stop. Immediately continue into guarded merge-gate evaluation in the same run
unless validation, checks, mergeability, artifact/secret policy, required review
gates, branch safety, or explicit user instruction blocks merge.

## Merge

Auto-merge only if:

- local validation passed;
- `make verify` passed unless the route documents a narrower equivalent;
- relevant fixture smoke validation passed when the changed surface needs it;
- `git diff --check` passed;
- GitHub checks passed, or no GitHub checks exist and local integration
  validation passed;
- staged and tracked artifact guards passed;
- no generated artifacts, secrets, or generated/private telemetry are staged;
- required reviews return `MERGE_READY: yes`;
- no conflicts;
- cleanup safety is confirmed for branches and worktrees;
- branch is not main.

Every required review final response must begin with exactly `MERGE_READY: yes`
or `MERGE_READY: no`. Missing, malformed, or negative review output blocks
merge.

Review routing:

- ML/research changes require `netguard-ml-research-architect` and
  `netguard-integration-reviewer`.
- Safety/privacy/artifact/capture changes require
  `netguard-product-security-reviewer` and `netguard-integration-reviewer`.
- Shared model/eval/native/runtime contracts require
  `netguard-integration-reviewer` and `netguard-ml-research-architect`.
- Product architecture changes require `netguard-product-architect` and
  `netguard-integration-reviewer`.
- Low-risk docs-only changes may use integration review only when safety,
  artifact, cleanup, and merge policy are untouched.

After merge:

1. Switch to `main`.
2. Pull with `git pull --ff-only`.
3. Run final validation.
4. Run relevant fixture smoke validation when the route changed that surface.
5. Confirm clean `git status --short`.
6. Confirm the merged PR state and final `main` commit.
7. Delete the merged local branch.
8. Delete the merged remote branch when safe.
9. Remove associated worktrees that are clean and merged.
10. Run `git worktree prune`.
11. Confirm clean status on `main`.

Never delete unmerged branches or dirty worktrees.
