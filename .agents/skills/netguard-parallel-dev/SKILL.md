---
name: netguard-parallel-dev
description: Split independent work into Git worktree lanes and run worktree-isolated writer agents when parallelism is safe.
---


# Parallel Development

Use when two or more independent tasks can be safely implemented without shared-file conflicts.

## Required steps

1. Confirm clean main.
2. Identify lane candidates.
3. Reject lanes touching shared chokepoints.
4. Create lane manifests.
5. Create worktrees with `git worktree add -b codex/<task> <path> main`.
6. Assign each writer to one worktree only.
7. Validate each lane.
8. Commit/push each lane.
9. Create PRs.
10. Run integration review.
11. Merge only when policy allows.
12. Cleanup merged worktrees.

## Never

- never run multiple writers in the same checkout;
- never let two lanes touch the same schema/contract/Makefile/requirements;
- never merge a lane with failed validation.
