---
name: netguard-integration-merge
description: Integrate and merge ready branches/PRs with required validation, reviews, and cleanup.
---


# Integration Merge

Use for ready PRs or branches.

## Steps

1. List open PRs and branches.
2. Check mergeability.
3. Check validation summaries.
4. Run required read-only reviews.
5. If `MERGE_READY: yes`, merge via configured method.
6. Switch to main and pull.
7. Run `make verify`.
8. Confirm clean status.
9. Delete merged branch and worktree.
10. Prune worktrees.

Never merge when validation, artifact guard, or review gates fail.
