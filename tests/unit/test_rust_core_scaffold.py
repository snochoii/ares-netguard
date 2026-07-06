from __future__ import annotations

import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUST_CORE_DIR = ROOT / "apps" / "rust-core"
STRATEGY_DOC = ROOT / "docs" / "RUST_CPP_RUNTIME_STRATEGY.md"
MAKEFILE = ROOT / "Makefile"


def _read(relative_path: str) -> str:
    return (RUST_CORE_DIR / relative_path).read_text(encoding="utf-8")


def test_rust_core_scaffold_files_exist() -> None:
    assert (RUST_CORE_DIR / "Cargo.toml").is_file()
    assert (RUST_CORE_DIR / "src" / "lib.rs").is_file()
    assert (RUST_CORE_DIR / "src" / "main.rs").is_file()
    assert STRATEGY_DOC.is_file()
    assert MAKEFILE.is_file()


def test_cargo_manifest_declares_only_strict_json_parser_dependencies() -> None:
    manifest = tomllib.loads(_read("Cargo.toml"))

    assert manifest["package"]["name"] == "ares-rust-core"
    assert manifest["package"]["edition"] == "2021"
    assert manifest["lib"]["name"] == "ares_rust_core"
    assert manifest["lib"]["path"] == "src/lib.rs"
    assert manifest["bin"] == [{"name": "ares-rust-core", "path": "src/main.rs"}]
    assert manifest.get("dependencies", {}) == {
        "serde": {"version": "1", "features": ["derive"]},
        "serde_json": "1",
    }
    assert manifest.get("build-dependencies", {}) == {}
    assert manifest.get("dev-dependencies", {}) == {}


def test_rust_core_exposes_expected_runtime_contract_anchors() -> None:
    lib_rs = _read("src/lib.rs")

    expected_anchors = [
        "RUNTIME_CONTRACT_VERSION",
        "RUNTIME_SUMMARY_SCHEMA_VERSION",
        "RUNTIME_SUMMARY_PROVIDER_SCHEMA_VERSION",
        "RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION",
        "RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION",
        "RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP",
        "RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES",
        "MODEL_REGISTRY_METADATA_SCHEMA_VERSION",
        "MODEL_REGISTRY_METADATA_ADAPTER_SCHEMA_VERSION",
        "MODEL_REGISTRY_METADATA_SCOPE",
        "MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION",
        "RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION",
        "RUNTIME_CONTROL_PLANE_ADAPTER_SCHEMA_VERSION",
        "RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION",
        "RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION",
        "RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION",
        "RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION",
        "RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES",
        "RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES",
        "RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES",
        "RUNTIME_CONTROL_PLANE_REQUEST_ID_MAX_BYTES",
        "WorkspaceId",
        "SessionId",
        "JobId",
        "JobKind",
        "JobState",
        "RuntimeSummary",
        "RuntimeSummaryProviderContract",
        "RuntimeSummaryProviderPolicy",
        "RuntimeRegistryProviderContract",
        "RuntimeRegistryProviderPolicy",
        "RuntimeRegistryRecord",
        "RuntimeRegistrySnapshot",
        "RuntimeRegistryProvider",
        "RuntimeRegistryStorageProviderContract",
        "RuntimeRegistryStoragePolicy",
        "RuntimeRegistryStorageDocument",
        "RuntimeRegistryStorageProvider",
        "ModelRegistryMetadata",
        "ModelRegistryEntry",
        "ModelRegistryAggregateSummary",
        "ModelRegistrySafetyFlags",
        "ModelRegistryMetadataAdapterContract",
        "ModelRegistryMetadataAdapterPolicy",
        "ModelRegistryState",
        "ModelPromotionState",
        "RuntimeHandoffSnapshot",
        "RuntimeHandoffSourceKind",
        "RuntimeHandoffTransportState",
        "RuntimeControlPlaneState",
        "RuntimeControlPlaneAdapterContract",
        "RuntimeControlPlaneAdapterKind",
        "RuntimeControlPlaneInputMode",
        "RuntimeControlPlaneAdapterState",
        "RuntimeControlPlaneOutputSnapshotSchema",
        "RuntimeControlPlaneFramePolicy",
        "RuntimeControlPlaneIpcPolicy",
        "RuntimeControlPlaneEndpointPolicy",
        "RuntimeControlPlaneFrameAdapterContract",
        "RuntimeControlPlaneIpcAdapterContract",
        "RuntimeControlPlaneEndpointAdapterContract",
        "RuntimeControlPlaneEndpointKind",
        "RuntimeControlPlaneFilePolicy",
        "RuntimeControlPlaneCommand",
        "RuntimeControlPlaneRequestId",
        "RuntimeControlPlaneMessageRequest",
        "RuntimeControlPlaneMessageResponse",
        "RuntimeControlPlaneMessageOutcome",
        "RuntimeControlPlaneMessageErrorCode",
        "RuntimeControlPlaneAdapterError",
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
        "StaticSyntheticFixture",
        "StaticContractFixture",
        "LocalJsonStringParser",
        "LocalJsonFileAdapter",
        "LocalControlPlaneMessageEnvelope",
        "LocalControlPlaneFrameAdapter",
        "LocalControlPlaneIpcStreamAdapter",
        "LocalControlPlaneEndpointPolicy",
        "AcceptedSchemaDeclarationOnly",
        "AcceptedLocalJsonString",
        "AcceptedLocalJsonFile",
        "AcceptedLocalMessageEnvelope",
        "AcceptedLocalMessageFrame",
        "AcceptedLocalIpcStream",
        "AcceptedLocalEndpointPolicy",
        "JsonStringParserAvailable",
        "LocalFileAdapterAvailable",
        "LocalMessageEnvelopeAvailable",
        "LocalMessageFrameAvailable",
        "LocalIpcStreamAvailable",
        "LocalEndpointPolicyAvailable",
        "CallerProvidedConnectedStream",
        "RuntimeHandoffSnapshotV0",
        "ParseHandoffSnapshotJson",
        "ParseHandoffSnapshotFile",
        "Success",
        "Failure",
        "InvalidJson",
        "OversizedFrame",
        "IpcReadFailed",
        "IpcWriteFailed",
        "MalformedIpcFrame",
        "IncompleteIpcFrame",
        "UnsafeFlag",
        "synthetic_fixture",
        "parse_handoff_snapshot_json",
        "parse_handoff_snapshot_file",
        "build_runtime_summary_from_events",
        "upsert_snapshot",
        "persist_snapshot_file",
        "load_snapshot_file",
        "parse_storage_document_json",
        "persist_runtime_registry_snapshot_file",
        "load_runtime_registry_snapshot_file",
        "parse_runtime_registry_storage_document_json",
        "default_provider",
        "parse_model_registry_metadata_json",
        "parse_model_registry_metadata_file",
        "execute_local_command",
        "parse_control_plane_message_request_json",
        "execute_control_plane_message_request",
        "execute_control_plane_message_json",
        "serialize_control_plane_message_response_json",
        "parse_control_plane_message_frame_bytes",
        "execute_control_plane_message_frame_bytes",
        "serialize_control_plane_message_response_frame_bytes",
        "read_control_plane_message_ipc_frame",
        "write_control_plane_message_ipc_frame",
        "execute_control_plane_message_ipc_stream",
        "execute_control_plane_endpoint_stream",
        "validate_control_plane_endpoint_policy",
        "command_kind",
        "output_snapshot_schema",
    ]
    for anchor in expected_anchors:
        assert anchor in lib_rs

    assert 'pub const RUNTIME_CONTRACT_VERSION: &str = "rust_runtime_contract.v0";' in lib_rs
    assert 'pub const RUNTIME_SUMMARY_SCHEMA_VERSION: &str = "runtime_summary.v0";' in lib_rs
    assert (
        'pub const RUNTIME_SUMMARY_PROVIDER_SCHEMA_VERSION: &str = "runtime_summary_provider.v0";'
        in lib_rs
    )
    assert (
        "pub const RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION: &str = "
        '"runtime_registry_provider.v0";' in lib_rs
    )
    assert (
        "pub const RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION: &str = "
        '"runtime_registry_storage_provider.v0";' in " ".join(lib_rs.split())
    )
    assert "pub const RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP: usize = 64;" in lib_rs
    assert "pub const RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES: u64 = 1024 * 1024;" in lib_rs
    assert (
        'pub const MODEL_REGISTRY_METADATA_SCHEMA_VERSION: &str = "model_registry_metadata.v0";'
        in lib_rs
    )
    assert (
        "pub const MODEL_REGISTRY_METADATA_ADAPTER_SCHEMA_VERSION: &str ="
        ' "model_registry_metadata_adapter.v0";' in " ".join(lib_rs.split())
    )
    assert (
        "pub const RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION: &str = "
        '"runtime_handoff_snapshot.v0";' in lib_rs
    )
    assert (
        "pub const RUNTIME_CONTROL_PLANE_ADAPTER_SCHEMA_VERSION: &str ="
        ' "runtime_control_plane_adapter.v0";' in " ".join(lib_rs.split())
    )
    assert (
        "pub const RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION: &str ="
        ' "runtime_control_plane_frame.v0";' in " ".join(lib_rs.split())
    )
    assert (
        "pub const RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION: &str ="
        ' "runtime_control_plane_endpoint.v0";' in " ".join(lib_rs.split())
    )
    assert (
        "pub const RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION: &str ="
        ' "runtime_control_plane_ipc.v0";' in " ".join(lib_rs.split())
    )
    assert (
        "pub const RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION: &str ="
        ' "runtime_control_plane_message.v0";' in " ".join(lib_rs.split())
    )
    assert "pub const RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES: usize = 4;" in lib_rs
    assert "fn validate_coarse_id(" in lib_rs
    assert "fn validate_control_plane_request_id(" in lib_rs
    assert "RuntimeIdError::RawIdentifier" in lib_rs
    assert "serde_json::from_str" in lib_rs
    assert "serde_json::to_string" in lib_rs
    assert "serde_json::from_value" not in lib_rs
    assert "use std::fs;" in lib_rs
    assert "use std::path::{Path, PathBuf};" in lib_rs
    assert "#[serde(deny_unknown_fields)]" in lib_rs


def test_rust_core_exposes_runtime_summary_contract_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub schema_version: String",
        "pub workspace_id: WorkspaceId",
        "pub session_id: SessionId",
        "pub total_job_count: u32",
        "pub queued_job_count: u32",
        "pub running_job_count: u32",
        "pub failed_job_count: u32",
        "pub last_event_label: String",
        "pub native_inference_state: NativeInferenceRuntimeState",
    ]
    for field in expected_fields:
        assert field in lib_rs

    assert "impl RuntimeSummary" in lib_rs
    assert "pub fn synthetic_fixture() -> Self" in lib_rs
    assert 'WorkspaceId::new("fixture-workspace-alpha")' in lib_rs
    assert 'SessionId::new("fixture-session-runtime-summary")' in lib_rs
    assert "native_inference_state: NativeInferenceRuntimeState::Disabled" in lib_rs


def test_rust_core_exposes_runtime_summary_provider_contract_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub output_summary_schema: &'static str",
        "pub local_only: bool",
        "pub caller_provided_events_only: bool",
        "pub event_replay_enabled: bool",
        "pub storage_provider_enabled: bool",
        "pub live_runtime_connection_enabled: bool",
        "pub file_io_enabled: bool",
        "pub process_spawning_enabled: bool",
        "pub qt_binding_enabled: bool",
        "pub capture_enabled: bool",
        "pub external_services_used: bool",
        "pub deployment_allowed: bool",
        "pub native_inference_execution_enabled: bool",
        "pub non_claims: &'static [&'static str]",
    ]
    for field in expected_fields:
        assert field in lib_rs

    expected_anchors = [
        "impl RuntimeSummaryProviderContract",
        "impl RuntimeSummaryProviderPolicy",
        "impl Default for RuntimeSummaryProviderPolicy",
        "RUNTIME_SUMMARY_PROVIDER_NON_CLAIMS",
        "pub fn build_runtime_summary_from_events",
        "fn count_runtime_jobs_by_state",
        "fn runtime_event_label",
        "RuntimeSummaryJobState",
        "runtime_summary_provider.local_only",
        "runtime_summary_provider.caller_provided_events_only",
        "runtime_summary_provider.storage_provider_enabled",
        "runtime_summary_provider.live_runtime_connection_enabled",
        "runtime_summary_provider.file_io_enabled",
        "runtime_summary_provider.process_spawning_enabled",
        "runtime_summary_provider.qt_binding_enabled",
        "runtime_summary_provider.capture_enabled",
        "runtime_summary_provider.external_services_used",
        "runtime_summary_provider.deployment_allowed",
        "runtime_summary_provider.native_inference_execution_enabled",
        "runtime_summary_provider.duplicate_job_id",
        "runtime_summary_provider.unknown_job_id",
        '"not_runtime_service"',
        '"not_persistent_storage"',
        '"not_event_store"',
        '"not_file_loader"',
        '"not_process_spawner"',
        '"not_qt_binding"',
        '"not_capture_boundary"',
        '"not_external_service"',
        '"not_deployment_approval"',
        '"not_native_runtime_execution"',
    ]
    for anchor in expected_anchors:
        assert anchor in lib_rs


def test_rust_core_exposes_runtime_registry_provider_contract_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub accepted_snapshot_schema: &'static str",
        "pub output_snapshot_schema: &'static str",
        "pub max_records: usize",
        "pub in_memory_only: bool",
        "pub accepts_validated_handoff_snapshots_only: bool",
        "pub strict_handoff_validation_enabled: bool",
        "pub upsert_replaces_matching_workspace_session: bool",
        "pub deterministic_snapshot_ordering: bool",
        "pub persistent_storage_enabled: bool",
        "pub database_or_indexing_enabled: bool",
        "pub generated_report_loading_enabled: bool",
        "pub generated_json_loading_enabled: bool",
        "pub file_io_enabled: bool",
        "pub live_transport_enabled: bool",
        "pub public_network_transport_enabled: bool",
        "pub socket_listener_enabled: bool",
        "pub filesystem_socket_path_policy_enabled: bool",
        "pub daemon_lifecycle_enabled: bool",
        "pub process_spawning_enabled: bool",
        "pub file_watching_enabled: bool",
        "pub qt_binding_enabled: bool",
        "pub capture_enabled: bool",
        "pub external_services_used: bool",
        "pub deployment_allowed: bool",
        "pub native_inference_execution_enabled: bool",
        "pub workspace_id: WorkspaceId",
        "pub session_id: SessionId",
        "pub snapshot_schema_version: String",
        "pub snapshot: RuntimeHandoffSnapshot",
        "pub record_count: u32",
        "pub max_record_count: u32",
        "pub records: Vec<RuntimeRegistryRecord>",
    ]
    for field in expected_fields:
        assert field in lib_rs

    expected_anchors = [
        "impl RuntimeRegistryProviderContract",
        "impl RuntimeRegistryProviderPolicy",
        "impl Default for RuntimeRegistryProviderPolicy",
        "impl RuntimeRegistryProvider",
        "impl Default for RuntimeRegistryProvider",
        "RuntimeRegistryProviderContract::synthetic_fixture",
        "RuntimeRegistryProviderPolicy::bounded",
        "pub fn default_provider() -> Self",
        "pub fn upsert_snapshot",
        "pub fn snapshot(&self) -> RuntimeRegistrySnapshot",
        "pub fn len(&self) -> usize",
        "pub fn is_empty(&self) -> bool",
        "BTreeMap<(String, String), RuntimeRegistryRecord>",
        "validate_runtime_handoff_snapshot(&snapshot)",
        "runtime_registry_provider.max_records",
        "runtime_registry_provider.record_cap",
        "runtime_registry_provider.local_only",
        "runtime_registry_provider.in_memory_only",
        "runtime_registry_provider.accepts_validated_handoff_snapshots_only",
        "runtime_registry_provider.strict_handoff_validation_enabled",
        "runtime_registry_provider.persistent_storage_enabled",
        "runtime_registry_provider.database_or_indexing_enabled",
        "runtime_registry_provider.generated_report_loading_enabled",
        "runtime_registry_provider.generated_json_loading_enabled",
        "runtime_registry_provider.file_io_enabled",
        "runtime_registry_provider.live_transport_enabled",
        "runtime_registry_provider.public_network_transport_enabled",
        "runtime_registry_provider.socket_listener_enabled",
        "runtime_registry_provider.filesystem_socket_path_policy_enabled",
        "runtime_registry_provider.daemon_lifecycle_enabled",
        "runtime_registry_provider.process_spawning_enabled",
        "runtime_registry_provider.file_watching_enabled",
        "runtime_registry_provider.qt_binding_enabled",
        "runtime_registry_provider.capture_enabled",
        "runtime_registry_provider.external_services_used",
        "runtime_registry_provider.deployment_allowed",
        "runtime_registry_provider.native_inference_execution_enabled",
        "RUNTIME_REGISTRY_PROVIDER_NON_CLAIMS",
        '"runtime_registry_provider.v0"',
        '"not_persistent_storage"',
        '"not_database_or_indexing_engine"',
        '"not_generated_report_loader"',
        '"not_generated_json_loader"',
        '"not_control_plane_transport"',
        '"not_public_network_transport"',
        '"not_socket_listener"',
        '"not_filesystem_socket_path_policy"',
        '"not_daemon_lifecycle"',
        '"not_process_spawner"',
        '"not_file_watcher"',
        '"not_qt_binding"',
        '"not_capture_boundary"',
        '"not_external_service"',
        '"not_deployment_approval"',
        '"not_native_runtime_execution"',
    ]
    for anchor in expected_anchors:
        assert anchor in lib_rs


def test_rust_core_exposes_runtime_registry_storage_provider_contract_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub accepted_registry_snapshot_schema: &'static str",
        "pub storage_document_schema: &'static str",
        "pub max_file_bytes: u64",
        "pub caller_authorized_allowed_root_required: bool",
        "pub typed_registry_snapshots_only: bool",
        "pub strict_registry_validation_enabled: bool",
        "pub storage_document_json_enabled: bool",
        "pub persistent_storage_enabled: bool",
        "pub database_or_indexing_enabled: bool",
        "pub generated_report_loading_enabled: bool",
        "pub generated_json_loading_enabled: bool",
        "pub arbitrary_file_loading_enabled: bool",
        "pub live_transport_enabled: bool",
        "pub public_network_transport_enabled: bool",
        "pub socket_listener_enabled: bool",
        "pub filesystem_socket_path_policy_enabled: bool",
        "pub daemon_lifecycle_enabled: bool",
        "pub process_spawning_enabled: bool",
        "pub file_watching_enabled: bool",
        "pub qt_binding_enabled: bool",
        "pub capture_enabled: bool",
        "pub external_services_used: bool",
        "pub deployment_allowed: bool",
        "pub native_inference_execution_enabled: bool",
        "pub registry_snapshot_schema: String",
        "pub registry_snapshot: RuntimeRegistrySnapshot",
        "pub file_policy: RuntimeControlPlaneFilePolicy",
    ]
    for field in expected_fields:
        assert field in lib_rs

    expected_anchors = [
        "RuntimeRegistryStorageProviderContract::synthetic_fixture",
        "impl RuntimeRegistryStorageProviderContract",
        "impl RuntimeRegistryStoragePolicy",
        "impl RuntimeRegistryStorageDocument",
        "impl RuntimeRegistryStorageProvider",
        "RUNTIME_REGISTRY_STORAGE_PROVIDER_NON_CLAIMS",
        "RuntimeRegistryStorageDocument::from_snapshot",
        "validate_runtime_registry_storage_json_read_path",
        "validate_runtime_registry_storage_json_write_path",
        "validate_runtime_registry_storage_json_path",
        "validate_runtime_registry_storage_document",
        "validate_runtime_registry_snapshot",
        "validate_runtime_registry_record",
        "serde_json::to_vec_pretty(&document)",
        "persist_runtime_registry_snapshot_file",
        "load_runtime_registry_storage_document_file",
        "load_runtime_registry_snapshot_file",
        "parse_runtime_registry_storage_document_json",
        "runtime_registry_storage_provider.max_file_bytes",
        "runtime_registry_storage_provider.local_only",
        "runtime_registry_storage_provider.caller_authorized_allowed_root_required",
        "runtime_registry_storage_provider.typed_registry_snapshots_only",
        "runtime_registry_storage_provider.strict_registry_validation_enabled",
        "runtime_registry_storage_provider.storage_document_json_enabled",
        "runtime_registry_storage_provider.persistent_storage_enabled",
        "runtime_registry_storage_provider.database_or_indexing_enabled",
        "runtime_registry_storage_provider.generated_report_loading_enabled",
        "runtime_registry_storage_provider.generated_json_loading_enabled",
        "runtime_registry_storage_provider.arbitrary_file_loading_enabled",
        "runtime_registry_storage_provider.public_network_transport_enabled",
        "runtime_registry_storage_provider.socket_listener_enabled",
        "runtime_registry_storage_provider.filesystem_socket_path_policy_enabled",
        "runtime_registry_storage_provider.daemon_lifecycle_enabled",
        "runtime_registry_storage_provider.process_spawning_enabled",
        "runtime_registry_storage_provider.file_watching_enabled",
        "runtime_registry_storage_provider.qt_binding_enabled",
        "runtime_registry_storage_provider.capture_enabled",
        "runtime_registry_storage_provider.external_services_used",
        "runtime_registry_storage_provider.deployment_allowed",
        "runtime_registry_storage_provider.native_inference_execution_enabled",
        "runtime_registry_snapshot.record_count",
        "runtime_registry_snapshot.max_record_count",
        "runtime_registry_snapshot.records",
        "runtime_registry_snapshot.records.workspace_id",
        "runtime_registry_snapshot.records.session_id",
        '"runtime_registry_storage_provider.v0"',
        '"not_database_or_indexing_engine"',
        '"not_generated_report_loader"',
        '"not_generated_json_loader"',
        '"not_arbitrary_file_loader"',
        '"not_control_plane_transport"',
        '"not_public_network_transport"',
        '"not_socket_listener"',
        '"not_filesystem_socket_path_policy"',
        '"not_daemon_lifecycle"',
        '"not_process_spawner"',
        '"not_file_watcher"',
        '"not_qt_binding"',
        '"not_capture_boundary"',
        '"not_external_service"',
        '"not_deployment_approval"',
        '"not_native_runtime_execution"',
    ]
    for anchor in expected_anchors:
        assert anchor in lib_rs


def test_rust_core_exposes_model_registry_metadata_contract_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub schema_version: String",
        "pub metadata_scope: String",
        "pub source_bundle_schema: String",
        "pub entries: Vec<ModelRegistryEntry>",
        "pub aggregate_summary: ModelRegistryAggregateSummary",
        "pub safety_flags: ModelRegistrySafetyFlags",
        "pub non_claims: Vec<String>",
        "pub model_id: String",
        "pub registry_state: ModelRegistryState",
        "pub promotion_state: ModelPromotionState",
        "pub observed_source_schemas: Vec<String>",
        "pub observed_source_names: Vec<String>",
        "pub source_count: u32",
        "pub has_score_rows: bool",
        "pub human_review_required: bool",
        "pub deployment_allowed: bool",
        "pub model_count: u32",
        "pub schemas_present: Vec<String>",
        "pub models_with_score_rows: Vec<String>",
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
    assert "fn model_registry_metadata_entries()" in lib_rs


def test_rust_core_exposes_model_registry_metadata_adapter_contract_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub accepted_metadata_schema: &'static str",
        "pub source_bundle_schema: &'static str",
        "pub max_file_bytes: u64",
        "pub local_only: bool",
        "pub synthetic_metadata_only: bool",
        "pub strict_json_parsing_enabled: bool",
        "pub file_io_enabled: bool",
        "pub storage_provider_enabled: bool",
        "pub generated_report_loading_enabled: bool",
        "pub qt_binding_enabled: bool",
        "pub capture_enabled: bool",
        "pub external_services_used: bool",
        "pub deployment_allowed: bool",
        "pub native_inference_execution_enabled: bool",
        "pub non_claims: &'static [&'static str]",
        "pub file_policy: RuntimeControlPlaneFilePolicy",
    ]
    for field in expected_fields:
        assert field in lib_rs

    expected_anchors = [
        "impl ModelRegistryMetadataAdapterContract",
        "impl ModelRegistryMetadataAdapterPolicy",
        "MODEL_REGISTRY_METADATA_ADAPTER_NON_CLAIMS",
        "pub fn parse_model_registry_metadata_json",
        "pub fn parse_model_registry_metadata_file",
        "validate_runtime_control_plane_json_file_path",
        "validate_model_registry_metadata(&metadata)",
        "model_registry_metadata_adapter.storage_provider_enabled",
        "model_registry_metadata_adapter.generated_report_loading_enabled",
        "model_registry_metadata_adapter.qt_binding_enabled",
        "model_registry_metadata_adapter.deployment_allowed",
        "model_registry_metadata_adapter.native_inference_execution_enabled",
        '"not_persistent_model_registry"',
        '"not_storage_provider"',
        '"not_generated_report_loader"',
        '"not_qt_binding"',
        '"not_deployment_approval"',
        '"not_external_service"',
        '"not_native_runtime_execution"',
    ]
    for anchor in expected_anchors:
        assert anchor in lib_rs


def test_rust_core_exposes_runtime_handoff_snapshot_contract_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub schema_version: String",
        "pub source_kind: RuntimeHandoffSourceKind",
        "pub transport_state: RuntimeHandoffTransportState",
        "pub control_plane_state: RuntimeControlPlaneState",
        "pub runtime_summary: RuntimeSummary",
        "pub model_registry_metadata: ModelRegistryMetadata",
        "pub local_only: bool",
        "pub static_synthetic_fixture: bool",
        "pub generated_json_loaded: bool",
        "pub live_runtime_connection: bool",
        "pub external_services_used: bool",
        "pub deployment_allowed: bool",
        "pub non_claims: Vec<String>",
    ]
    for field in expected_fields:
        assert field in lib_rs

    assert "impl RuntimeHandoffSnapshot" in lib_rs
    assert "impl RuntimeHandoffSourceKind" in lib_rs
    assert "impl RuntimeHandoffTransportState" in lib_rs
    assert "impl RuntimeControlPlaneState" in lib_rs
    assert "RuntimeSummary::synthetic_fixture()" in lib_rs
    assert "ModelRegistryMetadata::synthetic_fixture()" in lib_rs
    assert "RUNTIME_HANDOFF_NON_CLAIMS" in lib_rs


def test_rust_core_exposes_control_plane_adapter_contract_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub schema_version: &'static str",
        "pub adapter_kind: RuntimeControlPlaneAdapterKind",
        "pub input_mode: RuntimeControlPlaneInputMode",
        "pub adapter_state: RuntimeControlPlaneAdapterState",
        "pub output_snapshot_schema: RuntimeControlPlaneOutputSnapshotSchema",
        "pub accepted_input_schemas: &'static [&'static str]",
        "pub local_only: bool",
        "pub dependency_free: bool",
        "pub static_synthetic_fixture: bool",
        "pub json_parsing_enabled: bool",
        "pub file_io_enabled: bool",
        "pub live_transport_enabled: bool",
        "pub qt_binding_enabled: bool",
        "pub external_services_used: bool",
        "pub deployment_allowed: bool",
        "pub non_claims: &'static [&'static str]",
    ]
    for field in expected_fields:
        assert field in lib_rs

    assert "impl RuntimeControlPlaneAdapterContract" in lib_rs
    assert "impl RuntimeControlPlaneAdapterKind" in lib_rs
    assert "impl RuntimeControlPlaneInputMode" in lib_rs
    assert "impl RuntimeControlPlaneAdapterState" in lib_rs
    assert "impl RuntimeControlPlaneOutputSnapshotSchema" in lib_rs
    assert "RuntimeControlPlaneAdapterError::InvalidJson" in lib_rs
    assert "RuntimeControlPlaneAdapterError::NonObjectRoot" in lib_rs
    assert "RuntimeControlPlaneAdapterError::RelativeFilePath" in lib_rs
    assert "RuntimeControlPlaneAdapterError::RelativeAllowedRoot" in lib_rs
    assert "RuntimeControlPlaneAdapterError::MissingFile" in lib_rs
    assert "RuntimeControlPlaneAdapterError::MissingAllowedRoot" in lib_rs
    assert "RuntimeControlPlaneAdapterError::AllowedRootSymlink" in lib_rs
    assert "RuntimeControlPlaneAdapterError::AllowedRootNotDirectory" in lib_rs
    assert "RuntimeControlPlaneAdapterError::SymlinkPath" in lib_rs
    assert "RuntimeControlPlaneAdapterError::DirectoryPath" in lib_rs
    assert "RuntimeControlPlaneAdapterError::NonRegularFile" in lib_rs
    assert "RuntimeControlPlaneAdapterError::UnsupportedFileExtension" in lib_rs
    assert "RuntimeControlPlaneAdapterError::OutsideAllowedRoot" in lib_rs
    assert "RuntimeControlPlaneAdapterError::OversizedFile" in lib_rs
    assert "RuntimeControlPlaneAdapterError::FileReadFailed" in lib_rs
    assert "RuntimeControlPlaneAdapterError::FileWriteFailed" in lib_rs
    assert "RuntimeControlPlaneAdapterError::InvalidUtf8" in lib_rs
    assert "RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion" in lib_rs
    assert "RuntimeControlPlaneAdapterError::UnsafeFlag" in lib_rs
    assert "RuntimeControlPlaneAdapterError::UnsupportedValue" in lib_rs
    assert "RUNTIME_CONTROL_PLANE_ADAPTER_ACCEPTED_SCHEMAS" in lib_rs
    assert "RUNTIME_CONTROL_PLANE_ADAPTER_NON_CLAIMS" in lib_rs
    assert "pub struct RuntimeControlPlaneFilePolicy" in lib_rs
    assert "pub allowed_root: PathBuf" in lib_rs
    assert "pub fn new(allowed_root: impl Into<PathBuf>) -> Self" in lib_rs
    assert "pub fn max_bytes(&self) -> u64" in lib_rs
    assert "pub struct RuntimeControlPlaneFramePolicy" in lib_rs
    assert "pub max_frame_bytes: usize" in lib_rs
    assert "pub struct RuntimeControlPlaneIpcPolicy" in lib_rs
    assert "pub frame_policy: RuntimeControlPlaneFramePolicy" in lib_rs
    assert "pub struct RuntimeControlPlaneFrameAdapterContract" in lib_rs
    assert "pub payload_schema_version: &'static str" in lib_rs
    assert "pub caller_provided_bytes_only: bool" in lib_rs
    assert "pub utf8_json_payload_required: bool" in lib_rs
    assert "pub additional_dependencies_required: bool" in lib_rs
    assert "pub socket_listener_enabled: bool" in lib_rs
    assert "pub daemon_lifecycle_enabled: bool" in lib_rs
    assert "pub process_spawning_enabled: bool" in lib_rs
    assert "pub file_watching_enabled: bool" in lib_rs
    assert "pub storage_provider_enabled: bool" in lib_rs
    assert "pub capture_enabled: bool" in lib_rs
    assert "pub native_inference_execution_enabled: bool" in lib_rs
    assert "pub struct RuntimeControlPlaneIpcAdapterContract" in lib_rs
    assert "pub struct RuntimeControlPlaneEndpointPolicy" in lib_rs
    assert "pub endpoint_kind: RuntimeControlPlaneEndpointKind" in lib_rs
    assert "pub ipc_policy: RuntimeControlPlaneIpcPolicy" in lib_rs
    assert "pub public_network_transport_enabled: bool" in lib_rs
    assert "pub struct RuntimeControlPlaneEndpointAdapterContract" in lib_rs
    assert "pub ipc_schema_version: &'static str" in lib_rs
    assert "pub endpoint_policy_validation_enabled: bool" in lib_rs
    assert "pub connected_stream_execution_enabled: bool" in lib_rs
    assert "impl RuntimeControlPlaneEndpointKind" in lib_rs
    assert "impl RuntimeControlPlaneEndpointAdapterContract" in lib_rs
    assert "impl RuntimeControlPlaneEndpointPolicy" in lib_rs
    assert "impl Default for RuntimeControlPlaneEndpointPolicy" in lib_rs
    assert "pub frame_schema_version: &'static str" in lib_rs
    assert "pub message_schema_version: &'static str" in lib_rs
    assert "pub length_prefix_bytes: usize" in lib_rs
    assert "pub caller_provided_streams_only: bool" in lib_rs
    assert "pub one_shot_request_response: bool" in lib_rs
    assert "pub big_endian_length_prefix_required: bool" in lib_rs
    assert "pub stream_io_enabled: bool" in lib_rs
    assert "pub filesystem_socket_path_policy_enabled: bool" in lib_rs
    assert "impl RuntimeControlPlaneIpcAdapterContract" in lib_rs
    assert "impl RuntimeControlPlaneFrameAdapterContract" in lib_rs
    assert "impl RuntimeControlPlaneFramePolicy" in lib_rs
    assert "impl RuntimeControlPlaneIpcPolicy" in lib_rs
    assert "impl Default for RuntimeControlPlaneFramePolicy" in lib_rs
    assert "#[derive(Clone, Debug, Default, Eq, PartialEq)]" in lib_rs
    assert (
        "pub fn new(max_frame_bytes: usize) -> "
        "Result<Self, RuntimeControlPlaneAdapterError>" in lib_rs
    )
    assert "pub fn max_bytes(&self) -> usize" in lib_rs
    assert "fn validate_control_plane_frame_bytes" in lib_rs
    assert "pub enum RuntimeControlPlaneCommand" in lib_rs
    assert "ParseHandoffSnapshotJson {" in lib_rs
    assert "input: String" in lib_rs
    assert "ParseHandoffSnapshotFile {" in lib_rs
    assert "policy: RuntimeControlPlaneFilePolicy" in lib_rs
    assert "pub fn execute_local_command(" in lib_rs
    assert "pub fn parse_control_plane_message_frame_bytes(" in lib_rs
    assert "pub fn execute_control_plane_message_frame_bytes(" in lib_rs
    assert "pub fn serialize_control_plane_message_response_frame_bytes(" in lib_rs
    assert "pub fn read_control_plane_message_ipc_frame" in lib_rs
    assert "pub fn write_control_plane_message_ipc_frame" in lib_rs
    assert "pub fn execute_control_plane_message_ipc_stream" in lib_rs
    assert "pub fn execute_control_plane_endpoint_stream" in lib_rs
    assert "fn read_exact_control_plane_ipc" in lib_rs
    assert "fn validate_control_plane_endpoint_policy" in lib_rs
    assert "RuntimeControlPlaneCommand::ParseHandoffSnapshotJson" in lib_rs
    assert "RuntimeControlPlaneCommand::ParseHandoffSnapshotFile" in lib_rs
    assert "pub fn command_kind(&self) -> &'static str" in lib_rs
    assert '=> "parse_handoff_snapshot_json"' in lib_rs
    assert '=> "parse_handoff_snapshot_file"' in lib_rs
    assert "!file_metadata.file_type().is_file()" in lib_rs


def test_rust_core_exposes_control_plane_message_envelope_shape() -> None:
    lib_rs = _read("src/lib.rs")

    expected_fields = [
        "pub struct RuntimeControlPlaneRequestId",
        "pub struct RuntimeControlPlaneMessageRequest",
        "pub schema_version: String",
        "pub request_id: RuntimeControlPlaneRequestId",
        "pub command: RuntimeControlPlaneCommand",
        "pub struct RuntimeControlPlaneMessageResponse",
        "pub outcome: RuntimeControlPlaneMessageOutcome",
        "pub snapshot: Option<RuntimeHandoffSnapshot>",
        "pub error_code: Option<RuntimeControlPlaneMessageErrorCode>",
        "pub enum RuntimeControlPlaneMessageOutcome",
        "Success",
        "Failure",
        "pub enum RuntimeControlPlaneMessageErrorCode",
        "UnsupportedSchemaVersion",
        "UnsupportedValue",
        "UnsafeFlag",
        "RawRuntimeControlPlaneMessageRequest",
        "RawRuntimeControlPlaneMessageCommand",
        "command_kind: String",
        "input: Option<String>",
        "path: Option<PathBuf>",
        "policy: Option<RuntimeControlPlaneFilePolicy>",
    ]
    for field in expected_fields:
        assert field in lib_rs

    expected_helpers = [
        "impl RuntimeControlPlaneRequestId",
        "pub fn new(value: impl Into<String>) -> Result<Self, RuntimeControlPlaneAdapterError>",
        "pub fn as_str(&self) -> &str",
        "impl RuntimeControlPlaneMessageRequest",
        "pub fn new(",
        "impl RuntimeControlPlaneMessageResponse",
        "pub fn success(",
        "pub fn failure(",
        "impl RuntimeControlPlaneMessageOutcome",
        "impl RuntimeControlPlaneMessageErrorCode",
        "impl From<&RuntimeControlPlaneAdapterError> for RuntimeControlPlaneMessageErrorCode",
        "parse_control_plane_message_request_json",
        "execute_control_plane_message_request",
        "execute_control_plane_message_json",
        "serialize_control_plane_message_response_json",
        "parse_runtime_control_plane_message_command",
        "validate_control_plane_request_id",
    ]
    for helper in expected_helpers:
        assert helper in lib_rs

    expected_values = [
        '"runtime_control_plane_message.v0"',
        '"success"',
        '"failure"',
        '"invalid_json"',
        '"non_object_root"',
        '"file_write_failed"',
        '"oversized_frame"',
        '"ipc_read_failed"',
        '"ipc_write_failed"',
        '"malformed_ipc_frame"',
        '"incomplete_ipc_frame"',
        '"unsupported_schema_version"',
        '"unsupported_value"',
        '"unsafe_flag"',
        '"parse_handoff_snapshot_json"',
        '"parse_handoff_snapshot_file"',
        '"command.command_kind"',
        '"request_id"',
    ]
    for value in expected_values:
        assert value in lib_rs

    assert '#[serde(skip_serializing_if = "Option::is_none")]' in lib_rs


def test_rust_core_static_handoff_fixture_composes_existing_contracts() -> None:
    lib_rs = _read("src/lib.rs")

    expected_values = [
        '"runtime_handoff_snapshot.v0"',
        '"runtime_summary.v0"',
        '"model_registry_metadata.v0"',
        '"static_synthetic_fixture"',
        '"unavailable"',
        '"not_live_runtime_connection"',
        '"not_generated_json_loader"',
        '"not_control_plane_transport"',
        '"not_persistent_storage"',
        '"not_qt_runtime_integration"',
        '"not_model_promotion_gate"',
        '"not_deployment_approval"',
        '"not_native_runtime_execution"',
    ]
    for value in expected_values:
        assert value in lib_rs

    assert "source_kind: RuntimeHandoffSourceKind::StaticSyntheticFixture" in lib_rs
    assert "transport_state: RuntimeHandoffTransportState::Unavailable" in lib_rs
    assert "control_plane_state: RuntimeControlPlaneState::Unavailable" in lib_rs
    assert "runtime_summary: RuntimeSummary::synthetic_fixture()" in lib_rs
    assert "model_registry_metadata: ModelRegistryMetadata::synthetic_fixture()" in lib_rs
    assert "local_only: true" in lib_rs
    assert "static_synthetic_fixture: true" in lib_rs
    assert "generated_json_loaded: false" in lib_rs
    assert "live_runtime_connection: false" in lib_rs
    assert "external_services_used: false" in lib_rs
    assert "deployment_allowed: false" in lib_rs


def test_rust_core_static_control_plane_adapter_fixture_declares_only_local_contract() -> None:
    lib_rs = _read("src/lib.rs")

    expected_values = [
        '"runtime_control_plane_adapter.v0"',
        '"runtime_control_plane_endpoint.v0"',
        '"runtime_control_plane_ipc.v0"',
        '"runtime_control_plane_frame.v0"',
        '"runtime_control_plane_message.v0"',
        '"runtime_handoff_snapshot.v0"',
        '"runtime_summary.v0"',
        '"model_registry_metadata.v0"',
        '"local_control_plane_endpoint_policy"',
        '"accepted_local_endpoint_policy"',
        '"local_endpoint_policy_available"',
        '"not_arbitrary_file_loader"',
        '"not_file_watcher"',
        '"not_live_transport"',
        '"not_socket_listener"',
        '"not_daemon_lifecycle"',
        '"not_filesystem_socket_path_policy"',
        '"not_process_spawner"',
        '"not_qt_binding"',
        '"not_external_service"',
        '"not_deployment_approval"',
        '"not_runtime_service"',
        '"not_generated_report_loader"',
    ]
    for value in expected_values:
        assert value in lib_rs

    assert (
        "adapter_kind: RuntimeControlPlaneAdapterKind::"
        "LocalControlPlaneEndpointPolicy" in " ".join(lib_rs.split())
    )
    assert "input_mode: RuntimeControlPlaneInputMode::AcceptedLocalEndpointPolicy" in " ".join(
        lib_rs.split()
    )
    assert (
        "adapter_state: RuntimeControlPlaneAdapterState::LocalEndpointPolicyAvailable"
        in " ".join(lib_rs.split())
    )
    assert (
        "output_snapshot_schema: RuntimeControlPlaneOutputSnapshotSchema::"
        "RuntimeHandoffSnapshotV0" in " ".join(lib_rs.split())
    )
    assert "accepted_input_schemas: RUNTIME_CONTROL_PLANE_ADAPTER_ACCEPTED_SCHEMAS" in lib_rs
    assert "local_only: true" in lib_rs
    assert "dependency_free: false" in lib_rs
    assert "static_synthetic_fixture: true" in lib_rs
    assert "json_parsing_enabled: true" in lib_rs
    assert "file_io_enabled: true" in lib_rs
    assert "live_transport_enabled: false" in lib_rs
    assert "qt_binding_enabled: false" in lib_rs
    assert "external_services_used: false" in lib_rs
    assert "deployment_allowed: false" in lib_rs

    accepted_schema_block = re.search(
        r"RUNTIME_CONTROL_PLANE_ADAPTER_ACCEPTED_SCHEMAS: &\[&str\] = &\[(.*?)\];",
        lib_rs,
        flags=re.DOTALL,
    )
    assert accepted_schema_block is not None
    assert [
        schema.strip() for schema in accepted_schema_block.group(1).split(",") if schema.strip()
    ] == [
        "RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION",
        "RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION",
        "RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION",
        "RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION",
        "RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION",
        "RUNTIME_SUMMARY_SCHEMA_VERSION",
        "MODEL_REGISTRY_METADATA_SCHEMA_VERSION",
    ]

    ipc_expected_values = [
        "RuntimeControlPlaneIpcAdapterContract",
        "RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES",
        "caller_provided_streams_only: true",
        "one_shot_request_response: true",
        "big_endian_length_prefix_required: true",
        "stream_io_enabled: true",
        "socket_listener_enabled: false",
        "filesystem_socket_path_policy_enabled: false",
        "daemon_lifecycle_enabled: false",
        "process_spawning_enabled: false",
        "file_watching_enabled: false",
        "qt_binding_enabled: false",
        "storage_provider_enabled: false",
        "capture_enabled: false",
        "external_services_used: false",
        "deployment_allowed: false",
        "native_inference_execution_enabled: false",
        '"not_public_network_transport"',
        '"not_socket_listener"',
        '"not_daemon_lifecycle"',
        '"not_filesystem_socket_path_policy"',
        '"not_process_spawner"',
        '"not_file_watcher"',
        '"not_storage_provider"',
        '"not_capture_boundary"',
        '"not_external_service"',
    ]
    for value in ipc_expected_values:
        assert value in lib_rs

    endpoint_expected_values = [
        "RuntimeControlPlaneEndpointPolicy",
        "RuntimeControlPlaneEndpointAdapterContract",
        "RuntimeControlPlaneEndpointKind",
        "CallerProvidedConnectedStream",
        "caller_provided_connected_stream",
        "endpoint_kind: RuntimeControlPlaneEndpointKind::CallerProvidedConnectedStream",
        "endpoint_policy_validation_enabled: true",
        "connected_stream_execution_enabled: true",
        "public_network_transport_enabled: false",
        "socket_listener_enabled: false",
        "filesystem_socket_path_policy_enabled: false",
        "daemon_lifecycle_enabled: false",
        "process_spawning_enabled: false",
        "file_watching_enabled: false",
        "qt_binding_enabled: false",
        "storage_provider_enabled: false",
        "capture_enabled: false",
        "external_services_used: false",
        "deployment_allowed: false",
        "native_inference_execution_enabled: false",
        "execute_control_plane_endpoint_stream",
        "validate_control_plane_endpoint_policy",
        '"not_public_network_transport"',
        '"not_filesystem_socket_path_policy"',
        '"not_native_runtime_execution"',
    ]
    for value in endpoint_expected_values:
        assert value in lib_rs


def test_rust_core_static_registry_fixture_matches_validated_metadata_snapshot() -> None:
    lib_rs = _read("src/lib.rs")

    expected_values = [
        '"model_registry_metadata.v0"',
        '"local_synthetic_model_registry_metadata"',
        '"model_evaluation_bundle.v0"',
        '"observed_synthetic_only"',
        '"not_promoted"',
        '"graph_novelty"',
        '"isolation_forest"',
        '"model_disagreement"',
        '"pyod_copod"',
        '"pyod_ecod"',
        '"river_hst"',
        '"self_supervised_representation"',
        '"stdlib_linear_native"',
        '"suricata_alert"',
        '"time_series_residual"',
        '"agentic_investigation_report.v0"',
        '"detection_candidate_report.v0"',
        '"model_disagreement_report.v0"',
        '"model_score_rows.v0"',
        '"temporal_security_graph_report.v0"',
        '"time_series_residual_report.v0"',
        '"traffic_representation_report.v0"',
        '"agentic_investigation_report_v0_001"',
        '"detection_candidate_report_v0_001"',
        '"model_disagreement_report_v0_001"',
        '"model_score_rows_v0_001"',
        '"temporal_security_graph_report_v0_001"',
        '"time_series_residual_report_v0_001"',
        '"traffic_representation_report_v0_001"',
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

    assert "model_count: 10" in lib_rs
    assert "source_count: 4" in lib_rs
    assert "source_count: 3" in lib_rs
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

    static_entries_block = lib_rs.split("fn model_registry_metadata_entries()", 1)[1].split(
        "#[cfg(test)]", 1
    )[0]
    entry_model_ids = re.findall(r'model_id: "([^"]+)"', static_entries_block)
    assert entry_model_ids == [
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
        "tcpstream",
        "udp",
        "std::net",
        "file::open",
        "read_to_string",
        "from_reader",
        "std::process",
        "command::new",
        "model artifact",
        "private telemetry",
        "pcap",
        "packet capture",
        "http",
        "https",
        "url",
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

    rust_without_allowed_io_import = rust_source.replace("use std::io::{Read, Write};", "")
    assert "std::io::" not in rust_without_allowed_io_import.lower()
    assert "serde" in lowered
    assert "serde_json::from_str" in rust_source
    assert "serde_json::from_value" not in rust_source
    assert "std::fs" in rust_source
    assert "use std::io::{Read, Write};" in rust_source
    assert "std::path::{Path, PathBuf}" in rust_source
    assert "RuntimeControlPlaneFilePolicy" in rust_source
    assert "RuntimeSummaryProviderContract" in rust_source
    assert "RuntimeSummaryProviderPolicy" in rust_source
    assert "RuntimeRegistryProviderContract" in rust_source
    assert "RuntimeRegistryProviderPolicy" in rust_source
    assert "RuntimeRegistryRecord" in rust_source
    assert "RuntimeRegistrySnapshot" in rust_source
    assert "RuntimeRegistryProvider" in rust_source
    assert "RuntimeRegistryStorageProviderContract" in rust_source
    assert "RuntimeRegistryStoragePolicy" in rust_source
    assert "RuntimeRegistryStorageDocument" in rust_source
    assert "RuntimeRegistryStorageProvider" in rust_source
    assert "RuntimeControlPlaneFramePolicy" in rust_source
    assert "RuntimeControlPlaneIpcPolicy" in rust_source
    assert "RuntimeControlPlaneEndpointPolicy" in rust_source
    assert "ModelRegistryMetadataAdapterContract" in rust_source
    assert "ModelRegistryMetadataAdapterPolicy" in rust_source
    assert "RuntimeControlPlaneFrameAdapterContract" in rust_source
    assert "RuntimeControlPlaneIpcAdapterContract" in rust_source
    assert "RuntimeControlPlaneEndpointAdapterContract" in rust_source
    assert "RuntimeControlPlaneCommand" in rust_source
    assert "RuntimeControlPlaneMessageRequest" in rust_source
    assert "RuntimeControlPlaneMessageResponse" in rust_source
    assert "RuntimeControlPlaneMessageErrorCode" in rust_source
    assert "parse_control_plane_message_frame_bytes" in rust_source
    assert "execute_control_plane_message_frame_bytes" in rust_source
    assert "serialize_control_plane_message_response_frame_bytes" in rust_source
    assert "read_control_plane_message_ipc_frame" in rust_source
    assert "write_control_plane_message_ipc_frame" in rust_source
    assert "execute_control_plane_message_ipc_stream" in rust_source
    assert "execute_control_plane_endpoint_stream" in rust_source
    assert "validate_control_plane_endpoint_policy" in rust_source
    assert "parse_handoff_snapshot_file" in rust_source
    assert "build_runtime_summary_from_events" in rust_source
    assert "parse_model_registry_metadata_json" in rust_source
    assert "parse_model_registry_metadata_file" in rust_source
    assert "execute_local_command" in rust_source
    assert "parse_control_plane_message_request_json" in rust_source
    assert "execute_control_plane_message_json" in rust_source
    assert "serialize_control_plane_message_response_json" in rust_source
    assert "RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES" in rust_source
    assert "RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES" in rust_source
    assert "RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION" in rust_source
    assert "RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION" in rust_source
    assert "RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION" in rust_source
    assert "RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES" in rust_source
    assert "RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION" in rust_source
    assert re.search(r"\b(?:\d{1,3}\.){3}\d{1,3}\b", rust_source) is None
    assert re.search(r"\b[A-Za-z0-9.-]+\.(?:com|net|org|io)\b", rust_source) is None


def test_makefile_exposes_separate_rust_validation_target() -> None:
    makefile = MAKEFILE.read_text(encoding="utf-8")

    assert "verify-rust-core" in makefile
    assert "$(CARGO) fmt --check" in makefile
    assert "$(CARGO) test" in makefile
    assert "$(CARGO) clippy -- -D warnings" in makefile


def test_runtime_strategy_documents_v0_limits_and_migration() -> None:
    strategy = STRATEGY_DOC.read_text(encoding="utf-8")
    normalized_strategy = " ".join(strategy.split())

    expected_text = [
        "source-only contract",
        "Rust package boundary",
        "runtime_summary.v0",
        "RuntimeSummary",
        "NativeInferenceRuntimeState",
        "runtime_summary_provider.v0",
        "RuntimeSummaryProviderContract",
        "RuntimeSummaryProviderPolicy",
        "RuntimeEvent",
        "build_runtime_summary_from_events",
        "model_registry_metadata.v0",
        "ModelRegistryMetadata",
        "ModelRegistryEntry",
        "ModelRegistryAggregateSummary",
        "ModelRegistrySafetyFlags",
        "model_registry_metadata_adapter.v0",
        "ModelRegistryMetadataAdapterContract",
        "ModelRegistryMetadataAdapterPolicy",
        "parse_model_registry_metadata_json",
        "parse_model_registry_metadata_file",
        "runtime_handoff_snapshot.v0",
        "RuntimeHandoffSnapshot",
        "runtime_registry_provider.v0",
        "RuntimeRegistryProviderContract",
        "RuntimeRegistryProviderPolicy",
        "RuntimeRegistryRecord",
        "RuntimeRegistrySnapshot",
        "RuntimeRegistryProvider",
        "runtime_registry_storage_provider.v0",
        "RuntimeRegistryStorageProviderContract",
        "RuntimeRegistryStoragePolicy",
        "RuntimeRegistryStorageDocument",
        "RuntimeRegistryStorageProvider",
        "load_runtime_registry_snapshot_file",
        "runtime_control_plane_adapter.v0",
        "RuntimeControlPlaneAdapterContract",
        "RuntimeControlPlaneAdapterKind",
        "RuntimeControlPlaneInputMode",
        "RuntimeControlPlaneAdapterState",
        "RuntimeControlPlaneOutputSnapshotSchema",
        "runtime_control_plane_frame.v0",
        "RuntimeControlPlaneFramePolicy",
        "RuntimeControlPlaneFrameAdapterContract",
        "parse_control_plane_message_frame_bytes",
        "execute_control_plane_message_frame_bytes",
        "serialize_control_plane_message_response_frame_bytes",
        "runtime_control_plane_endpoint.v0",
        "RuntimeControlPlaneEndpointPolicy",
        "RuntimeControlPlaneEndpointAdapterContract",
        "RuntimeControlPlaneEndpointKind",
        "execute_control_plane_endpoint_stream",
        "validate_control_plane_endpoint_policy",
        "runtime_control_plane_ipc.v0",
        "RuntimeControlPlaneIpcPolicy",
        "RuntimeControlPlaneIpcAdapterContract",
        "RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES",
        "read_control_plane_message_ipc_frame",
        "write_control_plane_message_ipc_frame",
        "execute_control_plane_message_ipc_stream",
        "RuntimeControlPlaneFilePolicy",
        "RuntimeControlPlaneCommand",
        "runtime_control_plane_message.v0",
        "RuntimeControlPlaneMessageRequest",
        "RuntimeControlPlaneRequestId",
        "RuntimeControlPlaneMessageResponse",
        "RuntimeControlPlaneMessageOutcome",
        "RuntimeControlPlaneMessageErrorCode",
        "static runtime_summary.v0 handoff",
        "runtime_summary_provider.v0 over caller-provided local RuntimeEvent slices",
        "first real Rust runtime summary provider",
        "rejects empty event streams",
        "duplicate job queue events",
        "job state changes for unknown jobs",
        "not an event store",
        "static model_registry_metadata.v0 handoff",
        "static runtime_handoff_snapshot.v0 handoff",
        "static runtime_control_plane_adapter.v0 contract",
        "accepted local endpoint, IPC, frame, message, and handoff schemas",
        "JSON-string parsing is now enabled through `serde` and `serde_json`",
        "bounded local file adapter is now enabled",
        "explicitly supplied synthetic metadata JSON",
        "storage, indexing, generated report loading, model promotion",
        "typed local command dispatcher",
        "RuntimeControlPlaneCommand",
        "execute_local_command",
        "ParseHandoffSnapshotJson",
        "ParseHandoffSnapshotFile",
        "parse_handoff_snapshot_file",
        "strict local `runtime_control_plane_message.v0` request/response message envelope",
        "caller-supplied `RuntimeControlPlaneRequestId`",
        "rejects malformed JSON, unknown fields, unsupported message schema versions",
        "unsupported command variants",
        "mixed command fields",
        "RuntimeControlPlaneAdapterContract::serialize_control_plane_message_response_json",
        "bounded local byte-frame adapter",
        "caps frames at 256 KiB by default",
        "requires UTF-8 JSON payloads",
        "caller-provided `&[u8]` frames",
        "typed failure responses with `RuntimeControlPlaneMessageErrorCode`",
        "Frame parsing failures without a valid request identifier return adapter errors",
        "absolute `.json` path",
        "canonical allowed root",
        "256 KiB",
        (
            "rejects symlinks, directories, non-regular files, missing files, "
            "non-JSON paths, oversized files, and invalid UTF-8"
        ),
        "validates sorted Python-derived synthetic registry entries and derived aggregate metadata",
        "bounded in-memory `runtime_registry_provider.v0`",
        "already validated `runtime_handoff_snapshot.v0` values",
        "re-runs strict handoff validation",
        "caps the default registry at 64 records",
        "sorted by workspace/session key",
        "bounded local `runtime_registry_storage_provider.v0`",
        "writes UTF-8 JSON under a caller-provided absolute allowed root",
        "reloads the same typed snapshot through `load_runtime_registry_snapshot_file`",
        "oversized files above 1 MiB",
        "not a database or indexing engine",
        "not a generated report loader",
        "not arbitrary file loading",
        "not a control-plane transport",
        "Indexed storage, migrations, storage compaction, OS-local listener/path binding",
        "real Rust runtime summary provider",
        "typed registry metadata adapter",
        "typed model_registry_metadata_adapter.v0 over supplied metadata JSON/files",
        "typed local control-plane command dispatcher over JSON/file parsers",
        "strict runtime_control_plane_message.v0 local request/response envelope",
        "bounded runtime_control_plane_frame.v0 local byte-frame adapter",
        "bounded runtime_control_plane_ipc.v0 connected-stream adapter",
        "bounded runtime_control_plane_endpoint.v0 endpoint policy",
        "bounded in-memory runtime_registry_provider.v0 over validated handoff snapshots",
        "bounded runtime_registry_storage_provider.v0 local JSON persistence",
        "future OS-local listener/path binding implementation",
        "does not implement a daemon",
        "not a database or indexing engine",
        "not a generated JSON loader",
        "not arbitrary file loading",
        "not file watching",
        "no socket listener",
        "no filesystem socket path policy",
        "does not require Rust tooling for `make verify`",
        "make verify-rust-core",
        "Qt workstation data-flow integration",
        "Python ML Lab report handoff",
        "cargo fmt --check",
        "cargo test",
        "cargo clippy",
    ]
    for text in expected_text:
        assert text in normalized_strategy
