# Parallel Worktree Lanes

## Principle

Read-only subagents may analyze in parallel in one checkout. Writer agents require isolated worktrees or strict serial execution.

## Lane manifest

Every worktree lane must have:

```text
lane_id:
branch:
worktree_path:
objective:
allowed_files:
forbidden_files:
dependencies:
validation:
commit_message:
pr_title:
merge_policy:
stop_conditions:
```

## Good parallel candidates

- docs and UI copy
- independent detector prototype under its own module
- independent graph feature baseline
- independent time-series residual prototype
- independent agentic investigation prototype
- tests for an already-fixed contract

## Bad parallel candidates

- shared feature schema
- model registry schema
- Makefile/requirements/CI
- native inference contracts
- generated artifact guard
- orchestrator skill
- merge policy
- capture safety gate

## Cleanup

After successful merge:

```bash
git switch main
git pull --ff-only
make verify
git worktree remove <path>
git branch -d <branch>
git worktree prune
```
