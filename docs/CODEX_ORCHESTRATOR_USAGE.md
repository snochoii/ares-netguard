# Codex Orchestrator Usage

## Plan first

```text
/plan $netguard-orchestrator
```

Plan mode may use read-only subagents. It must not edit files, branch, commit, push, PR, merge, or cleanup.

## Execute

```text
$netguard-orchestrator
```

The orchestrator may choose:

- single milestone;
- parallel worktree lanes;
- finish open PRs;
- integration merge;
- commit/push only;
- safety cleanup;
- plan-only.

## Explicit permission

Calling `$netguard-orchestrator` is explicit permission to use subagents and parallel worktree lanes when high leverage. The user does not need to separately say "use subagents."

## Expected final report

```text
Selected route:
Progress before:
Progress after:
Validation:
Commit:
Push:
PR:
Merge:
Cleanup:
Next milestone:
```
