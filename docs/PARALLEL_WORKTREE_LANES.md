# Parallel Worktree Lanes

## Principle

Read-only subagents may analyze in parallel in one checkout. Writer agents
require isolated worktrees or strict serial execution.

Worktree required: yes for two or more concurrent writer agents or independent
implementation lanes. Worktree required: no for read-only subagents, serial
single-writer work, docs-only serial work, merge/review-only routes, or
plan-only output.

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

- schemas
- model score contracts
- feature contracts
- model artifact contracts
- Makefile/requirements/CI
- `AGENTS.md`
- orchestrator skills
- native inference contracts
- generated artifact guard
- validation policy
- merge policy
- capture safety gate
- storage migrations
- product runtime interfaces

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

Only remove associated worktrees that are clean and whose branch has already
merged. Never delete unmerged branches or dirty worktrees. Delete the merged
remote branch only when the PR branch is no longer needed and branch ownership
is clear.
