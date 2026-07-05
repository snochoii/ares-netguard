from __future__ import annotations

import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUST_CORE_DIR = ROOT / "apps" / "rust-core"
STRATEGY_DOC = ROOT / "docs" / "RUST_CPP_RUNTIME_STRATEGY.md"


def _read(relative_path: str) -> str:
    return (RUST_CORE_DIR / relative_path).read_text(encoding="utf-8")


def test_rust_core_scaffold_files_exist() -> None:
    assert (RUST_CORE_DIR / "Cargo.toml").is_file()
    assert (RUST_CORE_DIR / "src" / "lib.rs").is_file()
    assert (RUST_CORE_DIR / "src" / "main.rs").is_file()
    assert STRATEGY_DOC.is_file()


def test_cargo_manifest_declares_dependency_free_runtime_package() -> None:
    manifest = tomllib.loads(_read("Cargo.toml"))

    assert manifest["package"]["name"] == "ares-rust-core"
    assert manifest["package"]["edition"] == "2021"
    assert manifest["lib"]["name"] == "ares_rust_core"
    assert manifest["lib"]["path"] == "src/lib.rs"
    assert manifest["bin"] == [{"name": "ares-rust-core", "path": "src/main.rs"}]
    assert manifest.get("dependencies", {}) == {}
    assert manifest.get("build-dependencies", {}) == {}
    assert manifest.get("dev-dependencies", {}) == {}


def test_rust_core_exposes_expected_runtime_contract_anchors() -> None:
    lib_rs = _read("src/lib.rs")

    expected_anchors = [
        "RUNTIME_CONTRACT_VERSION",
        "WorkspaceId",
        "SessionId",
        "JobId",
        "JobKind",
        "JobState",
        "RuntimeEvent",
        "CompareModelScores",
        "RefreshEvidenceIndex",
        "RunNativeInferenceCandidate",
        "RenderWorkstationSnapshot",
    ]
    for anchor in expected_anchors:
        assert anchor in lib_rs

    assert 'pub const RUNTIME_CONTRACT_VERSION: &str = "rust_runtime_contract.v0";' in lib_rs
    assert "fn validate_coarse_id(" in lib_rs
    assert "RuntimeIdError::RawIdentifier" in lib_rs


def test_rust_core_source_stays_local_contract_only() -> None:
    rust_source = "\n".join(
        [
            _read("src/lib.rs"),
            _read("src/main.rs"),
        ]
    )
    lowered = rust_source.lower()

    forbidden_terms = [
        "live capture",
        "socket",
        "tcpstream",
        "udp",
        "std::net",
        "std::process",
        "command::new",
        "model artifact",
        "private telemetry",
        "pcap",
        "packet capture",
        "http",
        "https",
        "api key",
        "bearer token",
    ]
    for term in forbidden_terms:
        assert term not in lowered

    assert re.search(r"\b(?:\d{1,3}\.){3}\d{1,3}\b", rust_source) is None
    assert re.search(r"\b[A-Za-z0-9.-]+\.(?:com|net|org|io)\b", rust_source) is None


def test_runtime_strategy_documents_v0_limits_and_migration() -> None:
    strategy = STRATEGY_DOC.read_text(encoding="utf-8")
    normalized_strategy = " ".join(strategy.split())

    expected_text = [
        "source-only contract",
        "no-dependency Rust package boundary",
        "does not implement a daemon",
        "does not provide `cargo` or `rustc`",
        "Qt workstation data-flow integration",
        "Python ML Lab report handoff",
        "cargo fmt --check",
        "cargo test",
        "cargo clippy",
    ]
    for text in expected_text:
        assert text in normalized_strategy
