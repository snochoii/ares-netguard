---
name: git-safe-commit-push
description: Safely commit and push source/docs/config/test changes after validation and artifact checks pass.
---


# Git Safe Commit Push

This skill does not grant commit or push authority. Use it only when the user or
an accepted implementation plan explicitly authorizes those actions.

## Flow

1. Record the base and current head SHA.
2. Run `git branch --show-current` and stop if it is empty or `main`.
3. Run validation required by the milestone.
4. Run `git diff --check`.
5. Run artifact guards for staged and tracked files.
6. Inspect `git status --short` and the exact diff.
7. Stage only authorized source, tests, docs, config, scripts, and synthetic fixtures.
8. Recheck that the branch is not `main`, then commit with the planned message.
9. Push only when push was explicitly authorized.

Never commit normal implementation directly on `main`. Stop on branch drift,
unexpected files, failed validation, artifact-policy violations, or missing
commit/push authority.

## Never stage

- `.venv/`
- `.env*`
- PCAPs
- Parquet
- joblib/pkl/model binaries
- data runtime outputs
- private logs
- runtime artifacts
