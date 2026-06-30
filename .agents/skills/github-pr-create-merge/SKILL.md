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
- required reviews.

## Merge

Auto-merge only if:

- local validation passed;
- CI passed or local integration validation is explicitly accepted;
- artifact guard clean;
- required reviews return `MERGE_READY: yes`;
- no conflicts;
- branch is not main.

After merge, cleanup branch/worktree.
