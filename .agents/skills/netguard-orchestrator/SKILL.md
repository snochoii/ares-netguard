---
name: netguard-orchestrator
description: Master orchestrator for ARES NetGuard-ML. Selects and, within explicit user authority, executes the highest-leverage safe route.
---


# NetGuard Orchestrator Skill

Invoking `$netguard-orchestrator` selects the safest highest-leverage workflow.
It does not by itself authorize edits, branches, commits, pushes, PRs, merges,
cleanup, or other mutation. Derive mutation authority from the user's request or
an accepted implementation plan, following the repository `AGENTS.md`.

## Phase A safety contract

These requirements take precedence over older workflow prose in this skill:

- Treat implicit routing separately from mutation authority.
- For authorized implementation, record the base SHA and move to a dedicated
  non-main branch before editing. Recheck the branch before staging or commit.
- Execute shared chokepoints serially under one root integration owner.
- Before delegated work, revalidate the visible spawn schema, named-agent
  selection, `fork_turns`, effective sandbox, concurrency, and visible
  model/effort overrides.
- Use `fork_turns: "none"` for a named specialized or heterogeneous child.
- Fall back from a named child to a generic child with a complete task packet,
  then to root-thread serial execution. Do not fail a batch only because named
  selection or spawning is unavailable.
- A generic implementation packet must name the exact skill and `SKILL.md`
  path, objective, base SHA, worktree/branch, owned and forbidden paths,
  required tests, stopping conditions, and result contract. The child must
  acknowledge that it found and read that skill before editing; otherwise it
  must return `STATUS: capability_failure` without edits and the root must run
  the same packet serially.
- `sandbox_mode = "read-only"` in TOML is declarative only. Delegate read-only
  work only after effective sandbox verification; otherwise use the root.
- Merge-gating review output must use `MERGE_READY: yes` or
  `MERGE_READY: no` exactly on line one and `HEAD_SHA: <reviewed_head_sha>` on
  line two. A head change invalidates the result.
- Product capability progress is not a workflow routing or approval gate.

## Plan mode

For `/plan $netguard-orchestrator`:

- inspect repository state;
- spawn read-only subagents if useful;
- choose one route;
- choose and report technology using `docs/TECHNOLOGY_SELECTION_POLICY.md`;
- output plan only;
- do not edit files;
- do not create branches;
- do not commit, push, PR, merge, or cleanup.

## Authorized implementation mode

For `$netguard-orchestrator`:

1. Run preflight:
   - current branch;
   - `git status --short`;
   - `git remote -v`;
   - `gh auth status -h github.com` if available;
   - open PRs if GitHub CLI is available;
   - pushed unmerged branches;
   - open and dirty worktrees;
   - blocked merge gates from existing PRs;
   - docs and capability map.

2. Spawn read-only subagents when useful:
   - codebase explorer;
   - product architect;
   - ML research architect;
   - security reviewer;
   - test/eval engineer.

   Read-only subagents may run in the same checkout for exploration, research,
   security/privacy, integration, test/eval, and product architecture review.
   Implementation subagents that write concurrently require isolated Git
   worktrees. Skipping subagents requires a concrete reason, such as small
   docs-only work, narrow serial work, unavailable tools, sufficient context,
   plan-mode limits, or no review gate yet.

3. Choose route:
   - `finish-open-prs` if an already validated branch/PR should be completed;
   - `integration-merge` if merge-ready PRs exist;
   - `merge-only` when normalizing `finish-open-prs` or `integration-merge`
     as the selected merge-priority route;
   - `commit-push-only` if validated local changes only need commit/push;
   - `safety-cleanup` if generated artifacts or unsafe staged files exist;
   - `plan-only` if the safe outcome is a decision-complete plan without mutation;
   - `parallel-worktree` if two or more independent non-conflicting high-leverage tasks exist;
   - `single-milestone` otherwise.

   Before selecting new feature work, prioritize existing completed PRs or
   pushed unmerged branches that can be validated and merged.

   Worktree required: yes for two or more concurrent writer agents or
   independent implementation lanes. Worktree required: no for read-only
   subagents, serial single-writer work, docs-only serial work,
   merge/review-only routes, or plan-only output. Reject parallel lanes that
   touch shared chokepoints: schemas, model score contracts, feature contracts,
   model artifact contracts, `Makefile`, requirements files, `AGENTS.md`,
   orchestrator skills, artifact guards, validation policy, storage migrations,
   or product runtime interfaces.

4. Select highest-leverage next milestone from the experimental AI-NDR roadmap:
   - model disagreement engine;
   - time-series foundation residual anomaly;
   - self-supervised traffic representation;
   - temporal security graph;
   - agentic investigation;
   - detection engineering candidates;
   - native inference adapters;
   - Qt/QML workstation;
   - Rust/C++ runtime.

5. Select technology using `docs/TECHNOLOGY_SELECTION_POLICY.md`.
6. Implement only bounded milestones.
7. Validate.
8. If explicitly authorized, commit and push using `$git-safe-commit-push`.
9. If explicitly authorized, create a PR using `$github-pr-create-merge`, then
   continue into guarded merge-gate evaluation only when merge was also
   explicitly authorized.
10. If merge is authorized and all gates pass, merge and complete only the
    authorized cleanup. Otherwise report the exact blocker or authority limit.
11. Report workflow status, technology selection, and next task.

When the user explicitly authorized both PR creation and guarded merge, PR
creation is not a terminal success state: continue into merge-gate evaluation.
When only PR creation was authorized, stop after reporting the open PR.

Every required read-only review gate must return a final response beginning
with exactly `MERGE_READY: yes` or `MERGE_READY: no` on line one and
`HEAD_SHA: <reviewed_head_sha>` on line two. Missing, malformed, stale, or
negative review output blocks merge.

Every plan and final report must include:

```text
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
```

## Hard stops

Stop without commit/merge if:

- validation fails;
- generated artifacts are staged/tracked incorrectly;
- secrets/private telemetry are detected;
- unreviewed technology boundary, dependency, runtime, UI toolkit, storage,
  capture, packaging, or native inference changes are present;
- merge conflicts exist;
- required review output is missing, does not begin with exactly
  `MERGE_READY: yes` or `MERGE_READY: no`, omits the reviewed head SHA on line
  two, or is stale for the current head;
- required review returns `MERGE_READY: no`;
- live capture/probing appears without explicit authorized safety contract;
- multiple writer lanes would touch shared chokepoints.

## Final response

Report:

```text
Selected route:
Workflow migration status:
Confidence:
Selected technology:
Why this technology:
Why not Python/Rust/C++/Qt for this milestone:
Migration path if this is a prototype:
Production-readiness implication:
Subagent decision:
Parallel decision:
Worktree decision:
Completed workflow changes:
Remaining workflow risks:
Subagents used:
Worktrees used:
Validation:
Commit:
Push:
PR:
Merge:
Cleanup:
Next highest-value milestone:
```
