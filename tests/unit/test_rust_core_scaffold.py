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
        "RUNTIME_SUMMARY_SCHEMA_VERSION",
        "MODEL_REGISTRY_METADATA_SCHEMA_VERSION",
        "MODEL_REGISTRY_METADATA_SCOPE",
        "MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION",
        "WorkspaceId",
        "SessionId",
        "JobId",
        "JobKind",
        "JobState",
        "RuntimeSummary",
        "ModelRegistryMetadata",
        "ModelRegistryEntry",
        "ModelRegistryAggregateSummary",
        "ModelRegistrySafetyFlags",
        "ModelRegistryState",
        "ModelPromotionState",
        "RuntimeEvent",
        "NativeInferenceRuntimeState",
        "CompareModelScores",
        "RefreshEvidenceIndex",
        "RunNativeInferenceCandidate",
        "RenderWorkstationSnapshot",
        "Unavailable",
        "Available",
        "Disabled",
        "ObservedSyntheticOnly",
        "NotPromoted",
        "synthetic_fixture",
    ]
    for anchor in expected_anchors:
        assert anchor in lib_rs

    assert 'pub const RUNTIME_CONTRACT_VERSION: &str = "rust_runtime_contract.v0";' in lib_rs
    assert 'pub const RUNTIME_SUMMARY_SCHEMA_VERSION: &str = "runtime_summary.v0";' in lib_rs
    assert (
        'pub const MODEL_REGISTRY_METADATA_SCHEMA_VERSION: &str = "model_registry_metadata.v0";'
        in lib_rs
    )
    assert "fn validate_coarse_id(" in lib_rs
    assert "RuntimeIdError::RawIdentifier" in lib_rs


def test_rust_core_exposes_runtime_summary_contract_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub schema_version: &'static str",
        "pub workspace_id: WorkspaceId",
        "pub session_id: SessionId",
        "pub total_job_count: u32",
        "pub queued_job_count: u32",
        "pub running_job_count: u32",
        "pub failed_job_count: u32",
        "pub last_event_label: &'static str",
        "pub native_inference_state: NativeInferenceRuntimeState",
    ]
    for field in expected_fields:
        assert field in lib_rs

    assert "impl RuntimeSummary" in lib_rs
    assert "pub fn synthetic_fixture() -> Self" in lib_rs
    assert 'WorkspaceId::new("fixture-workspace-alpha")' in lib_rs
    assert 'SessionId::new("fixture-session-runtime-summary")' in lib_rs
    assert "native_inference_state: NativeInferenceRuntimeState::Disabled" in lib_rs


def test_rust_core_exposes_model_registry_metadata_contract_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub schema_version: &'static str",
        "pub metadata_scope: &'static str",
        "pub source_bundle_schema: &'static str",
        "pub entries: &'static [ModelRegistryEntry]",
        "pub aggregate_summary: ModelRegistryAggregateSummary",
        "pub safety_flags: ModelRegistrySafetyFlags",
        "pub non_claims: &'static [&'static str]",
        "pub model_id: &'static str",
        "pub registry_state: ModelRegistryState",
        "pub promotion_state: ModelPromotionState",
        "pub observed_source_schemas: &'static [&'static str]",
        "pub observed_source_names: &'static [&'static str]",
        "pub source_count: u32",
        "pub has_score_rows: bool",
        "pub human_review_required: bool",
        "pub deployment_allowed: bool",
        "pub model_count: u32",
        "pub schemas_present: &'static [&'static str]",
        "pub models_with_score_rows: &'static [&'static str]",
        "pub local_only: bool",
        "pub strict_json_loaded: bool",
        "pub derived_from_evaluation_bundle_only: bool",
        "pub input_paths_copied: bool",
        "pub source_filenames_copied: bool",
        "pub raw_identifiers_copied: bool",
        "pub generated_artifact_references_copied: bool",
        "pub secrets_detected: bool",
        "pub report_payload_copied: bool",
        "pub live_capture_used: bool",
        "pub external_services_used: bool",
    ]
    for field in expected_fields:
        assert field in lib_rs

    assert "impl ModelRegistryMetadata" in lib_rs
    assert "impl ModelRegistryState" in lib_rs
    assert "impl ModelPromotionState" in lib_rs
    assert "MODEL_REGISTRY_METADATA_ENTRIES" in lib_rs


def test_rust_core_static_registry_fixture_matches_validated_metadata_snapshot() -> None:
    lib_rs = _read("src/lib.rs")

    expected_values = [
        '"model_registry_metadata.v0"',
        '"local_synthetic_model_registry_metadata"',
        '"model_evaluation_bundle.v0"',
        '"observed_synthetic_only"',
        '"not_promoted"',
        '"isolation_forest"',
        '"model_disagreement"',
        '"pyod_ecod"',
        '"stdlib_linear_native"',
        '"agentic_investigation_report.v0"',
        '"detection_candidate_report.v0"',
        '"model_disagreement_report.v0"',
        '"model_score_rows.v0"',
        '"agentic_investigation_report_v0_001"',
        '"detection_candidate_report_v0_001"',
        '"model_disagreement_report_v0_001"',
        '"model_score_rows_v0_001"',
        '"not_persistent_model_registry"',
        '"not_model_promotion_gate"',
        '"not_deployment_approval"',
        '"not_live_capture"',
        '"not_external_enrichment"',
        '"not_rule_deployment"',
        '"not_native_runtime_execution"',
    ]
    for value in expected_values:
        assert value in lib_rs

    assert "model_count: 4" in lib_rs
    assert "source_count: 2" in lib_rs
    assert "source_count: 1" in lib_rs
    assert "has_score_rows: true" in lib_rs
    assert "has_score_rows: false" in lib_rs
    assert "human_review_required: true" in lib_rs
    assert "deployment_allowed: false" in lib_rs
    assert "local_only: true" in lib_rs
    assert "strict_json_loaded: true" in lib_rs
    assert "derived_from_evaluation_bundle_only: true" in lib_rs
    assert "input_paths_copied: false" in lib_rs
    assert "source_filenames_copied: false" in lib_rs
    assert "raw_identifiers_copied: false" in lib_rs
    assert "generated_artifact_references_copied: false" in lib_rs
    assert "secrets_detected: false" in lib_rs
    assert "report_payload_copied: false" in lib_rs
    assert "live_capture_used: false" in lib_rs
    assert "external_services_used: false" in lib_rs

    entry_model_ids = re.findall(r'model_id: "([^"]+)"', lib_rs)
    assert entry_model_ids == [
        "isolation_forest",
        "model_disagreement",
        "pyod_ecod",
        "stdlib_linear_native",
    ]


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
        "std::fs",
        "file::open",
        "std::process",
        "command::new",
        "model artifact",
        "private telemetry",
        "pcap",
        "packet capture",
        "http",
        "https",
        "api key",
        "api_key",
        "bearer token",
        "access token",
        "refresh token",
        ".onnx",
        ".pt",
        ".pth",
        ".joblib",
        ".pkl",
        ".duckdb",
        ".sqlite",
        ".jsonl",
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
        "runtime_summary.v0",
        "RuntimeSummary",
        "NativeInferenceRuntimeState",
        "model_registry_metadata.v0",
        "ModelRegistryMetadata",
        "ModelRegistryEntry",
        "ModelRegistryAggregateSummary",
        "ModelRegistrySafetyFlags",
        "static runtime_summary.v0 handoff",
        "static model_registry_metadata.v0 handoff",
        "real Rust runtime summary provider",
        "typed registry metadata adapter",
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
