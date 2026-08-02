# Technology Selection Policy

ARES NetGuard-ML uses technology based on product boundary and milestone intent.
The orchestrator must infer the right technology from the final product goal,
current repository state, and selected milestone.

## Required orchestrator report fields

Every `$netguard-orchestrator` plan and final report must include:

```text
Selected technology:
Why this technology:
Why not Python/Rust/C++/Qt for this milestone:
Migration path if this is a prototype:
Production-readiness implication:
```

For docs-only governance work, the selected technology is `Markdown/docs`.

## Default boundaries

Use Python for:

- ML research.
- PyOD, River, scikit-learn.
- LightGBM, XGBoost, and CatBoost training.
- PyTorch experiments.
- SHAP and explainability research.
- Time-series foundation model experiments.
- Graph ML experiments.
- Synthetic fixture-based model reports.
- Model registry and evaluation prototypes.

Use Rust for:

- Product runtime.
- Session and workspace management.
- Job orchestration.
- Artifact registry.
- Storage and indexing layer.
- Process supervision.
- Capture safety boundary.
- Native inference adapters where Rust is suitable.
- Long-running reliable backend services.

Use C++/Qt/QML for:

- Professional native desktop product UI.
- Wireshark-like analyst workstation shell.
- High-performance native tables and views.
- Packet, session, and incident detail UI.
- Qt model/view based UI components.

Use ONNX Runtime, LightGBM native prediction, or selected native model runtimes for:

- Stable production inference.
- Exported deep and time-series models.
- LightGBM native prediction.
- Reducing long-term Python runtime dependency.

Use a Python sidecar only when:

- The model is experimental.
- The model is not exportable yet.
- PyOD, River, or research code is still changing.
- Rapid model iteration is more important than native runtime stability.

## Anti-rules

- Do not rewrite a working Python research or evaluation pipeline into Rust,
  C++, or Qt only for aesthetics.
- Do not choose Rust or C++ for early research prototypes unless the milestone
  is explicitly runtime, packaging, native inference, capture safety, or product
  UI related.
- Do not choose Python for product runtime work when the milestone is a
  long-running core service, native UI backend, capture boundary,
  storage/runtime, or packaging-sensitive.
- Do not treat Streamlit as the final product UI. Streamlit remains
  developer/debug UI only.

## Decision triggers

Record a technology decision when a milestone introduces or changes:

- Programming language or runtime boundary.
- UI toolkit.
- Model framework or inference runtime.
- Storage engine, index, or artifact registry.
- Capture library or safety boundary.
- External service, API client, or network dependency.
- Native build system, packaging mechanism, or long-running service.
- Dependency whose license, supply chain, or platform support changes the
  product risk profile.

## Review gates

- ML methodology and evaluation design require
  `netguard-correctness-reviewer`. ML framework or runtime selection rationale
  requires a root-owned `technology_choice` packet, and current external
  availability, API, or version claims require
  `netguard-docs-api-researcher` evidence.
- Shared model, evaluation, native inference, runtime, or storage contracts
  require `netguard-integration-reviewer`.
- Qt, Rust, C++, native runtime, UI, or product architecture choices require a
  root-owned `technology_choice` packet for technology selection, an
  `architectural_tradeoff` packet for component topology, or a
  `product_direction` packet for product direction or positioning. Add
  `netguard-integration-reviewer` when the choice affects shared compatibility.
- Capture, telemetry, privacy, external service, or artifact-policy-impacting
  choices require `netguard-product-security-reviewer`.

## Prototype migration

Experimental prototypes may start in Python when that accelerates model
iteration. The plan must state the migration path:

```text
research prototype
  -> reproducible evaluation
  -> stable schema and feature contract
  -> exportable or native-inference candidate
  -> product runtime integration
```

If no migration is planned, the plan must say why the component is expected to
remain research-only.

## Decision record template

```text
Selected technology:
Milestone boundary:
Alternatives considered:
Why selected:
Why rejected:
Migration path:
Production-readiness implication:
Required reviews:
Validation:
Rollback:
```
