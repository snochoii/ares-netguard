# Qt/QML AI-NDR Workstation Strategy

## Role

Qt/QML is the primary product UI direction because ARES NetGuard-ML should feel like a professional native security analysis workstation, not a web dashboard.

## v0 source scaffold

The first source-controlled Qt/QML product UI shell lives under
`apps/qt-workstation/`. It establishes a Qt 6 Quick application boundary with a
minimal C++ bootstrap and a QML analyst workspace screen.

The v0 screen is intentionally operational rather than promotional:

- left navigation for workspace, model disagreement, evidence graph,
  investigation, detection candidates, and model registry;
- central model evidence matrix using static synthetic rows;
- compact Runtime Boundary panel that mirrors the Rust-owned
  `runtime_summary.v0` fields from a static synthetic QML object;
- compact Evidence Index Snapshot panel that mirrors static `evidence_index.v0`
  fields from a static synthetic QML object;
- compact Model Registry Snapshot panel that mirrors
  `model_registry_metadata.v0` fields from a static synthetic QML object;
- right detail panel for selected entity context, evidence, analyst actions,
  and registry status.

This milestone makes no packaged application, runtime integration, model
execution, live capture, external service, or private telemetry claim. All v0 UI
content is static and synthetic. The Runtime Boundary panel is a source-level
handoff preview only; it is not a live runtime connection and does not make the
Qt shell the owner of workspace, session, job, or native inference state.
The Rust runtime now also defines a typed `runtime_summary_provider.v0` over
caller-provided local `RuntimeEvent` slices and exposes
`build_runtime_summary_from_events` for deriving `runtime_summary.v0` in Rust.
Qt still displays its own static Runtime Boundary preview object; it does not
call the provider, own the event stream, bind to live runtime state, read a
storage provider, open a transport, capture traffic, allow deployment, use
external services, or execute native inference.
The Model Registry Snapshot panel is also a source-level handoff preview only;
it is not a persistent registry, does not read generated reports, and does not
own model promotion or deployment state.
The Evidence Index Snapshot panel is also a source-level handoff preview only.
Qt displays its own static Evidence Index preview object with generated source
names, row pointers, aggregate counts, and pointer-only safety flags; it does
not call Rust parsers, read generated evidence index files, copy raw evidence
payloads, bind to live runtime state, use a database or indexing engine, allow
deployment, capture traffic, use external services, or execute native
inference.
The Rust runtime now also defines a typed `model_registry_metadata_adapter.v0`
for caller-provided `model_registry_metadata.v0` JSON strings and bounded local
metadata `.json` files. Qt still displays its own static Model Registry
Snapshot preview object; it does not call `parse_model_registry_metadata_json`,
call `parse_model_registry_metadata_file`, perform registry metadata file I/O,
load generated reports, bind to a storage provider, own promotion/deployment
state, use external services, capture traffic, or execute native inference.
The Rust runtime now also defines a static `runtime_handoff_snapshot.v0`
envelope that composes the runtime summary and model registry metadata fixtures.
Qt still displays its own static preview objects; it does not read the Rust
snapshot, open a control-plane transport, or bind to live runtime state yet.
The Rust runtime also defines a static `runtime_control_plane_adapter.v0`
contract for the future local handoff path. Qt does not read that adapter
contract, parse generated JSON, open a live transport, or bind to live runtime
state yet. Rust can now parse a caller-provided local
`runtime_handoff_snapshot.v0` JSON string and can read a bounded local
`runtime_handoff_snapshot.v0` JSON file under a caller-supplied allowed root
into typed contract structs. Rust also has a typed local control-plane command
dispatcher over those JSON/file parser paths, but Qt does not call it or bind
to it. Rust now also defines a strict local
`runtime_control_plane_message.v0` request/response message envelope over that
typed dispatcher, with a safe request identifier, a single local command, and
typed success/failure responses. Qt does not parse the message envelope, call
the dispatcher, open a transport, or bind to live runtime state. Rust now also
defines a bounded local `runtime_control_plane_frame.v0` byte-frame adapter
over that message envelope, with caller-provided UTF-8 JSON bytes, a 256 KiB
default frame cap, and serialized UTF-8 JSON response bytes. Qt does not parse
frame bytes, call the frame adapter, perform file I/O, bind to live runtime
state, load generated reports, use external services, allow deployment, capture
traffic, or execute native inference. Rust now also defines a bounded
`runtime_control_plane_ipc.v0` connected-stream adapter over that frame layer,
with caller-provided streams, a 4-byte big-endian length prefix, the same
256 KiB frame cap, and a one-shot request/response execution path. Qt does not
open IPC, call `read_control_plane_message_ipc_frame`, call
`write_control_plane_message_ipc_frame`, call
`execute_control_plane_message_ipc_stream`, bind to live runtime state, start a
listener, manage a filesystem socket path, spawn a process, load generated
reports, use external services, allow deployment, capture traffic, or execute
native inference.
Rust now also defines a bounded `runtime_control_plane_endpoint.v0` endpoint
policy over that connected-stream IPC layer, with a caller-provided connected
stream endpoint kind, strict endpoint policy validation, and
`execute_control_plane_endpoint_stream` delegation into the IPC adapter. Qt does
not call `execute_control_plane_endpoint_stream`, choose endpoint paths, open
IPC, start a listener, manage a filesystem socket path, spawn a process, bind to
live runtime state, load generated reports, use external services, allow
deployment, capture traffic, or execute native inference.
Rust now also defines a bounded `runtime_control_plane_endpoint_path.v0` path
policy for caller-authorized OS-local `.sock` endpoint path selection through
`validate_control_plane_endpoint_path`. Qt still does not call or read the
endpoint path policy, choose endpoint paths, bind a listener, manage daemon
lifecycle, supervise a process, bind to live runtime state, load generated
reports, use external services, allow deployment, capture traffic, or execute
native inference.
Rust now also defines a bounded
`runtime_control_plane_endpoint_listener.v0` one-shot OS-local listener over
validated endpoint paths through `execute_control_plane_endpoint_listener_once`.
Qt still does not call or bind the listener, open IPC, choose endpoint paths,
manage a filesystem socket path, run a listener loop, manage daemon lifecycle,
supervise a process, bind to live runtime state, load generated reports, use
external services, allow deployment, capture traffic, or execute native
inference.
Rust now also defines a bounded
`runtime_control_plane_endpoint_lifecycle.v0` one-shot endpoint lifecycle
wrapper over that listener through `execute_control_plane_endpoint_lifecycle_once`.
Qt still does not call or bind the lifecycle wrapper, open IPC, choose endpoint
paths, manage a filesystem socket path, start or stop endpoint lifecycle,
run a listener loop, manage daemon lifecycle, supervise a process, bind to
live runtime state, load generated reports, use external services, allow
deployment, capture traffic, or execute native inference.
Rust now also defines a bounded
`runtime_control_plane_service_lifecycle.v0` service lifecycle state wrapper
over one endpoint lifecycle execution through
`execute_control_plane_service_lifecycle_once`, with capped in-memory audit
events and explicit `Stopped`, `Starting`, `RunningEndpointOnce`, `Stopping`,
and `Failed` states. Qt still does not call or bind the service lifecycle
wrapper, own service lifecycle state, open IPC, choose endpoint paths, manage a
filesystem socket path, start or stop service lifecycle, run a listener loop,
manage daemon lifecycle, supervise a process, bind to live runtime state, load
generated reports, use external services, allow deployment, capture traffic, or
execute native inference.
Rust now also defines a bounded in-memory `runtime_registry_provider.v0` that
stores already validated `runtime_handoff_snapshot.v0` values and emits a typed
`RuntimeRegistrySnapshot` sorted by workspace/session key. Qt still displays
its own static preview objects; it does not call `RuntimeRegistryProvider`,
read `RuntimeRegistrySnapshot`, bind to live runtime state, read persistent
storage, use a database or indexing engine, load generated reports, load
generated JSON, open transport, start a listener, manage a filesystem socket
path, spawn a process, use external services, allow deployment, capture
traffic, or execute native inference.
Rust now also defines a bounded
`runtime_registry_storage_provider.v0` local JSON storage provider for typed
`RuntimeRegistrySnapshot` documents under a caller-authorized absolute allowed
root. Qt still does not call `RuntimeRegistryStorageProvider`, read
`RuntimeRegistryStorageDocument`, read registry storage files, bind to live
runtime state, use a database or indexing engine, load generated reports, load
generated JSON, open transport, start a listener, manage a filesystem socket
path, spawn a process, use external services, allow deployment, capture
traffic, or execute native inference.
Rust now also defines a typed `evidence_index_adapter.v0` for caller-provided
pointer-only `evidence_index.v0` JSON strings and bounded local `.json` files.
Qt still displays its own static Evidence Index preview object; it does not
call `parse_evidence_index_json`, call `parse_evidence_index_file`, read
evidence index files, bind to live runtime state, use a database or indexing
engine, load generated reports, copy raw evidence payloads, perform file I/O,
open transport, use external services, allow deployment, capture traffic, or
execute native inference.
Rust now also defines `runtime_workstation_snapshot.v0` through
`RuntimeWorkstationSnapshot`, `RuntimeWorkstationSnapshotAggregateSummary`,
and `RuntimeWorkstationSnapshotSafetyFlags`, with
`RuntimeWorkstationSnapshotProviderContract` and
`RuntimeWorkstationSnapshotProviderPolicy`. The provider composes the existing
`runtime_handoff_snapshot.v0` and `evidence_index.v0` into one local typed
snapshot. Qt still displays its own static preview objects; it does not call
`build_runtime_workstation_snapshot`, does not call
`parse_runtime_workstation_snapshot_json`, does not read
`RuntimeWorkstationSnapshot`, does not bind to live runtime state, does not
read generated evidence files, does not open transport, does not start a
listener, does not manage a filesystem socket path, does not spawn a process,
does not use external services, does not allow deployment, does not capture
traffic, and does not execute native inference.

Expected integration path:

```text
QML shell scaffold
  -> buildable Qt project in an environment with Qt 6 and CMake
  -> static runtime_summary.v0 handoff preview
  -> runtime_summary_provider.v0 in the Rust runtime, not called by Qt
  -> static model_registry_metadata.v0 handoff preview
  -> typed model_registry_metadata_adapter.v0 in the Rust runtime, not called by Qt
  -> static runtime_handoff_snapshot.v0 envelope in the Rust runtime
  -> static runtime_control_plane_adapter.v0 contract in the Rust runtime
  -> Rust JSON-string parser and bounded local file adapter for local handoff snapshots
  -> typed local control-plane command dispatcher in the Rust runtime
  -> strict local runtime_control_plane_message.v0 envelope in the Rust runtime
  -> bounded runtime_control_plane_frame.v0 byte-frame adapter in the Rust runtime
  -> bounded runtime_control_plane_ipc.v0 connected-stream adapter in the Rust runtime
  -> bounded runtime_control_plane_endpoint.v0 endpoint policy in the Rust runtime
  -> bounded runtime_control_plane_endpoint_path.v0 path policy in the Rust runtime, not called by Qt
  -> bounded one-shot runtime_control_plane_endpoint_listener.v0 OS-local listener in the Rust runtime, not called by Qt
  -> bounded one-shot runtime_control_plane_endpoint_lifecycle.v0 lifecycle wrapper in the Rust runtime, not called by Qt
  -> bounded one-shot runtime_control_plane_service_lifecycle.v0 service lifecycle wrapper in the Rust runtime, not called by Qt
  -> bounded in-memory runtime_registry_provider.v0 in the Rust runtime, not called by Qt
  -> bounded runtime_registry_storage_provider.v0 in the Rust runtime, not called by Qt
  -> typed evidence_index_adapter.v0 in the Rust runtime, not called by Qt
  -> static evidence_index.v0 handoff preview in QML
  -> runtime_workstation_snapshot.v0 over validated runtime handoff and pointer-only evidence index snapshots
  -> future supervised local runtime service daemon with explicit async start/stop
  -> typed model/evidence data adapters
  -> Rust/C++ runtime workspace/session integration
  -> native analyst workstation packaging
```

Local validation is currently static because the implementation environment does
not provide `cmake`, `qmake`, `qml`, or `qt-cmake`. The v0 gate therefore checks
source layout, expected Qt Quick/QML anchors, and local-only synthetic UI text
through pytest and repository validation. Once Qt tooling is available, add a
configure/build check and QML linting to the workstation validation gate.

## UI priorities

- session/workspace browser
- PCAP and telemetry import
- model score matrix
- model disagreement view
- incident timeline
- evidence graph
- feature evidence panel
- model registry/evaluation panel
- investigation notebook
- detection candidate review
- exportable reports

## Non-goals

- Do not replace ML roadmap with UI polish.
- Do not remove Streamlit until Qt workflow is useful.
- Do not build the full Qt app in one milestone.
