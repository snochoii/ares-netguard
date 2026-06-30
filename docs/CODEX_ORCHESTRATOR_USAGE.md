# Codex Orchestrator Usage

## Plan first

```text
/plan $netguard-orchestrator
```

Plan mode may use read-only subagents. It must not edit files, branch, commit, push, PR, merge, or cleanup.

Every plan must report selected technology, why it was chosen, why the other
major technology boundaries were not chosen, prototype migration path, and
production-readiness implication.

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

For normal execution, PR creation is not the end of the run when guarded
auto-merge is allowed. The expected flow is:

```text
implement bounded milestone
  -> validate
  -> commit and push
  -> create PR
  -> evaluate guarded merge gates in the same run
  -> merge only if all gates pass
  -> post-merge validation and cleanup
```

The orchestrator stops after PR creation only when validation, checks,
mergeability, artifact/secret policy, required review output, branch safety, or
explicit user instruction blocks merge.

## Explicit permission

Calling `$netguard-orchestrator` is explicit permission to use subagents and parallel worktree lanes when high leverage. The user does not need to separately say "use subagents."

It is also explicit permission to continue from PR creation into guarded
auto-merge evaluation and post-merge cleanup when policy gates allow it. The
user does not need to separately say "merge" after invoking the orchestrator.

## Review gate output

Every required read-only review gate must return a final response that begins
with exactly one of:

```text
MERGE_READY: yes
MERGE_READY: no
```

Missing review output, malformed output, or any `MERGE_READY: no` result blocks
merge.

## Technology selection

The orchestrator must infer language, runtime, UI toolkit, model runtime, and
sidecar choices from `docs/TECHNOLOGY_SELECTION_POLICY.md`. The user does not
need to manually tell Codex when to use Python, Rust, C++, Qt/QML, ONNX Runtime,
LightGBM native prediction, or a Python sidecar.

## Expected final report

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
Validation:
Commit:
Push:
PR:
Merge:
Cleanup:
Next highest-value milestone:
```
