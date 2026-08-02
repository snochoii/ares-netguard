---
name: netguard-orchestrator
description: Coordinate ARES NetGuard-ML work by interpreting user authority, selecting safe execution and fallback paths, sequencing integration, and making the final root judgment.
---


# NetGuard Orchestrator

Coordinate repository work without expanding the authority granted by the user
or an accepted plan. Keep product capability progress separate from workflow
routing and approval.

## Root coordinator ownership

Own user-goal interpretation, mutation-authority checks, preflight, dependency
ordering, execution-path selection, retry and fallback, integration sequencing,
and the final root judgment. Delegate only these bounded responsibilities:

- use `$netguard-parallel-dev` to decide lane and worktree topology;
- use `$netguard-worktree-lane-worker` for a delegated implementation packet;
- use `$git-safe-commit-push` for an authorized commit or push;
- use `$github-pr-create-merge` for GitHub transport only;
- use `$netguard-integration-merge` as the sole merge-readiness judge and for
  post-merge verification.

Do not own lane implementation details, review-receipt parsing, or GitHub
transport inside this skill.

## Preflight and authority

Before mutation:

1. Derive edit, branch, commit, push, PR, merge, and cleanup authority
   separately from the request or accepted plan. Skill invocation grants none
   of them.
2. Record the current branch, base SHA, `HEAD`, status, remotes, open PRs,
   pushed unmerged branches, worktrees, and known blocked gates.
3. For implementation, require a clean understood baseline and switch to a
   dedicated non-main branch before the first edit.
4. Read the repository authority, merge, technology, safety, and artifact
   policies relevant to the changed surface.
5. Recheck authority and branch state immediately before every staging,
   commit, push, PR, merge, or cleanup boundary.

Treat review, audit, explain, plan, and research requests as read-only unless
the user separately authorizes mutation. Execute all shared chokepoints
serially under the root integration owner.

## Execution-path selection

Before a delegated batch, inspect the currently visible spawn schema and
verify named-agent selection, `fork_turns`, effective sandbox, available
concurrency, and visible model or effort overrides. Never enable
`MultiAgentV2` manually.

Select execution in this order:

1. Use a verified named custom agent with `fork_turns: "none"` and a complete
   task packet.
2. If named selection is unavailable, rejected, or cannot use
   `fork_turns: "none"`, use a generic child with the same complete packet.
3. If spawning, skill loading, isolation, or permission verification fails,
   execute the unchanged packet root-serial and label the handoff `SIMULATED`.

Do not treat inherited context as a substitute for packet fields or skill
acknowledgment. Require isolated worktrees for concurrent writers. Use root
serial execution when an isolated writer worktree is unavailable or when work
touches a shared chokepoint.

A `SIMULATED` root handoff must report:

```text
EXECUTION_MODE: SIMULATED_ROOT_SERIAL
ACTUAL_SPAWN_COUNT_DELTA: 0
ACTUAL_HANDOFF_COUNT_DELTA: 0
```

## Effective sandbox

Treat an agent TOML `sandbox_mode = "read-only"` declaration as unverified.
Before trusting delegated read-only work:

1. Allocate an isolated disposable probe directory under `/tmp` for that exact
   execution surface.
2. Run a safe write-denial probe inside only that directory.
3. Trust the surface only when the write is denied and runtime permissions are
   otherwise consistent with read-only operation.
4. If the write succeeds or enforcement is ambiguous, discard the probe and
   every result from that child, safely remove only its probe artifact, and
   rerun the unchanged review packet through a verified CLI
   `--sandbox read-only` root-serial path marked `SIMULATED`.

Never accept an unverified child result as a merge-gating receipt.

## Retry and fallback

Classify failures before retrying:

- Retry an infrastructure failure on the same path at most once, and only
  when retrying is safe and cannot duplicate side effects.
- Do not automatically retry policy, authority, correctness, stale-SHA,
  malformed-result, or partial-write failures.
- After an eligible retry fails, move to the next execution path.
- Preserve the objective, scope, authority, packet, base SHA, required tests,
  and stopping conditions across fallback.
- If a malformed writer completion follows an edit, inspect that worktree
  directly and block integration until its state is understood.

## Integration sequence

Use this order for an authorized implementation:

1. Select serial or worktree topology.
2. Execute bounded implementation.
3. Validate the exact diff.
4. Commit and optionally push only with their separate authorities.
5. Create or update a PR only with PR authority and a pushed head.
6. Determine the final candidate SHA and required review categories from the
   current repository policies.
7. Obtain only effective-read-only review receipts bound to that SHA.
8. Ask `$netguard-integration-merge` for the sole `MERGE_GATE` decision.
9. If the gate is ready and merge authority exists, issue the exact
   `MERGE_EXECUTION` authorization for GitHub transport.
10. Ask `$netguard-integration-merge` to verify the merged base.
11. Make the final cleanup decision from explicit cleanup authority and the
    verified branch and worktree state.

PR creation is not terminal when the accepted plan explicitly authorizes a
same-run guarded merge. Stop after PR creation when merge authority is absent.

## Hard stops

Stop the affected mutation or gate when any of these conditions holds:

- authority is absent or ambiguous;
- the implementation branch is empty, `main`, detached, dirty for unexplained
  reasons, or no longer based on the recorded SHA;
- an edit would exceed authorized paths or touch a shared chokepoint in a
  parallel lane;
- validation, artifact, secret, privacy, branch-safety, mergeability, or check
  requirements fail;
- a required skill cannot be read or acknowledged;
- a child sandbox is unverified;
- a review receipt is missing, malformed, negative, or bound to a different
  SHA;
- local `HEAD`, PR head, authorization SHA, or review SHA disagree;
- the candidate head changes after validation or review;
- live capture, public probing, exploitation, or third-party telemetry appears
  without the required explicit authority and safety contract.

## Result reporting

Report the actual route, authority used, validation, branch, commit, push, PR,
merge, cleanup, blockers, and remaining risks. Include:

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
Completed workflow changes:
Remaining workflow risks:
Validation:
Commit:
Push:
PR:
Merge:
Cleanup:
Next highest-value milestone:
```
