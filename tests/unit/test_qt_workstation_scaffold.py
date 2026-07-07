from __future__ import annotations

import json
import re
from pathlib import Path

from ares_netguard.models import registry_metadata

ROOT = Path(__file__).resolve().parents[2]
APP_DIR = ROOT / "apps" / "qt-workstation"
STRATEGY_DOC = ROOT / "docs" / "QT_WORKSTATION_STRATEGY.md"


def _read(relative_path: str) -> str:
    return (APP_DIR / relative_path).read_text(encoding="utf-8")


def _extract_qml_json_property(qml: str, property_name: str) -> dict[str, object]:
    marker = f"readonly property var {property_name}: ("
    start = qml.index(marker) + len(marker)
    while qml[start].isspace():
        start += 1
    if qml[start] != "{":
        raise AssertionError(f"{property_name} must begin with a JSON object")

    depth = 0
    in_string = False
    escaped = False
    for index in range(start, len(qml)):
        char = qml[index]
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return json.loads(qml[start : index + 1])

    raise AssertionError(f"{property_name} JSON object was not closed")


def test_qt_workstation_scaffold_files_exist() -> None:
    assert (APP_DIR / "CMakeLists.txt").is_file()
    assert (APP_DIR / "src" / "main.cpp").is_file()
    assert (APP_DIR / "qml" / "Main.qml").is_file()


def test_cmake_declares_qt6_quick_application() -> None:
    cmake = _read("CMakeLists.txt")

    assert "find_package(Qt6 6.5 REQUIRED COMPONENTS Quick)" in cmake
    assert "qt_add_executable(ares_qt_workstation" in cmake
    assert "qt_add_qml_module(ares_qt_workstation" in cmake
    assert "qml/Main.qml" in cmake
    assert "Qt6::Quick" in cmake


def test_main_bootstraps_qml_application_engine() -> None:
    main_cpp = _read("src/main.cpp")

    assert "#include <QGuiApplication>" in main_cpp
    assert "#include <QQmlApplicationEngine>" in main_cpp
    assert "QGuiApplication app(argc, argv);" in main_cpp
    assert "QQmlApplicationEngine engine;" in main_cpp
    assert "AresNetGuard/Workstation/Main.qml" in main_cpp
    assert "objectCreationFailed" in main_cpp


def test_qml_imports_and_workstation_areas() -> None:
    qml = _read("qml/Main.qml")

    assert "import QtQuick" in qml
    assert "import QtQuick.Controls" in qml
    assert "import QtQuick.Layouts" in qml
    assert "required property int index" in qml
    assert "required property var modelData" in qml

    expected_anchors = [
        "workstationShellRoot",
        "leftNavigation",
        "modelEvidenceWorkspace",
        "modelDisagreementSummary",
        "modelEvidenceMatrix",
        "rightDetailPanel",
        "selectedEntityDetail",
        "runtimeBoundaryPanel",
        "evidenceDetailPanel",
        "analystActionPanel",
        "modelRegistrySnapshot",
    ]
    for anchor in expected_anchors:
        assert anchor in qml

    expected_text = [
        "Workspace",
        "Model Disagreement",
        "Evidence Graph",
        "Investigation",
        "Detection Candidates",
        "Model Registry",
        "Entity Evidence Matrix",
        "Next Analyst Actions",
        "Runtime Boundary",
        "no live runtime connection",
    ]
    for text in expected_text:
        assert text in qml


def test_qml_runtime_summary_fields_mirror_rust_contract() -> None:
    qml = _read("qml/Main.qml")

    expected_fields = [
        "schema_version",
        "workspace_id",
        "session_id",
        "total_job_count",
        "queued_job_count",
        "running_job_count",
        "failed_job_count",
        "last_event_label",
        "native_inference_state",
    ]
    for field in expected_fields:
        assert f'"{field}"' in qml
        assert f"root.runtimeSummary.{field}" in qml

    assert '"runtime_summary.v0"' in qml
    assert '"fixture-workspace-alpha"' in qml
    assert '"fixture-session-runtime-summary"' in qml
    assert '"disabled"' in qml


def test_qml_registry_metadata_fields_mirror_registry_contract() -> None:
    qml = _read("qml/Main.qml")

    expected_fields = [
        "schema_version",
        "metadata_scope",
        "source_bundle_schema",
        "entries",
        "aggregate_summary",
        "model_id",
        "registry_state",
        "promotion_state",
        "observed_source_schemas",
        "observed_source_names",
        "source_count",
        "has_score_rows",
        "human_review_required",
        "deployment_allowed",
        "model_count",
        "schemas_present",
        "models_with_score_rows",
        "safety_flags",
        "non_claims",
    ]
    for field in expected_fields:
        assert f'"{field}"' in qml

    metadata = _extract_qml_json_property(qml, "modelRegistryMetadata")
    registry_metadata.validate_registry_metadata(metadata)
    assert metadata["aggregate_summary"]["model_count"] == 10
    assert [entry["model_id"] for entry in metadata["entries"]] == [
        "graph_novelty",
        "isolation_forest",
        "model_disagreement",
        "pyod_copod",
        "pyod_ecod",
        "river_hst",
        "self_supervised_representation",
        "stdlib_linear_native",
        "suricata_alert",
        "time_series_residual",
    ]
    assert metadata["aggregate_summary"]["models_with_score_rows"] == [
        "graph_novelty",
        "isolation_forest",
        "pyod_copod",
        "pyod_ecod",
        "river_hst",
        "stdlib_linear_native",
        "suricata_alert",
        "time_series_residual",
    ]

    expected_references = [
        "root.modelRegistryMetadata.schema_version",
        "root.modelRegistryMetadata.source_bundle_schema",
        "root.modelRegistryMetadata.aggregate_summary.model_count",
        "root.modelRegistryMetadata.aggregate_summary.models_with_score_rows.length",
        "root.modelRegistryMetadata.entries[0].registry_state",
        "root.modelRegistryMetadata.entries[0].promotion_state",
        "root.modelRegistryMetadata.entries[0].human_review_required",
        "root.modelRegistryMetadata.aggregate_summary.deployment_allowed",
    ]
    for reference in expected_references:
        assert reference in qml

    expected_values = [
        '"model_registry_metadata.v0"',
        '"local_synthetic_model_registry_metadata"',
        '"model_evaluation_bundle.v0"',
        '"observed_synthetic_only"',
        '"not_promoted"',
        '"graph_novelty"',
        '"pyod_copod"',
        '"river_hst"',
        '"self_supervised_representation"',
        '"stdlib_linear_native"',
        '"suricata_alert"',
        '"time_series_residual"',
    ]
    for value in expected_values:
        assert value in qml

    assert "4 detectors / 1 disagreement report / 0 exported artifacts" not in qml


def test_qml_uses_static_synthetic_local_content_only() -> None:
    qml = _read("qml/Main.qml")
    lowered = qml.lower()

    assert "synthetic" in lowered
    assert "local scaffold" in lowered
    assert "static synthetic fixture" in lowered

    forbidden_terms = [
        "live capture",
        "external service",
        "external api",
        "third-party",
        "private telemetry",
        "public scan",
        "packet capture",
        "pcap import",
        "api key",
        "bearer token",
    ]
    for term in forbidden_terms:
        assert term not in lowered

    assert re.search(r"\b(?:\d{1,3}\.){3}\d{1,3}\b", qml) is None
    assert re.search(r"\b[A-Za-z0-9.-]+\.(?:com|net|org|io)\b", qml) is None


def test_qt_strategy_documents_runtime_summary_static_handoff() -> None:
    strategy = STRATEGY_DOC.read_text(encoding="utf-8")
    normalized_strategy = " ".join(strategy.split())

    expected_text = [
        "Runtime Boundary panel",
        "Rust-owned `runtime_summary.v0` fields",
        "static synthetic QML object",
        "not a live runtime connection",
        "runtime_summary_provider.v0",
        "caller-provided local `RuntimeEvent` slices",
        "build_runtime_summary_from_events",
        "Qt still displays its own static Runtime Boundary preview object",
        "does not call the provider",
        "runtime_summary_provider.v0 in the Rust runtime, not called by Qt",
        "static runtime_summary.v0 handoff preview",
        "runtime_handoff_snapshot.v0",
        "static runtime_handoff_snapshot.v0 envelope in the Rust runtime",
        "runtime_control_plane_adapter.v0",
        "static runtime_control_plane_adapter.v0 contract in the Rust runtime",
        "Rust JSON-string parser and bounded local file adapter for local handoff snapshots",
        "typed local control-plane command dispatcher",
        "runtime_control_plane_message.v0",
        "request/response message envelope",
        "safe request identifier",
        "typed success/failure responses",
        "runtime_control_plane_frame.v0",
        "bounded local `runtime_control_plane_frame.v0` byte-frame adapter",
        "caller-provided UTF-8 JSON bytes",
        "256 KiB default frame cap",
        "serialized UTF-8 JSON response bytes",
        "Qt does not parse frame bytes",
        "call the frame adapter",
        "bounded runtime_control_plane_frame.v0 byte-frame adapter in the Rust runtime",
        "runtime_control_plane_ipc.v0",
        "bounded `runtime_control_plane_ipc.v0` connected-stream adapter",
        "caller-provided streams",
        "4-byte big-endian length prefix",
        "one-shot request/response execution path",
        "read_control_plane_message_ipc_frame",
        "write_control_plane_message_ipc_frame",
        "execute_control_plane_message_ipc_stream",
        "bounded runtime_control_plane_ipc.v0 connected-stream adapter in the Rust runtime",
        "runtime_control_plane_endpoint.v0",
        "bounded `runtime_control_plane_endpoint.v0` endpoint policy",
        "caller-provided connected stream endpoint kind",
        "strict endpoint policy validation",
        "execute_control_plane_endpoint_stream",
        "bounded runtime_control_plane_endpoint.v0 endpoint policy in the Rust runtime",
        "runtime_control_plane_endpoint_path.v0",
        "bounded `runtime_control_plane_endpoint_path.v0` path policy",
        "validate_control_plane_endpoint_path",
        "Qt still does not call or read the endpoint path policy",
        (
            "bounded runtime_control_plane_endpoint_path.v0 path policy in the Rust runtime, "
            "not called by Qt"
        ),
        "runtime_control_plane_endpoint_listener.v0",
        "one-shot OS-local listener",
        "execute_control_plane_endpoint_listener_once",
        "Qt still does not call or bind the listener",
        "run a listener loop",
        (
            "bounded one-shot runtime_control_plane_endpoint_listener.v0 OS-local listener "
            "in the Rust runtime, not called by Qt"
        ),
        "runtime_control_plane_endpoint_lifecycle.v0",
        "one-shot endpoint lifecycle wrapper",
        "execute_control_plane_endpoint_lifecycle_once",
        "Qt still does not call or bind the lifecycle wrapper",
        "start or stop endpoint lifecycle",
        (
            "bounded one-shot runtime_control_plane_endpoint_lifecycle.v0 lifecycle wrapper "
            "in the Rust runtime, not called by Qt"
        ),
        "runtime_control_plane_service_lifecycle.v0",
        "service lifecycle state wrapper",
        "execute_control_plane_service_lifecycle_once",
        "Qt still does not call or bind the service lifecycle wrapper",
        "own service lifecycle state",
        "start or stop service lifecycle",
        (
            "bounded one-shot runtime_control_plane_service_lifecycle.v0 service lifecycle "
            "wrapper in the Rust runtime, not called by Qt"
        ),
        "runtime_registry_provider.v0",
        "bounded in-memory `runtime_registry_provider.v0`",
        "runtime_handoff_snapshot.v0` values",
        "`RuntimeRegistrySnapshot` sorted by workspace/session key",
        "does not call `RuntimeRegistryProvider`",
        "read `RuntimeRegistrySnapshot`",
        "runtime_registry_storage_provider.v0",
        "bounded `runtime_registry_storage_provider.v0` local JSON storage provider",
        "does not call `RuntimeRegistryStorageProvider`",
        "read `RuntimeRegistryStorageDocument`",
        "read registry storage files",
        "read persistent storage",
        "use a database or indexing engine",
        "load generated JSON",
        "bounded in-memory runtime_registry_provider.v0 in the Rust runtime, not called by Qt",
        "bounded runtime_registry_storage_provider.v0 in the Rust runtime, not called by Qt",
        "future supervised local runtime service daemon with explicit async start/stop",
        "perform file I/O",
        "open IPC",
        "bind to live runtime state",
        "start a listener",
        "manage a filesystem socket path",
        "spawn a process",
    ]
    for text in expected_text:
        assert text in normalized_strategy


def test_qt_strategy_documents_registry_metadata_static_handoff() -> None:
    strategy = STRATEGY_DOC.read_text(encoding="utf-8")
    normalized_strategy = " ".join(strategy.split())

    expected_text = [
        "Model Registry Snapshot panel",
        "`model_registry_metadata.v0` fields",
        "static synthetic QML object",
        "not a persistent registry",
        "does not read generated reports",
        "model_registry_metadata_adapter.v0",
        "does not call `parse_model_registry_metadata_json`",
        "call `parse_model_registry_metadata_file`",
        "not called by Qt",
        "runtime_control_plane_adapter.v0",
        "Rust JSON-string parser and bounded local file adapter for local handoff snapshots",
        "typed local control-plane command dispatcher",
        "runtime_control_plane_message.v0",
        "request/response message envelope",
        "runtime_control_plane_frame.v0",
        "byte-frame adapter",
        "runtime_control_plane_ipc.v0",
        "connected-stream adapter",
        "runtime_control_plane_endpoint.v0",
        "endpoint policy",
        "runtime_control_plane_endpoint_path.v0",
        "endpoint path policy",
        "runtime_control_plane_endpoint_listener.v0",
        "one-shot OS-local listener",
        "execute_control_plane_endpoint_listener_once",
        "runtime_control_plane_endpoint_lifecycle.v0",
        "one-shot endpoint lifecycle wrapper",
        "execute_control_plane_endpoint_lifecycle_once",
        "runtime_control_plane_service_lifecycle.v0",
        "service lifecycle state wrapper",
        "execute_control_plane_service_lifecycle_once",
        "runtime_registry_provider.v0",
        "does not call `RuntimeRegistryProvider`",
        "read `RuntimeRegistrySnapshot`",
        "runtime_registry_storage_provider.v0",
        "does not call `RuntimeRegistryStorageProvider`",
        "read `RuntimeRegistryStorageDocument`",
        "not called by Qt",
        "future supervised local runtime service daemon with explicit async start/stop",
    ]
    for text in expected_text:
        assert text in normalized_strategy
