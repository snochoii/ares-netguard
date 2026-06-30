---
name: netguard-integration-merge
description: Integrate and merge ready branches/PRs with required validation, reviews, and cleanup.
---


# Integration Merge

Use for ready PRs or branches, including PRs just created earlier in the same
normal `$netguard-orchestrator` run.

## Steps

1. List open PRs and branches.
2. Check mergeability.
3. Check validation summaries.
4. Run required read-only reviews.
5. Require every review final response to begin with exactly
   `MERGE_READY: yes` or `MERGE_READY: no`.
6. If all required reviews return `MERGE_READY: yes`, merge via configured
   method.
7. Switch to `main`.
8. Pull with `git pull --ff-only`.
9. Run `make verify` and any route-specific smoke validation.
10. Confirm the merged PR state, final `main` commit, and clean status.
11. Delete the merged local branch.
12. Delete the merged remote branch when safe.
13. Remove any associated worktree lanes.
14. Run `git worktree prune`.
15. Confirm clean status on `main`.

Never merge when validation, artifact guard, checks, mergeability, branch safety,
or review gates fail. Missing or malformed `MERGE_READY` review output is a
failed review gate.
