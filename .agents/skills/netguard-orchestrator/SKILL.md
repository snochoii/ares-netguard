---
name: netguard-orchestrator
description: Master orchestrator for ARES NetGuard-ML. Plans and executes the highest-leverage safe route, including subagents, worktrees, implementation, validation, commit/push, PR, guarded merge, and cleanup.
---


# NetGuard Orchestrator Skill

Invoking `$netguard-orchestrator` is explicit user permission to autonomously decide and run the safest highest-leverage workflow.

Do not say the user failed to request subagents, parallel work, commit, push, PR creation, or merge. The user did request them by invoking this skill.

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

## Normal mode

For `$netguard-orchestrator`:

1. Run preflight:
   - current branch;
   - `git status --short`;
   - `git remote -v`;
   - `gh auth status -h github.com` if available;
   - open PRs if GitHub CLI is available;
   - docs and capability map.

2. Spawn read-only subagents when useful:
   - codebase explorer;
   - product architect;
   - ML research architect;
   - security reviewer;
   - test/eval engineer.

3. Choose route:
   - `finish-open-prs` if an already validated branch/PR should be completed;
   - `integration-merge` if merge-ready PRs exist;
   - `commit-push-only` if validated local changes only need commit/push;
   - `safety-cleanup` if generated artifacts or unsafe staged files exist;
   - `plan-only` if the safe outcome is a decision-complete plan without mutation;
   - `parallel-worktree` if two or more independent non-conflicting high-leverage tasks exist;
   - `single-milestone` otherwise.

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
8. Commit and push using `$git-safe-commit-push`.
9. Create PR using `$github-pr-create-merge`, then immediately continue into
   guarded merge-gate evaluation in the same run.
10. If merge gates pass, merge and complete post-merge cleanup. If any gate
    blocks merge, stop after reporting the exact blocker.
11. Report progress, technology selection, and next task.

Creating a PR is not a terminal success state for normal `$netguard-orchestrator`
execution. Do not stop at PR creation unless validation, CI/checks, mergeability,
required reviews, artifact/secret policy, branch safety, or explicit user
instruction blocks guarded auto-merge.

Every required read-only review gate must return a final response that begins
with exactly `MERGE_READY: yes` or `MERGE_READY: no`. Missing, malformed, or
negative review output blocks merge.

## Hard stops

Stop without commit/merge if:

- validation fails;
- generated artifacts are staged/tracked incorrectly;
- secrets/private telemetry are detected;
- unreviewed technology boundary, dependency, runtime, UI toolkit, storage,
  capture, packaging, or native inference changes are present;
- merge conflicts exist;
- required review output is missing or does not begin with exactly
  `MERGE_READY: yes` or `MERGE_READY: no`;
- required review returns `MERGE_READY: no`;
- live capture/probing appears without explicit authorized safety contract;
- multiple writer lanes would touch shared chokepoints.

## Final response

Report:

```text
Selected route:
Current progress before route:
Expected progress after route:
Confidence:
Selected technology:
Why this technology:
Why not Python/Rust/C++/Qt for this milestone:
Migration path if this is a prototype:
Production-readiness implication:
Completed capabilities:
Missing capabilities:
Why this percentage:
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
