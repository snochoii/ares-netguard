---
name: netguard-parallel-dev
description: Decide whether implementation can run in parallel and define isolated branch, worktree, and path topology for non-conflicting lanes.
---


# Parallel Development Topology

Own parallel eligibility, shared-chokepoint classification, and lane topology.
Do not implement, commit, push, review, create or merge PRs, or clean up
worktrees.

## Eligibility

Select parallel writers only when two or more tasks are independent and all
paths can be partitioned without overlap. Otherwise select a serial route.

Fix one 40-character base SHA for the complete batch. For every accepted lane,
define:

- one dedicated non-main branch;
- one absolute isolated worktree path;
- explicit, repository-relative `owned_paths`;
- explicit, repository-relative `forbidden_paths`;
- required tests and stopping conditions.

Require every writer to operate in exactly one assigned worktree. A serial
single writer, read-only work, review, integration, merge, or planning does not
require an additional worktree.

## Conflict rules

Reject parallel execution for shared chokepoints, including:

- root instructions, `.codex/config.toml`, `.codex/agents/**`, and
  orchestration skills;
- `Makefile`, dependency files, artifact guards, and validation policy;
- schemas, feature, model, and evaluation contracts;
- storage migrations, dashboard/model boundaries, and product runtime
  interfaces;
- integration, readiness, merge, and cleanup state.

Shared chokepoints cannot be assigned to a lane. If two proposed lanes require
the same file or bounded directory, route all related work serially. Reject a
packet whose owned and forbidden paths overlap, whose owned paths overlap
another lane, or whose owned paths include a shared chokepoint.

## Handoff

Return topology only:

```text
PARALLEL_SELECTED: yes | no
BASE_SHA: <40-character commit SHA>
LANES: <branch, absolute worktree, owned paths, forbidden paths>
SERIAL_WORK: none | <shared or overlapping work>
REJECTED_LANES: none | <lane and reason>
```

Hand accepted lane definitions to `$netguard-worktree-lane-worker`. Hand
commit or push to `$git-safe-commit-push`, PR transport to
`$github-pr-create-merge`, and readiness or post-merge verification to
`$netguard-integration-merge`.
