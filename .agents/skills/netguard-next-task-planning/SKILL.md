---
name: netguard-next-task-planning
description: Plan-only skill that maps repo state to the highest-leverage next experimental AI-NDR milestone.
---


# Next Task Planning

Analyze current repo state against `AGENTS.md`, `docs/ROADMAP.md`, and `docs/PROGRESS_RUBRIC.md`.

Do not edit files.

## Output

```text
Current repo state:
Completed capabilities:
Missing capabilities:
Progress:
Highest leverage next milestone:
Why:
Route:
Subagent decision:
  Read-only subagents:
  Implementation subagents:
  Review subagents:
  Selected agents:
  Why used:
  Why skipped:
Parallel decision:
  Selected route:
  Parallel selected:
  Lane candidates:
  Rejected lanes:
  Shared chokepoints:
Worktree decision:
  Worktrees required:
  Why:
Parallel eligible:
Technology policy impact:
Files likely to change:
Validation:
Required reviews:
Commit message:
Merge policy:
```

## Priority order

1. validation/safety breakage
2. completed open PRs, pushed unmerged branches, or blocked merge gates that can be finished safely
3. missing progress/roadmap contracts
4. model disagreement engine
5. time-series foundation residual anomaly
6. self-supervised traffic representation
7. temporal security graph
8. agentic investigation layer
9. detection engineering candidates
10. native inference adapters
11. Qt/QML workstation shell
12. Rust/C++ runtime

## Subagents and worktrees

Read-only subagents may run in the same checkout for exploration, research,
security/privacy, integration, test/eval, and product architecture review.
Implementation subagents that write concurrently require isolated Git
worktrees.

Worktree required: yes for two or more concurrent writer agents or independent
implementation lanes. Worktree required: no for read-only subagents, serial
single-writer work, docs-only serial work, merge/review-only routes, or
plan-only output.

Reject parallel lanes that touch shared chokepoints: schemas, model score
contracts, feature contracts, model artifact contracts, `Makefile`,
requirements files, `AGENTS.md`, orchestrator skills, artifact guards,
validation policy, storage migrations, or product runtime interfaces.
