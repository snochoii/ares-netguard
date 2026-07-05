# Rust/C++ Product Runtime Strategy

## Role

Rust/C++ is the product runtime direction for workspace/session/job ownership,
storage and indexing boundaries, process supervision, capture safety boundaries,
and stable native inference integration. Python remains the ML Lab for research
models and reproducible evaluation. Qt/QML remains the native analyst
workstation UI.

## v0 source scaffold

The first source-controlled Rust runtime shell lives under `apps/rust-core/`.
It establishes a no-dependency Rust package boundary with coarse runtime
contract types for:

- workspace identifiers;
- session identifiers;
- job identifiers;
- job kinds;
- job states;
- runtime events.

The v0 runtime also owns a static `runtime_summary.v0` handoff contract through
`RuntimeSummary` and `NativeInferenceRuntimeState`. The summary is deliberately
coarse: workspace/session identifiers, total/queued/running/failed job counts, a
last event label, and native inference availability state. The checked-in
fixture values are local and synthetic so the Qt shell can display the contract
shape without owning runtime lifecycle or job state.

The v0 scaffold is intentionally a source-only contract. It does not implement a
daemon, storage engine, process supervisor, capture wrapper, native inference
executor, model artifact loader, external service client, packaging flow, or UI
data adapter. It does not read PCAPs, private telemetry, logs, model binaries,
databases, or runtime artifacts.

Expected integration path:

```text
Rust source contract
  -> buildable Cargo project in an environment with Rust tooling
  -> static runtime_summary.v0 handoff displayed by the Qt shell
  -> typed JSON/control-plane adapters for local evidence summaries
  -> real Rust runtime summary provider
  -> Qt workstation data-flow integration
  -> Python ML Lab report handoff for experimental models
  -> runtime storage, job supervision, and stable native inference adapters
```

Local validation is currently static because the implementation environment does
not provide `cargo` or `rustc`. The v0 gate therefore checks source layout,
dependency absence, public runtime anchors, and local-only safety constraints
through pytest and repository validation. Once Rust tooling is available, add
`cargo fmt --check`, `cargo test`, and `cargo clippy` to the runtime validation
gate.

## Runtime priorities

- workspace and session lifecycle ownership;
- durable job state and audit event contracts;
- safe boundaries for future owned/authorized capture wrappers;
- runtime storage and artifact registry contracts;
- native inference promotion for stable models;
- typed handoff to the Qt/QML workstation;
- controlled Python sidecar interaction for research-only models.

## Non-goals

- Do not rewrite working Python ML research pipelines into Rust/C++ for
  aesthetics.
- Do not claim live capture, native inference execution, persistent storage, or
  packaging before the corresponding safety and validation gates exist.
- Do not let the Qt/QML UI own runtime lifecycle or job supervision state.
- Do not commit generated runtime outputs, databases, model artifacts, PCAPs, or
  private telemetry.
