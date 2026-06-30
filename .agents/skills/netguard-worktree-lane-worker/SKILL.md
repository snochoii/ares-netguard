---
name: netguard-worktree-lane-worker
description: Implement one bounded lane inside one isolated Git worktree according to a lane manifest.
---


# Worktree Lane Worker

Operate only inside the assigned worktree and only on allowed files.

A lane worker is required only for a worktree-backed implementation lane.
Worktree required: yes for two or more concurrent writer agents or independent
implementation lanes. Worktree required: no for read-only subagents, serial
single-writer work, docs-only serial work, merge/review-only routes, or
plan-only output.

## Required

- Read lane manifest.
- Confirm branch and worktree path.
- Modify only allowed files.
- Avoid shared chokepoints unless the lane manifest explicitly owns them:
  schemas, model score contracts, feature contracts, model artifact contracts,
  `Makefile`, requirements files, `AGENTS.md`, orchestrator skills, artifact
  guards, validation policy, storage migrations, and product runtime
  interfaces.
- Run targeted validation.
- Run artifact guard.
- Commit/push only if validation passes.

## Stop if

- allowed files are unclear;
- forbidden file would be needed;
- generated artifact appears;
- validation fails;
- another branch has a dependency not merged.
- cleanup would delete an unmerged branch or dirty worktree.
