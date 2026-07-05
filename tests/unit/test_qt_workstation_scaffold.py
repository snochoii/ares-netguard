from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
APP_DIR = ROOT / "apps" / "qt-workstation"
STRATEGY_DOC = ROOT / "docs" / "QT_WORKSTATION_STRATEGY.md"


def _read(relative_path: str) -> str:
    return (APP_DIR / relative_path).read_text(encoding="utf-8")


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
        "source_count",
        "has_score_rows",
        "human_review_required",
        "deployment_allowed",
        "model_count",
        "schemas_present",
        "models_with_score_rows",
    ]
    for field in expected_fields:
        assert f'"{field}"' in qml

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
        '"stdlib_linear_native"',
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
        "static runtime_summary.v0 handoff preview",
        "future JSON/control-plane adapter from the Rust runtime",
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
        "future JSON/control-plane adapter from the Rust runtime",
    ]
    for text in expected_text:
        assert text in normalized_strategy
