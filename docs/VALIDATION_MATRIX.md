# Validation Matrix

## Always

```bash
make verify
git diff --check
bash scripts/check_no_generated_artifacts.sh --staged
bash scripts/check_no_generated_artifacts.sh --tracked
git status --short
```

## Docs-only strategy change

```bash
make verify
rg "Experimental AI-NDR|model disagreement|foundation|self-supervised|graph|agentic|native inference" AGENTS.md docs .agents .codex
```

## Docs-only technology policy change

```bash
make verify
git diff --check
bash scripts/check_no_generated_artifacts.sh --staged
bash scripts/check_no_generated_artifacts.sh --tracked
rg "TECHNOLOGY_SELECTION_POLICY|Selected technology|Why this technology|Why not Python/Rust/C\\+\\+/Qt|Migration path|Production-readiness implication" AGENTS.md docs .agents .codex
```

## Docs-only orchestration policy change

```bash
make verify
git diff --check
bash scripts/check_no_generated_artifacts.sh --staged
bash scripts/check_no_generated_artifacts.sh --tracked
rg "Subagent decision|Parallel decision|Worktree decision|MERGE_READY|same-run merge|worktree required|read-only subagents" AGENTS.md docs .agents
```

## Model changes

```bash
pytest -q tests/unit tests/integration -k "model or registry or evaluation or pyod or river or disagreement or self_supervised or representation or token or embedding"
make fixture-smoke
```

## Telemetry and feature foundation

```bash
pytest -q tests/unit tests/integration -k "telemetry or ingest or feature or evidence"
make fixture-smoke
```

## Agentic investigation

```bash
pytest -q tests/unit tests/integration -k "investigation or agentic or evidence"
```

## Graph changes

```bash
pytest -q tests/unit tests/integration -k "graph or temporal or edge"
```

## UI changes

```bash
pytest -q tests/unit tests/integration -k "dashboard or qt or ui"
```

No validation may commit generated PCAPs, Parquet outputs, model binaries, notebooks with private outputs, or runtime logs.
