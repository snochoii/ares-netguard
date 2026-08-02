---
name: git-safe-commit-push
description: Create an explicitly authorized local Git commit and optionally push it after exact-scope, validation, branch, and artifact gates pass.
---


# Git Safe Commit Push

Own only an explicitly authorized local commit and an independently authorized
optional push. Do not create branches, create or update PRs, merge, or clean up.
Skill invocation grants no authority.

## Preconditions

1. Record the approved base SHA, current `HEAD`, current branch, and expected
   changed paths.
2. Hard stop when `git branch --show-current` is empty.
3. Hard stop when the current branch is `main`.
4. Hard stop when `git symbolic-ref -q HEAD` shows detached `HEAD`.
5. Compare `git status --short`, the unstaged diff, the staged diff, and the
   approved scope. Stop on every unrelated, unexplained, generated, or
   prohibited path.
6. Confirm the current branch and head have not drifted from the validated
   candidate.
7. Run every milestone-required test, `git diff --check`, and the tracked
   artifact guard. Stop on any failure.

## Commit

Confirm commit authority separately. Stage only exact authorized paths, then:

1. inspect `git diff --cached --name-status` and `git diff --cached`;
2. run the staged artifact guard;
3. recheck status, branch, attached `HEAD`, and candidate SHA;
4. create the planned commit only when the staged diff exactly matches the
   validated diff;
5. capture the resulting commit SHA and verify status contains no unexplained
   changes.

Never stage secrets, private logs or telemetry, unauthorized captures, or
runtime outputs. No fixture allowlist overrides this prohibition. Treat
`.venv/`, `.env*` except `.env.example`, PCAP/PCAPNG, Parquet, joblib/pickle,
model binaries, databases, `data/**`, `.runtime/**`, and `artifacts/**` as
prohibited unless an applicable repository policy explicitly allows a narrowly
documented synthetic fixture. Require every allowed synthetic fixture to
contain no secret or private telemetry.

## Push

Confirm push authority independently from commit authority. Without it, return
`PUSH_STATUS: not_authorized`. With it, push only the validated non-main branch
to the specified remote and ref. Stop on branch drift, remote mismatch,
non-fast-forward requirements, or any request to rewrite history.

## Result

Return:

```text
BRANCH: <non-main branch>
BASE_SHA: <approved base sha>
COMMIT_SHA: <created commit sha>
PUSHED_REMOTE_REF: not_authorized | not_pushed | <remote/ref>
VALIDATION_SUMMARY: <commands and results>
CHANGED_PATHS: <exact committed paths>
UNRESOLVED_RISKS: none | <risks>
```
