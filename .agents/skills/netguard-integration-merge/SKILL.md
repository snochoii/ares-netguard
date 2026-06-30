---
name: netguard-integration-merge
description: Integrate and merge ready branches/PRs with required validation, reviews, and cleanup.
---


# Integration Merge

Use for ready PRs or branches, including PRs just created earlier in the same
normal `$netguard-orchestrator` run.

## Steps

1. List open PRs, current branch, pushed unmerged branches, open worktrees, and
   dirty status.
2. Check mergeability and base branch.
3. Check validation summaries, including `make verify`, route-specific fixture
   smoke when relevant, `git diff --check`, and staged/tracked artifact guards.
4. Confirm no generated artifacts, secrets, or generated/private telemetry are
   staged.
5. Confirm GitHub checks passed, or no checks exist and local integration
   validation is accepted as the fallback.
6. Run required read-only reviews.
7. Require every review final response to begin with exactly
   `MERGE_READY: yes` or `MERGE_READY: no`.
8. If all required reviews return `MERGE_READY: yes`, merge via configured
   method.
9. Switch to `main`.
10. Pull with `git pull --ff-only`.
11. Run `make verify` and any route-specific smoke validation.
12. Confirm clean `git status --short`.
13. Confirm the merged PR state and final `main` commit.
14. Delete the merged local branch.
15. Delete the merged remote branch when safe.
16. Remove any associated worktree lanes that are clean and merged.
17. Run `git worktree prune`.
18. Confirm clean status on `main`.

Never merge when validation, artifact guard, checks, mergeability, branch safety,
or review gates fail. Missing or malformed `MERGE_READY` review output is a
failed review gate.

Never delete unmerged branches or dirty worktrees.
