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
- right detail panel for selected entity context, evidence, analyst actions,
  and registry status.

This milestone makes no packaged application, runtime integration, model
execution, live capture, external service, or private telemetry claim. All v0 UI
content is static and synthetic.

Expected integration path:

```text
QML shell scaffold
  -> buildable Qt project in an environment with Qt 6 and CMake
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
