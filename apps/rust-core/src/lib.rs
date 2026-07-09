use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

#[cfg(unix)]
unsafe extern "C" {
    fn geteuid() -> u32;
}

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
pub const EVIDENCE_INDEX_SCHEMA_VERSION: &str = "evidence_index.v0";
pub const EVIDENCE_INDEX_SCOPE: &str = "local_synthetic_evidence_pointer_index";
pub const EVIDENCE_INDEX_ADAPTER_SCHEMA_VERSION: &str = "evidence_index_adapter.v0";
pub const RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION: &str = "runtime_handoff_snapshot.v0";
pub const RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION: &str = "runtime_workstation_snapshot.v0";
pub const RUNTIME_WORKSTATION_SNAPSHOT_PROVIDER_SCHEMA_VERSION: &str =
    "runtime_workstation_snapshot_provider.v0";
pub const RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_SCHEMA_VERSION: &str =
    "runtime_workstation_snapshot_service.v0";
pub const RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_DEFAULT_EVENT_CAP: usize = 16;
pub const RUNTIME_CONTROL_PLANE_ADAPTER_SCHEMA_VERSION: &str = "runtime_control_plane_adapter.v0";
pub const RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION: &str = "runtime_control_plane_endpoint.v0";
pub const RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION: &str =
    "runtime_control_plane_endpoint_path.v0";
pub const RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION: &str =
    "runtime_control_plane_endpoint_listener.v0";
pub const RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_SCHEMA_VERSION: &str =
    "runtime_control_plane_endpoint_lifecycle.v0";
pub const RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_SCHEMA_VERSION: &str =
    "runtime_control_plane_service_lifecycle.v0";
pub const RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION: &str = "runtime_control_plane_frame.v0";
pub const RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION: &str = "runtime_control_plane_ipc.v0";
pub const RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION: &str = "runtime_control_plane_message.v0";
pub const RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES: u64 = 256 * 1024;
pub const RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES: usize = 107;
pub const RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES: usize = 256 * 1024;
pub const RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_DEFAULT_EVENT_CAP: usize = 16;
pub const RUNTIME_CONTROL_PLANE_IPC_LENGTH_PREFIX_BYTES: usize = 4;
pub const RUNTIME_CONTROL_PLANE_REQUEST_ID_MAX_BYTES: usize = 96;
#[cfg(unix)]
const RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_DIRECTORY_MODE_MASK: u32 = 0o077;
#[cfg(unix)]
const RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SOCKET_MODE: u32 = 0o600;

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
    OversizedPath {
        max_bytes: usize,
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
    EndpointBindFailed,
    EndpointAcceptFailed,
    EndpointCleanupFailed,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceIndex {
    pub schema_version: String,
    pub index_scope: String,
    pub source_summaries: Vec<EvidenceIndexSourceSummary>,
    pub entity_window_index: Vec<EvidenceIndexEntityWindow>,
    pub aggregate_summary: EvidenceIndexAggregateSummary,
    pub safety_flags: EvidenceIndexSafetyFlags,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceIndexSourceSummary {
    pub source_name: String,
    pub source_schema: String,
    pub row_count: u32,
    pub entity_window_count: u32,
    pub source_ref_count: u32,
    pub evidence_ref_count: u32,
    pub feature_count: u32,
    pub model_count: u32,
    pub feature_names: Vec<String>,
    pub model_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceIndexEntityWindow {
    pub entity_id: String,
    pub window_start: String,
    pub source_refs: Vec<EvidenceIndexSourceRef>,
    pub feature_names: Vec<String>,
    pub model_ids: Vec<String>,
    pub source_ref_count: u32,
    pub evidence_ref_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceIndexSourceRef {
    pub source_name: String,
    pub source_schema: String,
    pub row_index: u32,
    pub row_kind: String,
    pub feature_names: Vec<String>,
    pub model_ids: Vec<String>,
    pub evidence_indexes: Vec<EvidenceIndexEvidenceRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceIndexEvidenceRef {
    pub model_id: String,
    pub evidence_index: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceIndexAggregateSummary {
    pub source_count: u32,
    pub schemas_present: Vec<String>,
    pub source_count_by_schema: BTreeMap<String, u32>,
    pub row_count_by_schema: BTreeMap<String, u32>,
    pub entity_count: u32,
    pub entity_window_count: u32,
    pub source_ref_count: u32,
    pub evidence_ref_count: u32,
    pub feature_count: u32,
    pub model_count: u32,
    pub feature_names: Vec<String>,
    pub model_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceIndexSafetyFlags {
    pub local_only: bool,
    pub strict_json_loaded: bool,
    pub pointer_only: bool,
    pub input_paths_copied: bool,
    pub source_filenames_copied: bool,
    pub raw_evidence_payload_copied: bool,
    pub raw_identifiers_copied: bool,
    pub generated_artifact_references_copied: bool,
    pub secrets_detected: bool,
    pub capture_claims_copied: bool,
    pub live_capture_used: bool,
    pub external_service_claims_copied: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceIndexAdapterContract {
    pub schema_version: &'static str,
    pub accepted_index_schema: &'static str,
    pub accepted_index_scope: &'static str,
    pub max_file_bytes: u64,
    pub local_only: bool,
    pub pointer_only_index: bool,
    pub strict_json_parsing_enabled: bool,
    pub file_io_enabled: bool,
    pub storage_provider_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub raw_evidence_payload_loading_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceIndexAdapterPolicy {
    pub file_policy: RuntimeControlPlaneFilePolicy,
    pub local_only: bool,
    pub pointer_only_index: bool,
    pub storage_provider_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub raw_evidence_payload_loading_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeWorkstationSnapshot {
    pub schema_version: String,
    pub runtime_handoff_snapshot: RuntimeHandoffSnapshot,
    pub evidence_index: EvidenceIndex,
    pub aggregate_summary: RuntimeWorkstationSnapshotAggregateSummary,
    pub safety_flags: RuntimeWorkstationSnapshotSafetyFlags,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeWorkstationSnapshotAggregateSummary {
    pub workspace_id: WorkspaceId,
    pub session_id: SessionId,
    pub runtime_total_job_count: u32,
    pub runtime_queued_job_count: u32,
    pub runtime_running_job_count: u32,
    pub runtime_failed_job_count: u32,
    pub registry_model_count: u32,
    pub registry_models_with_score_rows_count: u32,
    pub evidence_source_count: u32,
    pub evidence_entity_count: u32,
    pub evidence_entity_window_count: u32,
    pub evidence_source_ref_count: u32,
    pub evidence_ref_count: u32,
    pub source_schema_count: u32,
    pub feature_count: u32,
    pub evidence_model_count: u32,
    pub model_count: u32,
    pub source_schemas: Vec<String>,
    pub feature_names: Vec<String>,
    pub model_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeWorkstationSnapshotSafetyFlags {
    pub local_only: bool,
    pub strict_json_loaded: bool,
    pub caller_provided_snapshots_only: bool,
    pub validated_runtime_handoff_snapshot: bool,
    pub validated_evidence_index: bool,
    pub pointer_only_evidence: bool,
    pub generated_json_loaded: bool,
    pub raw_evidence_payload_copied: bool,
    pub live_runtime_connection: bool,
    pub file_io_enabled: bool,
    pub storage_provider_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWorkstationSnapshotProviderContract {
    pub schema_version: &'static str,
    pub output_snapshot_schema: &'static str,
    pub accepted_handoff_snapshot_schema: &'static str,
    pub accepted_evidence_index_schema: &'static str,
    pub local_only: bool,
    pub in_memory_only: bool,
    pub caller_provided_snapshots_only: bool,
    pub strict_runtime_handoff_validation_enabled: bool,
    pub strict_evidence_index_validation_enabled: bool,
    pub derived_aggregate_validation_enabled: bool,
    pub pointer_only_evidence_required: bool,
    pub file_io_enabled: bool,
    pub storage_provider_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub raw_evidence_payload_loading_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
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
pub struct RuntimeWorkstationSnapshotProviderPolicy {
    pub local_only: bool,
    pub in_memory_only: bool,
    pub caller_provided_snapshots_only: bool,
    pub strict_runtime_handoff_validation_enabled: bool,
    pub strict_evidence_index_validation_enabled: bool,
    pub derived_aggregate_validation_enabled: bool,
    pub pointer_only_evidence_required: bool,
    pub file_io_enabled: bool,
    pub storage_provider_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub raw_evidence_payload_loading_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeWorkstationSnapshotServiceState {
    Stopped,
    Starting,
    Running,
    RefreshingSnapshot,
    Stopping,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeWorkstationSnapshotServiceEventKind {
    StartRequested,
    SnapshotAccepted,
    RefreshRequested,
    SnapshotRefreshed,
    StopRequested,
    Stopped,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWorkstationSnapshotServiceEvent {
    pub schema_version: String,
    pub event_index: u32,
    pub state: RuntimeWorkstationSnapshotServiceState,
    pub event_kind: RuntimeWorkstationSnapshotServiceEventKind,
    pub event_label: &'static str,
    pub snapshot_schema_version: String,
    pub local_only: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWorkstationSnapshotServiceStatus {
    pub schema_version: String,
    pub accepted_snapshot_schema: String,
    pub final_state: RuntimeWorkstationSnapshotServiceState,
    pub latest_snapshot: Option<RuntimeWorkstationSnapshot>,
    pub accepted_snapshot_count: u32,
    pub event_cap: usize,
    pub events: Vec<RuntimeWorkstationSnapshotServiceEvent>,
    pub local_only: bool,
    pub in_memory_only: bool,
    pub service_state_enabled: bool,
    pub explicit_start_stop_enabled: bool,
    pub snapshot_refresh_enabled: bool,
    pub audit_events_enabled: bool,
    pub capped_in_memory_events_enabled: bool,
    pub validates_snapshot_before_accept: bool,
    pub caller_provided_snapshots_only: bool,
    pub file_io_enabled: bool,
    pub storage_provider_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub raw_evidence_payload_loading_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub listener_loop_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub async_stop_api_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWorkstationSnapshotServiceContract {
    pub schema_version: &'static str,
    pub accepted_snapshot_schema: &'static str,
    pub default_event_cap: usize,
    pub local_only: bool,
    pub in_memory_only: bool,
    pub service_state_enabled: bool,
    pub explicit_start_stop_enabled: bool,
    pub snapshot_refresh_enabled: bool,
    pub audit_events_enabled: bool,
    pub capped_in_memory_events_enabled: bool,
    pub validates_snapshot_before_accept: bool,
    pub caller_provided_snapshots_only: bool,
    pub file_io_enabled: bool,
    pub storage_provider_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub raw_evidence_payload_loading_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub listener_loop_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub async_stop_api_enabled: bool,
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
pub struct RuntimeWorkstationSnapshotServicePolicy {
    pub event_cap: usize,
    pub local_only: bool,
    pub in_memory_only: bool,
    pub service_state_enabled: bool,
    pub explicit_start_stop_enabled: bool,
    pub snapshot_refresh_enabled: bool,
    pub audit_events_enabled: bool,
    pub capped_in_memory_events_enabled: bool,
    pub validates_snapshot_before_accept: bool,
    pub caller_provided_snapshots_only: bool,
    pub file_io_enabled: bool,
    pub storage_provider_enabled: bool,
    pub database_or_indexing_enabled: bool,
    pub generated_report_loading_enabled: bool,
    pub generated_json_loading_enabled: bool,
    pub raw_evidence_payload_loading_enabled: bool,
    pub live_transport_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub listener_loop_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub async_stop_api_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWorkstationSnapshotServiceSupervisor {
    state: RuntimeWorkstationSnapshotServiceState,
    event_cap: usize,
    latest_snapshot: Option<RuntimeWorkstationSnapshot>,
    accepted_snapshot_count: u32,
    events: Vec<RuntimeWorkstationSnapshotServiceEvent>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneEndpointPathContract {
    pub schema_version: &'static str,
    pub endpoint_schema_version: &'static str,
    pub max_path_bytes: usize,
    pub local_only: bool,
    pub caller_authorized_allowed_root_required: bool,
    pub absolute_allowed_root_required: bool,
    pub absolute_endpoint_path_required: bool,
    pub allowed_root_must_exist: bool,
    pub allowed_root_symlink_rejected: bool,
    pub target_parent_must_exist: bool,
    pub target_parent_symlink_rejected: bool,
    pub target_must_not_exist: bool,
    pub socket_extension_required: bool,
    pub endpoint_filename_safety_enabled: bool,
    pub path_selection_only: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub filesystem_metadata_validation_enabled: bool,
    pub filesystem_mutation_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
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
pub struct RuntimeControlPlaneEndpointPathPolicy {
    pub allowed_root: PathBuf,
    pub max_path_bytes: usize,
    pub local_only: bool,
    pub caller_authorized_allowed_root_required: bool,
    pub path_selection_only: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub filesystem_mutation_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeControlPlaneEndpointPathSelection {
    pub schema_version: String,
    pub endpoint_schema_version: String,
    pub endpoint_path: String,
    pub allowed_root: String,
    pub endpoint_filename: String,
    pub max_path_bytes: usize,
    pub local_only: bool,
    pub caller_authorized_allowed_root_required: bool,
    pub absolute_endpoint_path: bool,
    pub under_allowed_root: bool,
    pub target_parent_exists: bool,
    pub target_did_not_exist: bool,
    pub socket_extension: String,
    pub path_selection_only: bool,
    pub filesystem_socket_path_policy_enabled: bool,
    pub filesystem_mutation_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub socket_listener_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneEndpointListenerContract {
    pub schema_version: &'static str,
    pub endpoint_schema_version: &'static str,
    pub endpoint_path_schema_version: &'static str,
    pub ipc_schema_version: &'static str,
    pub frame_schema_version: &'static str,
    pub message_schema_version: &'static str,
    pub max_path_bytes: usize,
    pub max_frame_bytes: usize,
    pub local_only: bool,
    pub one_shot_listener: bool,
    pub filesystem_socket_binding_enabled: bool,
    pub cleanup_on_completion: bool,
    pub endpoint_path_validation_enabled: bool,
    pub endpoint_stream_execution_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub listener_loop_enabled: bool,
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
pub struct RuntimeControlPlaneEndpointListenerPolicy {
    pub endpoint_path_policy: RuntimeControlPlaneEndpointPathPolicy,
    pub endpoint_policy: RuntimeControlPlaneEndpointPolicy,
    pub local_only: bool,
    pub one_shot_listener: bool,
    pub filesystem_socket_binding_enabled: bool,
    pub cleanup_on_completion: bool,
    pub endpoint_path_validation_enabled: bool,
    pub endpoint_stream_execution_enabled: bool,
    pub public_network_transport_enabled: bool,
    pub listener_loop_enabled: bool,
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
pub struct RuntimeControlPlaneEndpointListenerOutcome {
    pub schema_version: String,
    pub endpoint_schema_version: String,
    pub endpoint_path_schema_version: String,
    pub endpoint_path_selection: RuntimeControlPlaneEndpointPathSelection,
    pub local_only: bool,
    pub one_shot_listener: bool,
    pub filesystem_socket_binding_enabled: bool,
    pub endpoint_path_validation_enabled: bool,
    pub endpoint_stream_execution_enabled: bool,
    pub cleanup_attempted: bool,
    pub socket_path_removed: bool,
    pub public_network_transport_enabled: bool,
    pub listener_loop_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeControlPlaneEndpointLifecycleState {
    NotStarted,
    StartRequested,
    Listening,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeControlPlaneEndpointLifecycleEventKind {
    StartRequested,
    PathValidated,
    SocketBound,
    ClientAccepted,
    RequestCompleted,
    CleanupCompleted,
    StopRequested,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneEndpointLifecycleEvent {
    pub schema_version: String,
    pub event_index: u32,
    pub state: RuntimeControlPlaneEndpointLifecycleState,
    pub event_kind: RuntimeControlPlaneEndpointLifecycleEventKind,
    pub event_label: &'static str,
    pub local_only: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneEndpointLifecycleContract {
    pub schema_version: &'static str,
    pub listener_schema_version: &'static str,
    pub endpoint_schema_version: &'static str,
    pub endpoint_path_schema_version: &'static str,
    pub ipc_schema_version: &'static str,
    pub frame_schema_version: &'static str,
    pub message_schema_version: &'static str,
    pub max_path_bytes: usize,
    pub max_frame_bytes: usize,
    pub local_only: bool,
    pub one_shot_lifecycle: bool,
    pub start_stop_state_enabled: bool,
    pub audit_events_enabled: bool,
    pub endpoint_listener_execution_enabled: bool,
    pub cleanup_on_completion: bool,
    pub public_network_transport_enabled: bool,
    pub listener_loop_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub persistent_event_store_enabled: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneEndpointLifecyclePolicy {
    pub listener_policy: RuntimeControlPlaneEndpointListenerPolicy,
    pub local_only: bool,
    pub one_shot_lifecycle: bool,
    pub start_stop_state_enabled: bool,
    pub audit_events_enabled: bool,
    pub endpoint_listener_execution_enabled: bool,
    pub cleanup_on_completion: bool,
    pub public_network_transport_enabled: bool,
    pub listener_loop_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub persistent_event_store_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneEndpointLifecycleOutcome {
    pub schema_version: String,
    pub listener_schema_version: String,
    pub endpoint_schema_version: String,
    pub endpoint_path_schema_version: String,
    pub listener_outcome: Option<RuntimeControlPlaneEndpointListenerOutcome>,
    pub final_state: RuntimeControlPlaneEndpointLifecycleState,
    pub failure_error_code: Option<RuntimeControlPlaneMessageErrorCode>,
    pub events: Vec<RuntimeControlPlaneEndpointLifecycleEvent>,
    pub cleanup_attempted: bool,
    pub socket_path_removed: bool,
    pub local_only: bool,
    pub one_shot_lifecycle: bool,
    pub start_stop_state_enabled: bool,
    pub audit_events_enabled: bool,
    pub endpoint_listener_execution_enabled: bool,
    pub cleanup_on_completion: bool,
    pub public_network_transport_enabled: bool,
    pub listener_loop_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub persistent_event_store_enabled: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeControlPlaneServiceLifecycleState {
    Stopped,
    Starting,
    RunningEndpointOnce,
    Stopping,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeControlPlaneServiceLifecycleEventKind {
    StartRequested,
    EndpointLifecycleStarted,
    EndpointLifecycleCompleted,
    StopRequested,
    Stopped,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneServiceLifecycleEvent {
    pub schema_version: String,
    pub event_index: u32,
    pub state: RuntimeControlPlaneServiceLifecycleState,
    pub event_kind: RuntimeControlPlaneServiceLifecycleEventKind,
    pub event_label: &'static str,
    pub local_only: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneServiceLifecycleContract {
    pub schema_version: &'static str,
    pub endpoint_lifecycle_schema_version: &'static str,
    pub listener_schema_version: &'static str,
    pub endpoint_schema_version: &'static str,
    pub endpoint_path_schema_version: &'static str,
    pub ipc_schema_version: &'static str,
    pub frame_schema_version: &'static str,
    pub message_schema_version: &'static str,
    pub default_event_cap: usize,
    pub max_path_bytes: usize,
    pub max_frame_bytes: usize,
    pub local_only: bool,
    pub service_state_enabled: bool,
    pub explicit_start_stop_state_enabled: bool,
    pub one_shot_endpoint_execution_enabled: bool,
    pub audit_events_enabled: bool,
    pub capped_in_memory_events_enabled: bool,
    pub nested_endpoint_lifecycle_execution_enabled: bool,
    pub cleanup_on_completion: bool,
    pub public_network_transport_enabled: bool,
    pub listener_loop_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub async_stop_api_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub persistent_event_store_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneServiceLifecyclePolicy {
    pub endpoint_lifecycle_policy: RuntimeControlPlaneEndpointLifecyclePolicy,
    pub event_cap: usize,
    pub local_only: bool,
    pub service_state_enabled: bool,
    pub explicit_start_stop_state_enabled: bool,
    pub one_shot_endpoint_execution_enabled: bool,
    pub audit_events_enabled: bool,
    pub capped_in_memory_events_enabled: bool,
    pub nested_endpoint_lifecycle_execution_enabled: bool,
    pub cleanup_on_completion: bool,
    pub public_network_transport_enabled: bool,
    pub listener_loop_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub async_stop_api_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub persistent_event_store_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneServiceLifecycleOutcome {
    pub schema_version: String,
    pub endpoint_lifecycle_schema_version: String,
    pub listener_schema_version: String,
    pub endpoint_schema_version: String,
    pub endpoint_path_schema_version: String,
    pub endpoint_lifecycle_outcome: Option<RuntimeControlPlaneEndpointLifecycleOutcome>,
    pub final_state: RuntimeControlPlaneServiceLifecycleState,
    pub failure_error_code: Option<RuntimeControlPlaneMessageErrorCode>,
    pub events: Vec<RuntimeControlPlaneServiceLifecycleEvent>,
    pub event_cap: usize,
    pub cleanup_attempted: bool,
    pub socket_path_removed: bool,
    pub local_only: bool,
    pub service_state_enabled: bool,
    pub explicit_start_stop_state_enabled: bool,
    pub one_shot_endpoint_execution_enabled: bool,
    pub audit_events_enabled: bool,
    pub capped_in_memory_events_enabled: bool,
    pub nested_endpoint_lifecycle_execution_enabled: bool,
    pub cleanup_on_completion: bool,
    pub public_network_transport_enabled: bool,
    pub listener_loop_enabled: bool,
    pub daemon_lifecycle_enabled: bool,
    pub async_stop_api_enabled: bool,
    pub process_spawning_enabled: bool,
    pub file_watching_enabled: bool,
    pub qt_binding_enabled: bool,
    pub storage_provider_enabled: bool,
    pub persistent_event_store_enabled: bool,
    pub capture_enabled: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
    pub native_inference_execution_enabled: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeControlPlaneServiceLifecycleSupervisor {
    state: RuntimeControlPlaneServiceLifecycleState,
    event_cap: usize,
    events: Vec<RuntimeControlPlaneServiceLifecycleEvent>,
}

#[cfg(unix)]
#[derive(Debug, Eq, PartialEq)]
struct RuntimeControlPlaneEndpointListenerFailure {
    error: RuntimeControlPlaneAdapterError,
    cleanup_attempted: bool,
    socket_path_removed: bool,
}

#[cfg(unix)]
#[derive(Debug, Eq, PartialEq)]
enum RuntimeControlPlaneEndpointListenerExecution {
    Succeeded(Box<RuntimeControlPlaneEndpointListenerOutcome>),
    Failed(RuntimeControlPlaneEndpointListenerFailure),
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
    OversizedPath,
    OversizedFrame,
    FileReadFailed,
    FileWriteFailed,
    InvalidUtf8,
    IpcReadFailed,
    IpcWriteFailed,
    MalformedIpcFrame,
    IncompleteIpcFrame,
    EndpointBindFailed,
    EndpointAcceptFailed,
    EndpointCleanupFailed,
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

impl RuntimeControlPlaneEndpointLifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::StartRequested => "start_requested",
            Self::Listening => "listening",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

impl RuntimeControlPlaneEndpointLifecycleEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StartRequested => "start_requested",
            Self::PathValidated => "path_validated",
            Self::SocketBound => "socket_bound",
            Self::ClientAccepted => "client_accepted",
            Self::RequestCompleted => "request_completed",
            Self::CleanupCompleted => "cleanup_completed",
            Self::StopRequested => "stop_requested",
            Self::Failed => "failed",
        }
    }
}

impl RuntimeControlPlaneServiceLifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Starting => "starting",
            Self::RunningEndpointOnce => "running_endpoint_once",
            Self::Stopping => "stopping",
            Self::Failed => "failed",
        }
    }
}

impl RuntimeControlPlaneServiceLifecycleEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StartRequested => "start_requested",
            Self::EndpointLifecycleStarted => "endpoint_lifecycle_started",
            Self::EndpointLifecycleCompleted => "endpoint_lifecycle_completed",
            Self::StopRequested => "stop_requested",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

impl RuntimeWorkstationSnapshotServiceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::RefreshingSnapshot => "refreshing_snapshot",
            Self::Stopping => "stopping",
            Self::Failed => "failed",
        }
    }
}

impl RuntimeWorkstationSnapshotServiceEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StartRequested => "start_requested",
            Self::SnapshotAccepted => "snapshot_accepted",
            Self::RefreshRequested => "refresh_requested",
            Self::SnapshotRefreshed => "snapshot_refreshed",
            Self::StopRequested => "stop_requested",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
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
            Self::OversizedPath => "oversized_path",
            Self::OversizedFrame => "oversized_frame",
            Self::FileReadFailed => "file_read_failed",
            Self::FileWriteFailed => "file_write_failed",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::IpcReadFailed => "ipc_read_failed",
            Self::IpcWriteFailed => "ipc_write_failed",
            Self::MalformedIpcFrame => "malformed_ipc_frame",
            Self::IncompleteIpcFrame => "incomplete_ipc_frame",
            Self::EndpointBindFailed => "endpoint_bind_failed",
            Self::EndpointAcceptFailed => "endpoint_accept_failed",
            Self::EndpointCleanupFailed => "endpoint_cleanup_failed",
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
            RuntimeControlPlaneAdapterError::OversizedPath { .. } => Self::OversizedPath,
            RuntimeControlPlaneAdapterError::OversizedFrame { .. } => Self::OversizedFrame,
            RuntimeControlPlaneAdapterError::FileReadFailed => Self::FileReadFailed,
            RuntimeControlPlaneAdapterError::FileWriteFailed => Self::FileWriteFailed,
            RuntimeControlPlaneAdapterError::InvalidUtf8 => Self::InvalidUtf8,
            RuntimeControlPlaneAdapterError::IpcReadFailed => Self::IpcReadFailed,
            RuntimeControlPlaneAdapterError::IpcWriteFailed => Self::IpcWriteFailed,
            RuntimeControlPlaneAdapterError::MalformedIpcFrame => Self::MalformedIpcFrame,
            RuntimeControlPlaneAdapterError::IncompleteIpcFrame => Self::IncompleteIpcFrame,
            RuntimeControlPlaneAdapterError::EndpointBindFailed => Self::EndpointBindFailed,
            RuntimeControlPlaneAdapterError::EndpointAcceptFailed => Self::EndpointAcceptFailed,
            RuntimeControlPlaneAdapterError::EndpointCleanupFailed => Self::EndpointCleanupFailed,
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

impl EvidenceIndex {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: EVIDENCE_INDEX_SCHEMA_VERSION.to_owned(),
            index_scope: EVIDENCE_INDEX_SCOPE.to_owned(),
            source_summaries: vec![
                EvidenceIndexSourceSummary {
                    source_name: "model_disagreement_report_v0_001".to_owned(),
                    source_schema: "model_disagreement_report.v0".to_owned(),
                    row_count: 1,
                    entity_window_count: 1,
                    source_ref_count: 1,
                    evidence_ref_count: 1,
                    feature_count: 0,
                    model_count: 1,
                    feature_names: vec![],
                    model_ids: static_str_vec(&["isolation_forest"]),
                },
                EvidenceIndexSourceSummary {
                    source_name: "model_score_rows_v0_001".to_owned(),
                    source_schema: "model_score_rows.v0".to_owned(),
                    row_count: 1,
                    entity_window_count: 1,
                    source_ref_count: 1,
                    evidence_ref_count: 2,
                    feature_count: 1,
                    model_count: 2,
                    feature_names: static_str_vec(&["dns_failure_ratio"]),
                    model_ids: static_str_vec(&["isolation_forest", "stdlib_linear_native"]),
                },
            ],
            entity_window_index: vec![EvidenceIndexEntityWindow {
                entity_id: "host-alpha".to_owned(),
                window_start: "2026-01-01T00:00:00Z".to_owned(),
                source_refs: vec![
                    EvidenceIndexSourceRef {
                        source_name: "model_disagreement_report_v0_001".to_owned(),
                        source_schema: "model_disagreement_report.v0".to_owned(),
                        row_index: 0,
                        row_kind: "model_disagreement_row".to_owned(),
                        feature_names: vec![],
                        model_ids: static_str_vec(&["isolation_forest"]),
                        evidence_indexes: vec![EvidenceIndexEvidenceRef {
                            model_id: "isolation_forest".to_owned(),
                            evidence_index: 0,
                        }],
                    },
                    EvidenceIndexSourceRef {
                        source_name: "model_score_rows_v0_001".to_owned(),
                        source_schema: "model_score_rows.v0".to_owned(),
                        row_index: 0,
                        row_kind: "model_score_row".to_owned(),
                        feature_names: static_str_vec(&["dns_failure_ratio"]),
                        model_ids: static_str_vec(&["isolation_forest", "stdlib_linear_native"]),
                        evidence_indexes: vec![
                            EvidenceIndexEvidenceRef {
                                model_id: "isolation_forest".to_owned(),
                                evidence_index: 0,
                            },
                            EvidenceIndexEvidenceRef {
                                model_id: "stdlib_linear_native".to_owned(),
                                evidence_index: 0,
                            },
                        ],
                    },
                ],
                feature_names: static_str_vec(&["dns_failure_ratio"]),
                model_ids: static_str_vec(&["isolation_forest", "stdlib_linear_native"]),
                source_ref_count: 2,
                evidence_ref_count: 3,
            }],
            aggregate_summary: EvidenceIndexAggregateSummary {
                source_count: 2,
                schemas_present: static_str_vec(&[
                    "model_disagreement_report.v0",
                    "model_score_rows.v0",
                ]),
                source_count_by_schema: BTreeMap::from([
                    ("model_disagreement_report.v0".to_owned(), 1),
                    ("model_score_rows.v0".to_owned(), 1),
                ]),
                row_count_by_schema: BTreeMap::from([
                    ("model_disagreement_report.v0".to_owned(), 1),
                    ("model_score_rows.v0".to_owned(), 1),
                ]),
                entity_count: 1,
                entity_window_count: 1,
                source_ref_count: 2,
                evidence_ref_count: 3,
                feature_count: 1,
                model_count: 2,
                feature_names: static_str_vec(&["dns_failure_ratio"]),
                model_ids: static_str_vec(&["isolation_forest", "stdlib_linear_native"]),
            },
            safety_flags: EvidenceIndexSafetyFlags {
                local_only: true,
                strict_json_loaded: true,
                pointer_only: true,
                input_paths_copied: false,
                source_filenames_copied: false,
                raw_evidence_payload_copied: false,
                raw_identifiers_copied: false,
                generated_artifact_references_copied: false,
                secrets_detected: false,
                capture_claims_copied: false,
                live_capture_used: false,
                external_service_claims_copied: false,
                external_services_used: false,
                deployment_allowed: false,
            },
            non_claims: static_str_vec(EVIDENCE_INDEX_NON_CLAIMS),
        }
    }
}

impl EvidenceIndexAdapterContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: EVIDENCE_INDEX_ADAPTER_SCHEMA_VERSION,
            accepted_index_schema: EVIDENCE_INDEX_SCHEMA_VERSION,
            accepted_index_scope: EVIDENCE_INDEX_SCOPE,
            max_file_bytes: RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES,
            local_only: true,
            pointer_only_index: true,
            strict_json_parsing_enabled: true,
            file_io_enabled: true,
            storage_provider_enabled: false,
            generated_report_loading_enabled: false,
            raw_evidence_payload_loading_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: EVIDENCE_INDEX_ADAPTER_NON_CLAIMS,
        }
    }

    pub fn parse_evidence_index_json(
        input: &str,
    ) -> Result<EvidenceIndex, RuntimeControlPlaneAdapterError> {
        parse_evidence_index_json(input)
    }

    pub fn parse_evidence_index_file(
        path: impl AsRef<Path>,
        policy: &EvidenceIndexAdapterPolicy,
    ) -> Result<EvidenceIndex, RuntimeControlPlaneAdapterError> {
        policy.validate()?;
        parse_evidence_index_file(path, &policy.file_policy)
    }
}

impl EvidenceIndexAdapterPolicy {
    pub fn new(allowed_root: impl Into<PathBuf>) -> Self {
        Self::from_file_policy(RuntimeControlPlaneFilePolicy::new(allowed_root))
    }

    pub fn from_file_policy(file_policy: RuntimeControlPlaneFilePolicy) -> Self {
        Self {
            file_policy,
            local_only: true,
            pointer_only_index: true,
            storage_provider_enabled: false,
            generated_report_loading_enabled: false,
            raw_evidence_payload_loading_enabled: false,
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
        validate_required_flag("evidence_index_adapter.local_only", self.local_only, true)?;
        validate_required_flag(
            "evidence_index_adapter.pointer_only_index",
            self.pointer_only_index,
            true,
        )?;
        validate_required_flag(
            "evidence_index_adapter.storage_provider_enabled",
            self.storage_provider_enabled,
            false,
        )?;
        validate_required_flag(
            "evidence_index_adapter.generated_report_loading_enabled",
            self.generated_report_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "evidence_index_adapter.raw_evidence_payload_loading_enabled",
            self.raw_evidence_payload_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "evidence_index_adapter.qt_binding_enabled",
            self.qt_binding_enabled,
            false,
        )?;
        validate_required_flag(
            "evidence_index_adapter.capture_enabled",
            self.capture_enabled,
            false,
        )?;
        validate_required_flag(
            "evidence_index_adapter.external_services_used",
            self.external_services_used,
            false,
        )?;
        validate_required_flag(
            "evidence_index_adapter.deployment_allowed",
            self.deployment_allowed,
            false,
        )?;
        validate_required_flag(
            "evidence_index_adapter.native_inference_execution_enabled",
            self.native_inference_execution_enabled,
            false,
        )
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

impl RuntimeWorkstationSnapshot {
    pub fn synthetic_fixture() -> Self {
        build_runtime_workstation_snapshot(
            RuntimeHandoffSnapshot::synthetic_fixture(),
            EvidenceIndex::synthetic_fixture(),
            &RuntimeWorkstationSnapshotProviderPolicy::new(),
        )
        .expect("static runtime workstation snapshot fixture must validate")
    }
}

impl RuntimeWorkstationSnapshotProviderContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_WORKSTATION_SNAPSHOT_PROVIDER_SCHEMA_VERSION,
            output_snapshot_schema: RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION,
            accepted_handoff_snapshot_schema: RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
            accepted_evidence_index_schema: EVIDENCE_INDEX_SCHEMA_VERSION,
            local_only: true,
            in_memory_only: true,
            caller_provided_snapshots_only: true,
            strict_runtime_handoff_validation_enabled: true,
            strict_evidence_index_validation_enabled: true,
            derived_aggregate_validation_enabled: true,
            pointer_only_evidence_required: true,
            file_io_enabled: false,
            storage_provider_enabled: false,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            raw_evidence_payload_loading_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_WORKSTATION_SNAPSHOT_PROVIDER_NON_CLAIMS,
        }
    }

    pub fn build_runtime_workstation_snapshot(
        handoff_snapshot: RuntimeHandoffSnapshot,
        evidence_index: EvidenceIndex,
        policy: &RuntimeWorkstationSnapshotProviderPolicy,
    ) -> Result<RuntimeWorkstationSnapshot, RuntimeControlPlaneAdapterError> {
        build_runtime_workstation_snapshot(handoff_snapshot, evidence_index, policy)
    }

    pub fn parse_runtime_workstation_snapshot_json(
        input: &str,
    ) -> Result<RuntimeWorkstationSnapshot, RuntimeControlPlaneAdapterError> {
        parse_runtime_workstation_snapshot_json(input)
    }
}

impl RuntimeWorkstationSnapshotProviderPolicy {
    pub fn new() -> Self {
        Self {
            local_only: true,
            in_memory_only: true,
            caller_provided_snapshots_only: true,
            strict_runtime_handoff_validation_enabled: true,
            strict_evidence_index_validation_enabled: true,
            derived_aggregate_validation_enabled: true,
            pointer_only_evidence_required: true,
            file_io_enabled: false,
            storage_provider_enabled: false,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            raw_evidence_payload_loading_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
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
        validate_required_flag(
            "runtime_workstation_snapshot_provider.local_only",
            self.local_only,
            true,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.in_memory_only",
            self.in_memory_only,
            true,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.caller_provided_snapshots_only",
            self.caller_provided_snapshots_only,
            true,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.strict_runtime_handoff_validation_enabled",
            self.strict_runtime_handoff_validation_enabled,
            true,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.strict_evidence_index_validation_enabled",
            self.strict_evidence_index_validation_enabled,
            true,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.derived_aggregate_validation_enabled",
            self.derived_aggregate_validation_enabled,
            true,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.pointer_only_evidence_required",
            self.pointer_only_evidence_required,
            true,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.file_io_enabled",
            self.file_io_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.storage_provider_enabled",
            self.storage_provider_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.database_or_indexing_enabled",
            self.database_or_indexing_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.generated_report_loading_enabled",
            self.generated_report_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.generated_json_loading_enabled",
            self.generated_json_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.raw_evidence_payload_loading_enabled",
            self.raw_evidence_payload_loading_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.live_transport_enabled",
            self.live_transport_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.public_network_transport_enabled",
            self.public_network_transport_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.socket_listener_enabled",
            self.socket_listener_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.daemon_lifecycle_enabled",
            self.daemon_lifecycle_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.process_spawning_enabled",
            self.process_spawning_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.file_watching_enabled",
            self.file_watching_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.qt_binding_enabled",
            self.qt_binding_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.capture_enabled",
            self.capture_enabled,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.external_services_used",
            self.external_services_used,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.deployment_allowed",
            self.deployment_allowed,
            false,
        )?;
        validate_required_flag(
            "runtime_workstation_snapshot_provider.native_inference_execution_enabled",
            self.native_inference_execution_enabled,
            false,
        )
    }
}

impl Default for RuntimeWorkstationSnapshotProviderPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeWorkstationSnapshotServiceContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_SCHEMA_VERSION,
            accepted_snapshot_schema: RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION,
            default_event_cap: RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_DEFAULT_EVENT_CAP,
            local_only: true,
            in_memory_only: true,
            service_state_enabled: true,
            explicit_start_stop_enabled: true,
            snapshot_refresh_enabled: true,
            audit_events_enabled: true,
            capped_in_memory_events_enabled: true,
            validates_snapshot_before_accept: true,
            caller_provided_snapshots_only: true,
            file_io_enabled: false,
            storage_provider_enabled: false,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            raw_evidence_payload_loading_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            listener_loop_enabled: false,
            daemon_lifecycle_enabled: false,
            async_stop_api_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_NON_CLAIMS,
        }
    }

    pub fn execute_once(
        initial_snapshot: RuntimeWorkstationSnapshot,
        refresh_snapshots: &[RuntimeWorkstationSnapshot],
        policy: &RuntimeWorkstationSnapshotServicePolicy,
    ) -> Result<RuntimeWorkstationSnapshotServiceStatus, RuntimeControlPlaneAdapterError> {
        execute_runtime_workstation_snapshot_service_once(
            initial_snapshot,
            refresh_snapshots,
            policy,
        )
    }
}

impl RuntimeWorkstationSnapshotServicePolicy {
    pub fn new() -> Self {
        Self {
            event_cap: RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_DEFAULT_EVENT_CAP,
            local_only: true,
            in_memory_only: true,
            service_state_enabled: true,
            explicit_start_stop_enabled: true,
            snapshot_refresh_enabled: true,
            audit_events_enabled: true,
            capped_in_memory_events_enabled: true,
            validates_snapshot_before_accept: true,
            caller_provided_snapshots_only: true,
            file_io_enabled: false,
            storage_provider_enabled: false,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            raw_evidence_payload_loading_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            listener_loop_enabled: false,
            daemon_lifecycle_enabled: false,
            async_stop_api_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
        }
    }

    pub fn bounded(event_cap: usize) -> Result<Self, RuntimeControlPlaneAdapterError> {
        if event_cap == 0 || event_cap > RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_DEFAULT_EVENT_CAP {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_workstation_snapshot_service.event_cap",
            });
        }
        let mut policy = Self::new();
        policy.event_cap = event_cap;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), RuntimeControlPlaneAdapterError> {
        validate_runtime_workstation_snapshot_service_policy(self)
    }
}

impl Default for RuntimeWorkstationSnapshotServicePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeWorkstationSnapshotServiceSupervisor {
    pub fn new(
        policy: &RuntimeWorkstationSnapshotServicePolicy,
    ) -> Result<Self, RuntimeControlPlaneAdapterError> {
        policy.validate()?;
        Ok(Self {
            state: RuntimeWorkstationSnapshotServiceState::Stopped,
            event_cap: policy.event_cap,
            latest_snapshot: None,
            accepted_snapshot_count: 0,
            events: Vec::new(),
        })
    }

    pub fn state(&self) -> RuntimeWorkstationSnapshotServiceState {
        self.state
    }

    pub fn events(&self) -> &[RuntimeWorkstationSnapshotServiceEvent] {
        &self.events
    }

    pub fn latest_snapshot(&self) -> Option<&RuntimeWorkstationSnapshot> {
        self.latest_snapshot.as_ref()
    }

    pub fn accepted_snapshot_count(&self) -> u32 {
        self.accepted_snapshot_count
    }

    pub fn start(
        &mut self,
        snapshot: RuntimeWorkstationSnapshot,
    ) -> Result<(), RuntimeControlPlaneAdapterError> {
        self.record_event(RuntimeWorkstationSnapshotServiceEventKind::StartRequested)?;
        if let Err(error) = validate_runtime_workstation_snapshot(&snapshot) {
            self.record_event(RuntimeWorkstationSnapshotServiceEventKind::Failed)?;
            return Err(error);
        }
        self.latest_snapshot = Some(snapshot);
        self.accepted_snapshot_count += 1;
        self.record_event(RuntimeWorkstationSnapshotServiceEventKind::SnapshotAccepted)
    }

    pub fn refresh_snapshot(
        &mut self,
        snapshot: RuntimeWorkstationSnapshot,
    ) -> Result<(), RuntimeControlPlaneAdapterError> {
        self.record_event(RuntimeWorkstationSnapshotServiceEventKind::RefreshRequested)?;
        if let Err(error) = validate_runtime_workstation_snapshot(&snapshot) {
            self.record_event(RuntimeWorkstationSnapshotServiceEventKind::Failed)?;
            return Err(error);
        }
        self.latest_snapshot = Some(snapshot);
        self.accepted_snapshot_count += 1;
        self.record_event(RuntimeWorkstationSnapshotServiceEventKind::SnapshotRefreshed)
    }

    pub fn stop(&mut self) -> Result<(), RuntimeControlPlaneAdapterError> {
        self.record_event(RuntimeWorkstationSnapshotServiceEventKind::StopRequested)?;
        self.record_event(RuntimeWorkstationSnapshotServiceEventKind::Stopped)
    }

    pub fn status(&self) -> RuntimeWorkstationSnapshotServiceStatus {
        RuntimeWorkstationSnapshotServiceStatus {
            schema_version: RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_SCHEMA_VERSION.to_owned(),
            accepted_snapshot_schema: RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION.to_owned(),
            final_state: self.state,
            latest_snapshot: self.latest_snapshot.clone(),
            accepted_snapshot_count: self.accepted_snapshot_count,
            event_cap: self.event_cap,
            events: self.events.clone(),
            local_only: true,
            in_memory_only: true,
            service_state_enabled: true,
            explicit_start_stop_enabled: true,
            snapshot_refresh_enabled: true,
            audit_events_enabled: true,
            capped_in_memory_events_enabled: true,
            validates_snapshot_before_accept: true,
            caller_provided_snapshots_only: true,
            file_io_enabled: false,
            storage_provider_enabled: false,
            database_or_indexing_enabled: false,
            generated_report_loading_enabled: false,
            generated_json_loading_enabled: false,
            raw_evidence_payload_loading_enabled: false,
            live_transport_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            listener_loop_enabled: false,
            daemon_lifecycle_enabled: false,
            async_stop_api_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: static_str_vec(RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_NON_CLAIMS),
        }
    }

    fn record_event(
        &mut self,
        event_kind: RuntimeWorkstationSnapshotServiceEventKind,
    ) -> Result<(), RuntimeControlPlaneAdapterError> {
        let next_state = match (self.state, event_kind) {
            (
                RuntimeWorkstationSnapshotServiceState::Stopped,
                RuntimeWorkstationSnapshotServiceEventKind::StartRequested,
            ) => RuntimeWorkstationSnapshotServiceState::Starting,
            (
                RuntimeWorkstationSnapshotServiceState::Starting,
                RuntimeWorkstationSnapshotServiceEventKind::SnapshotAccepted,
            ) => RuntimeWorkstationSnapshotServiceState::Running,
            (
                RuntimeWorkstationSnapshotServiceState::Running,
                RuntimeWorkstationSnapshotServiceEventKind::RefreshRequested,
            ) => RuntimeWorkstationSnapshotServiceState::RefreshingSnapshot,
            (
                RuntimeWorkstationSnapshotServiceState::RefreshingSnapshot,
                RuntimeWorkstationSnapshotServiceEventKind::SnapshotRefreshed,
            ) => RuntimeWorkstationSnapshotServiceState::Running,
            (
                RuntimeWorkstationSnapshotServiceState::Running,
                RuntimeWorkstationSnapshotServiceEventKind::StopRequested,
            ) => RuntimeWorkstationSnapshotServiceState::Stopping,
            (
                RuntimeWorkstationSnapshotServiceState::Stopping,
                RuntimeWorkstationSnapshotServiceEventKind::Stopped,
            ) => RuntimeWorkstationSnapshotServiceState::Stopped,
            (
                RuntimeWorkstationSnapshotServiceState::Starting
                | RuntimeWorkstationSnapshotServiceState::RefreshingSnapshot
                | RuntimeWorkstationSnapshotServiceState::Stopping,
                RuntimeWorkstationSnapshotServiceEventKind::Failed,
            ) => RuntimeWorkstationSnapshotServiceState::Failed,
            _ => {
                return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                    field: "runtime_workstation_snapshot_service.transition",
                });
            }
        };
        self.state = next_state;
        push_runtime_workstation_snapshot_service_event(
            &mut self.events,
            self.event_cap,
            self.state,
            event_kind,
        );
        Ok(())
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

impl RuntimeControlPlaneEndpointPathContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION,
            endpoint_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
            max_path_bytes: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES,
            local_only: true,
            caller_authorized_allowed_root_required: true,
            absolute_allowed_root_required: true,
            absolute_endpoint_path_required: true,
            allowed_root_must_exist: true,
            allowed_root_symlink_rejected: true,
            target_parent_must_exist: true,
            target_parent_symlink_rejected: true,
            target_must_not_exist: true,
            socket_extension_required: true,
            endpoint_filename_safety_enabled: true,
            path_selection_only: true,
            filesystem_socket_path_policy_enabled: true,
            filesystem_metadata_validation_enabled: true,
            filesystem_mutation_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            storage_provider_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_NON_CLAIMS,
        }
    }
}

impl RuntimeControlPlaneEndpointListenerContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION,
            endpoint_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
            endpoint_path_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION,
            ipc_schema_version: RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION,
            frame_schema_version: RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION,
            message_schema_version: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            max_path_bytes: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES,
            max_frame_bytes: RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES,
            local_only: true,
            one_shot_listener: true,
            filesystem_socket_binding_enabled: true,
            cleanup_on_completion: true,
            endpoint_path_validation_enabled: true,
            endpoint_stream_execution_enabled: true,
            public_network_transport_enabled: false,
            listener_loop_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            storage_provider_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_NON_CLAIMS,
        }
    }
}

impl RuntimeControlPlaneEndpointLifecycleContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_SCHEMA_VERSION,
            listener_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION,
            endpoint_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
            endpoint_path_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION,
            ipc_schema_version: RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION,
            frame_schema_version: RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION,
            message_schema_version: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            max_path_bytes: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES,
            max_frame_bytes: RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES,
            local_only: true,
            one_shot_lifecycle: true,
            start_stop_state_enabled: true,
            audit_events_enabled: true,
            endpoint_listener_execution_enabled: true,
            cleanup_on_completion: true,
            public_network_transport_enabled: false,
            listener_loop_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            storage_provider_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            persistent_event_store_enabled: false,
            non_claims: RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_NON_CLAIMS,
        }
    }
}

impl RuntimeControlPlaneServiceLifecycleContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_SCHEMA_VERSION,
            endpoint_lifecycle_schema_version:
                RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_SCHEMA_VERSION,
            listener_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION,
            endpoint_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
            endpoint_path_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION,
            ipc_schema_version: RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION,
            frame_schema_version: RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION,
            message_schema_version: RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
            default_event_cap: RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_DEFAULT_EVENT_CAP,
            max_path_bytes: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES,
            max_frame_bytes: RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES,
            local_only: true,
            service_state_enabled: true,
            explicit_start_stop_state_enabled: true,
            one_shot_endpoint_execution_enabled: true,
            audit_events_enabled: true,
            capped_in_memory_events_enabled: true,
            nested_endpoint_lifecycle_execution_enabled: true,
            cleanup_on_completion: true,
            public_network_transport_enabled: false,
            listener_loop_enabled: false,
            daemon_lifecycle_enabled: false,
            async_stop_api_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            storage_provider_enabled: false,
            persistent_event_store_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            non_claims: RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_NON_CLAIMS,
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

impl RuntimeControlPlaneEndpointPathPolicy {
    pub fn new(allowed_root: impl Into<PathBuf>) -> Self {
        Self {
            allowed_root: allowed_root.into(),
            max_path_bytes: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES,
            local_only: true,
            caller_authorized_allowed_root_required: true,
            path_selection_only: true,
            filesystem_socket_path_policy_enabled: true,
            filesystem_mutation_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
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

    pub fn bounded(
        allowed_root: impl Into<PathBuf>,
        max_path_bytes: usize,
    ) -> Result<Self, RuntimeControlPlaneAdapterError> {
        if max_path_bytes == 0 || max_path_bytes > RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.max_path_bytes",
            });
        }
        let mut policy = Self::new(allowed_root);
        policy.max_path_bytes = max_path_bytes;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), RuntimeControlPlaneAdapterError> {
        if self.max_path_bytes == 0
            || self.max_path_bytes > RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES
        {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.max_path_bytes",
            });
        }
        validate_required_flag("endpoint_path.local_only", self.local_only, true)?;
        validate_required_flag(
            "endpoint_path.caller_authorized_allowed_root_required",
            self.caller_authorized_allowed_root_required,
            true,
        )?;
        validate_required_flag(
            "endpoint_path.path_selection_only",
            self.path_selection_only,
            true,
        )?;
        validate_required_flag(
            "endpoint_path.filesystem_socket_path_policy_enabled",
            self.filesystem_socket_path_policy_enabled,
            true,
        )?;
        validate_required_flag(
            "endpoint_path.filesystem_mutation_enabled",
            self.filesystem_mutation_enabled,
            false,
        )?;
        validate_required_flag(
            "endpoint_path.public_network_transport_enabled",
            self.public_network_transport_enabled,
            false,
        )?;
        validate_required_flag(
            "endpoint_path.socket_listener_enabled",
            self.socket_listener_enabled,
            false,
        )?;
        validate_required_flag(
            "endpoint_path.daemon_lifecycle_enabled",
            self.daemon_lifecycle_enabled,
            false,
        )?;
        validate_required_flag(
            "endpoint_path.process_spawning_enabled",
            self.process_spawning_enabled,
            false,
        )?;
        validate_required_flag(
            "endpoint_path.file_watching_enabled",
            self.file_watching_enabled,
            false,
        )?;
        validate_required_flag(
            "endpoint_path.qt_binding_enabled",
            self.qt_binding_enabled,
            false,
        )?;
        validate_required_flag(
            "endpoint_path.storage_provider_enabled",
            self.storage_provider_enabled,
            false,
        )?;
        validate_required_flag("endpoint_path.capture_enabled", self.capture_enabled, false)?;
        validate_required_flag(
            "endpoint_path.external_services_used",
            self.external_services_used,
            false,
        )?;
        validate_required_flag(
            "endpoint_path.deployment_allowed",
            self.deployment_allowed,
            false,
        )?;
        validate_required_flag(
            "endpoint_path.native_inference_execution_enabled",
            self.native_inference_execution_enabled,
            false,
        )
    }

    pub fn max_bytes(&self) -> usize {
        self.max_path_bytes
    }
}

impl RuntimeControlPlaneEndpointListenerPolicy {
    pub fn new(endpoint_path_policy: RuntimeControlPlaneEndpointPathPolicy) -> Self {
        Self::with_endpoint_policy(
            endpoint_path_policy,
            RuntimeControlPlaneEndpointPolicy::default(),
        )
    }

    pub fn with_endpoint_policy(
        endpoint_path_policy: RuntimeControlPlaneEndpointPathPolicy,
        endpoint_policy: RuntimeControlPlaneEndpointPolicy,
    ) -> Self {
        Self {
            endpoint_path_policy,
            endpoint_policy,
            local_only: true,
            one_shot_listener: true,
            filesystem_socket_binding_enabled: true,
            cleanup_on_completion: true,
            endpoint_path_validation_enabled: true,
            endpoint_stream_execution_enabled: true,
            public_network_transport_enabled: false,
            listener_loop_enabled: false,
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
        validate_control_plane_endpoint_listener_policy(self)
    }

    pub fn max_path_bytes(&self) -> usize {
        self.endpoint_path_policy.max_bytes()
    }

    pub fn max_frame_bytes(&self) -> usize {
        self.endpoint_policy.max_frame_bytes()
    }
}

impl RuntimeControlPlaneEndpointLifecyclePolicy {
    pub fn new(listener_policy: RuntimeControlPlaneEndpointListenerPolicy) -> Self {
        Self {
            listener_policy,
            local_only: true,
            one_shot_lifecycle: true,
            start_stop_state_enabled: true,
            audit_events_enabled: true,
            endpoint_listener_execution_enabled: true,
            cleanup_on_completion: true,
            public_network_transport_enabled: false,
            listener_loop_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            storage_provider_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
            persistent_event_store_enabled: false,
        }
    }

    pub fn validate(&self) -> Result<(), RuntimeControlPlaneAdapterError> {
        validate_control_plane_endpoint_lifecycle_policy(self)
    }

    pub fn max_path_bytes(&self) -> usize {
        self.listener_policy.max_path_bytes()
    }

    pub fn max_frame_bytes(&self) -> usize {
        self.listener_policy.max_frame_bytes()
    }
}

impl RuntimeControlPlaneServiceLifecyclePolicy {
    pub fn new(endpoint_lifecycle_policy: RuntimeControlPlaneEndpointLifecyclePolicy) -> Self {
        Self {
            endpoint_lifecycle_policy,
            event_cap: RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_DEFAULT_EVENT_CAP,
            local_only: true,
            service_state_enabled: true,
            explicit_start_stop_state_enabled: true,
            one_shot_endpoint_execution_enabled: true,
            audit_events_enabled: true,
            capped_in_memory_events_enabled: true,
            nested_endpoint_lifecycle_execution_enabled: true,
            cleanup_on_completion: true,
            public_network_transport_enabled: false,
            listener_loop_enabled: false,
            daemon_lifecycle_enabled: false,
            async_stop_api_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            storage_provider_enabled: false,
            persistent_event_store_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
        }
    }

    pub fn bounded(
        endpoint_lifecycle_policy: RuntimeControlPlaneEndpointLifecyclePolicy,
        event_cap: usize,
    ) -> Result<Self, RuntimeControlPlaneAdapterError> {
        if event_cap == 0 || event_cap > RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_DEFAULT_EVENT_CAP {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "service_lifecycle.event_cap",
            });
        }
        let mut policy = Self::new(endpoint_lifecycle_policy);
        policy.event_cap = event_cap;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), RuntimeControlPlaneAdapterError> {
        validate_control_plane_service_lifecycle_policy(self)
    }

    pub fn max_path_bytes(&self) -> usize {
        self.endpoint_lifecycle_policy.max_path_bytes()
    }

    pub fn max_frame_bytes(&self) -> usize {
        self.endpoint_lifecycle_policy.max_frame_bytes()
    }
}

impl RuntimeControlPlaneServiceLifecycleSupervisor {
    pub fn new(
        policy: &RuntimeControlPlaneServiceLifecyclePolicy,
    ) -> Result<Self, RuntimeControlPlaneAdapterError> {
        policy.validate()?;
        Ok(Self {
            state: RuntimeControlPlaneServiceLifecycleState::Stopped,
            event_cap: policy.event_cap,
            events: Vec::new(),
        })
    }

    pub fn state(&self) -> RuntimeControlPlaneServiceLifecycleState {
        self.state
    }

    pub fn events(&self) -> &[RuntimeControlPlaneServiceLifecycleEvent] {
        &self.events
    }

    pub fn record_event(
        &mut self,
        event_kind: RuntimeControlPlaneServiceLifecycleEventKind,
    ) -> Result<(), RuntimeControlPlaneAdapterError> {
        let next_state = match (self.state, event_kind) {
            (
                RuntimeControlPlaneServiceLifecycleState::Stopped,
                RuntimeControlPlaneServiceLifecycleEventKind::StartRequested,
            ) => RuntimeControlPlaneServiceLifecycleState::Starting,
            (
                RuntimeControlPlaneServiceLifecycleState::Starting,
                RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleStarted,
            ) => RuntimeControlPlaneServiceLifecycleState::RunningEndpointOnce,
            (
                RuntimeControlPlaneServiceLifecycleState::RunningEndpointOnce,
                RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleCompleted,
            ) => RuntimeControlPlaneServiceLifecycleState::Stopping,
            (
                RuntimeControlPlaneServiceLifecycleState::Stopping,
                RuntimeControlPlaneServiceLifecycleEventKind::StopRequested,
            ) => RuntimeControlPlaneServiceLifecycleState::Stopping,
            (
                RuntimeControlPlaneServiceLifecycleState::Stopping,
                RuntimeControlPlaneServiceLifecycleEventKind::Stopped,
            ) => RuntimeControlPlaneServiceLifecycleState::Stopped,
            (
                RuntimeControlPlaneServiceLifecycleState::Starting
                | RuntimeControlPlaneServiceLifecycleState::RunningEndpointOnce
                | RuntimeControlPlaneServiceLifecycleState::Stopping,
                RuntimeControlPlaneServiceLifecycleEventKind::Failed,
            ) => RuntimeControlPlaneServiceLifecycleState::Failed,
            _ => {
                return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                    field: "service_lifecycle.transition",
                });
            }
        };
        self.state = next_state;
        push_control_plane_service_lifecycle_event(
            &mut self.events,
            self.event_cap,
            self.state,
            event_kind,
        );
        Ok(())
    }

    fn into_events(self) -> Vec<RuntimeControlPlaneServiceLifecycleEvent> {
        self.events
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

#[cfg(unix)]
pub fn execute_control_plane_endpoint_listener_once(
    path: impl AsRef<Path>,
    policy: &RuntimeControlPlaneEndpointListenerPolicy,
) -> Result<RuntimeControlPlaneEndpointListenerOutcome, RuntimeControlPlaneAdapterError> {
    match execute_control_plane_endpoint_listener_once_audited(path.as_ref(), policy)? {
        RuntimeControlPlaneEndpointListenerExecution::Succeeded(outcome) => Ok(*outcome),
        RuntimeControlPlaneEndpointListenerExecution::Failed(failure) => Err(failure.error),
    }
}

#[cfg(unix)]
fn execute_control_plane_endpoint_listener_once_audited(
    path: &Path,
    policy: &RuntimeControlPlaneEndpointListenerPolicy,
) -> Result<RuntimeControlPlaneEndpointListenerExecution, RuntimeControlPlaneAdapterError> {
    policy.validate()?;
    let selection = validate_control_plane_endpoint_path(path, &policy.endpoint_path_policy)?;
    let endpoint_path = PathBuf::from(&selection.endpoint_path);
    validate_control_plane_endpoint_listener_path_permissions(&selection)?;
    let listener = UnixListener::bind(&endpoint_path)
        .map_err(|_| RuntimeControlPlaneAdapterError::EndpointBindFailed)?;
    if let Err(error) = restrict_control_plane_endpoint_socket_permissions(&endpoint_path) {
        drop(listener);
        let cleanup_result = cleanup_control_plane_endpoint_socket_path(&endpoint_path);
        let failure = match cleanup_result {
            Ok(socket_path_removed) => RuntimeControlPlaneEndpointListenerFailure {
                error,
                cleanup_attempted: true,
                socket_path_removed,
            },
            Err(cleanup_error) => RuntimeControlPlaneEndpointListenerFailure {
                error: cleanup_error,
                cleanup_attempted: true,
                socket_path_removed: false,
            },
        };
        return Ok(RuntimeControlPlaneEndpointListenerExecution::Failed(
            failure,
        ));
    }
    let execution_result = match listener.accept() {
        Ok((mut stream, _address)) => match stream.try_clone() {
            Ok(mut writer) => execute_control_plane_endpoint_stream(
                &mut stream,
                &mut writer,
                &policy.endpoint_policy,
            ),
            Err(_) => Err(RuntimeControlPlaneAdapterError::EndpointAcceptFailed),
        },
        Err(_) => Err(RuntimeControlPlaneAdapterError::EndpointAcceptFailed),
    };
    drop(listener);

    let cleanup_result = cleanup_control_plane_endpoint_socket_path(&endpoint_path);
    match (execution_result, cleanup_result) {
        (Ok(()), Ok(socket_path_removed)) => {
            Ok(RuntimeControlPlaneEndpointListenerExecution::Succeeded(
                Box::new(RuntimeControlPlaneEndpointListenerOutcome {
                    schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION
                        .to_owned(),
                    endpoint_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION
                        .to_owned(),
                    endpoint_path_schema_version:
                        RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION.to_owned(),
                    endpoint_path_selection: selection,
                    local_only: true,
                    one_shot_listener: true,
                    filesystem_socket_binding_enabled: true,
                    endpoint_path_validation_enabled: true,
                    endpoint_stream_execution_enabled: true,
                    cleanup_attempted: true,
                    socket_path_removed,
                    public_network_transport_enabled: false,
                    listener_loop_enabled: false,
                    daemon_lifecycle_enabled: false,
                    process_spawning_enabled: false,
                    file_watching_enabled: false,
                    qt_binding_enabled: false,
                    storage_provider_enabled: false,
                    capture_enabled: false,
                    external_services_used: false,
                    deployment_allowed: false,
                    native_inference_execution_enabled: false,
                    non_claims: static_str_vec(RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_NON_CLAIMS),
                }),
            ))
        }
        (Ok(()), Err(cleanup_error)) => Ok(RuntimeControlPlaneEndpointListenerExecution::Failed(
            RuntimeControlPlaneEndpointListenerFailure {
                error: cleanup_error,
                cleanup_attempted: true,
                socket_path_removed: false,
            },
        )),
        (Err(error), Ok(socket_path_removed)) => {
            Ok(RuntimeControlPlaneEndpointListenerExecution::Failed(
                RuntimeControlPlaneEndpointListenerFailure {
                    error,
                    cleanup_attempted: true,
                    socket_path_removed,
                },
            ))
        }
        (Err(_error), Err(cleanup_error)) => {
            Ok(RuntimeControlPlaneEndpointListenerExecution::Failed(
                RuntimeControlPlaneEndpointListenerFailure {
                    error: cleanup_error,
                    cleanup_attempted: true,
                    socket_path_removed: false,
                },
            ))
        }
    }
}

#[cfg(not(unix))]
pub fn execute_control_plane_endpoint_listener_once(
    path: impl AsRef<Path>,
    policy: &RuntimeControlPlaneEndpointListenerPolicy,
) -> Result<RuntimeControlPlaneEndpointListenerOutcome, RuntimeControlPlaneAdapterError> {
    let _ = path.as_ref();
    let _ = policy;
    Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
        field: "endpoint_listener.platform",
    })
}

#[cfg(unix)]
pub fn execute_control_plane_endpoint_lifecycle_once(
    path: impl AsRef<Path>,
    policy: &RuntimeControlPlaneEndpointLifecyclePolicy,
) -> Result<RuntimeControlPlaneEndpointLifecycleOutcome, RuntimeControlPlaneAdapterError> {
    policy.validate()?;
    let path = path.as_ref();
    let mut events = Vec::new();
    push_control_plane_endpoint_lifecycle_event(
        &mut events,
        RuntimeControlPlaneEndpointLifecycleState::StartRequested,
        RuntimeControlPlaneEndpointLifecycleEventKind::StartRequested,
    );

    match execute_control_plane_endpoint_listener_once_audited(path, &policy.listener_policy) {
        Ok(RuntimeControlPlaneEndpointListenerExecution::Succeeded(listener_outcome)) => {
            let listener_outcome = *listener_outcome;
            push_control_plane_endpoint_lifecycle_event(
                &mut events,
                RuntimeControlPlaneEndpointLifecycleState::Listening,
                RuntimeControlPlaneEndpointLifecycleEventKind::PathValidated,
            );
            push_control_plane_endpoint_lifecycle_event(
                &mut events,
                RuntimeControlPlaneEndpointLifecycleState::Listening,
                RuntimeControlPlaneEndpointLifecycleEventKind::SocketBound,
            );
            push_control_plane_endpoint_lifecycle_event(
                &mut events,
                RuntimeControlPlaneEndpointLifecycleState::Listening,
                RuntimeControlPlaneEndpointLifecycleEventKind::ClientAccepted,
            );
            push_control_plane_endpoint_lifecycle_event(
                &mut events,
                RuntimeControlPlaneEndpointLifecycleState::Stopping,
                RuntimeControlPlaneEndpointLifecycleEventKind::RequestCompleted,
            );
            push_control_plane_endpoint_lifecycle_event(
                &mut events,
                RuntimeControlPlaneEndpointLifecycleState::Stopping,
                RuntimeControlPlaneEndpointLifecycleEventKind::StopRequested,
            );
            push_control_plane_endpoint_lifecycle_event(
                &mut events,
                RuntimeControlPlaneEndpointLifecycleState::Stopped,
                RuntimeControlPlaneEndpointLifecycleEventKind::CleanupCompleted,
            );
            let cleanup_attempted = listener_outcome.cleanup_attempted;
            let socket_path_removed = listener_outcome.socket_path_removed;
            Ok(control_plane_endpoint_lifecycle_outcome(
                policy,
                Some(listener_outcome),
                RuntimeControlPlaneEndpointLifecycleState::Stopped,
                None,
                events,
                cleanup_attempted,
                socket_path_removed,
            ))
        }
        Ok(RuntimeControlPlaneEndpointListenerExecution::Failed(failure)) => {
            if failure.cleanup_attempted {
                push_control_plane_endpoint_lifecycle_event(
                    &mut events,
                    RuntimeControlPlaneEndpointLifecycleState::Listening,
                    RuntimeControlPlaneEndpointLifecycleEventKind::PathValidated,
                );
            }
            if failure.socket_path_removed {
                push_control_plane_endpoint_lifecycle_event(
                    &mut events,
                    RuntimeControlPlaneEndpointLifecycleState::Stopping,
                    RuntimeControlPlaneEndpointLifecycleEventKind::CleanupCompleted,
                );
            }
            push_control_plane_endpoint_lifecycle_event(
                &mut events,
                RuntimeControlPlaneEndpointLifecycleState::Failed,
                RuntimeControlPlaneEndpointLifecycleEventKind::Failed,
            );
            Ok(control_plane_endpoint_lifecycle_outcome(
                policy,
                None,
                RuntimeControlPlaneEndpointLifecycleState::Failed,
                Some((&failure.error).into()),
                events,
                failure.cleanup_attempted,
                failure.socket_path_removed,
            ))
        }
        Err(error) => {
            push_control_plane_endpoint_lifecycle_event(
                &mut events,
                RuntimeControlPlaneEndpointLifecycleState::Failed,
                RuntimeControlPlaneEndpointLifecycleEventKind::Failed,
            );
            Ok(control_plane_endpoint_lifecycle_outcome(
                policy,
                None,
                RuntimeControlPlaneEndpointLifecycleState::Failed,
                Some((&error).into()),
                events,
                false,
                false,
            ))
        }
    }
}

#[cfg(not(unix))]
pub fn execute_control_plane_endpoint_lifecycle_once(
    path: impl AsRef<Path>,
    policy: &RuntimeControlPlaneEndpointLifecyclePolicy,
) -> Result<RuntimeControlPlaneEndpointLifecycleOutcome, RuntimeControlPlaneAdapterError> {
    let _ = path.as_ref();
    policy.validate()?;
    let mut events = Vec::new();
    push_control_plane_endpoint_lifecycle_event(
        &mut events,
        RuntimeControlPlaneEndpointLifecycleState::StartRequested,
        RuntimeControlPlaneEndpointLifecycleEventKind::StartRequested,
    );
    push_control_plane_endpoint_lifecycle_event(
        &mut events,
        RuntimeControlPlaneEndpointLifecycleState::Failed,
        RuntimeControlPlaneEndpointLifecycleEventKind::Failed,
    );
    Ok(control_plane_endpoint_lifecycle_outcome(
        policy,
        None,
        RuntimeControlPlaneEndpointLifecycleState::Failed,
        Some(RuntimeControlPlaneMessageErrorCode::UnsupportedValue),
        events,
        false,
        false,
    ))
}

pub fn execute_control_plane_service_lifecycle_once(
    path: impl AsRef<Path>,
    policy: &RuntimeControlPlaneServiceLifecyclePolicy,
) -> Result<RuntimeControlPlaneServiceLifecycleOutcome, RuntimeControlPlaneAdapterError> {
    let path = path.as_ref();
    let mut supervisor = RuntimeControlPlaneServiceLifecycleSupervisor::new(policy)?;
    supervisor.record_event(RuntimeControlPlaneServiceLifecycleEventKind::StartRequested)?;
    supervisor
        .record_event(RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleStarted)?;

    match execute_control_plane_endpoint_lifecycle_once(path, &policy.endpoint_lifecycle_policy) {
        Ok(endpoint_lifecycle_outcome) => {
            let cleanup_attempted = endpoint_lifecycle_outcome.cleanup_attempted;
            let socket_path_removed = endpoint_lifecycle_outcome.socket_path_removed;
            let failure_error_code = endpoint_lifecycle_outcome.failure_error_code;

            if endpoint_lifecycle_outcome.final_state
                == RuntimeControlPlaneEndpointLifecycleState::Stopped
            {
                supervisor.record_event(
                    RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleCompleted,
                )?;
                supervisor
                    .record_event(RuntimeControlPlaneServiceLifecycleEventKind::StopRequested)?;
                supervisor.record_event(RuntimeControlPlaneServiceLifecycleEventKind::Stopped)?;
                Ok(control_plane_service_lifecycle_outcome(
                    policy,
                    Some(endpoint_lifecycle_outcome),
                    supervisor.state(),
                    None,
                    supervisor.into_events(),
                    cleanup_attempted,
                    socket_path_removed,
                ))
            } else {
                supervisor.record_event(RuntimeControlPlaneServiceLifecycleEventKind::Failed)?;
                Ok(control_plane_service_lifecycle_outcome(
                    policy,
                    Some(endpoint_lifecycle_outcome),
                    supervisor.state(),
                    failure_error_code
                        .or(Some(RuntimeControlPlaneMessageErrorCode::UnsupportedValue)),
                    supervisor.into_events(),
                    cleanup_attempted,
                    socket_path_removed,
                ))
            }
        }
        Err(error) => {
            supervisor.record_event(RuntimeControlPlaneServiceLifecycleEventKind::Failed)?;
            Ok(control_plane_service_lifecycle_outcome(
                policy,
                None,
                supervisor.state(),
                Some((&error).into()),
                supervisor.into_events(),
                false,
                false,
            ))
        }
    }
}

pub fn validate_control_plane_endpoint_path(
    path: impl AsRef<Path>,
    policy: &RuntimeControlPlaneEndpointPathPolicy,
) -> Result<RuntimeControlPlaneEndpointPathSelection, RuntimeControlPlaneAdapterError> {
    policy.validate()?;
    let path = path.as_ref();
    if !path.is_absolute() {
        return Err(RuntimeControlPlaneAdapterError::RelativeFilePath);
    }
    if !policy.allowed_root.is_absolute() {
        return Err(RuntimeControlPlaneAdapterError::RelativeAllowedRoot);
    }
    let path_text = path
        .to_str()
        .ok_or(RuntimeControlPlaneAdapterError::InvalidUtf8)?;
    let allowed_root_text = policy
        .allowed_root
        .to_str()
        .ok_or(RuntimeControlPlaneAdapterError::InvalidUtf8)?;
    if path_text.len() > policy.max_path_bytes {
        return Err(RuntimeControlPlaneAdapterError::OversizedPath {
            max_bytes: policy.max_path_bytes,
        });
    }
    if path.extension().and_then(|extension| extension.to_str()) != Some("sock") {
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
    let canonical_allowed_root = fs::canonicalize(&policy.allowed_root)
        .map_err(|_| RuntimeControlPlaneAdapterError::MissingAllowedRoot)?;

    let parent = path
        .parent()
        .ok_or(RuntimeControlPlaneAdapterError::MissingFile)?;
    let parent_metadata =
        fs::symlink_metadata(parent).map_err(|_| RuntimeControlPlaneAdapterError::MissingFile)?;
    if parent_metadata.file_type().is_symlink() {
        return Err(RuntimeControlPlaneAdapterError::SymlinkPath);
    }
    if !parent_metadata.is_dir() {
        return Err(RuntimeControlPlaneAdapterError::MissingFile);
    }
    let canonical_parent =
        fs::canonicalize(parent).map_err(|_| RuntimeControlPlaneAdapterError::MissingFile)?;
    if !canonical_parent.starts_with(&canonical_allowed_root) {
        return Err(RuntimeControlPlaneAdapterError::OutsideAllowedRoot);
    }

    if let Ok(target_metadata) = fs::symlink_metadata(path) {
        if target_metadata.file_type().is_symlink() {
            return Err(RuntimeControlPlaneAdapterError::SymlinkPath);
        }
        if target_metadata.is_dir() {
            return Err(RuntimeControlPlaneAdapterError::DirectoryPath);
        }
        if target_metadata.file_type().is_file() {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.target_exists",
            });
        }
        return Err(RuntimeControlPlaneAdapterError::NonRegularFile);
    }

    let endpoint_filename = path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .ok_or(RuntimeControlPlaneAdapterError::InvalidUtf8)?;
    validate_safe_endpoint_filename(endpoint_filename)?;

    let selected_path = canonical_parent.join(endpoint_filename);
    let selected_path_text = selected_path
        .to_str()
        .ok_or(RuntimeControlPlaneAdapterError::InvalidUtf8)?;
    if selected_path_text.len() > policy.max_path_bytes {
        return Err(RuntimeControlPlaneAdapterError::OversizedPath {
            max_bytes: policy.max_path_bytes,
        });
    }
    let selected_allowed_root_text = canonical_allowed_root
        .to_str()
        .unwrap_or(allowed_root_text)
        .to_owned();

    Ok(RuntimeControlPlaneEndpointPathSelection {
        schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION.to_owned(),
        endpoint_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION.to_owned(),
        endpoint_path: selected_path_text.to_owned(),
        allowed_root: selected_allowed_root_text,
        endpoint_filename: endpoint_filename.to_owned(),
        max_path_bytes: policy.max_path_bytes,
        local_only: true,
        caller_authorized_allowed_root_required: true,
        absolute_endpoint_path: true,
        under_allowed_root: true,
        target_parent_exists: true,
        target_did_not_exist: true,
        socket_extension: "sock".to_owned(),
        path_selection_only: true,
        filesystem_socket_path_policy_enabled: true,
        filesystem_mutation_enabled: false,
        public_network_transport_enabled: false,
        socket_listener_enabled: false,
        daemon_lifecycle_enabled: false,
        process_spawning_enabled: false,
        file_watching_enabled: false,
        qt_binding_enabled: false,
        storage_provider_enabled: false,
        capture_enabled: false,
        external_services_used: false,
        deployment_allowed: false,
        native_inference_execution_enabled: false,
        non_claims: static_str_vec(RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_NON_CLAIMS),
    })
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

pub fn parse_evidence_index_json(
    input: &str,
) -> Result<EvidenceIndex, RuntimeControlPlaneAdapterError> {
    match input.trim_start().as_bytes().first() {
        Some(b'{') => {}
        Some(_) => return Err(RuntimeControlPlaneAdapterError::NonObjectRoot),
        None => return Err(RuntimeControlPlaneAdapterError::InvalidJson),
    }

    let index: EvidenceIndex =
        serde_json::from_str(input).map_err(|_| RuntimeControlPlaneAdapterError::InvalidJson)?;
    validate_schema_version(
        "schema_version",
        &index.schema_version,
        EVIDENCE_INDEX_SCHEMA_VERSION,
    )?;
    validate_evidence_index(&index)?;
    Ok(index)
}

pub fn parse_evidence_index_file(
    path: impl AsRef<Path>,
    policy: &RuntimeControlPlaneFilePolicy,
) -> Result<EvidenceIndex, RuntimeControlPlaneAdapterError> {
    let canonical_path = validate_runtime_control_plane_json_file_path(path.as_ref(), policy)?;
    let bytes =
        fs::read(&canonical_path).map_err(|_| RuntimeControlPlaneAdapterError::FileReadFailed)?;
    if bytes.len() as u64 > RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES {
        return Err(RuntimeControlPlaneAdapterError::OversizedFile {
            max_bytes: RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES,
        });
    }
    let input =
        String::from_utf8(bytes).map_err(|_| RuntimeControlPlaneAdapterError::InvalidUtf8)?;
    parse_evidence_index_json(&input)
}

pub fn build_runtime_workstation_snapshot(
    handoff_snapshot: RuntimeHandoffSnapshot,
    evidence_index: EvidenceIndex,
    policy: &RuntimeWorkstationSnapshotProviderPolicy,
) -> Result<RuntimeWorkstationSnapshot, RuntimeControlPlaneAdapterError> {
    policy.validate()?;
    validate_runtime_handoff_snapshot(&handoff_snapshot)?;
    validate_evidence_index(&evidence_index)?;

    let snapshot = RuntimeWorkstationSnapshot {
        schema_version: RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION.to_owned(),
        aggregate_summary: derive_runtime_workstation_snapshot_aggregate_summary(
            &handoff_snapshot,
            &evidence_index,
        ),
        runtime_handoff_snapshot: handoff_snapshot,
        evidence_index,
        safety_flags: RuntimeWorkstationSnapshotSafetyFlags {
            local_only: true,
            strict_json_loaded: true,
            caller_provided_snapshots_only: true,
            validated_runtime_handoff_snapshot: true,
            validated_evidence_index: true,
            pointer_only_evidence: true,
            generated_json_loaded: false,
            raw_evidence_payload_copied: false,
            live_runtime_connection: false,
            file_io_enabled: false,
            storage_provider_enabled: false,
            database_or_indexing_enabled: false,
            public_network_transport_enabled: false,
            socket_listener_enabled: false,
            daemon_lifecycle_enabled: false,
            process_spawning_enabled: false,
            file_watching_enabled: false,
            qt_binding_enabled: false,
            capture_enabled: false,
            external_services_used: false,
            deployment_allowed: false,
            native_inference_execution_enabled: false,
        },
        non_claims: static_str_vec(RUNTIME_WORKSTATION_SNAPSHOT_NON_CLAIMS),
    };
    validate_runtime_workstation_snapshot(&snapshot)?;
    Ok(snapshot)
}

pub fn parse_runtime_workstation_snapshot_json(
    input: &str,
) -> Result<RuntimeWorkstationSnapshot, RuntimeControlPlaneAdapterError> {
    match input.trim_start().as_bytes().first() {
        Some(b'{') => {}
        Some(_) => return Err(RuntimeControlPlaneAdapterError::NonObjectRoot),
        None => return Err(RuntimeControlPlaneAdapterError::InvalidJson),
    }

    let snapshot: RuntimeWorkstationSnapshot =
        serde_json::from_str(input).map_err(|_| RuntimeControlPlaneAdapterError::InvalidJson)?;
    validate_runtime_workstation_snapshot(&snapshot)?;
    Ok(snapshot)
}

pub fn execute_runtime_workstation_snapshot_service_once(
    initial_snapshot: RuntimeWorkstationSnapshot,
    refresh_snapshots: &[RuntimeWorkstationSnapshot],
    policy: &RuntimeWorkstationSnapshotServicePolicy,
) -> Result<RuntimeWorkstationSnapshotServiceStatus, RuntimeControlPlaneAdapterError> {
    let mut supervisor = RuntimeWorkstationSnapshotServiceSupervisor::new(policy)?;
    supervisor.start(initial_snapshot)?;
    for snapshot in refresh_snapshots {
        supervisor.refresh_snapshot(snapshot.clone())?;
    }
    supervisor.stop()?;
    let status = supervisor.status();
    validate_runtime_workstation_snapshot_service_status(&status)?;
    Ok(status)
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

fn validate_control_plane_endpoint_listener_policy(
    policy: &RuntimeControlPlaneEndpointListenerPolicy,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    policy.endpoint_path_policy.validate()?;
    policy.endpoint_policy.validate()?;
    validate_required_flag("endpoint_listener.local_only", policy.local_only, true)?;
    validate_required_flag(
        "endpoint_listener.one_shot_listener",
        policy.one_shot_listener,
        true,
    )?;
    validate_required_flag(
        "endpoint_listener.filesystem_socket_binding_enabled",
        policy.filesystem_socket_binding_enabled,
        true,
    )?;
    validate_required_flag(
        "endpoint_listener.cleanup_on_completion",
        policy.cleanup_on_completion,
        true,
    )?;
    validate_required_flag(
        "endpoint_listener.endpoint_path_validation_enabled",
        policy.endpoint_path_validation_enabled,
        true,
    )?;
    validate_required_flag(
        "endpoint_listener.endpoint_stream_execution_enabled",
        policy.endpoint_stream_execution_enabled,
        true,
    )?;
    validate_required_flag(
        "endpoint_listener.public_network_transport_enabled",
        policy.public_network_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_listener.listener_loop_enabled",
        policy.listener_loop_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_listener.daemon_lifecycle_enabled",
        policy.daemon_lifecycle_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_listener.process_spawning_enabled",
        policy.process_spawning_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_listener.file_watching_enabled",
        policy.file_watching_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_listener.qt_binding_enabled",
        policy.qt_binding_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_listener.storage_provider_enabled",
        policy.storage_provider_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_listener.capture_enabled",
        policy.capture_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_listener.external_services_used",
        policy.external_services_used,
        false,
    )?;
    validate_required_flag(
        "endpoint_listener.deployment_allowed",
        policy.deployment_allowed,
        false,
    )?;
    validate_required_flag(
        "endpoint_listener.native_inference_execution_enabled",
        policy.native_inference_execution_enabled,
        false,
    )
}

fn validate_control_plane_endpoint_lifecycle_policy(
    policy: &RuntimeControlPlaneEndpointLifecyclePolicy,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    policy.listener_policy.validate()?;
    validate_required_flag("endpoint_lifecycle.local_only", policy.local_only, true)?;
    validate_required_flag(
        "endpoint_lifecycle.one_shot_lifecycle",
        policy.one_shot_lifecycle,
        true,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.start_stop_state_enabled",
        policy.start_stop_state_enabled,
        true,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.audit_events_enabled",
        policy.audit_events_enabled,
        true,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.endpoint_listener_execution_enabled",
        policy.endpoint_listener_execution_enabled,
        true,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.cleanup_on_completion",
        policy.cleanup_on_completion,
        true,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.public_network_transport_enabled",
        policy.public_network_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.listener_loop_enabled",
        policy.listener_loop_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.daemon_lifecycle_enabled",
        policy.daemon_lifecycle_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.process_spawning_enabled",
        policy.process_spawning_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.file_watching_enabled",
        policy.file_watching_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.qt_binding_enabled",
        policy.qt_binding_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.storage_provider_enabled",
        policy.storage_provider_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.capture_enabled",
        policy.capture_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.external_services_used",
        policy.external_services_used,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.deployment_allowed",
        policy.deployment_allowed,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.native_inference_execution_enabled",
        policy.native_inference_execution_enabled,
        false,
    )?;
    validate_required_flag(
        "endpoint_lifecycle.persistent_event_store_enabled",
        policy.persistent_event_store_enabled,
        false,
    )
}

fn push_control_plane_endpoint_lifecycle_event(
    events: &mut Vec<RuntimeControlPlaneEndpointLifecycleEvent>,
    state: RuntimeControlPlaneEndpointLifecycleState,
    event_kind: RuntimeControlPlaneEndpointLifecycleEventKind,
) {
    events.push(RuntimeControlPlaneEndpointLifecycleEvent {
        schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_SCHEMA_VERSION.to_owned(),
        event_index: u32::try_from(events.len()).unwrap_or(u32::MAX),
        state,
        event_kind,
        event_label: event_kind.as_str(),
        local_only: true,
        external_services_used: false,
        deployment_allowed: false,
        native_inference_execution_enabled: false,
    });
}

fn control_plane_endpoint_lifecycle_outcome(
    policy: &RuntimeControlPlaneEndpointLifecyclePolicy,
    listener_outcome: Option<RuntimeControlPlaneEndpointListenerOutcome>,
    final_state: RuntimeControlPlaneEndpointLifecycleState,
    failure_error_code: Option<RuntimeControlPlaneMessageErrorCode>,
    events: Vec<RuntimeControlPlaneEndpointLifecycleEvent>,
    cleanup_attempted: bool,
    socket_path_removed: bool,
) -> RuntimeControlPlaneEndpointLifecycleOutcome {
    RuntimeControlPlaneEndpointLifecycleOutcome {
        schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_SCHEMA_VERSION.to_owned(),
        listener_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION.to_owned(),
        endpoint_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION.to_owned(),
        endpoint_path_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION.to_owned(),
        listener_outcome,
        final_state,
        failure_error_code,
        events,
        cleanup_attempted,
        socket_path_removed,
        local_only: policy.local_only,
        one_shot_lifecycle: policy.one_shot_lifecycle,
        start_stop_state_enabled: policy.start_stop_state_enabled,
        audit_events_enabled: policy.audit_events_enabled,
        endpoint_listener_execution_enabled: policy.endpoint_listener_execution_enabled,
        cleanup_on_completion: policy.cleanup_on_completion,
        public_network_transport_enabled: policy.public_network_transport_enabled,
        listener_loop_enabled: policy.listener_loop_enabled,
        daemon_lifecycle_enabled: policy.daemon_lifecycle_enabled,
        process_spawning_enabled: policy.process_spawning_enabled,
        file_watching_enabled: policy.file_watching_enabled,
        qt_binding_enabled: policy.qt_binding_enabled,
        storage_provider_enabled: policy.storage_provider_enabled,
        capture_enabled: policy.capture_enabled,
        external_services_used: policy.external_services_used,
        deployment_allowed: policy.deployment_allowed,
        native_inference_execution_enabled: policy.native_inference_execution_enabled,
        persistent_event_store_enabled: policy.persistent_event_store_enabled,
        non_claims: static_str_vec(RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_NON_CLAIMS),
    }
}

fn validate_control_plane_service_lifecycle_policy(
    policy: &RuntimeControlPlaneServiceLifecyclePolicy,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    policy.endpoint_lifecycle_policy.validate()?;
    if policy.event_cap == 0
        || policy.event_cap > RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_DEFAULT_EVENT_CAP
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "service_lifecycle.event_cap",
        });
    }
    validate_required_flag("service_lifecycle.local_only", policy.local_only, true)?;
    validate_required_flag(
        "service_lifecycle.service_state_enabled",
        policy.service_state_enabled,
        true,
    )?;
    validate_required_flag(
        "service_lifecycle.explicit_start_stop_state_enabled",
        policy.explicit_start_stop_state_enabled,
        true,
    )?;
    validate_required_flag(
        "service_lifecycle.one_shot_endpoint_execution_enabled",
        policy.one_shot_endpoint_execution_enabled,
        true,
    )?;
    validate_required_flag(
        "service_lifecycle.audit_events_enabled",
        policy.audit_events_enabled,
        true,
    )?;
    validate_required_flag(
        "service_lifecycle.capped_in_memory_events_enabled",
        policy.capped_in_memory_events_enabled,
        true,
    )?;
    validate_required_flag(
        "service_lifecycle.nested_endpoint_lifecycle_execution_enabled",
        policy.nested_endpoint_lifecycle_execution_enabled,
        true,
    )?;
    validate_required_flag(
        "service_lifecycle.cleanup_on_completion",
        policy.cleanup_on_completion,
        true,
    )?;
    validate_required_flag(
        "service_lifecycle.public_network_transport_enabled",
        policy.public_network_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.listener_loop_enabled",
        policy.listener_loop_enabled,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.daemon_lifecycle_enabled",
        policy.daemon_lifecycle_enabled,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.async_stop_api_enabled",
        policy.async_stop_api_enabled,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.process_spawning_enabled",
        policy.process_spawning_enabled,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.file_watching_enabled",
        policy.file_watching_enabled,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.qt_binding_enabled",
        policy.qt_binding_enabled,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.storage_provider_enabled",
        policy.storage_provider_enabled,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.persistent_event_store_enabled",
        policy.persistent_event_store_enabled,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.capture_enabled",
        policy.capture_enabled,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.external_services_used",
        policy.external_services_used,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.deployment_allowed",
        policy.deployment_allowed,
        false,
    )?;
    validate_required_flag(
        "service_lifecycle.native_inference_execution_enabled",
        policy.native_inference_execution_enabled,
        false,
    )
}

fn push_control_plane_service_lifecycle_event(
    events: &mut Vec<RuntimeControlPlaneServiceLifecycleEvent>,
    event_cap: usize,
    state: RuntimeControlPlaneServiceLifecycleState,
    event_kind: RuntimeControlPlaneServiceLifecycleEventKind,
) {
    if events.len() >= event_cap {
        return;
    }
    events.push(RuntimeControlPlaneServiceLifecycleEvent {
        schema_version: RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_SCHEMA_VERSION.to_owned(),
        event_index: u32::try_from(events.len()).unwrap_or(u32::MAX),
        state,
        event_kind,
        event_label: event_kind.as_str(),
        local_only: true,
        external_services_used: false,
        deployment_allowed: false,
        native_inference_execution_enabled: false,
    });
}

fn control_plane_service_lifecycle_outcome(
    policy: &RuntimeControlPlaneServiceLifecyclePolicy,
    endpoint_lifecycle_outcome: Option<RuntimeControlPlaneEndpointLifecycleOutcome>,
    final_state: RuntimeControlPlaneServiceLifecycleState,
    failure_error_code: Option<RuntimeControlPlaneMessageErrorCode>,
    events: Vec<RuntimeControlPlaneServiceLifecycleEvent>,
    cleanup_attempted: bool,
    socket_path_removed: bool,
) -> RuntimeControlPlaneServiceLifecycleOutcome {
    RuntimeControlPlaneServiceLifecycleOutcome {
        schema_version: RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_SCHEMA_VERSION.to_owned(),
        endpoint_lifecycle_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_SCHEMA_VERSION
            .to_owned(),
        listener_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION.to_owned(),
        endpoint_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION.to_owned(),
        endpoint_path_schema_version: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION.to_owned(),
        endpoint_lifecycle_outcome,
        final_state,
        failure_error_code,
        events,
        event_cap: policy.event_cap,
        cleanup_attempted,
        socket_path_removed,
        local_only: policy.local_only,
        service_state_enabled: policy.service_state_enabled,
        explicit_start_stop_state_enabled: policy.explicit_start_stop_state_enabled,
        one_shot_endpoint_execution_enabled: policy.one_shot_endpoint_execution_enabled,
        audit_events_enabled: policy.audit_events_enabled,
        capped_in_memory_events_enabled: policy.capped_in_memory_events_enabled,
        nested_endpoint_lifecycle_execution_enabled: policy
            .nested_endpoint_lifecycle_execution_enabled,
        cleanup_on_completion: policy.cleanup_on_completion,
        public_network_transport_enabled: policy.public_network_transport_enabled,
        listener_loop_enabled: policy.listener_loop_enabled,
        daemon_lifecycle_enabled: policy.daemon_lifecycle_enabled,
        async_stop_api_enabled: policy.async_stop_api_enabled,
        process_spawning_enabled: policy.process_spawning_enabled,
        file_watching_enabled: policy.file_watching_enabled,
        qt_binding_enabled: policy.qt_binding_enabled,
        storage_provider_enabled: policy.storage_provider_enabled,
        persistent_event_store_enabled: policy.persistent_event_store_enabled,
        capture_enabled: policy.capture_enabled,
        external_services_used: policy.external_services_used,
        deployment_allowed: policy.deployment_allowed,
        native_inference_execution_enabled: policy.native_inference_execution_enabled,
        non_claims: static_str_vec(RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_NON_CLAIMS),
    }
}

fn validate_runtime_workstation_snapshot_service_policy(
    policy: &RuntimeWorkstationSnapshotServicePolicy,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if policy.event_cap == 0
        || policy.event_cap > RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_DEFAULT_EVENT_CAP
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_workstation_snapshot_service.event_cap",
        });
    }
    validate_required_flag(
        "runtime_workstation_snapshot_service.local_only",
        policy.local_only,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.in_memory_only",
        policy.in_memory_only,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.service_state_enabled",
        policy.service_state_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.explicit_start_stop_enabled",
        policy.explicit_start_stop_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.snapshot_refresh_enabled",
        policy.snapshot_refresh_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.audit_events_enabled",
        policy.audit_events_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.capped_in_memory_events_enabled",
        policy.capped_in_memory_events_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.validates_snapshot_before_accept",
        policy.validates_snapshot_before_accept,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.caller_provided_snapshots_only",
        policy.caller_provided_snapshots_only,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.file_io_enabled",
        policy.file_io_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.storage_provider_enabled",
        policy.storage_provider_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.database_or_indexing_enabled",
        policy.database_or_indexing_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.generated_report_loading_enabled",
        policy.generated_report_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.generated_json_loading_enabled",
        policy.generated_json_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.raw_evidence_payload_loading_enabled",
        policy.raw_evidence_payload_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.live_transport_enabled",
        policy.live_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.public_network_transport_enabled",
        policy.public_network_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.socket_listener_enabled",
        policy.socket_listener_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.listener_loop_enabled",
        policy.listener_loop_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.daemon_lifecycle_enabled",
        policy.daemon_lifecycle_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.async_stop_api_enabled",
        policy.async_stop_api_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.process_spawning_enabled",
        policy.process_spawning_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.file_watching_enabled",
        policy.file_watching_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.qt_binding_enabled",
        policy.qt_binding_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.capture_enabled",
        policy.capture_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.external_services_used",
        policy.external_services_used,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.deployment_allowed",
        policy.deployment_allowed,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.native_inference_execution_enabled",
        policy.native_inference_execution_enabled,
        false,
    )
}

fn push_runtime_workstation_snapshot_service_event(
    events: &mut Vec<RuntimeWorkstationSnapshotServiceEvent>,
    event_cap: usize,
    state: RuntimeWorkstationSnapshotServiceState,
    event_kind: RuntimeWorkstationSnapshotServiceEventKind,
) {
    if events.len() >= event_cap {
        return;
    }
    events.push(RuntimeWorkstationSnapshotServiceEvent {
        schema_version: RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_SCHEMA_VERSION.to_owned(),
        event_index: u32::try_from(events.len()).unwrap_or(u32::MAX),
        state,
        event_kind,
        event_label: event_kind.as_str(),
        snapshot_schema_version: RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION.to_owned(),
        local_only: true,
        external_services_used: false,
        deployment_allowed: false,
        native_inference_execution_enabled: false,
    });
}

fn validate_runtime_workstation_snapshot_service_status(
    status: &RuntimeWorkstationSnapshotServiceStatus,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_schema_version(
        "runtime_workstation_snapshot_service.schema_version",
        &status.schema_version,
        RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_SCHEMA_VERSION,
    )?;
    validate_schema_version(
        "runtime_workstation_snapshot_service.accepted_snapshot_schema",
        &status.accepted_snapshot_schema,
        RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION,
    )?;
    if status.event_cap == 0
        || status.event_cap > RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_DEFAULT_EVENT_CAP
        || status.events.len() > status.event_cap
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_workstation_snapshot_service.event_cap",
        });
    }
    if (status.accepted_snapshot_count == 0) != status.latest_snapshot.is_none() {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_workstation_snapshot_service.accepted_snapshot_count",
        });
    }
    if let Some(snapshot) = &status.latest_snapshot {
        validate_runtime_workstation_snapshot(snapshot)?;
    }
    if let Some(last_event) = status.events.last() {
        if status.final_state != last_event.state && status.events.len() < status.event_cap {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_workstation_snapshot_service.final_state",
            });
        }
    }
    validate_required_flag(
        "runtime_workstation_snapshot_service.local_only",
        status.local_only,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.in_memory_only",
        status.in_memory_only,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.service_state_enabled",
        status.service_state_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.explicit_start_stop_enabled",
        status.explicit_start_stop_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.snapshot_refresh_enabled",
        status.snapshot_refresh_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.audit_events_enabled",
        status.audit_events_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.capped_in_memory_events_enabled",
        status.capped_in_memory_events_enabled,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.validates_snapshot_before_accept",
        status.validates_snapshot_before_accept,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.caller_provided_snapshots_only",
        status.caller_provided_snapshots_only,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.file_io_enabled",
        status.file_io_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.storage_provider_enabled",
        status.storage_provider_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.database_or_indexing_enabled",
        status.database_or_indexing_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.generated_report_loading_enabled",
        status.generated_report_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.generated_json_loading_enabled",
        status.generated_json_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.raw_evidence_payload_loading_enabled",
        status.raw_evidence_payload_loading_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.live_transport_enabled",
        status.live_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.public_network_transport_enabled",
        status.public_network_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.socket_listener_enabled",
        status.socket_listener_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.listener_loop_enabled",
        status.listener_loop_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.daemon_lifecycle_enabled",
        status.daemon_lifecycle_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.async_stop_api_enabled",
        status.async_stop_api_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.process_spawning_enabled",
        status.process_spawning_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.file_watching_enabled",
        status.file_watching_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.qt_binding_enabled",
        status.qt_binding_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.capture_enabled",
        status.capture_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.external_services_used",
        status.external_services_used,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.deployment_allowed",
        status.deployment_allowed,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.native_inference_execution_enabled",
        status.native_inference_execution_enabled,
        false,
    )?;
    validate_exact_strings(
        "runtime_workstation_snapshot_service.non_claims",
        &status.non_claims,
        RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_NON_CLAIMS,
    )?;
    for (index, event) in status.events.iter().enumerate() {
        validate_runtime_workstation_snapshot_service_event(event, index as u32)?;
    }
    Ok(())
}

fn validate_runtime_workstation_snapshot_service_event(
    event: &RuntimeWorkstationSnapshotServiceEvent,
    expected_index: u32,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_schema_version(
        "runtime_workstation_snapshot_service.events.schema_version",
        &event.schema_version,
        RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_SCHEMA_VERSION,
    )?;
    if event.event_index != expected_index {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_workstation_snapshot_service.events.event_index",
        });
    }
    validate_schema_version(
        "runtime_workstation_snapshot_service.events.snapshot_schema_version",
        &event.snapshot_schema_version,
        RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION,
    )?;
    validate_exact_string(
        "runtime_workstation_snapshot_service.events.event_label",
        event.event_label,
        event.event_kind.as_str(),
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.events.local_only",
        event.local_only,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.events.external_services_used",
        event.external_services_used,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.events.deployment_allowed",
        event.deployment_allowed,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot_service.events.native_inference_execution_enabled",
        event.native_inference_execution_enabled,
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

#[cfg(unix)]
fn current_effective_user_id() -> u32 {
    unsafe { geteuid() }
}

#[cfg(unix)]
fn validate_owner_only_directory(
    path: &Path,
    owner_field: &'static str,
    permissions_field: &'static str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| RuntimeControlPlaneAdapterError::MissingFile)?;
    if metadata.uid() != current_effective_user_id() {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field: owner_field });
    }
    if metadata.permissions().mode() & RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_DIRECTORY_MODE_MASK
        != 0
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: permissions_field,
        });
    }
    Ok(())
}

#[cfg(unix)]
fn validate_control_plane_endpoint_listener_path_permissions(
    selection: &RuntimeControlPlaneEndpointPathSelection,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_owner_only_directory(
        Path::new(&selection.allowed_root),
        "endpoint_listener.allowed_root_owner",
        "endpoint_listener.allowed_root_permissions",
    )?;
    let endpoint_parent = Path::new(&selection.endpoint_path)
        .parent()
        .ok_or(RuntimeControlPlaneAdapterError::MissingFile)?;
    validate_owner_only_directory(
        endpoint_parent,
        "endpoint_listener.parent_owner",
        "endpoint_listener.parent_permissions",
    )
}

#[cfg(unix)]
fn validate_control_plane_endpoint_socket_metadata(
    path: &Path,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| RuntimeControlPlaneAdapterError::MissingFile)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_socket() {
        return Err(RuntimeControlPlaneAdapterError::EndpointCleanupFailed);
    }
    if metadata.uid() != current_effective_user_id() {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "endpoint_listener.socket_owner",
        });
    }
    if metadata.permissions().mode() & 0o777 != RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SOCKET_MODE
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "endpoint_listener.socket_permissions",
        });
    }
    Ok(())
}

#[cfg(unix)]
fn restrict_control_plane_endpoint_socket_permissions(
    path: &Path,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    fs::set_permissions(
        path,
        fs::Permissions::from_mode(RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SOCKET_MODE),
    )
    .map_err(|_| RuntimeControlPlaneAdapterError::EndpointBindFailed)?;
    validate_control_plane_endpoint_socket_metadata(path)
}

fn cleanup_control_plane_endpoint_socket_path(
    path: &Path,
) -> Result<bool, RuntimeControlPlaneAdapterError> {
    if fs::symlink_metadata(path).is_err() && !path.exists() {
        return Ok(false);
    }
    #[cfg(unix)]
    validate_control_plane_endpoint_socket_metadata(path)?;
    fs::remove_file(path).map_err(|_| RuntimeControlPlaneAdapterError::EndpointCleanupFailed)?;
    Ok(true)
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

fn validate_runtime_workstation_snapshot(
    snapshot: &RuntimeWorkstationSnapshot,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_schema_version(
        "runtime_workstation_snapshot.schema_version",
        &snapshot.schema_version,
        RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION,
    )?;
    validate_schema_version(
        "runtime_workstation_snapshot.runtime_handoff_snapshot.schema_version",
        &snapshot.runtime_handoff_snapshot.schema_version,
        RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
    )?;
    validate_schema_version(
        "runtime_workstation_snapshot.evidence_index.schema_version",
        &snapshot.evidence_index.schema_version,
        EVIDENCE_INDEX_SCHEMA_VERSION,
    )?;
    validate_runtime_workstation_snapshot_safety_flags(&snapshot.safety_flags)?;
    validate_exact_strings(
        "runtime_workstation_snapshot.non_claims",
        &snapshot.non_claims,
        RUNTIME_WORKSTATION_SNAPSHOT_NON_CLAIMS,
    )?;
    validate_runtime_handoff_snapshot(&snapshot.runtime_handoff_snapshot)?;
    validate_evidence_index(&snapshot.evidence_index)?;
    validate_runtime_workstation_snapshot_aggregate_summary(&snapshot.aggregate_summary)?;
    let derived_summary = derive_runtime_workstation_snapshot_aggregate_summary(
        &snapshot.runtime_handoff_snapshot,
        &snapshot.evidence_index,
    );
    if snapshot.aggregate_summary != derived_summary {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_workstation_snapshot.aggregate_summary",
        });
    }
    Ok(())
}

fn validate_runtime_workstation_snapshot_aggregate_summary(
    summary: &RuntimeWorkstationSnapshotAggregateSummary,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if summary.runtime_total_job_count < summary.runtime_queued_job_count
        || summary.runtime_total_job_count < summary.runtime_running_job_count
        || summary.runtime_total_job_count < summary.runtime_failed_job_count
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_workstation_snapshot.aggregate_summary.runtime_job_counts",
        });
    }
    if summary.registry_model_count == 0
        || summary.registry_models_with_score_rows_count > summary.registry_model_count
        || summary.evidence_source_count == 0
        || summary.evidence_entity_count == 0
        || summary.evidence_entity_window_count == 0
        || summary.evidence_source_ref_count == 0
        || summary.evidence_ref_count == 0
        || summary.source_schema_count == 0
        || summary.evidence_model_count == 0
        || summary.model_count == 0
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_workstation_snapshot.aggregate_summary.counts",
        });
    }
    validate_sorted_unique_strings(
        "runtime_workstation_snapshot.aggregate_summary.source_schemas",
        &summary.source_schemas,
    )?;
    for schema in &summary.source_schemas {
        validate_supported_evidence_source_schema(
            "runtime_workstation_snapshot.aggregate_summary.source_schemas",
            schema,
        )?;
    }
    validate_sorted_unique_feature_names(
        "runtime_workstation_snapshot.aggregate_summary.feature_names",
        &summary.feature_names,
    )?;
    validate_sorted_unique_model_ids(
        "runtime_workstation_snapshot.aggregate_summary.model_ids",
        &summary.model_ids,
    )?;
    if summary.source_schema_count != summary.source_schemas.len() as u32
        || summary.feature_count != summary.feature_names.len() as u32
        || summary.model_count != summary.model_ids.len() as u32
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "runtime_workstation_snapshot.aggregate_summary.counts",
        });
    }
    Ok(())
}

fn validate_runtime_workstation_snapshot_safety_flags(
    flags: &RuntimeWorkstationSnapshotSafetyFlags,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.local_only",
        flags.local_only,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.strict_json_loaded",
        flags.strict_json_loaded,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.caller_provided_snapshots_only",
        flags.caller_provided_snapshots_only,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.validated_runtime_handoff_snapshot",
        flags.validated_runtime_handoff_snapshot,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.validated_evidence_index",
        flags.validated_evidence_index,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.pointer_only_evidence",
        flags.pointer_only_evidence,
        true,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.generated_json_loaded",
        flags.generated_json_loaded,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.raw_evidence_payload_copied",
        flags.raw_evidence_payload_copied,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.live_runtime_connection",
        flags.live_runtime_connection,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.file_io_enabled",
        flags.file_io_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.storage_provider_enabled",
        flags.storage_provider_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.database_or_indexing_enabled",
        flags.database_or_indexing_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.public_network_transport_enabled",
        flags.public_network_transport_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.socket_listener_enabled",
        flags.socket_listener_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.daemon_lifecycle_enabled",
        flags.daemon_lifecycle_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.process_spawning_enabled",
        flags.process_spawning_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.file_watching_enabled",
        flags.file_watching_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.qt_binding_enabled",
        flags.qt_binding_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.capture_enabled",
        flags.capture_enabled,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.external_services_used",
        flags.external_services_used,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.deployment_allowed",
        flags.deployment_allowed,
        false,
    )?;
    validate_required_flag(
        "runtime_workstation_snapshot.safety_flags.native_inference_execution_enabled",
        flags.native_inference_execution_enabled,
        false,
    )
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

#[derive(Default)]
struct EvidenceIndexDerivedSourceStats {
    entity_windows: BTreeSet<(String, String)>,
    source_ref_count: u32,
    evidence_ref_count: u32,
    feature_names: BTreeSet<String>,
    model_ids: BTreeSet<String>,
}

fn validate_evidence_index(index: &EvidenceIndex) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_exact_string(
        "evidence_index.index_scope",
        &index.index_scope,
        EVIDENCE_INDEX_SCOPE,
    )?;
    validate_exact_strings(
        "evidence_index.non_claims",
        &index.non_claims,
        EVIDENCE_INDEX_NON_CLAIMS,
    )?;
    validate_evidence_index_safety_flags(&index.safety_flags)?;
    if index.source_summaries.is_empty() {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.source_summaries",
        });
    }

    let mut summaries_by_name = BTreeMap::new();
    let mut previous_source_name: Option<&str> = None;
    for summary in &index.source_summaries {
        validate_evidence_index_source_summary(summary)?;
        if previous_source_name.is_some_and(|previous| previous >= summary.source_name.as_str())
            || summaries_by_name
                .insert(summary.source_name.as_str(), summary)
                .is_some()
        {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.source_summaries",
            });
        }
        previous_source_name = Some(summary.source_name.as_str());
    }

    let mut source_stats = index
        .source_summaries
        .iter()
        .map(|summary| {
            (
                summary.source_name.as_str(),
                EvidenceIndexDerivedSourceStats::default(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut previous_window: Option<(&str, &str)> = None;
    for row in &index.entity_window_index {
        validate_evidence_index_entity_window(row, &summaries_by_name, &mut source_stats)?;
        let current_window = (row.entity_id.as_str(), row.window_start.as_str());
        if previous_window.is_some_and(|previous| previous >= current_window) {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index",
            });
        }
        previous_window = Some(current_window);
    }

    for summary in &index.source_summaries {
        let stats = source_stats.get(summary.source_name.as_str()).ok_or(
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.source_summaries",
            },
        )?;
        if summary.entity_window_count != stats.entity_windows.len() as u32
            || summary.source_ref_count != stats.source_ref_count
            || summary.evidence_ref_count != stats.evidence_ref_count
            || summary.feature_names != string_set_to_vec(&stats.feature_names)
        {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.source_summaries",
            });
        }
        if summary.source_schema != MODEL_REGISTRY_METADATA_SCHEMA_VERSION
            && summary.model_ids != string_set_to_vec(&stats.model_ids)
        {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.source_summaries.model_ids",
            });
        }
    }

    validate_evidence_index_aggregate_summary(&index.aggregate_summary)?;
    let derived_summary = derive_evidence_index_aggregate_summary(index);
    if index.aggregate_summary != derived_summary {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.aggregate_summary",
        });
    }

    Ok(())
}

fn validate_evidence_index_source_summary(
    summary: &EvidenceIndexSourceSummary,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_safe_source_name(
        "evidence_index.source_summaries.source_name",
        &summary.source_name,
    )?;
    validate_supported_evidence_source_schema(
        "evidence_index.source_summaries.source_schema",
        &summary.source_schema,
    )?;
    validate_sorted_unique_feature_names(
        "evidence_index.source_summaries.feature_names",
        &summary.feature_names,
    )?;
    validate_sorted_unique_model_ids(
        "evidence_index.source_summaries.model_ids",
        &summary.model_ids,
    )?;
    if summary.feature_count != summary.feature_names.len() as u32 {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.source_summaries.feature_count",
        });
    }
    if summary.model_count != summary.model_ids.len() as u32 {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.source_summaries.model_count",
        });
    }
    Ok(())
}

fn validate_evidence_index_entity_window(
    row: &EvidenceIndexEntityWindow,
    summaries_by_name: &BTreeMap<&str, &EvidenceIndexSourceSummary>,
    source_stats: &mut BTreeMap<&str, EvidenceIndexDerivedSourceStats>,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_safe_entity_id(
        "evidence_index.entity_window_index.entity_id",
        &row.entity_id,
    )?;
    validate_safe_window_start(
        "evidence_index.entity_window_index.window_start",
        &row.window_start,
    )?;
    if row.source_refs.is_empty() {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.entity_window_index.source_refs",
        });
    }

    let mut derived_features = BTreeSet::new();
    let mut derived_models = BTreeSet::new();
    let mut derived_evidence_ref_count = 0_u32;
    let mut previous_ref: Option<(&str, &str, u32, &str)> = None;
    for source_ref in &row.source_refs {
        validate_evidence_index_source_ref(source_ref)?;
        let current_ref = (
            source_ref.source_name.as_str(),
            source_ref.source_schema.as_str(),
            source_ref.row_index,
            source_ref.row_kind.as_str(),
        );
        if previous_ref.is_some_and(|previous| previous >= current_ref) {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs",
            });
        }
        previous_ref = Some(current_ref);

        let summary = summaries_by_name
            .get(source_ref.source_name.as_str())
            .ok_or(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs.source_name",
            })?;
        if summary.source_schema != source_ref.source_schema {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs.source_schema",
            });
        }
        if source_ref.row_index >= summary.row_count {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs.row_index",
            });
        }

        let stats = source_stats
            .get_mut(source_ref.source_name.as_str())
            .ok_or(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs.source_name",
            })?;
        stats
            .entity_windows
            .insert((row.entity_id.clone(), row.window_start.clone()));
        stats.source_ref_count += 1;
        stats.evidence_ref_count += source_ref.evidence_indexes.len() as u32;
        stats
            .feature_names
            .extend(source_ref.feature_names.iter().cloned());
        stats.model_ids.extend(source_ref.model_ids.iter().cloned());

        derived_features.extend(source_ref.feature_names.iter().cloned());
        derived_models.extend(source_ref.model_ids.iter().cloned());
        derived_evidence_ref_count += source_ref.evidence_indexes.len() as u32;
    }

    validate_sorted_unique_feature_names(
        "evidence_index.entity_window_index.feature_names",
        &row.feature_names,
    )?;
    validate_sorted_unique_model_ids(
        "evidence_index.entity_window_index.model_ids",
        &row.model_ids,
    )?;
    if row.feature_names != string_set_to_vec(&derived_features) {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.entity_window_index.feature_names",
        });
    }
    if row.model_ids != string_set_to_vec(&derived_models) {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.entity_window_index.model_ids",
        });
    }
    if row.source_ref_count != row.source_refs.len() as u32 {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.entity_window_index.source_ref_count",
        });
    }
    if row.evidence_ref_count != derived_evidence_ref_count {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.entity_window_index.evidence_ref_count",
        });
    }

    Ok(())
}

fn validate_evidence_index_source_ref(
    source_ref: &EvidenceIndexSourceRef,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_safe_source_name(
        "evidence_index.entity_window_index.source_refs.source_name",
        &source_ref.source_name,
    )?;
    validate_supported_evidence_source_schema(
        "evidence_index.entity_window_index.source_refs.source_schema",
        &source_ref.source_schema,
    )?;
    validate_safe_model_id(
        "evidence_index.entity_window_index.source_refs.row_kind",
        &source_ref.row_kind,
    )?;
    validate_sorted_unique_feature_names(
        "evidence_index.entity_window_index.source_refs.feature_names",
        &source_ref.feature_names,
    )?;
    validate_sorted_unique_model_ids(
        "evidence_index.entity_window_index.source_refs.model_ids",
        &source_ref.model_ids,
    )?;
    let model_ids = source_ref
        .model_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut previous_evidence_ref: Option<(&str, u32)> = None;
    for evidence_ref in &source_ref.evidence_indexes {
        validate_safe_model_id(
            "evidence_index.entity_window_index.source_refs.evidence_indexes.model_id",
            &evidence_ref.model_id,
        )?;
        if !model_ids.contains(evidence_ref.model_id.as_str()) {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs.evidence_indexes.model_id",
            });
        }
        let current_ref = (evidence_ref.model_id.as_str(), evidence_ref.evidence_index);
        if previous_evidence_ref.is_some_and(|previous| previous >= current_ref) {
            return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs.evidence_indexes",
            });
        }
        previous_evidence_ref = Some(current_ref);
    }
    Ok(())
}

fn validate_evidence_index_aggregate_summary(
    summary: &EvidenceIndexAggregateSummary,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if summary.source_count == 0 {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.aggregate_summary.source_count",
        });
    }
    validate_sorted_unique_strings(
        "evidence_index.aggregate_summary.schemas_present",
        &summary.schemas_present,
    )?;
    for schema in &summary.schemas_present {
        validate_supported_evidence_source_schema(
            "evidence_index.aggregate_summary.schemas_present",
            schema,
        )?;
    }
    validate_evidence_index_count_map(
        "evidence_index.aggregate_summary.source_count_by_schema",
        &summary.source_count_by_schema,
    )?;
    validate_evidence_index_count_map(
        "evidence_index.aggregate_summary.row_count_by_schema",
        &summary.row_count_by_schema,
    )?;
    validate_sorted_unique_feature_names(
        "evidence_index.aggregate_summary.feature_names",
        &summary.feature_names,
    )?;
    validate_sorted_unique_model_ids(
        "evidence_index.aggregate_summary.model_ids",
        &summary.model_ids,
    )?;
    if summary.feature_count != summary.feature_names.len() as u32 {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.aggregate_summary.feature_count",
        });
    }
    if summary.model_count != summary.model_ids.len() as u32 {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "evidence_index.aggregate_summary.model_count",
        });
    }
    Ok(())
}

fn validate_evidence_index_count_map(
    field: &'static str,
    counts: &BTreeMap<String, u32>,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if counts.is_empty() {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    for schema in counts.keys() {
        validate_supported_evidence_source_schema(field, schema)?;
    }
    Ok(())
}

fn validate_evidence_index_safety_flags(
    flags: &EvidenceIndexSafetyFlags,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_required_flag(
        "evidence_index.safety_flags.local_only",
        flags.local_only,
        true,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.strict_json_loaded",
        flags.strict_json_loaded,
        true,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.pointer_only",
        flags.pointer_only,
        true,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.input_paths_copied",
        flags.input_paths_copied,
        false,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.source_filenames_copied",
        flags.source_filenames_copied,
        false,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.raw_evidence_payload_copied",
        flags.raw_evidence_payload_copied,
        false,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.raw_identifiers_copied",
        flags.raw_identifiers_copied,
        false,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.generated_artifact_references_copied",
        flags.generated_artifact_references_copied,
        false,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.secrets_detected",
        flags.secrets_detected,
        false,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.capture_claims_copied",
        flags.capture_claims_copied,
        false,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.live_capture_used",
        flags.live_capture_used,
        false,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.external_service_claims_copied",
        flags.external_service_claims_copied,
        false,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.external_services_used",
        flags.external_services_used,
        false,
    )?;
    validate_required_flag(
        "evidence_index.safety_flags.deployment_allowed",
        flags.deployment_allowed,
        false,
    )
}

fn derive_evidence_index_aggregate_summary(index: &EvidenceIndex) -> EvidenceIndexAggregateSummary {
    let mut source_count_by_schema = BTreeMap::<String, u32>::new();
    let mut row_count_by_schema = BTreeMap::<String, u32>::new();
    let mut entities = BTreeSet::<String>::new();
    let mut feature_names = BTreeSet::<String>::new();
    let mut model_ids = BTreeSet::<String>::new();
    let mut source_ref_count = 0_u32;
    let mut evidence_ref_count = 0_u32;

    for summary in &index.source_summaries {
        *source_count_by_schema
            .entry(summary.source_schema.clone())
            .or_default() += 1;
        *row_count_by_schema
            .entry(summary.source_schema.clone())
            .or_default() += summary.row_count;
        feature_names.extend(summary.feature_names.iter().cloned());
        model_ids.extend(summary.model_ids.iter().cloned());
    }
    for row in &index.entity_window_index {
        entities.insert(row.entity_id.clone());
        feature_names.extend(row.feature_names.iter().cloned());
        source_ref_count += row.source_ref_count;
        evidence_ref_count += row.evidence_ref_count;
    }

    EvidenceIndexAggregateSummary {
        source_count: index.source_summaries.len() as u32,
        schemas_present: source_count_by_schema.keys().cloned().collect(),
        source_count_by_schema,
        row_count_by_schema,
        entity_count: entities.len() as u32,
        entity_window_count: index.entity_window_index.len() as u32,
        source_ref_count,
        evidence_ref_count,
        feature_count: feature_names.len() as u32,
        model_count: model_ids.len() as u32,
        feature_names: string_set_to_vec(&feature_names),
        model_ids: string_set_to_vec(&model_ids),
    }
}

fn derive_runtime_workstation_snapshot_aggregate_summary(
    handoff_snapshot: &RuntimeHandoffSnapshot,
    evidence_index: &EvidenceIndex,
) -> RuntimeWorkstationSnapshotAggregateSummary {
    let mut source_schemas = BTreeSet::<String>::new();
    source_schemas.extend(
        handoff_snapshot
            .model_registry_metadata
            .aggregate_summary
            .schemas_present
            .iter()
            .cloned(),
    );
    source_schemas.extend(
        evidence_index
            .aggregate_summary
            .schemas_present
            .iter()
            .cloned(),
    );

    let mut model_ids = BTreeSet::<String>::new();
    model_ids.extend(
        handoff_snapshot
            .model_registry_metadata
            .entries
            .iter()
            .map(|entry| entry.model_id.clone()),
    );
    model_ids.extend(evidence_index.aggregate_summary.model_ids.iter().cloned());
    let model_ids = string_set_to_vec(&model_ids);
    let source_schemas = string_set_to_vec(&source_schemas);

    RuntimeWorkstationSnapshotAggregateSummary {
        workspace_id: handoff_snapshot.runtime_summary.workspace_id.clone(),
        session_id: handoff_snapshot.runtime_summary.session_id.clone(),
        runtime_total_job_count: handoff_snapshot.runtime_summary.total_job_count,
        runtime_queued_job_count: handoff_snapshot.runtime_summary.queued_job_count,
        runtime_running_job_count: handoff_snapshot.runtime_summary.running_job_count,
        runtime_failed_job_count: handoff_snapshot.runtime_summary.failed_job_count,
        registry_model_count: handoff_snapshot
            .model_registry_metadata
            .aggregate_summary
            .model_count,
        registry_models_with_score_rows_count: handoff_snapshot
            .model_registry_metadata
            .aggregate_summary
            .models_with_score_rows
            .len() as u32,
        evidence_source_count: evidence_index.aggregate_summary.source_count,
        evidence_entity_count: evidence_index.aggregate_summary.entity_count,
        evidence_entity_window_count: evidence_index.aggregate_summary.entity_window_count,
        evidence_source_ref_count: evidence_index.aggregate_summary.source_ref_count,
        evidence_ref_count: evidence_index.aggregate_summary.evidence_ref_count,
        source_schema_count: source_schemas.len() as u32,
        feature_count: evidence_index.aggregate_summary.feature_count,
        evidence_model_count: evidence_index.aggregate_summary.model_count,
        model_count: model_ids.len() as u32,
        source_schemas,
        feature_names: evidence_index.aggregate_summary.feature_names.clone(),
        model_ids,
    }
}

fn validate_supported_evidence_source_schema(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if !EVIDENCE_INDEX_SUPPORTED_SOURCE_SCHEMAS.contains(&value) {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_sorted_unique_feature_names(
    field: &'static str,
    values: &[String],
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_sorted_unique_strings(field, values)?;
    for value in values {
        validate_safe_feature_name(field, value)?;
    }
    Ok(())
}

fn validate_sorted_unique_model_ids(
    field: &'static str,
    values: &[String],
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_sorted_unique_strings(field, values)?;
    for value in values {
        validate_safe_model_id(field, value)?;
    }
    Ok(())
}

fn string_set_to_vec(values: &BTreeSet<String>) -> Vec<String> {
    values.iter().cloned().collect()
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

fn validate_safe_entity_id(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_required_safe_text(field, value)?;
    let Some(suffix) = value
        .strip_prefix("asset-")
        .or_else(|| value.strip_prefix("entity-"))
        .or_else(|| value.strip_prefix("fixture-"))
        .or_else(|| value.strip_prefix("host-"))
        .or_else(|| value.strip_prefix("sensor-"))
    else {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    };
    let mut bytes = suffix.bytes();
    let Some(first) = bytes.next() else {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    };
    if suffix.len() > 63
        || !first.is_ascii_lowercase() && !first.is_ascii_digit()
        || !bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_safe_window_start(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_required_safe_text(field, value)?;
    let Some((_date, time)) = value.split_once('T') else {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    };
    if time.is_empty()
        || !(value.ends_with('Z')
            || time.contains('+')
            || time
                .char_indices()
                .skip(1)
                .any(|(_index, character)| character == '-'))
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'-' | b':' | b'T' | b'Z' | b'+'))
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_safe_feature_name(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_no_unsafe_label_parts(field, value)?;
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    };
    if value.len() > 64
        || !first.is_ascii_lowercase()
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn validate_safe_model_id(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    validate_required_safe_text(field, value)?;
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
    validate_required_safe_text(field, value)?;
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

fn validate_required_safe_text(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if value.trim().is_empty()
        || value.trim() != value
        || value.len() > 2048
        || contains_unsafe_text(value)
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue { field });
    }
    Ok(())
}

fn contains_unsafe_text(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    lowered.contains("://")
        || value.contains('@')
        || contains_blocked_artifact_extension(&lowered)
        || lowered.contains("password")
        || lowered.contains("passwd")
        || lowered.contains("credential")
        || lowered.contains("secret")
        || lowered.contains(concat!("api", "_key"))
        || lowered.contains(concat!("api", "key"))
        || lowered.contains(concat!("private", "_key"))
        || lowered.contains(concat!("c", "ur", "l "))
        || lowered.contains("wget ")
        || lowered.contains("bash ")
        || lowered.contains("powershell ")
        || value.contains("&&")
        || value.contains("||")
        || value.contains('`')
        || contains_path_like_text(value)
        || contains_ipv4_literal(value)
}

fn contains_blocked_artifact_extension(value: &str) -> bool {
    [
        concat!(".", "p", "cap"),
        concat!(".", "p", "capng"),
        ".parquet",
        concat!(".", "job", "lib"),
        concat!(".", "p", "kl"),
        concat!(".", "on", "nx"),
        concat!(".", "p", "t"),
        concat!(".", "p", "th"),
        ".ckpt",
        concat!(".", "sql", "ite"),
        concat!(".", "duck", "db"),
        concat!(".", "json", "l"),
    ]
    .iter()
    .any(|extension| value.contains(extension))
}

fn contains_path_like_text(value: &str) -> bool {
    value
        .split_whitespace()
        .any(|part| part.matches('/').count() >= 2 || part.contains(":\\"))
}

fn contains_ipv4_literal(value: &str) -> bool {
    value
        .split(|character: char| {
            character.is_whitespace() || matches!(character, ',' | ';' | '|' | '/' | '[' | ']')
        })
        .filter(|part| !part.is_empty())
        .any(|part| {
            let candidate = part.trim_matches(|character: char| {
                matches!(character, '(' | ')' | '{' | '}' | '<' | '>' | '.' | ':')
            });
            let mut octets = candidate.split('.');
            let parsed = [octets.next(), octets.next(), octets.next(), octets.next()];
            octets.next().is_none()
                && parsed
                    .iter()
                    .all(|octet| octet.is_some_and(|value| value.parse::<u8>().is_ok()))
        })
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

fn validate_safe_endpoint_filename(value: &str) -> Result<(), RuntimeControlPlaneAdapterError> {
    let Some(stem) = value.strip_suffix(".sock") else {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedFileExtension);
    };
    if stem.is_empty() || stem.len() > RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "endpoint_path.endpoint_filename",
        });
    }
    let mut bytes = stem.bytes();
    let Some(first) = bytes.next() else {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "endpoint_path.endpoint_filename",
        });
    };
    if !first.is_ascii_lowercase()
        || !bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
        || stem
            .split(['-', '_'])
            .any(|part| RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_BLOCKED_PARTS.contains(&part))
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "endpoint_path.endpoint_filename",
        });
    }
    Ok(())
}

const RUNTIME_CONTROL_PLANE_ADAPTER_ACCEPTED_SCHEMAS: &[&str] = &[
    RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION,
    RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION,
    RUNTIME_CONTROL_PLANE_IPC_SCHEMA_VERSION,
    RUNTIME_CONTROL_PLANE_FRAME_SCHEMA_VERSION,
    RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
    RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
    RUNTIME_SUMMARY_SCHEMA_VERSION,
    MODEL_REGISTRY_METADATA_SCHEMA_VERSION,
];

const RUNTIME_CONTROL_PLANE_REQUEST_ID_BLOCKED_PARTS: &[&str] =
    &["private", "secret", "credential"];

const RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_BLOCKED_PARTS: &[&str] = &[
    "private",
    "secret",
    "credential",
    "password",
    "passwd",
    "token",
    "apikey",
];

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

const RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_NON_CLAIMS: &[&str] = &[
    "not_public_network_transport",
    "not_socket_listener",
    "not_socket_binding",
    "not_daemon_lifecycle",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_storage_provider",
    "not_capture_boundary",
    "not_external_service",
    "not_deployment_approval",
    "not_native_runtime_execution",
    "not_filesystem_mutation",
    "not_runtime_service",
];

const RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_NON_CLAIMS: &[&str] = &[
    "not_public_network_transport",
    "not_listener_loop",
    "not_daemon_lifecycle",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_storage_provider",
    "not_capture_boundary",
    "not_external_service",
    "not_deployment_approval",
    "not_native_runtime_execution",
    "not_runtime_service",
    "not_supervised_service",
];

const RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_NON_CLAIMS: &[&str] = &[
    "not_public_network_transport",
    "not_listener_loop",
    "not_daemon_lifecycle",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_storage_provider",
    "not_capture_boundary",
    "not_external_service",
    "not_deployment_approval",
    "not_native_runtime_execution",
    "not_runtime_service_daemon",
    "not_persistent_event_store",
    "not_async_stop_api",
];

const RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_NON_CLAIMS: &[&str] = &[
    "not_public_network_transport",
    "not_listener_loop",
    "not_daemon_lifecycle",
    "not_process_supervisor",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_storage_provider",
    "not_persistent_event_store",
    "not_capture_boundary",
    "not_external_service",
    "not_deployment_approval",
    "not_native_runtime_execution",
    "not_runtime_service_daemon",
    "not_async_stop_api",
    "not_multi_client_loop",
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

const RUNTIME_WORKSTATION_SNAPSHOT_NON_CLAIMS: &[&str] = &[
    "not_live_runtime_connection",
    "not_generated_json_loader",
    "not_file_loader",
    "not_storage_provider",
    "not_database_or_indexing_engine",
    "not_raw_evidence_payload_loader",
    "not_control_plane_transport",
    "not_socket_listener",
    "not_daemon_lifecycle",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_capture_boundary",
    "not_external_service",
    "not_model_promotion_gate",
    "not_deployment_approval",
    "not_native_runtime_execution",
];

const RUNTIME_WORKSTATION_SNAPSHOT_PROVIDER_NON_CLAIMS: &[&str] = &[
    "not_runtime_service",
    "not_file_loader",
    "not_storage_provider",
    "not_database_or_indexing_engine",
    "not_generated_report_loader",
    "not_generated_json_loader",
    "not_raw_evidence_payload_loader",
    "not_control_plane_transport",
    "not_public_network_transport",
    "not_socket_listener",
    "not_daemon_lifecycle",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_capture_boundary",
    "not_external_service",
    "not_deployment_approval",
    "not_native_runtime_execution",
];

const RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_NON_CLAIMS: &[&str] = &[
    "not_daemon_service",
    "not_async_runtime_service",
    "not_listener_loop",
    "not_socket_listener",
    "not_control_plane_transport",
    "not_file_loader",
    "not_storage_provider",
    "not_database_or_indexing_engine",
    "not_persistent_event_store",
    "not_generated_report_loader",
    "not_generated_json_loader",
    "not_raw_evidence_payload_loader",
    "not_process_spawner",
    "not_file_watcher",
    "not_qt_binding",
    "not_capture_boundary",
    "not_external_service",
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

const EVIDENCE_INDEX_SUPPORTED_SOURCE_SCHEMAS: &[&str] = &[
    "agentic_investigation_report.v0",
    "detection_candidate_report.v0",
    MODEL_REGISTRY_METADATA_SCHEMA_VERSION,
    "model_disagreement_report.v0",
    "model_score_rows.v0",
    "telemetry_feature_window_report.v0",
    "temporal_security_graph_report.v0",
    "time_series_residual_report.v0",
    "traffic_representation_report.v0",
];

const EVIDENCE_INDEX_NON_CLAIMS: &[&str] = &[
    "not_durable_evidence_store",
    "not_database",
    "not_live_capture",
    concat!("not_p", "cap_parser"),
    "not_private_telemetry",
    "not_external_enrichment",
    "not_rule_deployment",
    "not_model_promotion_gate",
    "not_native_runtime_execution",
    "not_qt_binding",
];

const EVIDENCE_INDEX_ADAPTER_NON_CLAIMS: &[&str] = &[
    "not_durable_evidence_store",
    "not_database_or_indexing_engine",
    "not_generated_report_loader",
    "not_raw_evidence_payload_loader",
    "not_arbitrary_file_loader",
    "not_live_capture",
    concat!("not_p", "cap_parser"),
    "not_private_telemetry",
    "not_external_enrichment",
    "not_rule_deployment",
    "not_model_promotion_gate",
    "not_qt_binding",
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
    use std::ffi::{CString, OsString};
    #[cfg(unix)]
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
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

    fn minimal_evidence_index_fixture() -> EvidenceIndex {
        EvidenceIndex::synthetic_fixture()
    }

    fn minimal_evidence_index_json() -> String {
        serde_json::to_string_pretty(&minimal_evidence_index_fixture())
            .expect("minimal evidence index fixture must serialize")
    }

    fn evidence_index_json(index: &EvidenceIndex) -> String {
        serde_json::to_string_pretty(index).expect("evidence index fixture must serialize")
    }

    fn patched_evidence_index_json(target: &str, replacement: &str) -> String {
        minimal_evidence_index_json().replacen(target, replacement, 1)
    }

    fn runtime_workstation_snapshot_json(snapshot: &RuntimeWorkstationSnapshot) -> String {
        serde_json::to_string_pretty(snapshot)
            .expect("runtime workstation snapshot fixture must serialize")
    }

    fn synthetic_runtime_workstation_snapshot_json() -> String {
        runtime_workstation_snapshot_json(&RuntimeWorkstationSnapshot::synthetic_fixture())
    }

    fn patched_runtime_workstation_snapshot_json(target: &str, replacement: &str) -> String {
        synthetic_runtime_workstation_snapshot_json().replacen(target, replacement, 1)
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
        #[cfg(unix)]
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .expect("test temp root must be owner-only");
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

    #[cfg(unix)]
    fn connect_control_plane_listener_client(path: &Path) -> UnixStream {
        let mut last_error = None;
        for _attempt in 0..100 {
            match UnixStream::connect(path) {
                Ok(stream) => return stream,
                Err(error) => {
                    last_error = Some(error);
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        }
        panic!(
            "test client could not connect to listener socket: {:?}",
            last_error
        );
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
    fn emits_static_evidence_index_adapter_contract_fixture() {
        let contract = EvidenceIndexAdapterContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            EVIDENCE_INDEX_ADAPTER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.accepted_index_schema,
            EVIDENCE_INDEX_SCHEMA_VERSION
        );
        assert_eq!(contract.accepted_index_scope, EVIDENCE_INDEX_SCOPE);
        assert_eq!(
            contract.max_file_bytes,
            RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES
        );
        assert!(contract.local_only);
        assert!(contract.pointer_only_index);
        assert!(contract.strict_json_parsing_enabled);
        assert!(contract.file_io_enabled);
        assert!(!contract.storage_provider_enabled);
        assert!(!contract.generated_report_loading_enabled);
        assert!(!contract.raw_evidence_payload_loading_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert_eq!(
            contract.non_claims,
            &[
                "not_durable_evidence_store",
                "not_database_or_indexing_engine",
                "not_generated_report_loader",
                "not_raw_evidence_payload_loader",
                "not_arbitrary_file_loader",
                "not_live_capture",
                concat!("not_p", "cap_parser"),
                "not_private_telemetry",
                "not_external_enrichment",
                "not_rule_deployment",
                "not_model_promotion_gate",
                "not_qt_binding",
                "not_native_runtime_execution"
            ]
        );
    }

    #[test]
    fn exposes_evidence_index_adapter_policy() {
        let root = temp_policy_root("evidence-index-adapter-policy");
        let policy = EvidenceIndexAdapterPolicy::new(root.clone());

        assert_eq!(policy.file_policy.allowed_root, root);
        assert_eq!(policy.max_bytes(), RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES);
        assert!(policy.local_only);
        assert!(policy.pointer_only_index);
        assert!(!policy.storage_provider_enabled);
        assert!(!policy.generated_report_loading_enabled);
        assert!(!policy.raw_evidence_payload_loading_enabled);
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
                field: "evidence_index_adapter.storage_provider_enabled",
            }
        );

        remove_temp_root(&policy.file_policy.allowed_root);
    }

    #[test]
    fn parses_minimal_evidence_index_json_string() {
        let json = minimal_evidence_index_json();
        let index = parse_evidence_index_json(&json).unwrap();
        let from_contract = EvidenceIndexAdapterContract::parse_evidence_index_json(&json).unwrap();

        assert_eq!(index, minimal_evidence_index_fixture());
        assert_eq!(from_contract, index);
        assert_eq!(index.schema_version, EVIDENCE_INDEX_SCHEMA_VERSION);
        assert_eq!(index.index_scope, EVIDENCE_INDEX_SCOPE);
        assert_eq!(index.source_summaries.len(), 2);
        assert_eq!(index.entity_window_index[0].source_ref_count, 2);
        assert_eq!(index.aggregate_summary.source_ref_count, 2);
        assert!(index.safety_flags.local_only);
        assert!(index.safety_flags.pointer_only);
        assert!(!index.safety_flags.deployment_allowed);
    }

    #[test]
    fn parses_evidence_index_file_under_allowed_root() {
        let root = temp_policy_root("valid-evidence-index-file");
        let path = write_test_file(&root, "evidence_index.json", minimal_evidence_index_json());
        let file_policy = RuntimeControlPlaneFilePolicy::new(root.clone());
        let adapter_policy = EvidenceIndexAdapterPolicy::from_file_policy(file_policy.clone());

        let from_file = parse_evidence_index_file(&path, &file_policy).unwrap();
        let from_contract =
            EvidenceIndexAdapterContract::parse_evidence_index_file(&path, &adapter_policy)
                .unwrap();
        let from_json = parse_evidence_index_json(&minimal_evidence_index_json()).unwrap();

        assert_eq!(from_file, from_json);
        assert_eq!(from_contract, from_file);
        assert_eq!(
            from_file.aggregate_summary.model_ids,
            strings(&["isolation_forest", "stdlib_linear_native"])
        );

        remove_temp_root(&root);
    }

    #[test]
    fn rejects_evidence_index_file_policy_path_violations() {
        let root = temp_policy_root("evidence-index-path-policy");
        let outside_root = temp_policy_root("outside-evidence-index-policy");
        let policy = RuntimeControlPlaneFilePolicy::new(root.clone());

        assert_eq!(
            parse_evidence_index_file(Path::new("evidence_index.json"), &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeFilePath
        );

        let relative_root_policy = RuntimeControlPlaneFilePolicy::new("relative-root");
        let relative_root_path = write_test_file(
            &root,
            "relative_root_evidence_index.json",
            minimal_evidence_index_json(),
        );
        assert_eq!(
            parse_evidence_index_file(&relative_root_path, &relative_root_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeAllowedRoot
        );

        let missing_root_policy =
            RuntimeControlPlaneFilePolicy::new(root.join("missing-policy-root"));
        let missing_root_path = root.join("missing-policy-root").join("evidence_index.json");
        assert_eq!(
            parse_evidence_index_file(&missing_root_path, &missing_root_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingAllowedRoot
        );

        let file_root = write_test_file(
            &root,
            "file_policy_root.json",
            minimal_evidence_index_json(),
        );
        let file_root_policy = RuntimeControlPlaneFilePolicy::new(file_root.clone());
        assert_eq!(
            parse_evidence_index_file(&file_root, &file_root_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootNotDirectory
        );

        let outside_path = write_test_file(
            &outside_root,
            "evidence_index.json",
            minimal_evidence_index_json(),
        );
        assert_eq!(
            parse_evidence_index_file(&outside_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OutsideAllowedRoot
        );

        let directory_path = root.join("directory.json");
        std::fs::create_dir_all(&directory_path).expect("test directory path must be created");
        assert_eq!(
            parse_evidence_index_file(&directory_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::DirectoryPath
        );

        let text_path = write_test_file(&root, "evidence_index.txt", "{}");
        assert_eq!(
            parse_evidence_index_file(&text_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedFileExtension
        );

        let missing_path = root.join("missing_evidence_index.json");
        assert_eq!(
            parse_evidence_index_file(&missing_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingFile
        );

        let oversized_path = write_test_file(
            &root,
            "oversized_evidence_index.json",
            vec![b' '; RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES as usize + 1],
        );
        assert_eq!(
            parse_evidence_index_file(&oversized_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedFile {
                max_bytes: RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES,
            }
        );

        let malformed_path = write_test_file(&root, "malformed_evidence_index.json", "{");
        assert_eq!(
            parse_evidence_index_file(&malformed_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        let invalid_utf8_path = write_test_file(&root, "invalid_utf8_evidence_index.json", [0xff]);
        assert_eq!(
            parse_evidence_index_file(&invalid_utf8_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );

        remove_temp_root(&outside_root);
        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_evidence_index_file_boundaries() {
        let root = temp_policy_root("evidence-index-symlink-policy");
        let real_root = root.join("real-root");
        std::fs::create_dir_all(&real_root).expect("test real root must be created");
        let symlink_root = root.join("symlink-root");
        std::os::unix::fs::symlink(&real_root, &symlink_root)
            .expect("test allowed root symlink must be created");
        let path = write_test_file(
            &real_root,
            "evidence_index.json",
            minimal_evidence_index_json(),
        );
        let policy = RuntimeControlPlaneFilePolicy::new(symlink_root);

        assert_eq!(
            parse_evidence_index_file(&path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootSymlink
        );

        let file_policy = RuntimeControlPlaneFilePolicy::new(real_root.clone());
        let symlink_path = real_root.join("linked_evidence_index.json");
        std::os::unix::fs::symlink(&path, &symlink_path)
            .expect("test evidence symlink must be created");
        assert_eq!(
            parse_evidence_index_file(&symlink_path, &file_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::SymlinkPath
        );

        let fifo_path = real_root.join("fifo_evidence_index.json");
        make_fifo(&fifo_path);
        assert_eq!(
            parse_evidence_index_file(&fifo_path, &file_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::NonRegularFile
        );

        remove_temp_root(&root);
    }

    #[test]
    fn rejects_malformed_or_drifted_evidence_index_json_strings() {
        assert_eq!(
            parse_evidence_index_json("{").unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert_eq!(
            parse_evidence_index_json("[]").unwrap_err(),
            RuntimeControlPlaneAdapterError::NonObjectRoot
        );

        let with_unknown_field =
            minimal_evidence_index_json().replacen("{\n", "{\n  \"unexpected_field\": true,\n", 1);
        assert_eq!(
            parse_evidence_index_json(&with_unknown_field).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json(
                r#""schema_version": "evidence_index.v0""#,
                r#""schema_version": "evidence_index.v1""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "schema_version",
                expected: EVIDENCE_INDEX_SCHEMA_VERSION,
            }
        );
        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json(
                r#""index_scope": "local_synthetic_evidence_pointer_index""#,
                r#""index_scope": "private_evidence_index""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.index_scope",
            }
        );
        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json(
                r#""not_qt_binding""#,
                r#""not_live_qt_binding""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.non_claims",
            }
        );
    }

    #[test]
    fn rejects_evidence_index_unsafe_flags() {
        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json(
                r#""local_only": true"#,
                r#""local_only": false"#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "evidence_index.safety_flags.local_only",
            }
        );
        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json(
                r#""raw_evidence_payload_copied": false"#,
                r#""raw_evidence_payload_copied": true"#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "evidence_index.safety_flags.raw_evidence_payload_copied",
            }
        );
        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json(
                r#""external_services_used": false"#,
                r#""external_services_used": true"#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "evidence_index.safety_flags.external_services_used",
            }
        );
        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json(
                r#""deployment_allowed": false"#,
                r#""deployment_allowed": true"#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "evidence_index.safety_flags.deployment_allowed",
            }
        );
    }

    #[test]
    fn rejects_evidence_index_unsafe_strings_and_unsupported_schemas() {
        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json("host-alpha", "host-secret",))
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.entity_id",
            }
        );
        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json(
                "dns_failure_ratio",
                concat!("dns_failure.", "p", "cap"),
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.source_summaries.feature_names",
            }
        );
        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json(
                "model_score_rows.v0",
                "private_report.v0",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.source_summaries.source_schema",
            }
        );
        assert_eq!(
            parse_evidence_index_json(&patched_evidence_index_json(
                "model_disagreement_report_v0_001",
                "password_001",
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.source_summaries.source_name",
            }
        );
    }

    #[test]
    fn rejects_unsorted_or_duplicate_evidence_index_rows_and_refs() {
        let mut unsorted_sources = minimal_evidence_index_fixture();
        unsorted_sources.source_summaries.swap(0, 1);
        assert_eq!(
            parse_evidence_index_json(&evidence_index_json(&unsorted_sources)).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.source_summaries",
            }
        );

        let mut unsorted_windows = minimal_evidence_index_fixture();
        let mut earlier_row = unsorted_windows.entity_window_index[0].clone();
        earlier_row.entity_id = "asset-alpha".to_owned();
        unsorted_windows.entity_window_index.push(earlier_row);
        assert_eq!(
            parse_evidence_index_json(&evidence_index_json(&unsorted_windows)).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index",
            }
        );

        let mut duplicate_source_ref = minimal_evidence_index_fixture();
        let duplicate_ref = duplicate_source_ref.entity_window_index[0].source_refs[0].clone();
        duplicate_source_ref.entity_window_index[0]
            .source_refs
            .insert(1, duplicate_ref);
        assert_eq!(
            parse_evidence_index_json(&evidence_index_json(&duplicate_source_ref)).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs",
            }
        );

        let mut duplicate_evidence_ref = minimal_evidence_index_fixture();
        let duplicate_ref = duplicate_evidence_ref.entity_window_index[0].source_refs[1]
            .evidence_indexes[0]
            .clone();
        duplicate_evidence_ref.entity_window_index[0].source_refs[1]
            .evidence_indexes
            .insert(1, duplicate_ref);
        assert_eq!(
            parse_evidence_index_json(&evidence_index_json(&duplicate_evidence_ref)).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs.evidence_indexes",
            }
        );
    }

    #[test]
    fn rejects_evidence_index_aggregate_and_count_drift() {
        let mut aggregate_drift = minimal_evidence_index_fixture();
        aggregate_drift.aggregate_summary.source_count = 1;
        assert_eq!(
            parse_evidence_index_json(&evidence_index_json(&aggregate_drift)).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.aggregate_summary",
            }
        );

        let mut source_count_drift = minimal_evidence_index_fixture();
        source_count_drift.source_summaries[0].source_ref_count = 9;
        assert_eq!(
            parse_evidence_index_json(&evidence_index_json(&source_count_drift)).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.source_summaries",
            }
        );

        let mut row_feature_drift = minimal_evidence_index_fixture();
        row_feature_drift.entity_window_index[0]
            .feature_names
            .push("z_extra_feature".to_owned());
        assert_eq!(
            parse_evidence_index_json(&evidence_index_json(&row_feature_drift)).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.feature_names",
            }
        );
    }

    #[test]
    fn rejects_evidence_index_pointer_semantic_drift() {
        let mut row_index_drift = minimal_evidence_index_fixture();
        row_index_drift.entity_window_index[0].source_refs[0].row_index = 1;
        assert_eq!(
            parse_evidence_index_json(&evidence_index_json(&row_index_drift)).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs.row_index",
            }
        );

        let mut evidence_model_drift = minimal_evidence_index_fixture();
        evidence_model_drift.entity_window_index[0].source_refs[1]
            .model_ids
            .retain(|model_id| model_id != "stdlib_linear_native");
        assert_eq!(
            parse_evidence_index_json(&evidence_index_json(&evidence_model_drift)).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.entity_window_index.source_refs.evidence_indexes.model_id",
            }
        );
    }

    #[test]
    fn validates_public_evidence_index_synthetic_fixture() {
        let fixture = EvidenceIndex::synthetic_fixture();

        validate_evidence_index(&fixture).unwrap();
        assert_eq!(fixture, minimal_evidence_index_fixture());
        assert_eq!(
            parse_evidence_index_json(&evidence_index_json(&fixture)).unwrap(),
            fixture
        );
    }

    #[test]
    fn emits_static_runtime_workstation_snapshot_provider_contract_fixture() {
        let contract = RuntimeWorkstationSnapshotProviderContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_WORKSTATION_SNAPSHOT_PROVIDER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.output_snapshot_schema,
            RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.accepted_handoff_snapshot_schema,
            RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.accepted_evidence_index_schema,
            EVIDENCE_INDEX_SCHEMA_VERSION
        );
        assert!(contract.local_only);
        assert!(contract.in_memory_only);
        assert!(contract.caller_provided_snapshots_only);
        assert!(contract.strict_runtime_handoff_validation_enabled);
        assert!(contract.strict_evidence_index_validation_enabled);
        assert!(contract.derived_aggregate_validation_enabled);
        assert!(contract.pointer_only_evidence_required);
        assert!(!contract.file_io_enabled);
        assert!(!contract.storage_provider_enabled);
        assert!(!contract.database_or_indexing_enabled);
        assert!(!contract.generated_report_loading_enabled);
        assert!(!contract.generated_json_loading_enabled);
        assert!(!contract.raw_evidence_payload_loading_enabled);
        assert!(!contract.live_transport_enabled);
        assert!(!contract.public_network_transport_enabled);
        assert!(!contract.socket_listener_enabled);
        assert!(!contract.daemon_lifecycle_enabled);
        assert!(!contract.process_spawning_enabled);
        assert!(!contract.file_watching_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert_eq!(
            contract.non_claims,
            RUNTIME_WORKSTATION_SNAPSHOT_PROVIDER_NON_CLAIMS
        );
    }

    #[test]
    fn builds_static_runtime_workstation_snapshot_fixture() {
        let snapshot = RuntimeWorkstationSnapshot::synthetic_fixture();

        validate_runtime_workstation_snapshot(&snapshot).unwrap();
        assert_eq!(
            snapshot.schema_version,
            RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(
            snapshot.runtime_handoff_snapshot,
            RuntimeHandoffSnapshot::synthetic_fixture()
        );
        assert_eq!(snapshot.evidence_index, EvidenceIndex::synthetic_fixture());
        assert_eq!(
            snapshot.aggregate_summary.workspace_id.as_str(),
            "fixture-workspace-alpha"
        );
        assert_eq!(
            snapshot.aggregate_summary.session_id.as_str(),
            "fixture-session-runtime-summary"
        );
        assert_eq!(snapshot.aggregate_summary.runtime_total_job_count, 4);
        assert_eq!(snapshot.aggregate_summary.registry_model_count, 10);
        assert_eq!(
            snapshot
                .aggregate_summary
                .registry_models_with_score_rows_count,
            8
        );
        assert_eq!(snapshot.aggregate_summary.evidence_source_count, 2);
        assert_eq!(snapshot.aggregate_summary.evidence_entity_count, 1);
        assert_eq!(snapshot.aggregate_summary.evidence_entity_window_count, 1);
        assert_eq!(snapshot.aggregate_summary.evidence_source_ref_count, 2);
        assert_eq!(snapshot.aggregate_summary.evidence_ref_count, 3);
        assert_eq!(
            snapshot.aggregate_summary.feature_names,
            strings(&["dns_failure_ratio"])
        );
        assert!(snapshot
            .aggregate_summary
            .model_ids
            .contains(&"isolation_forest".to_owned()));
        assert!(snapshot
            .aggregate_summary
            .model_ids
            .contains(&"stdlib_linear_native".to_owned()));
        assert_eq!(
            snapshot.aggregate_summary.model_count,
            snapshot.aggregate_summary.model_ids.len() as u32
        );
        assert!(snapshot.safety_flags.local_only);
        assert!(snapshot.safety_flags.strict_json_loaded);
        assert!(snapshot.safety_flags.pointer_only_evidence);
        assert!(!snapshot.safety_flags.generated_json_loaded);
        assert!(!snapshot.safety_flags.file_io_enabled);
        assert!(!snapshot.safety_flags.qt_binding_enabled);
        assert!(!snapshot.safety_flags.capture_enabled);
        assert!(!snapshot.safety_flags.external_services_used);
        assert!(!snapshot.safety_flags.deployment_allowed);
        assert_eq!(
            snapshot.non_claims,
            static_str_vec(RUNTIME_WORKSTATION_SNAPSHOT_NON_CLAIMS)
        );
    }

    #[test]
    fn builds_runtime_workstation_snapshot_from_contract_api() {
        let policy = RuntimeWorkstationSnapshotProviderPolicy::new();
        let snapshot =
            RuntimeWorkstationSnapshotProviderContract::build_runtime_workstation_snapshot(
                RuntimeHandoffSnapshot::synthetic_fixture(),
                EvidenceIndex::synthetic_fixture(),
                &policy,
            )
            .unwrap();

        assert_eq!(snapshot, RuntimeWorkstationSnapshot::synthetic_fixture());
    }

    #[test]
    fn parses_runtime_workstation_snapshot_json_string() {
        let json = synthetic_runtime_workstation_snapshot_json();
        let snapshot = parse_runtime_workstation_snapshot_json(&json).unwrap();
        let from_contract =
            RuntimeWorkstationSnapshotProviderContract::parse_runtime_workstation_snapshot_json(
                &json,
            )
            .unwrap();

        assert_eq!(snapshot, RuntimeWorkstationSnapshot::synthetic_fixture());
        assert_eq!(from_contract, snapshot);
    }

    #[test]
    fn rejects_runtime_workstation_snapshot_provider_unsafe_flags() {
        let mut file_io_policy = RuntimeWorkstationSnapshotProviderPolicy::new();
        file_io_policy.file_io_enabled = true;
        assert_eq!(
            build_runtime_workstation_snapshot(
                RuntimeHandoffSnapshot::synthetic_fixture(),
                EvidenceIndex::synthetic_fixture(),
                &file_io_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_workstation_snapshot_provider.file_io_enabled",
            }
        );

        let mut non_local_policy = RuntimeWorkstationSnapshotProviderPolicy::new();
        non_local_policy.local_only = false;
        assert_eq!(
            RuntimeWorkstationSnapshotProviderContract::build_runtime_workstation_snapshot(
                RuntimeHandoffSnapshot::synthetic_fixture(),
                EvidenceIndex::synthetic_fixture(),
                &non_local_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_workstation_snapshot_provider.local_only",
            }
        );
    }

    #[test]
    fn rejects_malformed_or_drifted_runtime_workstation_snapshot_json_strings() {
        assert_eq!(
            parse_runtime_workstation_snapshot_json("{").unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );
        assert_eq!(
            parse_runtime_workstation_snapshot_json("[]").unwrap_err(),
            RuntimeControlPlaneAdapterError::NonObjectRoot
        );

        let with_unknown_field = synthetic_runtime_workstation_snapshot_json().replacen(
            "{\n",
            "{\n  \"unexpected_field\": true,\n",
            1,
        );
        assert_eq!(
            parse_runtime_workstation_snapshot_json(&with_unknown_field).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidJson
        );

        assert_eq!(
            parse_runtime_workstation_snapshot_json(&patched_runtime_workstation_snapshot_json(
                r#""schema_version": "runtime_workstation_snapshot.v0""#,
                r#""schema_version": "runtime_workstation_snapshot.v1""#,
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "runtime_workstation_snapshot.schema_version",
                expected: RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION,
            }
        );

        let mut non_claim_drift = RuntimeWorkstationSnapshot::synthetic_fixture();
        non_claim_drift.non_claims[0] = "not_real_workstation_snapshot".to_owned();
        assert_eq!(
            parse_runtime_workstation_snapshot_json(&runtime_workstation_snapshot_json(
                &non_claim_drift
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_workstation_snapshot.non_claims",
            }
        );

        let mut aggregate_drift = RuntimeWorkstationSnapshot::synthetic_fixture();
        aggregate_drift.aggregate_summary.evidence_ref_count += 1;
        assert_eq!(
            parse_runtime_workstation_snapshot_json(&runtime_workstation_snapshot_json(
                &aggregate_drift
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_workstation_snapshot.aggregate_summary",
            }
        );
    }

    #[test]
    fn rejects_runtime_workstation_snapshot_unsafe_flags_and_nested_drift() {
        let mut unsafe_snapshot = RuntimeWorkstationSnapshot::synthetic_fixture();
        unsafe_snapshot.safety_flags.file_io_enabled = true;
        assert_eq!(
            parse_runtime_workstation_snapshot_json(&runtime_workstation_snapshot_json(
                &unsafe_snapshot
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_workstation_snapshot.safety_flags.file_io_enabled",
            }
        );

        let mut handoff_schema_drift = RuntimeWorkstationSnapshot::synthetic_fixture();
        handoff_schema_drift.runtime_handoff_snapshot.schema_version =
            "runtime_handoff_snapshot.v1".to_owned();
        assert_eq!(
            parse_runtime_workstation_snapshot_json(&runtime_workstation_snapshot_json(
                &handoff_schema_drift
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "runtime_workstation_snapshot.runtime_handoff_snapshot.schema_version",
                expected: RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
            }
        );

        let mut unsafe_handoff = RuntimeWorkstationSnapshot::synthetic_fixture();
        unsafe_handoff.runtime_handoff_snapshot.local_only = false;
        assert_eq!(
            parse_runtime_workstation_snapshot_json(&runtime_workstation_snapshot_json(
                &unsafe_handoff
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "local_only",
            }
        );

        let mut evidence_schema_drift = RuntimeWorkstationSnapshot::synthetic_fixture();
        evidence_schema_drift.evidence_index.schema_version = "evidence_index.v1".to_owned();
        assert_eq!(
            parse_runtime_workstation_snapshot_json(&runtime_workstation_snapshot_json(
                &evidence_schema_drift
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedSchemaVersion {
                field: "runtime_workstation_snapshot.evidence_index.schema_version",
                expected: EVIDENCE_INDEX_SCHEMA_VERSION,
            }
        );

        let mut unsafe_evidence = RuntimeWorkstationSnapshot::synthetic_fixture();
        unsafe_evidence.evidence_index.safety_flags.pointer_only = false;
        assert_eq!(
            parse_runtime_workstation_snapshot_json(&runtime_workstation_snapshot_json(
                &unsafe_evidence
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "evidence_index.safety_flags.pointer_only",
            }
        );

        let mut evidence_aggregate_drift = RuntimeWorkstationSnapshot::synthetic_fixture();
        evidence_aggregate_drift
            .evidence_index
            .aggregate_summary
            .source_count = 1;
        assert_eq!(
            parse_runtime_workstation_snapshot_json(&runtime_workstation_snapshot_json(
                &evidence_aggregate_drift
            ))
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "evidence_index.aggregate_summary",
            }
        );
    }

    #[test]
    fn emits_static_runtime_workstation_snapshot_service_contract_fixture() {
        let contract = RuntimeWorkstationSnapshotServiceContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_SCHEMA_VERSION
        );
        assert_eq!(
            contract.accepted_snapshot_schema,
            RUNTIME_WORKSTATION_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.default_event_cap,
            RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_DEFAULT_EVENT_CAP
        );
        assert!(contract.local_only);
        assert!(contract.in_memory_only);
        assert!(contract.service_state_enabled);
        assert!(contract.explicit_start_stop_enabled);
        assert!(contract.snapshot_refresh_enabled);
        assert!(contract.audit_events_enabled);
        assert!(contract.capped_in_memory_events_enabled);
        assert!(contract.validates_snapshot_before_accept);
        assert!(contract.caller_provided_snapshots_only);
        assert!(!contract.file_io_enabled);
        assert!(!contract.storage_provider_enabled);
        assert!(!contract.database_or_indexing_enabled);
        assert!(!contract.generated_report_loading_enabled);
        assert!(!contract.generated_json_loading_enabled);
        assert!(!contract.raw_evidence_payload_loading_enabled);
        assert!(!contract.live_transport_enabled);
        assert!(!contract.public_network_transport_enabled);
        assert!(!contract.socket_listener_enabled);
        assert!(!contract.listener_loop_enabled);
        assert!(!contract.daemon_lifecycle_enabled);
        assert!(!contract.async_stop_api_enabled);
        assert!(!contract.process_spawning_enabled);
        assert!(!contract.file_watching_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert_eq!(
            contract.non_claims,
            RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_NON_CLAIMS
        );
    }

    #[test]
    fn supervises_runtime_workstation_snapshot_service_start_refresh_stop() {
        let policy = RuntimeWorkstationSnapshotServicePolicy::new();
        let mut supervisor = RuntimeWorkstationSnapshotServiceSupervisor::new(&policy).unwrap();
        let initial_snapshot = RuntimeWorkstationSnapshot::synthetic_fixture();
        let mut refresh_snapshot = RuntimeWorkstationSnapshot::synthetic_fixture();
        refresh_snapshot.aggregate_summary.runtime_total_job_count = 5;
        refresh_snapshot
            .runtime_handoff_snapshot
            .runtime_summary
            .total_job_count = 5;

        supervisor.start(initial_snapshot.clone()).unwrap();
        assert_eq!(
            supervisor.state(),
            RuntimeWorkstationSnapshotServiceState::Running
        );
        assert_eq!(supervisor.accepted_snapshot_count(), 1);
        assert_eq!(supervisor.latest_snapshot(), Some(&initial_snapshot));

        supervisor
            .refresh_snapshot(refresh_snapshot.clone())
            .unwrap();
        assert_eq!(
            supervisor.state(),
            RuntimeWorkstationSnapshotServiceState::Running
        );
        assert_eq!(supervisor.accepted_snapshot_count(), 2);
        assert_eq!(supervisor.latest_snapshot(), Some(&refresh_snapshot));

        supervisor.stop().unwrap();
        let status = supervisor.status();
        validate_runtime_workstation_snapshot_service_status(&status).unwrap();
        assert_eq!(
            status.final_state,
            RuntimeWorkstationSnapshotServiceState::Stopped
        );
        assert_eq!(status.accepted_snapshot_count, 2);
        assert_eq!(status.latest_snapshot, Some(refresh_snapshot));
        assert_eq!(
            status
                .events
                .iter()
                .map(|event| event.event_kind.as_str())
                .collect::<Vec<_>>(),
            vec![
                "start_requested",
                "snapshot_accepted",
                "refresh_requested",
                "snapshot_refreshed",
                "stop_requested",
                "stopped"
            ]
        );
        assert!(status.local_only);
        assert!(status.in_memory_only);
        assert!(!status.file_io_enabled);
        assert!(!status.storage_provider_enabled);
        assert!(!status.socket_listener_enabled);
        assert!(!status.daemon_lifecycle_enabled);
        assert!(!status.qt_binding_enabled);
        assert!(!status.capture_enabled);
        assert!(!status.external_services_used);
        assert!(!status.deployment_allowed);
        assert!(!status.native_inference_execution_enabled);
    }

    #[test]
    fn executes_runtime_workstation_snapshot_service_once_from_contract_api() {
        let policy = RuntimeWorkstationSnapshotServicePolicy::new();
        let refresh_snapshot = RuntimeWorkstationSnapshot::synthetic_fixture();
        let status = RuntimeWorkstationSnapshotServiceContract::execute_once(
            RuntimeWorkstationSnapshot::synthetic_fixture(),
            &[refresh_snapshot.clone()],
            &policy,
        )
        .unwrap();

        validate_runtime_workstation_snapshot_service_status(&status).unwrap();
        assert_eq!(
            status.final_state,
            RuntimeWorkstationSnapshotServiceState::Stopped
        );
        assert_eq!(status.accepted_snapshot_count, 2);
        assert_eq!(status.latest_snapshot, Some(refresh_snapshot));
        assert_eq!(
            status.event_cap,
            RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_DEFAULT_EVENT_CAP
        );
        assert_eq!(status.events.len(), 6);
    }

    #[test]
    fn caps_runtime_workstation_snapshot_service_events_in_memory() {
        let policy = RuntimeWorkstationSnapshotServicePolicy::bounded(3).unwrap();
        let refresh_snapshot = RuntimeWorkstationSnapshot::synthetic_fixture();
        let status = execute_runtime_workstation_snapshot_service_once(
            RuntimeWorkstationSnapshot::synthetic_fixture(),
            &[refresh_snapshot.clone()],
            &policy,
        )
        .unwrap();

        validate_runtime_workstation_snapshot_service_status(&status).unwrap();
        assert_eq!(
            status.final_state,
            RuntimeWorkstationSnapshotServiceState::Stopped
        );
        assert_eq!(status.event_cap, 3);
        assert_eq!(status.events.len(), 3);
        assert_eq!(status.accepted_snapshot_count, 2);
        assert_eq!(status.latest_snapshot, Some(refresh_snapshot));
    }

    #[test]
    fn rejects_runtime_workstation_snapshot_service_policy_drift() {
        assert_eq!(
            RuntimeWorkstationSnapshotServicePolicy::bounded(0).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_workstation_snapshot_service.event_cap",
            }
        );
        assert_eq!(
            RuntimeWorkstationSnapshotServicePolicy::bounded(
                RUNTIME_WORKSTATION_SNAPSHOT_SERVICE_DEFAULT_EVENT_CAP + 1,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_workstation_snapshot_service.event_cap",
            }
        );

        let mut policy = RuntimeWorkstationSnapshotServicePolicy::new();
        policy.file_io_enabled = true;
        assert_eq!(
            RuntimeWorkstationSnapshotServiceSupervisor::new(&policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_workstation_snapshot_service.file_io_enabled",
            }
        );

        let mut policy = RuntimeWorkstationSnapshotServicePolicy::new();
        policy.daemon_lifecycle_enabled = true;
        assert_eq!(
            execute_runtime_workstation_snapshot_service_once(
                RuntimeWorkstationSnapshot::synthetic_fixture(),
                &[],
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_workstation_snapshot_service.daemon_lifecycle_enabled",
            }
        );
    }

    #[test]
    fn rejects_runtime_workstation_snapshot_service_invalid_transitions_and_snapshot_drift() {
        let policy = RuntimeWorkstationSnapshotServicePolicy::new();
        let mut supervisor = RuntimeWorkstationSnapshotServiceSupervisor::new(&policy).unwrap();
        assert_eq!(
            supervisor.stop().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_workstation_snapshot_service.transition",
            }
        );

        let mut drifted_snapshot = RuntimeWorkstationSnapshot::synthetic_fixture();
        drifted_snapshot.safety_flags.local_only = false;
        let mut supervisor = RuntimeWorkstationSnapshotServiceSupervisor::new(&policy).unwrap();
        assert_eq!(
            supervisor.start(drifted_snapshot).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_workstation_snapshot.safety_flags.local_only",
            }
        );
        assert_eq!(
            supervisor.state(),
            RuntimeWorkstationSnapshotServiceState::Failed
        );
        assert_eq!(
            supervisor
                .events()
                .iter()
                .map(|event| event.event_kind)
                .collect::<Vec<_>>(),
            vec![
                RuntimeWorkstationSnapshotServiceEventKind::StartRequested,
                RuntimeWorkstationSnapshotServiceEventKind::Failed,
            ]
        );
    }

    #[test]
    fn rejects_runtime_workstation_snapshot_service_status_drift() {
        let policy = RuntimeWorkstationSnapshotServicePolicy::new();
        let mut supervisor = RuntimeWorkstationSnapshotServiceSupervisor::new(&policy).unwrap();
        supervisor
            .start(RuntimeWorkstationSnapshot::synthetic_fixture())
            .unwrap();
        supervisor.stop().unwrap();

        let mut unsafe_status = supervisor.status();
        unsafe_status.deployment_allowed = true;
        assert_eq!(
            validate_runtime_workstation_snapshot_service_status(&unsafe_status).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "runtime_workstation_snapshot_service.deployment_allowed",
            }
        );

        let mut non_claim_drift = supervisor.status();
        non_claim_drift.non_claims[0] = "not_real_service_guard".to_owned();
        assert_eq!(
            validate_runtime_workstation_snapshot_service_status(&non_claim_drift).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_workstation_snapshot_service.non_claims",
            }
        );

        let mut event_drift = supervisor.status();
        event_drift.events[0].event_label = "started";
        assert_eq!(
            validate_runtime_workstation_snapshot_service_status(&event_drift).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "runtime_workstation_snapshot_service.events.event_label",
            }
        );
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
                RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION,
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
    fn emits_static_runtime_control_plane_endpoint_path_contract_fixture() {
        let contract = RuntimeControlPlaneEndpointPathContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION
        );
        assert_eq!(
            contract.endpoint_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.max_path_bytes,
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES
        );
        assert!(contract.local_only);
        assert!(contract.caller_authorized_allowed_root_required);
        assert!(contract.absolute_allowed_root_required);
        assert!(contract.absolute_endpoint_path_required);
        assert!(contract.allowed_root_must_exist);
        assert!(contract.allowed_root_symlink_rejected);
        assert!(contract.target_parent_must_exist);
        assert!(contract.target_parent_symlink_rejected);
        assert!(contract.target_must_not_exist);
        assert!(contract.socket_extension_required);
        assert!(contract.endpoint_filename_safety_enabled);
        assert!(contract.path_selection_only);
        assert!(contract.filesystem_socket_path_policy_enabled);
        assert!(contract.filesystem_metadata_validation_enabled);
        assert!(!contract.filesystem_mutation_enabled);
        assert!(!contract.public_network_transport_enabled);
        assert!(!contract.socket_listener_enabled);
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
                "not_socket_binding",
                "not_daemon_lifecycle",
                "not_process_spawner",
                "not_file_watcher",
                "not_qt_binding",
                "not_storage_provider",
                "not_capture_boundary",
                "not_external_service",
                "not_deployment_approval",
                "not_native_runtime_execution",
                "not_filesystem_mutation",
                "not_runtime_service"
            ]
        );
    }

    #[test]
    fn emits_static_runtime_control_plane_endpoint_listener_contract_fixture() {
        let contract = RuntimeControlPlaneEndpointListenerContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.endpoint_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.endpoint_path_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION
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
            contract.max_path_bytes,
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES
        );
        assert_eq!(
            contract.max_frame_bytes,
            RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
        );
        assert!(contract.local_only);
        assert!(contract.one_shot_listener);
        assert!(contract.filesystem_socket_binding_enabled);
        assert!(contract.cleanup_on_completion);
        assert!(contract.endpoint_path_validation_enabled);
        assert!(contract.endpoint_stream_execution_enabled);
        assert!(!contract.public_network_transport_enabled);
        assert!(!contract.listener_loop_enabled);
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
                "not_listener_loop",
                "not_daemon_lifecycle",
                "not_process_spawner",
                "not_file_watcher",
                "not_qt_binding",
                "not_storage_provider",
                "not_capture_boundary",
                "not_external_service",
                "not_deployment_approval",
                "not_native_runtime_execution",
                "not_runtime_service",
                "not_supervised_service"
            ]
        );
    }

    #[test]
    fn emits_static_runtime_control_plane_endpoint_lifecycle_contract_fixture() {
        let contract = RuntimeControlPlaneEndpointLifecycleContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_SCHEMA_VERSION
        );
        assert_eq!(
            contract.listener_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.endpoint_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.endpoint_path_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION
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
            contract.max_path_bytes,
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES
        );
        assert_eq!(
            contract.max_frame_bytes,
            RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
        );
        assert!(contract.local_only);
        assert!(contract.one_shot_lifecycle);
        assert!(contract.start_stop_state_enabled);
        assert!(contract.audit_events_enabled);
        assert!(contract.endpoint_listener_execution_enabled);
        assert!(contract.cleanup_on_completion);
        assert!(!contract.public_network_transport_enabled);
        assert!(!contract.listener_loop_enabled);
        assert!(!contract.daemon_lifecycle_enabled);
        assert!(!contract.process_spawning_enabled);
        assert!(!contract.file_watching_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.storage_provider_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert!(!contract.persistent_event_store_enabled);
        assert_eq!(
            contract.non_claims,
            &[
                "not_public_network_transport",
                "not_listener_loop",
                "not_daemon_lifecycle",
                "not_process_spawner",
                "not_file_watcher",
                "not_qt_binding",
                "not_storage_provider",
                "not_capture_boundary",
                "not_external_service",
                "not_deployment_approval",
                "not_native_runtime_execution",
                "not_runtime_service_daemon",
                "not_persistent_event_store",
                "not_async_stop_api"
            ]
        );
        assert_eq!(
            RuntimeControlPlaneEndpointLifecycleState::NotStarted.as_str(),
            "not_started"
        );
        assert_eq!(
            RuntimeControlPlaneEndpointLifecycleEventKind::StartRequested.as_str(),
            "start_requested"
        );
    }

    #[test]
    fn emits_static_runtime_control_plane_service_lifecycle_contract_fixture() {
        let contract = RuntimeControlPlaneServiceLifecycleContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_SCHEMA_VERSION
        );
        assert_eq!(
            contract.endpoint_lifecycle_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_SCHEMA_VERSION
        );
        assert_eq!(
            contract.listener_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.endpoint_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.endpoint_path_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION
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
            contract.default_event_cap,
            RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_DEFAULT_EVENT_CAP
        );
        assert_eq!(
            contract.max_path_bytes,
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES
        );
        assert_eq!(
            contract.max_frame_bytes,
            RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
        );
        assert!(contract.local_only);
        assert!(contract.service_state_enabled);
        assert!(contract.explicit_start_stop_state_enabled);
        assert!(contract.one_shot_endpoint_execution_enabled);
        assert!(contract.audit_events_enabled);
        assert!(contract.capped_in_memory_events_enabled);
        assert!(contract.nested_endpoint_lifecycle_execution_enabled);
        assert!(contract.cleanup_on_completion);
        assert!(!contract.public_network_transport_enabled);
        assert!(!contract.listener_loop_enabled);
        assert!(!contract.daemon_lifecycle_enabled);
        assert!(!contract.async_stop_api_enabled);
        assert!(!contract.process_spawning_enabled);
        assert!(!contract.file_watching_enabled);
        assert!(!contract.qt_binding_enabled);
        assert!(!contract.storage_provider_enabled);
        assert!(!contract.persistent_event_store_enabled);
        assert!(!contract.capture_enabled);
        assert!(!contract.external_services_used);
        assert!(!contract.deployment_allowed);
        assert!(!contract.native_inference_execution_enabled);
        assert_eq!(
            contract.non_claims,
            &[
                "not_public_network_transport",
                "not_listener_loop",
                "not_daemon_lifecycle",
                "not_process_supervisor",
                "not_process_spawner",
                "not_file_watcher",
                "not_qt_binding",
                "not_storage_provider",
                "not_persistent_event_store",
                "not_capture_boundary",
                "not_external_service",
                "not_deployment_approval",
                "not_native_runtime_execution",
                "not_runtime_service_daemon",
                "not_async_stop_api",
                "not_multi_client_loop"
            ]
        );
        assert_eq!(
            RuntimeControlPlaneServiceLifecycleState::RunningEndpointOnce.as_str(),
            "running_endpoint_once"
        );
        assert_eq!(
            RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleStarted.as_str(),
            "endpoint_lifecycle_started"
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
    fn exposes_bounded_runtime_control_plane_endpoint_path_policy() {
        let root = temp_policy_root("endpoint-path-policy");
        let policy = RuntimeControlPlaneEndpointPathPolicy::new(root.clone());

        assert_eq!(policy.allowed_root, root);
        assert_eq!(
            policy.max_bytes(),
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES
        );
        assert!(policy.local_only);
        assert!(policy.caller_authorized_allowed_root_required);
        assert!(policy.path_selection_only);
        assert!(policy.filesystem_socket_path_policy_enabled);
        assert!(!policy.filesystem_mutation_enabled);
        assert!(!policy.public_network_transport_enabled);
        assert!(!policy.socket_listener_enabled);
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

        let bounded = RuntimeControlPlaneEndpointPathPolicy::bounded(
            policy.allowed_root.clone(),
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES - 1,
        )
        .unwrap();
        assert_eq!(
            bounded.max_bytes(),
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES - 1
        );

        assert_eq!(
            RuntimeControlPlaneEndpointPathPolicy::bounded(root.clone(), 0).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.max_path_bytes",
            }
        );
        assert_eq!(
            RuntimeControlPlaneEndpointPathPolicy::bounded(
                root.clone(),
                RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES + 1,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.max_path_bytes",
            }
        );

        remove_temp_root(&root);
    }

    #[test]
    fn exposes_bounded_runtime_control_plane_endpoint_listener_policy() {
        let root = temp_policy_root("endpoint-listener-policy");
        let path_policy = RuntimeControlPlaneEndpointPathPolicy::new(root.clone());
        let policy = RuntimeControlPlaneEndpointListenerPolicy::new(path_policy);

        assert_eq!(
            policy.max_path_bytes(),
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES
        );
        assert_eq!(
            policy.max_frame_bytes(),
            RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
        );
        assert!(policy.local_only);
        assert!(policy.one_shot_listener);
        assert!(policy.filesystem_socket_binding_enabled);
        assert!(policy.cleanup_on_completion);
        assert!(policy.endpoint_path_validation_enabled);
        assert!(policy.endpoint_stream_execution_enabled);
        assert!(!policy.public_network_transport_enabled);
        assert!(!policy.listener_loop_enabled);
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
        let endpoint_policy = RuntimeControlPlaneEndpointPolicy::caller_provided_connected_stream(
            RuntimeControlPlaneIpcPolicy::new(frame_policy),
        );
        let bounded_policy = RuntimeControlPlaneEndpointListenerPolicy::with_endpoint_policy(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
            endpoint_policy,
        );
        assert_eq!(bounded_policy.max_frame_bytes(), 1024);

        remove_temp_root(&root);
    }

    #[test]
    fn exposes_bounded_runtime_control_plane_endpoint_lifecycle_policy() {
        let root = temp_policy_root("endpoint-lifecycle-policy");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let policy = RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);

        assert_eq!(
            policy.max_path_bytes(),
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES
        );
        assert_eq!(
            policy.max_frame_bytes(),
            RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
        );
        assert!(policy.local_only);
        assert!(policy.one_shot_lifecycle);
        assert!(policy.start_stop_state_enabled);
        assert!(policy.audit_events_enabled);
        assert!(policy.endpoint_listener_execution_enabled);
        assert!(policy.cleanup_on_completion);
        assert!(!policy.public_network_transport_enabled);
        assert!(!policy.listener_loop_enabled);
        assert!(!policy.daemon_lifecycle_enabled);
        assert!(!policy.process_spawning_enabled);
        assert!(!policy.file_watching_enabled);
        assert!(!policy.qt_binding_enabled);
        assert!(!policy.storage_provider_enabled);
        assert!(!policy.capture_enabled);
        assert!(!policy.external_services_used);
        assert!(!policy.deployment_allowed);
        assert!(!policy.native_inference_execution_enabled);
        assert!(!policy.persistent_event_store_enabled);
        policy.validate().unwrap();

        remove_temp_root(&root);
    }

    #[test]
    fn exposes_bounded_runtime_control_plane_service_lifecycle_policy() {
        let root = temp_policy_root("service-lifecycle-policy");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let endpoint_lifecycle_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);
        let policy =
            RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy.clone());

        assert_eq!(
            policy.event_cap,
            RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_DEFAULT_EVENT_CAP
        );
        assert_eq!(
            policy.max_path_bytes(),
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES
        );
        assert_eq!(
            policy.max_frame_bytes(),
            RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES
        );
        assert!(policy.local_only);
        assert!(policy.service_state_enabled);
        assert!(policy.explicit_start_stop_state_enabled);
        assert!(policy.one_shot_endpoint_execution_enabled);
        assert!(policy.audit_events_enabled);
        assert!(policy.capped_in_memory_events_enabled);
        assert!(policy.nested_endpoint_lifecycle_execution_enabled);
        assert!(policy.cleanup_on_completion);
        assert!(!policy.public_network_transport_enabled);
        assert!(!policy.listener_loop_enabled);
        assert!(!policy.daemon_lifecycle_enabled);
        assert!(!policy.async_stop_api_enabled);
        assert!(!policy.process_spawning_enabled);
        assert!(!policy.file_watching_enabled);
        assert!(!policy.qt_binding_enabled);
        assert!(!policy.storage_provider_enabled);
        assert!(!policy.persistent_event_store_enabled);
        assert!(!policy.capture_enabled);
        assert!(!policy.external_services_used);
        assert!(!policy.deployment_allowed);
        assert!(!policy.native_inference_execution_enabled);
        policy.validate().unwrap();

        let capped_policy =
            RuntimeControlPlaneServiceLifecyclePolicy::bounded(endpoint_lifecycle_policy, 3)
                .unwrap();
        assert_eq!(capped_policy.event_cap, 3);
        capped_policy.validate().unwrap();

        remove_temp_root(&root);
    }

    #[test]
    fn endpoint_listener_policy_rejects_unsafe_flags_and_nested_drift() {
        let root = temp_policy_root("endpoint-listener-policy-drift");

        let mut network_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        network_policy.public_network_transport_enabled = true;
        assert_eq!(
            network_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_listener.public_network_transport_enabled",
            }
        );

        let mut loop_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        loop_policy.listener_loop_enabled = true;
        assert_eq!(
            loop_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_listener.listener_loop_enabled",
            }
        );

        let mut cleanup_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        cleanup_policy.cleanup_on_completion = false;
        assert_eq!(
            cleanup_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_listener.cleanup_on_completion",
            }
        );

        let mut unsafe_path_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        unsafe_path_policy
            .endpoint_path_policy
            .public_network_transport_enabled = true;
        assert_eq!(
            unsafe_path_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_path.public_network_transport_enabled",
            }
        );

        let mut unsafe_endpoint_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        unsafe_endpoint_policy
            .endpoint_policy
            .socket_listener_enabled = true;
        assert_eq!(
            unsafe_endpoint_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint.socket_listener_enabled",
            }
        );

        let mut oversized_frame_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        oversized_frame_policy
            .endpoint_policy
            .ipc_policy
            .frame_policy
            .max_frame_bytes = RUNTIME_CONTROL_PLANE_FRAME_MAX_BYTES + 1;
        assert_eq!(
            oversized_frame_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint.ipc_policy.frame_policy.max_frame_bytes",
            }
        );

        remove_temp_root(&root);
    }

    #[test]
    fn endpoint_lifecycle_policy_rejects_unsafe_flags_and_nested_drift() {
        let root = temp_policy_root("endpoint-lifecycle-policy-drift");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );

        let mut network_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy.clone());
        network_policy.public_network_transport_enabled = true;
        assert_eq!(
            network_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_lifecycle.public_network_transport_enabled",
            }
        );

        let mut loop_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy.clone());
        loop_policy.listener_loop_enabled = true;
        assert_eq!(
            loop_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_lifecycle.listener_loop_enabled",
            }
        );

        let mut audit_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy.clone());
        audit_policy.audit_events_enabled = false;
        assert_eq!(
            audit_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_lifecycle.audit_events_enabled",
            }
        );

        let mut event_store_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy.clone());
        event_store_policy.persistent_event_store_enabled = true;
        assert_eq!(
            event_store_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_lifecycle.persistent_event_store_enabled",
            }
        );

        let mut unsafe_listener_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);
        unsafe_listener_policy
            .listener_policy
            .endpoint_policy
            .socket_listener_enabled = true;
        assert_eq!(
            unsafe_listener_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint.socket_listener_enabled",
            }
        );

        remove_temp_root(&root);
    }

    #[test]
    fn service_lifecycle_policy_rejects_unsafe_flags_nested_drift_and_invalid_caps() {
        let root = temp_policy_root("service-lifecycle-policy-drift");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let endpoint_lifecycle_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);

        assert_eq!(
            RuntimeControlPlaneServiceLifecyclePolicy::bounded(
                endpoint_lifecycle_policy.clone(),
                0,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "service_lifecycle.event_cap",
            }
        );
        assert_eq!(
            RuntimeControlPlaneServiceLifecyclePolicy::bounded(
                endpoint_lifecycle_policy.clone(),
                RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_DEFAULT_EVENT_CAP + 1,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "service_lifecycle.event_cap",
            }
        );

        let mut network_policy =
            RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy.clone());
        network_policy.public_network_transport_enabled = true;
        assert_eq!(
            network_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "service_lifecycle.public_network_transport_enabled",
            }
        );

        let mut loop_policy =
            RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy.clone());
        loop_policy.listener_loop_enabled = true;
        assert_eq!(
            loop_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "service_lifecycle.listener_loop_enabled",
            }
        );

        let mut async_stop_policy =
            RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy.clone());
        async_stop_policy.async_stop_api_enabled = true;
        assert_eq!(
            async_stop_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "service_lifecycle.async_stop_api_enabled",
            }
        );

        let mut audit_policy =
            RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy.clone());
        audit_policy.audit_events_enabled = false;
        assert_eq!(
            audit_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "service_lifecycle.audit_events_enabled",
            }
        );

        let mut event_store_policy =
            RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy.clone());
        event_store_policy.persistent_event_store_enabled = true;
        assert_eq!(
            event_store_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "service_lifecycle.persistent_event_store_enabled",
            }
        );

        let mut invalid_event_cap_policy =
            RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy.clone());
        invalid_event_cap_policy.event_cap = 0;
        assert_eq!(
            invalid_event_cap_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "service_lifecycle.event_cap",
            }
        );

        let mut unsafe_endpoint_policy =
            RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy);
        unsafe_endpoint_policy
            .endpoint_lifecycle_policy
            .listener_policy
            .endpoint_policy
            .socket_listener_enabled = true;
        assert_eq!(
            unsafe_endpoint_policy.validate().unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint.socket_listener_enabled",
            }
        );

        remove_temp_root(&root);
    }

    #[test]
    fn service_lifecycle_supervisor_starts_stopped_rejects_invalid_transition_and_caps_events() {
        let root = temp_policy_root("service-lifecycle-supervisor");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let endpoint_lifecycle_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);
        let policy =
            RuntimeControlPlaneServiceLifecyclePolicy::bounded(endpoint_lifecycle_policy, 3)
                .unwrap();
        let mut supervisor = RuntimeControlPlaneServiceLifecycleSupervisor::new(&policy).unwrap();

        assert_eq!(
            supervisor.state(),
            RuntimeControlPlaneServiceLifecycleState::Stopped
        );
        assert!(supervisor.events().is_empty());
        assert_eq!(
            supervisor
                .record_event(RuntimeControlPlaneServiceLifecycleEventKind::StopRequested)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "service_lifecycle.transition",
            }
        );
        assert_eq!(
            supervisor.state(),
            RuntimeControlPlaneServiceLifecycleState::Stopped
        );
        assert!(supervisor.events().is_empty());

        supervisor
            .record_event(RuntimeControlPlaneServiceLifecycleEventKind::StartRequested)
            .unwrap();
        supervisor
            .record_event(RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleStarted)
            .unwrap();
        supervisor
            .record_event(RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleCompleted)
            .unwrap();
        supervisor
            .record_event(RuntimeControlPlaneServiceLifecycleEventKind::StopRequested)
            .unwrap();
        supervisor
            .record_event(RuntimeControlPlaneServiceLifecycleEventKind::Stopped)
            .unwrap();

        assert_eq!(
            supervisor.state(),
            RuntimeControlPlaneServiceLifecycleState::Stopped
        );
        assert_eq!(
            supervisor
                .events()
                .iter()
                .map(|event| event.event_kind)
                .collect::<Vec<_>>(),
            vec![
                RuntimeControlPlaneServiceLifecycleEventKind::StartRequested,
                RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleStarted,
                RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleCompleted,
            ]
        );
        assert_eq!(supervisor.events()[0].event_index, 0);
        assert_eq!(supervisor.events()[1].event_index, 1);
        assert_eq!(supervisor.events()[2].event_index, 2);

        remove_temp_root(&root);
    }

    #[test]
    fn validates_safe_runtime_control_plane_endpoint_path_selection() {
        let root = temp_policy_root("endpoint-path-valid");
        let nested = root.join("ipc");
        std::fs::create_dir(&nested).unwrap();
        let path = nested.join("runtime-control.sock");
        let policy = RuntimeControlPlaneEndpointPathPolicy::new(root.clone());

        let selection = validate_control_plane_endpoint_path(&path, &policy).unwrap();

        assert_eq!(
            selection.schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_SCHEMA_VERSION
        );
        assert_eq!(
            selection.endpoint_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_SCHEMA_VERSION
        );
        assert_eq!(selection.endpoint_path, path.to_str().unwrap());
        assert_eq!(selection.allowed_root, root.to_str().unwrap());
        assert_eq!(selection.endpoint_filename, "runtime-control.sock");
        assert_eq!(
            selection.max_path_bytes,
            RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES
        );
        assert!(selection.local_only);
        assert!(selection.caller_authorized_allowed_root_required);
        assert!(selection.absolute_endpoint_path);
        assert!(selection.under_allowed_root);
        assert!(selection.target_parent_exists);
        assert!(selection.target_did_not_exist);
        assert_eq!(selection.socket_extension, "sock");
        assert!(selection.path_selection_only);
        assert!(selection.filesystem_socket_path_policy_enabled);
        assert!(!selection.filesystem_mutation_enabled);
        assert!(!selection.public_network_transport_enabled);
        assert!(!selection.socket_listener_enabled);
        assert!(!selection.daemon_lifecycle_enabled);
        assert!(!selection.process_spawning_enabled);
        assert!(!selection.file_watching_enabled);
        assert!(!selection.qt_binding_enabled);
        assert!(!selection.storage_provider_enabled);
        assert!(!selection.capture_enabled);
        assert!(!selection.external_services_used);
        assert!(!selection.deployment_allowed);
        assert!(!selection.native_inference_execution_enabled);
        assert_eq!(
            selection.non_claims,
            strings(RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_NON_CLAIMS)
        );
        assert!(!path.exists());

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn dispatches_runtime_control_plane_json_message_endpoint_listener_once() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let root = temp_policy_root("valid-endpoint-listener");
        let socket_path = root.join("runtime-control.sock");
        let policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let request_json =
            json_message_request("request-listener-json-001", synthetic_handoff_json());

        let server_path = socket_path.clone();
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            execute_control_plane_endpoint_listener_once(&server_path, &server_policy)
        });

        let mut client = connect_control_plane_listener_client(&socket_path);
        write_control_plane_message_ipc_frame(
            &mut client,
            request_json.as_bytes(),
            &policy.endpoint_policy.ipc_policy,
        )
        .unwrap();
        let response_frame =
            read_control_plane_message_ipc_frame(&mut client, &policy.endpoint_policy.ipc_policy)
                .unwrap();
        let from_listener = response_from_frame_bytes(response_frame);
        let from_endpoint = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap(),
        );
        let outcome = server_thread
            .join()
            .expect("test listener thread must complete")
            .unwrap();

        assert_eq!(from_listener, from_endpoint);
        assert_eq!(
            from_listener.request_id.as_str(),
            "request-listener-json-001"
        );
        assert_eq!(
            from_listener.outcome,
            RuntimeControlPlaneMessageOutcome::Success
        );
        assert_eq!(
            outcome.schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION
        );
        assert_eq!(
            outcome.endpoint_path_selection.endpoint_path,
            socket_path.to_str().unwrap()
        );
        assert!(outcome.local_only);
        assert!(outcome.one_shot_listener);
        assert!(outcome.filesystem_socket_binding_enabled);
        assert!(outcome.endpoint_path_validation_enabled);
        assert!(outcome.endpoint_stream_execution_enabled);
        assert!(outcome.cleanup_attempted);
        assert!(outcome.socket_path_removed);
        assert!(!outcome.public_network_transport_enabled);
        assert!(!outcome.listener_loop_enabled);
        assert!(!outcome.daemon_lifecycle_enabled);
        assert!(!outcome.process_spawning_enabled);
        assert!(!outcome.file_watching_enabled);
        assert!(!outcome.qt_binding_enabled);
        assert!(!outcome.storage_provider_enabled);
        assert!(!outcome.capture_enabled);
        assert!(!outcome.external_services_used);
        assert!(!outcome.deployment_allowed);
        assert!(!outcome.native_inference_execution_enabled);
        assert!(!socket_path.exists());

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn endpoint_listener_returns_failure_response_and_cleans_up_for_nested_rejections() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let root = temp_policy_root("endpoint-listener-nested-failure");
        let socket_path = root.join("runtime-control.sock");
        let policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let request_json = json_message_request("request-listener-006", "{");

        let server_path = socket_path.clone();
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            execute_control_plane_endpoint_listener_once(&server_path, &server_policy)
        });

        let mut client = connect_control_plane_listener_client(&socket_path);
        write_control_plane_message_ipc_frame(
            &mut client,
            request_json.as_bytes(),
            &policy.endpoint_policy.ipc_policy,
        )
        .unwrap();
        let response_frame =
            read_control_plane_message_ipc_frame(&mut client, &policy.endpoint_policy.ipc_policy)
                .unwrap();
        let response = response_from_frame_bytes(response_frame);
        let outcome = server_thread
            .join()
            .expect("test listener thread must complete")
            .unwrap();

        assert_eq!(response.request_id.as_str(), "request-listener-006");
        assert_eq!(response.outcome, RuntimeControlPlaneMessageOutcome::Failure);
        assert!(response.snapshot.is_none());
        assert_eq!(
            response.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::InvalidJson)
        );
        assert!(outcome.cleanup_attempted);
        assert!(outcome.socket_path_removed);
        assert!(!socket_path.exists());

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn endpoint_listener_parse_failures_return_adapter_errors_and_clean_up() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let root = temp_policy_root("endpoint-listener-parse-failure");
        let socket_path = root.join("runtime-control.sock");
        let policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );

        let server_path = socket_path.clone();
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            execute_control_plane_endpoint_listener_once(&server_path, &server_policy)
        });

        let mut client = connect_control_plane_listener_client(&socket_path);
        client.write_all(&ipc_frame_bytes(&[0xff])).unwrap();
        drop(client);

        assert_eq!(
            server_thread
                .join()
                .expect("test listener thread must complete")
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );
        assert!(!socket_path.exists());

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn dispatches_runtime_control_plane_json_message_endpoint_lifecycle_once() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let root = temp_policy_root("valid-endpoint-lifecycle");
        let socket_path = root.join("runtime-control.sock");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let policy = RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);
        let request_json =
            json_message_request("request-lifecycle-json-001", synthetic_handoff_json());

        let server_path = socket_path.clone();
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            execute_control_plane_endpoint_lifecycle_once(&server_path, &server_policy)
        });

        let mut client = connect_control_plane_listener_client(&socket_path);
        write_control_plane_message_ipc_frame(
            &mut client,
            request_json.as_bytes(),
            &policy.listener_policy.endpoint_policy.ipc_policy,
        )
        .unwrap();
        let response_frame = read_control_plane_message_ipc_frame(
            &mut client,
            &policy.listener_policy.endpoint_policy.ipc_policy,
        )
        .unwrap();
        let from_lifecycle = response_from_frame_bytes(response_frame);
        let from_frame = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap(),
        );
        let outcome = server_thread
            .join()
            .expect("test lifecycle thread must complete")
            .unwrap();

        assert_eq!(from_lifecycle, from_frame);
        assert_eq!(
            from_lifecycle.request_id.as_str(),
            "request-lifecycle-json-001"
        );
        assert_eq!(
            from_lifecycle.outcome,
            RuntimeControlPlaneMessageOutcome::Success
        );
        assert_eq!(
            outcome.schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_SCHEMA_VERSION
        );
        assert_eq!(
            outcome.listener_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_LISTENER_SCHEMA_VERSION
        );
        assert_eq!(
            outcome.final_state,
            RuntimeControlPlaneEndpointLifecycleState::Stopped
        );
        assert!(outcome.failure_error_code.is_none());
        assert!(outcome.listener_outcome.is_some());
        assert!(outcome.cleanup_attempted);
        assert!(outcome.socket_path_removed);
        assert!(outcome.local_only);
        assert!(outcome.one_shot_lifecycle);
        assert!(outcome.start_stop_state_enabled);
        assert!(outcome.audit_events_enabled);
        assert!(outcome.endpoint_listener_execution_enabled);
        assert!(!outcome.public_network_transport_enabled);
        assert!(!outcome.listener_loop_enabled);
        assert!(!outcome.daemon_lifecycle_enabled);
        assert!(!outcome.process_spawning_enabled);
        assert!(!outcome.file_watching_enabled);
        assert!(!outcome.qt_binding_enabled);
        assert!(!outcome.storage_provider_enabled);
        assert!(!outcome.capture_enabled);
        assert!(!outcome.external_services_used);
        assert!(!outcome.deployment_allowed);
        assert!(!outcome.native_inference_execution_enabled);
        assert!(!outcome.persistent_event_store_enabled);
        assert_eq!(
            outcome
                .events
                .iter()
                .map(|event| event.event_kind)
                .collect::<Vec<_>>(),
            vec![
                RuntimeControlPlaneEndpointLifecycleEventKind::StartRequested,
                RuntimeControlPlaneEndpointLifecycleEventKind::PathValidated,
                RuntimeControlPlaneEndpointLifecycleEventKind::SocketBound,
                RuntimeControlPlaneEndpointLifecycleEventKind::ClientAccepted,
                RuntimeControlPlaneEndpointLifecycleEventKind::RequestCompleted,
                RuntimeControlPlaneEndpointLifecycleEventKind::StopRequested,
                RuntimeControlPlaneEndpointLifecycleEventKind::CleanupCompleted,
            ]
        );
        assert_eq!(
            outcome.events[0].state,
            RuntimeControlPlaneEndpointLifecycleState::StartRequested
        );
        assert_eq!(outcome.events[0].event_index, 0);
        assert_eq!(outcome.events[0].event_label, "start_requested");
        assert_eq!(
            outcome.non_claims,
            strings(RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_NON_CLAIMS)
        );
        assert!(!socket_path.exists());

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn endpoint_lifecycle_returns_failure_response_and_cleans_up_for_nested_rejections() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let root = temp_policy_root("endpoint-lifecycle-nested-failure");
        let socket_path = root.join("runtime-control.sock");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let policy = RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);
        let request_json = json_message_request("request-lifecycle-006", "{");

        let server_path = socket_path.clone();
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            execute_control_plane_endpoint_lifecycle_once(&server_path, &server_policy)
        });

        let mut client = connect_control_plane_listener_client(&socket_path);
        write_control_plane_message_ipc_frame(
            &mut client,
            request_json.as_bytes(),
            &policy.listener_policy.endpoint_policy.ipc_policy,
        )
        .unwrap();
        let response_frame = read_control_plane_message_ipc_frame(
            &mut client,
            &policy.listener_policy.endpoint_policy.ipc_policy,
        )
        .unwrap();
        let response = response_from_frame_bytes(response_frame);
        let outcome = server_thread
            .join()
            .expect("test lifecycle thread must complete")
            .unwrap();

        assert_eq!(response.request_id.as_str(), "request-lifecycle-006");
        assert_eq!(response.outcome, RuntimeControlPlaneMessageOutcome::Failure);
        assert!(response.snapshot.is_none());
        assert_eq!(
            response.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::InvalidJson)
        );
        assert_eq!(
            outcome.final_state,
            RuntimeControlPlaneEndpointLifecycleState::Stopped
        );
        assert!(outcome.failure_error_code.is_none());
        assert!(outcome.listener_outcome.is_some());
        assert!(outcome.cleanup_attempted);
        assert!(outcome.socket_path_removed);
        assert!(!socket_path.exists());

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn endpoint_lifecycle_parse_failures_return_typed_failure_and_clean_up() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let root = temp_policy_root("endpoint-lifecycle-parse-failure");
        let socket_path = root.join("runtime-control.sock");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let policy = RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);

        let server_path = socket_path.clone();
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            execute_control_plane_endpoint_lifecycle_once(&server_path, &server_policy)
        });

        let mut client = connect_control_plane_listener_client(&socket_path);
        client.write_all(&ipc_frame_bytes(&[0xff])).unwrap();
        drop(client);

        let outcome = server_thread
            .join()
            .expect("test lifecycle thread must complete")
            .unwrap();

        assert_eq!(
            outcome.final_state,
            RuntimeControlPlaneEndpointLifecycleState::Failed
        );
        assert_eq!(
            outcome.failure_error_code,
            Some(RuntimeControlPlaneMessageErrorCode::InvalidUtf8)
        );
        assert!(outcome.listener_outcome.is_none());
        assert!(outcome.cleanup_attempted);
        assert!(outcome.socket_path_removed);
        assert_eq!(
            outcome.events.last().unwrap().event_kind,
            RuntimeControlPlaneEndpointLifecycleEventKind::Failed
        );
        assert!(outcome.events.iter().any(|event| event.event_kind
            == RuntimeControlPlaneEndpointLifecycleEventKind::CleanupCompleted));
        assert!(!socket_path.exists());

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn endpoint_lifecycle_rejections_before_bind_return_failed_outcomes() {
        let root = temp_policy_root("elifecycle-path");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let policy = RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);

        let outcome =
            execute_control_plane_endpoint_lifecycle_once(Path::new("relative.sock"), &policy)
                .unwrap();

        assert_eq!(
            outcome.final_state,
            RuntimeControlPlaneEndpointLifecycleState::Failed
        );
        assert_eq!(
            outcome.failure_error_code,
            Some(RuntimeControlPlaneMessageErrorCode::RelativeFilePath)
        );
        assert!(outcome.listener_outcome.is_none());
        assert!(!outcome.cleanup_attempted);
        assert!(!outcome.socket_path_removed);
        assert_eq!(
            outcome
                .events
                .iter()
                .map(|event| event.event_kind)
                .collect::<Vec<_>>(),
            vec![
                RuntimeControlPlaneEndpointLifecycleEventKind::StartRequested,
                RuntimeControlPlaneEndpointLifecycleEventKind::Failed,
            ]
        );

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn dispatches_runtime_control_plane_json_message_service_lifecycle_once() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let root = temp_policy_root("valid-service-lifecycle");
        let socket_path = root.join("runtime-control.sock");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let endpoint_lifecycle_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);
        let policy = RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy);
        let request_json =
            json_message_request("request-service-json-001", synthetic_handoff_json());

        let server_path = socket_path.clone();
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            execute_control_plane_service_lifecycle_once(&server_path, &server_policy)
        });

        let mut client = connect_control_plane_listener_client(&socket_path);
        write_control_plane_message_ipc_frame(
            &mut client,
            request_json.as_bytes(),
            &policy
                .endpoint_lifecycle_policy
                .listener_policy
                .endpoint_policy
                .ipc_policy,
        )
        .unwrap();
        let response_frame = read_control_plane_message_ipc_frame(
            &mut client,
            &policy
                .endpoint_lifecycle_policy
                .listener_policy
                .endpoint_policy
                .ipc_policy,
        )
        .unwrap();
        let from_service = response_from_frame_bytes(response_frame);
        let from_frame = response_from_frame_bytes(
            execute_control_plane_message_frame_bytes(request_json.as_bytes()).unwrap(),
        );
        let outcome = server_thread
            .join()
            .expect("test service lifecycle thread must complete")
            .unwrap();

        assert_eq!(from_service, from_frame);
        assert_eq!(from_service.request_id.as_str(), "request-service-json-001");
        assert_eq!(
            from_service.outcome,
            RuntimeControlPlaneMessageOutcome::Success
        );
        assert_eq!(
            outcome.schema_version,
            RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_SCHEMA_VERSION
        );
        assert_eq!(
            outcome.endpoint_lifecycle_schema_version,
            RUNTIME_CONTROL_PLANE_ENDPOINT_LIFECYCLE_SCHEMA_VERSION
        );
        assert_eq!(
            outcome.final_state,
            RuntimeControlPlaneServiceLifecycleState::Stopped
        );
        assert!(outcome.failure_error_code.is_none());
        assert!(outcome.endpoint_lifecycle_outcome.is_some());
        assert!(outcome.cleanup_attempted);
        assert!(outcome.socket_path_removed);
        assert!(outcome.local_only);
        assert!(outcome.service_state_enabled);
        assert!(outcome.explicit_start_stop_state_enabled);
        assert!(outcome.one_shot_endpoint_execution_enabled);
        assert!(outcome.audit_events_enabled);
        assert!(outcome.capped_in_memory_events_enabled);
        assert!(outcome.nested_endpoint_lifecycle_execution_enabled);
        assert_eq!(
            outcome.event_cap,
            RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_DEFAULT_EVENT_CAP
        );
        assert!(!outcome.public_network_transport_enabled);
        assert!(!outcome.listener_loop_enabled);
        assert!(!outcome.daemon_lifecycle_enabled);
        assert!(!outcome.async_stop_api_enabled);
        assert!(!outcome.process_spawning_enabled);
        assert!(!outcome.file_watching_enabled);
        assert!(!outcome.qt_binding_enabled);
        assert!(!outcome.storage_provider_enabled);
        assert!(!outcome.persistent_event_store_enabled);
        assert!(!outcome.capture_enabled);
        assert!(!outcome.external_services_used);
        assert!(!outcome.deployment_allowed);
        assert!(!outcome.native_inference_execution_enabled);
        assert_eq!(
            outcome
                .events
                .iter()
                .map(|event| event.event_kind)
                .collect::<Vec<_>>(),
            vec![
                RuntimeControlPlaneServiceLifecycleEventKind::StartRequested,
                RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleStarted,
                RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleCompleted,
                RuntimeControlPlaneServiceLifecycleEventKind::StopRequested,
                RuntimeControlPlaneServiceLifecycleEventKind::Stopped,
            ]
        );
        assert_eq!(
            outcome.events[0].state,
            RuntimeControlPlaneServiceLifecycleState::Starting
        );
        assert_eq!(outcome.events[0].event_index, 0);
        assert_eq!(outcome.events[0].event_label, "start_requested");
        assert_eq!(
            outcome.non_claims,
            strings(RUNTIME_CONTROL_PLANE_SERVICE_LIFECYCLE_NON_CLAIMS)
        );
        assert!(!socket_path.exists());

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn service_lifecycle_returns_failure_response_and_stops_for_nested_rejections() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let root = temp_policy_root("service-lifecycle-nested-failure");
        let socket_path = root.join("runtime-control.sock");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let endpoint_lifecycle_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);
        let policy = RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy);
        let request_json = json_message_request("request-service-006", "{");

        let server_path = socket_path.clone();
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            execute_control_plane_service_lifecycle_once(&server_path, &server_policy)
        });

        let mut client = connect_control_plane_listener_client(&socket_path);
        write_control_plane_message_ipc_frame(
            &mut client,
            request_json.as_bytes(),
            &policy
                .endpoint_lifecycle_policy
                .listener_policy
                .endpoint_policy
                .ipc_policy,
        )
        .unwrap();
        let response_frame = read_control_plane_message_ipc_frame(
            &mut client,
            &policy
                .endpoint_lifecycle_policy
                .listener_policy
                .endpoint_policy
                .ipc_policy,
        )
        .unwrap();
        let response = response_from_frame_bytes(response_frame);
        let outcome = server_thread
            .join()
            .expect("test service lifecycle thread must complete")
            .unwrap();

        assert_eq!(response.request_id.as_str(), "request-service-006");
        assert_eq!(response.outcome, RuntimeControlPlaneMessageOutcome::Failure);
        assert!(response.snapshot.is_none());
        assert_eq!(
            response.error_code,
            Some(RuntimeControlPlaneMessageErrorCode::InvalidJson)
        );
        assert_eq!(
            outcome.final_state,
            RuntimeControlPlaneServiceLifecycleState::Stopped
        );
        assert!(outcome.failure_error_code.is_none());
        assert_eq!(
            outcome
                .endpoint_lifecycle_outcome
                .as_ref()
                .unwrap()
                .final_state,
            RuntimeControlPlaneEndpointLifecycleState::Stopped
        );
        assert!(outcome.cleanup_attempted);
        assert!(outcome.socket_path_removed);
        assert!(!socket_path.exists());

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn service_lifecycle_parse_failures_return_failed_outcome_and_clean_up() {
        if !unix_stream_pair_writes_are_permitted() {
            return;
        }

        let root = temp_policy_root("service-lifecycle-parse-failure");
        let socket_path = root.join("runtime-control.sock");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let endpoint_lifecycle_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);
        let policy = RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy);

        let server_path = socket_path.clone();
        let server_policy = policy.clone();
        let server_thread = std::thread::spawn(move || {
            execute_control_plane_service_lifecycle_once(&server_path, &server_policy)
        });

        let mut client = connect_control_plane_listener_client(&socket_path);
        client.write_all(&ipc_frame_bytes(&[0xff])).unwrap();
        drop(client);

        let outcome = server_thread
            .join()
            .expect("test service lifecycle thread must complete")
            .unwrap();

        assert_eq!(
            outcome.final_state,
            RuntimeControlPlaneServiceLifecycleState::Failed
        );
        assert_eq!(
            outcome.failure_error_code,
            Some(RuntimeControlPlaneMessageErrorCode::InvalidUtf8)
        );
        assert_eq!(
            outcome
                .endpoint_lifecycle_outcome
                .as_ref()
                .unwrap()
                .final_state,
            RuntimeControlPlaneEndpointLifecycleState::Failed
        );
        assert!(outcome.cleanup_attempted);
        assert!(outcome.socket_path_removed);
        assert_eq!(
            outcome.events.last().unwrap().event_kind,
            RuntimeControlPlaneServiceLifecycleEventKind::Failed
        );
        assert!(!socket_path.exists());

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn service_lifecycle_rejections_before_bind_return_failed_outcomes() {
        let root = temp_policy_root("service-lifecycle-path");
        let listener_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );
        let endpoint_lifecycle_policy =
            RuntimeControlPlaneEndpointLifecyclePolicy::new(listener_policy);
        let policy = RuntimeControlPlaneServiceLifecyclePolicy::new(endpoint_lifecycle_policy);

        let outcome =
            execute_control_plane_service_lifecycle_once(Path::new("relative.sock"), &policy)
                .unwrap();

        assert_eq!(
            outcome.final_state,
            RuntimeControlPlaneServiceLifecycleState::Failed
        );
        assert_eq!(
            outcome.failure_error_code,
            Some(RuntimeControlPlaneMessageErrorCode::RelativeFilePath)
        );
        assert_eq!(
            outcome
                .endpoint_lifecycle_outcome
                .as_ref()
                .unwrap()
                .final_state,
            RuntimeControlPlaneEndpointLifecycleState::Failed
        );
        assert!(!outcome.cleanup_attempted);
        assert!(!outcome.socket_path_removed);
        assert_eq!(
            outcome
                .events
                .iter()
                .map(|event| event.event_kind)
                .collect::<Vec<_>>(),
            vec![
                RuntimeControlPlaneServiceLifecycleEventKind::StartRequested,
                RuntimeControlPlaneServiceLifecycleEventKind::EndpointLifecycleStarted,
                RuntimeControlPlaneServiceLifecycleEventKind::Failed,
            ]
        );

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn endpoint_listener_rejects_unsafe_paths_before_bind() {
        let root = temp_policy_root("elist-path");
        let policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );

        assert_eq!(
            execute_control_plane_endpoint_listener_once(Path::new("relative.sock"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeFilePath
        );

        let relative_root_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new("relative-root"),
        );
        assert_eq!(
            execute_control_plane_endpoint_listener_once(
                root.join("runtime-control.sock"),
                &relative_root_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeAllowedRoot
        );

        let missing_root = root.join("missing-root");
        let missing_root_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(missing_root.clone()),
        );
        assert_eq!(
            execute_control_plane_endpoint_listener_once(
                missing_root.join("runtime-control.sock"),
                &missing_root_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingAllowedRoot
        );

        assert_eq!(
            execute_control_plane_endpoint_listener_once(
                root.join("missing/runtime.sock"),
                &policy
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingFile
        );

        let outside = temp_policy_root("elist-out");
        let outside_existing = write_test_file(&outside, "outside-exists.sock", b"outside");
        assert_eq!(
            execute_control_plane_endpoint_listener_once(outside_existing, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OutsideAllowedRoot
        );

        let existing_file = write_test_file(&root, "runtime-control-file.sock", b"existing");
        assert_eq!(
            execute_control_plane_endpoint_listener_once(&existing_file, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.target_exists",
            }
        );

        let existing_dir = root.join("runtime-control-dir.sock");
        std::fs::create_dir(&existing_dir).unwrap();
        assert_eq!(
            execute_control_plane_endpoint_listener_once(&existing_dir, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::DirectoryPath
        );

        let existing_socket = root.join("runtime-control-existing.sock");
        match UnixListener::bind(&existing_socket) {
            Ok(existing_listener) => {
                assert_eq!(
                    execute_control_plane_endpoint_listener_once(&existing_socket, &policy)
                        .unwrap_err(),
                    RuntimeControlPlaneAdapterError::NonRegularFile
                );
                drop(existing_listener);
                std::fs::remove_file(&existing_socket).unwrap();
            }
            Err(error) if error.raw_os_error() == Some(1) => {}
            Err(error) => panic!("test socket fixture failed unexpectedly: {error}"),
        }

        assert_eq!(
            execute_control_plane_endpoint_listener_once(root.join("runtime-control.txt"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedFileExtension
        );
        assert_eq!(
            execute_control_plane_endpoint_listener_once(root.join("secret.sock"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.endpoint_filename",
            }
        );
        assert_eq!(
            execute_control_plane_endpoint_listener_once(root.join("private-key.sock"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.endpoint_filename",
            }
        );

        let long_path = root.join(format!("{}.sock", "a".repeat(128)));
        assert_eq!(
            execute_control_plane_endpoint_listener_once(long_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedPath {
                max_bytes: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES,
            }
        );

        let real_root = root.join("real-root");
        std::fs::create_dir(&real_root).unwrap();
        let symlink_root = root.join("symlink-root");
        std::os::unix::fs::symlink(&real_root, &symlink_root).unwrap();
        let symlink_root_policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(symlink_root.clone()),
        );
        assert_eq!(
            execute_control_plane_endpoint_listener_once(
                symlink_root.join("runtime-control.sock"),
                &symlink_root_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootSymlink
        );

        let real_parent = root.join("real-parent");
        std::fs::create_dir(&real_parent).unwrap();
        let symlink_parent = root.join("symlink-parent");
        std::os::unix::fs::symlink(&real_parent, &symlink_parent).unwrap();
        assert_eq!(
            execute_control_plane_endpoint_listener_once(
                symlink_parent.join("runtime-control.sock"),
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::SymlinkPath
        );

        let target = write_test_file(&root, "target.sock", b"existing");
        let symlink_target = root.join("linked.sock");
        std::os::unix::fs::symlink(&target, &symlink_target).unwrap();
        assert_eq!(
            execute_control_plane_endpoint_listener_once(&symlink_target, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::SymlinkPath
        );

        let non_utf8_path = PathBuf::from(OsString::from_vec(b"/tmp/\xff.sock".to_vec()));
        assert_eq!(
            execute_control_plane_endpoint_listener_once(non_utf8_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );

        remove_temp_root(&outside);
        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn endpoint_listener_rejects_permissive_roots_and_parents() {
        let root = temp_policy_root("elist-perms");
        let socket_path = root.join("runtime-control.sock");
        let policy = RuntimeControlPlaneEndpointListenerPolicy::new(
            RuntimeControlPlaneEndpointPathPolicy::new(root.clone()),
        );

        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            execute_control_plane_endpoint_listener_once(&socket_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_listener.allowed_root_permissions",
            }
        );
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();

        let nested = root.join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            execute_control_plane_endpoint_listener_once(
                nested.join("runtime-control.sock"),
                &policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_listener.parent_permissions",
            }
        );

        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn endpoint_listener_cleanup_rejects_non_socket_replacements() {
        let root = temp_policy_root("elist-cleanup");
        let socket_path = root.join("runtime-control.sock");
        std::fs::write(&socket_path, b"replacement").unwrap();

        assert_eq!(
            cleanup_control_plane_endpoint_socket_path(&socket_path).unwrap_err(),
            RuntimeControlPlaneAdapterError::EndpointCleanupFailed
        );
        assert!(socket_path.exists());

        remove_temp_root(&root);
    }

    #[test]
    fn endpoint_path_policy_rejects_unsafe_flags() {
        let root = temp_policy_root("endpoint-path-unsafe-flags");
        let path = root.join("runtime-control.sock");

        let mut network_policy = RuntimeControlPlaneEndpointPathPolicy::new(root.clone());
        network_policy.public_network_transport_enabled = true;
        assert_eq!(
            validate_control_plane_endpoint_path(&path, &network_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_path.public_network_transport_enabled",
            }
        );

        let mut listener_policy = RuntimeControlPlaneEndpointPathPolicy::new(root.clone());
        listener_policy.socket_listener_enabled = true;
        assert_eq!(
            validate_control_plane_endpoint_path(&path, &listener_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_path.socket_listener_enabled",
            }
        );

        let mut mutation_policy = RuntimeControlPlaneEndpointPathPolicy::new(root.clone());
        mutation_policy.filesystem_mutation_enabled = true;
        assert_eq!(
            validate_control_plane_endpoint_path(&path, &mutation_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsafeFlag {
                field: "endpoint_path.filesystem_mutation_enabled",
            }
        );

        let mut oversized_policy = RuntimeControlPlaneEndpointPathPolicy::new(root.clone());
        oversized_policy.max_path_bytes = RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES + 1;
        assert_eq!(
            validate_control_plane_endpoint_path(&path, &oversized_policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.max_path_bytes",
            }
        );

        remove_temp_root(&root);
    }

    #[test]
    fn endpoint_path_validation_rejects_unsafe_paths() {
        let root = temp_policy_root("endpoint-path-unsafe");
        let policy = RuntimeControlPlaneEndpointPathPolicy::new(root.clone());

        assert_eq!(
            validate_control_plane_endpoint_path(Path::new("relative.sock"), &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeFilePath
        );

        let relative_root_policy = RuntimeControlPlaneEndpointPathPolicy::new("relative-root");
        assert_eq!(
            validate_control_plane_endpoint_path(root.join("runtime.sock"), &relative_root_policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::RelativeAllowedRoot
        );

        let missing_root = root.join("missing-root");
        let missing_root_policy = RuntimeControlPlaneEndpointPathPolicy::new(missing_root.clone());
        assert_eq!(
            validate_control_plane_endpoint_path(
                missing_root.join("runtime.sock"),
                &missing_root_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingAllowedRoot
        );

        let file_root = write_test_file(&root, "file-root", b"not a directory");
        let file_root_policy = RuntimeControlPlaneEndpointPathPolicy::new(file_root.clone());
        assert_eq!(
            validate_control_plane_endpoint_path(file_root.join("runtime.sock"), &file_root_policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootNotDirectory
        );

        assert_eq!(
            validate_control_plane_endpoint_path(root.join("missing/runtime.sock"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::MissingFile
        );

        let outside = temp_policy_root("endpoint-path-outside");
        assert_eq!(
            validate_control_plane_endpoint_path(outside.join("runtime.sock"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::OutsideAllowedRoot
        );
        let outside_existing = write_test_file(&outside, "outside-exists.sock", b"outside");
        assert_eq!(
            validate_control_plane_endpoint_path(outside_existing, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OutsideAllowedRoot
        );

        let existing_file = write_test_file(&root, "runtime.sock", b"existing");
        assert_eq!(
            validate_control_plane_endpoint_path(&existing_file, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.target_exists",
            }
        );

        let existing_dir = root.join("directory.sock");
        std::fs::create_dir(&existing_dir).unwrap();
        assert_eq!(
            validate_control_plane_endpoint_path(&existing_dir, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::DirectoryPath
        );

        assert_eq!(
            validate_control_plane_endpoint_path(root.join("runtime.txt"), &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedFileExtension
        );
        assert_eq!(
            validate_control_plane_endpoint_path(root.join("secret.sock"), &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.endpoint_filename",
            }
        );
        assert_eq!(
            validate_control_plane_endpoint_path(root.join("private-key.sock"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::UnsupportedValue {
                field: "endpoint_path.endpoint_filename",
            }
        );

        let long_path = root.join(format!("{}.sock", "a".repeat(128)));
        assert_eq!(
            validate_control_plane_endpoint_path(long_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::OversizedPath {
                max_bytes: RUNTIME_CONTROL_PLANE_ENDPOINT_PATH_MAX_BYTES,
            }
        );

        remove_temp_root(&outside);
        remove_temp_root(&root);
    }

    #[cfg(unix)]
    #[test]
    fn endpoint_path_validation_rejects_symlinks_non_regular_and_non_utf8_paths() {
        let root = temp_policy_root("endpoint-path-unix");
        let policy = RuntimeControlPlaneEndpointPathPolicy::new(root.clone());

        let real_root = root.join("real-root");
        std::fs::create_dir(&real_root).unwrap();
        let symlink_root = root.join("symlink-root");
        std::os::unix::fs::symlink(&real_root, &symlink_root).unwrap();
        let symlink_root_policy = RuntimeControlPlaneEndpointPathPolicy::new(symlink_root.clone());
        assert_eq!(
            validate_control_plane_endpoint_path(
                symlink_root.join("runtime.sock"),
                &symlink_root_policy,
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::AllowedRootSymlink
        );

        let real_parent = root.join("real-parent");
        std::fs::create_dir(&real_parent).unwrap();
        let symlink_parent = root.join("symlink-parent");
        std::os::unix::fs::symlink(&real_parent, &symlink_parent).unwrap();
        assert_eq!(
            validate_control_plane_endpoint_path(symlink_parent.join("runtime.sock"), &policy)
                .unwrap_err(),
            RuntimeControlPlaneAdapterError::SymlinkPath
        );

        let target = write_test_file(&root, "target.sock", b"existing");
        let symlink_target = root.join("linked.sock");
        std::os::unix::fs::symlink(&target, &symlink_target).unwrap();
        assert_eq!(
            validate_control_plane_endpoint_path(&symlink_target, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::SymlinkPath
        );

        let fifo_path = root.join("fifo.sock");
        make_fifo(&fifo_path);
        assert_eq!(
            validate_control_plane_endpoint_path(&fifo_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::NonRegularFile
        );

        let non_utf8_path = PathBuf::from(OsString::from_vec(b"/tmp/\xff.sock".to_vec()));
        assert_eq!(
            validate_control_plane_endpoint_path(non_utf8_path, &policy).unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );

        let non_utf8_root = PathBuf::from(OsString::from_vec(b"/tmp/\xff-root".to_vec()));
        let non_utf8_root_policy = RuntimeControlPlaneEndpointPathPolicy::new(non_utf8_root);
        assert_eq!(
            validate_control_plane_endpoint_path(
                root.join("runtime-two.sock"),
                &non_utf8_root_policy
            )
            .unwrap_err(),
            RuntimeControlPlaneAdapterError::InvalidUtf8
        );

        remove_temp_root(&root);
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
