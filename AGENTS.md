# AGENTS.md — ARES NetGuard-ML Codex Repository Contract

ARES NetGuard-ML is a local-first Experimental AI-NDR Workstation. It is an
experimental AI layer for authorized network and host security telemetry, not a
commercial NDR clone, generic packet viewer, or Streamlit product.

## Instruction and authority model

Follow system and developer instructions first, then the user's request, then
the nearest applicable `AGENTS.md`. Skills and custom agents refine execution;
they do not expand the user's authority.

Routing and mutation authority are separate:

- Implicit skill routing authorizes only workflow selection and safe read-only
  discovery.
- Plan, audit, explain, review, and research requests are read-only unless the
  user separately authorizes changes.
- Implement, change, build, or fix requests authorize bounded repository-local
  edits and validation for that request.
- Commit, push, PR creation, merge, branch deletion, worktree cleanup, and other
  remote or destructive actions require explicit user authorization or an
  accepted implementation plan that explicitly includes them.
- Never infer broader authority from `$netguard-orchestrator`, another skill
  name, an agent role, or a task packet.
- If the requested authority is unclear, continue with useful read-only work
  and report the missing authority instead of mutating state.

## Default routing

Use `$netguard-orchestrator` as the default workflow router unless the user
explicitly requests a lower-level skill. Domain expertise belongs in the
matching `.agents/skills/<skill>/SKILL.md`; the root contract owns only durable
repository-wide invariants.

Important execution skills:

- `$netguard-parallel-dev`
- `$netguard-worktree-lane-worker`
- `$netguard-integration-merge`
- `$git-safe-commit-push`
- `$github-pr-create-merge`

Skill discovery is not proof that a delegated child read or applied the skill.
Delegated implementation packets must require an explicit skill acknowledgment
before editing.

## Product and technology boundaries

Preserve the doctrine and technology ownership in:

- `docs/EXPERIMENTAL_AI_NDR_STRATEGY.md`
- `docs/TECHNOLOGY_SELECTION_POLICY.md`
- `docs/MERGE_POLICY.md`
- `docs/SAFETY_AND_PRIVACY.md`

Python owns research, training, evaluation, explainability, model export, and
fast-changing experimental models. Rust/C++ own reliable product runtime,
storage, capture boundaries, process supervision, and suitable native
inference. Qt/QML owns the professional native workstation UI. Streamlit is a
developer/debug UI only. Do not move working research code across technology
boundaries for aesthetics.

When a task changes a language, runtime, UI, storage, capture, packaging, or
inference boundary, report:

```text
Selected technology:
Why this technology:
Why not Python/Rust/C++/Qt for this milestone:
Migration path if this is a prototype:
Production-readiness implication:
```

## Safety and artifact invariants

1. Defensive monitoring and detection only.
2. No public scanning, exploitation, third-party probing, or unauthorized
   capture.
3. Live capture requires explicit authorization for the target interface or
   network.
4. Tests use synthetic fixtures by default.
5. Never commit secrets, private telemetry, runtime outputs, generated PCAPs,
   feature stores, model binaries, databases, or large generated artifacts.
6. Generated investigation hypotheses and detection candidates require human
   approval; never deploy rules automatically.
7. Preserve WSL and Python-venv reproducibility.

Before staging, run the repository artifact guard. The following remain
prohibited unless a narrowly documented synthetic-fixture allowlist applies:

```text
.venv/  .env  .env.* except .env.example  *.pcap  *.pcapng  *.parquet
*.joblib  *.pkl  *.onnx  *.pt  *.pth  *.ckpt  *.safetensors
*.db  *.sqlite  *.duckdb  *.jsonl
data/** except .gitkeep  .runtime/**  artifacts/**
```

## Branch and write isolation

- Normal implementation commits directly on `main` are forbidden.
- Before the first implementation edit, record the base SHA and switch to a
  dedicated non-main branch. Recheck the branch immediately before staging and
  committing.
- A single writer may work serially in the current checkout on its dedicated
  branch.
- Two or more concurrent writers require separate branches and isolated Git
  worktrees, with explicit file ownership.
- Never let concurrent writers touch the same file or shared chokepoint.
- Do not delete an unmerged branch or dirty worktree.

Shared chokepoints execute serially under one integration owner. They include:

- `AGENTS.md`, `.codex/config.toml`, `.codex/agents/**`, and orchestration skills;
- `Makefile`, requirements, artifact guards, and validation policy;
- schemas, feature/model/evaluation contracts, storage migrations;
- dashboard/model interfaces and product runtime interfaces.

## Delegation contract

Before each batch that depends on subagents, inspect the currently visible
spawn schema and verify named-agent selection, `fork_turns`, effective sandbox,
available concurrency, and visible model/effort overrides. Do not enable
`MultiAgentV2` manually.

Use these execution paths in order:

1. A verified named custom agent.
2. A generic child with a complete task packet.
3. Root-thread serial execution of the same packet.

Named or heterogeneous children must use `fork_turns: "none"`. If named-agent
selection is absent or fails, use a generic child. If spawning, skill loading,
or permission verification fails, use the root-thread serial fallback. A batch
must not fail only because named agents are unavailable.

Every delegated implementation packet must contain:

```text
skill_name:
skill_path:
objective:
base_sha:
worktree_path:
branch:
owned_paths:
forbidden_paths:
required_tests:
stopping_conditions:
result_contract:
```

Before editing, the child must locate and read the exact `SKILL.md`, then return:

```text
STATUS: ready
SKILL_ACK: <skill name>
SKILL_PATH: <exact path>
BASE_SHA: <sha>
CWD: <path>
BRANCH: <branch>
```

If the skill cannot be found or read, the child must not edit and must return:

```text
STATUS: capability_failure
CAPABILITY: required_skill
SKILL_NAME: <skill name>
SKILL_PATH: <exact path>
ROOT_ACTION: execute_same_packet_serially
```

Writer results must report status, skill acknowledgment, base/head SHA, changed
paths, tests, unresolved risks, and recommended parent action. The root agent
remains responsible for dependency decisions, conflict resolution, integration,
final validation, and final judgment.

### Read-only agent safety

`sandbox_mode = "read-only"` in an agent TOML is a declaration, not proof of
the effective child sandbox. Before delegated read-only work, verify effective
runtime permissions. If the sandbox cannot be verified, do not delegate that
work; perform it serially in the root thread. A read-only agent must not edit,
stage, commit, push, or create/merge PRs.

## Review and merge contract

Every merge-gating reviewer must bind its result to the exact reviewed head SHA.
Its final response must begin with exactly two lines:

```text
MERGE_READY: yes
HEAD_SHA: <reviewed_head_sha>
```

or:

```text
MERGE_READY: no
HEAD_SHA: <reviewed_head_sha>
```

The marker is valid only on the first line. Prose containing it elsewhere is
not approval. Missing or malformed markers, a missing/mismatched SHA, and every
`MERGE_READY: no` result block merge. Any head change invalidates all earlier
review results and requires fresh review.

Follow `docs/MERGE_POLICY.md` for active reviewer routing. Merge and cleanup are
allowed only when explicitly authorized and all local validation, artifact,
review, CI, mergeability, and branch-safety gates pass.

## Validation

Use the narrowest relevant checks, then the repository-wide gate when required:

```bash
make verify
git diff --check
bash scripts/check_no_generated_artifacts.sh --staged
bash scripts/check_no_generated_artifacts.sh --tracked
git status --short
```

Use relevant unit/integration/smoke tests for the changed surface. Fixture output
must go to `/tmp` or gitignored `data/`. Never weaken verification merely to
reduce tokens or elapsed time.

## Completion and reporting

Report actual validation, branch, commit, push, PR, merge, cleanup, and remaining
risks. Product capability progress is separate from Codex workflow migration
status and must not be used as a routing or approval gate.

For orchestration decisions, report:

```text
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
```
