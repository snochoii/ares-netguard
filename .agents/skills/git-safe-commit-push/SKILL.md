---
name: git-safe-commit-push
description: Safely commit and push source/docs/config/test changes after validation and artifact checks pass.
---


# Git Safe Commit Push

## Flow

1. Run validation required by milestone.
2. Run `git diff --check`.
3. Run artifact guard.
4. Inspect `git status --short`.
5. Stage only source, tests, docs, config, scripts, synthetic fixtures.
6. Commit with the planned message.
7. Push current branch.

## Never stage

- `.venv/`
- `.env*`
- PCAPs
- Parquet
- joblib/pkl/model binaries
- data runtime outputs
- private logs
- runtime artifacts
