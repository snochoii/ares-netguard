---
name: github-pr-create-merge
description: Perform authorized GitHub PR create, update, read, checks, and merge transport without routing reviews or deciding merge readiness.
---


# GitHub PR Transport

Own GitHub PR create, update, read, checks lookup, and an already-authorized
merge execution. Do not route reviews, parse review receipts, decide readiness,
grant authority, or decide and perform cleanup.

## PR create or update

Require explicit PR create or update authority and a pushed non-main head.
Before mutation, verify the remote head SHA, intended base, repository, and
branch. Stop on an unpushed or mismatched head.

Include in the PR body:

- summary and exact changed scope;
- validation commands and results;
- staged and tracked artifact-guard results;
- privacy and safety impact;
- technology selection and rejected alternatives when applicable;
- subagent, parallel, and worktree decisions;
- required review categories as supplied by the root from current policy.

Return the PR number, URL, remote head SHA, base, and remote state. Reading PR
state or checks is transport evidence only and never produces a readiness
decision.

## Merge transport

Execute merge only after the root supplies this exact authorization from a
current ready integration gate:

```text
MERGE_EXECUTION: authorized
HEAD_SHA: <current_pr_head_sha>
PR_NUMBER: <number>
MERGE_METHOD: squash | merge | rebase
```

Immediately before the merge command, read the remote PR again and require its
repository, PR number, base, open state, and head SHA to match the authorization
and expected target. Hard stop if the remote PR head differs from `HEAD_SHA`,
if checks or mergeability changed adversely, or if any authorization field is
missing or malformed. Return `STATUS: authority_failure` when no valid
`MERGE_EXECUTION` receipt exists.

Do not reinterpret validation or reviews and do not independently approve the
merge. Do not switch branches, pull, delete local or remote branches, remove
worktrees, or prune.

## Transport result

Return:

```text
REMOTE_STATUS: created | updated | open | checks_reported | merged | blocked
PR_NUMBER: <number>
PR_URL: <url>
REMOTE_HEAD_SHA: <sha>
MERGE_COMMIT: not_applicable | none | <sha>
CHECKS_STATUS: passed | pending | failed | none
```
