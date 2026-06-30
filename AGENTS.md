# AGENTS.md — ARES NetGuard-ML Experimental AI-NDR Repository Contract

ARES NetGuard-ML is **not** an open-source clone of a commercial NDR product and is **not** merely a Wireshark-like packet viewer.

ARES NetGuard-ML is a **local-first Experimental AI-NDR Workstation**: a professional analyst and research platform that applies cutting-edge AI/ML techniques to network, host, and existing security telemetry in a transparent, reproducible, model-comparable way.

## Default Master Entry Point

```text
$netguard-orchestrator
```

Use `$netguard-orchestrator` unless the user explicitly asks for a lower-level skill.

Invoking `$netguard-orchestrator` is the user's explicit request for Codex to autonomously decide and run the highest-leverage safe workflow based on:

- the final experimental AI-NDR product goal;
- current repository state;
- current branch and GitHub PR state;
- validation health;
- open branches/worktrees;
- model roadmap gaps;
- safety and artifact policy.

This invocation explicitly permits Codex to use, when appropriate and policy-safe:

- read-only analysis subagents;
- single bounded milestone implementation;
- parallel Git worktree planning;
- worktree-isolated writer agents;
- validation;
- safe commit and push;
- GitHub PR creation;
- guarded auto-merge;
- post-merge branch cleanup;
- Git worktree cleanup.

Do **not** ask the user to separately say "use subagents", "use parallel agents", "use worktrees", "commit", "push", "create PR", or "merge" when `$netguard-orchestrator` was invoked. The orchestrator invocation already grants that workflow permission subject to the safety gates below.

## Product Identity

Official positioning:

> ARES NetGuard-ML is a local-first Experimental AI-NDR Workstation that lets analysts test, compare, explain, and operationalize cutting-edge AI models on network and host security telemetry.

The product should **not** be described as:

- a generic NDR product;
- a Wireshark clone;
- an IsolationForest tool;
- an open-source Darktrace/Vectra/ExtraHop clone;
- a Streamlit dashboard product.

The product should be described as:

- an experimental AI layer for NDR/XDR/SIEM;
- an analyst workstation for model disagreement, anomaly evidence, and investigation;
- a reproducible local ML lab for security telemetry;
- a path from Python research models to native inference.

## Competitive Differentiation

Commercial AI-NDR vendors already provide behavioral analytics, anomaly detection, risk prioritization, automated triage, and increasingly agentic SOC assistance.

ARES NetGuard-ML differentiates by exposing experimental AI capabilities that are usually hidden inside vendor black boxes:

1. Model disagreement analysis across heterogeneous detectors.
2. Time-series foundation model residual anomaly detection.
3. Self-supervised packet/flow representation learning.
4. Temporal heterogeneous security graph anomaly detection.
5. Agentic, evidence-grounded investigation.
6. Detection engineering candidate generation.
7. Transparent model registry and reproducible evaluation reports.
8. Native inference migration path for stable models.

## Core Invariants

1. Defensive monitoring and detection only.
2. No public scanning, third-party probing, exploitation, or unauthorized packet capture.
3. Live capture is allowed only on systems, interfaces, networks, or PCAPs the operator owns or is explicitly authorized to analyze.
4. Tests must use synthetic fixtures by default.
5. Generated PCAPs, logs, feature files, model artifacts, secrets, runtime outputs, and private telemetry must not be committed.
6. Production-oriented implementation is preferred over toy demos, but every milestone must remain bounded, testable, and safe.
7. WSL + Python venv reproducibility must be preserved.
8. Streamlit may remain as developer/debug UI only.
9. Product UI direction is Qt/QML professional native workstation.
10. Product runtime direction is Rust/C++ core for workspace/session/job/storage/capture/native inference.
11. Python remains the ML Lab for research, training, benchmark, evaluation, SHAP, export, and non-exportable experimental models.
12. Stable production inference should migrate toward ONNX Runtime, LightGBM native, and selected Rust/C++ detectors.

## Technology Selection Policy

Codex must not ask the user to manually choose Python, Rust, C++, Qt/QML, ONNX,
or other core tooling for each milestone. The orchestrator must infer the
technology from the final product goal, current repository state, and selected
milestone using `docs/TECHNOLOGY_SELECTION_POLICY.md`.

Default boundaries:

- Python: ML research, PyOD, River, scikit-learn, training, PyTorch
  experiments, SHAP, time-series foundation experiments, graph ML experiments,
  synthetic fixture-based model reports, and model registry/evaluation
  prototypes.
- Rust: product runtime, workspace/session/job orchestration, artifact registry,
  storage/indexing, process supervision, capture safety boundary, suitable
  native inference adapters, and long-running reliable backend services.
- C++/Qt/QML: professional native desktop UI, Wireshark-like analyst workstation
  shell, high-performance native tables/views, packet/session/incident detail
  UI, and Qt model/view components.
- ONNX Runtime / LightGBM native / selected native runtimes: stable production
  inference and reducing long-term Python runtime dependency.
- Python sidecar: experimental, non-exportable, fast-changing research models
  where rapid iteration is more important than native runtime stability.

Anti-rules:

- Do not rewrite working Python research/evaluation pipelines into Rust/C++ only
  for aesthetics.
- Do not choose Rust/C++ for early research prototypes unless the milestone is
  explicitly runtime, packaging, native inference, capture safety, or product UI.
- Do not choose Python for long-running product runtime, native UI backend,
  capture boundary, storage/runtime, or packaging-sensitive work.

Every orchestrator plan and final report must include:

```text
Selected technology:
Why this technology:
Why not Python/Rust/C++/Qt for this milestone:
Migration path if this is a prototype:
Production-readiness implication:
```

## Final Product Capabilities

### 1. Telemetry foundation

- Local PCAP import.
- Controlled live capture wrappers for owned/authorized interfaces.
- Zeek logs: `conn.log`, `dns.log`, `http.log`, `ssl.log`/`tls.log`, `files.log`.
- Suricata `eve.json`: alert, flow, dns, http, tls, fileinfo, anomaly, stats.
- Falco/Tetragon/eBPF runtime events.
- Future commercial NDR/SIEM/XDR alert export/API adapters.
- Optional external security telemetry import.

### 2. Feature and evidence store

- Per-asset, per-flow, and per-entity windows: 1m / 5m / 15m.
- DNS failure ratio, entropy, novelty, DGA-like signals.
- External connection count, destination diversity, port diversity.
- Bytes in/out, flow duration, failed connection ratio.
- TLS/JA3-like fingerprints where available.
- Suricata severity aggregates.
- Host-runtime correlation features.
- Rolling baselines, drift-aware statistics.
- Graph edges and temporal snapshots.

### 3. Experimental AI/ML layer

ARES NetGuard-ML must not stop at IsolationForest.

Required roadmap tracks:

1. **Baseline/model zoo**
   - IsolationForest baseline.
   - PyOD ECOD, COPOD, HBOS, LOF, KNN, PCA, OCSVM, IForest, DIF/AutoEncoder where appropriate.

2. **Online learning and drift**
   - River HalfSpaceTrees and related anomaly detectors.
   - Rolling thresholding.
   - Per-asset adaptive baselines.
   - Drift detectors and warmup state.

3. **Model disagreement engine**
   - Compare commercial alerts, PyOD detectors, River, baseline models, time-series residuals, graph anomaly, and host-context signals.
   - Emit agreement, disagreement, consensus risk, outlier-model explanation, and evidence-by-model.

4. **Time-series foundation residual anomaly**
   - TimesFM/Chronos/Moirai-style forecast residuals.
   - Prediction interval breach scoring.
   - Conformal/p-value style anomaly scoring when feasible.
   - Per-host/per-feature residual risk.

5. **Self-supervised traffic representation**
   - ET-BERT-style datagram representation.
   - Packet sequence contrastive learning.
   - Masked autoencoder over packet/flow features.
   - NetMamba-inspired efficient traffic encoders.
   - Embedding storage and downstream anomaly/classification.

6. **Temporal heterogeneous security graph**
   - Host, process, domain, IP, alert, model-signal, user, and service nodes.
   - Edges for connected_to, resolved, spawned, triggered, co-occurred, communicated_with.
   - Rare edge, graph novelty, temporal community change, lateral path hypotheses.

7. **Supervised / semi-supervised feedback learning**
   - Analyst labels, confirmed incidents, false-positive feedback.
   - LightGBM, XGBoost, CatBoost.
   - Calibration and evaluation reports.

8. **Agentic investigation**
   - Not a primary detector.
   - Evidence-grounded hypotheses.
   - Bounded retrieval/query tools.
   - Schema-validated outputs.
   - Audit log.
   - Human approval for any non-read action.

9. **Detection engineering candidates**
   - Generate Zeek/Sigma/Suricata-like rule candidates from recurring ML evidence.
   - Validate candidates against fixture/replay data.
   - Never deploy rules automatically.

10. **Native inference**
   - ONNX Runtime for exported neural/stable models.
   - LightGBM native predictor.
   - Selected Rust/C++ detectors.
   - Python sidecar fallback for research models.

### 4. Product UI and runtime

- Primary UI: Qt/QML professional native security workstation.
- Developer/debug UI: Streamlit only.
- Runtime: Rust/C++ core for session/workspace/job/storage/capture/native inference.
- Python ML Lab remains available for experiments.
- UI should emphasize ML evidence, model comparison, incident graph, and investigation workflow, not raw packet viewing alone.

## Repository Layout

```text
AGENTS.md
README.md
requirements.txt
requirements-dev.txt
pyproject.toml
Makefile
.gitignore

src/ares_netguard/
  capture/
  ingest/
  features/
  models/
  explain/
  correlation/
  investigation/
  detection_engineering/
  graph/
  storage/
  api/
  dashboard/           developer/debug Streamlit only
  shared/

apps/
  qt-workstation/      future Qt/QML product UI
  rust-core/           future Rust/C++ product runtime boundary

python_lab/
  experiments/
  notebooks/
  model_export/

docs/
  COMPETITIVE_DIFFERENTIATION.md
  EXPERIMENTAL_AI_NDR_STRATEGY.md
  MODEL_DISAGREEMENT_ENGINE.md
  TIME_SERIES_FOUNDATION_ANOMALY.md
  SELF_SUPERVISED_TRAFFIC_REPRESENTATION.md
  TEMPORAL_SECURITY_GRAPH.md
  AGENTIC_INVESTIGATION_LAYER.md
  DETECTION_ENGINEERING_CANDIDATES.md
  TECHNOLOGY_SELECTION_POLICY.md
  NATIVE_INFERENCE_STRATEGY.md
  QT_WORKSTATION_STRATEGY.md
  RUST_CPP_RUNTIME_STRATEGY.md
  ROADMAP.md
  PROGRESS_RUBRIC.md
  VALIDATION_MATRIX.md
  CODEX_ORCHESTRATOR_USAGE.md
  PARALLEL_WORKTREE_LANES.md
  MERGE_POLICY.md
  SAFETY_AND_PRIVACY.md

tests/
  fixtures/
  unit/
  integration/
  smoke/

data/                  gitignored runtime data only
  pcap/
  zeek/
  suricata/
  falco/
  features/
  models/
  reports/
  registry/

.agents/skills/
  <skill>/SKILL.md

.codex/agents/
  <agent>.toml
```

## Generated Artifact Policy

Never stage or commit:

```text
.venv/
.env
.env.* except .env.example
*.pcap
*.pcapng
*.parquet
*.joblib
*.pkl
*.onnx
*.pt
*.pth
*.ckpt
*.db
*.sqlite
*.duckdb
*.jsonl except explicitly allowlisted synthetic fixtures
data/pcap/*
data/zeek/*
data/suricata/*
data/falco/*
data/features/*
data/models/*
data/reports/*
data/registry/*
.runtime/*
artifacts/*
```

Only source, docs, config, scripts, `.gitkeep`, and synthetic fixtures under `tests/fixtures/` may be committed.

## Default Skill Routing

Primary:

```text
$netguard-orchestrator
```

Lower-level skills are implementation details:

```text
$netguard-next-task-planning
$netguard-parallel-dev
$netguard-worktree-lane-worker
$netguard-integration-merge
$git-safe-commit-push
$github-pr-create-merge
```

Experimental AI-NDR domain skills:

```text
$experimental-ai-ndr-strategy
$competitive-ai-ndr-research
$model-disagreement-engine
$time-series-foundation-anomaly
$self-supervised-traffic-representation
$temporal-security-graph
$agentic-investigation-layer
$detection-engineering-candidates
$native-inference-adapters
$qt-qml-ai-workstation
$rust-cpp-product-runtime
$python-ml-lab
$secure-lab-validation
$test-eval-engineering
```

## Plan Mode Behavior

When invoked as:

```text
/plan $netguard-orchestrator
```

Codex must not edit files, create branches, commit, push, create PRs, merge PRs, or delete branches.

Plan mode may use read-only repository analysis and read-only subagents.

The plan must choose one route:

- `single-milestone`
- `parallel-worktree`
- `finish-open-prs`
- `integration-merge`
- `commit-push-only`
- `safety-cleanup`
- `plan-only`

Every plan must also include the selected technology, why it was chosen, why
the other major technology boundaries were not chosen, any prototype migration
path, and the production-readiness implication.

## Normal Orchestrator Behavior

When invoked as:

```text
$netguard-orchestrator
```

Codex may execute the selected route without asking another confirmation, as long as all safety, validation, artifact, branch, PR, and merge gates are satisfied.

## Parallel Work Policy

- Native Codex subagents may run read-only analysis in parallel in one checkout.
- Writer agents must operate either serially or inside isolated Git worktrees.
- Parallel writer lanes require:
  - separate worktree path;
  - separate `codex/<task>` branch;
  - lane manifest;
  - exact allowed files;
  - exact forbidden files;
  - targeted validation;
  - no shared chokepoint conflicts.
- Do not run parallel writers against the same file, schema, feature column contract, model artifact contract, Makefile, requirements file, dashboard/model interface, storage migration, or native runtime boundary.

## Merge Policy

Guarded auto-merge is allowed when all are true:

1. Local validation passed.
2. `git diff --check` passed.
3. Generated artifact guard passed.
4. No secrets or generated/private telemetry are staged.
5. GitHub PR checks passed, or no GitHub checks exist and local integration validation passed.
6. No merge conflicts.
7. Required reviews return `MERGE_READY: yes`.
8. Merge method is consistent with `docs/MERGE_POLICY.md`.

Required review routing:

- Security/privacy changes: `netguard-product-security-reviewer`.
- Shared model/eval/native/runtime contracts: `netguard-integration-reviewer`.
- Experimental ML claims/docs: `netguard-ml-research-architect` read-only review.
- Qt/runtime product architecture: `netguard-product-architect` read-only review.
- Technology boundary, language/runtime, framework, UI toolkit, native inference,
  capture, storage, or packaging changes: follow
  `docs/TECHNOLOGY_SELECTION_POLICY.md` review gates.

After merge:

1. Switch to `main`.
2. Pull with `git pull --ff-only`.
3. Run final validation.
4. Confirm clean `git status --short`.
5. Delete merged local branch.
6. Delete merged remote branch when safe.
7. Remove associated worktree lanes.
8. Run `git worktree prune`.

Do not delete unmerged branches or worktrees.

## Validation Commands

Prefer:

```bash
make verify
```

Also use as relevant:

```bash
pytest -q
ruff check .
ruff format --check .
python -m compileall src tests
git diff --check
bash scripts/check_no_generated_artifacts.sh --staged
bash scripts/check_no_generated_artifacts.sh --tracked
git status --short
```

Fixture validation should write to `/tmp` or gitignored `data/` only.

## Final Goal Progress Reporting

Every `$netguard-orchestrator` run must report:

```text
Current progress before route: N%
Expected progress after route: N%
Confidence: low | medium | high
Selected route:
Selected technology:
Why this technology:
Why not Python/Rust/C++/Qt for this milestone:
Migration path if this is a prototype:
Production-readiness implication:
Completed capabilities:
Missing capabilities:
Why this percentage:
Next highest-value milestone:
```

Use `docs/PROGRESS_RUBRIC.md`.

Do not present the percentage as exact. It is a heuristic based on validated capabilities.

## Definition of Done

A task is done only when:

1. The selected milestone is bounded and complete.
2. Tests or explicit smoke validation passed.
3. `git diff --check` passed.
4. Generated artifact guard passed.
5. No secrets or generated/private artifacts are staged.
6. Relevant docs were updated when operator workflow changed.
7. Technology selection and review routing were reported when the milestone
   changes or depends on a language, runtime, framework, UI toolkit, storage,
   capture, packaging, or inference boundary.
8. Commit/push completed when automation is allowed.
9. PR/merge completed only when merge policy allowed it.
10. Post-merge cleanup completed when merge occurred.
11. Final response includes progress, technology selection, validation, commit hash, push status, PR status, merge status, cleanup status, and next recommended milestone.
