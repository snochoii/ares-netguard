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
- compact Model Registry Snapshot panel that mirrors
  `model_registry_metadata.v0` fields from a static synthetic QML object;
- right detail panel for selected entity context, evidence, analyst actions,
  and registry status.

This milestone makes no packaged application, runtime integration, model
execution, live capture, external service, or private telemetry claim. All v0 UI
content is static and synthetic. The Runtime Boundary panel is a source-level
handoff preview only; it is not a live runtime connection and does not make the
Qt shell the owner of workspace, session, job, or native inference state.
The Model Registry Snapshot panel is also a source-level handoff preview only;
it is not a persistent registry, does not read generated reports, and does not
own model promotion or deployment state.
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
frame bytes, call the frame adapter, perform file I/O, open IPC, bind to live
runtime state, load generated reports, use external services, allow deployment,
capture traffic, or execute native inference.

Expected integration path:

```text
QML shell scaffold
  -> buildable Qt project in an environment with Qt 6 and CMake
  -> static runtime_summary.v0 handoff preview
  -> static model_registry_metadata.v0 handoff preview
  -> static runtime_handoff_snapshot.v0 envelope in the Rust runtime
  -> static runtime_control_plane_adapter.v0 contract in the Rust runtime
  -> Rust JSON-string parser and bounded local file adapter for local handoff snapshots
  -> typed local control-plane command dispatcher in the Rust runtime
  -> strict local runtime_control_plane_message.v0 envelope in the Rust runtime
  -> bounded runtime_control_plane_frame.v0 byte-frame adapter in the Rust runtime
  -> future local IPC/control-plane adapter from the Rust runtime
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
