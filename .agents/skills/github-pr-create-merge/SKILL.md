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
- required review routing.

After creating the PR during normal `$netguard-orchestrator` execution, do not
stop. Immediately continue into guarded merge-gate evaluation in the same run
unless validation, checks, mergeability, artifact/secret policy, required review
gates, branch safety, or explicit user instruction blocks merge.

## Merge

Auto-merge only if:

- local validation passed;
- GitHub checks passed, or no GitHub checks exist and local integration
  validation passed;
- artifact guard clean;
- required reviews return `MERGE_READY: yes`;
- no conflicts;
- branch is not main.

Every required review final response must begin with exactly `MERGE_READY: yes`
or `MERGE_READY: no`. Missing, malformed, or negative review output blocks
merge.

After merge:

1. Switch to `main`.
2. Pull with `git pull --ff-only`.
3. Run final validation.
4. Confirm the merged PR state and final `main` commit.
5. Delete the merged remote branch when safe.
6. Delete the merged local branch.
7. Remove associated worktrees.
8. Run `git worktree prune`.
9. Confirm clean status on `main`.
