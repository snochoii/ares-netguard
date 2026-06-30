---
name: netguard-worktree-lane-worker
description: Implement one bounded lane inside one isolated Git worktree according to a lane manifest.
---


# Worktree Lane Worker

Operate only inside the assigned worktree and only on allowed files.

## Required

- Read lane manifest.
- Confirm branch and worktree path.
- Modify only allowed files.
- Run targeted validation.
- Run artifact guard.
- Commit/push only if validation passes.

## Stop if

- allowed files are unclear;
- forbidden file would be needed;
- generated artifact appears;
- validation fails;
- another branch has a dependency not merged.
