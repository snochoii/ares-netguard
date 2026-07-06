# Rust/C++ Product Runtime Strategy

## Role

Rust/C++ is the product runtime direction for workspace/session/job ownership,
storage and indexing boundaries, process supervision, capture safety boundaries,
and stable native inference integration. Python remains the ML Lab for research
models and reproducible evaluation. Qt/QML remains the native analyst
workstation UI.

## v0 source scaffold

The first source-controlled Rust runtime shell lives under `apps/rust-core/`.
It establishes a Rust package boundary with coarse runtime contract types for:

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

The scaffold now also owns a static `model_registry_metadata.v0` handoff
contract through `ModelRegistryMetadata`, `ModelRegistryEntry`,
`ModelRegistryAggregateSummary`, and `ModelRegistrySafetyFlags`. This mirrors
the validated Python `model_registry_metadata.v0` output shape and the Qt
workstation snapshot using source-only Rust constants. The static fixture lists
the local synthetic model registry scope, its `model_evaluation_bundle.v0`
source schema, ten sanitized model entries, derived aggregate metadata, safety
flags, and non-claim strings. Every entry remains `observed_synthetic_only`,
`not_promoted`, `human_review_required`, and `deployment_allowed: false`.

The scaffold also exposes a static `runtime_handoff_snapshot.v0` handoff
envelope through `RuntimeHandoffSnapshot`. The envelope composes the existing
`RuntimeSummary` and `ModelRegistryMetadata` fixtures and records that the
source is a static synthetic fixture, transport is unavailable, the
control-plane adapter is unavailable, no generated JSON has been loaded, no
live runtime connection exists, no external services are used, and deployment is
not allowed. This gives the future adapter a runtime-owned shape without
claiming adapter behavior.

The scaffold now also owns a `runtime_control_plane_adapter.v0` contract through
`RuntimeControlPlaneAdapterContract`. The adapter contract declares the accepted
local handoff schemas, `runtime_handoff_snapshot.v0`, `runtime_summary.v0`, and
`model_registry_metadata.v0`, and exposes `RuntimeControlPlaneAdapterKind`,
`RuntimeControlPlaneInputMode`, `RuntimeControlPlaneAdapterState`, and
`RuntimeControlPlaneOutputSnapshotSchema`. JSON-string parsing is now enabled
through `serde` and `serde_json` using
`RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json`. The parser
accepts only a caller-provided local JSON string, denies unknown fields, rejects
unsupported schema versions and enum values, validates coarse runtime IDs,
enforces local-only and non-deployment safety flags, preserves the exact
Python-derived synthetic registry entry order and aggregate metadata, and
returns a typed `RuntimeHandoffSnapshot`.

The bounded local file adapter is now enabled through
`RuntimeControlPlaneFilePolicy` and
`RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file`. The file
adapter accepts only an absolute `.json` path, canonicalizes it against one
canonical allowed root, rejects symlinks, directories, non-regular files,
missing files, non-JSON paths, oversized files, and invalid UTF-8, then
delegates the file contents to the same strict JSON-string parser. The fixed
file cap is 256 KiB. This is only a local `runtime_handoff_snapshot.v0`
handoff path; it is not an arbitrary file loader and does not load generated
reports, model artifacts, telemetry, databases, or runtime output trees. Live
transport, Qt binding, external services, and deployment remain disabled.

The v0 scaffold is intentionally a source-only contract with bounded parser
behavior. It does not implement a daemon, storage engine, process supervisor,
capture wrapper, native inference executor, model artifact loader, external
service client, packaging flow, or UI data adapter. It does not read PCAPs,
private telemetry, logs, model binaries, databases, runtime artifacts, or
generated report files. The registry metadata preview is not a persistent model
registry, promotion gate, deployment approval workflow, database-backed
registry provider, generated JSON file loader, or native inference execution
path. The handoff snapshot is not a live
control-plane transport, runtime service, Qt data binding, storage provider,
generated report loader, or live state feed. The control-plane adapter is
strict local JSON-string parsing plus bounded local file reading only; it is
not arbitrary file loading, not file watching, not socket/IPC transport, not Qt
binding, not external-service integration, not storage, not deployment
behavior, not capture behavior, and not native inference execution.

Expected integration path:

```text
Rust source contract
  -> buildable Cargo project in an environment with Rust tooling
  -> static runtime_summary.v0 handoff displayed by the Qt shell
  -> static model_registry_metadata.v0 handoff aligned with Python and Qt
  -> static runtime_handoff_snapshot.v0 handoff envelope over both fixtures
  -> static runtime_control_plane_adapter.v0 contract over accepted schemas
  -> typed JSON-string parser and bounded local file adapter for handoff snapshots
  -> local IPC/control-plane adapter
  -> local control-plane adapter
  -> typed registry metadata adapter
  -> real Rust runtime summary provider
  -> runtime registry/storage provider
  -> Qt workstation data-flow integration
  -> Python ML Lab report handoff for experimental models
  -> runtime storage, job supervision, and stable native inference adapters
```

The repository does not require Rust tooling for `make verify`; that target
remains Python/static-compatible for every validation run. Rust-specific
validation is available through `make verify-rust-core`, which runs
`cargo fmt --check`, `cargo test`, and `cargo clippy -- -D warnings` when Cargo
is available.

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
