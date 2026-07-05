from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
APP_DIR = ROOT / "apps" / "qt-workstation"


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

    expected_anchors = [
        "workstationShellRoot",
        "leftNavigation",
        "modelEvidenceWorkspace",
        "modelDisagreementSummary",
        "modelEvidenceMatrix",
        "rightDetailPanel",
        "selectedEntityDetail",
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
    ]
    for text in expected_text:
        assert text in qml


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
