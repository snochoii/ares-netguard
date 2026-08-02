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
| safety/privacy/artifact/capture/telemetry | netguard-product-security-reviewer + netguard-integration-reviewer |
| ML/research changes | netguard-correctness-reviewer + netguard-integration-reviewer |
| model/eval/native inference contracts | netguard-correctness-reviewer + netguard-integration-reviewer |
| experimental AI claim docs | netguard-correctness-reviewer; add netguard-docs-api-researcher evidence for external/version/source claims and the applicable root-owned packet for product direction or product fit judgments |
| shared contracts | netguard-integration-reviewer |
| Qt/Rust/C++ product architecture | root-owned `architectural_tradeoff` packet + netguard-integration-reviewer |
| agentic investigation / generated rules | netguard-product-security-reviewer + netguard-integration-reviewer |
| technology boundary / language / runtime / UI toolkit / packaging policy | root-owned `technology_choice` packet + netguard-integration-reviewer |
| low-risk docs-only with no safety/artifact/cleanup/merge policy change | netguard-integration-reviewer |

Additional routing from `docs/TECHNOLOGY_SELECTION_POLICY.md` applies when a
technology choice touches ML frameworks, capture/telemetry, native inference,
storage, external services, or artifact policy.

`netguard-docs-api-researcher` is an evidence provider only. Its findings may
be required evidence for another review, but it does not issue a merge receipt
or decide merge readiness.

For product direction, technology rationale, architectural tradeoffs, and
product fit, the root owns a bounded `review_packet_v1` with this canonical
schema:

```text
review_category: product_direction | technology_choice | architectural_tradeoff | product_fit
objective: <one bounded review objective>
base_sha: <40-character base SHA>
head_sha: <40-character reviewed head SHA>
inspected_paths:
  - <repository-relative path>
required_evidence:
  - <exact evidence or command result>
decision_criteria:
  - <explicit criterion>
output_contract: sha_bound_review_v1
stopping_conditions:
  - <explicit stop condition>
```

Use `product_direction` for positioning or strategic capability direction,
`technology_choice` for a language, framework, runtime, UI toolkit, or
packaging choice, `architectural_tradeoff` for component topology or boundary
tradeoffs, and `product_fit` for whether milestone scope matches product
doctrine.

The root-owned packet is neither a persistent custom-agent definition nor
mutation authority. Its execution follows the repository delegation contract:
verify effective read-only enforcement for the exact surface, prefer named and
then generic execution with `fork_turns: "none"`, and use a verified
`SIMULATED_ROOT_SERIAL` path when delegation is unavailable or unverified.
The packet output is:

```text
MERGE_READY: yes | no
HEAD_SHA: <reviewed_head_sha>
REVIEW_CATEGORY: <review_category>
EXECUTION_MODE: generic | SIMULATED_ROOT_SERIAL
SANDBOX_VERIFIED: yes
FINDINGS: <summary>
UNRESOLVED_RISKS: none | <risks>
ROOT_ACTION: accept | reject | inspect
```

The root owns the packet and final judgment. An unverified child, stale SHA,
malformed output, or negative receipt blocks merge. Evidence from
`netguard-docs-api-researcher` may satisfy `required_evidence`, but does not
count as an independent merge receipt. `$netguard-integration-merge` remains
the sole merge-readiness gate.

## Review output contract

Every required read-only review gate must return a final response that begins
at byte one with exactly one of these two-line SHA-bound headers:

```text
MERGE_READY: yes
HEAD_SHA: <reviewed_head_sha>
```

or:

```text
MERGE_READY: no
HEAD_SHA: <reviewed_head_sha>
```

`MERGE_READY: yes` allows merge only when all other gates pass. `MERGE_READY:
no`, missing review output, or output that does not begin with one of the exact
headers blocks merge. The reviewed SHA must be one 40-character commit SHA and
must match the unchanged candidate head.

## Same-run merge gate

After a normal orchestrator run creates a PR, the next step in that same run is
guarded merge-gate evaluation:

1. Confirm the PR is based on the intended base branch and has a clean merge
   state.
2. Confirm local validation passed.
3. Confirm `make verify` passed unless the route documents a narrower
   equivalent.
4. Confirm relevant fixture smoke validation passed when the changed surface
   needs it.
5. Confirm `git diff --check` passed.
6. Confirm staged and tracked generated artifact guards passed.
7. Confirm no generated artifacts, secrets, or generated/private telemetry are
   staged.
8. Confirm GitHub checks passed, or that no checks exist and local integration
   validation is accepted as the gate.
9. Confirm no merge conflicts exist.
10. Run the required read-only review gates and enforce the review output
    contract.
11. Confirm branch and worktree cleanup is safe.
12. Merge using the configured method only if every gate passes.

The phrase "same-run merge" means PR creation is followed immediately by
merge-gate review, eligible merge, final `main` validation, and cleanup in the
same normal `$netguard-orchestrator` run unless a named gate blocks.

Stopping after PR creation is allowed only when validation, checks,
mergeability, artifact/secret policy, required review output, branch safety, or
explicit user instruction blocks merge.

## Auto-merge allowed

Auto-merge is allowed if:

- local validation passed;
- `make verify` passed unless the route documents a narrower equivalent;
- relevant fixture smoke validation passed when the changed surface needs it;
- `git diff --check` passed;
- GitHub checks passed, or no GitHub checks exist and local integration
  validation passed;
- staged and tracked artifact guards passed;
- no generated artifacts, secrets, or generated/private telemetry are staged;
- no conflict exists;
- required reviews passed;
- cleanup safety is confirmed for branches and worktrees;
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
4. Run relevant fixture smoke validation when the route changed that surface.
5. Confirm clean `git status --short`.
6. Confirm the merged PR state and final `main` commit.
7. Delete the merged local branch.
8. Delete the merged remote branch when safe.
9. Remove associated worktree lanes that are clean and merged.
10. Run `git worktree prune`.
11. Confirm clean status on `main`.

Never delete unmerged branches or dirty worktrees.
