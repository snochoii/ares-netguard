# Roadmap

## Current strategic direction

ARES NetGuard-ML is an Experimental AI-NDR Workstation.

The near-term roadmap prioritizes experimental ML differentiation before a full product UI rewrite.

## Milestone sequence

1. **Contract update: experimental AI-NDR strategy**
   - AGENTS, docs, skills, subagents updated.
   - No source behavior change.

2. **Contract update: technology selection policy**
   - Orchestrator chooses Python, Rust, C++, Qt/QML, ONNX/native runtimes, or
     Python sidecar from milestone intent.
   - Plans and final reports include technology selection, rejected alternatives,
     prototype migration path, and production-readiness implication.
   - No feature code or dependency change.

3. **Synthetic Telemetry Foundation**
   - Normalize safe Zeek/Suricata/Falco-like synthetic fixture rows.
   - Emit 1m/5m `feature_vector_row.v0` feature windows.
   - No live capture, PCAP parsing, private telemetry, or external services.
   - See `docs/TELEMETRY_FOUNDATION.md`.

4. **Model Disagreement Engine**
   - Compare model families and signal sources.
   - Emit disagreement report.

5. **Time-Series Foundation Residual Prototype**
   - Forecast-residual anomaly using mocked or optional foundation model backend.
   - Conformal/residual risk contract.

6. **Self-Supervised Traffic Representation Prototype**
   - Tokenization/embedding experiment over sanitized synthetic telemetry.
   - No raw payload storage.

7. **Temporal Security Graph Baseline**
   - NetworkX graph features and rare-edge anomaly.

8. **Agentic Investigation Layer**
   - Evidence-grounded hypotheses over local artifacts.

9. **Detection Engineering Candidates**
   - Draft Zeek/Sigma/Suricata candidates from recurring ML evidence.

10. **Native Inference Adapters**
   - ONNX/LightGBM native path.

11. **Model Evaluation Bundle**
    - Local `model_evaluation_bundle.v0` summary over synthetic reports and
      score-row lists.
    - Reproducible evaluation counts and privacy guardrails.
    - See `docs/MODEL_EVALUATION_BUNDLE.md`.

12. **Model Registry Metadata**
    - Local `model_registry_metadata.v0` derived from the evaluation bundle.
    - Synthetic-only observed model entries, source schema/name references, and
      promotion/deployment false-claim guardrails.
    - Not a persistent registry, promotion workflow, deployment gate, runtime
      service, or UI integration.
    - See `docs/MODEL_REGISTRY_METADATA.md`.

13. **Evidence Index**
    - Local `evidence_index.v0` pointer-only index over synthetic reports and
      score-row lists.
    - Entity/window source row references, feature names, model IDs, evidence
      indexes, counts, and local-only safety flags.
    - Not durable storage, a database, runtime job execution, live capture,
      deployment approval, or UI integration.
    - See `docs/EVIDENCE_INDEX.md`.

14. **Qt/QML AI-NDR Workstation Shell**
   - Product UI begins once ML evidence artifacts are meaningful.

15. **Rust/C++ Product Runtime**
   - Workspace/session/job/storage/native inference core.
