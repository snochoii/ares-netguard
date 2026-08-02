---
name: netguard-integration-merge
description: Evaluate the sole merge-readiness gate from exact SHA-bound evidence and verify the merged base without performing GitHub merge transport.
---


# Integration Merge Gate

Own review-receipt parsing, the sole merge-readiness decision, and post-merge
verification. Do not grant mutation authority, operate GitHub merge transport,
or decide cleanup.

## Active review routing

Determine required review categories from the current `AGENTS.md`,
`docs/MERGE_POLICY.md`, and `docs/TECHNOLOGY_SELECTION_POLICY.md`. Do not embed
reviewer names or duplicate their routing tables in this skill.

Accept a review only from an execution surface whose effective read-only
sandbox was verified for that run. Discard results from an unverified surface
regardless of their content.

## Receipt parsing

Require every review response to start at byte one with exactly one of these
two-line headers, with no leading blank line or prose:

```text
MERGE_READY: yes
HEAD_SHA: <reviewed_head_sha>
```

or:

```text
MERGE_READY: no
HEAD_SHA: <reviewed_head_sha>
```

Require `HEAD_SHA` to be one 40-character commit SHA. Treat a missing,
malformed, negative, duplicated, or extra-required receipt as blocking. Bind
each accepted receipt to its required review category and verified execution
surface. Optional metadata may follow the first two lines:

```text
REVIEWER: <role or SIMULATED root role>
EXECUTION_MODE: named | generic | SIMULATED_ROOT_SERIAL
SANDBOX_VERIFIED: yes
FINDINGS: <summary>
UNRESOLVED_RISKS: none | <risks>
```

## Readiness evaluation

Capture one candidate SHA. Require local `HEAD`, the remote PR head, and every
receipt `HEAD_SHA` to equal that candidate. Invalidate all receipts and every
earlier readiness result after any candidate-head change.

Also require:

- the intended base branch and a mergeable, conflict-free PR;
- passed milestone validation and repository-wide verification;
- passed `git diff --check` and staged/tracked artifact guards;
- no secrets, prohibited artifacts, or private/generated telemetry;
- passed required GitHub checks, or a policy-permitted no-check fallback with
  accepted local integration validation;
- an understood authorized diff with no unrelated changes;
- every required review category represented by one positive valid receipt.

Return the sole readiness decision in this exact shape:

```text
MERGE_GATE: ready | blocked
HEAD_SHA: <current_head_sha>
PR_NUMBER: <number>
REQUIRED_RECEIPTS: <received>/<required>
BLOCKERS: none | <blocking reasons>
```

Never execute a merge command. A ready gate reports evidence only; it does not
authorize merge. Require the root to check merge authority separately after a
ready result.

## Post-merge verification

After GitHub transport reports success, verify that the PR is merged, the
intended merge commit is present on the updated base branch, required
post-merge tests and artifact guards pass, and the base checkout is clean.
Return the merged base SHA, executed checks, status, and blockers to the root.
Leave branch deletion, remote deletion, worktree removal, and pruning to the
root's separately authorized cleanup judgment.
