# Merge Policy

## Default

The orchestrator may merge when all automated gates pass and required read-only reviews return `MERGE_READY: yes`.

Normal `$netguard-orchestrator` execution must not stop at PR creation when
guarded auto-merge is allowed. After creating a PR, the orchestrator must
continue immediately in the same run into merge-gate evaluation unless a
documented gate blocks merge.

## Required review gates

| Change type | Required review |
|---|---|
| safety/privacy/capture/telemetry | netguard-product-security-reviewer |
| model/eval/native inference contracts | netguard-integration-reviewer + netguard-ml-research-architect |
| experimental AI claim docs | netguard-ml-research-architect |
| Qt/Rust/C++ product architecture | netguard-product-architect |
| agentic investigation / generated rules | netguard-product-security-reviewer + netguard-integration-reviewer |
| technology boundary / language / runtime / UI toolkit / packaging policy | netguard-product-architect + netguard-integration-reviewer |

Additional routing from `docs/TECHNOLOGY_SELECTION_POLICY.md` applies when a
technology choice touches ML frameworks, capture/telemetry, native inference,
storage, external services, or artifact policy.

## Review output contract

Every required read-only review gate must return a final response that begins
with exactly one of:

```text
MERGE_READY: yes
MERGE_READY: no
```

`MERGE_READY: yes` allows merge only when all other gates pass. `MERGE_READY:
no`, missing review output, or output that does not begin with one of the exact
markers blocks merge.

## Same-run merge gate

After a normal orchestrator run creates a PR, the next step in that same run is
guarded merge-gate evaluation:

1. Confirm the PR is based on the intended base branch and has a clean merge
   state.
2. Confirm local validation, `git diff --check`, and artifact guards passed.
3. Confirm GitHub checks passed, or that no checks exist and local integration
   validation is accepted as the gate.
4. Run the required read-only review gates and enforce the review output
   contract.
5. Merge using the configured method only if every gate passes.

Stopping after PR creation is allowed only when validation, checks,
mergeability, artifact/secret policy, required review output, branch safety, or
explicit user instruction blocks merge.

## Auto-merge allowed

Auto-merge is allowed if:

- local validation passed;
- GitHub checks passed, or no GitHub checks exist and local integration
  validation passed;
- no generated artifacts are staged;
- no secrets/private telemetry are staged;
- no conflict exists;
- required reviews passed;
- branch is pushed;
- PR body includes validation summary.
- technology selection and rejected alternatives are reported when the change
  affects a language, runtime, framework, UI toolkit, storage, capture,
  packaging, or inference boundary.

## Auto-merge forbidden

Do not auto-merge if:

- validation failed;
- generated artifacts are present;
- secret/private telemetry is detected;
- conflict exists;
- required review is missing, malformed, or negative;
- branch includes unrelated changes;
- branch rewrites history;
- live capture/probing was added without explicit safety documentation.
- unreviewed technology boundary, dependency, runtime, UI toolkit, storage,
  capture, packaging, or native inference changes are present.

## Post-merge cleanup

After a guarded merge:

1. Switch to `main`.
2. Pull with `git pull --ff-only`.
3. Run final validation.
4. Confirm the merged PR state and final `main` commit.
5. Confirm clean `git status --short`.
6. Delete the merged remote branch when safe.
7. Delete the merged local branch.
8. Remove associated worktree lanes.
9. Run `git worktree prune`.
10. Confirm clean status on `main`.
