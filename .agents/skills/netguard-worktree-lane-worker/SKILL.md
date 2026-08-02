---
name: netguard-worktree-lane-worker
description: Validate and execute one bounded delegated implementation packet inside its assigned isolated Git worktree, with canonical acknowledgment and completion results.
---


# Worktree Lane Worker

Own the delegated implementation packet, pre-edit acknowledgment, bounded lane
execution, and completion result. Do not expand scope, independently infer
mutation authority, modify shared chokepoints, create or merge PRs, or clean up
branches and worktrees.

## Canonical packet

Require this complete packet:

```text
skill_name: <frontmatter skill name>
skill_path: <repository-relative path to SKILL.md>
objective: <one bounded objective>
base_sha: <40-character commit SHA>
worktree_path: <absolute isolated worktree path>
branch: <non-main branch>
owned_paths:
  - <repository-relative file or bounded directory>
forbidden_paths:
  - <repository-relative file or directory>
required_tests:
  - <exact command>
stopping_conditions:
  - <explicit stop condition>
result_contract: lane_result_v1
```

Reject the packet before editing when a field is absent or malformed,
`result_contract` differs, owned and forbidden paths overlap, owned paths
overlap another lane, or owned paths contain a shared chokepoint. Shared
chokepoints include root instructions, `.codex/config.toml`,
`.codex/agents/**`, orchestration skills, `Makefile`, dependency files,
artifact guards, validation policy, schemas, feature/model/evaluation
contracts, storage migrations, dashboard/model boundaries, and product runtime
interfaces. Manifest ownership never overrides this prohibition.

## Pre-edit verification and acknowledgment

Before editing:

1. Resolve `skill_path` as a repository-relative path inside the assigned
   worktree; reject absolute paths and traversal outside the repository.
2. Read the exact `SKILL.md` completely and confirm its frontmatter `name`
   equals `skill_name`.
3. Confirm the current working directory equals the absolute `worktree_path`.
4. Confirm the worktree is isolated, `HEAD` equals `base_sha`, and the current
   non-main branch equals `branch`.
5. Confirm current status and diff are clean or exactly match packet-declared
   starting state.

If all checks pass, return this acknowledgment before any edit:

```text
STATUS: ready
SKILL_ACK: <skill_name>
SKILL_PATH: <skill_path>
BASE_SHA: <base_sha>
CWD: <absolute_worktree_path>
BRANCH: <branch>
```

If the skill is missing, unreadable, or mismatched, do not edit and return:

```text
STATUS: capability_failure
CAPABILITY: required_skill
SKILL_NAME: <skill_name>
SKILL_PATH: <skill_path>
ROOT_ACTION: execute_same_packet_serially
```

Stop before editing on any other packet, path, worktree, branch, SHA, authority,
or isolation mismatch and report the exact blocker.

## Bounded execution

Modify only `owned_paths`. Stop when the objective needs a forbidden path,
shared chokepoint, undeclared dependency, scope expansion, unauthorized live
capture or telemetry, prohibited artifact, or failed required test. Run every
`required_tests` command and the applicable diff and artifact checks.

Commit only when the packet explicitly authorizes commit and the user's
authority independently authorizes commit. Push only when the packet explicitly
authorizes push and the user's authority independently authorizes push. Route
either operation through `$git-safe-commit-push`.

Never create or update a PR, decide merge readiness, merge, delete a branch, or
remove a worktree.

## Completion result

Return exactly these fields for completed, blocked, or capability-failure
outcomes:

```text
STATUS: completed | blocked | capability_failure
SKILL_ACK: <skill_name>
SKILL_PATH: <skill_path>
BASE_SHA: <base_sha>
HEAD_SHA: <current_head_sha>
CWD: <absolute_worktree_path>
BRANCH: <branch>
CHANGED_PATHS: <exact list>
FORBIDDEN_PATHS_TOUCHED: none | <exact list>
TEST_RESULTS: <command and result list>
COMMIT_STATUS: not_authorized | not_created | <commit_sha>
PUSH_STATUS: not_authorized | not_pushed | <remote/ref>
UNRESOLVED_RISKS: none | <risks>
PARENT_ACTION: integrate | execute_same_packet_serially | inspect_blocker
```

Do not automatically retry a missing or malformed completion. If edits may
exist, require the root to inspect the worktree and block integration until its
state is understood.
