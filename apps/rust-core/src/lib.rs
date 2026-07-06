use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const RUNTIME_CONTRACT_VERSION: &str = "rust_runtime_contract.v0";
pub const RUNTIME_SUMMARY_SCHEMA_VERSION: &str = "runtime_summary.v0";
pub const MODEL_REGISTRY_METADATA_SCHEMA_VERSION: &str = "model_registry_metadata.v0";
pub const MODEL_REGISTRY_METADATA_SCOPE: &str = "local_synthetic_model_registry_metadata";
pub const MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION: &str = "model_evaluation_bundle.v0";
pub const RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION: &str = "runtime_handoff_snapshot.v0";
pub const RUNTIME_CONTROL_PLANE_ADAPTER_SCHEMA_VERSION: &str = "runtime_control_plane_adapter.v0";
pub const RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION: &str = "runtime_control_plane_message.v0";
pub const RUNTIME_CONTROL_PLANE_FILE_MAX_BYTES: u64 = 256 * 1024;
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
    FileReadFailed,
    InvalidUtf8,
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
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlPlaneInputMode {
    AcceptedSchemaDeclarationOnly,
    AcceptedLocalJsonString,
    AcceptedLocalJsonFile,
    AcceptedLocalMessageEnvelope,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlPlaneAdapterState {
    Unavailable,
    JsonStringParserAvailable,
    LocalFileAdapterAvailable,
    LocalMessageEnvelopeAvailable,
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
    FileReadFailed,
    InvalidUtf8,
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
            Self::FileReadFailed => "file_read_failed",
            Self::InvalidUtf8 => "invalid_utf8",
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
            RuntimeControlPlaneAdapterError::FileReadFailed => Self::FileReadFailed,
            RuntimeControlPlaneAdapterError::InvalidUtf8 => Self::InvalidUtf8,
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

impl RuntimeControlPlaneAdapterContract {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_CONTROL_PLANE_ADAPTER_SCHEMA_VERSION,
            adapter_kind: RuntimeControlPlaneAdapterKind::LocalControlPlaneMessageEnvelope,
            input_mode: RuntimeControlPlaneInputMode::AcceptedLocalMessageEnvelope,
            adapter_state: RuntimeControlPlaneAdapterState::LocalMessageEnvelopeAvailable,
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
        let canonical_path = validate_runtime_handoff_snapshot_file_path(path.as_ref(), policy)?;
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

fn validate_runtime_handoff_snapshot_file_path(
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
    validate_exact_strings(
        "model_registry_metadata.aggregate_summary.schemas_present",
        &metadata.aggregate_summary.schemas_present,
        MODEL_REGISTRY_AGGREGATE_SCHEMAS,
    )?;
    validate_exact_strings(
        "model_registry_metadata.aggregate_summary.models_with_score_rows",
        &metadata.aggregate_summary.models_with_score_rows,
        MODEL_REGISTRY_MODELS_WITH_SCORE_ROWS,
    )?;
    if metadata.aggregate_summary.model_count != metadata.entries.len() as u32 {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.aggregate_summary.model_count",
        });
    }
    validate_model_registry_entry_order(&metadata.entries)?;
    validate_model_registry_safety_flags(&metadata.safety_flags)?;
    for entry in &metadata.entries {
        validate_model_registry_entry(entry)?;
    }

    Ok(())
}

fn validate_model_registry_entry_order(
    entries: &[ModelRegistryEntry],
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if entries.len() != MODEL_REGISTRY_MODEL_IDS.len()
        || !entries
            .iter()
            .zip(MODEL_REGISTRY_MODEL_IDS.iter())
            .all(|(entry, expected_model_id)| entry.model_id == *expected_model_id)
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.entries",
        });
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
    validate_safe_label("model_registry_metadata.entries.model_id", &entry.model_id)?;
    let expected = expected_model_registry_entry(&entry.model_id)?;
    validate_exact_strings(
        "model_registry_metadata.entries.observed_source_schemas",
        &entry.observed_source_schemas,
        expected.observed_source_schemas,
    )?;
    validate_exact_strings(
        "model_registry_metadata.entries.observed_source_names",
        &entry.observed_source_names,
        expected.observed_source_names,
    )?;
    if entry.source_count != expected.source_count
        || entry.has_score_rows != expected.has_score_rows
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.entries.model_shape",
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
    if entry.source_count != entry.observed_source_schemas.len() as u32
        || entry.source_count != entry.observed_source_names.len() as u32
    {
        return Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.entries.source_count",
        });
    }
    for source_name in &entry.observed_source_names {
        validate_safe_label(
            "model_registry_metadata.entries.observed_source_names",
            source_name,
        )?;
    }
    Ok(())
}

struct ExpectedModelRegistryEntry {
    observed_source_schemas: &'static [&'static str],
    observed_source_names: &'static [&'static str],
    source_count: u32,
    has_score_rows: bool,
}

fn expected_model_registry_entry(
    model_id: &str,
) -> Result<ExpectedModelRegistryEntry, RuntimeControlPlaneAdapterError> {
    match model_id {
        "graph_novelty" => Ok(ExpectedModelRegistryEntry {
            observed_source_schemas: MODEL_REGISTRY_GRAPH_NOVELTY_SCHEMAS,
            observed_source_names: MODEL_REGISTRY_GRAPH_NOVELTY_SOURCE_NAMES,
            source_count: 4,
            has_score_rows: true,
        }),
        "isolation_forest" | "pyod_ecod" | "river_hst" => Ok(ExpectedModelRegistryEntry {
            observed_source_schemas: MODEL_REGISTRY_AGENTIC_DETECTION_DISAGREEMENT_SCHEMAS,
            observed_source_names: MODEL_REGISTRY_AGENTIC_DETECTION_DISAGREEMENT_SOURCE_NAMES,
            source_count: 3,
            has_score_rows: true,
        }),
        "model_disagreement" => Ok(ExpectedModelRegistryEntry {
            observed_source_schemas: MODEL_REGISTRY_INVESTIGATION_SCHEMAS,
            observed_source_names: MODEL_REGISTRY_INVESTIGATION_SOURCE_NAMES,
            source_count: 2,
            has_score_rows: false,
        }),
        "pyod_copod" | "suricata_alert" => Ok(ExpectedModelRegistryEntry {
            observed_source_schemas: MODEL_REGISTRY_AGENTIC_DISAGREEMENT_SCHEMAS,
            observed_source_names: MODEL_REGISTRY_AGENTIC_DISAGREEMENT_SOURCE_NAMES,
            source_count: 2,
            has_score_rows: true,
        }),
        "self_supervised_representation" => Ok(ExpectedModelRegistryEntry {
            observed_source_schemas: MODEL_REGISTRY_REPRESENTATION_SCHEMAS,
            observed_source_names: MODEL_REGISTRY_REPRESENTATION_SOURCE_NAMES,
            source_count: 2,
            has_score_rows: false,
        }),
        "stdlib_linear_native" => Ok(ExpectedModelRegistryEntry {
            observed_source_schemas: MODEL_REGISTRY_NATIVE_SCORE_SCHEMAS,
            observed_source_names: MODEL_REGISTRY_NATIVE_SCORE_SOURCE_NAMES,
            source_count: 1,
            has_score_rows: true,
        }),
        "time_series_residual" => Ok(ExpectedModelRegistryEntry {
            observed_source_schemas: MODEL_REGISTRY_TIME_SERIES_SCHEMAS,
            observed_source_names: MODEL_REGISTRY_TIME_SERIES_SOURCE_NAMES,
            source_count: 4,
            has_score_rows: true,
        }),
        _ => Err(RuntimeControlPlaneAdapterError::UnsupportedValue {
            field: "model_registry_metadata.entries.model_id",
        }),
    }
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

fn validate_safe_label(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeControlPlaneAdapterError> {
    if value.is_empty()
        || value.len() > 96
        || value.contains('.')
        || value.contains(':')
        || value.contains('@')
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
    {
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
    RUNTIME_CONTROL_PLANE_MESSAGE_SCHEMA_VERSION,
    RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION,
    RUNTIME_SUMMARY_SCHEMA_VERSION,
    MODEL_REGISTRY_METADATA_SCHEMA_VERSION,
];

const RUNTIME_CONTROL_PLANE_REQUEST_ID_BLOCKED_PARTS: &[&str] =
    &["private", "secret", "credential"];

const RUNTIME_CONTROL_PLANE_ADAPTER_NON_CLAIMS: &[&str] = &[
    "not_arbitrary_file_loader",
    "not_file_watcher",
    "not_ipc_or_socket_transport",
    "not_live_transport",
    "not_message_transport",
    "not_qt_binding",
    "not_external_service",
    "not_deployment_approval",
    "not_runtime_service",
    "not_generated_report_loader",
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
const MODEL_REGISTRY_MODEL_IDS: &[&str] = &[
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
    fn emits_static_runtime_control_plane_adapter_contract_fixture() {
        let contract = RuntimeControlPlaneAdapterContract::synthetic_fixture();

        assert_eq!(
            contract.schema_version,
            RUNTIME_CONTROL_PLANE_ADAPTER_SCHEMA_VERSION
        );
        assert_eq!(
            contract.adapter_kind.as_str(),
            "local_control_plane_message_envelope"
        );
        assert_eq!(
            contract.input_mode.as_str(),
            "accepted_local_message_envelope"
        );
        assert_eq!(
            contract.adapter_state.as_str(),
            "local_message_envelope_available"
        );
        assert_eq!(
            contract.output_snapshot_schema.as_str(),
            RUNTIME_HANDOFF_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(
            contract.accepted_input_schemas,
            &[
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
                "not_ipc_or_socket_transport",
                "not_live_transport",
                "not_message_transport",
                "not_qt_binding",
                "not_external_service",
                "not_deployment_approval",
                "not_runtime_service",
                "not_generated_report_loader"
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
