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
pytest -q tests/unit tests/integration -k "composer or model or registry or evaluation or pyod or river or disagreement or self_supervised or representation or token or embedding"
pytest -q tests/unit/test_detector_zoo.py tests/integration/test_detector_zoo_fixture.py
pytest -q tests/unit/test_score_row_composer.py tests/integration/test_score_row_composer_fixture.py
python -m pip check
make fixture-smoke
```

## Telemetry and feature foundation

```bash
pytest -q tests/unit tests/integration -k "telemetry or ingest or feature or evidence"
make fixture-smoke
```

## Evidence index and storage contract

```bash
pytest -q tests/unit/test_evidence_index.py tests/integration/test_evidence_index_fixture.py
pytest -q tests/unit tests/integration -k "telemetry or feature or evidence or index or investigation or detection or evaluation or registry"
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

## Time-series forecast and residual changes

```bash
pytest -q tests/unit/test_time_series_forecast.py tests/unit/test_time_series_residual.py tests/integration/test_time_series_residual_fixture.py
pytest -q tests/unit tests/integration -k "time_series or residual or composer or disagreement or evaluation or registry or evidence or investigation or detection"
pytest -q tests/unit/test_rust_core_scaffold.py tests/unit/test_qt_workstation_scaffold.py
make fixture-smoke
make verify-rust-core
python -m pip check
```

## UI changes

```bash
pytest -q tests/unit tests/integration -k "dashboard or qt or ui"
```

No validation may commit generated PCAPs, Parquet outputs, model binaries, notebooks with private outputs, or runtime logs.
