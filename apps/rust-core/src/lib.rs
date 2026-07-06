use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const RUNTIME_CONTRACT_VERSION: &str = "rust_runtime_contract.v0";
pub const RUNTIME_SUMMARY_SCHEMA_VERSION: &str = "runtime_summary.v0";
pub const RUNTIME_SUMMARY_PROVIDER_SCHEMA_VERSION: &str = "runtime_summary_provider.v0";
pub const RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION: &str = "runtime_registry_provider.v0";
pub const RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION: &str =
    "runtime_registry_storage_provider.v0";
pub const RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP: usize = 64;
pub const RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES: u64 = 1024 * 1024;
pub const MODEL_REGISTRY_METADATA_SCHEMA_VERSION: &str = "model_registry_metadata.v0";
pub const MODEL_REGISTRY_METADATA_ADAPTER_SCHEMA_VERSION: &str =
    "model_registry_metadata_adapter.v0";
pub const MODEL_REGISTRY_METADATA_SCOPE: &str = "local_synthetic_model_registry_metadata";
pub const MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION: &str = "model_evaluation_bundle.v0";
pub const RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION: &str = "runtime_handoff_snapshot.v0";
pub const RUNTIME_CONTROL_PLANE_ADAPTER_SCHEMA_VERSION: &str = "runtime_control_plane_adapter.v0";
pub const RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION: &str = "runtime_control_plane_endpoint.v0";
pub const RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION: &str = "runtime_control_plane_frame.v0";
pub const RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION: &str = "runtime_control_plane_ipc.v0";
pub const RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION: &str = "runtime_control_plane_message.v0";
pub const RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES: u64 = 256 * 1024;
pub const RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES: usize = 256 * 1024;
pub const RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES: usize = 4;
pub const RUNTIME_CONTROL_PLANE_REQUEST_ID_MAX_BYTES: usize = 96;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeIdError {
    Empty,
    TooLong,
    InvalidPrefix,
    InvalidCharacter,
    RawIdentifier,
}

impl std::fmt::Display for RuntimeIdError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("runtime identifier is empty"),
            Self::TooLong => formatter.write_str("runtime identifier is too long"),
            Self::InvalidPrefix => formatter.write_str("runtime identifier has an invalid prefix"),
            Self::InvalidCharacter => {
                formatter.write_str("runtime identifier contains an invalid character")
            }
            Self::RawIdentifier => {
                formatter.write_str("runtime identifier contains raw identifier syntax")
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeControlPlaneAdapterError {
    InvalidJson,
    NonObjectRoot,
    RelativeFilePath,
    RelativeAllowedRoot,
    MissingFile,
    MissingAllowedRoot,
    AllowedRootSymlink,
    AllowedRootNotDirectory,
    SymlinkPath,
    DirectoryPath,
    NonRegularFile,
    UnsupportedFileExtension,
    OutsideAllowedRoot,
    OversizedFile {
        max_bytes: u64,
    },
    OversizedFrame {
        max_bytes: usize,
    },
    FileReadFailed,
    FileWriteFailed,
    InvalidUtf8,
    IpcReadFailed,
    IpcWriteFailed,
    MalformedIpcFrame,
    IncompleteIpcFrame,
    UnsupportedSchemaVersion {
        field: &'static str,
        expected: &'static str,
    },
    UnsupportedValue {
        field: &'static str,
    },
    UnsafeFlag {
        field: &'static str,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WorkspaceId(String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionId(String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct JobId(String);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    CompareModelScores,
    RefreshEvidenceIndex,
    RunNativeInferenceCandidate,
    RenderWorkstationSnapshot,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeInferenceRuntimeState {
    Unavailable,
    Available,
    Disabled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRegistryState {
    ObservedSyntheticOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPromotionState {
    NotPromoted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeHandoffSourceKind {
    StaticSyntheticFixture,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeHandoffTransportState {
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlPlaneState {
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlPlaneAdapterKind {
    StaticContractFixture,
    LocalJsonStringParser,
    LocalJsonFileAdapter,
    LocalControlPlaneMessageEnvelope,
    LocalControlPlaneFrameAdapter,
    LocalControlPlaneIpcStreamAdapter,
    LocalControlPlaneEndpointPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlPlaneInputMode {
    AcceptedSchemaDeclarationOnly,
    AcceptedLocalJsonString,
    AcceptedLocalJsonFile,
    AcceptedLocalMessageEnvelope,
    AcceptedLocalMessageFrame,
    AcceptedLocalIpcStream,
    AcceptedLocalEndpointPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlPlaneAdapterState {
    Unavailable,
    JsonStringParserAvailable,
    LocalFileAdapterAvailable,
    LocalMessageEnvelopeAvailable,
    LocalMessageFrameAvailable,
    LocalIpcStreamAvailable,
    LocalEndpointPolicyAvailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeControlPlaneEndpointKind {
    CallerProvidedConnectedStream,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlPlaneOutputSnapshotSchema {
    RuntimeHandoffSnapshotV0,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSummary {
    pub schema_version: String,
    pub workspace_id: WorkspaceId,
    pub session_id: SessionId,
    pub total_job_count: u32,
    pub queued_job_count: u32,
    pub running_job_count: u32,
    pub failed_job_count: u32,
    pub last_event_label: String,
    pub native_inference_state: NativeInferenceRuntimeState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeSummaryProviderContract {
    pub schema_version: &'static str,
    pub output_summary_schema: &'static str,
    pub local_only: bool,
    pub caller_provided_events_only: bool,
    pub event_replay_enabled: bool,
    pub storage_provider_enabled: bool,
    pub live_runtime_connection_enabled: bool,
    pub file_io_enabled: bool,
    pub process_spawning_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeSummaryProviderPolicy {
    pub local_only: bool,
    pub caller_provided_events_only: bool,
    pub storage_provider_enabled: bool,
    pub live_runtime_connection_enabled: bool,
    pub file_io_enabled: bool,
    pub process_spawning_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRegistryMetadata {
    pub schema_version: String,
    pub metadata_scope: String,
    pub source_bundle_schema: String,
    pub entries: Vec<ModelRegistryEntry>,
    pub aggregate_summary: ModelRegistryAggregateSummary,
    pub safety_flags: ModelRegistrySafetyFlags,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRegistryEntry {
    pub model_id: String,
    pub registry_state: ModelRegistryState,
    pub promotion_state: ModelPromotionState,
    pub observed_source_schemas: Vec<String>,
    pub observed_source_names: Vec<String>,
    pub source_count: u32,
    pub has_score_rows: bool,
    pub human_review_required: bool,
    pub deployment_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRegistryAggregateSummary {
    pub model_count: u32,
    pub schemas_present: Vec<String>,
    pub models_with_score_rows: Vec<String>,
    pub deployment_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRegistrySafetyFlags {
    pub local_only: bool,
    pub strict_json_loaded: bool,
    pub derived_from_evaluation_bundle_only: bool,
    pub input_paths_copied: bool,
    pub source_filenames_copied: bool,
    pub raw_identifiers_copied: bool,
    pub generated_artifact_references_copied: bool,
    pub secrets_detected: bool,
    pub report_payload_copied: bool,
    pub live_capture_used: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRegistryMetadataAdapterContract {
    pub schema_version: &'static str,
    pub accepted_metadata_schema: &'static str,
    pub source_bundle_schema: &'static str,
    pub max_file_bytes: u64,
    pub local_only: bool,
    pub synthetic_metadata_only: bool,
    pub strict_json_parsing_enabled: bool,
    pub file_io_enabled: bool,
    pub storage_provider_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRegistryMetadataAdapterPolicy {
    pub file_policy: RuntimeControlPlaneFilePolicy,
    pub local_only: bool,
    pub synthetic_metadata_only: bool,
    pub storage_provider_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeHandoffSnapshot {
    pub schema_version: String,
    pub source_kind: RuntimeHandoffSourceKind,
    pub transport_state: RuntimeHandoffTransportState,
    pub control_plane_state: RuntimeControlPlaneState,
    pub runtime_summary: RuntimeSummary,
    pub model_registry_metadata: ModelRegistryMetadata,
    pub local_only: bool,
    pub static_synthetic_fixture: bool,
    pub generated_json_loaded: bool,
    pub live_runtime_connection: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRegistryProviderContract {
    pub schema_version: &'static str,
    pub accepted_snapshot_schema: &'static str,
    pub output_snapshot_schema: &'static str,
    pub max_records: usize,
    pub local_only: bool,
    pub in_memory_only: bool,
    pub accepts_validated_handoff_snapshots_only: bool,
    pub strict_handoff_validation_enabled: bool,
    pub upsert_replaces_matching_workspace_session: bool,
    pub deterministic_snapshot_ordering: bool,
    pub persistent_storage_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub file_io_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRegistryProviderPolicy {
    pub max_records: usize,
    pub local_only: bool,
    pub in_memory_only: bool,
    pub accepts_validated_handoff_snapshots_only: bool,
    pub strict_handoff_validation_enabled: bool,
    pub persistent_storage_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub file_io_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRegistryRecord {
    pub workspace_id: WorkspaceId,
    pub session_id: SessionId,
    pub snapshot_schema_version: String,
    pub snapshot: RuntimeHandoffSnapshot,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRegistrySnapshot {
    pub schema_version: String,
    pub accepted_snapshot_schema: String,
    pub record_count: u32,
    pub max_record_count: u32,
    pub local_only: bool,
    pub in_memory_only: bool,
    pub persistent_storage_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub file_io_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub records: Vec<RuntimeRegistryRecord>,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRegistryProvider {
    policy: RuntimeRegistryProviderPolicy,
    records: BTreeMap<(String, String), RuntimeRegistryRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRegistryStorageProviderContract {
    pub schema_version: &'static str,
    pub accepted_registry_snapshot_schema: &'static str,
    pub storage_document_schema: &'static str,
    pub max_file_bytes: u64,
    pub local_only: bool,
    pub caller_authorized_allowed_root_required: bool,
    pub typed_registry_snapshots_only: bool,
    pub strict_registry_validation_enabled: bool,
    pub storage_document_json_enabled: bool,
    pub file_io_enabled: bool,
    pub persistent_storage_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub arbitrary_file_loading_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRegistryStoragePolicy {
    pub file_policy: RuntimeControlPlaneFilePolicy,
    pub max_file_bytes: u64,
    pub local_only: bool,
    pub caller_authorized_allowed_root_required: bool,
    pub typed_registry_snapshots_only: bool,
    pub strict_registry_validation_enabled: bool,
    pub storage_document_json_enabled: bool,
    pub file_io_enabled: bool,
    pub persistent_storage_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub arbitrary_file_loading_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRegistryStorageDocument {
    pub schema_version: String,
    pub registry_snapshot_schema: String,
    pub local_only: bool,
    pub caller_authorized_allowed_root_required: bool,
    pub typed_registry_snapshots_only: bool,
    pub strict_registry_validation_enabled: bool,
    pub storage_document_json_enabled: bool,
    pub file_io_enabled: bool,
    pub persistent_storage_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub arbitrary_file_loading_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub registry_snapshot: RuntimeRegistrySnapshot,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRegistryStorageProvider {
    policy: RuntimeRegistryStoragePolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneAdapterContract {
    pub schema_version: &'static str,
    pub adapter_kind: RuntimeControlPlaneAdapterKind,
    pub input_mode: RuntimeControlPlaneInputMode,
    pub adapter_state: RuntimeControlPlaneAdapterState,
    pub output_snapshot_schema: RuntimeControlPlaneOutputSnapshotSchema,
    pub accepted_input_schemas: &'static [&'static str],
    pub local_only: bool,
    pub dependency_free: bool,
    pub static_synthetic_fixture: bool,
    pub json_parsing_enabled: bool,
    pub file_io_enabled: bool,
    pub live_transport_enabled: bool,
    pub qt_binding_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneFramePolicy {
    pub max_frame_bytes: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeControlPlaneIpcPolicy {
    pub frame_policy: RuntimeControlPlaneFramePolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneEndpointPolicy {
    pub schema_version: &'static str,
    pub endpoint_kind: RuntimeControlPlaneEndpointKind,
    pub ipc_policy: RuntimeControlPlaneIpcPolicy,
    pub local_only: bool,
    pub caller_provided_streams_only: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneFrameAdapterContract {
    pub schema_version: &'static str,
    pub payload_schema_version: &'static str,
    pub max_frame_bytes: usize,
    pub local_only: bool,
    pub caller_provided_bytes_only: bool,
    pub utf8_json_payload_required: bool,
    pub additional_dependencies_required: bool,
    pub live_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub capture_enabled: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneIpcAdapterContract {
    pub schema_version: &'static str,
    pub frame_schema_version: &'static str,
    pub message_schema_version: &'static str,
    pub length_prefix_bytes: usize,
    pub max_frame_bytes: usize,
    pub local_only: bool,
    pub caller_provided_streams_only: bool,
    pub one_shot_request_response: bool,
    pub big_endian_length_prefix_required: bool,
    pub utf8_json_payload_required: bool,
    pub additional_dependencies_required: bool,
    pub stream_io_enabled: bool,
    pub live_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneEndpointAdapterContract {
    pub schema_version: &'static str,
    pub ipc_schema_version: &'static str,
    pub frame_schema_version: &'static str,
    pub message_schema_version: &'static str,
    pub endpoint_kind: RuntimeControlPlaneEndpointKind,
    pub local_only: bool,
    pub caller_provided_streams_only: bool,
    pub endpoint_policy_validation_enabled: bool,
    pub connected_stream_execution_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeControlPlaneFilePolicy {
    pub allowed_root: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeControlPlaneCommand {
    ParseHandoffSnapshotJson {
        input: String,
    },
    ParseHandoffSnapshotFile {
        path: PathBuf,
        policy: RuntimeControlPlaneFilePolicy,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeControlPlaneRequestId(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneMessageRequest {
    pub schema_version: String,
    pub request_id: RuntimeControlPlaneRequestId,
    pub command: RuntimeControlPlaneCommand,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeControlPlaneMessageResponse {
    pub schema_version: String,
    pub request_id: RuntimeControlPlaneRequestId,
    pub outcome: RuntimeControlPlaneMessageOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<RuntimeHandoffSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<RuntimeControlPlaneMessageErrorCode>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlPlaneMessageOutcome {
    Success,
    Failure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlPlaneMessageErrorCode {
    InvalidJson,
    NonObjectRoot,
    RelativeFilePath,
    RelativeAllowedRoot,
    MissingFile,
    MissingAllowedRoot,
    AllowedRootSymlink,
    AllowedRootNotDirectory,
    SymlinkPath,
    DirectoryPath,
    NonRegularFile,
    UnsupportedFileExtension,
    OutsideAllowedRoot,
    OversizedFile,
    OversizedFrame,
    FileReadFailed,
    FileWriteFailed,
    InvalidUtf8,
    IpcReadFailed,
    IpcWriteFailed,
    MalformedIpcFrame,
    IncompleteIpcFrame,
    UnsupportedSchemaVersion,
    UnsupportedValue,
    UnsafeFlag,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct RawRuntimeControlPlaneMessageRequest {
    schema_version: String,
    request_id: String,
    command: RawRuntimeControlPlaneMessageCommand,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct RawRuntimeControlPlaneMessageCommand {
    command_kind: String,
    input: Option<String>,
    path: Option<PathBuf>,
    policy: Option<RuntimeControlPlaneFilePolicy>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeSummaryJobState {
    job_id: JobId,
    state: JobState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeEvent {
    WorkspaceOpened {
        workspace_id: WorkspaceId,
    },
    SessionStarted {
        workspace_id: WorkspaceId,
        session_id: SessionId,
    },
    JobQueued {
        session_id: SessionId,
        job_id: JobId,
        kind: JobKind,
    },
    JobStateChanged {
        job_id: JobId,
        state: JobState,
    },
}

impl<'de> Deserialize<'de> for WorkspaceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for JobId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for RuntimeControlPlaneRequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value)
            .map_err(|_| serde::de::Error::custom("runtime control-plane request id is unsafe"))
    }
}

impl WorkspaceId {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeIdError> {
        let value = value.into();
        validate_coarse_id(&value, &["workspace-", "fixture-workspace-"])?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl SessionId {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeIdError> {
        let value = value.into();
        validate_coarse_id(&value, &["session-", "fixture-session-"])?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl JobId {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeIdError> {
        let value = value.into();
        validate_coarse_id(&value, &["job-", "fixture-job-"])?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl RuntimeControlPlaneRequestId {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeControlPlaneAdapterError> {
        let value = value.into();
        validate_control_plane_request_id(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl RuntimeEvent {
    pub fn contract_summary() -> &'static str {
        "ARES local runtime boundary contract v0"
    }
}

impl JobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CompareModelScores => "compare_model_scores",
            Self::RefreshEvidenceIndex => "refresh_evidence_index",
            Self::RunNativeInferenceCandidate => "run_native_inference_candidate",
            Self::RenderWorkstationSnapshot => "render_workstation_snapshot",
        }
    }
}

impl JobState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl NativeInferenceRuntimeState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Available => "available",
            Self::Disabled => "disabled",
        }
    }
}

impl ModelRegistryState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ObservedSyntheticOnly => "observed_synthetic_only",
        }
    }
}

impl ModelPromotionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotPromoted => "not_promoted",
        }
    }
}

impl RuntimeHandoffSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StaticSyntheticFixture => "static_synthetic_fixture",
        }
    }
}

impl RuntimeHandoffTransportState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
        }
    }
}

impl RuntimeControlPlaneState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
        }
    }
}

impl RuntimeControlPlaneAdapterKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StaticContractFixture => "static_contract_fixture",
            Self::LocalJsonStringParser => "local_json_string_parser",
            Self::LocalJsonFileAdapter => "local_json_file_adapter",
            Self::LocalControlPlaneMessageEnvelope => "local_control_plane_message_envelope",
            Self::LocalControlPlaneFrameAdapter => "local_control_plane_frame_adapter",
            Self::LocalControlPlaneIpcStreamAdapter => "local_control_plane_ipc_stream_adapter",
            Self::LocalControlPlaneEndpointPolicy => "local_control_plane_endpoint_policy",
        }
    }
}

impl RuntimeControlPlaneInputMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptedSchemaDeclarationOnly => "accepted_schema_declaration_only",
            Self::AcceptedLocalJsonString => "accepted_local_json_string",
            Self::AcceptedLocalJsonFile => "accepted_local_json_file",
            Self::AcceptedLocalMessageEnvelope => "accepted_local_message_envelope",
            Self::AcceptedLocalMessageFrame => "accepted_local_message_frame",
            Self::AcceptedLocalIpcStream => "accepted_local_ipc_stream",
            Self::AcceptedLocalEndpointPolicy => "accepted_local_endpoint_policy",
        }
    }
}

impl RuntimeControlPlaneAdapterState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::JsonStringParserAvailable => "json_string_parser_available",
            Self::LocalFileAdapterAvailable => "local_file_adapter_available",
            Self::LocalMessageEnvelopeAvailable => "local_message_envelope_available",
            Self::LocalMessageFrameAvailable => "local_message_frame_available",
            Self::LocalIpcStreamAvailable => "local_ipc_stream_available",
            Self::LocalEndpointPolicyAvailable => "local_endpoint_policy_available",
        }
    }
}

impl RuntimeControlPlaneEndpointKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CallerProvidedConnectedStream => "caller_provided_connected_stream",
        }
    }
}

impl RuntimeControlPlaneOutputSnapshotSchema {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeHandoffSnapshotV0 => RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
        }
    }
}

impl RuntimeControlPlaneMessageOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

impl RuntimeControlPlaneMessageErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidJson => "invalid_json",
            Self::NonObjectRoot => "non_object_root",
            Self::RelativeFilePath => "relative_file_path",
            Self::RelativeAllowedRoot => "relative_allowed_root",
            Self::MissingFile => "missing_file",
            Self::MissingAllowedRoot => "missing_allowed_root",
            Self::AllowedRootSymlink => "allowed_root_symlink",
            Self::AllowedRootNotDirectory => "allowed_root_not_directory",
            Self::SymlinkPath => "symlink_path",
            Self::DirectoryPath => "directory_path",
            Self::NonRegularFile => "non_regular_file",
            Self::UnsupportedFileExtension => "unsupported_file_extension",
            Self::OutsideAllowedRoot => "outside_allowed_root",
            Self::OversizedFile => "oversized_file",
            Self::OversizedFrame => "oversized_frame",
            Self::FileReadFailed => "file_read_failed",
            Self::FileWriteFailed => "file_write_failed",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::IpcReadFailed => "ipc_read_failed",
            Self::IpcWriteFailed => "ipc_write_failed",
            Self::MalformedIpcFrame => "malformed_ipc_frame",
            Self::IncompleteIpcFrame => "incomplete_ipc_frame",
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnsupportedValue => "unsupported_value",
            Self::UnsafeFlag => "unsafe_flag",
        }
    }
}

impl From<&RuntimeControlPlaneAdapterError> for RuntimeControlPlaneMessageErrorCode {
    fn from(error: &RuntimeControlPlaneAdapterError) -> Self {
        match error {
            RuntimeControlPlaneAdapterError::InvalidJson => Self::InvalidJson,
            RuntimeControlPlaneAdapterError::NonObjectRoot => Self::NonObjectRoot,
            RuntimeControlPlaneAdapterError::RelativeFilePath => Self::RelativeFilePath,
            RuntimeControlPlaneAdapterError::RelativeAllowedRoot => Self::RelativeAllowedRoot,
            RuntimeControlPlaneAdapterError::MissingFile => Self::MissingFile,
            RuntimeControlPlaneAdapterError::MissingAllowedRoot => Self::MissingAllowedRoot,
            RuntimeControlPlaneAdapterError::AllowedRootSymlink => Self::AllowedRootSymlink,
            RuntimeControlPlaneAdapterError::AllowedRootNotDirectory => {
                Self::AllowedRootNotDirectory
            }
            RuntimeControlPlaneAdapterError::SymlinkPath => Self::SymlinkPath,
            RuntimeControlPlaneAdapterError::DirectoryPath => Self::DirectoryPath,
            RuntimeControlPlaneAdapterError::NonRegularFile => Self::NonRegularFile,
            RuntimeControlPlaneAdapterError::UnsupportedFileExtension => {
                Self::UnsupportedFileExtension
            }
            RuntimeControlPlaneAdapterError::OutsideAllowedRoot => Self::OutsideAllowedRoot,
            RuntimeControlPlaneAdapterError::OversizedFile { .. } => Self::OversizedFile,
            RuntimeControlPlaneAdapterError::OversizedFrame { .. } => Self::OversizedFrame,
            RuntimeControlPlaneAdapterError::FileReadFailed => Self::FileReadFailed,
            RuntimeControlPlaneAdapterError::FileWriteFailed => Self::FileWriteFailed,
            RuntimeControlPlaneAdapterError::InvalidUtf8 => Self::InvalidUtf8,
            RuntimeControlPlaneAdapterError::IpcReadFailed => Self::IpcReadFailed,
            RuntimeControlPlaneAdapterError::IpcWriteFailed => Self::IpcWriteFailed,
            RuntimeControlPlaneAdapterError::MalformedIpcFrame => Self::MalformedIpcFrame,
            RuntimeControlPlaneAdapterError::IncompleteIpcFrame => Self::IncompleteIpcFrame,
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion { .. } => {
                Self::UnsupportedSchemaVersion
            }
            RuntimeControlPlaneAdapterError::UnsupportedValue { .. } => Self::UnsupportedValue,
            RuntimeControlPlaneAdapterError::UnsafeFlag { .. } => Self::UnsafeFlag,
        }
    }
}

impl RuntimeSummary {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_SUMMARY_SCHEMA_VERSION.to_owned(),
            workspace_id: WorkspaceId::new("fixture-workspace-alpha")
                .expect("static fixture workspace id must be valid"),
            session_id: SessionId::new("fixture-session-runtime-summary")
                .expect("static fixture session id must be valid"),
            total_job_count: 4,
            queued_job_count: 1,
            running_job_count: 1,
            failed_job_count: 0,
            last_event_label: "synthetic workstation snapshot rendered".to_owned(),
            native_inference_state: NativeInferenceRuntimeState::Disabled,
        }
    }
}

impl RuntimeSummaryProviderContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_SUMMARY_PROVIDER_SCHEMA_VERSION,
            output_summary_schema: RUNTIME_SUMMARY_SCHEMA_VERSION,
            local_only: true,
            caller_provided_events_only: true,
            event_replay_enabled: true,
            storage_provider_enabled: false,
            live_runtime_connection_enabled: false,
            file_io_enabled: false,
            process_spawning_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_SUMMARY_PROVIDER_NON_CLAIMS,
        }
    }

    pub fn build_runtime_summary_from_events(
        workspace_id: WorkspaceId,
        session_id: SessionId,
        events: &[RuntimeEvent],
        native_inference_state: NativeInferenceRuntimeState,
        policy: &RuntimeSummaryProviderPolicy,
    ) -> Result<RuntimeSummary, RuntimeControlPlaneAdapterError> {
        build_runtime_summary_from_events(
            workspace_id,
            session_id,
            events,
            native_inference_state,
            policy,
        )
    }
}

impl RuntimeSummaryProviderPolicy {
    pub fn new() -> Self {
        Self {
            local_only: true,
            caller_provided_events_only: true,
            storage_provider_enabled: false,
            live_runtime_connection_enabled: false,
            file_io_enabled: false,
            process_spawning_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
        }
    }

    pub fn validate(&self) -> Result<(), RuntimeControlPlaneAdapterError> {
        validate_required_flag("runtime_summary_provider.local_only", self.local_only, true)?;
        validate_required_flag(
            "runtime_summary_provider.caller_provided_events_only",
            self.caller_provided_events_only,
            true,
        )?;
        validate_required_flag(
            "runtime_summary_provider.storage_provider_enabled",
            self.storage_provider_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_summary_provider.live_runtime_connection_enabled",
            self.live_runtime_connection_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_summary_provider.file_io_enabled",
            self.file_io_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_summary_provider.process_spawning_enabled",
            self.process_spawning_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_summary_provider.qt_binding_enabled",
            self.qt_binding_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_summary_provider.capture_enabled",
            self.capture_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_summary_provider.external_services_used",
            self.external_services_used,
            false,
        )?;
        validate_required_flag(
            "runtime_summary_provider.deployment_allowed",
            self.deployment_allowed,
            false,
        )?;
        validate_required_flag(
            "runtime_summary_provider.native_inference_execution_enabled",
            self.native_inference_execution_enabled,
            false,
        )
    }
}

impl Default for RuntimeSummaryProviderPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRegistryMetadata {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: MODEL_REGISTRY_METADATA_SCHEMA_VERSION.to_owned(),
            metadata_scope: MODEL_REGISTRY_METADATA_SCOPE.to_owned(),
            source_bundle_schema: MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION.to_owned(),
            entries: model_registry_metadata_entries(),
            aggregate_summary: ModelRegistryAggregateSummary {
                model_count: 10,
                schemas_present: static_str_vec(MODEL_REGISTRY_AGGREGATE_SCHEMAS),
                models_with_score_rows: static_str_vec(MODEL_REGISTRY_MODELS_WITH_SCORE_ROWS),
                deployment_allowed: false,
            },
            safety_flags: ModelRegistrySafetyFlags {
                local_only: true,
                strict_json_loaded: true,
                derived_from_evaluation_bundle_only: true,
                input_paths_copied: false,
                source_filenames_copied: false,
                raw_identifiers_copied: false,
                generated_artifact_references_copied: false,
                secrets_detected: false,
                report_payload_copied: false,
                live_capture_used: false,
                external_services_used: false,
                deployment_allowed: false,
            },
            non_claims: static_str_vec(MODEL_REGISTRY_NON_CLAIMS),
        }
    }
}

impl ModelRegistryMetadataAdapterContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: MODEL_REGISTRY_METADATA_ADAPTER_SCHEMA_VERSION,
            accepted_metadata_schema: MODEL_REGISTRY_METADATA_SCHEMA_VERSION,
            source_bundle_schema: MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION,
            max_file_bytes: RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES,
            local_only: true,
            synthetic_metadata_only: true,
            strict_json_parsing_enabled: true,
            file_io_enabled: true,
            storage_provider_enabled: false,
            generated_report_loading_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: MODEL_REGISTRY_METADATA_ADAPTER_NON_CLAIMS,
        }
    }

    pub fn parse_model_registry_metadata_json(
        input: &str,
    ) -> Result<ModelRegistryMetadata, RuntimeControlPlaneAdapterError> {
        parse_model_registry_metadata_json(input)
    }

    pub fn parse_model_registry_metadata_file(
        path: impl AsRef<Path>,
        policy: &ModelRegistryMetadataAdapterPolicy,
    ) -> Result<ModelRegistryMetadata, RuntimeControlPlaneAdapterError> {
        parse_model_registry_metadata_file(path, policy)
    }
}

impl ModelRegistryMetadataAdapterPolicy {
    pub fn new(allowed_root: impl Into<PathBuf>) -> Self {
        Self::from_file_policy(RuntimeControlPlaneFilePolicy::new(allowed_root))
    }

    pub fn from_file_policy(file_policy: RuntimeControlPlaneFilePolicy) -> Self {
        Self {
            file_policy,
            local_only: true,
            synthetic_metadata_only: true,
            storage_provider_enabled: false,
            generated_report_loading_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
        }
    }

    pub fn max_bytes(&self) -> u64 {
        self.file_policy.max_bytes()
    }

    pub fn validate(&self) -> Result<(), RuntimeControlPlaneAdapterError> {
        validate_required_flag(
            "model_registry_metadata_adapter.local_only",
            self.local_only,
            true,
        )?;
        validate_required_flag(
            "model_registry_metadata_adapter.synthetic_metadata_only",
            self.synthetic_metadata_only,
            true,
        )?;
        validate_required_flag(
            "model_registry_metadata_adapter.storage_provider_enabled",
            self.storage_provider_enabled,
            false,
        )?;
        validate_required_flag(
            "model_registry_metadata_adapter.generated_report_loading_enabled",
            self.generated_report_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "model_registry_metadata_adapter.qt_binding_enabled",
            self.qt_binding_enabled,
            false,
        )?;
        validate_required_flag(
            "model_registry_metadata_adapter.capture_enabled",
            self.capture_enabled,
            false,
        )?;
        validate_required_flag(
            "model_registry_metadata_adapter.external_services_used",
            self.external_services_used,
            false,
        )?;
        validate_required_flag(
            "model_registry_metadata_adapter.deployment_allowed",
            self.deployment_allowed,
            false,
        )?;
        validate_required_flag(
            "model_registry_metadata_adapter.native_inference_execution_enabled",
            self.native_inference_execution_enabled,
            false,
        )
    }
}

impl RuntimeHandoffSnapshot {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION.to_owned(),
            source_kind: RuntimeHandoffSourceKind::StaticSyntheticFixture,
            transport_state: RuntimeHandoffTransportState::Unavailable,
            control_plane_state: RuntimeControlPlaneState::Unavailable,
            runtime_summary: RuntimeSummary::synthetic_fixture(),
            model_registry_metadata: ModelRegistryMetadata::synthetic_fixture(),
            local_only: true,
            static_synthetic_fixture: true,
            generated_json_loaded: false,
            live_runtime_connection: false,
            external_services_used: false,
            deployment_allowed: false,
            non_claims: static_str_vec(RUNTIME_HANDOFF_NON_CLAIMS),
        }
    }
}

impl RuntimeRegistryProviderContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION,
            accepted_snapshot_schema: RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
            output_snapshot_schema: RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION,
            max_records: RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP,
            local_only: true,
            in_memory_only: true,
            accepts_validated_handoff_snapshots_only: true,
            strict_handoff_validation_enabled: true,
            upsert_replaces_matching_workspace_session: true,
            deterministic_snapshot_ordering: true,
            persistent_storage_enabled: false,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            file_io_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            filesystem_socket_path_policy_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_REGISTRY_PROVIDER_NON_CLAIMS,
        }
    }
}

impl RuntimeRegistryProviderPolicy {
    pub fn new() -> Self {
        Self::bounded(RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP)
    }

    pub fn bounded(max_records: usize) -> Self {
        Self {
            max_records,
            local_only: true,
            in_memory_only: true,
            accepts_validated_handoff_snapshots_only: true,
            strict_handoff_validation_enabled: true,
            persistent_storage_enabled: false,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            file_io_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            filesystem_socket_path_policy_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
        }
    }

    pub fn validate(&self) -> Result<(), RuntimeControlPlaneAdapterError> {
        if self.max_records == 0 || self.max_records > RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP
        {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_provider.max_records",
            });
        }
        validate_required_flag(
            "runtime_registry_provider.local_only",
            self.local_only,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_provider.in_memory_only",
            self.in_memory_only,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_provider.accepts_validated_handoff_snapshots_only",
            self.accepts_validated_handoff_snapshots_only,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_provider.strict_handoff_validation_enabled",
            self.strict_handoff_validation_enabled,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_provider.persistent_storage_enabled",
            self.persistent_storage_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.database_or_indexing_enabled",
            self.database_or_indexing_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.generated_report_loading_enabled",
            self.generated_report_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.generated_json_loading_enabled",
            self.generated_json_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.file_io_enabled",
            self.file_io_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.live_transport_enabled",
            self.live_transport_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.public_network_transport_enabled",
            self.public_network_transport_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.socket_listener_enabled",
            self.socket_listener_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.filesystem_socket_path_policy_enabled",
            self.filesystem_socket_path_policy_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.daemon_lifecycle_enabled",
            self.daemon_lifecycle_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.process_spawning_enabled",
            self.process_spawning_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.file_watching_enabled",
            self.file_watching_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.qt_binding_enabled",
            self.qt_binding_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.capture_enabled",
            self.capture_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.external_services_used",
            self.external_services_used,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.deployment_allowed",
            self.deployment_allowed,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_provider.native_inference_execution_enabled",
            self.native_inference_execution_enabled,
            false,
        )
    }
}

impl Default for RuntimeRegistryProviderPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeRegistryProvider {
    pub fn new(
        policy: RuntimeRegistryProviderPolicy,
    ) -> Result<Self, RuntimeControlPlaneAdapterError> {
        policy.validate()?;
        Ok(Self {
            policy,
            records: BTreeMap::new(),
        })
    }

    pub fn default_provider() -> Self {
        Self::new(RuntimeRegistryProviderPolicy::new())
            .expect("default runtime registry provider policy must be valid")
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn upsert_snapshot(
        &mut self,
        snapshot: RuntimeHandoffSnapshot,
    ) -> Result<RuntimeRegistryRecord, RuntimeControlPlaneAdapterError> {
        self.policy.validate()?;
        validate_runtime_handoff_snapshot(&snapshot)?;

        let workspace_id = snapshot.runtime_summary.workspace_id.clone();
        let session_id = snapshot.runtime_summary.session_id.clone();
        let key = (
            workspace_id.as_str().to_owned(),
            session_id.as_str().to_owned(),
        );
        if !self.records.contains_key(&key) && self.records.len() >= self.policy.max_records {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_provider.record_cap",
            });
        }

        let record = RuntimeRegistryRecord {
            workspace_id,
            session_id,
            snapshot_schema_version: RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION.to_owned(),
            snapshot,
        };
        self.records.insert(key, record.clone());
        Ok(record)
    }

    pub fn snapshot(&self) -> RuntimeRegistrySnapshot {
        RuntimeRegistrySnapshot {
            schema_version: RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION.to_owned(),
            accepted_snapshot_schema: RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION.to_owned(),
            record_count: self.records.len() as u32,
            max_record_count: self.policy.max_records as u32,
            local_only: true,
            in_memory_only: true,
            persistent_storage_enabled: false,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            file_io_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            filesystem_socket_path_policy_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            records: self.records.values().cloned().collect(),
            non_claims: static_str_vec(RUNTIME_REGISTRY_PROVIDER_NON_CLAIMS),
        }
    }
}

impl Default for RuntimeRegistryProvider {
    fn default() -> Self {
        Self::default_provider()
    }
}

impl RuntimeRegistryStorageProviderContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION,
            accepted_registry_snapshot_schema: RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION,
            storage_document_schema: RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION,
            max_file_bytes: RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES,
            local_only: true,
            caller_authorized_allowed_root_required: true,
            typed_registry_snapshots_only: true,
            strict_registry_validation_enabled: true,
            storage_document_json_enabled: true,
            file_io_enabled: true,
            persistent_storage_enabled: true,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            arbitrary_file_loading_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            filesystem_socket_path_policy_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_REGISTRY_STORAGE_PROVIDER_NON_CLAIMS,
        }
    }

    pub fn persist_snapshot_file(
        path: impl AsRef<Path>,
        snapshot: &RuntimeRegistrySnapshot,
        policy: &RuntimeRegistryStoragePolicy,
    ) -> Result<RuntimeRegistryStorageDocument, RuntimeControlPlaneAdapterError> {
        persist_runtime_registry_snapshot_file(path, snapshot, policy)
    }

    pub fn load_snapshot_file(
        path: impl AsRef<Path>,
        policy: &RuntimeRegistryStoragePolicy,
    ) -> Result<RuntimeRegistrySnapshot, RuntimeControlPlaneAdapterError> {
        load_runtime_registry_snapshot_file(path, policy)
    }

    pub fn parse_storage_document_json(
        input: &str,
    ) -> Result<RuntimeRegistryStorageDocument, RuntimeControlPlaneAdapterError> {
        parse_runtime_registry_storage_document_json(input)
    }
}

impl RuntimeRegistryStoragePolicy {
    pub fn new(allowed_root: impl Into<PathBuf>) -> Self {
        Self::from_file_policy(RuntimeControlPlaneFilePolicy::new(allowed_root))
    }

    pub fn from_file_policy(file_policy: RuntimeControlPlaneFilePolicy) -> Self {
        Self {
            file_policy,
            max_file_bytes: RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES,
            local_only: true,
            caller_authorized_allowed_root_required: true,
            typed_registry_snapshots_only: true,
            strict_registry_validation_enabled: true,
            storage_document_json_enabled: true,
            file_io_enabled: true,
            persistent_storage_enabled: true,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            arbitrary_file_loading_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            filesystem_socket_path_policy_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
        }
    }

    pub fn max_bytes(&self) -> u64 {
        self.max_file_bytes
    }

    pub fn validate(&self) -> Result<(), RuntimeControlPlaneAdapterError> {
        if self.max_file_bytes == 0 || self.max_file_bytes > RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES
        {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_storage_provider.max_file_bytes",
            });
        }
        validate_required_flag(
            "runtime_registry_storage_provider.local_only",
            self.local_only,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.caller_authorized_allowed_root_required",
            self.caller_authorized_allowed_root_required,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.typed_registry_snapshots_only",
            self.typed_registry_snapshots_only,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.strict_registry_validation_enabled",
            self.strict_registry_validation_enabled,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.storage_document_json_enabled",
            self.storage_document_json_enabled,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.file_io_enabled",
            self.file_io_enabled,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.persistent_storage_enabled",
            self.persistent_storage_enabled,
            true,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.database_or_indexing_enabled",
            self.database_or_indexing_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.generated_report_loading_enabled",
            self.generated_report_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.generated_json_loading_enabled",
            self.generated_json_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.arbitrary_file_loading_enabled",
            self.arbitrary_file_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.live_transport_enabled",
            self.live_transport_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.public_network_transport_enabled",
            self.public_network_transport_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.socket_listener_enabled",
            self.socket_listener_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.filesystem_socket_path_policy_enabled",
            self.filesystem_socket_path_policy_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.daemon_lifecycle_enabled",
            self.daemon_lifecycle_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.process_spawning_enabled",
            self.process_spawning_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.file_watching_enabled",
            self.file_watching_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.qt_binding_enabled",
            self.qt_binding_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.capture_enabled",
            self.capture_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.external_services_used",
            self.external_services_used,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.deployment_allowed",
            self.deployment_allowed,
            false,
        )?;
        validate_required_flag(
            "runtime_registry_storage_provider.native_inference_execution_enabled",
            self.native_inference_execution_enabled,
            false,
        )
    }
}

impl RuntimeRegistryStorageDocument {
    pub fn from_snapshot(
        snapshot: RuntimeRegistrySnapshot,
    ) -> Result<Self, RuntimeControlPlaneAdapterError> {
        validate_runtime_registry_snapshot(&snapshot)?;
        let document = Self {
            schema_version: RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION.to_owned(),
            registry_snapshot_schema: RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION.to_owned(),
            local_only: true,
            caller_authorized_allowed_root_required: true,
            typed_registry_snapshots_only: true,
            strict_registry_validation_enabled: true,
            storage_document_json_enabled: true,
            file_io_enabled: true,
            persistent_storage_enabled: true,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            arbitrary_file_loading_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            filesystem_socket_path_policy_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            registry_snapshot: snapshot,
            non_claims: static_str_vec(RUNTIME_REGISTRY_STORAGE_PROVIDER_NON_CLAIMS),
        };
        validate_runtime_registry_storage_document(&document)?;
        Ok(document)
    }
}

impl RuntimeRegistryStorageProvider {
    pub fn new(
        policy: RuntimeRegistryStoragePolicy,
    ) -> Result<Self, RuntimeControlPlaneAdapterError> {
        policy.validate()?;
        Ok(Self { policy })
    }

    pub fn persist_snapshot(
        &self,
        path: impl AsRef<Path>,
        snapshot: &RuntimeRegistrySnapshot,
    ) -> Result<RuntimeRegistryStorageDocument, RuntimeControlPlaneAdapterError> {
        persist_runtime_registry_snapshot_file(path, snapshot, &self.policy)
    }

    pub fn load_document(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<RuntimeRegistryStorageDocument, RuntimeControlPlaneAdapterError> {
        load_runtime_registry_storage_document_file(path, &self.policy)
    }

    pub fn load_snapshot(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<RuntimeRegistrySnapshot, RuntimeControlPlaneAdapterError> {
        load_runtime_registry_snapshot_file(path, &self.policy)
    }
}

impl RuntimeControlPlaneAdapterContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_ADAPTER_SCHEMA_VERSION,
            adapter_kind: RuntimeControlPlaneAdapterKind::LocalControlPlaneEndpointPolicy,
            input_mode: RuntimeControlPlaneInputMode::AcceptedLocalEndpointPolicy,
            adapter_state: RuntimeControlPlaneAdapterState::LocalEndpointPolicyAvailable,
            output_snapshot_schema:
                RuntimeControlPlaneOutputSnapshotSchema::RuntimeHandoffSnapshotV0,
            accepted_input_schemas: RUNTIME_CONTROL_PLANE_ADAPTER_ACCEPTED_SCHEMAS,
            local_only: true,
            dependency_free: false,
            static_synthetic_fixture: true,
            json_parsing_enabled: true,
            file_io_enabled: true,
            live_transport_enabled: false,
            qt_binding_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            non_claims: RUNTIME_CONTROL_PLANE_ADAPTER_NON_CLAIMS,
        }
    }

    pub fn parse_handoff_snapshot_json(
        input: &str,
    ) -> Result<RuntimeHandoffSnapshot, RuntimeControlPlaneAdapterError> {
        match input.trim_start().as_bytes().first() {
            Some(b'{') => {}
            Some(_) => return Err(RuntimeControlPlaneAdapterError::NonObjectRoot),
            None => return Err(RuntimeControlPlaneAdapterError::InvalidJson),
        }

        let snapshot: RuntimeHandoffSnapshot = serde_json::from_str(input)
            .map_err(|_| RuntimeControlPlaneAdapterError::InvalidJson)?;
        validate_runtime_handoff_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    pub fn parse_handoff_snapshot_file(
        path: impl AsRef<Path>,
        policy: &RuntimeControlPlaneFilePolicy,
    ) -> Result<RuntimeHandoffSnapshot, RuntimeControlPlaneAdapterError> {
        let canonical_path = validate_runtime_control_plane_json_file_path(path.as_ref(), policy)?;
        let bytes = fs::read(&canonical_path)
            .map_err(|_| RuntimeControlPlaneAdapterError::FileReadFailed)?;
        if bytes.len() as u64 > RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES {
            return Err(RuntimeControlPlaneAdapterError::OversizedFile {
                max_bytes: RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES,
            });
        }
        let input =
            String::from_utf8(bytes).map_err(|_| RuntimeControlPlaneAdapterError::InvalidUtf8)?;
        Self::parse_handoff_snapshot_json(&input)
    }

    pub fn execute_local_command(
        command: RuntimeControlPlaneCommand,
    ) -> Result<RuntimeHandoffSnapshot, RuntimeControlPlaneAdapterError> {
        match command {
            RuntimeControlPlaneCommand::ParseHandoffSnapshotJson { input } => {
                Self::parse_handoff_snapshot_json(&input)
            }
            RuntimeControlPlaneCommand::ParseHandoffSnapshotFile { path, policy } => {
                Self::parse_handoff_snapshot_file(&path, &policy)
            }
        }
    }

    pub fn parse_control_plane_message_request_json(
        input: &str,
    ) -> Result<RuntimeControlPlaneMessageRequest, RuntimeControlPlaneAdapterError> {
        match input.trim_start().as_bytes().first() {
            Some(b'{') => {}
            Some(_) => return Err(RuntimeControlPlaneAdapterError::NonObjectRoot),
            None => return Err(RuntimeControlPlaneAdapterError::InvalidJson),
        }

        let raw_request: RawRuntimeControlPlaneMessageRequest = serde_json::from_str(input)
            .map_err(|_| RuntimeControlPlaneAdapterError::InvalidJson)?;
        validate_schema_version(
            "schema_version",
            &raw_request.schema_version,
            RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
        )?;
        let request_id = RuntimeControlPlaneRequestId::new(raw_request.request_id)?;
        let command = parse_runtime_control_plane_message_command(raw_request.command)?;

        Ok(RuntimeControlPlaneMessageRequest {
            schema_version: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION.to_owned(),
            request_id,
            command,
        })
    }

    pub fn execute_control_plane_message_request(
        request: RuntimeControlPlaneMessageRequest,
    ) -> RuntimeControlPlaneMessageResponse {
        let request_id = request.request_id;
        match Self::execute_local_command(request.command) {
            Ok(snapshot) => RuntimeControlPlaneMessageResponse::success(request_id, snapshot),
            Err(error) => RuntimeControlPlaneMessageResponse::failure(request_id, (&error).into()),
        }
    }

    pub fn execute_control_plane_message_json(
        input: &str,
    ) -> Result<RuntimeControlPlaneMessageResponse, RuntimeControlPlaneAdapterError> {
        let request = Self::parse_control_plane_message_request_json(input)?;
        Ok(Self::execute_control_plane_message_request(request))
    }

    pub fn serialize_control_plane_message_response_json(
        response: &RuntimeControlPlaneMessageResponse,
    ) -> Result<String, RuntimeControlPlaneAdapterError> {
        serde_json::to_string(response).map_err(|_| RuntimeControlPlaneAdapterError::InvalidJson)
    }
}

impl RuntimeControlPlaneFrameAdapterContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION,
            payload_schema_version: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            max_frame_bytes: RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES,
            local_only: true,
            caller_provided_bytes_only: true,
            utf8_json_payload_required: true,
            additional_dependencies_required: false,
            live_transport_enabled: false,
            socket_listener_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            storage_provider_enabled: false,
            capture_enabled: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_CONTROL_PLANE_FRAME_NON_CLAIMS,
        }
    }

    pub fn parse_control_plane_message_frame_bytes(
        frame: &[u8],
        policy: &RuntimeControlPlaneFramePolicy,
    ) -> Result<RuntimeControlPlaneMessageRequest, RuntimeControlPlaneAdapterError> {
        let input = validate_control_plane_frame_bytes(frame, policy)?;
        RuntimeControlPlaneAdapterContract::parse_control_plane_message_request_json(input)
    }

    pub fn execute_control_plane_message_frame_bytes(
        frame: &[u8],
        policy: &RuntimeControlPlaneFramePolicy,
    ) -> Result<Vec<u8>, RuntimeControlPlaneAdapterError> {
        let request = Self::parse_control_plane_message_frame_bytes(frame, policy)?;
        let response =
            RuntimeControlPlaneAdapterContract::execute_control_plane_message_request(request);
        Self::serialize_control_plane_message_response_frame_bytes(&response, policy)
    }

    pub fn serialize_control_plane_message_response_frame_bytes(
        response: &RuntimeControlPlaneMessageResponse,
        policy: &RuntimeControlPlaneFramePolicy,
    ) -> Result<Vec<u8>, RuntimeControlPlaneAdapterError> {
        let response_json =
            RuntimeControlPlaneAdapterContract::serialize_control_plane_message_response_json(
                response,
            )?;
        let response_bytes = response_json.into_bytes();
        if response_bytes.len() > policy.max_frame_bytes {
            return Err(RuntimeControlPlaneAdapterError::OversizedFrame {
                max_bytes: policy.max_frame_bytes,
            });
        }
        Ok(response_bytes)
    }
}

impl RuntimeControlPlaneIpcAdapterContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION,
            frame_schema_version: RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION,
            message_schema_version: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            length_prefix_bytes: RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES,
            max_frame_bytes: RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES,
            local_only: true,
            caller_provided_streams_only: true,
            one_shot_request_response: true,
            big_endian_length_prefix_required: true,
            utf8_json_payload_required: true,
            additional_dependencies_required: false,
            stream_io_enabled: true,
            live_transport_enabled: false,
            socket_listener_enabled: false,
            filesystem_socket_path_policy_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            storage_provider_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_CONTROL_PLANE_IPC_NON_CLAIMS,
        }
    }
}

impl RuntimeControlPlaneEndpointAdapterContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
            ipc_schema_version: RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION,
            frame_schema_version: RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION,
            message_schema_version: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            endpoint_kind: RuntimeControlPlaneEndpointKind::CallerProvidedConnectedStream,
            local_only: true,
            caller_provided_streams_only: true,
            endpoint_policy_validation_enabled: true,
            connected_stream_execution_enabled: true,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            filesystem_socket_path_policy_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            storage_provider_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_CONTROL_PLANE_ENDPOINT_NON_CLAIMS,
        }
    }
}

impl Default for RuntimeControlPlaneFramePolicy {
    fn default() -> Self {
        Self {
            max_frame_bytes: RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES,
        }
    }
}

impl RuntimeControlPlaneFramePolicy {
    pub fn new(max_frame_bytes: usize) -> Result<Self, RuntimeControlPlaneAdapterError> {
        if max_frame_bytes == 0 || max_frame_bytes > RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "frame.max_frame_bytes",
            });
        }
        Ok(Self { max_frame_bytes })
    }

    pub fn max_bytes(&self) -> usize {
        self.max_frame_bytes
    }
}

impl RuntimeControlPlaneIpcPolicy {
    pub fn new(frame_policy: RuntimeControlPlaneFramePolicy) -> Self {
        Self { frame_policy }
    }

    pub fn max_frame_bytes(&self) -> usize {
        self.frame_policy.max_bytes()
    }
}

impl Default for RuntimeControlPlaneEndpointPolicy {
    fn default() -> Self {
        Self::caller_provided_connected_stream(RuntimeControlPlaneIpcPolicy::default())
    }
}

impl RuntimeControlPlaneEndpointPolicy {
    pub fn caller_provided_connected_stream(ipc_policy: RuntimeControlPlaneIpcPolicy) -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
            endpoint_kind: RuntimeControlPlaneEndpointKind::CallerProvidedConnectedStream,
            ipc_policy,
            local_only: true,
            caller_provided_streams_only: true,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            filesystem_socket_path_policy_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            storage_provider_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
        }
    }

    pub fn validate(&self) -> Result<(), RuntimeControlPlaneAdapterError> {
        validate_control_plane_endpoint_policy(self)
    }

    pub fn max_frame_bytes(&self) -> usize {
        self.ipc_policy.max_frame_bytes()
    }
}

pub fn parse_control_plane_message_frame_bytes(
    frame: &[u8],
) -> Result<RuntimeControlPlaneMessageRequest, RuntimeControlPlaneAdapterError> {
    RuntimeControlPlaneFrameAdapterContract::parse_control_plane_message_frame_bytes(
        frame,
        &RuntimeControlPlaneFramePolicy::default(),
    )
}

pub fn execute_control_plane_message_frame_bytes(
    frame: &[u8],
) -> Result<Vec<u8>, RuntimeControlPlaneAdapterError> {
    RuntimeControlPlaneFrameAdapterContract::execute_control_plane_message_frame_bytes(
        frame,
        &RuntimeControlPlaneFramePolicy::default(),
    )
}

pub fn serialize_control_plane_message_response_frame_bytes(
    response: &RuntimeControlPlaneMessageResponse,
) -> Result<Vec<u8>, RuntimeControlPlaneAdapterError> {
    RuntimeControlPlaneFrameAdapterContract::serialize_control_plane_message_response_frame_bytes(
        response,
        &RuntimeControlPlaneFramePolicy::default(),
    )
}

pub fn read_control_plane_message_ipc_frame<R: Read>(
    reader: &mut R,
    policy: &RuntimeControlPlaneIpcPolicy,
) -> Result<Vec<u8>, RuntimeControlPlaneAdapterError> {
    let mut length_prefix = [0_u8; RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES];
    read_exact_control_plane_ipc(reader, &mut length_prefix)?;

    let frame_len = u32::from_be_bytes(length_prefix) as usize;
    if frame_len == 0 {
        return Err(RuntimeControlPlaneAdapterError::InvalidJson);
    }
    if frame_len > policy.frame_policy.max_bytes() {
        return Err(RuntimeControlPlaneAdapterError::OversizedFrame {
            max_bytes: policy.frame_policy.max_bytes(),
        });
    }

    let mut frame = vec![0_u8; frame_len];
    read_exact_control_plane_ipc(reader, &mut frame)?;
    Ok(frame)
}

pub fn write_control_plane_message_ipc_frame<W: Write>(
    writer: &mut W,
    frame: &[u8],
    policy: &RuntimeControlPlaneIpcPolicy,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_control_plane_frame_bytes(frame, &policy.frame_policy)?;
    let frame_len = u32::try_from(frame.len()).map_err(|_| {
        RuntimeControlPlaneAdapterError::OversizedFrame {
            max_bytes: policy.frame_policy.max_bytes(),
        }
    })?;
    writer
        .write_all(&frame_len.to_be_bytes())
        .map_err(|_| RuntimeControlPlaneAdapterError::IpcWriteFailed)?;
    writer
        .write_all(frame)
        .map_err(|_| RuntimeControlPlaneAdapterError::IpcWriteFailed)
}

pub fn execute_control_plane_message_ipc_stream<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    policy: &RuntimeControlPlaneIpcPolicy,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    let request_frame = read_control_plane_message_ipc_frame(reader, policy)?;
    let response_frame =
        RuntimeControlPlaneFrameAdapterContract::execute_control_plane_message_frame_bytes(
            &request_frame,
            &policy.frame_policy,
        )?;
    write_control_plane_message_ipc_frame(writer, &response_frame, policy)
}

pub fn execute_control_plane_endpoint_stream<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    policy: &RuntimeControlPlaneEndpointPolicy,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    policy.validate()?;
    execute_control_plane_message_ipc_stream(reader, writer, &policy.ipc_policy)
}

pub fn parse_model_registry_metadata_json(
    input: &str,
) -> Result<ModelRegistryMetadata, RuntimeControlPlaneAdapterError> {
    match input.trim_start().as_bytes().first() {
        Some(b'{') => {}
        Some(_) => return Err(RuntimeControlPlaneAdapterError::NonObjectRoot),
        None => return Err(RuntimeControlPlaneAdapterError::InvalidJson),
    }

    let metadata: ModelRegistryMetadata =
        serde_json::from_str(input).map_err(|_| RuntimeControlPlaneAdapterError::InvalidJson)?;
    validate_schema_version(
        "schema_version",
        &metadata.schema_version,
        MODEL_REGISTRY_METADATA_SCHEMA_VERSION,
    )?;
    validate_model_registry_metadata(&metadata)?;
    Ok(metadata)
}

pub fn parse_model_registry_metadata_file(
    path: impl AsRef<Path>,
    policy: &ModelRegistryMetadataAdapterPolicy,
) -> Result<ModelRegistryMetadata, RuntimeControlPlaneAdapterError> {
    policy.validate()?;
    let canonical_path =
        validate_runtime_control_plane_json_file_path(path.as_ref(), &policy.file_policy)?;
    let bytes =
        fs::read(&canonical_path).map_err(|_| RuntimeControlPlaneAdapterError::FileReadFailed)?;
    if bytes.len() as u64 > RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES {
        return Err(RuntimeControlPlaneAdapterError::OversizedFile {
            max_bytes: RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES,
        });
    }
    let input =
        String::from_utf8(bytes).map_err(|_| RuntimeControlPlaneAdapterError::InvalidUtf8)?;
    parse_model_registry_metadata_json(&input)
}

pub fn parse_runtime_registry_storage_document_json(
    input: &str,
) -> Result<RuntimeRegistryStorageDocument, RuntimeControlPlaneAdapterError> {
    match input.trim_start().as_bytes().first() {
        Some(b'{') => {}
        Some(_) => return Err(RuntimeControlPlaneAdapterError::NonObjectRoot),
        None => return Err(RuntimeControlPlaneAdapterError::InvalidJson),
    }

    let document: RuntimeRegistryStorageDocument =
        serde_json::from_str(input).map_err(|_| RuntimeControlPlaneAdapterError::InvalidJson)?;
    validate_runtime_registry_storage_document(&document)?;
    Ok(document)
}

pub fn load_runtime_registry_storage_document_file(
    path: impl AsRef<Path>,
    policy: &RuntimeRegistryStoragePolicy,
) -> Result<RuntimeRegistryStorageDocument, RuntimeControlPlaneAdapterError> {
    policy.validate()?;
    let canonical_path = validate_runtime_registry_storage_json_read_path(path.as_ref(), policy)?;
    let bytes =
        fs::read(&canonical_path).map_err(|_| RuntimeControlPlaneAdapterError::FileReadFailed)?;
    if bytes.len() as u64 > policy.max_bytes() {
        return Err(RuntimeControlPlaneAdapterError::OversizedFile {
            max_bytes: policy.max_bytes(),
        });
    }
    let input =
        String::from_utf8(bytes).map_err(|_| RuntimeControlPlaneAdapterError::InvalidUtf8)?;
    parse_runtime_registry_storage_document_json(&input)
}

pub fn load_runtime_registry_snapshot_file(
    path: impl AsRef<Path>,
    policy: &RuntimeRegistryStoragePolicy,
) -> Result<RuntimeRegistrySnapshot, RuntimeControlPlaneAdapterError> {
    let document = load_runtime_registry_storage_document_file(path, policy)?;
    Ok(document.registry_snapshot)
}

pub fn persist_runtime_registry_snapshot_file(
    path: impl AsRef<Path>,
    snapshot: &RuntimeRegistrySnapshot,
    policy: &RuntimeRegistryStoragePolicy,
) -> Result<RuntimeRegistryStorageDocument, RuntimeControlPlaneAdapterError> {
    policy.validate()?;
    let document = RuntimeRegistryStorageDocument::from_snapshot(snapshot.clone())?;
    let canonical_path = validate_runtime_registry_storage_json_write_path(path.as_ref(), policy)?;
    let bytes = serde_json::to_vec_pretty(&document)
        .map_err(|_| RuntimeControlPlaneAdapterError::InvalidJson)?;
    if bytes.len() as u64 > policy.max_bytes() {
        return Err(RuntimeControlPlaneAdapterError::OversizedFile {
            max_bytes: policy.max_bytes(),
        });
    }
    fs::write(&canonical_path, bytes)
        .map_err(|_| RuntimeControlPlaneAdapterError::FileWriteFailed)?;
    Ok(document)
}

impl RuntimeControlPlaneFilePolicy {
    pub fn new(allowed_root: impl Into<PathBuf>) -> Self {
        Self {
            allowed_root: allowed_root.into(),
        }
    }

    pub fn max_bytes(&self) -> u64 {
        RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES
    }
}

impl RuntimeControlPlaneCommand {
    pub fn parse_handoff_snapshot_json(input: impl Into<String>) -> Self {
        Self::ParseHandoffSnapshotJson {
            input: input.into(),
        }
    }

    pub fn parse_handoff_snapshot_file(
        path: impl Into<PathBuf>,
        policy: RuntimeControlPlaneFilePolicy,
    ) -> Self {
        Self::ParseHandoffSnapshotFile {
            path: path.into(),
            policy,
        }
    }

    pub fn command_kind(&self) -> &'static str {
        match self {
            Self::ParseHandoffSnapshotJson { .. } => "parse_handoff_snapshot_json",
            Self::ParseHandoffSnapshotFile { .. } => "parse_handoff_snapshot_file",
        }
    }

    pub fn output_snapshot_schema(&self) -> RuntimeControlPlaneOutputSnapshotSchema {
        RuntimeControlPlaneOutputSnapshotSchema::RuntimeHandoffSnapshotV0
    }
}

impl RuntimeControlPlaneMessageRequest {
    pub fn new(
        request_id: impl Into<String>,
        command: RuntimeControlPlaneCommand,
    ) -> Result<Self, RuntimeControlPlaneAdapterError> {
        Ok(Self {
            schema_version: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION.to_owned(),
            request_id: RuntimeControlPlaneRequestId::new(request_id)?,
            command,
        })
    }
}

impl RuntimeControlPlaneMessageResponse {
    pub fn success(
        request_id: RuntimeControlPlaneRequestId,
        snapshot: RuntimeHandoffSnapshot,
    ) -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION.to_owned(),
            request_id,
            outcome: RuntimeControlPlaneMessageOutcome::Success,
            snapshot: Some(snapshot),
            error_code: None,
        }
    }

    pub fn failure(
        request_id: RuntimeControlPlaneRequestId,
        error_code: RuntimeControlPlaneMessageErrorCode,
    ) -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION.to_owned(),
            request_id,
            outcome: RuntimeControlPlaneMessageOutcome::Failure,
            snapshot: None,
            error_code: Some(error_code),
        }
    }
}

pub fn build_runtime_summary_from_events(
    workspace_id: WorkspaceId,
    session_id: SessionId,
    events: &[RuntimeEvent],
    native_inference_state: NativeInferenceRuntimeState,
    policy: &RuntimeSummaryProviderPolicy,
) -> Result<RuntimeSummary, RuntimeControlPlaneAdapterError> {
    policy.validate()?;
    if events.is_empty() {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_summary_provider.events",
        });
    }

    let mut jobs: Vec<RuntimeSummaryJobState> = Vec::new();
    let mut last_event_label = String::new();
    for event in events {
        match event {
            RuntimeEvent::WorkspaceOpened {
                workspace_id: event_workspace_id,
            } => {
                if event_workspace_id != &workspace_id {
                    return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                        field: "runtime_summary_provider.workspace_id",
                    });
                }
            }
            RuntimeEvent::SessionStarted {
                workspace_id: event_workspace_id,
                session_id: event_session_id,
            } => {
                if event_workspace_id != &workspace_id {
                    return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                        field: "runtime_summary_provider.workspace_id",
                    });
                }
                if event_session_id != &session_id {
                    return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                        field: "runtime_summary_provider.session_id",
                    });
                }
            }
            RuntimeEvent::JobQueued {
                session_id: event_session_id,
                job_id,
                kind: _,
            } => {
                if event_session_id != &session_id {
                    return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                        field: "runtime_summary_provider.session_id",
                    });
                }
                if jobs
                    .iter()
                    .any(|job| job.job_id.as_str() == job_id.as_str())
                {
                    return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                        field: "runtime_summary_provider.duplicate_job_id",
                    });
                }
                jobs.push(RuntimeSummaryJobState {
                    job_id: job_id.clone(),
                    state: JobState::Queued,
                });
            }
            RuntimeEvent::JobStateChanged { job_id, state } => {
                let job = jobs
                    .iter_mut()
                    .find(|known_job| known_job.job_id.as_str() == job_id.as_str())
                    .ok_or(RuntimeControlPlaneAdapterError::UnsupportedValue {
                        field: "runtime_summary_provider.unknown_job_id",
                    })?;
                job.state = *state;
            }
        }
        last_event_label = runtime_event_label(event).to_owned();
    }

    let summary = RuntimeSummary {
        schema_version: RUNTIME_SUMMARY_SCHEMA_VERSION.to_owned(),
        workspace_id,
        session_id,
        total_job_count: jobs.len() as u32,
        queued_job_count: count_runtime_jobs_by_state(&jobs, JobState::Queued),
        running_job_count: count_runtime_jobs_by_state(&jobs, JobState::Running),
        failed_job_count: count_runtime_jobs_by_state(&jobs, JobState::Failed),
        last_event_label,
        native_inference_state,
    };
    validate_runtime_summary(&summary)?;
    Ok(summary)
}

fn count_runtime_jobs_by_state(jobs: &[RuntimeSummaryJobState], state: JobState) -> u32 {
    jobs.iter().filter(|job| job.state == state).count() as u32
}

fn runtime_event_label(event: &RuntimeEvent) -> &'static str {
    match event {
        RuntimeEvent::WorkspaceOpened { .. } => "workspace opened",
        RuntimeEvent::SessionStarted { .. } => "session started",
        RuntimeEvent::JobQueued { kind, .. } => match kind {
            JobKind::CompareModelScores => "compare model scores job queued",
            JobKind::RefreshEvidenceIndex => "refresh evidence index job queued",
            JobKind::RunNativeInferenceCandidate => "native inference candidate job queued",
            JobKind::RenderWorkstationSnapshot => "workstation snapshot job queued",
        },
        RuntimeEvent::JobStateChanged { state, .. } => match state {
            JobState::Queued => "job queued",
            JobState::Running => "job running",
            JobState::Succeeded => "job succeeded",
            JobState::Failed => "job failed",
            JobState::Cancelled => "job cancelled",
        },
    }
}

fn parse_runtime_control_plane_message_command(
    raw_command: RawRuntimeControlPlaneMessageCommand,
) -> Result<RuntimeControlPlaneCommand, RuntimeControlPlaneAdapterError> {
    match raw_command.command_kind.as_str() {
        "parse_handoff_snapshot_json" => {
            if raw_command.path.is_some() || raw_command.policy.is_some() {
                return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field: "command" });
            }
            let input =
                raw_command
                    .input
                    .ok_or(RuntimeControlPlaneAdapterError::UnsupportedValue {
                        field: "command.input",
                    })?;
            Ok(RuntimeControlPlaneCommand::parse_handoff_snapshot_json(
                input,
            ))
        }
        "parse_handoff_snapshot_file" => {
            if raw_command.input.is_some() {
                return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field: "command" });
            }
            let path =
                raw_command
                    .path
                    .ok_or(RuntimeControlPlaneAdapterError::UnsupportedValue {
                        field: "command.path",
                    })?;
            let policy =
                raw_command
                    .policy
                    .ok_or(RuntimeControlPlaneAdapterError::UnsupportedValue {
                        field: "command.policy",
                    })?;
            Ok(RuntimeControlPlaneCommand::parse_handoff_snapshot_file(
                path, policy,
            ))
        }
        _ => Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "command.command_kind",
        }),
    }
}

fn validate_control_plane_frame_bytes<'a>(
    frame: &'a [u8],
    policy: &RuntimeControlPlaneFramePolicy,
) -> Result<&'a str, RuntimeControlPlaneAdapterError> {
    if frame.is_empty() {
        return Err(RuntimeControlPlaneAdapterError::InvalidJson);
    }
    if frame.len() > policy.max_frame_bytes {
        return Err(RuntimeControlPlaneAdapterError::OversizedFrame {
            max_bytes: policy.max_frame_bytes,
        });
    }
    std::str::from_utf8(frame).map_err(|_| RuntimeControlPlaneAdapterError::InvalidUtf8)
}

fn validate_control_plane_endpoint_policy(
    policy: &RuntimeControlPlaneEndpointPolicy,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_schema_version(
        "endpoint.schema_version",
        policy.schema_version,
        RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
    )?;
    match policy.endpoint_kind {
        RuntimeControlPlaneEndpointKind::CallerProvidedConnectedStream => {}
    }
    if policy.ipc_policy.frame_policy.max_frame_bytes == 0
        || policy.ipc_policy.frame_policy.max_frame_bytes > RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "endpoint.ipc_policy.frame_policy.max_frame_bytes",
        });
    }
    validate_required_flag("endpoint.local_only", policy.local_only, true)?;
    validate_required_flag(
        "endpoint.caller_provided_streams_only",
        policy.caller_provided_streams_only,
        true,
    )?;
    validate_required_flag(
        "endpoint.public_network_transport_enabled",
        policy.public_network_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint.socket_listener_enabled",
        policy.socket_listener_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint.filesystem_socket_path_policy_enabled",
        policy.filesystem_socket_path_policy_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint.daemon_lifecycle_enabled",
        policy.daemon_lifecycle_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint.process_spawning_enabled",
        policy.process_spawning_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint.file_watching_enabled",
        policy.file_watching_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint.qt_binding_enabled",
        policy.qt_binding_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint.storage_provider_enabled",
        policy.storage_provider_enabled,
        false,
    )?;
    validate_required_flag("endpoint.capture_enabled", policy.capture_enabled, false)?;
    validate_required_flag(
        "endpoint.external_services_used",
        policy.external_services_used,
        false,
    )?;
    validate_required_flag(
        "endpoint.deployment_allowed",
        policy.deployment_allowed,
        false,
    )?;
    validate_required_flag(
        "endpoint.native_inference_execution_enabled",
        policy.native_inference_execution_enabled,
        false,
    )
}

fn read_exact_control_plane_ipc<R: Read>(
    reader: &mut R,
    mut buffer: &mut [u8],
) -> Result<(), RuntimeControlPlaneAdapterError> {
    while !buffer.is_empty() {
        match reader.read(buffer) {
            Ok(0) => return Err(RuntimeControlPlaneAdapterError::IncompleteIpcFrame),
            Ok(bytes_read) => {
                let remaining = buffer;
                buffer = &mut remaining[bytes_read..];
            }
            Err(_) => return Err(RuntimeControlPlaneAdapterError::IpcReadFailed),
        }
    }
    Ok(())
}

fn validate_runtime_control_plane_json_file_path(
    path: &Path,
    policy: &RuntimeControlPlaneFilePolicy,
) -> Result<PathBuf, RuntimeControlPlaneAdapterError> {
    if !path.is_absolute() {
        return Err(RuntimeControlPlaneAdapterError::RelativeFilePath);
    }
    if !policy.allowed_root.is_absolute() {
        return Err(RuntimeControlPlaneAdapterError::RelativeAllowedRoot);
    }
    if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedFileExtension);
    }

    let allowed_root_metadata = fs::symlink_metadata(&policy.allowed_root)
        .map_err(|_| RuntimeControlPlaneAdapterError::MissingAllowedRoot)?;
    if allowed_root_metadata.file_type().is_symlink() {
        return Err(RuntimeControlPlaneAdapterError::AllowedRootSymlink);
    }
    if !allowed_root_metadata.is_dir() {
        return Err(RuntimeControlPlaneAdapterError::AllowedRootNotDirectory);
    }

    let file_metadata =
        fs::symlink_metadata(path).map_err(|_| RuntimeControlPlaneAdapterError::MissingFile)?;
    if file_metadata.file_type().is_symlink() {
        return Err(RuntimeControlPlaneAdapterError::SymlinkPath);
    }
    if file_metadata.is_dir() {
        return Err(RuntimeControlPlaneAdapterError::DirectoryPath);
    }
    if !file_metadata.file_type().is_file() {
        return Err(RuntimeControlPlaneAdapterError::NonRegularFile);
    }
    if file_metadata.len() > RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES {
        return Err(RuntimeControlPlaneAdapterError::OversizedFile {
            max_bytes: RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES,
        });
    }

    let canonical_allowed_root = fs::canonicalize(&policy.allowed_root)
        .map_err(|_| RuntimeControlPlaneAdapterError::MissingAllowedRoot)?;
    let canonical_path =
        fs::canonicalize(path).map_err(|_| RuntimeControlPlaneAdapterError::MissingFile)?;
    if !canonical_path.starts_with(&canonical_allowed_root) {
        return Err(RuntimeControlPlaneAdapterError::OutsideAllowedRoot);
    }

    Ok(canonical_path)
}

fn validate_runtime_registry_storage_json_read_path(
    path: &Path,
    policy: &RuntimeRegistryStoragePolicy,
) -> Result<PathBuf, RuntimeControlPlaneAdapterError> {
    validate_runtime_registry_storage_json_path(path, policy, true)
}

fn validate_runtime_registry_storage_json_write_path(
    path: &Path,
    policy: &RuntimeRegistryStoragePolicy,
) -> Result<PathBuf, RuntimeControlPlaneAdapterError> {
    validate_runtime_registry_storage_json_path(path, policy, false)
}

fn validate_runtime_registry_storage_json_path(
    path: &Path,
    policy: &RuntimeRegistryStoragePolicy,
    file_must_exist: bool,
) -> Result<PathBuf, RuntimeControlPlaneAdapterError> {
    if !path.is_absolute() {
        return Err(RuntimeControlPlaneAdapterError::RelativeFilePath);
    }
    if !policy.file_policy.allowed_root.is_absolute() {
        return Err(RuntimeControlPlaneAdapterError::RelativeAllowedRoot);
    }
    if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedFileExtension);
    }

    let allowed_root_metadata = fs::symlink_metadata(&policy.file_policy.allowed_root)
        .map_err(|_| RuntimeControlPlaneAdapterError::MissingAllowedRoot)?;
    if allowed_root_metadata.file_type().is_symlink() {
        return Err(RuntimeControlPlaneAdapterError::AllowedRootSymlink);
    }
    if !allowed_root_metadata.is_dir() {
        return Err(RuntimeControlPlaneAdapterError::AllowedRootNotDirectory);
    }
    let canonical_allowed_root = fs::canonicalize(&policy.file_policy.allowed_root)
        .map_err(|_| RuntimeControlPlaneAdapterError::MissingAllowedRoot)?;

    match fs::symlink_metadata(path) {
        Ok(file_metadata) => {
            if file_metadata.file_type().is_symlink() {
                return Err(RuntimeControlPlaneAdapterError::SymlinkPath);
            }
            if file_metadata.is_dir() {
                return Err(RuntimeControlPlaneAdapterError::DirectoryPath);
            }
            if !file_metadata.file_type().is_file() {
                return Err(RuntimeControlPlaneAdapterError::NonRegularFile);
            }
            if file_metadata.len() > policy.max_bytes() {
                return Err(RuntimeControlPlaneAdapterError::OversizedFile {
                    max_bytes: policy.max_bytes(),
                });
            }

            let canonical_path =
                fs::canonicalize(path).map_err(|_| RuntimeControlPlaneAdapterError::MissingFile)?;
            if !canonical_path.starts_with(&canonical_allowed_root) {
                return Err(RuntimeControlPlaneAdapterError::OutsideAllowedRoot);
            }
            Ok(canonical_path)
        }
        Err(_) if file_must_exist => Err(RuntimeControlPlaneAdapterError::MissingFile),
        Err(_) => {
            let parent = path
                .parent()
                .ok_or(RuntimeControlPlaneAdapterError::MissingFile)?;
            let parent_metadata = fs::symlink_metadata(parent)
                .map_err(|_| RuntimeControlPlaneAdapterError::MissingFile)?;
            if parent_metadata.file_type().is_symlink() {
                return Err(RuntimeControlPlaneAdapterError::SymlinkPath);
            }
            if !parent_metadata.is_dir() {
                return Err(RuntimeControlPlaneAdapterError::MissingFile);
            }
            let canonical_parent = fs::canonicalize(parent)
                .map_err(|_| RuntimeControlPlaneAdapterError::MissingFile)?;
            if !canonical_parent.starts_with(&canonical_allowed_root) {
                return Err(RuntimeControlPlaneAdapterError::OutsideAllowedRoot);
            }
            let file_name = path
                .file_name()
                .ok_or(RuntimeControlPlaneAdapterError::MissingFile)?;
            Ok(canonical_parent.join(file_name))
        }
    }
}

fn validate_runtime_registry_storage_document(
    document: &RuntimeRegistryStorageDocument,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_schema_version(
        "runtime_registry_storage_provider.schema_version",
        &document.schema_version,
        RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION,
    )?;
    validate_schema_version(
        "runtime_registry_storage_provider.registry_snapshot_schema",
        &document.registry_snapshot_schema,
        RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.local_only",
        document.local_only,
        true,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.caller_authorized_allowed_root_required",
        document.caller_authorized_allowed_root_required,
        true,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.typed_registry_snapshots_only",
        document.typed_registry_snapshots_only,
        true,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.strict_registry_validation_enabled",
        document.strict_registry_validation_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.storage_document_json_enabled",
        document.storage_document_json_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.file_io_enabled",
        document.file_io_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.persistent_storage_enabled",
        document.persistent_storage_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.database_or_indexing_enabled",
        document.database_or_indexing_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.generated_report_loading_enabled",
        document.generated_report_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.generated_json_loading_enabled",
        document.generated_json_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.arbitrary_file_loading_enabled",
        document.arbitrary_file_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.live_transport_enabled",
        document.live_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.public_network_transport_enabled",
        document.public_network_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.socket_listener_enabled",
        document.socket_listener_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.filesystem_socket_path_policy_enabled",
        document.filesystem_socket_path_policy_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.daemon_lifecycle_enabled",
        document.daemon_lifecycle_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.process_spawning_enabled",
        document.process_spawning_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.file_watching_enabled",
        document.file_watching_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.qt_binding_enabled",
        document.qt_binding_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.capture_enabled",
        document.capture_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.external_services_used",
        document.external_services_used,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.deployment_allowed",
        document.deployment_allowed,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_storage_provider.native_inference_execution_enabled",
        document.native_inference_execution_enabled,
        false,
    )?;
    validate_exact_strings(
        "runtime_registry_storage_provider.non_claims",
        &document.non_claims,
        RUNTIME_REGISTRY_STORAGE_PROVIDER_NON_CLAIMS,
    )?;
    validate_runtime_registry_snapshot(&document.registry_snapshot)
}

fn validate_runtime_registry_snapshot(
    snapshot: &RuntimeRegistrySnapshot,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_schema_version(
        "runtime_registry_snapshot.schema_version",
        &snapshot.schema_version,
        RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION,
    )?;
    validate_schema_version(
        "runtime_registry_snapshot.accepted_snapshot_schema",
        &snapshot.accepted_snapshot_schema,
        RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
    )?;
    if snapshot.max_record_count == 0
        || snapshot.max_record_count > RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP as u32
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_registry_snapshot.max_record_count",
        });
    }
    if snapshot.record_count != snapshot.records.len() as u32
        || snapshot.record_count > snapshot.max_record_count
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_registry_snapshot.record_count",
        });
    }
    validate_required_flag(
        "runtime_registry_snapshot.local_only",
        snapshot.local_only,
        true,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.in_memory_only",
        snapshot.in_memory_only,
        true,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.persistent_storage_enabled",
        snapshot.persistent_storage_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.database_or_indexing_enabled",
        snapshot.database_or_indexing_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.generated_report_loading_enabled",
        snapshot.generated_report_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.generated_json_loading_enabled",
        snapshot.generated_json_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.file_io_enabled",
        snapshot.file_io_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.live_transport_enabled",
        snapshot.live_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.public_network_transport_enabled",
        snapshot.public_network_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.socket_listener_enabled",
        snapshot.socket_listener_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.filesystem_socket_path_policy_enabled",
        snapshot.filesystem_socket_path_policy_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.daemon_lifecycle_enabled",
        snapshot.daemon_lifecycle_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.process_spawning_enabled",
        snapshot.process_spawning_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.file_watching_enabled",
        snapshot.file_watching_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.qt_binding_enabled",
        snapshot.qt_binding_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.capture_enabled",
        snapshot.capture_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.external_services_used",
        snapshot.external_services_used,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.deployment_allowed",
        snapshot.deployment_allowed,
        false,
    )?;
    validate_required_flag(
        "runtime_registry_snapshot.native_inference_execution_enabled",
        snapshot.native_inference_execution_enabled,
        false,
    )?;
    validate_exact_strings(
        "runtime_registry_snapshot.non_claims",
        &snapshot.non_claims,
        RUNTIME_REGISTRY_PROVIDER_NON_CLAIMS,
    )?;

    let mut previous_key: Option<(String, String)> = None;
    for record in &snapshot.records {
        validate_runtime_registry_record(record)?;
        let key = (
            record.workspace_id.as_str().to_owned(),
            record.session_id.as_str().to_owned(),
        );
        if previous_key
            .as_ref()
            .is_some_and(|previous_key| previous_key >= &key)
        {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_snapshot.records",
            });
        }
        previous_key = Some(key);
    }
    Ok(())
}

fn validate_runtime_registry_record(
    record: &RuntimeRegistryRecord,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_schema_version(
        "runtime_registry_snapshot.records.snapshot_schema_version",
        &record.snapshot_schema_version,
        RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
    )?;
    validate_runtime_handoff_snapshot(&record.snapshot)?;
    if record.workspace_id != record.snapshot.runtime_summary.workspace_id {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_registry_snapshot.records.workspace_id",
        });
    }
    if record.session_id != record.snapshot.runtime_summary.session_id {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_registry_snapshot.records.session_id",
        });
    }
    Ok(())
}

fn validate_runtime_handoff_snapshot(
    snapshot: &RuntimeHandoffSnapshot,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_schema_version(
        "schema_version",
        &snapshot.schema_version,
        RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
    )?;
    validate_schema_version(
        "runtime_summary.schema_version",
        &snapshot.runtime_summary.schema_version,
        RUNTIME_SUMMARY_SCHEMA_VERSION,
    )?;
    validate_schema_version(
        "model_registry_metadata.schema_version",
        &snapshot.model_registry_metadata.schema_version,
        MODEL_REGISTRY_METADATA_SCHEMA_VERSION,
    )?;

    validate_required_flag("local_only", snapshot.local_only, true)?;
    validate_required_flag(
        "static_synthetic_fixture",
        snapshot.static_synthetic_fixture,
        true,
    )?;
    validate_required_flag(
        "generated_json_loaded",
        snapshot.generated_json_loaded,
        false,
    )?;
    validate_required_flag(
        "live_runtime_connection",
        snapshot.live_runtime_connection,
        false,
    )?;
    validate_required_flag(
        "external_services_used",
        snapshot.external_services_used,
        false,
    )?;
    validate_required_flag("deployment_allowed", snapshot.deployment_allowed, false)?;
    validate_exact_strings(
        "non_claims",
        &snapshot.non_claims,
        RUNTIME_HANDOFF_NON_CLAIMS,
    )?;
    validate_runtime_summary(&snapshot.runtime_summary)?;
    validate_model_registry_metadata(&snapshot.model_registry_metadata)?;

    Ok(())
}

fn validate_runtime_summary(
    summary: &RuntimeSummary,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if summary.total_job_count < summary.queued_job_count
        || summary.total_job_count < summary.running_job_count
        || summary.total_job_count < summary.failed_job_count
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_summary.job_counts",
        });
    }
    if summary.last_event_label.trim().is_empty() {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_summary.last_event_label",
        });
    }
    validate_safe_event_label(
        "runtime_summary.last_event_label",
        &summary.last_event_label,
    )?;
    Ok(())
}

fn validate_model_registry_metadata(
    metadata: &ModelRegistryMetadata,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_exact_string(
        "model_registry_metadata.metadata_scope",
        &metadata.metadata_scope,
        MODEL_REGISTRY_METADATA_SCOPE,
    )?;
    validate_exact_string(
        "model_registry_metadata.source_bundle_schema",
        &metadata.source_bundle_schema,
        MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION,
    )?;
    validate_exact_strings(
        "model_registry_metadata.non_claims",
        &metadata.non_claims,
        MODEL_REGISTRY_NON_CLAIMS,
    )?;
    validate_required_flag(
        "model_registry_metadata.aggregate_summary.deployment_allowed",
        metadata.aggregate_summary.deployment_allowed,
        false,
    )?;
    validate_model_registry_entry_order(&metadata.entries)?;
    validate_model_registry_safety_flags(&metadata.safety_flags)?;
    for entry in &metadata.entries {
        validate_model_registry_entry(entry)?;
    }
    validate_model_registry_aggregate_summary(metadata)?;

    Ok(())
}

fn validate_model_registry_entry_order(
    entries: &[ModelRegistryEntry],
) -> Result<(), RuntimeControlPlaneAdapterError> {
    let mut seen_model_ids = BTreeSet::new();
    let mut previous_model_id: Option<&str> = None;
    for entry in entries {
        validate_safe_model_id("model_registry_metadata.entries.model_id", &entry.model_id)?;
        if previous_model_id.is_some_and(|previous| previous >= entry.model_id.as_str())
            || !seen_model_ids.insert(entry.model_id.as_str())
        {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.entries",
            });
        }
        previous_model_id = Some(entry.model_id.as_str());
    }
    Ok(())
}

fn validate_model_registry_safety_flags(
    flags: &ModelRegistrySafetyFlags,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_required_flag(
        "model_registry_metadata.safety_flags.local_only",
        flags.local_only,
        true,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.strict_json_loaded",
        flags.strict_json_loaded,
        true,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.derived_from_evaluation_bundle_only",
        flags.derived_from_evaluation_bundle_only,
        true,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.input_paths_copied",
        flags.input_paths_copied,
        false,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.source_filenames_copied",
        flags.source_filenames_copied,
        false,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.raw_identifiers_copied",
        flags.raw_identifiers_copied,
        false,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.generated_artifact_references_copied",
        flags.generated_artifact_references_copied,
        false,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.secrets_detected",
        flags.secrets_detected,
        false,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.report_payload_copied",
        flags.report_payload_copied,
        false,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.live_capture_used",
        flags.live_capture_used,
        false,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.external_services_used",
        flags.external_services_used,
        false,
    )?;
    validate_required_flag(
        "model_registry_metadata.safety_flags.deployment_allowed",
        flags.deployment_allowed,
        false,
    )?;
    Ok(())
}

fn validate_model_registry_entry(
    entry: &ModelRegistryEntry,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_safe_model_id("model_registry_metadata.entries.model_id", &entry.model_id)?;
    validate_sorted_unique_strings(
        "model_registry_metadata.entries.observed_source_schemas",
        &entry.observed_source_schemas,
    )?;
    if entry.observed_source_schemas.is_empty() {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.entries.observed_source_schemas",
        });
    }
    for source_schema in &entry.observed_source_schemas {
        validate_supported_model_registry_source_schema(
            "model_registry_metadata.entries.observed_source_schemas",
            source_schema,
        )?;
    }
    validate_sorted_unique_strings(
        "model_registry_metadata.entries.observed_source_names",
        &entry.observed_source_names,
    )?;
    if entry.observed_source_names.is_empty() {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.entries.observed_source_names",
        });
    }
    validate_required_flag(
        "model_registry_metadata.entries.human_review_required",
        entry.human_review_required,
        true,
    )?;
    validate_required_flag(
        "model_registry_metadata.entries.deployment_allowed",
        entry.deployment_allowed,
        false,
    )?;
    if entry.source_count != entry.observed_source_names.len() as u32 {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.entries.source_count",
        });
    }
    for source_name in &entry.observed_source_names {
        validate_safe_source_name(
            "model_registry_metadata.entries.observed_source_names",
            source_name,
        )?;
    }
    Ok(())
}

fn validate_model_registry_aggregate_summary(
    metadata: &ModelRegistryMetadata,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if metadata.aggregate_summary.model_count != metadata.entries.len() as u32 {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.aggregate_summary.model_count",
        });
    }

    validate_sorted_unique_strings(
        "model_registry_metadata.aggregate_summary.schemas_present",
        &metadata.aggregate_summary.schemas_present,
    )?;
    for source_schema in &metadata.aggregate_summary.schemas_present {
        validate_supported_model_registry_source_schema(
            "model_registry_metadata.aggregate_summary.schemas_present",
            source_schema,
        )?;
    }
    validate_sorted_unique_strings(
        "model_registry_metadata.aggregate_summary.models_with_score_rows",
        &metadata.aggregate_summary.models_with_score_rows,
    )?;
    for model_id in &metadata.aggregate_summary.models_with_score_rows {
        validate_safe_model_id(
            "model_registry_metadata.aggregate_summary.models_with_score_rows",
            model_id,
        )?;
    }

    let derived_schemas = metadata
        .entries
        .iter()
        .flat_map(|entry| entry.observed_source_schemas.iter().map(String::as_str))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let derived_models_with_score_rows = metadata
        .entries
        .iter()
        .filter(|entry| entry.has_score_rows)
        .map(|entry| entry.model_id.clone())
        .collect::<Vec<_>>();

    if metadata.aggregate_summary.schemas_present != derived_schemas {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.aggregate_summary.schemas_present",
        });
    }
    if metadata.aggregate_summary.models_with_score_rows != derived_models_with_score_rows {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.aggregate_summary.models_with_score_rows",
        });
    }

    Ok(())
}

fn validate_schema_version(
    field: &'static str,
    actual: &str,
    expected: &'static str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if actual != expected {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion { field, expected });
    }
    Ok(())
}

fn validate_exact_string(
    field: &'static str,
    actual: &str,
    expected: &'static str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if actual != expected {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_exact_strings(
    field: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if actual.len() != expected.len()
        || !actual
            .iter()
            .zip(expected.iter())
            .all(|(actual, expected)| actual == expected)
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_sorted_unique_strings(
    field: &'static str,
    values: &[String],
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_required_flag(
    field: &'static str,
    actual: bool,
    expected: bool,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if actual != expected {
        return Err(RuntimeControlPlaneAdapterError::UnsafeFlag { field });
    }
    Ok(())
}

fn validate_safe_model_id(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_no_unsafe_label_parts(field, value)?;
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    };
    if value.len() > 81
        || !first.is_ascii_lowercase()
        || !bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_safe_source_name(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_no_unsafe_label_parts(field, value)?;
    let bytes = value.as_bytes();
    if bytes.len() < 5
        || bytes.len() > 101
        || !bytes[0].is_ascii_lowercase()
        || bytes[bytes.len() - 4] != b'_'
        || !bytes[bytes.len() - 3..].iter().all(u8::is_ascii_digit)
        || !bytes[1..bytes.len() - 4]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_no_unsafe_label_parts(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    let mut previous_part: Option<&str> = None;
    for part in value.split(['-', '_']).filter(|part| !part.is_empty()) {
        if MODEL_REGISTRY_UNSAFE_LABEL_PARTS.contains(&part)
            || matches!(
                (previous_part, part),
                (Some("api"), "key") | (Some("private"), "key")
            )
        {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
        }
        previous_part = Some(part);
    }
    Ok(())
}

fn validate_supported_model_registry_source_schema(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if !MODEL_REGISTRY_AGGREGATE_SCHEMAS.contains(&value) {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_safe_event_label(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if value.len() > 128
        || value.contains('.')
        || value.contains(':')
        || value.contains('@')
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || byte == b' '
                || byte == b'-'
                || byte == b'_'
        })
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_control_plane_request_id(value: &str) -> Result<(), RuntimeControlPlaneAdapterError> {
    if value.is_empty() || value.len() > RUNTIME_CONTROL_PLANE_REQUEST_ID_MAX_BYTES {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "request_id",
        });
    }
    let lowered = value.to_ascii_lowercase();
    if value.contains('.')
        || value.contains(':')
        || value.contains('@')
        || value.contains('/')
        || value.contains('\\')
        || lowered
            .split(['-', '_'])
            .any(|part| RUNTIME_CONTROL_PLANE_REQUEST_ID_BLOCKED_PARTS.contains(&part))
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "request_id",
        });
    }
    Ok(())
}

const RUNTIME_CONTROL_PLANE_ADAPTER_ACCEPTED_SCHEMAS: &[&str] = &[
    RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
    RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION,
    RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION,
    RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
    RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
    RUNTIME_SUMMARY_SCHEMA_VERSION,
    MODEL_REGISTRY_METADATA_SCHEMA_VERSION,
];

const RUNTIME_CONTROL_PLANE_REQUEST_ID_BLOCKED_PARTS: &[&str] =
    &["private", "secret", "credential"];

const RUNTIME_SUMMARY_PROVIDER_NON_CLAIMS: &[&str] = &[
    "not_runtime_service",
    "not_persistent_storage",
    "not_event_store",
    "not_file_loader",
    "not_process_spawner",
    "not_qt_binding",
    "not_capture_boundary",
    "not_external_service",
    "not_deployment_approval",
    "not_native_runtime_execution",
];

const RUNTIME_CONTROL_PLANE_ADAPTER_NON_CLAIMS: &[&str] = &[
    "not_arbitrary_file_loader",
    "not_file_watcher",
    "not_live_transport",
    "not_socket_listener",
    "not_daemon_lifecycle",
    "not_filesystem_socket_path_policy",
    "not_process_spawner",
    "not_qt_binding",
    "not_external_service",
    "not_deployment_approval",
    "not_runtime_service",
    "not_generated_report_loader",
];

const RUNTIME_CONTROL_PLANE_FRAME_NON_CLAIMS: &[&str] = &[
    "not_network_transport",
    "not_ipc_or_socket_transport",
    "not_socket_listener",
    "not_daemon_lifecycle",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_storage_provider",
    "not_capture_boundary",
    "not_deployment_approval",
    "not_native_runtime_execution",
];

const RUNTIME_CONTROL_PLANE_IPC_NON_CLAIMS: &[&str] = &[
    "not_public_network_transport",
    "not_socket_listener",
    "not_daemon_lifecycle",
    "not_filesystem_socket_path_policy",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_storage_provider",
    "not_capture_boundary",
    "not_external_service",
    "not_deployment_approval",
    "not_native_runtime_execution",
];

const RUNTIME_CONTROL_PLANE_ENDPOINT_NON_CLAIMS: &[&str] = &[
    "not_public_network_transport",
    "not_socket_listener",
    "not_filesystem_socket_path_policy",
    "not_daemon_lifecycle",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_storage_provider",
    "not_capture_boundary",
    "not_external_service",
    "not_deployment_approval",
    "not_native_runtime_execution",
];

const RUNTIME_HANDOFF_NON_CLAIMS: &[&str] = &[
    "not_live_runtime_connection",
    "not_generated_json_loader",
    "not_control_plane_transport",
    "not_persistent_storage",
    "not_qt_runtime_integration",
    "not_model_promotion_gate",
    "not_deployment_approval",
    "not_native_runtime_execution",
];

const RUNTIME_REGISTRY_PROVIDER_NON_CLAIMS: &[&str] = &[
    "not_persistent_storage",
    "not_database_or_indexing_engine",
    "not_generated_report_loader",
    "not_generated_json_loader",
    "not_control_plane_transport",
    "not_public_network_transport",
    "not_socket_listener",
    "not_filesystem_socket_path_policy",
    "not_daemon_lifecycle",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_capture_boundary",
    "not_external_service",
    "not_deployment_approval",
    "not_native_runtime_execution",
];

const RUNTIME_REGISTRY_STORAGE_PROVIDER_NON_CLAIMS: &[&str] = &[
    "not_database_or_indexing_engine",
    "not_generated_report_loader",
    "not_generated_json_loader",
    "not_arbitrary_file_loader",
    "not_control_plane_transport",
    "not_public_network_transport",
    "not_socket_listener",
    "not_filesystem_socket_path_policy",
    "not_daemon_lifecycle",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_capture_boundary",
    "not_external_service",
    "not_deployment_approval",
    "not_native_runtime_execution",
];

const MODEL_REGISTRY_GRAPH_NOVELTY_SCHEMAS: &[&str] = &[
    "agentic_investigation_report.v0",
    "detection_candidate_report.v0",
    "model_disagreement_report.v0",
    "temporal_security_graph_report.v0",
];
const MODEL_REGISTRY_GRAPH_NOVELTY_SOURCE_NAMES: &[&str] = &[
    "agentic_investigation_report_v0_001",
    "detection_candidate_report_v0_001",
    "model_disagreement_report_v0_001",
    "temporal_security_graph_report_v0_001",
];
const MODEL_REGISTRY_AGENTIC_DETECTION_DISAGREEMENT_SCHEMAS: &[&str] = &[
    "agentic_investigation_report.v0",
    "detection_candidate_report.v0",
    "model_disagreement_report.v0",
];
const MODEL_REGISTRY_AGENTIC_DETECTION_DISAGREEMENT_SOURCE_NAMES: &[&str] = &[
    "agentic_investigation_report_v0_001",
    "detection_candidate_report_v0_001",
    "model_disagreement_report_v0_001",
];
const MODEL_REGISTRY_INVESTIGATION_SCHEMAS: &[&str] = &[
    "agentic_investigation_report.v0",
    "detection_candidate_report.v0",
];
const MODEL_REGISTRY_INVESTIGATION_SOURCE_NAMES: &[&str] = &[
    "agentic_investigation_report_v0_001",
    "detection_candidate_report_v0_001",
];
const MODEL_REGISTRY_AGENTIC_DISAGREEMENT_SCHEMAS: &[&str] = &[
    "agentic_investigation_report.v0",
    "model_disagreement_report.v0",
];
const MODEL_REGISTRY_AGENTIC_DISAGREEMENT_SOURCE_NAMES: &[&str] = &[
    "agentic_investigation_report_v0_001",
    "model_disagreement_report_v0_001",
];
const MODEL_REGISTRY_REPRESENTATION_SCHEMAS: &[&str] = &[
    "agentic_investigation_report.v0",
    "traffic_representation_report.v0",
];
const MODEL_REGISTRY_REPRESENTATION_SOURCE_NAMES: &[&str] = &[
    "agentic_investigation_report_v0_001",
    "traffic_representation_report_v0_001",
];
const MODEL_REGISTRY_NATIVE_SCORE_SCHEMAS: &[&str] = &["model_score_rows.v0"];
const MODEL_REGISTRY_NATIVE_SCORE_SOURCE_NAMES: &[&str] = &["model_score_rows_v0_001"];
const MODEL_REGISTRY_TIME_SERIES_SCHEMAS: &[&str] = &[
    "agentic_investigation_report.v0",
    "detection_candidate_report.v0",
    "model_disagreement_report.v0",
    "time_series_residual_report.v0",
];
const MODEL_REGISTRY_TIME_SERIES_SOURCE_NAMES: &[&str] = &[
    "agentic_investigation_report_v0_001",
    "detection_candidate_report_v0_001",
    "model_disagreement_report_v0_001",
    "time_series_residual_report_v0_001",
];
const MODEL_REGISTRY_AGGREGATE_SCHEMAS: &[&str] = &[
    "agentic_investigation_report.v0",
    "detection_candidate_report.v0",
    "model_disagreement_report.v0",
    "model_score_rows.v0",
    "temporal_security_graph_report.v0",
    "time_series_residual_report.v0",
    "traffic_representation_report.v0",
];
const MODEL_REGISTRY_MODELS_WITH_SCORE_ROWS: &[&str] = &[
    "graph_novelty",
    "isolation_forest",
    "pyod_copod",
    "pyod_ecod",
    "river_hst",
    "stdlib_linear_native",
    "suricata_alert",
    "time_series_residual",
];
const MODEL_REGISTRY_NON_CLAIMS: &[&str] = &[
    "not_persistent_model_registry",
    "not_model_promotion_gate",
    "not_deployment_approval",
    "not_live_capture",
    "not_external_enrichment",
    "not_rule_deployment",
    "not_native_runtime_execution",
];

const MODEL_REGISTRY_UNSAFE_LABEL_PARTS: &[&str] =
    &["password", "passwd", "credential", "secret", "apikey"];

const MODEL_REGISTRY_METADATA_ADAPTER_NON_CLAIMS: &[&str] = &[
    "not_persistent_model_registry",
    "not_storage_provider",
    "not_model_promotion_gate",
    "not_deployment_approval",
    "not_generated_report_loader",
    "not_qt_binding",
    "not_capture_boundary",
    "not_external_service",
    "not_native_runtime_execution",
];

fn model_registry_metadata_entries() -> Vec<ModelRegistryEntry> {
    vec![
        ModelRegistryEntry {
            model_id: "graph_novelty".to_owned(),
            registry_state: ModelRegistryState::ObservedSyntheticOnly,
            promotion_state: ModelPromotionState::NotPromoted,
            observed_source_schemas: static_str_vec(MODEL_REGISTRY_GRAPH_NOVELTY_SCHEMAS),
            observed_source_names: static_str_vec(MODEL_REGISTRY_GRAPH_NOVELTY_SOURCE_NAMES),
            source_count: 4,
            has_score_rows: true,
            human_review_required: true,
            deployment_allowed: false,
        },
        ModelRegistryEntry {
            model_id: "isolation_forest".to_owned(),
            registry_state: ModelRegistryState::ObservedSyntheticOnly,
            promotion_state: ModelPromotionState::NotPromoted,
            observed_source_schemas: static_str_vec(
                MODEL_REGISTRY_AGENTIC_DETECTION_DISAGREEMENT_SCHEMAS,
            ),
            observed_source_names: static_str_vec(
                MODEL_REGISTRY_AGENTIC_DETECTION_DISAGREEMENT_SOURCE_NAMES,
            ),
            source_count: 3,
            has_score_rows: true,
            human_review_required: true,
            deployment_allowed: false,
        },
        ModelRegistryEntry {
            model_id: "model_disagreement".to_owned(),
            registry_state: ModelRegistryState::ObservedSyntheticOnly,
            promotion_state: ModelPromotionState::NotPromoted,
            observed_source_schemas: static_str_vec(MODEL_REGISTRY_INVESTIGATION_SCHEMAS),
            observed_source_names: static_str_vec(MODEL_REGISTRY_INVESTIGATION_SOURCE_NAMES),
            source_count: 2,
            has_score_rows: false,
            human_review_required: true,
            deployment_allowed: false,
        },
        ModelRegistryEntry {
            model_id: "pyod_copod".to_owned(),
            registry_state: ModelRegistryState::ObservedSyntheticOnly,
            promotion_state: ModelPromotionState::NotPromoted,
            observed_source_schemas: static_str_vec(MODEL_REGISTRY_AGENTIC_DISAGREEMENT_SCHEMAS),
            observed_source_names: static_str_vec(MODEL_REGISTRY_AGENTIC_DISAGREEMENT_SOURCE_NAMES),
            source_count: 2,
            has_score_rows: true,
            human_review_required: true,
            deployment_allowed: false,
        },
        ModelRegistryEntry {
            model_id: "pyod_ecod".to_owned(),
            registry_state: ModelRegistryState::ObservedSyntheticOnly,
            promotion_state: ModelPromotionState::NotPromoted,
            observed_source_schemas: static_str_vec(
                MODEL_REGISTRY_AGENTIC_DETECTION_DISAGREEMENT_SCHEMAS,
            ),
            observed_source_names: static_str_vec(
                MODEL_REGISTRY_AGENTIC_DETECTION_DISAGREEMENT_SOURCE_NAMES,
            ),
            source_count: 3,
            has_score_rows: true,
            human_review_required: true,
            deployment_allowed: false,
        },
        ModelRegistryEntry {
            model_id: "river_hst".to_owned(),
            registry_state: ModelRegistryState::ObservedSyntheticOnly,
            promotion_state: ModelPromotionState::NotPromoted,
            observed_source_schemas: static_str_vec(
                MODEL_REGISTRY_AGENTIC_DETECTION_DISAGREEMENT_SCHEMAS,
            ),
            observed_source_names: static_str_vec(
                MODEL_REGISTRY_AGENTIC_DETECTION_DISAGREEMENT_SOURCE_NAMES,
            ),
            source_count: 3,
            has_score_rows: true,
            human_review_required: true,
            deployment_allowed: false,
        },
        ModelRegistryEntry {
            model_id: "self_supervised_representation".to_owned(),
            registry_state: ModelRegistryState::ObservedSyntheticOnly,
            promotion_state: ModelPromotionState::NotPromoted,
            observed_source_schemas: static_str_vec(MODEL_REGISTRY_REPRESENTATION_SCHEMAS),
            observed_source_names: static_str_vec(MODEL_REGISTRY_REPRESENTATION_SOURCE_NAMES),
            source_count: 2,
            has_score_rows: false,
            human_review_required: true,
            deployment_allowed: false,
        },
        ModelRegistryEntry {
            model_id: "stdlib_linear_native".to_owned(),
            registry_state: ModelRegistryState::ObservedSyntheticOnly,
            promotion_state: ModelPromotionState::NotPromoted,
            observed_source_schemas: static_str_vec(MODEL_REGISTRY_NATIVE_SCORE_SCHEMAS),
            observed_source_names: static_str_vec(MODEL_REGISTRY_NATIVE_SCORE_SOURCE_NAMES),
            source_count: 1,
            has_score_rows: true,
            human_review_required: true,
            deployment_allowed: false,
        },
        ModelRegistryEntry {
            model_id: "suricata_alert".to_owned(),
            registry_state: ModelRegistryState::ObservedSyntheticOnly,
            promotion_state: ModelPromotionState::NotPromoted,
            observed_source_schemas: static_str_vec(MODEL_REGISTRY_AGENTIC_DISAGREEMENT_SCHEMAS),
            observed_source_names: static_str_vec(MODEL_REGISTRY_AGENTIC_DISAGREEMENT_SOURCE_NAMES),
            source_count: 2,
            has_score_rows: true,
            human_review_required: true,
            deployment_allowed: false,
        },
        ModelRegistryEntry {
            model_id: "time_series_residual".to_owned(),
            registry_state: ModelRegistryState::ObservedSyntheticOnly,
            promotion_state: ModelPromotionState::NotPromoted,
            observed_source_schemas: static_str_vec(MODEL_REGISTRY_TIME_SERIES_SCHEMAS),
            observed_source_names: static_str_vec(MODEL_REGISTRY_TIME_SERIES_SOURCE_NAMES),
            source_count: 4,
            has_score_rows: true,
            human_review_required: true,
            deployment_allowed: false,
        },
    ]
}

fn static_str_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn validate_coarse_id(value: &str, allowed_prefixes: &[&str]) -> Result<(), RuntimeIdError> {
    if value.is_empty() {
        return Err(RuntimeIdError::Empty);
    }
    if value.len() > 96 {
        return Err(RuntimeIdError::TooLong);
    }
    if !allowed_prefixes
        .iter()
        .any(|allowed_prefix| value.starts_with(allowed_prefix))
    {
        return Err(RuntimeIdError::InvalidPrefix);
    }
    if value.contains('.') || value.contains(':') || value.contains('@') {
        return Err(RuntimeIdError::RawIdentifier);
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
    }) {
        return Err(RuntimeIdError::InvalidCharacter);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::ffi::CString;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(unix)]
    use std::os::unix::net::UnixStream;

    #[cfg(unix)]
    unsafe extern "C" {
        fn geteuid() -> u32;
        fn mkfifo(path: *const std::os::raw::c_char, mode: u32) -> i32;
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn synthetic_handoff_json() -> &'static str {
        r#"{
  "schema_version": "runtime_handoff_snapshot.v0",
  "source_kind": "static_synthetic_fixture",
  "transport_state": "unavailable",
  "control_plane_state": "unavailable",
  "runtime_summary": {
    "schema_version": "runtime_summary.v0",
    "workspace_id": "fixture-workspace-alpha",
    "session_id": "fixture-session-runtime-summary",
    "total_job_count": 4,
    "queued_job_count": 1,
    "running_job_count": 1,
    "failed_job_count": 0,
    "last_event_label": "synthetic workstation snapshot rendered",
    "native_inference_state": "disabled"
  },
  "model_registry_metadata": {
    "schema_version": "model_registry_metadata.v0",
    "metadata_scope": "local_synthetic_model_registry_metadata",
    "source_bundle_schema": "model_evaluation_bundle.v0",
    "entries": [
      {
        "model_id": "graph_novelty",
        "registry_state": "observed_synthetic_only",
        "promotion_state": "not_promoted",
        "observed_source_schemas": [
          "agentic_investigation_report.v0",
          "detection_candidate_report.v0",
          "model_disagreement_report.v0",
          "temporal_security_graph_report.v0"
        ],
        "observed_source_names": [
          "agentic_investigation_report_v0_001",
          "detection_candidate_report_v0_001",
          "model_disagreement_report_v0_001",
          "temporal_security_graph_report_v0_001"
        ],
        "source_count": 4,
        "has_score_rows": true,
        "human_review_required": true,
        "deployment_allowed": false
      },
      {
        "model_id": "isolation_forest",
        "registry_state": "observed_synthetic_only",
        "promotion_state": "not_promoted",
        "observed_source_schemas": [
          "agentic_investigation_report.v0",
          "detection_candidate_report.v0",
          "model_disagreement_report.v0"
        ],
        "observed_source_names": [
          "agentic_investigation_report_v0_001",
          "detection_candidate_report_v0_001",
          "model_disagreement_report_v0_001"
        ],
        "source_count": 3,
        "has_score_rows": true,
        "human_review_required": true,
        "deployment_allowed": false
      },
      {
        "model_id": "model_disagreement",
        "registry_state": "observed_synthetic_only",
        "promotion_state": "not_promoted",
        "observed_source_schemas": [
          "agentic_investigation_report.v0",
          "detection_candidate_report.v0"
        ],
        "observed_source_names": [
          "agentic_investigation_report_v0_001",
          "detection_candidate_report_v0_001"
        ],
        "source_count": 2,
        "has_score_rows": false,
        "human_review_required": true,
        "deployment_allowed": false
      },
      {
        "model_id": "pyod_copod",
        "registry_state": "observed_synthetic_only",
        "promotion_state": "not_promoted",
        "observed_source_schemas": [
          "agentic_investigation_report.v0",
          "model_disagreement_report.v0"
        ],
        "observed_source_names": [
          "agentic_investigation_report_v0_001",
          "model_disagreement_report_v0_001"
        ],
        "source_count": 2,
        "has_score_rows": true,
        "human_review_required": true,
        "deployment_allowed": false
      },
      {
        "model_id": "pyod_ecod",
        "registry_state": "observed_synthetic_only",
        "promotion_state": "not_promoted",
        "observed_source_schemas": [
          "agentic_investigation_report.v0",
          "detection_candidate_report.v0",
          "model_disagreement_report.v0"
        ],
        "observed_source_names": [
          "agentic_investigation_report_v0_001",
          "detection_candidate_report_v0_001",
          "model_disagreement_report_v0_001"
        ],
        "source_count": 3,
        "has_score_rows": true,
        "human_review_required": true,
        "deployment_allowed": false
      },
      {
        "model_id": "river_hst",
        "registry_state": "observed_synthetic_only",
        "promotion_state": "not_promoted",
        "observed_source_schemas": [
          "agentic_investigation_report.v0",
          "detection_candidate_report.v0",
          "model_disagreement_report.v0"
        ],
        "observed_source_names": [
          "agentic_investigation_report_v0_001",
          "detection_candidate_report_v0_001",
          "model_disagreement_report_v0_001"
        ],
        "source_count": 3,
        "has_score_rows": true,
        "human_review_required": true,
        "deployment_allowed": false
      },
      {
        "model_id": "self_supervised_representation",
        "registry_state": "observed_synthetic_only",
        "promotion_state": "not_promoted",
        "observed_source_schemas": [
          "agentic_investigation_report.v0",
          "traffic_representation_report.v0"
        ],
        "observed_source_names": [
          "agentic_investigation_report_v0_001",
          "traffic_representation_report_v0_001"
        ],
        "source_count": 2,
        "has_score_rows": false,
        "human_review_required": true,
        "deployment_allowed": false
      },
      {
        "model_id": "stdlib_linear_native",
        "registry_state": "observed_synthetic_only",
        "promotion_state": "not_promoted",
        "observed_source_schemas": [
          "model_score_rows.v0"
        ],
        "observed_source_names": [
          "model_score_rows_v0_001"
        ],
        "source_count": 1,
        "has_score_rows": true,
        "human_review_required": true,
        "deployment_allowed": false
      },
      {
        "model_id": "suricata_alert",
        "registry_state": "observed_synthetic_only",
        "promotion_state": "not_promoted",
        "observed_source_schemas": [
          "agentic_investigation_report.v0",
          "model_disagreement_report.v0"
        ],
        "observed_source_names": [
          "agentic_investigation_report_v0_001",
          "model_disagreement_report_v0_001"
        ],
        "source_count": 2,
        "has_score_rows": true,
        "human_review_required": true,
        "deployment_allowed": false
      },
      {
        "model_id": "time_series_residual",
        "registry_state": "observed_synthetic_only",
        "promotion_state": "not_promoted",
        "observed_source_schemas": [
          "agentic_investigation_report.v0",
          "detection_candidate_report.v0",
          "model_disagreement_report.v0",
          "time_series_residual_report.v0"
        ],
        "observed_source_names": [
          "agentic_investigation_report_v0_001",
          "detection_candidate_report_v0_001",
          "model_disagreement_report_v0_001",
          "time_series_residual_report_v0_001"
        ],
        "source_count": 4,
        "has_score_rows": true,
        "human_review_required": true,
        "deployment_allowed": false
      }
    ],
    "aggregate_summary": {
      "model_count": 10,
      "schemas_present": [
        "agentic_investigation_report.v0",
        "detection_candidate_report.v0",
        "model_disagreement_report.v0",
        "model_score_rows.v0",
        "temporal_security_graph_report.v0",
        "time_series_residual_report.v0",
        "traffic_representation_report.v0"
      ],
      "models_with_score_rows": [
        "graph_novelty",
        "isolation_forest",
        "pyod_copod",
        "pyod_ecod",
        "river_hst",
        "stdlib_linear_native",
        "suricata_alert",
        "time_series_residual"
      ],
      "deployment_allowed": false
    },
    "safety_flags": {
      "local_only": true,
      "strict_json_loaded": true,
      "derived_from_evaluation_bundle_only": true,
      "input_paths_copied": false,
      "source_filenames_copied": false,
      "raw_identifiers_copied": false,
      "generated_artifact_references_copied": false,
      "secrets_detected": false,
      "report_payload_copied": false,
      "live_capture_used": false,
      "external_services_used": false,
      "deployment_allowed": false
    },
    "non_claims": [
      "not_persistent_model_registry",
      "not_model_promotion_gate",
      "not_deployment_approval",
      "not_live_capture",
      "not_external_enrichment",
      "not_rule_deployment",
      "not_native_runtime_execution"
    ]
  },
  "local_only": true,
  "static_synthetic_fixture": true,
  "generated_json_loaded": false,
  "live_runtime_connection": false,
  "external_services_used": false,
  "deployment_allowed": false,
  "non_claims": [
    "not_live_runtime_connection",
    "not_generated_json_loader",
    "not_control_plane_transport",
    "not_persistent_storage",
    "not_qt_runtime_integration",
    "not_model_promotion_gate",
    "not_deployment_approval",
    "not_native_runtime_execution"
  ]
}"#
    }

    fn patched_json(target: &str, replacement: &str) -> String {
        synthetic_handoff_json().replacen(target, replacement, 1)
    }

    fn synthetic_model_registry_metadata_json() -> String {
        serde_json::to_string_pretty(&ModelRegistryMetadata::synthetic_fixture())
            .expect("synthetic metadata fixture must serialize")
    }

    fn three_model_registry_metadata_fixture() -> ModelRegistryMetadata {
        ModelRegistryMetadata {
            schema_version: MODEL_REGISTRY_METADATA_SCHEMA_VERSION.to_owned(),
            metadata_scope: MODEL_REGISTRY_METADATA_SCOPE.to_owned(),
            source_bundle_schema: MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION.to_owned(),
            entries: vec![
                ModelRegistryEntry {
                    model_id: "isolation_forest".to_owned(),
                    registry_state: ModelRegistryState::ObservedSyntheticOnly,
                    promotion_state: ModelPromotionState::NotPromoted,
                    observed_source_schemas: strings(&["model_disagreement_report.v0"]),
                    observed_source_names: strings(&["model_disagreement_report_v0_001"]),
                    source_count: 1,
                    has_score_rows: true,
                    human_review_required: true,
                    deployment_allowed: false,
                },
                ModelRegistryEntry {
                    model_id: "pyod_ecod".to_owned(),
                    registry_state: ModelRegistryState::ObservedSyntheticOnly,
                    promotion_state: ModelPromotionState::NotPromoted,
                    observed_source_schemas: strings(&["model_disagreement_report.v0"]),
                    observed_source_names: strings(&["model_disagreement_report_v0_001"]),
                    source_count: 1,
                    has_score_rows: true,
                    human_review_required: true,
                    deployment_allowed: false,
                },
                ModelRegistryEntry {
                    model_id: "stdlib_linear_native".to_owned(),
                    registry_state: ModelRegistryState::ObservedSyntheticOnly,
                    promotion_state: ModelPromotionState::NotPromoted,
                    observed_source_schemas: strings(&["model_score_rows.v0"]),
                    observed_source_names: strings(&["model_score_rows_v0_001"]),
                    source_count: 1,
                    has_score_rows: true,
                    human_review_required: true,
                    deployment_allowed: false,
                },
            ],
            aggregate_summary: ModelRegistryAggregateSummary {
                model_count: 3,
                schemas_present: strings(&["model_disagreement_report.v0", "model_score_rows.v0"]),
                models_with_score_rows: strings(&[
                    "isolation_forest",
                    "pyod_ecod",
                    "stdlib_linear_native",
                ]),
                deployment_allowed: false,
            },
            safety_flags: ModelRegistrySafetyFlags {
                local_only: true,
                strict_json_loaded: true,
                derived_from_evaluation_bundle_only: true,
                input_paths_copied: false,
                source_filenames_copied: false,
                raw_identifiers_copied: false,
                generated_artifact_references_copied: false,
                secrets_detected: false,
                report_payload_copied: false,
                live_capture_used: false,
                external_services_used: false,
                deployment_allowed: false,
            },
            non_claims: static_str_vec(MODEL_REGISTRY_NON_CLAIMS),
        }
    }

    fn three_model_registry_metadata_json() -> String {
        serde_json::to_string_pretty(&three_model_registry_metadata_fixture())
            .expect("three-model metadata fixture must serialize")
    }

    fn secret_model_registry_metadata_json() -> String {
        let mut metadata = three_model_registry_metadata_fixture();
        metadata.entries[0].model_id = "secret".to_owned();
        metadata.aggregate_summary.models_with_score_rows[0] = "secret".to_owned();
        serde_json::to_string_pretty(&metadata)
            .expect("secret-like metadata fixture must serialize")
    }

    fn patched_metadata_json(target: &str, replacement: &str) -> String {
        synthetic_model_registry_metadata_json().replacen(target, replacement, 1)
    }

    fn temp_policy_root(name: &str) -> PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock must produce a temp path suffix")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("ares-rust-core-{name}-{suffix}"));
        std::fs::create_dir_all(&root).expect("test temp root must be created");
        root
    }

    fn write_test_file(root: &Path, file_name: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = root.join(file_name);
        std::fs::write(&path, contents).expect("test handoff file must be written");
        path
    }

    fn execute_json_command(
        input: impl Into<String>,
    ) -> Result<RuntimeHandoffSnapshot, RuntimeControlPlaneAdapterError> {
        RuntimeControlPlaneAdapterContract::execute_local_command(
            RuntimeControlPlaneCommand::parse_handoff_snapshot_json(input),
        )
    }

    fn execute_file_command(
        path: impl Into<PathBuf>,
        policy: &RuntimeControlPlaneFilePolicy,
    ) -> Result<RuntimeHandoffSnapshot, RuntimeControlPlaneAdapterError> {
        RuntimeControlPlaneAdapterContract::execute_local_command(
            RuntimeControlPlaneCommand::parse_handoff_snapshot_file(path, policy.clone()),
        )
    }

    fn json_message_request(request_id: &str, input: &str) -> String {
        format!(
            r#"{{
  "schema_version": "{schema_version}",
  "request_id": {request_id},
  "command": {{
    "command_kind": "parse_handoff_snapshot_json",
    "input": {input}
  }}
}}"#,
            schema_version = RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            request_id = serde_json::to_string(request_id).unwrap(),
            input = serde_json::to_string(input).unwrap()
        )
    }

    fn file_message_request(request_id: &str, path: &Path, allowed_root: &Path) -> String {
        format!(
            r#"{{
  "schema_version": "{schema_version}",
  "request_id": {request_id},
  "command": {{
    "command_kind": "parse_handoff_snapshot_file",
    "path": {path},
    "policy": {{
      "allowed_root": {allowed_root}
    }}
  }}
}}"#,
            schema_version = RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            request_id = serde_json::to_string(request_id).unwrap(),
            path = serde_json::to_string(path.to_str().unwrap()).unwrap(),
            allowed_root = serde_json::to_string(allowed_root.to_str().unwrap()).unwrap()
        )
    }

    fn response_from_frame_bytes(frame: Vec<u8>) -> RuntimeControlPlaneMessageResponse {
        let response_json = String::from_utf8(frame).expect("response frame must be UTF-8 JSON");
        serde_json::from_str(&response_json).expect("response frame must parse")
    }

    fn ipc_frame_bytes(frame: &[u8]) -> Vec<u8> {
        let mut ipc_frame = u32::try_from(frame.len())
            .expect("test frame length must fit in the IPC prefix")
            .to_be_bytes()
            .to_vec();
        ipc_frame.extend_from_slice(frame);
        ipc_frame
    }

    fn ipc_response_payload(ipc_frame: &[u8]) -> &[u8] {
        assert!(ipc_frame.len() >= RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES);
        let length_prefix: [u8; RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES] = ipc_frame
            [..RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES]
            .try_into()
            .expect("IPC response prefix must be four bytes");
        let frame_len = u32::from_be_bytes(length_prefix) as usize;
        let payload_start = RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES;
        let payload_end = payload_start + frame_len;
        assert_eq!(ipc_frame.len(), payload_end);
        &ipc_frame[payload_start..payload_end]
    }

    fn response_from_ipc_bytes(ipc_frame: &[u8]) -> RuntimeControlPlaneMessageResponse {
        let response_json = String::from_utf8(ipc_response_payload(ipc_frame).to_vec()).unwrap();
        serde_json::from_str(&response_json).expect("IPC response frame must parse")
    }

    fn registry_handoff_fixture(
        workspace_id: &str,
        session_id: &str,
        total_job_count: u32,
        last_event_label: &str,
    ) -> RuntimeHandoffSnapshot {
        let mut snapshot = RuntimeHandoffSnapshot::synthetic_fixture();
        snapshot.runtime_summary.workspace_id =
            WorkspaceId::new(workspace_id).expect("registry fixture workspace id must be valid");
        snapshot.runtime_summary.session_id =
            SessionId::new(session_id).expect("registry fixture session id must be valid");
        snapshot.runtime_summary.total_job_count = total_job_count;
        snapshot.runtime_summary.queued_job_count = 0;
        snapshot.runtime_summary.running_job_count = 0;
        snapshot.runtime_summary.failed_job_count = 0;
        snapshot.runtime_summary.last_event_label = last_event_label.to_owned();
        snapshot
    }

    fn runtime_registry_snapshot_fixture() -> RuntimeRegistrySnapshot {
        let mut provider = RuntimeRegistryProvider::default();
        provider
            .upsert_snapshot(registry_handoff_fixture(
                "workspace-storage-beta",
                "session-storage-beta",
                2,
                "beta storage snapshot ready",
            ))
            .unwrap();
        provider
            .upsert_snapshot(registry_handoff_fixture(
                "workspace-storage-alpha",
                "session-storage-alpha",
                1,
                "alpha storage snapshot ready",
            ))
            .unwrap();
        provider.snapshot()
    }

    fn runtime_registry_storage_document_fixture() -> RuntimeRegistryStorageDocument {
        RuntimeRegistryStorageDocument::from_snapshot(runtime_registry_snapshot_fixture())
            .expect("storage document fixture must be valid")
    }

    fn runtime_registry_storage_document_json() -> String {
        serde_json::to_string_pretty(&runtime_registry_storage_document_fixture())
            .expect("storage document fixture must serialize")
    }

    fn execute_ipc_frame_bytes(
        frame: &[u8],
    ) -> (Result<(), RuntimeControlPlaneAdapterError>, Vec<u8>) {
        execute_ipc_frame_bytes_with_policy(frame, &RuntimeControlPlaneIpcPolicy::default())
    }

    fn execute_ipc_frame_bytes_with_policy(
        frame: &[u8],
        policy: &RuntimeControlPlaneIpcPolicy,
    ) -> (Result<(), RuntimeControlPlaneAdapterError>, Vec<u8>) {
        let input = ipc_frame_bytes(frame);
        let mut reader = input.as_slice();
        let mut writer = Vec::new();
        let result = execute_control_plane_message_ipc_stream(&mut reader, &mut writer, policy);
        (result, writer)
    }

    fn execute_endpoint_frame_bytes(
        frame: &[u8],
    ) -> (Result<(), RuntimeControlPlaneAdapterError>, Vec<u8>) {
        execute_endpoint_frame_bytes_with_policy(
            frame,
            &RuntimeControlPlaneEndpointPolicy::default(),
        )
    }

    fn execute_endpoint_frame_bytes_with_policy(
        frame: &[u8],
        policy: &RuntimeControlPlaneEndpointPolicy,
    ) -> (Result<(), RuntimeControlPlaneAdapterError>, Vec<u8>) {
        let input = ipc_frame_bytes(frame);
        let mut reader = input.as_slice();
        let mut writer = Vec::new();
        let result = execute_control_plane_endpoint_stream(&mut reader, &mut writer, policy);
        (result, writer)
    }

    fn remove_temp_root(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    fn make_fifo(path: &Path) {
        let c_path =
            CString::new(path.as_os_str().as_bytes()).expect("test path must be a C string");
        let result = unsafe { mkfifo(c_path.as_ptr(), 0o600) };
        assert_eq!(result, 0, "test fifo must be created");
    }

    #[cfg(unix)]
    fn effective_user_id_is_root() -> bool {
        unsafe { geteuid() == 0 }
    }

    #[cfg(unix)]
    fn unix_stream_pair_writes_are_permitted() -> bool {
        let (mut client, _server) =
            UnixStream::pair().expect("test connected stream pair must be created");
        match client.write(&[0]) {
            Ok(_) => true,
            Err(error) if error.raw_os_error() == Some(1) => false,
            Err(error) => panic!("test UnixStream probe failed unexpectedly: {error}"),
        }
    }

    #[test]
    fn accepts_coarse_runtime_identifiers() {
        let workspace = WorkspaceId::new("workspace-alpha").unwrap();
        let session = SessionId::new("session-alpha").unwrap();
        let job = JobId::new("job-alpha").unwrap();

        assert_eq!(workspace.as_str(), "workspace-alpha");
        assert_eq!(session.as_str(), "session-alpha");
        assert_eq!(job.as_str(), "job-alpha");
    }

    #[test]
    fn rejects_raw_identifier_shapes() {
        assert_eq!(
            WorkspaceId::new("workspace-alpha.raw").unwrap_err(),
            RuntimeIdError::RawIdentifier
        );
        assert_eq!(
            SessionId::new("session-alpha:01").unwrap_err(),
            RuntimeIdError::RawIdentifier
        );
        assert_eq!(
            JobId::new("job-alpha@example").unwrap_err(),
            RuntimeIdError::RawIdentifier
        );
    }

    #[test]
    fn emits_static_runtime_summary_fixture() {
        let summary = RuntimeSummary::synthetic_fixture();

        assert_eq!(summary.schema_version, RUNTIME_SUMMARY_SCHEMA_VERSION);
        assert_eq!(summary.workspace_id.as_str(), "fixture-workspace-alpha");
        assert_eq!(
            summary.session_id.as_str(),
            "fixture-session-runtime-summary"
        );
        assert_eq!(summary.total_job_count, 4);
        assert_eq!(summary.queued_job_count, 1);
        assert_eq!(summary.running_job_count, 1);
        assert_eq!(summary.failed_job_count, 0);
        assert_eq!(
            summary.last_event_label,
            "synthetic workstation snapshot rendered"
        );
        assert_eq!(summary.native_inference_state.as_str(), "disabled");
    }

    #[test]
    fn exposes_runtime_summary_provider_contract_fixture() {
        let contract = RuntimeSummaryProviderContract::synthetic_fixture();
        let policy = RuntimeSummaryProviderPolicy::new();

        assert_eq!(
            contract.schema_version,
            RUNTIME_SUMMARY_PROVIDER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.output_summary_schema,
            RUNTIME_SUMMARY_SCHEMA_VERSION
        );
        assert!(contract.local_only);
        assert!(contract.caller_provided_events_only);
        assert!(contract.event_replay_enabled);
        assert!(!contract.storage_provider_enabled);
        assert!(!contract.live_runtime_connection_enabled);
        assert!(!contract.file_io_enabled);
        assert!(!contract.process_spawning_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert!(contract.non_claims.contains(&"not_persistent_storage"));
        assert!(contract.non_claims.contains(&"not_event_store"));
        assert!(contract
            .non_claims
            .contains(&"not_native_runtime_execution"));
        assert_eq!(policy, RuntimeSummaryProviderPolicy::default());
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn builds_runtime_summary_from_caller_provided_events() {
        let workspace_id =
            WorkspaceId::new("workspace-runtime-provider").expect("workspace id must be valid");
        let session_id =
            SessionId::new("session-runtime-provider").expect("session id must be valid");
        let job_running = JobId::new("job-runtime-running").expect("job id must be valid");
        let job_failed = JobId::new("job-runtime-failed").expect("job id must be valid");
        let job_queued = JobId::new("job-runtime-queued").expect("job id must be valid");
        let job_succeeded = JobId::new("job-runtime-succeeded").expect("job id must be valid");
        let job_cancelled = JobId::new("job-runtime-cancelled").expect("job id must be valid");
        let events = vec![
            RuntimeEvent::WorkspaceOpened {
                workspace_id: workspace_id.clone(),
            },
            RuntimeEvent::SessionStarted {
                workspace_id: workspace_id.clone(),
                session_id: session_id.clone(),
            },
            RuntimeEvent::JobQueued {
                session_id: session_id.clone(),
                job_id: job_running.clone(),
                kind: JobKind::CompareModelScores,
            },
            RuntimeEvent::JobStateChanged {
                job_id: job_running,
                state: JobState::Running,
            },
            RuntimeEvent::JobQueued {
                session_id: session_id.clone(),
                job_id: job_failed.clone(),
                kind: JobKind::RefreshEvidenceIndex,
            },
            RuntimeEvent::JobStateChanged {
                job_id: job_failed,
                state: JobState::Failed,
            },
            RuntimeEvent::JobQueued {
                session_id: session_id.clone(),
                job_id: job_queued,
                kind: JobKind::RenderWorkstationSnapshot,
            },
            RuntimeEvent::JobQueued {
                session_id: session_id.clone(),
                job_id: job_succeeded.clone(),
                kind: JobKind::RunNativeInferenceCandidate,
            },
            RuntimeEvent::JobStateChanged {
                job_id: job_succeeded,
                state: JobState::Succeeded,
            },
            RuntimeEvent::JobQueued {
                session_id: session_id.clone(),
                job_id: job_cancelled.clone(),
                kind: JobKind::RenderWorkstationSnapshot,
            },
            RuntimeEvent::JobStateChanged {
                job_id: job_cancelled,
                state: JobState::Cancelled,
            },
        ];

        let summary = build_runtime_summary_from_events(
            workspace_id.clone(),
            session_id.clone(),
            &events,
            NativeInferenceRuntimeState::Available,
            &RuntimeSummaryProviderPolicy::new(),
        )
        .expect("caller-provided events must build a runtime summary");

        assert_eq!(summary.schema_version, RUNTIME_SUMMARY_SCHEMA_VERSION);
        assert_eq!(summary.workspace_id, workspace_id);
        assert_eq!(summary.session_id, session_id);
        assert_eq!(summary.total_job_count, 5);
        assert_eq!(summary.queued_job_count, 1);
        assert_eq!(summary.running_job_count, 1);
        assert_eq!(summary.failed_job_count, 1);
        assert_eq!(summary.last_event_label, "job cancelled");
        assert_eq!(summary.native_inference_state.as_str(), "available");
    }

    #[test]
    fn provider_contract_builds_runtime_summary_from_events() {
        let workspace_id =
            WorkspaceId::new("workspace-contract-provider").expect("workspace id must be valid");
        let session_id =
            SessionId::new("session-contract-provider").expect("session id must be valid");
        let job_id = JobId::new("job-contract-provider").expect("job id must be valid");
        let events = vec![
            RuntimeEvent::WorkspaceOpened {
                workspace_id: workspace_id.clone(),
            },
            RuntimeEvent::SessionStarted {
                workspace_id: workspace_id.clone(),
                session_id: session_id.clone(),
            },
            RuntimeEvent::JobQueued {
                session_id: session_id.clone(),
                job_id,
                kind: JobKind::RenderWorkstationSnapshot,
            },
        ];

        let summary = RuntimeSummaryProviderContract::build_runtime_summary_from_events(
            workspace_id,
            session_id,
            &events,
            NativeInferenceRuntimeState::Disabled,
            &RuntimeSummaryProviderPolicy::new(),
        )
        .expect("contract wrapper must delegate to provider");

        assert_eq!(summary.total_job_count, 1);
        assert_eq!(summary.queued_job_count, 1);
        assert_eq!(summary.running_job_count, 0);
        assert_eq!(summary.failed_job_count, 0);
        assert_eq!(summary.last_event_label, "workstation snapshot job queued");
    }

    #[test]
    fn rejects_empty_runtime_summary_provider_events() {
        let err = build_runtime_summary_from_events(
            WorkspaceId::new("workspace-empty-provider").unwrap(),
            SessionId::new("session-empty-provider").unwrap(),
            &[],
            NativeInferenceRuntimeState::Disabled,
            &RuntimeSummaryProviderPolicy::new(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_summary_provider.events"
            }
        );
    }

    #[test]
    fn rejects_mismatched_runtime_summary_provider_workspace_or_session() {
        let workspace_id = WorkspaceId::new("workspace-provider-alpha").unwrap();
        let session_id = SessionId::new("session-provider-alpha").unwrap();
        let wrong_workspace = WorkspaceId::new("workspace-provider-beta").unwrap();
        let wrong_session = SessionId::new("session-provider-beta").unwrap();

        let workspace_err = build_runtime_summary_from_events(
            workspace_id.clone(),
            session_id.clone(),
            &[RuntimeEvent::WorkspaceOpened {
                workspace_id: wrong_workspace,
            }],
            NativeInferenceRuntimeState::Disabled,
            &RuntimeSummaryProviderPolicy::new(),
        )
        .unwrap_err();
        assert_eq!(
            workspace_err,
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_summary_provider.workspace_id"
            }
        );

        let session_err = build_runtime_summary_from_events(
            workspace_id.clone(),
            session_id,
            &[RuntimeEvent::SessionStarted {
                workspace_id,
                session_id: wrong_session,
            }],
            NativeInferenceRuntimeState::Disabled,
            &RuntimeSummaryProviderPolicy::new(),
        )
        .unwrap_err();
        assert_eq!(
            session_err,
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_summary_provider.session_id"
            }
        );
    }

    #[test]
    fn rejects_unknown_or_duplicate_runtime_summary_provider_jobs() {
        let workspace_id = WorkspaceId::new("workspace-provider-jobs").unwrap();
        let session_id = SessionId::new("session-provider-jobs").unwrap();
        let job_id = JobId::new("job-provider-duplicate").unwrap();

        let unknown_err = build_runtime_summary_from_events(
            workspace_id.clone(),
            session_id.clone(),
            &[RuntimeEvent::JobStateChanged {
                job_id: job_id.clone(),
                state: JobState::Running,
            }],
            NativeInferenceRuntimeState::Disabled,
            &RuntimeSummaryProviderPolicy::new(),
        )
        .unwrap_err();
        assert_eq!(
            unknown_err,
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_summary_provider.unknown_job_id"
            }
        );

        let duplicate_err = build_runtime_summary_from_events(
            workspace_id,
            session_id.clone(),
            &[
                RuntimeEvent::JobQueued {
                    session_id: session_id.clone(),
                    job_id: job_id.clone(),
                    kind: JobKind::CompareModelScores,
                },
                RuntimeEvent::JobQueued {
                    session_id,
                    job_id,
                    kind: JobKind::RefreshEvidenceIndex,
                },
            ],
            NativeInferenceRuntimeState::Disabled,
            &RuntimeSummaryProviderPolicy::new(),
        )
        .unwrap_err();
        assert_eq!(
            duplicate_err,
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_summary_provider.duplicate_job_id"
            }
        );
    }

    #[test]
    fn rejects_unsafe_runtime_summary_provider_policy_flags() {
        let mut policy = RuntimeSummaryProviderPolicy::new();
        policy.storage_provider_enabled = true;

        let err = build_runtime_summary_from_events(
            WorkspaceId::new("workspace-unsafe-provider").unwrap(),
            SessionId::new("session-unsafe-provider").unwrap(),
            &[RuntimeEvent::WorkspaceOpened {
                workspace_id: WorkspaceId::new("workspace-unsafe-provider").unwrap(),
            }],
            NativeInferenceRuntimeState::Disabled,
            &policy,
        )
        .unwrap_err();

        assert_eq!(
            err,
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_summary_provider.storage_provider_enabled"
            }
        );
    }

    #[test]
    fn emits_static_model_registry_metadata_fixture() {
        let metadata = ModelRegistryMetadata::synthetic_fixture();

        assert_eq!(
            metadata.schema_version,
            MODEL_REGISTRY_METADATA_SCHEMA_VERSION
        );
        assert_eq!(metadata.metadata_scope, MODEL_REGISTRY_METADATA_SCOPE);
        assert_eq!(
            metadata.source_bundle_schema,
            MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION
        );
        assert_eq!(metadata.entries.len(), 10);
        assert_eq!(metadata.entries[0].model_id, "graph_novelty");
        assert_eq!(
            metadata.entries[0].registry_state.as_str(),
            "observed_synthetic_only"
        );
        assert_eq!(metadata.entries[0].promotion_state.as_str(), "not_promoted");
        assert_eq!(metadata.entries[0].source_count, 4);
        assert!(metadata.entries[0].has_score_rows);
        assert!(metadata.entries[0].human_review_required);
        assert!(!metadata.entries[0].deployment_allowed);
        assert_eq!(metadata.entries[9].model_id, "time_series_residual");
        assert_eq!(metadata.aggregate_summary.model_count, 10);
        assert_eq!(
            metadata.aggregate_summary.models_with_score_rows,
            strings(&[
                "graph_novelty",
                "isolation_forest",
                "pyod_copod",
                "pyod_ecod",
                "river_hst",
                "stdlib_linear_native",
                "suricata_alert",
                "time_series_residual"
            ])
        );
        assert!(!metadata.aggregate_summary.deployment_allowed);
        assert!(metadata.safety_flags.local_only);
        assert!(metadata.safety_flags.strict_json_loaded);
        assert!(metadata.safety_flags.derived_from_evaluation_bundle_only);
        assert!(!metadata.safety_flags.deployment_allowed);
        assert_eq!(
            metadata.non_claims,
            strings(&[
                "not_persistent_model_registry",
                "not_model_promotion_gate",
                "not_deployment_approval",
                "not_live_capture",
                "not_external_enrichment",
                "not_rule_deployment",
                "not_native_runtime_execution"
            ])
        );
    }

    #[test]
    fn emits_static_model_registry_metadata_adapter_contract_fixture() {
        let contract = ModelRegistryMetadataAdapterContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            MODEL_REGISTRY_METADATA_ADAPTER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.accepted_metadata_schema,
            MODEL_REGISTRY_METADATA_SCHEMA_VERSION
        );
        assert_eq!(
            contract.source_bundle_schema,
            MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION
        );
        assert_eq!(
            contract.max_file_bytes,
            RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES
        );
        assert!(contract.local_only);
        assert!(contract.synthetic_metadata_only);
        assert!(contract.strict_json_parsing_enabled);
        assert!(contract.file_io_enabled);
        assert!(!contract.storage_provider_enabled);
        assert!(!contract.generated_report_loading_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert_eq!(
            contract.non_claims,
            &[
                "not_persistent_model_registry",
                "not_storage_provider",
                "not_model_promotion_gate",
                "not_deployment_approval",
                "not_generated_report_loader",
                "not_qt_binding",
                "not_capture_boundary",
                "not_external_service",
                "not_native_runtime_execution"
            ]
        );
    }

    #[test]
    fn exposes_model_registry_metadata_adapter_policy() {
        let root = temp_policy_root("metadata-adapter-policy");
        let policy = ModelRegistryMetadataAdapterPolicy::new(root.clone());

        assert_eq!(policy.file_policy.allowed_root, root);
        assert_eq!(policy.max_bytes(), RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES);
        assert!(policy.local_only);
        assert!(policy.synthetic_metadata_only);
        assert!(!policy.storage_provider_enabled);
        assert!(!policy.generated_report_loading_enabled);
        assert!(!policy.qt_binding_enabled);
        assert!(!policy.capture_enabled);
        assert!(!policy.external_services_used);
        assert!(!policy.deployment_allowed);
        assert!(!policy.native_inference_execution_enabled);
        policy.validate().unwrap();

        let mut drifted_policy = policy.clone();
        drifted_policy.storage_provider_enabled = true;
        assert_eq!(
            drifted_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "model_registry_metadata_adapter.storage_provider_enabled",
            }
        );

        remove_temp_root(&policy.file_policy.allowed_root);
    }

    #[test]
    fn parses_model_registry_metadata_json_string() {
        let json = synthetic_model_registry_metadata_json();
        let metadata = parse_model_registry_metadata_json(&json).unwrap();
        let from_contract =
            ModelRegistryMetadataAdapterContract::parse_model_registry_metadata_json(&json)
                .unwrap();

        assert_eq!(metadata, ModelRegistryMetadata::synthetic_fixture());
        assert_eq!(from_contract, metadata);
        assert_eq!(
            metadata.schema_version,
            MODEL_REGISTRY_METADATA_SCHEMA_VERSION
        );
        assert_eq!(metadata.metadata_scope, MODEL_REGISTRY_METADATA_SCOPE);
        assert_eq!(metadata.entries.len(), 10);
        assert_eq!(metadata.entries[0].model_id, "graph_novelty");
        assert_eq!(
            metadata.aggregate_summary.models_with_score_rows,
            strings(&[
                "graph_novelty",
                "isolation_forest",
                "pyod_copod",
                "pyod_ecod",
                "river_hst",
                "stdlib_linear_native",
                "suricata_alert",
                "time_series_residual"
            ])
        );
        assert!(!metadata.aggregate_summary.deployment_allowed);
        assert!(!metadata.safety_flags.external_services_used);
        assert!(!metadata.safety_flags.deployment_allowed);
    }

    #[test]
    fn parses_python_valid_three_model_registry_metadata_json_string() {
        let json = three_model_registry_metadata_json();
        let metadata = parse_model_registry_metadata_json(&json).unwrap();

        assert_eq!(metadata, three_model_registry_metadata_fixture());
        assert_eq!(metadata.aggregate_summary.model_count, 3);
        assert_eq!(
            metadata.aggregate_summary.schemas_present,
            strings(&["model_disagreement_report.v0", "model_score_rows.v0"])
        );
        assert_eq!(
            metadata.aggregate_summary.models_with_score_rows,
            strings(&["isolation_forest", "pyod_ecod", "stdlib_linear_native"])
        );
    }

    #[test]
    fn parses_model_registry_metadata_file_under_allowed_root() {
        let root = temp_policy_root("valid-metadata-file");
        let path = write_test_file(
            &root,
            "model_registry_metadata.json",
            synthetic_model_registry_metadata_json(),
        );
        let policy = ModelRegistryMetadataAdapterPolicy::new(root.clone());

        let from_file = parse_model_registry_metadata_file(&path, &policy).unwrap();
        let from_contract =
            ModelRegistryMetadataAdapterContract::parse_model_registry_metadata_file(
                &path, &policy,
            )
            .unwrap();
        let from_json =
            parse_model_registry_metadata_json(&synthetic_model_registry_metadata_json()).unwrap();

        assert_eq!(from_file, from_json);
        assert_eq!(from_contract, from_file);
        assert_eq!(from_file.entries[9].model_id, "time_series_residual");

        remove_temp_root(&root);
    }

    #[test]
    fn parses_python_valid_three_model_registry_metadata_file_under_allowed_root() {
        let root = temp_policy_root("valid-three-model-metadata-file");
        let path = write_test_file(
            &root,
            "three_model_registry_metadata.json",
            three_model_registry_metadata_json(),
        );
        let policy = ModelRegistryMetadataAdapterPolicy::new(root.clone());

        let from_file = parse_model_registry_metadata_file(&path, &policy).unwrap();

        assert_eq!(from_file, three_model_registry_metadata_fixture());
        assert_eq!(from_file.entries[0].model_id, "isolation_forest");
        assert_eq!(from_file.entries[2].model_id, "stdlib_linear_native");

        remove_temp_root(&root);
    }

    #[test]
    fn emits_static_runtime_handoff_snapshot_fixture() {
        let snapshot = RuntimeHandoffSnapshot::synthetic_fixture();

        assert_eq!(
            snapshot.schema_version,
            RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(snapshot.source_kind.as_str(), "static_synthetic_fixture");
        assert_eq!(snapshot.transport_state.as_str(), "unavailable");
        assert_eq!(snapshot.control_plane_state.as_str(), "unavailable");
        assert_eq!(
            snapshot.runtime_summary.schema_version,
            RUNTIME_SUMMARY_SCHEMA_VERSION
        );
        assert_eq!(
            snapshot.model_registry_metadata.schema_version,
            MODEL_REGISTRY_METADATA_SCHEMA_VERSION
        );
        assert!(snapshot.local_only);
        assert!(snapshot.static_synthetic_fixture);
        assert!(!snapshot.generated_json_loaded);
        assert!(!snapshot.live_runtime_connection);
        assert!(!snapshot.external_services_used);
        assert!(!snapshot.deployment_allowed);
        assert_eq!(
            snapshot.non_claims,
            strings(&[
                "not_live_runtime_connection",
                "not_generated_json_loader",
                "not_control_plane_transport",
                "not_persistent_storage",
                "not_qt_runtime_integration",
                "not_model_promotion_gate",
                "not_deployment_approval",
                "not_native_runtime_execution"
            ])
        );
    }

    #[test]
    fn exposes_runtime_registry_provider_contract_fixture() {
        let contract = RuntimeRegistryProviderContract::synthetic_fixture();
        let policy = RuntimeRegistryProviderPolicy::new();

        assert_eq!(
            contract.schema_version,
            RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.accepted_snapshot_schema,
            RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.output_snapshot_schema,
            RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.max_records,
            RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP
        );
        assert!(contract.local_only);
        assert!(contract.in_memory_only);
        assert!(contract.accepts_validated_handoff_snapshots_only);
        assert!(contract.strict_handoff_validation_enabled);
        assert!(contract.upsert_replaces_matching_workspace_session);
        assert!(contract.deterministic_snapshot_ordering);
        assert!(!contract.persistent_storage_enabled);
        assert!(!contract.database_or_indexing_enabled);
        assert!(!contract.generated_report_loading_enabled);
        assert!(!contract.generated_json_loading_enabled);
        assert!(!contract.file_io_enabled);
        assert!(!contract.live_transport_enabled);
        assert!(!contract.public_network_transport_enabled);
        assert!(!contract.socket_listener_enabled);
        assert!(!contract.filesystem_socket_path_policy_enabled);
        assert!(!contract.daemon_lifecycle_enabled);
        assert!(!contract.process_spawning_enabled);
        assert!(!contract.file_watching_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert!(contract.non_claims.contains(&"not_persistent_storage"));
        assert!(contract
            .non_claims
            .contains(&"not_database_or_indexing_engine"));
        assert!(contract.non_claims.contains(&"not_generated_report_loader"));
        assert!(contract.non_claims.contains(&"not_qt_binding"));
        assert!(contract
            .non_claims
            .contains(&"not_native_runtime_execution"));
        assert_eq!(policy, RuntimeRegistryProviderPolicy::default());
        assert_eq!(
            policy.max_records,
            RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP
        );
        policy.validate().unwrap();

        assert_eq!(
            RuntimeRegistryProviderPolicy::bounded(0)
                .validate()
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_provider.max_records",
            }
        );
        assert_eq!(
            RuntimeRegistryProviderPolicy::bounded(
                RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP + 1,
            )
            .validate()
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_provider.max_records",
            }
        );
    }

    #[test]
    fn runtime_registry_provider_stores_validated_handoff_snapshots_in_key_order() {
        let mut provider = RuntimeRegistryProvider::default();

        provider
            .upsert_snapshot(registry_handoff_fixture(
                "workspace-beta",
                "session-beta",
                2,
                "beta snapshot ready",
            ))
            .unwrap();
        provider
            .upsert_snapshot(registry_handoff_fixture(
                "workspace-alpha",
                "session-alpha",
                1,
                "alpha snapshot ready",
            ))
            .unwrap();

        let snapshot = provider.snapshot();
        assert_eq!(
            snapshot.schema_version,
            RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION
        );
        assert_eq!(
            snapshot.accepted_snapshot_schema,
            RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(snapshot.record_count, 2);
        assert_eq!(
            snapshot.max_record_count,
            RUNTIME_REGISTRY_PROVIDER_DEFAULT_RECORD_CAP as u32
        );
        assert!(snapshot.local_only);
        assert!(snapshot.in_memory_only);
        assert!(!snapshot.persistent_storage_enabled);
        assert!(!snapshot.database_or_indexing_enabled);
        assert!(!snapshot.generated_report_loading_enabled);
        assert!(!snapshot.generated_json_loading_enabled);
        assert!(!snapshot.file_io_enabled);
        assert!(!snapshot.live_transport_enabled);
        assert!(!snapshot.public_network_transport_enabled);
        assert!(!snapshot.socket_listener_enabled);
        assert!(!snapshot.filesystem_socket_path_policy_enabled);
        assert!(!snapshot.daemon_lifecycle_enabled);
        assert!(!snapshot.process_spawning_enabled);
        assert!(!snapshot.file_watching_enabled);
        assert!(!snapshot.qt_binding_enabled);
        assert!(!snapshot.capture_enabled);
        assert!(!snapshot.external_services_used);
        assert!(!snapshot.deployment_allowed);
        assert!(!snapshot.native_inference_execution_enabled);
        assert_eq!(
            snapshot.non_claims,
            strings(&[
                "not_persistent_storage",
                "not_database_or_indexing_engine",
                "not_generated_report_loader",
                "not_generated_json_loader",
                "not_control_plane_transport",
                "not_public_network_transport",
                "not_socket_listener",
                "not_filesystem_socket_path_policy",
                "not_daemon_lifecycle",
                "not_process_spawner",
                "not_file_watcher",
                "not_qt_binding",
                "not_capture_boundary",
                "not_external_service",
                "not_deployment_approval",
                "not_native_runtime_execution"
            ])
        );
        assert_eq!(snapshot.records.len(), 2);
        assert_eq!(snapshot.records[0].workspace_id.as_str(), "workspace-alpha");
        assert_eq!(snapshot.records[0].session_id.as_str(), "session-alpha");
        assert_eq!(snapshot.records[1].workspace_id.as_str(), "workspace-beta");
        assert_eq!(snapshot.records[1].session_id.as_str(), "session-beta");
        assert_eq!(
            snapshot.records[0].snapshot_schema_version,
            RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(
            snapshot.records[0]
                .snapshot
                .runtime_summary
                .last_event_label,
            "alpha snapshot ready"
        );
    }

    #[test]
    fn runtime_registry_provider_upsert_replaces_existing_workspace_session() {
        let mut provider = RuntimeRegistryProvider::default();

        provider
            .upsert_snapshot(registry_handoff_fixture(
                "workspace-replace",
                "session-replace",
                1,
                "first snapshot ready",
            ))
            .unwrap();
        let replacement = provider
            .upsert_snapshot(registry_handoff_fixture(
                "workspace-replace",
                "session-replace",
                3,
                "replacement snapshot ready",
            ))
            .unwrap();

        assert_eq!(provider.len(), 1);
        assert!(!provider.is_empty());
        assert_eq!(
            replacement.snapshot.runtime_summary.last_event_label,
            "replacement snapshot ready"
        );

        let snapshot = provider.snapshot();
        assert_eq!(snapshot.record_count, 1);
        assert_eq!(
            snapshot.records[0].workspace_id.as_str(),
            "workspace-replace"
        );
        assert_eq!(snapshot.records[0].session_id.as_str(), "session-replace");
        assert_eq!(
            snapshot.records[0].snapshot.runtime_summary.total_job_count,
            3
        );
    }

    #[test]
    fn runtime_registry_provider_rejects_record_cap_overflow() {
        let mut provider =
            RuntimeRegistryProvider::new(RuntimeRegistryProviderPolicy::bounded(1)).unwrap();

        provider
            .upsert_snapshot(registry_handoff_fixture(
                "workspace-cap-alpha",
                "session-cap-alpha",
                1,
                "alpha cap snapshot",
            ))
            .unwrap();
        assert_eq!(
            provider
                .upsert_snapshot(registry_handoff_fixture(
                    "workspace-cap-beta",
                    "session-cap-beta",
                    1,
                    "beta cap snapshot",
                ))
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_provider.record_cap",
            }
        );

        provider
            .upsert_snapshot(registry_handoff_fixture(
                "workspace-cap-alpha",
                "session-cap-alpha",
                2,
                "alpha cap replacement",
            ))
            .unwrap();
        assert_eq!(provider.snapshot().record_count, 1);
        assert_eq!(
            provider.snapshot().records[0]
                .snapshot
                .runtime_summary
                .last_event_label,
            "alpha cap replacement"
        );
    }

    #[test]
    fn runtime_registry_provider_rejects_unsafe_policy_flags() {
        let mut policy = RuntimeRegistryProviderPolicy::new();
        policy.persistent_storage_enabled = true;

        assert_eq!(
            RuntimeRegistryProvider::new(policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_registry_provider.persistent_storage_enabled",
            }
        );

        let mut drifted_policy = RuntimeRegistryProviderPolicy::bounded(1);
        drifted_policy.qt_binding_enabled = true;
        let mut provider = RuntimeRegistryProvider {
            policy: drifted_policy,
            records: BTreeMap::new(),
        };
        assert_eq!(
            provider
                .upsert_snapshot(RuntimeHandoffSnapshot::synthetic_fixture())
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_registry_provider.qt_binding_enabled",
            }
        );
    }

    #[test]
    fn runtime_registry_provider_rejects_malformed_nested_snapshots() {
        let mut provider = RuntimeRegistryProvider::default();

        let mut generated_json = RuntimeHandoffSnapshot::synthetic_fixture();
        generated_json.generated_json_loaded = true;
        assert_eq!(
            provider.upsert_snapshot(generated_json).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "generated_json_loaded",
            }
        );

        let mut non_local = RuntimeHandoffSnapshot::synthetic_fixture();
        non_local.local_only = false;
        assert_eq!(
            provider.upsert_snapshot(non_local).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "local_only",
            }
        );

        let mut external_service = RuntimeHandoffSnapshot::synthetic_fixture();
        external_service.external_services_used = true;
        assert_eq!(
            provider.upsert_snapshot(external_service).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "external_services_used",
            }
        );

        let mut deployment = RuntimeHandoffSnapshot::synthetic_fixture();
        deployment.deployment_allowed = true;
        assert_eq!(
            provider.upsert_snapshot(deployment).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "deployment_allowed",
            }
        );

        let mut bad_summary = RuntimeHandoffSnapshot::synthetic_fixture();
        bad_summary.runtime_summary.total_job_count = 1;
        bad_summary.runtime_summary.running_job_count = 2;
        assert_eq!(
            provider.upsert_snapshot(bad_summary).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_summary.job_counts",
            }
        );

        let mut bad_metadata = RuntimeHandoffSnapshot::synthetic_fixture();
        bad_metadata
            .model_registry_metadata
            .aggregate_summary
            .model_count = 9;
        assert_eq!(
            provider.upsert_snapshot(bad_metadata).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.aggregate_summary.model_count",
            }
        );

        let mut unsafe_metadata = RuntimeHandoffSnapshot::synthetic_fixture();
        unsafe_metadata
            .model_registry_metadata
            .safety_flags
            .external_services_used = true;
        assert_eq!(
            provider.upsert_snapshot(unsafe_metadata).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "model_registry_metadata.safety_flags.external_services_used",
            }
        );
    }

    #[test]
    fn exposes_runtime_registry_storage_provider_contract_fixture() {
        let contract = RuntimeRegistryStorageProviderContract::synthetic_fixture();
        let root = temp_policy_root("registry-storage-policy");
        let policy = RuntimeRegistryStoragePolicy::new(root.clone());

        assert_eq!(
            contract.schema_version,
            RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.accepted_registry_snapshot_schema,
            RUNTIME_REGISTRY_PROVIDER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.storage_document_schema,
            RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.max_file_bytes,
            RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES
        );
        assert!(contract.local_only);
        assert!(contract.caller_authorized_allowed_root_required);
        assert!(contract.typed_registry_snapshots_only);
        assert!(contract.strict_registry_validation_enabled);
        assert!(contract.storage_document_json_enabled);
        assert!(contract.file_io_enabled);
        assert!(contract.persistent_storage_enabled);
        assert!(!contract.database_or_indexing_enabled);
        assert!(!contract.generated_report_loading_enabled);
        assert!(!contract.generated_json_loading_enabled);
        assert!(!contract.arbitrary_file_loading_enabled);
        assert!(!contract.live_transport_enabled);
        assert!(!contract.public_network_transport_enabled);
        assert!(!contract.socket_listener_enabled);
        assert!(!contract.filesystem_socket_path_policy_enabled);
        assert!(!contract.daemon_lifecycle_enabled);
        assert!(!contract.process_spawning_enabled);
        assert!(!contract.file_watching_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert!(contract
            .non_claims
            .contains(&"not_database_or_indexing_engine"));
        assert!(contract.non_claims.contains(&"not_generated_report_loader"));
        assert!(contract.non_claims.contains(&"not_arbitrary_file_loader"));
        assert!(contract.non_claims.contains(&"not_socket_listener"));
        assert!(contract
            .non_claims
            .contains(&"not_native_runtime_execution"));
        assert_eq!(policy.max_bytes(), RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES);
        policy.validate().unwrap();

        remove_temp_root(&root);
    }

    #[test]
    fn runtime_registry_storage_policy_rejects_unsafe_flags() {
        let root = temp_policy_root("registry-storage-unsafe-policy");
        let mut policy = RuntimeRegistryStoragePolicy::new(root.clone());
        policy.database_or_indexing_enabled = true;
        assert_eq!(
            policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_registry_storage_provider.database_or_indexing_enabled",
            }
        );

        let mut too_large = RuntimeRegistryStoragePolicy::new(root.clone());
        too_large.max_file_bytes = RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES + 1;
        assert_eq!(
            too_large.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_storage_provider.max_file_bytes",
            }
        );

        let mut drifted = RuntimeRegistryStoragePolicy::new(root.clone());
        drifted.persistent_storage_enabled = false;
        assert_eq!(
            drifted.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_registry_storage_provider.persistent_storage_enabled",
            }
        );

        remove_temp_root(&root);
    }

    #[test]
    fn runtime_registry_storage_provider_persists_and_loads_typed_snapshot() {
        let root = temp_policy_root("registry-storage-roundtrip");
        let policy = RuntimeRegistryStoragePolicy::new(root.clone());
        let provider = RuntimeRegistryStorageProvider::new(policy.clone()).unwrap();
        let path = root.join("runtime_registry_storage.json");
        let snapshot = runtime_registry_snapshot_fixture();

        let document = provider.persist_snapshot(&path, &snapshot).unwrap();
        let loaded_document = provider.load_document(&path).unwrap();
        let loaded_snapshot =
            RuntimeRegistryStorageProviderContract::load_snapshot_file(&path, &policy).unwrap();

        assert_eq!(document, loaded_document);
        assert_eq!(loaded_snapshot, snapshot);
        assert_eq!(
            loaded_document.schema_version,
            RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION
        );
        assert!(loaded_document.persistent_storage_enabled);
        assert!(!loaded_document.database_or_indexing_enabled);
        assert_eq!(loaded_snapshot.record_count, 2);
        assert_eq!(
            loaded_snapshot.records[0].workspace_id.as_str(),
            "workspace-storage-alpha"
        );
        assert_eq!(
            loaded_snapshot.records[1].workspace_id.as_str(),
            "workspace-storage-beta"
        );

        remove_temp_root(&root);
    }

    #[test]
    fn runtime_registry_storage_document_rejects_schema_and_flag_drift() {
        let mut document = runtime_registry_storage_document_fixture();
        document.schema_version = "runtime_registry_storage_provider.v1".to_owned();
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "runtime_registry_storage_provider.schema_version",
                expected: RUNTIME_REGISTRY_STORAGE_PROVIDER_SCHEMA_VERSION,
            }
        );

        let mut document = runtime_registry_storage_document_fixture();
        document.external_services_used = true;
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_registry_storage_provider.external_services_used",
            }
        );

        let mut document = runtime_registry_storage_document_fixture();
        document.non_claims.pop();
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_storage_provider.non_claims",
            }
        );
    }

    #[test]
    fn runtime_registry_storage_rejects_malformed_json_and_unknown_fields() {
        assert_eq!(
            parse_runtime_registry_storage_document_json("{").unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert_eq!(
            parse_runtime_registry_storage_document_json("[]").unwrap_err(),
            RuntimeControlPlaneAdapterError::NonObjectRoot
        );

        let unknown_root_field = runtime_registry_storage_document_json().replacen(
            r#"  "registry_snapshot_schema": "runtime_registry_provider.v0","#,
            r#"  "registry_snapshot_schema": "runtime_registry_provider.v0",
  "unexpected_field": true,"#,
            1,
        );
        assert_eq!(
            parse_runtime_registry_storage_document_json(&unknown_root_field).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        let duplicate_flag = runtime_registry_storage_document_json().replacen(
            r#"  "external_services_used": false,"#,
            r#"  "external_services_used": true,
  "external_services_used": false,"#,
            1,
        );
        assert_eq!(
            parse_runtime_registry_storage_document_json(&duplicate_flag).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
    }

    #[test]
    fn runtime_registry_storage_rejects_registry_snapshot_drift() {
        let mut document = runtime_registry_storage_document_fixture();
        document.registry_snapshot.record_count = 3;
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_snapshot.record_count",
            }
        );

        let mut document = runtime_registry_storage_document_fixture();
        document.registry_snapshot.max_record_count = 0;
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_snapshot.max_record_count",
            }
        );

        let mut document = runtime_registry_storage_document_fixture();
        document.registry_snapshot.records.swap(0, 1);
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_snapshot.records",
            }
        );

        let mut document = runtime_registry_storage_document_fixture();
        document.registry_snapshot.records[1] = document.registry_snapshot.records[0].clone();
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_snapshot.records",
            }
        );

        let mut document = runtime_registry_storage_document_fixture();
        document.registry_snapshot.qt_binding_enabled = true;
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_registry_snapshot.qt_binding_enabled",
            }
        );
    }

    #[test]
    fn runtime_registry_storage_rejects_record_and_nested_snapshot_drift() {
        let mut document = runtime_registry_storage_document_fixture();
        document.registry_snapshot.records[0].workspace_id =
            WorkspaceId::new("workspace-storage-mismatch").unwrap();
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_registry_snapshot.records.workspace_id",
            }
        );

        let mut document = runtime_registry_storage_document_fixture();
        document.registry_snapshot.records[0]
            .snapshot
            .generated_json_loaded = true;
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "generated_json_loaded",
            }
        );

        let mut document = runtime_registry_storage_document_fixture();
        document.registry_snapshot.records[0]
            .snapshot
            .runtime_summary
            .total_job_count = 1;
        document.registry_snapshot.records[0]
            .snapshot
            .runtime_summary
            .running_job_count = 2;
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_summary.job_counts",
            }
        );

        let mut document = runtime_registry_storage_document_fixture();
        document.registry_snapshot.records[0]
            .snapshot
            .model_registry_metadata
            .aggregate_summary
            .model_count = 9;
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.aggregate_summary.model_count",
            }
        );

        let mut document = runtime_registry_storage_document_fixture();
        document.registry_snapshot.records[0]
            .snapshot
            .model_registry_metadata
            .safety_flags
            .deployment_allowed = true;
        assert_eq!(
            validate_runtime_registry_storage_document(&document).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "model_registry_metadata.safety_flags.deployment_allowed",
            }
        );
    }

    #[test]
    fn runtime_registry_storage_file_policy_rejects_unsafe_read_paths() {
        let root = temp_policy_root("registry-storage-path-policy");
        let outside_root = temp_policy_root("outside-registry-storage-path-policy");
        let policy = RuntimeRegistryStoragePolicy::new(root.clone());
        let valid_json = runtime_registry_storage_document_json();

        assert_eq!(
            load_runtime_registry_snapshot_file("runtime_registry_storage.json", &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeFilePath
        );

        let relative_root_policy = RuntimeRegistryStoragePolicy::new("relative-root");
        let relative_root_path = write_test_file(
            &root,
            "relative_root_runtime_registry_storage.json",
            &valid_json,
        );
        assert_eq!(
            load_runtime_registry_snapshot_file(relative_root_path, &relative_root_policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeAllowedRoot
        );

        let missing_root_policy =
            RuntimeRegistryStoragePolicy::new(root.join("missing-storage-root"));
        let missing_root_path = root
            .join("missing-storage-root")
            .join("runtime_registry_storage.json");
        assert_eq!(
            load_runtime_registry_snapshot_file(missing_root_path, &missing_root_policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingAllowedRoot
        );

        assert_eq!(
            load_runtime_registry_snapshot_file(root.join("missing_storage.json"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingFile
        );

        let file_root = write_test_file(&root, "file_policy_root.json", &valid_json);
        let file_root_policy = RuntimeRegistryStoragePolicy::new(file_root.clone());
        assert_eq!(
            load_runtime_registry_snapshot_file(file_root, &file_root_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootNotDirectory
        );

        let outside_path =
            write_test_file(&outside_root, "runtime_registry_storage.json", &valid_json);
        assert_eq!(
            load_runtime_registry_snapshot_file(outside_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OutsideAllowedRoot
        );

        let text_path = write_test_file(&root, "runtime_registry_storage.txt", "{}");
        assert_eq!(
            load_runtime_registry_snapshot_file(text_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedFileExtension
        );

        let directory_path = root.join("runtime_registry_storage_directory.json");
        std::fs::create_dir_all(&directory_path).unwrap();
        assert_eq!(
            load_runtime_registry_snapshot_file(directory_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::DirectoryPath
        );

        let oversized_path = write_test_file(
            &root,
            "oversized_runtime_registry_storage.json",
            vec![b' '; RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES as usize + 1],
        );
        assert_eq!(
            load_runtime_registry_snapshot_file(oversized_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedFile {
                max_bytes: RUNTIME_REGISTRY_STORAGE_FILE_MAX_BYTES,
            }
        );

        let invalid_utf8_path =
            write_test_file(&root, "invalid_utf8_runtime_registry_storage.json", [0xff]);
        assert_eq!(
            load_runtime_registry_snapshot_file(invalid_utf8_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );

        remove_temp_root(&root);
        remove_temp_root(&outside_root);
    }

    #[test]
    fn runtime_registry_storage_file_policy_validates_write_paths() {
        let root = temp_policy_root("registry-storage-write-path-policy");
        let outside_root = temp_policy_root("outside-registry-storage-write-path-policy");
        let policy = RuntimeRegistryStoragePolicy::new(root.clone());
        let snapshot = runtime_registry_snapshot_fixture();

        assert_eq!(
            persist_runtime_registry_snapshot_file(
                "runtime_registry_storage.json",
                &snapshot,
                &policy
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeFilePath
        );

        let outside_path = outside_root.join("runtime_registry_storage.json");
        assert_eq!(
            persist_runtime_registry_snapshot_file(outside_path, &snapshot, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OutsideAllowedRoot
        );

        let text_path = root.join("runtime_registry_storage.txt");
        assert_eq!(
            persist_runtime_registry_snapshot_file(text_path, &snapshot, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedFileExtension
        );

        let directory_path = root.join("runtime_registry_storage_directory.json");
        std::fs::create_dir_all(&directory_path).unwrap();
        assert_eq!(
            persist_runtime_registry_snapshot_file(directory_path, &snapshot, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::DirectoryPath
        );

        let missing_parent_path = root
            .join("missing-parent")
            .join("runtime_registry_storage.json");
        assert_eq!(
            persist_runtime_registry_snapshot_file(missing_parent_path, &snapshot, &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingFile
        );

        remove_temp_root(&root);
        remove_temp_root(&outside_root);
    }

    #[cfg(unix)]
    #[test]
    fn runtime_registry_storage_file_policy_rejects_symlinks_and_non_regular_files() {
        use std::os::unix::fs::symlink;

        let root = temp_policy_root("registry-storage-symlink-policy");
        let target_path = write_test_file(
            &root,
            "runtime_registry_storage_target.json",
            runtime_registry_storage_document_json(),
        );
        let symlink_path = root.join("runtime_registry_storage_symlink.json");
        symlink(&target_path, &symlink_path).unwrap();
        let policy = RuntimeRegistryStoragePolicy::new(root.clone());
        assert_eq!(
            load_runtime_registry_snapshot_file(&symlink_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::SymlinkPath
        );

        let symlink_root = root.join("symlink-root");
        symlink(&root, &symlink_root).unwrap();
        let symlink_root_policy = RuntimeRegistryStoragePolicy::new(symlink_root.clone());
        assert_eq!(
            load_runtime_registry_snapshot_file(
                symlink_root.join("runtime_registry_storage_target.json"),
                &symlink_root_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootSymlink
        );

        if !effective_user_id_is_root() {
            let fifo_path = root.join("runtime_registry_storage_fifo.json");
            make_fifo(&fifo_path);
            assert_eq!(
                load_runtime_registry_snapshot_file(fifo_path, &policy).unwrap_err(),
                RuntimeControlPlaneAdapterError::NonRegularFile
            );
        }

        remove_temp_root(&root);
    }

    #[test]
    fn emits_static_runtime_control_plane_adapter_contract_fixture() {
        let contract = RuntimeControlPlaneAdapterContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_CONTROL_PLANE_ADAPTER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.adapter_kind.as_str(),
            "local_control_plane_endpoint_policy"
        );
        assert_eq!(
            contract.input_mode.as_str(),
            "accepted_local_endpoint_policy"
        );
        assert_eq!(
            contract.adapter_state.as_str(),
            "local_endpoint_policy_available"
        );
        assert_eq!(
            contract.output_snapshot_schema.as_str(),
            RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.accepted_input_schemas,
            &[
                RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
                RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION,
                RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION,
                RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
                RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
                RUNTIME_SUMMARY_SCHEMA_VERSION,
                MODEL_REGISTRY_METADATA_SCHEMA_VERSION
            ]
        );
        assert!(contract.local_only);
        assert!(!contract.dependency_free);
        assert!(contract.static_synthetic_fixture);
        assert!(contract.json_parsing_enabled);
        assert!(contract.file_io_enabled);
        assert!(!contract.live_transport_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert_eq!(
            contract.non_claims,
            &[
                "not_arbitrary_file_loader",
                "not_file_watcher",
                "not_live_transport",
                "not_socket_listener",
                "not_daemon_lifecycle",
                "not_filesystem_socket_path_policy",
                "not_process_spawner",
                "not_qt_binding",
                "not_external_service",
                "not_deployment_approval",
                "not_runtime_service",
                "not_generated_report_loader"
            ]
        );
    }

    #[test]
    fn emits_static_runtime_control_plane_ipc_adapter_contract_fixture() {
        let contract = RuntimeControlPlaneIpcAdapterContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION
        );
        assert_eq!(
            contract.frame_schema_version,
            RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION
        );
        assert_eq!(
            contract.message_schema_version,
            RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION
        );
        assert_eq!(
            contract.length_prefix_bytes,
            RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES
        );
        assert_eq!(
            contract.max_frame_bytes,
            RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
        );
        assert!(contract.local_only);
        assert!(contract.caller_provided_streams_only);
        assert!(contract.one_shot_request_response);
        assert!(contract.big_endian_length_prefix_required);
        assert!(contract.utf8_json_payload_required);
        assert!(!contract.additional_dependencies_required);
        assert!(contract.stream_io_enabled);
        assert!(!contract.live_transport_enabled);
        assert!(!contract.socket_listener_enabled);
        assert!(!contract.filesystem_socket_path_policy_enabled);
        assert!(!contract.daemon_lifecycle_enabled);
        assert!(!contract.process_spawning_enabled);
        assert!(!contract.file_watching_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.storage_provider_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert_eq!(
            contract.non_claims,
            &[
                "not_public_network_transport",
                "not_socket_listener",
                "not_daemon_lifecycle",
                "not_filesystem_socket_path_policy",
                "not_process_spawner",
                "not_file_watcher",
                "not_qt_binding",
                "not_storage_provider",
                "not_capture_boundary",
                "not_external_service",
                "not_deployment_approval",
                "not_native_runtime_execution"
            ]
        );
    }

    #[test]
    fn emits_static_runtime_control_plane_endpoint_adapter_contract_fixture() {
        let contract = RuntimeControlPlaneEndpointAdapterContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.ipc_schema_version,
            RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION
        );
        assert_eq!(
            contract.frame_schema_version,
            RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION
        );
        assert_eq!(
            contract.message_schema_version,
            RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION
        );
        assert_eq!(
            contract.endpoint_kind.as_str(),
            "caller_provided_connected_stream"
        );
        assert!(contract.local_only);
        assert!(contract.caller_provided_streams_only);
        assert!(contract.endpoint_policy_validation_enabled);
        assert!(contract.connected_stream_execution_enabled);
        assert!(!contract.public_network_transport_enabled);
        assert!(!contract.socket_listener_enabled);
        assert!(!contract.filesystem_socket_path_policy_enabled);
        assert!(!contract.daemon_lifecycle_enabled);
        assert!(!contract.process_spawning_enabled);
        assert!(!contract.file_watching_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.storage_provider_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert_eq!(
            contract.non_claims,
            &[
                "not_public_network_transport",
                "not_socket_listener",
                "not_filesystem_socket_path_policy",
                "not_daemon_lifecycle",
                "not_process_spawner",
                "not_file_watcher",
                "not_qt_binding",
                "not_storage_provider",
                "not_capture_boundary",
                "not_external_service",
                "not_deployment_approval",
                "not_native_runtime_execution"
            ]
        );
    }

    #[test]
    fn emits_static_runtime_control_plane_frame_adapter_contract_fixture() {
        let contract = RuntimeControlPlaneFrameAdapterContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION
        );
        assert_eq!(
            contract.payload_schema_version,
            RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION
        );
        assert_eq!(
            contract.max_frame_bytes,
            RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
        );
        assert!(contract.local_only);
        assert!(contract.caller_provided_bytes_only);
        assert!(contract.utf8_json_payload_required);
        assert!(!contract.additional_dependencies_required);
        assert!(!contract.live_transport_enabled);
        assert!(!contract.socket_listener_enabled);
        assert!(!contract.daemon_lifecycle_enabled);
        assert!(!contract.process_spawning_enabled);
        assert!(!contract.file_watching_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.storage_provider_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert_eq!(
            contract.non_claims,
            &[
                "not_network_transport",
                "not_ipc_or_socket_transport",
                "not_socket_listener",
                "not_daemon_lifecycle",
                "not_process_spawner",
                "not_file_watcher",
                "not_qt_binding",
                "not_storage_provider",
                "not_capture_boundary",
                "not_deployment_approval",
                "not_native_runtime_execution"
            ]
        );
    }

    #[test]
    fn exposes_bounded_runtime_control_plane_file_policy() {
        let root = temp_policy_root("policy");
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        assert_eq!(policy.allowed_root, root);
        assert_eq!(policy.max_bytes(), 256 * 1024);

        remove_temp_root(&policy.allowed_root);
    }

    #[test]
    fn exposes_bounded_runtime_control_plane_frame_policy() {
        let policy = RuntimeControlPlaneFramePolicy::default();
        assert_eq!(policy.max_frame_bytes, 256 * 1024);
        assert_eq!(policy.max_bytes(), RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES);

        let smaller_policy = RuntimeControlPlaneFramePolicy::new(1024).unwrap();
        assert_eq!(smaller_policy.max_bytes(), 1024);

        assert_eq!(
            RuntimeControlPlaneFramePolicy::new(0).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "frame.max_frame_bytes",
            }
        );
        assert_eq!(
            RuntimeControlPlaneFramePolicy::new(RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES + 1)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "frame.max_frame_bytes",
            }
        );
    }

    #[test]
    fn exposes_bounded_runtime_control_plane_ipc_policy() {
        let policy = RuntimeControlPlaneIpcPolicy::default();
        assert_eq!(
            policy.max_frame_bytes(),
            RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
        );

        let frame_policy = RuntimeControlPlaneFramePolicy::new(1024).unwrap();
        let ipc_policy = RuntimeControlPlaneIpcPolicy::new(frame_policy);
        assert_eq!(ipc_policy.max_frame_bytes(), 1024);
    }

    #[test]
    fn exposes_bounded_runtime_control_plane_endpoint_policy() {
        let policy = RuntimeControlPlaneEndpointPolicy::default();

        assert_eq!(
            policy.schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION
        );
        assert_eq!(
            policy.endpoint_kind.as_str(),
            "caller_provided_connected_stream"
        );
        assert_eq!(
            policy.max_frame_bytes(),
            RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
        );
        assert!(policy.local_only);
        assert!(policy.caller_provided_streams_only);
        assert!(!policy.public_network_transport_enabled);
        assert!(!policy.socket_listener_enabled);
        assert!(!policy.filesystem_socket_path_policy_enabled);
        assert!(!policy.daemon_lifecycle_enabled);
        assert!(!policy.process_spawning_enabled);
        assert!(!policy.file_watching_enabled);
        assert!(!policy.qt_binding_enabled);
        assert!(!policy.storage_provider_enabled);
        assert!(!policy.capture_enabled);
        assert!(!policy.external_services_used);
        assert!(!policy.deployment_allowed);
        assert!(!policy.native_inference_execution_enabled);
        policy.validate().unwrap();

        let frame_policy = RuntimeControlPlaneFramePolicy::new(1024).unwrap();
        let ipc_policy = RuntimeControlPlaneIpcPolicy::new(frame_policy);
        let endpoint_policy =
            RuntimeControlPlaneEndpointPolicy::caller_provided_connected_stream(ipc_policy);
        assert_eq!(endpoint_policy.max_frame_bytes(), 1024);
    }

    #[test]
    fn exposes_typed_runtime_control_plane_commands() {
        let root = temp_policy_root("typed-command");
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());
        let json_command =
            RuntimeControlPlaneCommand::parse_handoff_snapshot_json(synthetic_handoff_json());
        let file_command = RuntimeControlPlaneCommand::parse_handoff_snapshot_file(
            root.join("runtime_handoff_snapshot.json"),
            policy,
        );

        assert_eq!(json_command.command_kind(), "parse_handoff_snapshot_json");
        assert_eq!(
            json_command.output_snapshot_schema(),
            RuntimeControlPlaneOutputSnapshotSchema::RuntimeHandoffSnapshotV0
        );
        assert_eq!(file_command.command_kind(), "parse_handoff_snapshot_file");
        assert_eq!(
            file_command.output_snapshot_schema(),
            RuntimeControlPlaneOutputSnapshotSchema::RuntimeHandoffSnapshotV0
        );

        remove_temp_root(&root);
    }

    #[test]
    fn exposes_runtime_control_plane_message_envelope_types() {
        let request = RuntimeControlPlaneMessageRequest::new(
            "request-json-001",
            RuntimeControlPlaneCommand::parse_handoff_snapshot_json(synthetic_handoff_json()),
        )
        .unwrap();

        assert_eq!(
            request.schema_version,
            RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION
        );
        assert_eq!(request.request_id.as_str(), "request-json-001");
        assert_eq!(
            request.command.command_kind(),
            "parse_handoff_snapshot_json"
        );

        let success = RuntimeControlPlaneMessageResponse::success(
            request.request_id.clone(),
            RuntimeHandoffSnapshot::synthetic_fixture(),
        );
        assert_eq!(
            success.schema_version,
            RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION
        );
        assert_eq!(success.outcome.as_str(), "success");
        assert!(success.snapshot.is_some());
        assert!(success.error_code.is_none());

        let failure = RuntimeControlPlaneMessageResponse::failure(
            request.request_id,
            RuntimeControlPlaneMessageErrorCode::InvalidJson,
        );
        assert_eq!(failure.outcome.as_str(), "failure");
        assert!(failure.snapshot.is_none());
        assert_eq!(
            failure.error_code.unwrap().as_str(),
            RuntimeControlPlaneMessageErrorCode::InvalidJson.as_str()
        );
    }

    #[test]
    fn parses_runtime_handoff_snapshot_json_string() {
        let snapshot = RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(
            synthetic_handoff_json(),
        )
        .unwrap();

        assert_eq!(
            snapshot.schema_version,
            RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(
            snapshot.runtime_summary.workspace_id.as_str(),
            "fixture-workspace-alpha"
        );
        assert_eq!(
            snapshot.runtime_summary.session_id.as_str(),
            "fixture-session-runtime-summary"
        );
        assert_eq!(snapshot.runtime_summary.total_job_count, 4);
        assert_eq!(
            snapshot.runtime_summary.last_event_label,
            "synthetic workstation snapshot rendered"
        );
        assert_eq!(
            snapshot.runtime_summary.native_inference_state.as_str(),
            "disabled"
        );
        assert_eq!(
            snapshot.model_registry_metadata.metadata_scope,
            MODEL_REGISTRY_METADATA_SCOPE
        );
        assert_eq!(snapshot.model_registry_metadata.entries.len(), 10);
        assert_eq!(
            snapshot.model_registry_metadata.entries[0].model_id,
            "graph_novelty"
        );
        assert_eq!(
            snapshot
                .model_registry_metadata
                .aggregate_summary
                .models_with_score_rows,
            strings(&[
                "graph_novelty",
                "isolation_forest",
                "pyod_copod",
                "pyod_ecod",
                "river_hst",
                "stdlib_linear_native",
                "suricata_alert",
                "time_series_residual"
            ])
        );
        assert!(snapshot.local_only);
        assert!(!snapshot.live_runtime_connection);
        assert!(!snapshot.external_services_used);
        assert!(!snapshot.deployment_allowed);
    }

    #[test]
    fn dispatches_runtime_handoff_snapshot_json_command() {
        let from_command = execute_json_command(synthetic_handoff_json()).unwrap();
        let from_parser = RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(
            synthetic_handoff_json(),
        )
        .unwrap();

        assert_eq!(from_command, from_parser);
        assert_eq!(
            from_command.runtime_summary.workspace_id.as_str(),
            "fixture-workspace-alpha"
        );
        assert_eq!(from_command.model_registry_metadata.entries.len(), 10);
    }

    #[test]
    fn dispatches_runtime_control_plane_json_message_request() {
        let response = RuntimeControlPlaneAdapterContract::execute_control_plane_message_json(
            &json_message_request("request-json-001", synthetic_handoff_json()),
        )
        .unwrap();
        let from_command = RuntimeControlPlaneAdapterContract::execute_local_command(
            RuntimeControlPlaneCommand::parse_handoff_snapshot_json(synthetic_handoff_json()),
        )
        .unwrap();

        assert_eq!(
            response.schema_version,
            RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION
        );
        assert_eq!(response.request_id.as_str(), "request-json-001");
        assert_eq!(response.outcome, RuntimeControlPlaneMessageOutcome::Success);
        assert_eq!(response.snapshot.as_ref().unwrap(), &from_command);
        assert!(response.error_code.is_none());

        let serialized =
            RuntimeControlPlaneAdapterContract::serialize_control_plane_message_response_json(
                &response,
            )
            .unwrap();
        let parsed_response: RuntimeControlPlaneMessageResponse =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed_response, response);
    }

    #[test]
    fn dispatches_runtime_control_plane_json_message_frame_bytes() {
        let request_json = json_message_request("request-frame-json-001", synthetic_handoff_json());
        let request = parse_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap();
        assert_eq!(request.request_id.as_str(), "request-frame-json-001");
        assert_eq!(
            request.command.command_kind(),
            "parse_handoff_snapshot_json"
        );

        let from_frame = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap(),
        );
        let from_message =
            RuntimeControlPlaneAdapterContract::execute_control_plane_message_json(&request_json)
                .unwrap();

        assert_eq!(from_frame, from_message);
        assert_eq!(
            from_frame.outcome,
            RuntimeControlPlaneMessageOutcome::Success
        );
        assert_eq!(
            from_frame
                .snapshot
                .as_ref()
                .unwrap()
                .runtime_summary
                .workspace_id
                .as_str(),
            "fixture-workspace-alpha"
        );

        let response_frame =
            serialize_control_plane_message_response_frame_bytes(&from_frame).unwrap();
        assert_eq!(response_from_frame_bytes(response_frame), from_frame);
    }

    #[test]
    fn parses_runtime_handoff_snapshot_file_under_allowed_root() {
        let root = temp_policy_root("valid-file");
        let path = write_test_file(
            &root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        let from_file =
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(&path, &policy)
                .unwrap();
        let from_json = RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(
            synthetic_handoff_json(),
        )
        .unwrap();
        let from_command = execute_file_command(path.clone(), &policy).unwrap();

        assert_eq!(from_file, from_json);
        assert_eq!(from_command, from_file);
        assert_eq!(
            from_file.runtime_summary.workspace_id.as_str(),
            "fixture-workspace-alpha"
        );

        remove_temp_root(&root);
    }

    #[test]
    fn dispatches_runtime_control_plane_file_message_frame_bytes() {
        let root = temp_policy_root("valid-file-frame");
        let path = write_test_file(
            &root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        let request_json = file_message_request("request-frame-file-001", &path, &root);

        let from_frame = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap(),
        );
        let from_message =
            RuntimeControlPlaneAdapterContract::execute_control_plane_message_json(&request_json)
                .unwrap();

        assert_eq!(from_frame, from_message);
        assert_eq!(from_frame.request_id.as_str(), "request-frame-file-001");
        assert_eq!(
            from_frame.outcome,
            RuntimeControlPlaneMessageOutcome::Success
        );
        assert_eq!(
            from_frame
                .snapshot
                .as_ref()
                .unwrap()
                .model_registry_metadata
                .entries[9]
                .model_id,
            "time_series_residual"
        );

        remove_temp_root(&root);
    }

    #[test]
    fn dispatches_runtime_control_plane_json_message_ipc_stream() {
        let request_json = json_message_request("request-ipc-json-001", synthetic_handoff_json());
        let (result, response_bytes) = execute_ipc_frame_bytes(request_json.as_bytes());

        result.unwrap();
        let from_ipc = response_from_ipc_bytes(&response_bytes);
        let from_frame = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap(),
        );

        assert_eq!(from_ipc, from_frame);
        assert_eq!(from_ipc.request_id.as_str(), "request-ipc-json-001");
        assert_eq!(from_ipc.outcome, RuntimeControlPlaneMessageOutcome::Success);
        assert_eq!(
            from_ipc
                .snapshot
                .as_ref()
                .unwrap()
                .runtime_summary
                .workspace_id
                .as_str(),
            "fixture-workspace-alpha"
        );
    }

    #[test]
    fn dispatches_runtime_control_plane_json_message_endpoint_stream() {
        let request_json =
            json_message_request("request-endpoint-json-001", synthetic_handoff_json());
        let (result, response_bytes) = execute_endpoint_frame_bytes(request_json.as_bytes());

        result.unwrap();
        let from_endpoint = response_from_ipc_bytes(&response_bytes);
        let from_frame = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap(),
        );

        assert_eq!(from_endpoint, from_frame);
        assert_eq!(
            from_endpoint.request_id.as_str(),
            "request-endpoint-json-001"
        );
        assert_eq!(
            from_endpoint.outcome,
            RuntimeControlPlaneMessageOutcome::Success
        );
        assert_eq!(
            from_endpoint
                .snapshot
                .as_ref()
                .unwrap()
                .runtime_summary
                .workspace_id
                .as_str(),
            "fixture-workspace-alpha"
        );
    }

    #[test]
    fn dispatches_runtime_control_plane_file_message_ipc_stream() {
        let root = temp_policy_root("valid-file-ipc");
        let path = write_test_file(
            &root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        let request_json = file_message_request("request-ipc-file-001", &path, &root);
        let (result, response_bytes) = execute_ipc_frame_bytes(request_json.as_bytes());

        result.unwrap();
        let from_ipc = response_from_ipc_bytes(&response_bytes);
        let from_frame = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap(),
        );

        assert_eq!(from_ipc, from_frame);
        assert_eq!(from_ipc.request_id.as_str(), "request-ipc-file-001");
        assert_eq!(from_ipc.outcome, RuntimeControlPlaneMessageOutcome::Success);
        assert_eq!(
            from_ipc
                .snapshot
                .as_ref()
                .unwrap()
                .model_registry_metadata
                .entries[9]
                .model_id,
            "time_series_residual"
        );

        remove_temp_root(&root);
    }

    #[test]
    fn dispatches_runtime_control_plane_file_message_endpoint_stream() {
        let root = temp_policy_root("valid-file-endpoint");
        let path = write_test_file(
            &root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        let request_json = file_message_request("request-endpoint-file-001", &path, &root);
        let (result, response_bytes) = execute_endpoint_frame_bytes(request_json.as_bytes());

        result.unwrap();
        let from_endpoint = response_from_ipc_bytes(&response_bytes);
        let from_frame = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap(),
        );

        assert_eq!(from_endpoint, from_frame);
        assert_eq!(
            from_endpoint.request_id.as_str(),
            "request-endpoint-file-001"
        );
        assert_eq!(
            from_endpoint.outcome,
            RuntimeControlPlaneMessageOutcome::Success
        );
        assert_eq!(
            from_endpoint
                .snapshot
                .as_ref()
                .unwrap()
                .model_registry_metadata
                .entries[9]
                .model_id,
            "time_series_residual"
        );

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn dispatches_runtime_control_plane_message_over_unix_stream_pair() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let root = temp_policy_root("valid-unix-ipc");
        let path = write_test_file(
            &root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        let policy = RuntimeControlPlaneIpcPolicy::default();
        let request_json = file_message_request("request-ipc-unix-001", &path, &root);
        let (mut client, server) =
            UnixStream::pair().expect("test connected stream pair must be created");
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            let mut server_stream = server;
            let request_frame =
                read_control_plane_message_ipc_frame(&mut server_stream, &server_policy)?;
            let response_frame =
                RuntimeControlPlaneFrameAdapterContract::execute_control_plane_message_frame_bytes(
                    &request_frame,
                    &server_policy.frame_policy,
                )?;
            write_control_plane_message_ipc_frame(
                &mut server_stream,
                &response_frame,
                &server_policy,
            )
        });

        write_control_plane_message_ipc_frame(&mut client, request_json.as_bytes(), &policy)
            .unwrap();
        server_thread
            .join()
            .expect("test server thread must complete")
            .unwrap();

        let response_frame = read_control_plane_message_ipc_frame(&mut client, &policy).unwrap();
        let from_ipc = response_from_frame_bytes(response_frame);
        let from_frame = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap(),
        );

        assert_eq!(from_ipc, from_frame);
        assert_eq!(from_ipc.request_id.as_str(), "request-ipc-unix-001");
        assert_eq!(from_ipc.outcome, RuntimeControlPlaneMessageOutcome::Success);

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn reports_ipc_write_failures_without_fabricating_responses() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let policy = RuntimeControlPlaneIpcPolicy::default();
        let request_json = json_message_request("request-ipc-write-001", "{");
        let (mut client, server) =
            UnixStream::pair().expect("test connected stream pair must be created");
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            let mut server_stream = server;
            let request_frame =
                read_control_plane_message_ipc_frame(&mut server_stream, &server_policy)?;
            let response_frame =
                RuntimeControlPlaneFrameAdapterContract::execute_control_plane_message_frame_bytes(
                    &request_frame,
                    &server_policy.frame_policy,
                )?;
            write_control_plane_message_ipc_frame(
                &mut server_stream,
                &response_frame,
                &server_policy,
            )
        });

        write_control_plane_message_ipc_frame(&mut client, request_json.as_bytes(), &policy)
            .unwrap();
        drop(client);

        assert_eq!(
            server_thread
                .join()
                .expect("test server thread must complete")
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::IpcWriteFailed
        );
    }

    #[test]
    fn message_ipc_stream_returns_failure_response_for_valid_request_execution_errors() {
        let request_json = json_message_request("request-ipc-006", "{");
        let (result, response_bytes) = execute_ipc_frame_bytes(request_json.as_bytes());

        result.unwrap();
        let response = response_from_ipc_bytes(&response_bytes);

        assert_eq!(response.request_id.as_str(), "request-ipc-006");
        assert_eq!(response.outcome, RuntimeControlPlaneMessageOutcome::Failure);
        assert!(response.snapshot.is_none());
        assert_eq!(
            response.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::InvalidJson)
        );
    }

    #[test]
    fn endpoint_stream_returns_failure_response_for_valid_request_execution_errors() {
        let request_json = json_message_request("request-endpoint-006", "{");
        let (result, response_bytes) = execute_endpoint_frame_bytes(request_json.as_bytes());

        result.unwrap();
        let response = response_from_ipc_bytes(&response_bytes);

        assert_eq!(response.request_id.as_str(), "request-endpoint-006");
        assert_eq!(response.outcome, RuntimeControlPlaneMessageOutcome::Failure);
        assert!(response.snapshot.is_none());
        assert_eq!(
            response.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::InvalidJson)
        );
    }

    #[test]
    fn message_ipc_stream_parse_failures_return_adapter_errors_without_responses() {
        let invalid_schema =
            json_message_request("request-ipc-parse-001", synthetic_handoff_json()).replacen(
                RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
                "runtime_control_plane_message.v1",
                1,
            );
        let (result, response_bytes) = execute_ipc_frame_bytes(invalid_schema.as_bytes());
        assert_eq!(
            result.unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "schema_version",
                expected: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            }
        );
        assert!(response_bytes.is_empty());

        let (result, response_bytes) = execute_ipc_frame_bytes(
            json_message_request("secret-ipc", synthetic_handoff_json()).as_bytes(),
        );
        assert_eq!(
            result.unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "request_id",
            }
        );
        assert!(response_bytes.is_empty());
    }

    #[test]
    fn endpoint_policy_parse_failures_return_adapter_errors_without_responses() {
        let request_json =
            json_message_request("request-endpoint-policy-001", synthetic_handoff_json());

        let mut unsafe_policy = RuntimeControlPlaneEndpointPolicy::default();
        unsafe_policy.socket_listener_enabled = true;
        let (result, response_bytes) =
            execute_endpoint_frame_bytes_with_policy(request_json.as_bytes(), &unsafe_policy);
        assert_eq!(
            result.unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint.socket_listener_enabled",
            }
        );
        assert!(response_bytes.is_empty());

        let mut drifted_policy = RuntimeControlPlaneEndpointPolicy::default();
        drifted_policy.schema_version = "runtime_control_plane_endpoint.v1";
        let (result, response_bytes) =
            execute_endpoint_frame_bytes_with_policy(request_json.as_bytes(), &drifted_policy);
        assert_eq!(
            result.unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "endpoint.schema_version",
                expected: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
            }
        );
        assert!(response_bytes.is_empty());
    }

    #[test]
    fn dispatches_runtime_control_plane_file_message_request() {
        let root = temp_policy_root("valid-file-message");
        let path = write_test_file(
            &root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        let response = RuntimeControlPlaneAdapterContract::execute_control_plane_message_json(
            &file_message_request("request-file-001", &path, &root),
        )
        .unwrap();
        let from_file =
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(&path, &policy)
                .unwrap();

        assert_eq!(response.request_id.as_str(), "request-file-001");
        assert_eq!(response.outcome, RuntimeControlPlaneMessageOutcome::Success);
        assert_eq!(response.snapshot.as_ref().unwrap(), &from_file);
        assert!(response.error_code.is_none());

        remove_temp_root(&root);
    }

    #[test]
    fn dispatches_runtime_handoff_snapshot_file_command() {
        let root = temp_policy_root("valid-file-command");
        let path = write_test_file(
            &root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        let from_command = execute_file_command(path, &policy).unwrap();
        let from_json = RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(
            synthetic_handoff_json(),
        )
        .unwrap();

        assert_eq!(from_command, from_json);
        assert_eq!(
            from_command.model_registry_metadata.entries[9].model_id,
            "time_series_residual"
        );

        remove_temp_root(&root);
    }

    #[test]
    fn local_command_dispatch_fails_closed_for_json_drift() {
        let duplicate_unsafe_key = synthetic_handoff_json().replacen(
            "  \"generated_json_loaded\": false,",
            "  \"generated_json_loaded\": true,\n  \"generated_json_loaded\": false,",
            1,
        );
        let unsafe_flag = patched_json(
            "  \"generated_json_loaded\": false,\n  \"live_runtime_connection\": false",
            "  \"generated_json_loaded\": true,\n  \"live_runtime_connection\": false",
        );
        let schema_drift = patched_json(
            r#""schema_version": "runtime_handoff_snapshot.v0""#,
            r#""schema_version": "runtime_handoff_snapshot.v1""#,
        );
        let registry_drift = patched_json(r#""model_count": 10"#, r#""model_count": 9"#);
        let unsupported_schema_shape = r#"{
  "schema_version": "runtime_summary.v0",
  "workspace_id": "fixture-workspace-alpha",
  "session_id": "fixture-session-runtime-summary"
}"#;

        assert_eq!(
            execute_json_command("{").unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert_eq!(
            execute_json_command(duplicate_unsafe_key).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert_eq!(
            execute_json_command(unsafe_flag).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "generated_json_loaded",
            }
        );
        assert_eq!(
            execute_json_command(schema_drift).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "schema_version",
                expected: RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
            }
        );
        assert_eq!(
            execute_json_command(registry_drift).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.aggregate_summary.model_count",
            }
        );
        assert_eq!(
            execute_json_command(unsupported_schema_shape).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
    }

    #[test]
    fn rejects_unsafe_runtime_control_plane_request_ids() {
        for request_id in [
            "",
            "request.raw",
            "request@example",
            "scheme:/request",
            "private-request",
            "secret_request",
        ] {
            assert_eq!(
                RuntimeControlPlaneAdapterContract::parse_control_plane_message_request_json(
                    &json_message_request(request_id, synthetic_handoff_json()),
                )
                .unwrap_err(),
                RuntimeControlPlaneAdapterError::UnsupportedValue {
                    field: "request_id",
                }
            );
        }

        let too_long_request_id = format!(
            "request-{}",
            "a".repeat(RUNTIME_CONTROL_PLANE_REQUEST_ID_MAX_BYTES)
        );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_control_plane_message_request_json(
                &json_message_request(&too_long_request_id, synthetic_handoff_json()),
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "request_id",
            }
        );
    }

    #[test]
    fn message_request_parsing_fails_closed_for_schema_and_command_drift() {
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_control_plane_message_request_json("{")
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_control_plane_message_request_json("[]")
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::NonObjectRoot
        );

        let unsupported_schema = json_message_request("request-json-002", synthetic_handoff_json())
            .replacen(
                RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
                "runtime_control_plane_message.v1",
                1,
            );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_control_plane_message_request_json(
                &unsupported_schema,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "schema_version",
                expected: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            }
        );

        let unknown_root_field = json_message_request("request-json-003", synthetic_handoff_json())
            .replacen(
                r#"  "request_id": "request-json-003","#,
                r#"  "request_id": "request-json-003",
  "unexpected_field": true,"#,
                1,
            );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_control_plane_message_request_json(
                &unknown_root_field,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        let unsupported_command = json_message_request(
            "request-json-004",
            synthetic_handoff_json(),
        )
        .replacen("parse_handoff_snapshot_json", "open_runtime_service", 1);
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_control_plane_message_request_json(
                &unsupported_command,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "command.command_kind",
            }
        );

        let mixed_command = json_message_request("request-json-005", synthetic_handoff_json())
            .replacen(
                r#"    "input": "#,
                r#"    "path": "/tmp/runtime_handoff_snapshot.json",
    "input": "#,
                1,
            );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_control_plane_message_request_json(
                &mixed_command,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue { field: "command" }
        );
    }

    #[test]
    fn message_frame_parsing_fails_closed_for_invalid_frame_bytes() {
        let policy = RuntimeControlPlaneFramePolicy::default();

        assert_eq!(
            RuntimeControlPlaneFrameAdapterContract::parse_control_plane_message_frame_bytes(
                b"", &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert_eq!(
            RuntimeControlPlaneFrameAdapterContract::parse_control_plane_message_frame_bytes(
                &[0xff],
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );

        let oversized = vec![b' '; RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES + 1];
        assert_eq!(
            RuntimeControlPlaneFrameAdapterContract::parse_control_plane_message_frame_bytes(
                &oversized, &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedFrame {
                max_bytes: RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES,
            }
        );

        let short_policy = RuntimeControlPlaneFramePolicy::new(8).unwrap();
        assert_eq!(
            RuntimeControlPlaneFrameAdapterContract::parse_control_plane_message_frame_bytes(
                br#"{"schema_version":"runtime_control_plane_message.v0"}"#,
                &short_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedFrame { max_bytes: 8 }
        );
    }

    #[test]
    fn message_frame_parsing_preserves_message_envelope_rejections() {
        assert_eq!(
            parse_control_plane_message_frame_bytes(b"{").unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert_eq!(
            parse_control_plane_message_frame_bytes(b"[]").unwrap_err(),
            RuntimeControlPlaneAdapterError::NonObjectRoot
        );

        let unsupported_schema =
            json_message_request("request-frame-002", synthetic_handoff_json()).replacen(
                RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
                "runtime_control_plane_message.v1",
                1,
            );
        assert_eq!(
            parse_control_plane_message_frame_bytes(unsupported_schema.as_bytes()).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "schema_version",
                expected: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            }
        );

        let unknown_root_field =
            json_message_request("request-frame-003", synthetic_handoff_json()).replacen(
                r#"  "request_id": "request-frame-003","#,
                r#"  "request_id": "request-frame-003",
  "unexpected_field": true,"#,
                1,
            );
        assert_eq!(
            parse_control_plane_message_frame_bytes(unknown_root_field.as_bytes()).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        let unsupported_command = json_message_request(
            "request-frame-004",
            synthetic_handoff_json(),
        )
        .replacen("parse_handoff_snapshot_json", "open_runtime_service", 1);
        assert_eq!(
            parse_control_plane_message_frame_bytes(unsupported_command.as_bytes()).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "command.command_kind",
            }
        );

        let mixed_command = json_message_request("request-frame-005", synthetic_handoff_json())
            .replacen(
                r#"    "input": "#,
                r#"    "path": "/tmp/runtime_handoff_snapshot.json",
    "input": "#,
                1,
            );
        assert_eq!(
            parse_control_plane_message_frame_bytes(mixed_command.as_bytes()).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue { field: "command" }
        );

        assert_eq!(
            parse_control_plane_message_frame_bytes(
                json_message_request("secret-frame", synthetic_handoff_json()).as_bytes(),
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "request_id",
            }
        );
    }

    #[test]
    fn message_ipc_stream_fails_closed_for_invalid_ipc_frames() {
        let policy = RuntimeControlPlaneIpcPolicy::default();

        let zero_length = 0_u32.to_be_bytes().to_vec();
        let mut zero_reader = zero_length.as_slice();
        assert_eq!(
            read_control_plane_message_ipc_frame(&mut zero_reader, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        let oversized_length = (RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES as u32 + 1).to_be_bytes();
        let mut oversized_reader = oversized_length.as_slice();
        assert_eq!(
            read_control_plane_message_ipc_frame(&mut oversized_reader, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedFrame {
                max_bytes: RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES,
            }
        );

        let incomplete_prefix_bytes = [0_u8, 0_u8];
        let mut incomplete_prefix = incomplete_prefix_bytes.as_slice();
        let mut response_bytes = Vec::new();
        assert_eq!(
            execute_control_plane_message_ipc_stream(
                &mut incomplete_prefix,
                &mut response_bytes,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::IncompleteIpcFrame
        );
        assert!(response_bytes.is_empty());

        let incomplete_payload_bytes = vec![0_u8, 0_u8, 0_u8, 4_u8, b'{'];
        let mut incomplete_payload = incomplete_payload_bytes.as_slice();
        assert_eq!(
            execute_control_plane_message_ipc_stream(
                &mut incomplete_payload,
                &mut response_bytes,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::IncompleteIpcFrame
        );

        let (result, response_bytes) = execute_ipc_frame_bytes(&[0xff]);
        assert_eq!(
            result.unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );
        assert!(response_bytes.is_empty());

        let (result, response_bytes) = execute_ipc_frame_bytes(b"{");
        assert_eq!(
            result.unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert!(response_bytes.is_empty());

        let (result, response_bytes) = execute_ipc_frame_bytes(b"[]");
        assert_eq!(
            result.unwrap_err(),
            RuntimeControlPlaneAdapterError::NonObjectRoot
        );
        assert!(response_bytes.is_empty());

        let short_policy =
            RuntimeControlPlaneIpcPolicy::new(RuntimeControlPlaneFramePolicy::new(8).unwrap());
        let mut writer = Vec::new();
        assert_eq!(
            write_control_plane_message_ipc_frame(
                &mut writer,
                br#"{"too_long":true}"#,
                &short_policy
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedFrame { max_bytes: 8 }
        );
        assert!(writer.is_empty());
    }

    #[test]
    fn message_execution_returns_failure_responses_for_nested_handoff_rejections() {
        let malformed = RuntimeControlPlaneAdapterContract::execute_control_plane_message_json(
            &json_message_request("request-json-006", "{"),
        )
        .unwrap();
        assert_eq!(malformed.request_id.as_str(), "request-json-006");
        assert_eq!(
            malformed.outcome,
            RuntimeControlPlaneMessageOutcome::Failure
        );
        assert!(malformed.snapshot.is_none());
        assert_eq!(
            malformed.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::InvalidJson)
        );

        let unsafe_flag = RuntimeControlPlaneAdapterContract::execute_control_plane_message_json(
            &json_message_request(
                "request-json-007",
                &patched_json(
                    "  \"generated_json_loaded\": false,\n  \"live_runtime_connection\": false",
                    "  \"generated_json_loaded\": true,\n  \"live_runtime_connection\": false",
                ),
            ),
        )
        .unwrap();
        assert_eq!(
            unsafe_flag.outcome,
            RuntimeControlPlaneMessageOutcome::Failure
        );
        assert_eq!(
            unsafe_flag.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::UnsafeFlag)
        );

        let schema_drift = RuntimeControlPlaneAdapterContract::execute_control_plane_message_json(
            &json_message_request(
                "request-json-008",
                &patched_json(
                    r#""schema_version": "runtime_handoff_snapshot.v0""#,
                    r#""schema_version": "runtime_handoff_snapshot.v1""#,
                ),
            ),
        )
        .unwrap();
        assert_eq!(
            schema_drift.outcome,
            RuntimeControlPlaneMessageOutcome::Failure
        );
        assert_eq!(
            schema_drift.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::UnsupportedSchemaVersion)
        );

        let registry_drift =
            RuntimeControlPlaneAdapterContract::execute_control_plane_message_json(
                &json_message_request(
                    "request-json-009",
                    &patched_json(r#""model_count": 10"#, r#""model_count": 9"#),
                ),
            )
            .unwrap();
        assert_eq!(
            registry_drift.outcome,
            RuntimeControlPlaneMessageOutcome::Failure
        );
        assert_eq!(
            registry_drift.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::UnsupportedValue)
        );
    }

    #[test]
    fn message_frame_execution_returns_failure_response_frames_for_nested_rejections() {
        let malformed = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(
                json_message_request("request-frame-006", "{").as_bytes(),
            )
            .unwrap(),
        );
        assert_eq!(malformed.request_id.as_str(), "request-frame-006");
        assert_eq!(
            malformed.outcome,
            RuntimeControlPlaneMessageOutcome::Failure
        );
        assert_eq!(
            malformed.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::InvalidJson)
        );

        let unsafe_flag = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(
                json_message_request(
                    "request-frame-007",
                    &patched_json(
                        "  \"generated_json_loaded\": false,\n  \"live_runtime_connection\": false",
                        "  \"generated_json_loaded\": true,\n  \"live_runtime_connection\": false",
                    ),
                )
                .as_bytes(),
            )
            .unwrap(),
        );
        assert_eq!(
            unsafe_flag.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::UnsafeFlag)
        );

        let schema_drift = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(
                json_message_request(
                    "request-frame-008",
                    &patched_json(
                        r#""schema_version": "runtime_handoff_snapshot.v0""#,
                        r#""schema_version": "runtime_handoff_snapshot.v1""#,
                    ),
                )
                .as_bytes(),
            )
            .unwrap(),
        );
        assert_eq!(
            schema_drift.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::UnsupportedSchemaVersion)
        );
    }

    #[test]
    fn file_command_dispatch_preserves_file_policy_rejections() {
        let root = temp_policy_root("file-command-policy");
        let outside_root = temp_policy_root("outside-file-command-policy");
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        assert_eq!(
            execute_file_command(PathBuf::from("runtime_handoff_snapshot.json"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeFilePath
        );

        let relative_root_policy = RuntimeControlPlaneFilePolicy::new("relative-root");
        let relative_root_path = write_test_file(
            &root,
            "relative_root_runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        assert_eq!(
            execute_file_command(relative_root_path, &relative_root_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeAllowedRoot
        );

        let missing_root_policy =
            RuntimeControlPlaneFilePolicy::new(root.join("missing-policy-root"));
        let missing_root_path = root
            .join("missing-policy-root")
            .join("runtime_handoff_snapshot.json");
        assert_eq!(
            execute_file_command(missing_root_path, &missing_root_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingAllowedRoot
        );

        let file_root = write_test_file(&root, "file_policy_root.json", synthetic_handoff_json());
        let file_root_policy = RuntimeControlPlaneFilePolicy::new(file_root.clone());
        assert_eq!(
            execute_file_command(file_root, &file_root_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootNotDirectory
        );

        let outside_path = write_test_file(
            &outside_root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        assert_eq!(
            execute_file_command(outside_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OutsideAllowedRoot
        );

        let directory_path = root.join("directory.json");
        std::fs::create_dir_all(&directory_path).expect("test directory path must be created");
        assert_eq!(
            execute_file_command(directory_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::DirectoryPath
        );

        let text_path = write_test_file(&root, "runtime_handoff_snapshot.txt", "{}");
        assert_eq!(
            execute_file_command(text_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedFileExtension
        );

        let missing_path = root.join("missing_handoff_snapshot.json");
        assert_eq!(
            execute_file_command(missing_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingFile
        );

        let oversized_path = write_test_file(
            &root,
            "oversized_runtime_handoff_snapshot.json",
            vec![b' '; RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES as usize + 1],
        );
        assert_eq!(
            execute_file_command(oversized_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedFile {
                max_bytes: RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES,
            }
        );

        let invalid_utf8_path =
            write_test_file(&root, "invalid_utf8_runtime_handoff_snapshot.json", [0xff]);
        assert_eq!(
            execute_file_command(invalid_utf8_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );

        remove_temp_root(&outside_root);
        remove_temp_root(&root);
    }

    #[test]
    fn rejects_handoff_snapshot_file_policy_path_violations() {
        let root = temp_policy_root("path-policy");
        let outside_root = temp_policy_root("outside-policy");
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                Path::new("runtime_handoff_snapshot.json"),
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeFilePath
        );

        let relative_root_policy = RuntimeControlPlaneFilePolicy::new("relative-root");
        let relative_root_path = write_test_file(
            &root,
            "relative_root_runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &relative_root_path,
                &relative_root_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeAllowedRoot
        );

        let missing_root_policy =
            RuntimeControlPlaneFilePolicy::new(root.join("missing-policy-root"));
        let missing_root_path = root
            .join("missing-policy-root")
            .join("runtime_handoff_snapshot.json");
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &missing_root_path,
                &missing_root_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingAllowedRoot
        );

        let file_root = write_test_file(&root, "file_policy_root.json", synthetic_handoff_json());
        let file_root_policy = RuntimeControlPlaneFilePolicy::new(file_root.clone());
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &file_root,
                &file_root_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootNotDirectory
        );

        let outside_path = write_test_file(
            &outside_root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &outside_path,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::OutsideAllowedRoot
        );

        let directory_path = root.join("directory.json");
        std::fs::create_dir_all(&directory_path).expect("test directory path must be created");
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &directory_path,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::DirectoryPath
        );

        let text_path = write_test_file(&root, "runtime_handoff_snapshot.txt", "{}");
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(&text_path, &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedFileExtension
        );

        let missing_path = root.join("missing_handoff_snapshot.json");
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &missing_path,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingFile
        );

        let oversized_path = write_test_file(
            &root,
            "oversized_runtime_handoff_snapshot.json",
            vec![b' '; RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES as usize + 1],
        );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &oversized_path,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedFile {
                max_bytes: RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES,
            }
        );

        remove_temp_root(&outside_root);
        remove_temp_root(&root);
    }

    #[test]
    fn rejects_model_registry_metadata_file_policy_path_violations() {
        let root = temp_policy_root("metadata-path-policy");
        let outside_root = temp_policy_root("outside-metadata-policy");
        let policy = ModelRegistryMetadataAdapterPolicy::new(root.clone());

        assert_eq!(
            parse_model_registry_metadata_file(Path::new("model_registry_metadata.json"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeFilePath
        );

        let relative_root_policy = ModelRegistryMetadataAdapterPolicy::new("relative-root");
        let relative_root_path = write_test_file(
            &root,
            "relative_root_model_registry_metadata.json",
            synthetic_model_registry_metadata_json(),
        );
        assert_eq!(
            parse_model_registry_metadata_file(&relative_root_path, &relative_root_policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeAllowedRoot
        );

        let missing_root_policy =
            ModelRegistryMetadataAdapterPolicy::new(root.join("missing-policy-root"));
        let missing_root_path = root
            .join("missing-policy-root")
            .join("model_registry_metadata.json");
        assert_eq!(
            parse_model_registry_metadata_file(&missing_root_path, &missing_root_policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingAllowedRoot
        );

        let file_root = write_test_file(
            &root,
            "file_policy_root.json",
            synthetic_model_registry_metadata_json(),
        );
        let file_root_policy = ModelRegistryMetadataAdapterPolicy::new(file_root.clone());
        assert_eq!(
            parse_model_registry_metadata_file(&file_root, &file_root_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootNotDirectory
        );

        let outside_path = write_test_file(
            &outside_root,
            "model_registry_metadata.json",
            synthetic_model_registry_metadata_json(),
        );
        assert_eq!(
            parse_model_registry_metadata_file(&outside_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OutsideAllowedRoot
        );

        let directory_path = root.join("directory.json");
        std::fs::create_dir_all(&directory_path).expect("test directory path must be created");
        assert_eq!(
            parse_model_registry_metadata_file(&directory_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::DirectoryPath
        );

        let text_path = write_test_file(&root, "model_registry_metadata.txt", "{}");
        assert_eq!(
            parse_model_registry_metadata_file(&text_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedFileExtension
        );

        let missing_path = root.join("missing_model_registry_metadata.json");
        assert_eq!(
            parse_model_registry_metadata_file(&missing_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingFile
        );

        let oversized_path = write_test_file(
            &root,
            "oversized_model_registry_metadata.json",
            vec![b' '; RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES as usize + 1],
        );
        assert_eq!(
            parse_model_registry_metadata_file(&oversized_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedFile {
                max_bytes: RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES,
            }
        );

        let malformed_path = write_test_file(&root, "malformed_model_registry_metadata.json", "{");
        assert_eq!(
            parse_model_registry_metadata_file(&malformed_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        let invalid_utf8_path =
            write_test_file(&root, "invalid_utf8_model_registry_metadata.json", [0xff]);
        assert_eq!(
            parse_model_registry_metadata_file(&invalid_utf8_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );

        remove_temp_root(&outside_root);
        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_allowed_root() {
        let root = temp_policy_root("allowed-root-symlink-policy");
        let real_root = root.join("real-root");
        std::fs::create_dir_all(&real_root).expect("test real root must be created");
        let symlink_root = root.join("symlink-root");
        std::os::unix::fs::symlink(&real_root, &symlink_root)
            .expect("test allowed root symlink must be created");
        let path = write_test_file(
            &real_root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        let policy = RuntimeControlPlaneFilePolicy::new(symlink_root);

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(&path, &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootSymlink
        );
        assert_eq!(
            execute_file_command(path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootSymlink
        );

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_handoff_snapshot_file() {
        let root = temp_policy_root("symlink-policy");
        let target_path = write_test_file(
            &root,
            "target_runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        let symlink_path = root.join("linked_runtime_handoff_snapshot.json");
        std::os::unix::fs::symlink(&target_path, &symlink_path)
            .expect("test symlink must be created");
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &symlink_path,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::SymlinkPath
        );
        assert_eq!(
            execute_file_command(symlink_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::SymlinkPath
        );

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_model_registry_metadata_file() {
        let root = temp_policy_root("metadata-symlink-policy");
        let target_path = write_test_file(
            &root,
            "target_model_registry_metadata.json",
            synthetic_model_registry_metadata_json(),
        );
        let symlink_path = root.join("linked_model_registry_metadata.json");
        std::os::unix::fs::symlink(&target_path, &symlink_path)
            .expect("test symlink must be created");
        let policy = ModelRegistryMetadataAdapterPolicy::new(root.clone());

        assert_eq!(
            parse_model_registry_metadata_file(&symlink_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::SymlinkPath
        );

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_non_regular_handoff_snapshot_file() {
        let root = temp_policy_root("non-regular-policy");
        let fifo_path = root.join("runtime_handoff_snapshot.json");
        make_fifo(&fifo_path);
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(&fifo_path, &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::NonRegularFile
        );
        assert_eq!(
            execute_file_command(fifo_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::NonRegularFile
        );

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn reports_file_read_failures_after_regular_file_policy_checks() {
        if effective_user_id_is_root() {
            return;
        }

        use std::os::unix::fs::PermissionsExt;

        let root = temp_policy_root("file-read-failure-policy");
        let path = write_test_file(
            &root,
            "runtime_handoff_snapshot.json",
            synthetic_handoff_json(),
        );
        let mut permissions = std::fs::metadata(&path)
            .expect("test handoff file metadata must be readable")
            .permissions();
        permissions.set_mode(0o000);
        std::fs::set_permissions(&path, permissions)
            .expect("test handoff file permissions must be changed");
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(&path, &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::FileReadFailed
        );
        assert_eq!(
            execute_file_command(path.clone(), &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::FileReadFailed
        );

        let mut restored_permissions = std::fs::metadata(&path)
            .expect("test handoff file metadata must still be readable")
            .permissions();
        restored_permissions.set_mode(0o600);
        std::fs::set_permissions(&path, restored_permissions)
            .expect("test handoff file permissions must be restored");
        remove_temp_root(&root);
    }

    #[test]
    fn delegates_handoff_snapshot_file_contents_to_strict_json_parser() {
        let root = temp_policy_root("strict-content");
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        let malformed_path = write_test_file(&root, "malformed_runtime_handoff_snapshot.json", "{");
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &malformed_path,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        let duplicate_unsafe_key = synthetic_handoff_json().replacen(
            "  \"generated_json_loaded\": false,",
            "  \"generated_json_loaded\": true,\n  \"generated_json_loaded\": false,",
            1,
        );
        let duplicate_path = write_test_file(
            &root,
            "duplicate_runtime_handoff_snapshot.json",
            duplicate_unsafe_key,
        );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &duplicate_path,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        let unsafe_flag_path = write_test_file(
            &root,
            "unsafe_runtime_handoff_snapshot.json",
            patched_json(
                "  \"local_only\": true,\n  \"static_synthetic_fixture\": true",
                "  \"local_only\": false,\n  \"static_synthetic_fixture\": true",
            ),
        );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &unsafe_flag_path,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "local_only",
            }
        );

        let drift_path = write_test_file(
            &root,
            "drift_runtime_handoff_snapshot.json",
            patched_json(r#""model_count": 10"#, r#""model_count": 9"#),
        );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(&drift_path, &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.aggregate_summary.model_count",
            }
        );

        let invalid_utf8_path =
            write_test_file(&root, "invalid_utf8_runtime_handoff_snapshot.json", [0xff]);
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_file(
                &invalid_utf8_path,
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );

        remove_temp_root(&root);
    }

    #[test]
    fn rejects_malformed_or_drifted_model_registry_metadata_json_strings() {
        assert_eq!(
            parse_model_registry_metadata_json("{").unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert_eq!(
            parse_model_registry_metadata_json("[]").unwrap_err(),
            RuntimeControlPlaneAdapterError::NonObjectRoot
        );

        let with_unknown_field = synthetic_model_registry_metadata_json().replacen(
            "{\n",
            "{\n  \"unexpected_field\": true,\n",
            1,
        );
        assert_eq!(
            parse_model_registry_metadata_json(&with_unknown_field).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                r#""schema_version": "model_registry_metadata.v0""#,
                r#""schema_version": "model_registry_metadata.v1""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "schema_version",
                expected: MODEL_REGISTRY_METADATA_SCHEMA_VERSION,
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                r#""metadata_scope": "local_synthetic_model_registry_metadata""#,
                r#""metadata_scope": "private_registry_metadata""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.metadata_scope",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                r#""source_bundle_schema": "model_evaluation_bundle.v0""#,
                r#""source_bundle_schema": "model_evaluation_bundle.v1""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.source_bundle_schema",
            }
        );
    }

    #[test]
    fn rejects_unsorted_or_unsafe_model_registry_metadata_entries() {
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                r#""model_id": "graph_novelty""#,
                r#""model_id": "isolation_forest""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.entries",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                r#""model_id": "graph_novelty""#,
                r#""model_id": "graph.novelty""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.entries.model_id",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&secret_model_registry_metadata_json()).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.entries.model_id",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                r#""temporal_security_graph_report_v0_001""#,
                r#""temporal_security_graph_report.json""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.entries.observed_source_names",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                r#""detection_candidate_report_v0_001",
        "model_disagreement_report_v0_001""#,
                r#""detection_candidate_report_v0_001",
        "detection_candidate_report_v0_001""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.entries.observed_source_names",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                r#""temporal_security_graph_report_v0_001""#,
                r#""password_001""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.entries.observed_source_names",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                r#""model_disagreement_report.v0""#,
                r#""private_report.v0""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.entries.observed_source_schemas",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                r#""model_count": 10"#,
                r#""model_count": 9"#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.aggregate_summary.model_count",
            }
        );
    }

    #[test]
    fn rejects_model_registry_metadata_unsafe_flags() {
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                "      \"human_review_required\": true,\n      \"deployment_allowed\": false",
                "      \"human_review_required\": true,\n      \"deployment_allowed\": true",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "model_registry_metadata.entries.deployment_allowed",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                "    \"deployment_allowed\": false\n  },",
                "    \"deployment_allowed\": true\n  },",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "model_registry_metadata.aggregate_summary.deployment_allowed",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                "    \"live_capture_used\": false,\n    \"external_services_used\": false",
                "    \"live_capture_used\": true,\n    \"external_services_used\": false",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "model_registry_metadata.safety_flags.live_capture_used",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                "    \"live_capture_used\": false,\n    \"external_services_used\": false",
                "    \"live_capture_used\": false,\n    \"external_services_used\": true",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "model_registry_metadata.safety_flags.external_services_used",
            }
        );
        assert_eq!(
            parse_model_registry_metadata_json(&patched_metadata_json(
                "    \"external_services_used\": false,\n    \"deployment_allowed\": false",
                "    \"external_services_used\": false,\n    \"deployment_allowed\": true",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "model_registry_metadata.safety_flags.deployment_allowed",
            }
        );
    }

    #[test]
    fn rejects_malformed_or_non_object_json_strings() {
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json("{").unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json("[]").unwrap_err(),
            RuntimeControlPlaneAdapterError::NonObjectRoot
        );
    }

    #[test]
    fn rejects_unknown_schema_versions() {
        let err = RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
            r#""schema_version": "runtime_handoff_snapshot.v0""#,
            r#""schema_version": "runtime_handoff_snapshot.v1""#,
        ))
        .unwrap_err();

        assert_eq!(
            err,
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "schema_version",
                expected: RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
            }
        );
    }

    #[test]
    fn rejects_mismatched_nested_schema_versions() {
        let err = RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
            r#""schema_version": "runtime_summary.v0""#,
            r#""schema_version": "runtime_summary.v1""#,
        ))
        .unwrap_err();

        assert_eq!(
            err,
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "runtime_summary.schema_version",
                expected: RUNTIME_SUMMARY_SCHEMA_VERSION,
            }
        );

        let err = RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
            r#""schema_version": "model_registry_metadata.v0""#,
            r#""schema_version": "model_registry_metadata.v1""#,
        ))
        .unwrap_err();

        assert_eq!(
            err,
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "model_registry_metadata.schema_version",
                expected: MODEL_REGISTRY_METADATA_SCHEMA_VERSION,
            }
        );
    }

    #[test]
    fn rejects_unknown_fields_and_enum_values() {
        let with_unknown_field =
            synthetic_handoff_json().replacen("{\n", "{\n  \"unexpected_field\": true,\n", 1);
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&with_unknown_field)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        let duplicate_unsafe_key = synthetic_handoff_json().replacen(
            "  \"generated_json_loaded\": false,",
            "  \"generated_json_loaded\": true,\n  \"generated_json_loaded\": false,",
            1,
        );
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&duplicate_unsafe_key)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                r#""source_kind": "static_synthetic_fixture""#,
                r#""source_kind": "live_runtime""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
    }

    #[test]
    fn rejects_unsafe_ids_and_safety_flags() {
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                r#""workspace_id": "fixture-workspace-alpha""#,
                r#""workspace_id": "analyst@example""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                "  \"local_only\": true,\n  \"static_synthetic_fixture\": true",
                "  \"local_only\": false,\n  \"static_synthetic_fixture\": true",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "local_only",
            }
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                "  \"generated_json_loaded\": false,\n  \"live_runtime_connection\": false",
                "  \"generated_json_loaded\": true,\n  \"live_runtime_connection\": false",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "generated_json_loaded",
            }
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                r#""live_runtime_connection": false"#,
                r#""live_runtime_connection": true"#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "live_runtime_connection",
            }
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                "  \"external_services_used\": false,\n  \"deployment_allowed\": false",
                "  \"external_services_used\": true,\n  \"deployment_allowed\": false",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "external_services_used",
            }
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                "  \"external_services_used\": false,\n  \"deployment_allowed\": false",
                "  \"external_services_used\": false,\n  \"deployment_allowed\": true",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "deployment_allowed",
            }
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                "        \"human_review_required\": true,\n        \"deployment_allowed\": false",
                "        \"human_review_required\": true,\n        \"deployment_allowed\": true",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "model_registry_metadata.entries.deployment_allowed",
            }
        );
    }

    #[test]
    fn rejects_caller_controlled_private_or_unexpected_registry_strings() {
        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                r#""last_event_label": "synthetic workstation snapshot rendered""#,
                r#""last_event_label": "analyst@example""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_summary.last_event_label",
            }
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                r#""model_id": "model_disagreement""#,
                r#""model_id": "isolation_forest""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.entries",
            }
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                r#""model_disagreement_report.v0""#,
                r#""private_report.v0""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.entries.observed_source_schemas",
            }
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                r#""model_count": 10"#,
                r#""model_count": 9"#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.aggregate_summary.model_count",
            }
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                r#""schemas_present": [
        "agentic_investigation_report.v0",
        "detection_candidate_report.v0",
        "model_disagreement_report.v0",
        "model_score_rows.v0",
        "temporal_security_graph_report.v0",
        "time_series_residual_report.v0",
        "traffic_representation_report.v0"
      ]"#,
                r#""schemas_present": [
        "agentic_investigation_report.v0",
        "detection_candidate_report.v0",
        "private_report.v0",
        "model_score_rows.v0",
        "temporal_security_graph_report.v0",
        "time_series_residual_report.v0",
        "traffic_representation_report.v0"
      ]"#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.aggregate_summary.schemas_present",
            }
        );

        assert_eq!(
            RuntimeControlPlaneAdapterContract::parse_handoff_snapshot_json(&patched_json(
                r#""models_with_score_rows": [
        "graph_novelty",
        "isolation_forest",
        "pyod_copod",
        "pyod_ecod",
        "river_hst",
        "stdlib_linear_native",
        "suricata_alert",
        "time_series_residual"
      ]"#,
                r#""models_with_score_rows": [
        "graph_novelty",
        "isolation_forest",
        "pyod_copod",
        "pyod_ecod",
        "river_hst",
        "stdlib_linear_native",
        "suricata_alert",
        "private_model"
      ]"#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "model_registry_metadata.aggregate_summary.models_with_score_rows",
            }
        );
    }
}
