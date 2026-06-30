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
   - `safety-cleanup` if generated artifacts or unsafe staged files exist;
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

5. Implement only bounded milestones.
6. Validate.
7. Commit and push using `$git-safe-commit-push`.
8. Create PR and guarded merge using `$github-pr-create-merge`.
9. Cleanup merged branch/worktree.
10. Report progress and next task.

## Hard stops

Stop without commit/merge if:

- validation fails;
- generated artifacts are staged/tracked incorrectly;
- secrets/private telemetry are detected;
- merge conflicts exist;
- required review returns `MERGE_READY: no`;
- live capture/probing appears without explicit authorized safety contract;
- multiple writer lanes would touch shared chokepoints.

## Final response

Report:

```text
Selected route:
Progress before:
Progress after:
Confidence:
Subagents used:
Worktrees used:
Validation:
Commit:
Push:
PR:
Merge:
Cleanup:
Next milestone:
```
