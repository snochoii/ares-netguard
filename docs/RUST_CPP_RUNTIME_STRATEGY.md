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
local endpoint, IPC, frame, message, and handoff schemas,
`runtime_control_plane_endpoint.v0`, `runtime_control_plane_ipc.v0`,
`runtime_control_plane_frame.v0`, `runtime_control_plane_message.v0`,
`runtime_handoff_snapshot.v0`, `runtime_summary.v0`, and
`model_registry_metadata.v0`, and exposes `RuntimeControlPlaneAdapterKind`,
`RuntimeControlPlaneInputMode`, `RuntimeControlPlaneAdapterState`, and
`RuntimeControlPlaneOutputSnapshotSchema`. The top local adapter fixture now
identifies the bounded endpoint policy over the connected-stream IPC adapter as
available while listener, daemon, filesystem socket path policy, Qt binding,
external service, and deployment behavior remain disabled. JSON-string parsing
is now enabled through `serde` and `serde_json` using
`RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json`. The parser
accepts only a caller-provided local JSON string, denies unknown fields, rejects
unsupported schema versions and enum values, validates coarse runtime IDs,
enforces local-only and non-deployment safety flags, validates sorted
Python-derived synthetic registry entries and derived aggregate metadata, and
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

The scaffold now also adds a typed `model_registry_metadata_adapter.v0`
contract through `ModelRegistryMetadataAdapterContract` and
`ModelRegistryMetadataAdapterPolicy`. The adapter accepts a caller-provided
`model_registry_metadata.v0` JSON string through
`parse_model_registry_metadata_json` or a bounded local `.json` file through
`parse_model_registry_metadata_file` under the same explicit
`RuntimeControlPlaneFilePolicy` root checks used by the handoff file adapter.
Both paths deserialize into `ModelRegistryMetadata` with
`serde`/`serde_json`, deny unknown fields, validate the exact metadata schema,
scope, source bundle schema, sorted synthetic model entries, aggregate summary,
safe model/source labels, non-claims, and local-only safety flags, and reject
metadata that claims deployment, capture, or external services. The adapter
contract and policy keep storage, generated report loading, Qt binding,
external services, deployment, capture, and native inference execution disabled.
This is a typed metadata handoff adapter only; it is not a persistent model
registry, storage provider, promotion workflow, generated report loader, Qt
binding, external service, deployment approval path, capture boundary, or
native inference executor.

The scaffold now adds a typed local command dispatcher over those existing
parsers through `RuntimeControlPlaneCommand` and
`RuntimeControlPlaneAdapterContract::execute_local_command`. The typed local
command dispatcher accepts only `ParseHandoffSnapshotJson` for a
caller-provided `runtime_handoff_snapshot.v0` JSON string and
`ParseHandoffSnapshotFile` for a bounded local
`runtime_handoff_snapshot.v0` JSON file with an explicit
`RuntimeControlPlaneFilePolicy`. Dispatch returns the same typed
`RuntimeHandoffSnapshot` as the lower-level parser APIs and preserves the same
fail-closed schema, registry, safety flag, UTF-8, file size, symlink,
directory, regular-file, extension, and allowed-root checks. This command layer
is still local and pre-IPC; it does not add sockets, daemon lifecycle,
watching, storage, generated report loading, Qt binding, capture, deployment,
external services, or native inference execution.

The scaffold now adds a strict local `runtime_control_plane_message.v0`
request/response message envelope over the typed local command dispatcher.
`RuntimeControlPlaneMessageRequest` carries the schema version, a
caller-supplied `RuntimeControlPlaneRequestId`, and exactly one local command.
The request parser rejects malformed JSON, unknown fields, unsupported message
schema versions, unsafe request identifiers, unsupported command variants,
mixed command fields, and then delegates nested handoff snapshots to the same
strict JSON-string or bounded local file command paths. `RuntimeControlPlaneMessageResponse`
returns the same schema version and request identifier with
`RuntimeControlPlaneMessageOutcome::Success` plus a typed
`RuntimeHandoffSnapshot`, or `RuntimeControlPlaneMessageOutcome::Failure` plus a
`RuntimeControlPlaneMessageErrorCode` mapped from the existing
`RuntimeControlPlaneAdapterError` categories. Response serialization is
available through
`RuntimeControlPlaneAdapterContract::serialize_control_plane_message_response_json`.
This remains pre-transport contract code only: it adds no daemon, listener,
socket, file watcher, process spawning, storage provider, Qt binding, generated
report loader, capture behavior, deployment behavior, external service, or
native inference execution.

The scaffold now adds a bounded local byte-frame adapter over that strict
`runtime_control_plane_message.v0` envelope through
`runtime_control_plane_frame.v0`, `RuntimeControlPlaneFramePolicy`, and
`RuntimeControlPlaneFrameAdapterContract`. The adapter accepts only
caller-provided `&[u8]` frames, caps frames at 256 KiB by default, requires
UTF-8 JSON payloads, uses the existing `serde`/`serde_json` dependency surface
without adding a new dependency, delegates parsed requests to the existing
message envelope, and returns serialized UTF-8 JSON response bytes through
`parse_control_plane_message_frame_bytes`,
`execute_control_plane_message_frame_bytes`, and
`serialize_control_plane_message_response_frame_bytes`. Empty, oversized,
invalid UTF-8, malformed JSON, non-object roots, unknown fields, unsupported
message schema versions, unsafe request identifiers, unsupported command
variants, mixed command fields, unsafe nested handoff flags, and malformed
nested handoff snapshots fail closed. Frame parsing failures without a valid
request identifier return adapter errors; command execution failures after a
valid request identifier return typed failure responses with
`RuntimeControlPlaneMessageErrorCode`. This is a local byte-frame adapter only:
it adds no OS IPC adapter, socket listener, daemon lifecycle, file watcher,
process spawning, storage provider, Qt binding, generated report loader,
capture behavior, deployment behavior, external service, or native inference
execution.

The scaffold now adds a bounded `runtime_control_plane_ipc.v0`
connected-stream adapter over the validated `runtime_control_plane_frame.v0`
boundary through `RuntimeControlPlaneIpcPolicy`,
`RuntimeControlPlaneIpcAdapterContract`,
`read_control_plane_message_ipc_frame`,
`write_control_plane_message_ipc_frame`, and
`execute_control_plane_message_ipc_stream`. The adapter reads exactly one
4-byte big-endian length prefix, where
`RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES` is 4, followed by one UTF-8 JSON
message frame. It rejects zero-length frames as invalid JSON, rejects declared
lengths above `RuntimeControlPlaneFramePolicy`, rejects incomplete length
prefixes and incomplete payloads as `IncompleteIpcFrame`, maps stream read and
write failures to typed IPC adapter errors, delegates payload validation and
request execution to the existing frame adapter, and writes the response with
the same length-prefix format. Frame parsing failures without a valid request
identifier still return adapter errors with no fabricated response; command
execution failures after a valid request identifier still return typed failure
responses with `RuntimeControlPlaneMessageErrorCode`. This adapter is local,
caller-provided stream I/O only: it adds no public network transport, no socket
listener, no filesystem socket path policy, no daemon lifecycle, no process
spawning, no file watcher, no storage provider, no Qt binding, no capture
behavior, no generated report loading, no deployment behavior, no external
service, and no native inference execution.

The scaffold now adds a bounded `runtime_control_plane_endpoint.v0` endpoint
policy over the connected-stream IPC adapter through
`RuntimeControlPlaneEndpointPolicy`,
`RuntimeControlPlaneEndpointAdapterContract`, `RuntimeControlPlaneEndpointKind`,
and `execute_control_plane_endpoint_stream`. The only accepted endpoint kind is
`CallerProvidedConnectedStream`, which means the caller must supply already
connected `Read` and `Write` streams. `validate_control_plane_endpoint_policy`
rejects unsupported endpoint schema versions, invalid nested frame caps, and
unsafe policy flags including public network transport, socket listener,
filesystem socket path policy, daemon lifecycle, process spawning, file
watching, Qt binding, storage provider, capture, external services, deployment,
and native inference execution. After policy validation, execution delegates to
`execute_control_plane_message_ipc_stream` and preserves the same strict
request/response behavior as the IPC stream adapter. This policy is an endpoint
gate only: it adds no socket binding, no listener loop, no filesystem socket
path selection, no OS endpoint registration, no daemon lifecycle, no process
spawning, no file watcher, no storage provider, no Qt binding, no capture
behavior, no generated report loading, no deployment behavior, no external
service, and no native inference execution.

The v0 scaffold is intentionally a source-only contract with bounded parser
behavior. It does not implement a daemon, storage engine, process supervisor,
capture wrapper, native inference executor, model artifact loader, external
service client, packaging flow, or UI data adapter. It does not read PCAPs,
private telemetry, logs, model binaries, databases, runtime artifacts, or
generated report files. The registry metadata preview is not a persistent model
registry, promotion gate, deployment approval workflow, database-backed
registry provider, generated JSON file loader, Qt binding, external service,
capture boundary, or native inference execution path. The typed registry
metadata adapter validates only explicitly supplied synthetic metadata JSON; it
does not add storage, indexing, generated report loading, model promotion,
deployment approval, Qt data-flow integration, external services, capture, or
native inference execution. The handoff snapshot is not a live
control-plane transport, runtime service, Qt data binding, storage provider,
generated report loader, or live state feed. The control-plane adapter is
strict local JSON-string parsing plus bounded local file reading behind a typed
local command dispatcher, strict local request/response message envelope, and
bounded local byte-frame adapter plus a bounded connected-stream IPC adapter and
bounded endpoint policy only; it is not arbitrary file loading, not file
watching, not a public network transport, not a socket listener, not a
filesystem socket path policy, not Qt binding, not external-service integration,
not storage, not deployment behavior, not capture behavior, and not native
inference execution.

Expected integration path:

```text
Rust source contract
  -> buildable Cargo project in an environment with Rust tooling
  -> static runtime_summary.v0 handoff displayed by the Qt shell
  -> static model_registry_metadata.v0 handoff aligned with Python and Qt
  -> typed model_registry_metadata_adapter.v0 over supplied metadata JSON/files
  -> static runtime_handoff_snapshot.v0 handoff envelope over both fixtures
  -> static runtime_control_plane_adapter.v0 contract over accepted schemas
  -> typed JSON-string parser and bounded local file adapter for handoff snapshots
  -> typed local control-plane command dispatcher over JSON/file parsers
  -> strict runtime_control_plane_message.v0 local request/response envelope
  -> bounded runtime_control_plane_frame.v0 local byte-frame adapter
  -> bounded runtime_control_plane_ipc.v0 connected-stream adapter
  -> bounded runtime_control_plane_endpoint.v0 endpoint policy
  -> future OS-local listener/path binding implementation
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
