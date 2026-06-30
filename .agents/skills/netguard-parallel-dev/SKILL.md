---
name: netguard-parallel-dev
description: Split independent work into Git worktree lanes and run worktree-isolated writer agents when parallelism is safe.
---


# Parallel Development

Use when two or more independent tasks can be safely implemented without
shared-file conflicts.

Worktree required: yes for two or more concurrent writer agents or independent
implementation lanes. Worktree required: no for read-only subagents, serial
single-writer work, docs-only serial work, merge/review-only routes, or
plan-only output.

## Required steps

1. Confirm clean main.
2. Check open PRs, pushed unmerged branches, dirty worktrees, and blocked merge
   gates before selecting new feature lanes.
3. Identify lane candidates.
4. Reject lanes touching shared chokepoints.
5. Create lane manifests.
6. Create worktrees with `git worktree add -b codex/<task> <path> main`.
7. Assign each writer to one worktree only.
8. Validate each lane.
9. Commit/push each lane.
10. Create PRs.
11. Run integration review and other required read-only review gates.
12. Merge only when policy allows.
13. Cleanup merged worktrees.

## Never

- never run multiple writers in the same checkout;
- never let two lanes touch the same schema, model score contract, feature
  contract, model artifact contract, `Makefile`, requirements file,
  `AGENTS.md`, orchestrator skill, artifact guard, validation policy, storage
  migration, or product runtime interface;
- never merge a lane with failed validation.
